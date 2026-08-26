import type { HealthView, TxnView } from "./wire";

/**
 * The routes in `crates/api/src/lib.rs`. Vite proxies these to :8080 in dev
 * (see vite.config.ts), so everything here stays same-origin and relative.
 *
 * A deployed build has no dev server and therefore no proxy, so it needs to
 * be told where the API lives: set `VITE_API_BASE` at build time. Unset — the
 * dev case and the demo case — leaves every path relative and the proxy
 * handles it.
 */

/**
 * The API origin, or "" when the API is same-origin.
 *
 * Trailing slashes are trimmed because every caller below writes a path that
 * already starts with one, and `https://host//events` is not the same URL.
 */
const API_BASE = (import.meta.env.VITE_API_BASE ?? "").replace(/\/+$/, "");

/** Absolute when the API is elsewhere, relative when it is not. */
export function apiUrl(path: string): string {
  return `${API_BASE}${path}`;
}

export class ApiError extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message);
  }
}

async function send<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(apiUrl(path), {
    ...init,
    headers: init?.body ? { "content-type": "application/json" } : undefined,
  });
  if (!res.ok) {
    // The API answers with `{ "error": ... }`; fall back to the status line
    // when it hands us something else (a proxy error page, say).
    let detail = res.statusText;
    try {
      const body = (await res.json()) as { error?: string };
      if (body.error) detail = body.error;
    } catch {
      /* not JSON — keep the status line */
    }
    throw new ApiError(res.status, detail);
  }
  return (await res.json()) as T;
}

export const api = {
  health: () => send<HealthView>("/health"),

  transactions: () => send<TxnView[]>("/transactions"),

  transaction: (id: number) => send<TxnView>(`/transactions/${id}`),

  /** The judge-facing endpoint. Nothing about the attack is pre-recorded. */
  inbound: (text: string) =>
    send<{ recorded: boolean; inbox_messages: number }>("/inbound-sms", {
      method: "POST",
      body: JSON.stringify({ text }),
    }),

  transfer: (recipient: string, amountMinor: number, currency = "NGN") =>
    send<TxnView>("/transfers", {
      method: "POST",
      body: JSON.stringify({
        recipient,
        amount_minor: amountMinor,
        currency,
      }),
    }),

  /** The server re-checks the cooling period; a client that disagrees loses. */
  release: (id: number) =>
    send<TxnView>(`/transactions/${id}/release`, { method: "POST" }),

  cancel: (id: number) =>
    send<TxnView>(`/transactions/${id}/cancel`, { method: "POST" }),
};
