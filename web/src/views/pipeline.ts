/**
 * The operator / projector view: the airlock itself, drawn as a graph.
 *
 * Every node's state comes from a real server event or from /health. Nothing
 * animates on a timer of its own. When the Reader process dies in beat six,
 * `/health.reader_reachable` goes false and the Reader node goes dark on
 * screen — which is the brief's requirement that the fail-closed moment be
 * visible rather than buried in a log.
 */

import { esc } from "../lib/dom";
import { formatMoney, formatTime } from "../lib/format";
import { stateLabel } from "../lib/reasons";
import { store } from "../lib/store";
import type { StoreState } from "../lib/store";
import type { AirlockEvent, TxnView } from "../lib/wire";

type NodeState = "idle" | "on" | "done" | "hold" | "dead";

interface Node {
  id: string;
  kind: string;
  name: string;
  note: string;
  x: number;
  y: number;
  w: number;
  h: number;
}

/**
 * Node box and the canvas it sits on. These are the same coordinate space the
 * SVG viewBox and `.flow-inner` use — change one and change all three, or the
 * edges stop meeting their handles.
 */
const W = 208;
const H = 108;
const CANVAS_W = 1360;
const CANVAS_H = 520;

const NODES: Node[] = [
  { id: "sms", kind: "Untrusted", name: "Inbound message", note: "SMS or call transcript", x: 24, y: 60, w: W, h: H },
  { id: "reader", kind: "Agent", name: "Reader", note: "No account access. Raw text stops here.", x: 300, y: 60, w: W, h: H },
  { id: "transfer", kind: "User-authorised", name: "Transfer request", note: "PIN already verified", x: 24, y: 330, w: W, h: H },
  { id: "linker", kind: "Agent", name: "Linker", note: "Typed signal only. Never sees prose.", x: 576, y: 190, w: W, h: H },
  { id: "policy", kind: "Pure Rust", name: "Policy engine", note: "Owns the hold. No model.", x: 852, y: 190, w: W, h: H },
  { id: "outcome", kind: "Decision", name: "Outcome", note: "Pass, or hold for 60s", x: 1128, y: 190, w: W, h: H },
];

const EDGES: { from: string; to: string; label?: string }[] = [
  { from: "sms", to: "reader" },
  { from: "reader", to: "linker", label: "PressureSignal" },
  { from: "transfer", to: "linker", label: "TransferFacts" },
  { from: "linker", to: "policy", label: "Responsiveness" },
  { from: "policy", to: "outcome" },
];

const byId = (id: string) => NODES.find((n) => n.id === id)!;

export function renderPipeline(root: HTMLElement): () => void {
  root.innerHTML = `
    <div class="flow dotgrid">
      <div class="flow-inner" id="p-inner">
        <svg class="edges" id="p-edges" width="${CANVAS_W}" height="${CANVAS_H}"
             viewBox="0 0 ${CANVAS_W} ${CANVAS_H}"></svg>
        ${NODES.map(
          (n) => `
          <div class="node" id="n-${n.id}" style="left:${n.x}px;top:${n.y}px;width:${n.w}px;height:${n.h}px">
            ${n.id === "sms" || n.id === "transfer" ? "" : `<i class="handle in"></i>`}
            ${n.id === "outcome" ? "" : `<i class="handle out"></i>`}
            <div class="kind">${esc(n.kind)}</div>
            <div class="name">${esc(n.name)}</div>
            <div class="note" id="note-${n.id}">${esc(n.note)}</div>
          </div>`,
        ).join("")}
      </div>
    </div>

    <div class="legend">
      <span><i class="dot"></i> idle</span>
      <span><i class="dot busy"></i> running</span>
      <span><i class="dot live"></i> completed</span>
      <span><i class="dot down"></i> failed — holds rather than passes</span>
      <span class="spacer"></span>
      <span id="p-summary" class="muted"></span>
    </div>

    <div style="padding:18px 22px 40px 22px">
      <div class="card card-pad">
        <h2 class="card-title">Server events</h2>
        <div class="log" id="p-log"></div>
      </div>
    </div>
  `;

  const svg = root.querySelector("#p-edges")!;
  const log = root.querySelector("#p-log")!;
  const summary = root.querySelector("#p-summary")!;

  const unsubscribe = store.subscribe((s) => {
    const txn = s.txns.find((t) => t.state === "Held") ?? s.txns[0];
    const states = nodeStates(s, txn);

    for (const n of NODES) {
      const el = root.querySelector(`#n-${n.id}`)!;
      el.className = `node ${states[n.id] === "idle" ? "" : states[n.id]}`.trim();
    }

    const readerDown = s.health ? !s.health.reader_reachable : false;
    root.querySelector("#note-reader")!.textContent = readerDown
      ? "Not answering. Screening cannot complete."
      : byId("reader").note;
    root.querySelector("#note-outcome")!.textContent = txn
      ? outcomeNote(txn)
      : byId("outcome").note;

    svg.innerHTML = EDGES.map((e) => edgePath(e, states, s)).join("");

    summary.textContent = txn
      ? `txn ${txn.id} · ${formatMoney(txn.amount)} → ${txn.recipient} · ${stateLabel(txn.state)}`
      : "waiting for a transfer";

    log.innerHTML = s.log.length
      ? s.log
          .slice(0, 40)
          .map(
            (l) =>
              `<div><span class="t">${esc(formatTime(l.at))}</span>  ${esc(describe(l.event))}</div>`,
          )
          .join("")
      : `<div class="muted">No events yet.</div>`;
  });

  return unsubscribe;
}

