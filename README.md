# Airlock

**Your bank checks that it's really you. Airlock checks whether it was really your idea.**

A transaction guard for mobile money and digital wallets that stops authorized-push-payment fraud — the scams where the victim sends the money themselves.

Nothing passes straight from outside to inside. It waits in the chamber first.

> **Status:** in active development for Swarm Village. See [Roadmap](#roadmap) for what is and isn't built.

---

## The problem

Your phone buzzes: *"MTN Alert: your account will be suspended today. Call this number to reactivate."*

You call. A polite person walks you through some steps. At the end, **you** send the money. Your phone, your PIN, your fingerprint.

Every fraud control in the stack is asking one question — *is this really you?* — and the answer is yes. So every check passes correctly and the money is gone.

Africa loses over **$4 billion a year** to mobile money fraud. Continental cybercrime losses rose from $192M in 2024 to $484M in 2025. Nigeria saw a 300% increase in confirmed SIM-swap cases between 2022 and 2024; Kenya identified 123,000 fraudulent SIM cards in 2025. 97% of countries surveyed by INTERPOL name mobile money fraud a major threat.

The locks all work. The person holding the key was tricked.

## The insight

Airlock asks a different question:

> Not *"is this really you?"* — but *"whose idea was this?"*

Paying your landlord, buying airtime, sending money to family the way you do every month — your idea. Straight through, no interruption.

But a first-time recipient, four minutes after an unsolicited message arrived, in an amount that message named — that wasn't your idea. Someone put it there.

So Airlock holds it in the chamber for sixty seconds. These scams run on manufactured urgency; the caller is *hurrying you*. Remove the hurry and most of them collapse.

**A hold is not a block.** The user can always release it. A minute of inconvenience against a drained wallet is an easy trade.

---

## How it works

```
  inbound SMS / call transcript          transfer request
        (UNTRUSTED)                       (user-authorized)
             │                                   │
             ▼                                   │
    ┌──────────────────┐                         │
    │  Reader agent    │  no account access      │
    │                  │  cannot move money      │
    └────────┬─────────┘                         │
             │ typed PressureSignal              │
             │ (raw text stops here)             │
             ▼                                   ▼
    ┌────────────────────────────────────────────────┐
    │              Linker agent                      │
    │  "is this transfer responsive to that contact?" │
    │  sees typed signal + account facts              │
    │  never sees raw message text                    │
    └────────────────────┬───────────────────────────┘
                         │ Responsiveness verdict
                         ▼
    ┌────────────────────────────────────────────────┐
    │        Policy engine — pure Rust, no model      │
    │  owns: hold decision, duration, state, release  │
    └────────────────────┬───────────────────────────┘
                         │
              ┌──────────┴──────────┐
              ▼                     ▼
            PASS                  HOLD
                              (60s + explanation)
```

### Agents propose. Code controls.

| Concern | Owner |
|---|---|
| Understanding what a message is pressuring you to do | Agent |
| Judging whether a transfer answers that pressure | Agent |
| Whether to hold | **Deterministic policy** |
| How long the hold lasts | **Deterministic policy** |
| Transaction state transitions | **Deterministic policy** |
| Releasing a held transfer | **User only** |
| Moving money | **Deterministic policy** |

No model output ever moves money, and no model can open the chamber.

### Trust boundaries

Two separate processes, and neither can complete the attack alone.

The **Reader** handles untrusted content. It is good at language and has zero access to accounts or funds. A message crafted to manipulate it is talking to something with no power.

The **Linker** has account context but never receives raw message text — only a typed, schema-validated signal. It cannot be prompt-injected because it is never shown attacker-controlled prose.

```rust
pub enum Source { InboundSms, InboundCall, User, AccountHistory, System }

pub struct Evidence<T> {
    pub source: Source,
    pub received_at: DateTime<Utc>,
    pub payload: T,
}

pub struct Untrusted<T>(T);   // cannot reach the policy engine
pub struct Validated<T>(T);   // schema-checked, provenance-tagged
```

Invalid states are unrepresentable: there is no constructor that produces a `Validated<PressureSignal>` from raw inbound text without passing schema validation.

### Transaction lifecycle

```
Proposed ──▶ Screening ──▶ Cleared ──▶ Executed
                 │
                 ├──▶ Held ──▶ Released ──▶ Executed
                 │       │
                 │       └──▶ Cancelled
                 │
                 └──▶ Held  (fail-closed: screening unavailable
                             + novel recipient)
```

**Fail-closed by design.** If the Reader crashes, times out, or returns malformed output, a transfer to a first-time recipient defaults to `Held` — never to `Cleared`. Component death degrades toward safety, never toward silent approval. An airlock that loses power seals; it does not open both doors.

### Policy rules

Deterministic, auditable, no model involvement:

1. Recipient with established payment history → **pass**, regardless of any message.
2. Novel recipient + unsolicited inbound contact inside the correlation window + responsive verdict → **hold**.
3. Screening unavailable + novel recipient → **hold** (fail-closed).
4. Hold duration is fixed in code. The model cannot shorten it.
5. Release requires explicit user action after the cooling period elapses.

---

## Why Rust

- Sits in the hot path of every transaction — cannot crash, cannot add latency.
- The transaction lifecycle is a typed state machine where invalid transitions don't compile.
- The trust lattice is a type-system problem, not a convention to remember.
- Long-running supervised workers with timers that must survive restarts without double-releasing a held transfer.

## Stack

- **Tokio** — async runtime, timers, supervision
- **Axum** — HTTP API and SSE event stream to the UI
- **Serde** — schema-validated agent output at every trust boundary
- **tracing** — structured spans per transaction
- **thiserror** — typed errors
- **Rig** — agent and model abstraction (confined to the agent layer)

Rig implements the agent layer only. Policy, state and reliability logic are plain Rust with no model dependency, and are tested without any network access.

---

## Getting started

```bash
git clone <repo> && cd airlock
cp .env.example .env         # add your model API key
cargo run -p airlock-api     # backend on :8080
```

Then open the demo UI:

```bash
cd web && npm install && npm run dev
```

Run the deterministic core with no model calls at all:

```bash
cargo test -p airlock-policy -p airlock-core
```

## Layout

```
airlock/
├── crates/
│   ├── core/        types, trust lattice, transaction state machine
│   ├── policy/      deterministic rules — no model calls, fully tested
│   ├── agents/      Reader and Linker (Rig)
│   ├── runtime/     orchestration, supervision, hold timers
│   └── api/         Axum HTTP + SSE
├── web/             demo UI
├── evals/           red-team corpus and adversarial cases
└── fixtures/        real-world scam message samples
```

## Observability

Every transaction is one span, with child spans for screening, each agent call, and the policy decision. Fields are stable and low-cardinality — `txn_id`, `agent`, `verdict`, `latency_ms`, `outcome`. Message content is never written to traces.

The demo UI renders real backend events over SSE. Nothing in the interface is driven by a frontend timer.

## Evals

`cargo test -p airlock-evals` runs the adversarial suite:

- scam corpora across common local variants — fake telco agent, wrong-transfer refund, prize release fee, fake loan officer
- **prompt injection**: messages instructing the Reader to report the transfer as safe
- malformed and truncated agent output
- Reader unavailable → must fail closed
- **legitimate-transfer corpus** — measures the false-positive hold rate, reported honestly

---

## Demo

Two minutes, no props, no camera, no external data dependency.

1. Show the message. Everyone in the room has received it.
2. **A judge sends it themselves**, live. Nothing about the attack is pre-recorded.
3. Attempt the transfer for real — PIN and all.
4. Held, with a plain-language explanation of *why*.
5. **The honest beat:** a genuine family emergency transfer to a new number is also held. Show the false positive; show the one-tap release.
6. **The failure beat:** kill the Reader process. The transfer still holds instead of passing. Fail-closed, live.

The failure in step 6 is deliberately triggered. The system's response to it is real — real process death, real supervisor detection, real state transition.

## What Airlock is not

- Not spam detection. The message being a scam is not the finding; the *payment being caused by it* is.
- Not a block. Every hold is releasable by the account holder.
- Not a replacement for authentication. It runs after authentication succeeds, which is exactly when this fraud happens.
- Not a claim to catch everything. It targets one specific, enormous, currently-unaddressed class: fraud the victim authorizes.

## Known limitations

- Holds legitimate first-time transfers. Measured, reported, and mitigated with a one-tap release rather than hidden.
- Requires visibility into inbound messages — a control a wallet, bank or telco can run, not one a third party can bolt on.
- A patient attacker who waits out the correlation window defeats the recency signal. Novelty and responsiveness still apply; the window is defense in depth, not the whole defense.

## Roadmap

**Built:** transaction state machine, policy engine, trust types, Reader and Linker, supervision and fail-closed behavior, demo UI, eval suite.

**Postponed:** voice-call transcription, SIM-swap correlation, multi-tenant deployment, telco integration, offline USSD path, per-user learned baselines.

## License

MIT
