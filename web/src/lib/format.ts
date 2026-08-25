import type { MoneyView } from "./wire";

/**
 * Minor units to a readable amount. NGN is kobo — `500000` is ₦5,000.00, which
 * is what `fixtures/scams.json` means by `amount_minor`.
 */
const SYMBOLS: Record<string, string> = { NGN: "₦", KES: "KSh", USD: "$" };

export function formatMoney(m: MoneyView): string {
  const symbol = SYMBOLS[m.currency] ?? `${m.currency} `;
  const major = m.minor_units / 100;
  return (
    symbol +
    major.toLocaleString("en-NG", {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    })
  );
}

/** Whole seconds until `iso`, floored at zero. */
export function secondsUntil(iso: string, now = Date.now()): number {
  const ms = new Date(iso).getTime() - now;
  return Math.max(0, Math.ceil(ms / 1000));
}

export function formatClock(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return `${m}:${String(s).padStart(2, "0")}`;
}

export function formatTime(iso: string): string {
  return new Date(iso).toLocaleTimeString("en-GB", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}
