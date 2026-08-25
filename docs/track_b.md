# Track B — agents, API, evals

Status and handoff. Companion to [team_brief.md](./team_brief.md), which
still governs. Read the two "for A" and "for C" sections even if you skip the
rest — they contain things you need to know about your own layer.

```bash
cargo test --workspace     # 89 tests, no network, no API key
cargo run -p airlock-api   # the whole backend, offline
```

---

## Done

- [x] **Stub mode ships first** — full flow, offline, no API key. `cargo run
      -p airlock-api` is the entire backend; C is not blocked and we have an
      offline path if the venue wifi dies.
- [x] **Injection evals pass** — behaviourally, and structurally (below).
- [x] **False-positive rate measured and written down** —
      [evals/measured.md](../evals/measured.md), regenerated on every test
      run so the number we quote can't go stale.

Also built: the Reader as a separate process, schema validation and
sanitisation at both trust boundaries, Axum + SSE, tracing spans per
transaction, and the four-file eval suite.

---

## The injection hole that was open, and how it is closed

The brief says the Linker "cannot be prompt-injected because it is never
shown attacker-controlled prose". That was not true of the frozen contract as
written. `PressureSignal` carries three attacker-influenceable free-text
fields:

```rust
authority_claim:  Option<String>
requested_action: RequestedAction::Other(String)
named_recipient:  Option<MaskedMsisdn>   // MaskedMsisdn(pub String)
```

The Reader reads attacker prose and fills those in. They then flow to the
Linker. A Reader that has been talked into cooperating had a clean channel to
the component that holds account context.

Two layers now close it, and the second is the one that generalises.

**Sanitisation** ([`agents/src/validate.rs`](../crates/agents/src/validate.rs)).
Every free-text field is trimmed, length-capped and allowlisted to
`[A-Za-z0-9 .&'-]`. No newlines, braces, angle brackets or backticks survive.
MSISDNs are re-masked in Rust rather than trusted. Failures **drop the
field** rather than rejecting the signal — both are fail-closed, but dropping
doesn't turn every unusual honest message into a hold. What was dropped is
recorded in a `SanitisationReport` the evals assert on.

**Projection** ([`agents/src/linker.rs`](../crates/agents/src/linker.rs)).
The Linker never receives `PressureSignal`. It receives `LinkerView`, which
**contains no `String` and no variant carrying one** — every field is an
enum, a bool or an integer. Whether the message named this transfer's amount
or recipient is computed in Rust and passed as `FactMatch::{Matches, Differs,
NotNamed}`. `authority_claim` becomes `authority_claimed: bool`; *which*
institution was claimed is not needed to judge responsiveness and carrying it
would reopen the channel.

`the_linker_never_receives_free_text` stuffs every field a compromised Reader
controls with an attack payload, projects, serialises, and asserts none of it
appears. That is a property of the type, not a sample of payloads. The
matching result is that a Reader lying the *other* way — claiming a match
that isn't there — can only cause a hold. There is no lie that produces a
pass.

---

## For A

**One line changed in your crate.** `Untrusted<T>` had no accessor, so the
Reader could not read the text it exists to read — `validate()` is
synchronous and the Reader hop is `async`. I added
`Untrusted::expose_to_reader(&self) -> &T`
([trust.rs](../crates/core/src/trust.rs)), named the long way so call sites
are auditable, borrow-only, no owned equivalent. It has exactly one caller.
Say if you want it shaped differently.

**`screen_with_timeout` is currently unused, and I think its signature is the
reason.** It takes `F: Future<Output = Verdict>`, which has nowhere to put
"the Reader socket was refused". The only `Verdict` available for a failure
is `Unknown` — and `decide` **passes** on `Unknown`, so mapping errors onto
it would put a fail-open path through component death. I wrote
`supervised_screen` in `agents/src/screening.rs`, which is your function
widened to `Result<Verdict, E>` and otherwise identical. Roughly eight
duplicated lines. **Suggest widening yours and deleting mine** — it belongs
in `runtime`, not in the agent layer. Your call; it's your crate.
`failure_never_produces_unknown_which_would_pass` guards the reasoning either
way.

**Two smaller things.** `Responsiveness.rationale` is dead weight — `decide`
takes only `Verdict` and picks its own `PlainReason`, so the Linker's
rationale never reaches anything. Harmless, and I fill it in consistently,
but the model can only meaningfully emit `verdict`. And
`Validated::from_trusted_source` is an open bypass that the compiler cannot
protect: model output must never go through it. Discipline plus tests is all
we have there.

