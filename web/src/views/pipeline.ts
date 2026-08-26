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

/**
 * `stage` is the edge's position in the flow, and it phases the pulse so the
 * motion travels through the pipeline instead of every edge blinking at once.
 *
 * The two edges into the Linker share stage 1 deliberately: the message and
 * the transfer arrive together, which is the whole reason there is a Linker.
 */
const EDGES: { from: string; to: string; label?: string; stage: number }[] = [
  { from: "sms", to: "reader", stage: 0 },
  { from: "reader", to: "linker", label: "PressureSignal", stage: 1 },
  { from: "transfer", to: "linker", label: "TransferFacts", stage: 1 },
  { from: "linker", to: "policy", label: "Responsiveness", stage: 2 },
  { from: "policy", to: "outcome", stage: 3 },
];

/** One full trip through the chain, in seconds. Matches `--trace-period`. */
const TRACE_PERIOD = 2.8;
const STAGES = 4;

const byId = (id: string) => NODES.find((n) => n.id === id)!;

export function renderPipeline(root: HTMLElement): () => void {
  root.innerHTML = `
    <div class="flow dotgrid">
      <div class="flow-inner" id="p-inner">
        <svg class="edges" id="p-edges" width="${CANVAS_W}" height="${CANVAS_H}"
             viewBox="0 0 ${CANVAS_W} ${CANVAS_H}">${EDGES.map(edgeMarkup).join("")}</svg>
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

    paintEdges(root, states);

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

type Edge = { from: string; to: string; label?: string; stage: number };

const edgeKey = (e: Edge) => `${e.from}-${e.to}`;

/** Where an edge starts, ends, and the curve between. Pure geometry. */
function edgeCurve(e: Edge) {
  const a = byId(e.from);
  const b = byId(e.to);
  const x1 = a.x + a.w;
  const y1 = a.y + a.h / 2;
  const x2 = b.x;
  const y2 = b.y + b.h / 2;
  const dx = Math.max(50, Math.abs(x2 - x1) * 0.45);
  return {
    d: `M ${x1} ${y1} C ${x1 + dx} ${y1}, ${x2 - dx} ${y2}, ${x2} ${y2}`,
    midX: (x1 + x2) / 2,
    midY: (y1 + y2) / 2,
  };
}

/**
 * An edge, drawn once as a trace: a dim base line that is always there, plus
 * a short bright pulse that travels along it.
 *
 * **This markup is emitted exactly once.** The store ticks every second to
 * move the hold countdowns, and rebuilding these paths on each tick would
 * restart every CSS animation from zero — the pulse would never get more than
 * a second along the curve. Updates go through `paintEdges`, which only
 * changes classes.
 *
 * `pathLength="100"` normalises every curve to the same 100 units, so one
 * dash pattern and one keyframe work for edges of wildly different lengths —
 * no measuring with `getTotalLength()`.
 *
 * Every pulse shares one period, so a per-stage delay is a fixed phase offset
 * rather than a drift: each edge sits a quarter-cycle behind the one before
 * it, and the lit segments form a wave running down the chain.
 */
function edgeMarkup(e: Edge): string {
  const { d, midX, midY } = edgeCurve(e);
  const key = edgeKey(e);
  const delay = (e.stage * (TRACE_PERIOD / STAGES)).toFixed(2);

  const label = e.label
    ? `<text class="edge-label" x="${midX}" y="${midY - 9}" text-anchor="middle">${esc(
        e.label,
      )}</text>`
    : "";

  return `
    <path id="eb-${key}" class="edge-base" d="${d}" />
    <path id="ep-${key}" class="edge-pulse off" d="${d}" pathLength="100"
          style="animation-delay:${delay}s" />
    ${label}`;
}

/**
 * Restate which edges carried the signal. Classes only — never markup, so the
 * running animations are left alone.
 *
 * A pulse is hidden with a class rather than removed, because removing it and
 * putting it back would restart its animation and break the phase with its
 * neighbours.
 */
function paintEdges(root: HTMLElement, states: Record<string, NodeState>) {
  for (const e of EDGES) {
    const key = edgeKey(e);
    const from = states[e.from];
    // An edge out of a dead node is a path the signal never took.
    const broken = from === "dead";
    const carried =
      !broken && (from === "on" || from === "done" || from === "hold");

    const base = root.querySelector(`#eb-${key}`);
    if (base) {
      base.setAttribute(
        "class",
        `edge-base${broken ? " dead" : carried ? " on" : ""}`,
      );
    }

    const pulse = root.querySelector(`#ep-${key}`);
    if (pulse) {
      pulse.setAttribute("class", `edge-pulse${carried ? "" : " off"}`);
    }
  }
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
