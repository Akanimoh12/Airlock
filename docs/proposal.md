# Proposal — grading the model, not just the stub

**For:** whoever picks up the modelling stage (Track B)
**From:** Track C
**Status:** proposal, nothing implemented

Everything factual below was checked against the tree, not remembered.
File and line references are current as of writing.

---

## The short version

The eval harness is good and it already runs. It also **cannot grade a
model**, because it hard-codes `Reader::Stub` and `Linker::Stub`. Before
writing a prompt, make the agents a parameter. Otherwise the suite keeps
passing while telling you nothing about the thing you just built.

---

## What exists today

`cargo test -p airlock-evals` — 15 tests, all passing, no network, no key:

| File | Tests | Guards |
|---|---|---|
| `evals/tests/scams.rs` | 2 | Every scam in the corpus is held; named families all covered |
| `evals/tests/injection.rs` | 5 | A message telling the Reader to report "safe" changes nothing |
| `evals/tests/fail_closed.rs` | 6 | Dead, hanging, panicking, malformed Reader → `Unavailable`, never a verdict |
| `evals/tests/false_positives.rs` | 2 | Rate measured, `evals/measured.md` regenerated |

`evals/measured.md` is generated rather than hand-maintained. Current
figure: **1 of 10 legitimate transfers held — 10%**, the
`family-emergency-new-number` case. The file already states plainly that
ten hand-written cases graded by the people who wrote the rules is not a
real-world rate.

Three things about the current state that are easy to get wrong:

- **Rig is not a dependency.** No `rig` in any `Cargo.toml` in the
  workspace. The agent layer the README describes has not been started.
- **`Reader::Remote` is not a model.** It is the same stub logic in
  another process — `crates/reader/src/main.rs:38` calls
  `analyse_message`. It exists so demo beat six has a real PID to kill.
- **`Linker` has one variant.** `crates/agents/src/linker.rs:112` is
  `enum Linker { Stub }`. There is no seam for a second one yet.

So today's green suite means "the keyword matcher works" — which it
does, measurably. It says nothing about a model.

---

## The gap

`evals/src/lib.rs:77` and `:103`:

```rust
airlock_agents::screen_supervised(
    Reader::Stub,      // <- pinned
    Linker::Stub,      // <- pinned
    Untrusted::new(case.message.clone()),
    facts,
)
```

Both `run_case` and `verdict_for` pin the agents. Every one of the 15
tests therefore grades the stub, whatever else changes in the crate.

---

## Proposed change

### 1. Make the agents a parameter

Keep the existing signatures so no test has to change, and add the
injected variants beside them:

```rust
// evals/src/lib.rs

/// Run one case with a specific pair of agents.
pub async fn run_case_with(
    reader: Reader,
    linker: Linker,
    case: &Case,
) -> PolicyDecision { /* today's body, agents injected */ }

/// Screen one case with a specific pair of agents.
pub async fn verdict_for_with(
    reader: Reader,
    linker: Linker,
    case: &Case,
) -> ScreeningOutcome { /* likewise */ }

/// Stub-backed. Offline, no key — what CI runs.
pub async fn run_case(case: &Case) -> PolicyDecision {
    run_case_with(Reader::Stub, Linker::Stub, case).await
}

pub async fn verdict_for(case: &Case) -> ScreeningOutcome {
    verdict_for_with(Reader::Stub, Linker::Stub, case).await
}
```

That is the whole seam. Small, and it is the thing blocking everything
else.

### 2. Add the model variants

`Reader` already has the shape for it — `crates/agents/src/reader.rs:34`
is an enum with `Stub` and `Remote`. A third variant sits naturally
beside them. `Linker` needs the same treatment.

The important part is that **model output goes through the existing
validation**, not a new path. `validate_reader_json` already exists at
`crates/agents/src/validate.rs:211` and is what `Reader::Remote` uses.
The crate doc makes the reason explicit: stub and model are the same
code path, so the offline demo is not a different system. Keep that
true.

