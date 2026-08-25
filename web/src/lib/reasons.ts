/**
 * The only place `PlainReason` becomes words.
 *
 * The brief: the user sees "paused for a moment", never `Held` or
 * `NovelRecipientUnsolicitedContact`. Because the reason crosses the wire as
 * a closed enum rather than model prose, every string below is one we wrote —
 * attacker-controlled text cannot reach this file, and there is no variant
 * that renders as "whatever the model said".
 */

import type { PlainReason, TransactionState } from "./wire";

export interface ReasonCopy {
  /** Plain-language headline. No jargon, ever. */
  title: string;
  /** Why this happened, in the second person. */
  body: string;
  /** The honest caveat, shown when the hold may be a false positive. */
  aside?: string;
}

export const REASON_COPY: Record<PlainReason, ReasonCopy> = {
  NovelRecipientUnsolicitedContact: {
    title: "Paused for a moment",
    body: "A message arrived shortly before this transfer, asking you to send money to a number you have never paid.",
    aside:
      "If the request is genuine, you lose a minute. If it isn't, you keep your money.",
  },
  ScreeningUnavailable: {
    title: "Paused for a moment",
    body: "We could not finish our checks just now, and this is a number you have never paid. So we are holding it rather than letting it through.",
    aside: "When we are unsure, we stop. We never guess in your favour.",
  },
  EstablishedRecipient: {
    title: "Sent",
    body: "You have paid this number before, so it went straight through.",
  },
  UserReleased: {
    title: "Sent",
    body: "You confirmed this was your idea, so we let it through.",
  },
  CoolingPeriodNotElapsed: {
    title: "Still paused",
    body: "The pause has not finished yet.",
  },
};

/**
 * What the person sees for a given state. Deliberately not the variant name:
 * `Held` is our word, "Paused" is theirs.
 */
export function stateLabel(state: TransactionState): string {
  switch (state) {
    case "Proposed":
      return "Starting";
    case "Screening":
      return "Checking";
    case "Cleared":
      return "Approved";
    case "Held":
      return "Paused";
    case "Released":
      return "Released";
    case "Cancelled":
      return "Cancelled";
    case "Executed":
      return "Sent";
  }
}

/** Which of our colour roles a state reads as. */
export function stateTone(
  state: TransactionState,
): "hold" | "good" | "dead" | "busy" {
  switch (state) {
    case "Held":
      return "hold";
    case "Executed":
    case "Cleared":
    case "Released":
      return "good";
    case "Cancelled":
      return "dead";
    case "Proposed":
    case "Screening":
      return "busy";
  }
}