---

## For C

Backend on `:8080`. CORS is permissive.

### Endpoints

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/inbound-sms` | `{"text": "..."}` — the judge-facing scam sender. |
| `POST` | `/transfers` | `{"recipient": "08031234567", "amount_minor": 500000, "currency": "NGN"}` → `TxnView`. Runs the whole flow. |
| `GET` | `/transactions` | Snapshot, newest first. |
| `GET` | `/transactions/{id}` | One transaction. |
| `POST` | `/transactions/{id}/release` | 409 with `{"error": "..."}` if the cooling period hasn't elapsed. |
| `POST` | `/transactions/{id}/cancel` | Available immediately — no reason to make someone wait to *not* send money. |
| `GET` | `/events` | SSE. |
| `GET` | `/health` | Includes `reader_reachable` — goes false when beat six kills the Reader. |

### `TxnView`

```json
{
  "id": 1,
  "state": "Held",
  "amount": { "minor_units": 500000, "currency": "NGN" },
  "recipient": "*******567",
  "recipient_established": false,
  "proposed_at": "2026-08-25T01:25:14Z",
  "releases_at": "2026-08-25T01:26:14Z",
  "reason": "NovelRecipientUnsolicitedContact",
  "releasable": false
}
```

`amount.currency` is a string, not the `[78,71,78]` that `Money`'s `[u8; 3]`
would otherwise serialise to. `recipient` is always masked; the full number
never leaves the server, and there's a test asserting it.

### Events

Exactly A's frozen `AirlockEvent`, tagged with `type`:

```json
{"type":"StateChanged","txn":3,"from":"Proposed","to":"Screening"}
{"type":"ScreenFailed","txn":3,"component":"Reader"}
{"type":"HoldOpened","txn":3,"reason":"ScreeningUnavailable","releases_at":"..."}
```

**Four things you need to know.**

*Fetch `/transactions` when you connect.* SSE only carries what happens
after you subscribe, and `AirlockEvent` has no snapshot variant. Connecting
mid-flight would otherwise leave you blank until the next event.

*`ScreenFailed` can be followed by a pass.* With the Reader dead, an
established recipient still emits `ScreenFailed` and then clears — screening
genuinely failed, and rule 1 passed the transfer anyway. Both are true. Put
it in the technical panel, not the wallet view, or the main screen will
contradict itself during beat six.

*The countdown.* Render it from `releases_at`, which is a server timestamp —
that is not a frontend timer inventing state. But whether release is
*permitted* is `releasable`, and the server re-checks it on the release call
regardless of what you believe, so a client that thinks the timer has run out
doesn't get to be right about it. There is no event when the cooling period
elapses; adding one is a contract change needing all three of us. Say if you
want it.

*`reason` is a key, not copy.* The five variants are
`EstablishedRecipient`, `NovelRecipientUnsolicitedContact`,
`ScreeningUnavailable`, `UserReleased`, `CoolingPeriodNotElapsed`. Switch on
them; never display them.

### Seeded recipients

Established (pass on rule 1, whatever the message): `08055512345` landlord,
`08099987654` airtime, `08033344556` sister. Anything else is novel. Format
doesn't matter — `+234…` and `0…` compare equal.

---

## Open questions for the three of us

1. **The correlation window and hold duration are still placeholders** —
   10 minutes and 60 seconds, from the brief's own open items. Everything
   is built against them. Do we ship these numbers?
2. **Should `Verdict::Unknown` on a novel recipient inside the window
   hold?** Today it passes: rule 2 requires `Responsive`. That is the frozen
   rule and I built to it, but it means model uncertainty resolves toward
   passing while model failure resolves toward holding. Deliberate?
3. **Should an established recipient be screened at all?** Rule 1 passes
   them regardless, so we ship the user's message text to the Reader for a
   transfer that was always going to clear. I did *not* short-circuit it —
   that would put a copy of rule 1 in my layer and silently break if you
   ever change it — but the data-minimisation argument is real.
4. **`README.md` "Roadmap" claims the demo UI is built.** It isn't yet;
   `web/` is still a stub. Worth fixing before a judge reads it.

---

## What I did not build

Per the brief's "not building" list: no Rig, no model provider, no RAG, no
deployment infra, no user accounts. The Reader is deterministic keyword
matching — good enough for the demo and the corpus, and honest enough that
its false positives show up in the measured rate instead of being tuned away.
Dropping a model-backed Reader in behind `Reader::Remote` changes nothing
downstream: same validation, same projection, same evals.
