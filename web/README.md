# web/

Demo UI — Track C. Vite + TypeScript, no framework.

```bash
cargo run -p airlock-api    # :8080, stub Reader, offline
npm install && npm run dev  # :5173
```

Vite proxies `/health`, `/events`, `/inbound-sms`, `/transfers` and
`/transactions` to `:8080`, so everything stays same-origin.

## Screens

| Route | For | What it is |
|---|---|---|
| `#/wallet` | the victim | An ordinary wallet. Send money, see a hold, release or cancel. |
| `#/sender` | the judge | Type a message and deliver it. Nothing pre-recorded. |
| `#/pipeline` | the projector | The airlock as a node graph, lit by real events. |

## The rules this code follows

**Rule 4 — the UI shows real backend state.** Every value on screen came from
`/transactions`, `/health`, or an SSE frame. The one local clock is the hold
countdown, and it counts toward `releases_at`, which is a *server* timestamp —
not a duration the frontend invented.

**The countdown decides nothing.** "Send anyway" is gated on `releasable`,
which the server computes and re-checks on the release call. When the
countdown reaches zero the client does not flip anything; it re-fetches the
transaction and asks. A client that thinks the timer has run out does not get
to be right about it.

**No model prose reaches the screen.** `PlainReason` crosses the wire as a
variant name and is switched on in `src/lib/reasons.ts`, which is the only
file that turns it into words. Every one of those strings is ours, so
attacker-controlled text cannot reach the display.

**No jargon above the fold.** The user reads "Paused for a moment", never
`Held` or `NovelRecipientUnsolicitedContact`. Variant names live behind the
"Why did this happen?" disclosure.

## Layout

```
src/
  lib/
    wire.ts      the wire contract, mirrored from crates/api/src/dto.rs
    api.ts       the routes in crates/api/src/lib.rs
    store.ts     SSE subscription, resync, the one countdown tick
    reasons.ts   PlainReason -> the words a person reads
    format.ts    money, clocks
    theme.ts     light / dark / system
    dom.ts       escaping and inline SVG icons
  views/
    wallet.ts    #/wallet
    sender.ts    #/sender
    pipeline.ts  #/pipeline
```

`wire.ts` is a mirror, not a source of truth. If Track A changes a type in
`crates/core`, change it there too — otherwise the UI just lies quietly.

## Beat six

The fail-closed beat needs the Reader as its own process:

```bash
cargo run -p airlock-reader                                # :8081
READER_URL=http://127.0.0.1:8081 cargo run -p airlock-api  # :8080
```

Kill the Reader, then send to a first-time recipient. `/health.reader_reachable`
goes false, the Reader node on `#/pipeline` goes dark, its outgoing edge breaks,
and the transfer holds instead of passing. Nothing about that is simulated — the
UI is reporting a refused socket.

In the default stub mode the Reader is in-process and infallible, so the Reader
node never goes dark. That is correct, not a bug: there is no failure to show.