### 3. Gate the model evals behind a feature, not a default

The offline property is load-bearing twice over: it makes the suite a
pre-merge check rather than something remembered when a key is loaded,
and stub mode is the fallback if venue wifi dies. So:

```
cargo test -p airlock-evals                    # stub, offline, no key — unchanged
cargo test -p airlock-evals --features model   # model, needs a key
```

`cargo test` must never start requiring an API key.

---

## What this buys you

Not a pass or fail — **a diff**. The same corpus through both agents,
and the interesting output is the disagreements:

- Where does the model hold that the stub passed? Possibly better
  reading, possibly a new false positive.
- Where does the model pass that the stub held? That is the row to look
  at hardest. A regression here is a missed scam.
- What happens to the 10% false-positive rate?

Two suites become genuinely meaningful only after this change:

**Injection.** Right now `injection.rs` passes trivially — "ignore all
previous instructions" cannot talk a keyword matcher into anything.
Those five tests are real tests against a model and effectively free
tests against the stub. This is the suite that matters most, because it
is the claim the whole trust-boundary design rests on.

**False positives.** A model may read `"Mum is in hospital, please send
N20,000 to my new number"` differently from a keyword list. Whatever
happens, `measured.md` should be regenerated and the README figure kept
in step — the test that guards staleness already exists.

---

## Constraints that must not break

These come from `docs/team_brief.md` and are already enforced in code:

1. **Agents propose, code controls.** No model output moves money, sets
   a hold duration, or opens the chamber. The policy engine decides.
2. **Fail closed.** A model that is slow, down, or returns nonsense must
   produce `ScreeningOutcome::Unavailable`. `SCREENING_TIMEOUT` is 3s
   (`crates/agents/src/screening.rs:16`) and `screen_supervised` already
   guarantees it never returns an error — only an outcome.
3. **Raw text stops at the Reader.** `LinkerView`
   (`crates/agents/src/linker.rs:68`) contains no `String` and no variant
   carrying one. A model-backed Linker must receive the same projection.
   Do not add a field to carry "context" for the model.
4. **`PlainReason` stays an enum.** The product surface switches on the
   variant and renders our own copy. A model must never author what a
   user reads.
5. **Stub mode keeps working offline.** It is the demo fallback.

Point 3 is the one most likely to be eroded by accident. It is tempting
to hand the Linker a little more to work with, and `authority_claimed`
being a `bool` rather than the institution name is a deliberate choice —
the comment at `linker.rs:71` explains why. Carrying the name reopens the
text channel.

---

## Suggested order

1. The seam (§1). Nothing else is blocked once this lands, and it is
   independently reviewable.
2. Model-backed `Reader` behind the feature flag, output through
   `validate_reader_json`.
3. Run both suites, commit the diff, discuss what it shows.
4. Model-backed `Linker` only if the Reader diff justifies it — the
   Linker's job is narrow and structured, and the stub may already be
   the right answer.

---

## Open questions

- Which provider and model? The README names Rig as the abstraction, but
  the brief also says **no multiple model providers** — so pick one.
- Does a model-backed Reader need to beat the stub to ship, or is
  "different in explainable ways" enough for the demo?
- Beat six currently kills a process running stub logic. If the Reader
  becomes model-backed, does the demo still kill the process, or is a
  revoked key the more honest failure to show?
- The 10% figure is from ten hand-written cases. Is growing the corpus
  more valuable than swapping the Reader? An honest larger corpus may
  say more about the system than a better reader on a small one.

---

## One thing Track C could not verify

The fail-closed demo beat (beat six) is wired end to end on the product
surface — `/health.reader_reachable` drives the Reader node state and
`ScreenFailed` breaks its outgoing edge — but it has **never been run**
against a real `airlock-reader` process. In default stub mode the Reader
is in-process and infallible, so the node cannot go dark. Whoever runs
the two-process setup next should confirm it, since it is the closing
beat of the demo.
