/**
 * The wallet. A judge should recognise this as a wallet, not as our
 * architecture — so nothing here says `Held`, `PressureSignal` or
 * `NovelRecipientUnsolicitedContact` above the fold. The variant names live
 * behind the disclosure at the bottom, for the one person who asks.
 */

import { esc, icons, on } from "../lib/dom";
import { formatClock, formatMoney, formatTime, secondsUntil } from "../lib/format";
import { REASON_COPY, stateLabel, stateTone } from "../lib/reasons";
import { store } from "../lib/store";
import type { TxnView } from "../lib/wire";

const HOLD_SECONDS = 60; // airlock_policy::HOLD_DURATION, for the meter only.

const PRESETS = [
  { label: "Landlord", msisdn: "08055512345", note: "paid monthly" },
  { label: "Sister", msisdn: "08033344556", note: "paid before" },
  { label: "New number", msisdn: "08031234567", note: "never paid" },
];

export function renderWallet(root: HTMLElement): () => void {
  root.innerHTML = `
    <div class="wallet-wrap dotgrid">
      <section class="phone">
        <div class="phone-head">
          <div>
            <div class="tiny muted">Available balance</div>
            <div class="balance mono">₦48,200.00</div>
          </div>
          <span class="badge" id="w-inbox">inbox 0</span>
        </div>
        <div class="phone-body">
          <div id="w-active"></div>

          <div class="stack" id="w-form">
            <div class="field">
              <label for="w-to">Send to</label>
              <input id="w-to" class="mono" inputmode="numeric" placeholder="08031234567" value="08031234567" />
            </div>
            <div class="grid2">
              ${PRESETS.map(
                (p) =>
                  `<button class="preset" data-msisdn="${esc(p.msisdn)}">
                     <strong>${esc(p.label)}</strong><br />
                     <span class="tiny">${esc(p.note)}</span>
                   </button>`,
              ).join("")}
            </div>
            <div class="field">
              <label for="w-amt">Amount (₦)</label>
              <input id="w-amt" class="mono" inputmode="decimal" value="5000" />
            </div>
            <button class="btn primary block" id="w-send">Send money</button>
            <div id="w-err"></div>
          </div>

          <div class="spacer"></div>
        </div>
      </section>

      <aside class="side">
        <div class="card card-pad">
          <h2 class="card-title">Recent</h2>
          <div id="w-list"></div>
        </div>
      </aside>
    </div>
  `;

  const active = root.querySelector("#w-active")!;
  const list = root.querySelector("#w-list")!;
  const inbox = root.querySelector("#w-inbox")!;
  const err = root.querySelector("#w-err")!;
  const to = root.querySelector<HTMLInputElement>("#w-to")!;
  const amt = root.querySelector<HTMLInputElement>("#w-amt")!;
  const sendBtn = root.querySelector<HTMLButtonElement>("#w-send")!;

  on(root, ".preset", "click", (_e, el) => {
    to.value = el.dataset.msisdn ?? "";
  });

  sendBtn.addEventListener("click", async () => {
    const naira = Number(amt.value);
    if (!Number.isFinite(naira) || naira <= 0) {
      err.innerHTML = `<div class="notice bad">Enter an amount.</div>`;
      return;
    }
    err.innerHTML = "";
    sendBtn.disabled = true;
    sendBtn.textContent = "Sending…";
    try {
      await store.transfer(to.value.trim(), Math.round(naira * 100));
    } catch (e) {
      err.innerHTML = `<div class="notice bad">${esc((e as Error).message)}</div>`;
    } finally {
      sendBtn.disabled = false;
      sendBtn.textContent = "Send money";
    }
  });

  const unsubscribe = store.subscribe((s) => {
    inbox.textContent = `inbox ${s.health?.inbox_messages ?? 0}`;

    const held = s.txns.find((t) => t.state === "Held");
    const latest = held ?? s.txns[0];
    active.innerHTML = latest ? panel(latest) : "";

    on(active, "[data-release]", "click", async (_e, el) => {
      const b = el as HTMLButtonElement;
      b.disabled = true;
      try {
        await store.release(Number(b.dataset.release));
      } catch (e) {
        err.innerHTML = `<div class="notice bad">${esc((e as Error).message)}</div>`;
        b.disabled = false;
      }
    });

    on(active, "[data-cancel]", "click", async (_e, el) => {
      const b = el as HTMLButtonElement;
      b.disabled = true;
      try {
        await store.cancel(Number(b.dataset.cancel));
      } catch (e) {
        err.innerHTML = `<div class="notice bad">${esc((e as Error).message)}</div>`;
        b.disabled = false;
      }
    });

    list.innerHTML = s.txns.length
      ? s.txns
          .slice(0, 8)
          .map(
            (t) => `
        <div class="txn-item">
          <div>
            <div class="mono">${esc(t.recipient)}</div>
            <div class="tiny muted">${esc(formatTime(t.proposed_at))}</div>
          </div>
          <div style="text-align:right">
            <div class="mono" style="font-weight:600">${esc(formatMoney(t.amount))}</div>
            <span class="badge ${stateTone(t.state)}">${esc(stateLabel(t.state))}</span>
          </div>
        </div>`,
          )
          .join("")
      : `<div class="small muted">Nothing yet.</div>`;
  });

  return unsubscribe;
}

