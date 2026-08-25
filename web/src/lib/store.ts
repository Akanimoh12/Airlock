/**
 * Every transaction on screen, and the event stream that keeps it true.
 *
 * Rule 4 of the brief: the UI shows real backend state. So this store holds
 * only what the server told us. The one clock we run locally is the countdown,
 * and it counts toward `releases_at` — a server timestamp — rather than
 * inventing a duration. When it reaches zero we go and *ask* whether the
 * transfer is releasable instead of deciding for ourselves; `releasable` is
 * computed server-side and enforced again on the release call.
 */

import { api } from "./api";
import type { AirlockEvent, Component, HealthView, TxnView } from "./wire";

export type Connection = "connecting" | "live" | "down";

export interface LogEntry {
  at: string;
  event: AirlockEvent;
}

export interface StoreState {
  txns: TxnView[];
  health: HealthView | null;
  connection: Connection;
  /** Raw events, newest first. The pipeline view and the detail panel read it. */
  log: LogEntry[];
  /** Set by `ScreenFailed`, cleared when a later screening completes. */
  failedComponent: Component | null;
}

type Listener = (s: StoreState) => void;

const HEALTH_POLL_MS = 2000;

class Store {
  private state: StoreState = {
    txns: [],
    health: null,
    connection: "connecting",
    log: [],
    failedComponent: null,
  };

  private listeners = new Set<Listener>();
  private source: EventSource | null = null;
  private started = false;

  get(): StoreState {
    return this.state;
  }

  subscribe(fn: Listener): () => void {
    this.listeners.add(fn);
    fn(this.state);
    return () => this.listeners.delete(fn);
  }

  private set(patch: Partial<StoreState>) {
    this.state = { ...this.state, ...patch };
    for (const fn of this.listeners) fn(this.state);
  }

  start() {
    if (this.started) return;
    this.started = true;
    this.connect();
    void this.resync();
    void this.pollHealth();
    // The countdown is the only local clock, and only to re-ask the server.
    setInterval(() => this.tick(), 1000);
  }

  private connect() {
    this.source?.close();
    const source = new EventSource("/events");
    this.source = source;

    source.onopen = () => {
      this.set({ connection: "live" });
      // A stream that just opened may have missed everything before it; the
      // API exposes /transactions for exactly this.
      void this.resync();
    };

    source.onmessage = (e) => {
      let event: AirlockEvent;
      try {
        event = JSON.parse(e.data) as AirlockEvent;
      } catch {
        return;
      }
      this.apply(event);
    };

    source.onerror = () => {
      this.set({ connection: "down" });
      // EventSource reconnects on its own; when it does, `onopen` resyncs.
    };
  }

  /** Authoritative refetch. Used on connect, and after any state change. */
  private async resync() {
    try {
      const txns = await api.transactions();
      this.set({ txns: sortNewestFirst(txns) });
    } catch {
      /* the connection indicator already says what is wrong */
    }
  }

  private async pollHealth() {
    for (;;) {
      try {
        this.set({ health: await api.health() });
      } catch {
        this.set({ health: null });
      }
      await sleep(HEALTH_POLL_MS);
    }
  }

  private apply(event: AirlockEvent) {
    const log = [{ at: new Date().toISOString(), event }, ...this.state.log].slice(
      0,
      200,
    );

    if (event.type === "ScreenFailed") {
      this.set({ log, failedComponent: event.component });
    } else if (event.type === "StateChanged" && event.to === "Screening") {
      // A fresh screening run: whatever failed last time is no longer the
      // current story.
      this.set({ log, failedComponent: null });
    } else {
      this.set({ log });
    }

    // Every event carries a txn whose server-side view may now differ. Ask.
    void this.refresh(event.txn);
  }

  private async refresh(id: number) {
    try {
      const txn = await api.transaction(id);
      this.merge(txn);
    } catch {
      void this.resync();
    }
  }

  private merge(txn: TxnView) {
    const rest = this.state.txns.filter((t) => t.id !== txn.id);
    this.set({ txns: sortNewestFirst([txn, ...rest]) });
  }

  /**
   * Once a held transfer's server timestamp has passed, re-ask the server
   * whether it is releasable. We never flip `releasable` ourselves.
   */
  private tick() {
    const now = Date.now();
    for (const t of this.state.txns) {
      if (t.state !== "Held" || t.releasable || !t.releases_at) continue;
      if (new Date(t.releases_at).getTime() <= now) void this.refresh(t.id);
    }
    // Re-render so countdowns move.
    this.set({});
  }

  async release(id: number) {
    this.merge(await api.release(id));
  }

  async cancel(id: number) {
    this.merge(await api.cancel(id));
  }

  async transfer(recipient: string, amountMinor: number) {
    const txn = await api.transfer(recipient, amountMinor);
    this.merge(txn);
    return txn;
  }

  async inbound(text: string) {
    const res = await api.inbound(text);
    this.set({ health: await api.health().catch(() => this.state.health) });
    return res;
  }
}

function sortNewestFirst(txns: TxnView[]): TxnView[] {
  return [...txns].sort((a, b) => b.id - a.id);
}

function sleep(ms: number) {
  return new Promise((r) => setTimeout(r, ms));
}

export const store = new Store();
