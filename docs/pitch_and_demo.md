# Pitch and demo

The concept, the words, and the runbook. Companion to
[team_brief.md](./team_brief.md).

Whoever narrates should read this end to end once, then rehearse from
[The demo](#the-demo) alone.

---

## The one line

> Your bank checks that it's really you. Airlock checks whether it was
> really your idea.

All three of us say it identically. If a judge hears three versions, they
hear three products.

---

## The concept, properly

### The category error

Every fraud control in a payment stack answers one question: **is the person
authorizing this the account holder?** PIN, fingerprint, OTP, device binding,
SIM binding — all of it is authentication, and all of it works.

Authorized push payment fraud walks around it. The victim gets a message,
calls a number, speaks to someone convincing, and *sends the money
themselves*. Their phone. Their PIN. Their fingerprint.

So every control is asked "is this really you?", and every control correctly
answers **yes**. Nothing malfunctions. The money is gone and no system did
anything wrong.

That is not a gap in authentication. It is a question authentication cannot
be made to answer, because the fraud happens *after* authentication succeeds
— which is exactly why adding more of it does nothing.

### The reframe

Airlock asks a different question:

> Not *"is this really you?"* — but *"whose idea was this?"*

You can't read minds. But a payment someone else authored has a causal
signature, and it has three parts:

| Signal | Why it matters |
|---|---|
| **Novelty** | You have never paid this person before. Scams need fresh mule accounts; your landlord is not one. |
| **Recency** | An unsolicited message or call arrived shortly before. |
| **Responsiveness** | The transfer *answers* what that contact was pressuring you to do. |

Any one alone is noise. Your first payment to a new plumber is novel. A
marketing SMS is recent. Together, and only together, they describe a payment
that was placed in your head from outside.

### Why a sixty-second hold, and not a block

Here is the part that makes the whole thing work.

**These scams run on urgency.** The caller is hurrying you — "act now",
"before it's disconnected", "stay on the line". That is not decoration; it is
load-bearing. If you stop and think, or call your bank, or mention it to
someone in the room, the scam collapses. Speed is the attack's engine.

So the countermeasure is not to block the payment. It is to **take the hurry
away**. Sixty seconds in the chamber, with a plain explanation of why.

The payoff is asymmetric, which is the whole argument:

- **Wrong?** Someone waits a minute for a payment that was fine.
- **Right?** The attack loses the one thing it cannot function without.

A minute of inconvenience against a drained wallet is an easy trade, and
crucially it is a trade the user can always decline — **every hold is
releasable by the account holder.** A hold is not a block.

### Why this is an AI safety problem, not just a fraud problem

To judge "was this your idea", something has to understand the message. That
means a language model. Which means putting a model in the payment path whose
**input is written by the attacker**.

That is the worst position in security: hostile input reaching a component
near money. So the architecture is built so that *fooling the model doesn't
help you*.

```
inbound SMS (UNTRUSTED)          transfer request (user-authorized)
        │                                    │
        ▼                                    │
   ┌─────────┐  sees the prose               │
   │ Reader  │  no account access            │
   │         │  cannot move money            │
   └────┬────┘  its own OS process           │
        │ typed PressureSignal               │
        │ raw text stops here                │
        ▼                                    ▼
   ┌──────────────────────────────────────────┐
   │ Linker — has account facts               │
   │ never receives prose, structurally       │
   └───────────────────┬──────────────────────┘
                       │ Responsiveness verdict
                       ▼
   ┌──────────────────────────────────────────┐
   │ Policy engine — pure Rust, no model      │
   │ owns: hold, duration, state, release     │
   └──────────────────────────────────────────┘
```

Three properties, and each is enforced by code rather than promised in a
doc:

**The component that reads attacker text has no power.** The Reader has no
ledger, no account access, no ability to move money. Compromise it completely
and you have compromised something that can only return a typed struct.

**The component with power never sees attacker text.** The Linker receives
`LinkerView`, a type that *cannot contain a string* — every field is an enum,
a bool or an integer. Whether the message named this amount or this recipient
is computed in Rust and handed over as a match/differs/not-named enum. There
is no channel for prose to arrive on. That is a property of the type, not a
filter that might miss something.

**No model decides anything.** Agents propose; code controls. Whether to
hold, how long, every state transition, and who may release — all
deterministic Rust with no model dependency, tested with the network
unplugged.

### Fail closed

If the Reader crashes, times out, or returns garbage, a transfer to a
first-time recipient defaults to **held** — never to cleared.

> An airlock that loses power seals. It does not open both doors.

This is why the name is right, and it is the beat we demonstrate live rather
than assert.

### What Airlock is not

Have these ready; they are the four objections that arrive every time.

- **Not spam detection.** The message being a scam is not the finding. The
  *payment being caused by it* is. A scam message that produces no payment
  does nothing here.
- **Not a block.** Every hold is releasable by the account holder.
- **Not a replacement for authentication.** It runs after authentication
  succeeds, which is precisely when this fraud happens.
- **Not a claim to catch everything.** One specific, enormous, currently
  unaddressed class: fraud the victim authorizes.

### The market, in four numbers

- Africa loses **over $4 billion a year** to mobile money fraud.
- Continental cybercrime losses went **$192M (2024) → $484M (2025)**.
- Nigeria: **300% increase** in confirmed SIM-swap cases, 2022–2024.
- **97%** of countries INTERPOL surveyed name mobile money fraud a major
  threat.

**Who deploys it:** a wallet, a bank, or a telco — anyone who already sees
both the inbound messages and the outbound payments. This is a control an
operator runs, not one a third party bolts on. Say that before a judge asks;
it reads as clarity rather than a dodge.

---

## The pitch

### Thirty seconds

> Your phone buzzes: *"MTN Alert — your line will be suspended today. Call
> this number."* You call. Someone polite walks you through some steps. At
> the end, **you** send the money. Your phone, your PIN, your fingerprint.
>
> Every fraud control in the stack asks *is this really you* — and the answer
> is yes. So every check passes correctly and the money is gone. Africa loses
> four billion dollars a year this way.
>
> Airlock asks the question nobody asks: *whose idea was this?* And when the
> answer is "someone who messaged you four minutes ago", it holds the
> transfer for sixty seconds. These scams run on hurry. Take away the hurry
> and most of them collapse.

### Two minutes

**The problem — 30s.** As above. Land these three beats in order: everyone in
the room has received this message; the victim sends the money themselves;
every control passes *correctly*. The last one is the insight — do not rush
past it. The locks all work. The person holding the key was tricked.

**The insight — 30s.**

> Paying your landlord, buying airtime, sending money to family the way you do
> every month — that's your idea. Straight through, no interruption.
>
> But a first-time recipient, four minutes after an unsolicited message, in
> an amount that message named — that wasn't your idea. Someone put it there.
>
> So we hold it in the chamber for sixty seconds. Not block. Hold. You can
> always release it.

**How it works — 45s.** Two agents, and neither can complete the attack
alone. The Reader sees the message and has no account access — a message
crafted to manipulate it is talking to something with no power. The Linker
has your account context and *never receives the message text*, so there is
nothing to inject it with. And no model decides anything: whether to hold,
how long, and who can release are deterministic code.

> Agents propose. Code controls.

**Why us — 15s.** It is in Rust because it sits in the hot path of every
transaction and cannot crash or add latency; the transaction lifecycle is a
typed state machine where illegal transitions don't compile; and the trust
boundary is a type-system problem rather than a rule someone has to remember.

Then go to the demo.

---

## The demo

Two minutes. No props, no camera, no external data dependency.

### Roles

Three people, and **the narrator does not touch the laptop.** Someone
narrating while typing does both badly.

| Role | Does |
|---|---|
| **Narrator** | Talks. Never touches the keyboard. |
| **Driver** | Operates the laptop and the kill terminal. Silent. |
| **Judge** | Sends the scam message themselves, in beat 2. |

### Pre-flight

Three terminals, run in this order:

```bash
cargo run -p airlock-reader                                  # :8081, prints its PID
READER_URL=http://127.0.0.1:8081 cargo run -p airlock-api    # :8080
cd web && npm run dev                                        # the UI
```

Checklist before you start:

- [ ] Reader terminal visible on screen, or its PID written down. **Beat six
      needs it.**
- [ ] Connection pill in the top bar reads connected, not "connecting".
- [ ] Browser on the **Pipeline** tab.
- [ ] Wallet shows the seeded history — landlord, airtime, sister. It should
      look lived-in, not empty.
- [ ] Terminal font large enough for the back row.

The three established recipients are `08055512345` (landlord), `08099987654`
(airtime), `08033344556` (sister). **Anything else is a first-time
recipient.** Use `08031234567` as the scam number.

### The timing problem, and how the script solves it

A hold is a real sixty seconds. You cannot stand in silence waiting for it,
and you must not fake it.

So the honest beat's hold is **started early and released late**. Beat 3
opens a hold at roughly 0:25; the fail-closed beat fills the next minute with
something worth watching; you come back at 1:30 and release it for real. The
countdown on screen is genuine the whole time.

Rehearse with a stopwatch. The sixty seconds is the one thing you cannot
talk faster to fix.

### Six beats

**1 · The message — 0:00**
*Pipeline tab.*

> This is the message. Everyone in this room has received a version of it.

Read it aloud. Do not explain the architecture yet — the diagram is a frame
for what is about to happen, not the point.

---

**2 · A judge sends it — 0:10**
*Send a message tab. Hand the laptop to a judge.*

> Would you send it? Type it yourself.

**This is the beat that buys credibility.** Nothing about the attack is
pre-recorded, so nothing about the save can be doubted. Let them type. Do not
narrate over it.

---

**3 · The transfer is attempted — 0:25**
*Wallet tab. Driver enters `08031234567` and the amount from the message.*

> Now the part that matters — this goes through properly. PIN and all. Every
> check passes, because it really is her.

Watch it hold. **Hold #1 starts here — this is the one released in beat 5.**

---

**4 · Held, and why — 0:35**

> Held. Sixty seconds. And it says why, in words: a first-time recipient,
> minutes after a message she didn't ask for, for what that message asked
> for.
>
> Not blocked. She can release it. But not while someone is on the phone
> telling her to hurry.

Point at the countdown. It is real, and it is running.

---

**5a · The honest beat, opened — 0:45**
*Wallet tab. A genuine family emergency to a new number.*

> Here's where we're honest with you. This one is real — a family emergency,
> a number she's never sent to before. And we hold it too.
>
> That's a false positive, and we measure it rather than hide it. It costs
> her a minute. Being wrong the other way costs her the wallet.

Leave it held. You come back to it.

---

**6 · The failure beat — 1:00**
*Driver kills the Reader process on stage. Show the terminal.*

> This is the process that reads messages. I'm going to kill it.

Kill it. Show the connection state change in the UI.

> Now attempt a transfer to a first-time recipient with the thing that reads
> messages *dead*.

It holds — reason: screening unavailable.

> It could have failed open and let it through. It seals instead. An airlock
> that loses power seals; it does not open both doors.
>
> We triggered that failure on purpose. Everything after it was real — real
> process death, real supervisor detection, real state transition.

**That distinction is the whole reason the demo is worth anything.** Say it
in those words.

---

**5b · The honest beat, closed — 1:30**
*Back to the wallet. Hold #1's sixty seconds have elapsed.*

> And here's her transfer from a minute ago. The wait is over — one tap.

Release it. It executes.

> That's the trade. A minute when we're wrong. The whole wallet when we're
> right.

---

**Close — 1:50**

> Every fraud control in the world asks whether it's really you. Airlock asks
> whether it was really your idea. That's the question nobody was asking, and
> it's the one this fraud lives inside.

### If something breaks

You are on stage in front of people. Have these ready.

| Breaks | Do |
|---|---|
| Reader won't restart after beat 6 | You don't need it. Everything still holds — that *is* the point. Say so and continue. |
| The UI stops updating | Reload. State is server-side; the page refetches a snapshot on connect. Nothing is lost. |
| A hold passes when it should hold | Check the recipient isn't one of the three seeded numbers. Rule one passes established recipients regardless — that is correct behaviour, and worth saying out loud if it happens. |
| Anything else | Say what broke and what should have happened. Rule three cuts both ways: never fake the recovery. A team that explains its own failure reads as competent. |

---

## Q&A

The nine questions that actually get asked.

**"Isn't this just spam detection?"**
No. We don't act on messages, we act on payments. The message being a scam is
not the finding — the payment being *caused by it* is. A scam message that
produces no payment does nothing in our system.

**"What about false positives?"**
Real, measured, and we put one on stage rather than hide it. Legitimate
first-time transfers do get held. The mitigation is that a hold costs a
minute and is releasable in one tap — and rule one means ordinary traffic to
people you've paid before never touches this path at all.

**"Can't the attacker just wait out the window?"**
Yes, and we say so in the README. Novelty and responsiveness still apply —
the recency window is defence in depth, not the whole defence. It also isn't
free for the attacker: these operations run on volume and speed, and patience
costs them both.

**"What if the model is jailbroken?"**
Then nothing happens, and that's by construction. The component that reads
attacker text has no account access and cannot move money. The component with
account context never receives text — the type it takes cannot hold a string.
And no model output moves money, sets a hold duration, or opens a hold.

**"Sixty seconds — is that enough?"**
It is calibrated to the attack, not to the amount. The scam needs you rushed
and on the phone; sixty seconds is enough to break that and cheap enough not
to hurt. It is a constant in code, tunable per institution.

**"Won't people just release it anyway?"**
Some will. But the hold interrupts the script the scammer is running, and it
turns an invisible loss into an informed choice. We are not claiming to save
everyone — we're claiming to remove the conditions this fraud needs.

**"How is this different from existing fraud controls?"**
Every one of them asks *is this really you*, and in this fraud the honest
answer is yes. We run after authentication succeeds, which is exactly when
this happens.

**"Do you read people's messages?"**
The Reader runs inside the operator's own boundary, message content is never
written to traces — length only — and a recipient's full number never leaves
the server. This is a control a wallet, bank or telco runs on infrastructure
they already have, which is also why a third party can't bolt it on.

**"Why Rust?"**
It sits in the hot path of every transaction, so it cannot crash and cannot
add latency. The transaction lifecycle is a typed state machine where illegal
transitions don't compile. And the trust boundary is a type-system problem
rather than a convention someone has to remember.
