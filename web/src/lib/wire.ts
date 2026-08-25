/**
 * The wire contract, mirrored from the Rust side. Track A owns these shapes;
 * changing one here without changing `crates/core` (or `crates/api/src/dto.rs`)
 * just makes the UI lie.
 *
 * `TxnId` is a newtype tuple struct (`pub struct TxnId(pub u64)`), which serde
 * renders as a bare number — not `{ "0": 1 }`. Same for the `txn` field on
 * every event.
 */

/** `crates/core/src/transaction.rs` */
export type TransactionState =
  | "Proposed"
  | "Screening"
  | "Cleared"
  | "Held"
  | "Released"
  | "Cancelled"
  | "Executed";

/**
 * `crates/core/src/linker.rs`. Crosses the wire as a variant name: a stable
 * key for the UI to switch on, never copy to display. See `reasons.ts` for
 * the words a person actually reads.
 */
export type PlainReason =
  | "EstablishedRecipient"
  | "NovelRecipientUnsolicitedContact"
  | "ScreeningUnavailable"
  | "UserReleased"
  | "CoolingPeriodNotElapsed";

/** `crates/core/src/events.rs` */
export type Component = "Reader" | "Linker" | "PolicyEngine";

export interface MoneyView {
  minor_units: number;
  currency: string;
}

export interface TxnView {
  id: number;
  state: TransactionState;
  amount: MoneyView;
  /** Masked server-side. The full number never leaves the store. */
  recipient: string;
  recipient_established: boolean;
  proposed_at: string;
  releases_at: string | null;
  reason: PlainReason | null;
  /**
   * Server-computed. A countdown rendered from `releases_at` is honest — that
   * is a server timestamp — but whether release is *permitted* is this field,
   * and the server checks it again on the release call regardless of what we
   * believe here.
   */
  releasable: boolean;
}

export interface HealthView {
  status: string;
  /** Goes false when the Reader process dies. Beat six. */
  reader_reachable: boolean;
  reader_mode: string;
  inbox_messages: number;
}

/** `crates/core/src/events.rs`, `#[serde(tag = "type")]`. */
export type AirlockEvent =
  | {
      type: "StateChanged";
      txn: number;
      from: TransactionState;
      to: TransactionState;
    }
  | {
      type: "HoldOpened";
      txn: number;
      reason: PlainReason;
      releases_at: string;
    }
  | { type: "ScreenFailed"; txn: number; component: Component };
