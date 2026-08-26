# Making the agents real

**Status:** design, nothing implemented.
**For:** the three of us, to decide together.

Every claim about the current tree below was checked against the code.
Exact request shapes for the model API were **not** verified against
current provider docs — check those before writing the call.

---

## 1. Where we actually stand

There is no multi-agent system running. Precisely:

| Claim | Reality |
|---|---|
| Rig is the agent abstraction (README) | `rig` is **not a dependency** in any `Cargo.toml` |
| Reader agent | Keyword matching — `INSTITUTIONS`, `HIGH_URGENCY`, `SEND_WORDS` |
| Linker agent | `enum Linker { Stub }` — one variant, `linker.rs:112` |
| `Reader::Remote` | The **same stub logic** in another process (`crates/reader/src/main.rs:38`) |

`Reader::Remote` exists so beat six has a real PID to kill. It is not a
model and never was.

**This matters for judging.** The Innovation criterion asks whether we
use multi-agent systems "in an interesting or differentiated way." A
judge who opens `reader.rs` finds string lists. We should either make the
agents real or be scrupulously honest that the agent layer runs offline
today — the one thing we must not do is imply an LLM is running.

---

## 2. What is already differentiated — and worth protecting

Most multi-agent projects are *agents talking to each other*. Ours is
**agents deliberately crippled so that neither can complete the attack.**

- **Reader** sees attacker prose and has zero account access. A message
  engineered to manipulate it is talking to something with no power.
- **Linker** has account context and **never sees prose**. `LinkerView`
  (`linker.rs:68`) contains no `String` and no variant carrying one.
- Even the institution name is discarded — `authority_claimed` is a
  `bool`. The comment at `linker.rs:71` says why: carrying the name would
  reopen the text channel.

That is least privilege applied to agents, with a typed channel between
them. It is a genuinely novel pattern and it is the strongest story we
have. **Nothing below may weaken it.**

The demonstrable version of the claim: we can print the exact bytes sent
to the Linker and show there is no attacker text in them. Very few teams
can do that. See §7.

---

## 3. The shape of the change

### 3.1 Reader

`Reader` is already an enum with two variants (`reader.rs:34`). A third
sits beside them:

```rust
pub enum Reader {
    Stub,
    Remote { base_url: String, client: reqwest::Client, timeout: Duration },
    Model  { client: reqwest::Client, model: String, timeout: Duration },
}
```

`read()` already returns
`Result<(Validated<PressureSignal>, SanitisationReport), ScreenError>`.
The model arm asks for JSON and hands it to the **existing**
`validate_reader_json` (`validate.rs:211`) — the same function
`Reader::Remote` uses. One validation path, as the crate doc promises.

The model must emit exactly this, and nothing else:

```json
{
  "urgency": "None" | "Low" | "High",
  "authority_claim": string | null,
  "requested_action": "SendMoney" | "ShareCredentials" | "CallNumber"
                    | {"Other": string},
  "named_amount": {"minor_units": int, "currency": "NGN"} | null,
  "named_recipient": string | null,
  "confidence": "Low" | "Medium" | "High"
}
```

Force the shape with a tool definition rather than asking for JSON in
prose. Free-form JSON from a model is a parsing problem we do not need.

### 3.2 Linker

```rust
pub enum Linker {
    Stub,
    Model { client: reqwest::Client, model: String, timeout: Duration },
}
```

**This one has a real cost:** `judge()` is currently **synchronous**
(`linker.rs:118`). A model-backed Linker makes it `async`, which changes
its one call site in `screen()` (`screening.rs:76`) and every test that
calls it directly. Not hard, but it is not a one-line change either.

The Linker returns only:

```json
{ "verdict": "Responsive" | "Unrelated" | "Unknown" }
```

**The rationale is not the model's to write.** `PlainReason` is picked in
Rust from the verdict, exactly as `stub_judge` does today. A model must
never author what a user reads.

---

## 4. Three things that will bite

### 4.1 The timeout budget is already spent

`SCREENING_TIMEOUT` is **3 seconds** (`screening.rs:16`), and
`screen_supervised` returns `Unavailable` — a hold — when it expires.

Two sequential model calls will not reliably fit in 3s. The options:

1. **Raise the timeout.** Then a held transfer takes longer to decide and
   the demo drags.
2. **Model Reader, stub Linker.** The Reader is where the language
   problem actually is. The Linker's job is small and structured, and the
   stub may already be the right answer.
3. **Accept the timeouts.** They fail closed, so nothing is unsafe — but
   "our agents time out and we hold everything" is a bad demo.

**Recommendation: option 2.** One model call, in the place where reading
natural language is genuinely the hard part. It is also the honest
framing: the Linker is a small deterministic judgement over typed facts,
and making it a model call adds latency and risk for very little.

Pick a fast model for the hot path. A large reasoning model in front of
every transaction is the wrong trade here.

### 4.2 Validation stops being decorative

Today the stub **cannot** emit attacker text: `authority_claim` is drawn
from the fixed `INSTITUTIONS` list, never copied from the message. There
is a test asserting exactly that (`reader.rs:374`).

