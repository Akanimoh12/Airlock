/**
 * Counter-advice: what to tell someone who has just been told a lie by
 * somebody claiming to be their network, their bank, or the government.
 *
 * This is the table the hold screen exists to show. The sixty seconds is not
 * a speed bump — it is the only moment in the whole attack where the victim
 * is looking at something other than the scammer's script, and this is what
 * we get to put there.
 *
 * **Every string here is written by us.** `ClaimedAuthority` is a closed set
 * validated at the trust boundary (`crates/agents/src/validate.rs`), so the
 * only thing that crosses from an attacker's message is *which row to show*.
 * Never build these from message content, and never add a variant that
 * renders text the Reader produced.
 *
 * The `verify` line is the load-bearing half. A victim who is mid-call has
 * already been given a number to trust; the useful thing is a different
 * number they can reach independently. So it must never be a number from the
 * message, and it should be one they can check on the SIM pack, the card, or
 * the official app.
 */

import type { ClaimedAuthority } from "./wire";

export interface CounterAuthority {
  /** What the claimed institution does not do. Flat contradiction. */
  denial: string;
  /** An independent way to check. Never the number in the message. */
  verify: string;
}

const GENERIC: CounterAuthority = {
  denial:
    "No bank, network or agency asks you to send money to keep an account open.",
  verify:
    "Don’t call the number in that message. Use the number on your official app, your SIM pack, or the back of your card.",
};

const TELCO = (name: string, short: string): CounterAuthority => ({
  denial: `${name} will never ask you to pay to reactivate or revalidate your line.`,
  verify: `If you want to check, call ${short} — not the number in that message.`,
});

const COPY: Record<ClaimedAuthority, CounterAuthority> = {
  // Nigerian networks publish short codes that are free from the network.
  Mtn: TELCO("MTN", "180"),
  Airtel: TELCO("Airtel", "111"),
  Glo: TELCO("Glo", "121"),
  NineMobile: TELCO("9mobile", "200"),
  Safaricom: TELCO("Safaricom", "100"),

  MobileMoney: {
    denial:
      "A mobile money service will never ask you to send money to verify or unlock your wallet.",
    verify:
      "Don’t call the number in that message. Open the official app, or use the number printed on your agent’s signage.",
  },

  Bank: {
    denial:
      "Your bank will never ask you to move money to a “safe account”, or ask for your PIN or OTP.",
    verify:
      "Don’t call the number in that message. Call the number on the back of your card.",
  },

  Government: {
    denial:
      "The CBN, EFCC, NIMC and NCC do not collect payments by SMS, and never ask for transfers to an individual’s number.",
    verify:
      "Don’t call the number in that message. Look the agency up independently before you do anything.",
  },

  Unknown: GENERIC,
  None: GENERIC,
};

export function counterAuthority(a: ClaimedAuthority): CounterAuthority {
  return COPY[a] ?? GENERIC;
}

/**
 * Whether we can name who was impersonated. Drives whether the hold screen
 * leads with a specific denial or the generic one.
 */
export function isNamedAuthority(a: ClaimedAuthority): boolean {
  return a !== "Unknown" && a !== "None";
}