function nodeStates(s: StoreState, txn: TxnView | undefined): Record<string, NodeState> {
  const readerDown = s.health ? !s.health.reader_reachable : false;
  const inboxHas = (s.health?.inbox_messages ?? 0) > 0;
  const screening = txn?.state === "Screening" || txn?.state === "Proposed";
  const readerFailed = s.failedComponent === "Reader" || readerDown;

  const out: Record<string, NodeState> = {
    sms: inboxHas ? "done" : "idle",
    reader: readerFailed ? "dead" : screening ? "on" : txn ? "done" : "idle",
    transfer: txn ? "done" : "idle",
    linker: readerFailed ? "idle" : screening ? "on" : txn ? "done" : "idle",
    policy: txn ? (screening ? "on" : "done") : "idle",
    outcome: "idle",
  };

  if (txn) {
    if (txn.state === "Held") out.outcome = "hold";
    else if (txn.state === "Cancelled") out.outcome = "dead";
    else if (txn.state !== "Proposed" && txn.state !== "Screening") out.outcome = "done";
    else out.outcome = "on";
  }
  return out;
}

function outcomeNote(t: TxnView): string {
  switch (t.state) {
    case "Held":
      return `Held. ${t.reason ?? ""}`.trim();
    case "Executed":
    case "Cleared":
      return "Passed straight through.";
    case "Released":
      return "Released by the account holder.";
    case "Cancelled":
      return "Cancelled by the account holder.";
    default:
      return "Deciding…";
  }
}

function edgePath(
  e: { from: string; to: string; label?: string },
  states: Record<string, NodeState>,
  s: StoreState,
): string {
  const a = byId(e.from);
  const b = byId(e.to);
  const x1 = a.x + a.w;
  const y1 = a.y + a.h / 2;
  const x2 = b.x;
  const y2 = b.y + b.h / 2;
  const dx = Math.max(50, Math.abs(x2 - x1) * 0.45);
  const d = `M ${x1} ${y1} C ${x1 + dx} ${y1}, ${x2 - dx} ${y2}, ${x2} ${y2}`;

  // An edge out of a dead node is a path the signal never took.
  const broken = states[e.from] === "dead";
  const live = states[e.from] === "on" && !broken;
  const carried =
    !broken && (states[e.from] === "done" || states[e.from] === "hold");

  let cls = "edge";
  if (broken) cls += " dead";
  else if (live) cls += " flowing";
  else if (carried) cls += " on";
  void s;

  const label = e.label
    ? `<text class="edge-label" x="${(x1 + x2) / 2}" y="${(y1 + y2) / 2 - 9}" text-anchor="middle">${esc(
        e.label,
      )}</text>`
    : "";

  return `<path class="${cls}" d="${d}" />${label}`;
}

function describe(e: AirlockEvent): string {
  switch (e.type) {
    case "StateChanged":
      return `txn ${e.txn}  ${e.from} → ${e.to}`;
    case "HoldOpened":
      return `txn ${e.txn}  HoldOpened  ${e.reason}  releases_at=${e.releases_at}`;
    case "ScreenFailed":
      return `txn ${e.txn}  ScreenFailed  component=${e.component}`;
  }
}