A model can put anything in those fields. `PressureSignal` has three
free-text surfaces — `authority_claim`, `RequestedAction::Other(String)`,
and `MaskedMsisdn(String)`. `validate.rs` sanitises them, and that code
becomes load-bearing the moment a model is behind it.

**Before shipping this, add an eval** that feeds the Reader a message
engineered to make it echo attacker text into `authority_claim`, and
assert the sanitiser strips it. That test does not exist yet because it
could not fail today.

### 4.3 The evals cannot grade a model

`evals/src/lib.rs:77` and `:103` hard-code `Reader::Stub, Linker::Stub`.
The suite would stay green while telling us nothing. The seam is
specified in [`proposal.md`](./proposal.md) — do that first, or we are
building blind.

---

## 5. Keeping the offline path

Non-negotiable, for two reasons: `cargo test` must never need a key, and
stub mode is the fallback if venue wifi dies.

```toml
# crates/agents/Cargo.toml
[features]
default = []
model = []
```

```
cargo test --workspace                          # stub, offline — unchanged
cargo test -p airlock-evals --features model    # model, needs a key
```

Runtime selection by env, defaulting to stub:

```
MODEL_API_KEY=...          # already in .env.example; .env is gitignored
READER_MODE=stub | model   # default stub
```

`main.rs` already branches on `READER_URL` to pick `Remote`
(`crates/api/src/main.rs:26`). Same pattern, one more arm.

---

## 6. Order of work

1. **The eval seam** — agents as a parameter (`proposal.md` §1). Nothing
   else is safe without it.
2. **`Reader::Model`** behind the feature flag, output through
   `validate_reader_json`.
3. **The sanitiser eval** from §4.2.
4. **Run both suites and commit the diff.** Where does the model hold
   that the stub passed? Where does it *pass* that the stub held? The
   second list is the one that matters — a regression there is a missed
   scam.
5. **`Linker::Model` only if step 4 justifies it.** See §4.1.

Steps 1–3 are the whole job. Step 5 may never be worth doing.

---

## 7. How to demo it

The pipeline screen already has a node per agent. With a model behind the
Reader, two things become worth showing that are not worth showing now:

**Real latency.** The Reader node lights while an actual model call is in
flight. Currently it completes in microseconds and the animation is
theatre.

**The Linker's input, on screen.** This is the one. Add a panel showing
the exact payload sent to the Linker:

```
urgency               High
action                CallNumber
authority_claimed     true
confidence            High
amount                NotNamed
recipient             Matches
recipient_established false
minutes_since_contact 4
```

Then point at it and say: *"the attacker wrote a paragraph. This is
everything the second agent sees. There is no text in it, so there is
nothing to inject."*

That is a stronger multi-agent claim than any architecture diagram, and
it is true today — the Linker already receives exactly this. **We could
build that panel without touching the model at all.**

---

## 8. Other things that would make this stand out

Roughly in order of value per hour:

- **The Linker-input panel (§7).** Cheap, and it makes the strongest
  claim we have visible instead of asserted. Do this whether or not the
  model lands.
- **Rehearse beat six.** Two commands. Our closing beat has never been
  run against a real `airlock-reader` process. The wiring is there and
  the unit tests pass, but nobody has watched the node go dark.
- **Show the hold screen to a stranger.** Track C's own "done when" says
  *"a stranger reads the hold explanation and understands it."* Nobody
  has done this.
- **Grow the legitimate corpus.** The 10% false-positive figure is ten
  hand-written cases graded by the people who wrote the rules. Twenty
  honest cases would say more about the system than a better Reader on a
  small corpus does.
- **A latency number.** "Adds Nms to a transaction" is a question a judge
  will ask and we currently cannot answer. The tracing spans already
  record `latency_ms` per agent.
- **Fix the README's Rig line** if we do not use Rig. It currently
  promises something the tree does not contain, and being caught
  overclaiming costs more than the line is worth.

---

## 9. What we have to decide

1. **Model or no model before the demo?** If no, we lead with the
   architecture and say plainly that the agent layer runs offline today.
2. **Rig, or direct HTTP?** `reqwest` is already a dependency and adding
   nothing is the low-risk path — but the README promises Rig, so one of
   the two has to change.
3. **Model the Reader only, or both agents?** §4.1 argues Reader only.
4. **Does the model path ship as the demo default, or stay behind a
   flag?** Strong recommendation: **behind a flag.** Stub mode is the
   wifi fallback and the fail-closed story. The demo should run on the
   path we have rehearsed.

---

## 10. The honest framing, if we ship without a model

Worth writing down so all three of us say it the same way:

> The agent layer runs deterministic logic today. What is built is the
> thing that makes agents safe to put in a payment path: the Reader sees
> attacker text and has no power, the Linker has power and never sees
> text, and neither of them decides anything — the policy engine does.
> Swapping in a model is a variant on an enum. Making that swap *safe* is
> the part that took the work.

That is true, it is defensible, and it does not overclaim.
