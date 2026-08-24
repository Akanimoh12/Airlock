# Airlock — Team Brief

How the three of us build this. Read all of it before writing code.

For what Airlock is and why, see [README.md](./README.md).

---

## The one line we all say the same way

> Your bank checks that it's really you. Airlock checks whether it was really your idea.

If all three of us can say that identically, our pitch is consistent no matter who fields the question.

Short version of the rest: someone gets a scam message, calls the number, and sends the money themselves — their phone, their PIN, their fingerprint. Every existing fraud control asks *is this really you?* and correctly answers yes. Africa loses over **$4 billion a year** this way. Airlock asks the question nobody asks — *whose idea was this?* — and holds the transfer for sixty seconds when the answer is "someone who messaged you four minutes ago."

---

## Rules that bind all three of us

Not negotiable. Breaking one breaks the demo's honesty.

**1. Agents propose. Code controls.**
No model output ever moves money, sets a hold duration, or opens the chamber. The model reads and judges; Rust decides.

**2. Fail closed, always.**
An airlock that loses power seals — it does not open both doors. If screening dies, times out, or returns garbage, a novel-recipient transfer holds.

**3. Never fake backend behavior.**
Triggering a failure on purpose is fine. Faking the recovery is not. Every state change on screen happened for real.

**4. The UI shows real backend state.**
Everything on screen comes from a server event. No frontend timers inventing progress that isn't happening.

**5. New scope costs old scope.**
Nothing gets added unless something gets cut. Say what you're cutting when you propose it.

---

## Freeze these first

Four types, agreed together before anyone writes implementation code. These are the seams between our three layers — write them into `crates/core` in one sitting, then build against them in parallel.

Changing one afterwards requires all three of us. A silent change here is what makes integration fail at the worst moment.

```rust
// A owns these. B and C build against them.

pub struct PressureSignal {          // Reader output — raw text stops here
    urgency:          Urgency,
    authority_claim:  Option<String>,
    requested_action: RequestedAction,
    named_amount:     Option<Money>,
    named_recipient:  Option<MaskedMsisdn>,
    confidence:       Confidence,
}

pub struct Responsiveness {          // Linker output
    verdict:   Verdict,              // Responsive | Unrelated | Unknown
    rationale: PlainReason,          // enum, not free text — C renders it
}

pub enum TransactionState {
    Proposed, Screening, Cleared, Held,
    Released, Cancelled, Executed,
}

pub enum AirlockEvent {              // what SSE emits, what C renders
    StateChanged { txn: TxnId, from: TransactionState, to: TransactionState },
    HoldOpened   { txn: TxnId, reason: PlainReason, releases_at: Timestamp },
    ScreenFailed { txn: TxnId, component: Component },
}
```

`PlainReason` is an enum, not a string. That is deliberate: the model can never write what the user reads, C can style each variant properly instead of dumping model prose on screen, and attacker-controlled text cannot reach the display.

---

## Who owns what

### A — Deterministic core
`crates/core` · `crates/policy` · `crates/runtime`

**Owns.** Trust types, the transaction state machine, the five policy rules, hold timers, worker supervision and fail-closed behavior.

**Done when.**
- [ ] Every policy rule has a test
- [ ] Invalid state transitions don't compile
- [ ] A test proves a dead Reader produces `Held`
- [ ] All of it runs with no network and no API key

**Never.** Call a model from `core` or `policy`. Not once, not for convenience. These crates must compile and pass with the network unplugged — that property is what makes the demo's fail-closed beat honest.

**Your risk.** Over-building the type system before the five rules exist. Ship the rules first, make them elegant second.

### B — Agents, API and evals
`crates/agents` · `crates/api` · `evals`

**Owns.** Reader and Linker via Rig, schema validation at both trust boundaries, Axum HTTP and the SSE stream, tracing spans, and the eval suite.

**Done when.**
- [ ] **Stub mode ships first** — full flow, offline, no API key
- [ ] Injection evals pass: a message telling the Reader to report "safe" changes nothing
- [ ] False-positive hold rate measured on the legitimate-transfer corpus and written down

**Never.** Let raw message text reach the Linker — it gets the typed signal only. Never let unvalidated model output reach the policy engine. Both agents stay least-privileged: the Reader has no account access, the Linker never sees attacker prose.

**Your risk.** Polishing prompts before the pipeline runs end to end. Stub mode first — it unblocks C and becomes our offline fallback if the venue wifi dies.

### C — Product surface and demo
`web/` · the two-minute script

**Owns.** The wallet screen, the judge-facing scam-sender screen, SSE rendering, and the demo script and rehearsal.

**Done when.**
- [ ] A stranger reads the hold explanation and understands it
- [ ] The fail-closed moment is visible on screen, not in a log
- [ ] The script has been rehearsed end to end with a deliberate break

**Never.** Invent state with a frontend timer. Show internal jargon by default — the user sees *"paused for a moment"*, never `Held` or `PressureSignal`. Technical detail lives behind an expandable panel.

**Your risk.** Building an engineering dashboard instead of a product. A judge should see a wallet they recognise, not our architecture.

---

## How the three of us stay joined up

Integration is the risk, not the work.

**Freeze the contracts together, first.** One sitting, before implementation. Everything after that runs in parallel.

**B ships stub mode early.** C is never blocked waiting on agents, and we gain an offline demo path we can fall back to on stage.

**Integrate end to end while it's still ugly.** A rough full path beats three polished layers that meet for the first time at the end. That meeting is where hackathon projects die.

**Then everyone converges on the demo.** Stop splitting. Rehearse the flow, break it on purpose, fix what breaks, rehearse again. Whoever narrates is not the person driving the laptop.

---

## The demo

Six beats. No props, no camera, no external data dependency.

1. Show the message. Everyone in the room has received one.
2. **A judge sends it themselves**, live. Nothing about the attack is pre-recorded, so nothing about the save can be doubted.
3. Attempt the transfer for real — PIN and all.
4. Held, with a plain-language explanation of why.
5. **The honest beat.** A genuine family emergency transfer to a new number is also held. Show the false positive; show the one-tap release. Volunteering the limitation is the cheapest credibility available.
6. **The failure beat.** Kill the Reader process on stage. The transfer still holds instead of passing. Fail-closed, live.

The failure in beat six is deliberately triggered. The system's response to it is real — real process death, real supervisor detection, real state transition. That distinction is the whole reason this demo is worth anything.

---

## Not building

Say no to these by default:

smart contracts · voice transcription · SIM-swap correlation · multi-tenant · telco integration · offline USSD · learned per-user baselines · user accounts · multiple model providers · RAG · deployment infra

---

## Open items

- [ ] Trademark and name-collision check on "Airlock" against African fintech registries
- [ ] Agree the correlation window and hold duration as concrete numbers
- [ ] Decide who narrates and who drives the laptop