function panel(t: TxnView): string {
  if (t.state !== "Held") return receipt(t);

  const copy = t.reason
    ? REASON_COPY[t.reason]
    : REASON_COPY.NovelRecipientUnsolicitedContact;

  // The countdown runs toward `releases_at` — a server timestamp. It is not a
  // frontend timer inventing progress; and it does not decide anything. The
  // button below is gated on `releasable`, which the server computes and
  // re-checks when we call release.
  const left = t.releases_at ? secondsUntil(t.releases_at) : 0;
  const pct = Math.max(0, Math.min(100, ((HOLD_SECONDS - left) / HOLD_SECONDS) * 100));

  return `
    <div class="hold">
      <div class="hold-head">
        <span style="color:var(--hold)">${icons.pause}</span>
        <span class="hold-title">${esc(copy.title)}</span>
      </div>

      <div class="row"><span class="k">Sending</span>
        <span class="v mono">${esc(formatMoney(t.amount))}</span></div>
      <div class="row"><span class="k">To</span>
        <span class="v mono">${esc(t.recipient)}</span></div>
      <div class="row"><span class="k">Paid before</span>
        <span class="v">${t.recipient_established ? "Yes" : "Never"}</span></div>

      <p class="small" style="margin:14px 0 0 0">${esc(copy.body)}</p>
      ${copy.aside ? `<p class="small muted" style="margin:8px 0 0 0">${esc(copy.aside)}</p>` : ""}

      <div style="display:flex;justify-content:space-between;align-items:baseline;margin:16px 0 8px 0">
        <span class="small muted">${t.releasable ? "Ready" : "You can send this in"}</span>
        <span class="countdown" style="color:var(--hold)">${
          t.releasable ? "0:00" : esc(formatClock(left))
        }</span>
      </div>
      <div class="meter"><i style="width:${pct.toFixed(1)}%"></i></div>

      <div class="stack" style="margin-top:16px">
        <button class="btn primary block" data-release="${t.id}" ${
          t.releasable ? "" : "disabled"
        }>Send anyway</button>
        <button class="btn danger block" data-cancel="${t.id}">Cancel this transfer</button>
      </div>

      ${tech(t)}
    </div>
  `;
}

function receipt(t: TxnView): string {
  const tone = stateTone(t.state);
  const copy = t.reason ? REASON_COPY[t.reason] : null;
  return `
    <div class="card card-pad">
      <div style="display:flex;justify-content:space-between;align-items:center">
        <strong>${esc(copy?.title ?? stateLabel(t.state))}</strong>
        <span class="badge ${tone}">${esc(stateLabel(t.state))}</span>
      </div>
      <div class="row" style="margin-top:8px"><span class="k">Amount</span>
        <span class="v mono">${esc(formatMoney(t.amount))}</span></div>
      <div class="row"><span class="k">To</span>
        <span class="v mono">${esc(t.recipient)}</span></div>
      ${copy ? `<p class="small muted" style="margin:12px 0 0 0">${esc(copy.body)}</p>` : ""}
      ${tech(t)}
    </div>
  `;
}

/** The only place internal vocabulary is allowed on this screen. */
function tech(t: TxnView): string {
  return `
    <details class="tech">
      <summary>Why did this happen?</summary>
      <div class="body">
state        ${esc(t.state)}
reason       ${esc(t.reason ?? "—")}
releasable   ${t.releasable}
releases_at  ${esc(t.releases_at ?? "—")}
recipient    ${esc(t.recipient)} (established: ${t.recipient_established})
txn          ${t.id}
      </div>
    </details>
  `;
}
