# Deploying

Frontend on Vercel, API on Render.

**Read this first: the hosted instance is not where the demo runs.** It is a
leave-behind, so judges can open the link afterwards and poke at the UI. The
demo itself runs on the laptop, for three reasons that are all in
[pitch_and_demo.md](./pitch_and_demo.md) — chiefly that beat six kills a real
Reader process in front of people, and you cannot do that to a container.

---

## Why the hosted link was blank

The frontend calls relative paths (`/health`, `/events`, `/transfers`) and
relied entirely on Vite's **dev-server proxy** to forward them to `:8080`. That
proxy exists only under `vite dev`. A production build has no proxy, so those
paths hit the static host, which has no such routes, and every call returned
404.

The fix is `VITE_API_BASE`: unset, everything stays relative and the dev proxy
handles it; set, every path becomes absolute against the API's origin. One
helper, `apiUrl()` in `web/src/lib/api.ts`, is used by both `fetch` and
`EventSource`.

---

## The API on Render

`render.yaml` is a Blueprint — point Render at the repo and it reads it.

Or by hand: **New → Web Service → Docker**, root directory `.`, and set
`BIND_ALL=1`. Health check path `/health`.

Three things worth knowing.

**`BIND_ALL` is required and deliberate.** The server binds `127.0.0.1` unless
that variable is set. Binding `0.0.0.0` unconditionally would mean that every
local run also exposed the demo wallet to whatever network the laptop is on —
including venue wifi. That is not a property to discover on stage. Render sets
`PORT` itself and the server already reads it.

**The hosted instance runs the stub Reader.** `READER_URL` stays unset. A
second free-tier service would spin down on idle, and then every
novel-recipient transfer would fail closed and hold — correct behaviour that
is indistinguishable from a broken deployment. Fail-closed belongs on the
laptop where the failure is visible and deliberate.

**Free tier sleeps after 15 minutes idle.** First request after that takes
roughly a minute while the container cold-starts, and the UI will sit on
"connecting". If the link matters during judging, open it a minute early.

---

## The frontend on Vercel

Set one environment variable, for the Production environment:

```
VITE_API_BASE = https://<your-service>.onrender.com
```

Then **redeploy** — Vite inlines this at build time, so an existing build will
not pick it up. Confirm it took:

```bash
curl -s https://airlock-five.vercel.app/assets/index-*.js | grep -o 'onrender[^"]*'
```

No CORS configuration is needed. The API already answers with
`access-control-allow-origin: *`, on the SSE stream as well as the JSON
routes, which is what `EventSource` needs to connect cross-origin.

The API must be HTTPS. Render provides that on `*.onrender.com`; an HTTP
backend would be blocked as mixed content from an HTTPS page.

---

## Verifying a deployment

```bash
API=https://<your-service>.onrender.com

curl -s $API/health
# {"status":"ok","reader_reachable":true,"reader_mode":"stub","inbox_messages":0}

curl -s -X POST $API/inbound-sms -H 'content-type: application/json' \
  -d '{"text":"MTN Alert: your account will be suspended today. Call 08031234567."}'

curl -s -X POST $API/transfers -H 'content-type: application/json' \
  -d '{"recipient":"08031234567","amount_minor":500000}'
# ..."state":"Held","reason":"NovelRecipientUnsolicitedContact"...
```

If the page loads but nothing works, check in this order: `VITE_API_BASE` is
set on Vercel *and* the project was redeployed after setting it; the Render
service is awake; `/health` answers directly.

---

## The one real caveat

**State is global and in-memory.** One inbox, one ledger, one transaction
store, shared by everyone who opens the link. Two people using it at once see
each other's transactions, and one person's scam message sits in the
correlation window for the other person's transfer.

That is fine for a single controlled demo and messy for a public link. There
is no per-session isolation and adding it is not a small change — it would
mean a session key on every request and partitioning all three stores. Worth
knowing before sending the URL to a room full of people.

Everything also resets when the service restarts or wakes from sleep, seeded
history included. Nothing is persisted, by design — the brief rules out
deployment infra, and this is demo-lifetime state.
