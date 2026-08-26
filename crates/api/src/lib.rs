//! Axum HTTP API and SSE event stream. Owned by Track B.
//!
//! The whole flow lives in [`AppState::propose`]: record, screen, decide,
//! emit. Two things are worth knowing before reading it.
//!
//! **Every state change goes through `TransactionState::transition`.** The
//! store holds transactions in many states at once so it uses the plain
//! enum rather than A's `Transaction<S>` typestate, but the same edges are
//! checked, and an illegal one is an error rather than a silent overwrite.
//!
//! **The API never decides anything.** It gathers facts, calls `decide`, and
//! does what it is told. Hold duration, hold reason and release eligibility
//! all come from `airlock-policy`; there is no branch in this file that
//! second-guesses them.

pub mod dto;
pub mod store;

use airlock_agents::{
    mask_msisdn, screen_reported, Linker, Reader, ScreeningReport, TransferFacts,
};
use airlock_core::{
    AirlockEvent, Component, InvalidTransition, Money, TransactionState, TxnId, Verdict,
};
use airlock_policy::{
    decide, release as policy_release, DecisionInput, InboundContact, PolicyDecision,
    RecipientProfile, ScreeningOutcome,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use dto::{
    ApiError, HealthView, InboundRequest, TransferRequest, TxnView,
};
use futures::stream::Stream;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use store::{Inbox, Ledger, TxnRecord, TxnStore};
use tokio::sync::broadcast;

/// How many events a slow SSE client may fall behind before it starts
/// missing them. Generous: the UI reconnects and refetches the snapshot.
const EVENT_BUFFER: usize = 256;

#[derive(Debug, thiserror::Error)]
pub enum ApiErr {
    #[error("{0}")]
    BadRequest(String),
    #[error("no such transaction")]
    NotFound,
    #[error("transfer is not on hold")]
    NotHeld,
    #[error("the cooling period has not elapsed yet")]
    TooEarly,
    #[error(transparent)]
    Transition(#[from] InvalidTransition),
}

impl IntoResponse for ApiErr {
    fn into_response(self) -> Response {
        let status = match self {
            ApiErr::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiErr::NotFound => StatusCode::NOT_FOUND,
            ApiErr::NotHeld | ApiErr::Transition(_) => StatusCode::CONFLICT,
            ApiErr::TooEarly => StatusCode::CONFLICT,
        };
        (status, Json(ApiError { error: self.to_string() })).into_response()
    }
}

pub struct AppState {
    pub ledger: Ledger,
    pub inbox: Mutex<Inbox>,
    pub txns: Mutex<TxnStore>,
    pub events: broadcast::Sender<AirlockEvent>,
    pub reader: Reader,
    pub linker: Linker,
}

impl AppState {
    pub fn new(reader: Reader) -> Arc<Self> {
        let (events, _) = broadcast::channel(EVENT_BUFFER);
        Arc::new(AppState {
            ledger: Ledger::seeded(),
            inbox: Mutex::new(Inbox::default()),
            // Seeded with prior payments that went straight through, so the
            // wallet can show how rarely anything is interrupted.
            txns: Mutex::new(TxnStore::seeded(Utc::now())),
            events,
            reader,
            linker: Linker::Stub,
        })
    }

    fn emit(&self, event: AirlockEvent) {
        // Fails only when nothing is listening, which is not an error.
        let _ = self.events.send(event);
    }

    /// Move a transaction to `to`, checking the edge and telling the world.
    /// The check is A's — this function has no opinion about which
    /// transitions are legal.
    fn advance(&self, id: TxnId, to: TransactionState) -> Result<(), ApiErr> {
        let from = {
            let mut txns = self.txns.lock().unwrap();
            let record = txns.get_mut(id).ok_or(ApiErr::NotFound)?;
            let from = record.state;
            record.state = from.transition(to)?;
            from
        };
        tracing::info!(txn_id = id.0, ?from, to = ?to, "state changed");
        self.emit(AirlockEvent::StateChanged { txn: id, from, to });
        Ok(())
    }

    fn view(&self, id: TxnId) -> Result<TxnView, ApiErr> {
        let txns = self.txns.lock().unwrap();
        let record = txns.get(id).ok_or(ApiErr::NotFound)?;
        Ok(TxnView::of(record, Utc::now()))
    }

    /// Record an inbound message. This is the judge-facing endpoint: the
    /// scam arrives the same way a real one would, and nothing about it is
    /// pre-recorded.
    pub fn record_inbound(&self, text: String) -> Result<usize, ApiErr> {
        if text.trim().is_empty() {
            return Err(ApiErr::BadRequest("message is empty".into()));
        }
        let mut inbox = self.inbox.lock().unwrap();
        inbox.record(text, Utc::now());
        // Length only — message content is never written to traces.
        tracing::info!(inbox_size = inbox.len(), "inbound message recorded");
        Ok(inbox.len())
    }

    /// Propose a transfer and run it through screening and policy.
    pub async fn propose(&self, req: TransferRequest) -> Result<TxnView, ApiErr> {
        let now = Utc::now();

        let masked = mask_msisdn(&req.recipient)
            .ok_or_else(|| ApiErr::BadRequest("recipient is not a phone number".into()))?;
        if req.amount_minor <= 0 {
            return Err(ApiErr::BadRequest("amount must be positive".into()));
        }
        let amount = Money {
            minor_units: req.amount_minor,
            currency: parse_currency(&req.currency)?,
        };
        let established = self.ledger.is_established(&req.recipient);

        let id = {
            let mut txns = self.txns.lock().unwrap();
            let id = txns.next_id();
            txns.insert(TxnRecord {
                id,
                state: TransactionState::Proposed,
                amount,
                recipient: req.recipient.clone(),
                masked: masked.clone(),
                recipient_established: established,
                proposed_at: now,
                releases_at: None,
                reason: None,
                claimed_authority: airlock_core::ClaimedAuthority::None,
                contact_received_at: None,
            });
            id
        };

        let span = tracing::info_span!("transaction", txn_id = id.0);
        let _guard = span.enter();

        self.advance(id, TransactionState::Screening)?;

        // Copy the message out from under the lock; screening awaits.
        let contact = {
            let inbox = self.inbox.lock().unwrap();
            inbox
                .most_recent_within_window(now)
                .map(|m| (m.text.clone(), m.received_at))
        };

        let report = match &contact {
            Some((text, received_at)) => {
                let facts = TransferFacts {
                    amount,
                    recipient: masked.clone(),
                    recipient_established: established,
                    minutes_since_contact: u32::try_from((now - *received_at).num_minutes()).ok(),
                };
                screen_reported(self.reader.clone(), self.linker, text.clone(), facts).await
            }
            // Nothing arrived in the correlation window, so there is nothing
            // for the transfer to be responsive *to*. Screening completed;
            // it simply had no link to find. Rule 2 needs `Responsive`, so
            // this passes on novelty alone — the acknowledged limitation in
            // the README about an attacker who waits out the window.
            None => ScreeningReport {
                outcome: ScreeningOutcome::Completed { verdict: Verdict::Unknown },
                claimed_authority: airlock_core::ClaimedAuthority::None,
            },
        };
        let screening = report.outcome;

        // Record what the message claimed to be, so the hold screen can say
        // who was impersonated. A variant, never text.
        {
            let mut txns = self.txns.lock().unwrap();
            let record = txns.get_mut(id).ok_or(ApiErr::NotFound)?;
            record.claimed_authority = report.claimed_authority;
            record.contact_received_at = contact.as_ref().map(|(_, at)| *at);
        }

        if screening == ScreeningOutcome::Unavailable {
            // In stub mode the Linker is in-process and infallible, so every
            // route to `Unavailable` runs through the Reader hop. Revisit
            // this attribution when the Linker becomes model-backed.
            tracing::warn!(txn_id = id.0, "screening unavailable");
            self.emit(AirlockEvent::ScreenFailed { txn: id, component: Component::Reader });
        }

        let decision = decide(&DecisionInput {
            recipient: RecipientProfile { established },
            inbound_contact: contact
                .as_ref()
                .map(|(_, at)| InboundContact { received_at: *at }),
            screening,
            proposed_at: now,
        });

        match decision {
            PolicyDecision::Pass => {
                tracing::info!(txn_id = id.0, outcome = "pass", "policy decided");
                self.advance(id, TransactionState::Cleared)?;
                self.advance(id, TransactionState::Executed)?;
            }
            PolicyDecision::Hold { reason, releases_at } => {
                tracing::info!(txn_id = id.0, outcome = "hold", ?reason, "policy decided");
                {
                    let mut txns = self.txns.lock().unwrap();
                    let record = txns.get_mut(id).ok_or(ApiErr::NotFound)?;
                    record.releases_at = Some(releases_at);
                    record.reason = Some(reason);
                }
                self.advance(id, TransactionState::Held)?;
                self.emit(AirlockEvent::HoldOpened { txn: id, reason, releases_at });
            }
        }

        self.view(id)
    }

    /// Release a held transfer. Rule 5: the user asks, and only after the
    /// cooling period. The gate is `airlock_policy::release`; this function
    /// does not decide, and a client that thinks the timer has run out does
    /// not get to be right about it.
    pub fn release(&self, id: TxnId) -> Result<TxnView, ApiErr> {
        let releases_at = {
            let txns = self.txns.lock().unwrap();
            let record = txns.get(id).ok_or(ApiErr::NotFound)?;
            if record.state != TransactionState::Held {
                return Err(ApiErr::NotHeld);
            }
            record.releases_at.ok_or(ApiErr::NotHeld)?
        };

        let reason = policy_release(releases_at, Utc::now()).map_err(|_| ApiErr::TooEarly)?;
        {
            let mut txns = self.txns.lock().unwrap();
            txns.get_mut(id).ok_or(ApiErr::NotFound)?.reason = Some(reason);
        }

        self.advance(id, TransactionState::Released)?;
        self.advance(id, TransactionState::Executed)?;
        self.view(id)
    }

    /// Cancel a held transfer. Also the user's call, and available
    /// immediately — there is no reason to make someone wait to *not* send
    /// money.
    pub fn cancel(&self, id: TxnId) -> Result<TxnView, ApiErr> {
        {
            let txns = self.txns.lock().unwrap();
            let record = txns.get(id).ok_or(ApiErr::NotFound)?;
            if record.state != TransactionState::Held {
                return Err(ApiErr::NotHeld);
            }
        }
        self.advance(id, TransactionState::Cancelled)?;
        self.view(id)
    }

    pub fn snapshot(&self) -> Vec<TxnView> {
        let now = Utc::now();
        let txns = self.txns.lock().unwrap();
        txns.all().iter().map(|r| TxnView::of(r, now)).collect()
    }
}

fn parse_currency(code: &str) -> Result<[u8; 3], ApiErr> {
    let bytes = code.as_bytes();
    if bytes.len() != 3 || !bytes.iter().all(u8::is_ascii_uppercase) {
        return Err(ApiErr::BadRequest(format!(
            "{code:?} is not a three-letter currency code"
        )));
    }
    let mut out = [0u8; 3];
    out.copy_from_slice(bytes);
    Ok(out)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

type Shared = State<Arc<AppState>>;

async fn inbound(State(app): Shared, Json(req): Json<InboundRequest>) -> Result<Json<serde_json::Value>, ApiErr> {
    let count = app.record_inbound(req.text)?;
    Ok(Json(serde_json::json!({ "recorded": true, "inbox_messages": count })))
}

async fn transfer(State(app): Shared, Json(req): Json<TransferRequest>) -> Result<Json<TxnView>, ApiErr> {
    Ok(Json(app.propose(req).await?))
}

async fn get_txn(State(app): Shared, Path(id): Path<u64>) -> Result<Json<TxnView>, ApiErr> {
    Ok(Json(app.view(TxnId(id))?))
}

async fn release_txn(State(app): Shared, Path(id): Path<u64>) -> Result<Json<TxnView>, ApiErr> {
    Ok(Json(app.release(TxnId(id))?))
}

async fn cancel_txn(State(app): Shared, Path(id): Path<u64>) -> Result<Json<TxnView>, ApiErr> {
    Ok(Json(app.cancel(TxnId(id))?))
}

/// Snapshot of every transaction. A UI that connects to the event stream
/// mid-flight would otherwise stare at an empty screen until something else
/// happens.
async fn transactions(State(app): Shared) -> Json<Vec<TxnView>> {
    Json(app.snapshot())
}

async fn health(State(app): Shared) -> Json<HealthView> {
    Json(HealthView {
        status: "ok",
        reader_reachable: app.reader.is_reachable().await,
        reader_mode: match app.reader {
            Reader::Stub => "stub",
            Reader::Remote { .. } => "remote",
        },
        inbox_messages: app.inbox.lock().unwrap().len(),
    })
}

/// The event stream. Everything the product surface renders arrives here —
/// no endpoint returns progress the UI is expected to animate on its own.
async fn events(State(app): Shared) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = app.events.subscribe();
    let stream = futures::stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let sse = Event::default()
                        .json_data(&event)
                        .unwrap_or_else(|_| Event::default().comment("unserialisable"));
                    return Some((Ok(sse), rx));
                }
                // A client that fell behind rejoins at the current position;
                // it refetches /transactions to resync.
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(missed = n, "sse client lagged");
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/events", get(events))
        .route("/inbound-sms", post(inbound))
        .route("/transfers", post(transfer))
        .route("/transactions", get(transactions))
        .route("/transactions/{id}", get(get_txn))
        .route("/transactions/{id}/release", post(release_txn))
        .route("/transactions/{id}/cancel", post(cancel_txn))
        .layer(tower_http::cors::CorsLayer::permissive())
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use airlock_core::PlainReason;

    const SCAM: &str =
        "MTN Alert: your account will be suspended today. Call 08031234567 to reactivate.";

    fn app() -> Arc<AppState> {
        AppState::new(Reader::Stub)
    }

    fn transfer_to(recipient: &str, amount_minor: i64) -> TransferRequest {
        TransferRequest {
            recipient: recipient.to_string(),
            amount_minor,
            currency: "NGN".to_string(),
        }
    }

    #[tokio::test]
    async fn an_established_recipient_passes_even_right_after_a_scam() {
        let app = app();
        app.record_inbound(SCAM.into()).unwrap();
        let view = app.propose(transfer_to("08055512345", 500_000)).await.unwrap();
        assert_eq!(view.state, TransactionState::Executed);
        assert!(view.recipient_established);
    }

    #[tokio::test]
    async fn a_novel_recipient_right_after_a_scam_is_held() {
        let app = app();
        app.record_inbound(SCAM.into()).unwrap();
        let view = app.propose(transfer_to("08031234567", 500_000)).await.unwrap();
        assert_eq!(view.state, TransactionState::Held);
        assert_eq!(view.reason, Some(PlainReason::NovelRecipientUnsolicitedContact));
        assert!(view.releases_at.is_some());
    }

    #[tokio::test]
    async fn a_novel_recipient_with_no_message_at_all_passes() {
        let app = app();
        let view = app.propose(transfer_to("08031234567", 500_000)).await.unwrap();
        assert_eq!(view.state, TransactionState::Executed);
    }

    #[tokio::test]
    async fn a_dead_reader_holds_a_novel_recipient() {
        let app = AppState::new(Reader::remote(
            "http://127.0.0.1:1",
            std::time::Duration::from_millis(200),
        ));
        app.record_inbound(SCAM.into()).unwrap();
        let view = app.propose(transfer_to("08031234567", 500_000)).await.unwrap();
        assert_eq!(view.state, TransactionState::Held);
        assert_eq!(view.reason, Some(PlainReason::ScreeningUnavailable));
    }

    #[tokio::test]
    async fn a_dead_reader_still_passes_an_established_recipient() {
        let app = AppState::new(Reader::remote(
            "http://127.0.0.1:1",
            std::time::Duration::from_millis(200),
        ));
        app.record_inbound(SCAM.into()).unwrap();
        let view = app.propose(transfer_to("08055512345", 500_000)).await.unwrap();
        assert_eq!(view.state, TransactionState::Executed);
    }

    #[tokio::test]
    async fn release_is_refused_while_the_hold_is_still_running() {
        let app = app();
        app.record_inbound(SCAM.into()).unwrap();
        let held = app.propose(transfer_to("08031234567", 500_000)).await.unwrap();
        assert!(!held.releasable);
        assert!(matches!(app.release(held.id), Err(ApiErr::TooEarly)));
        // And it is still held, not knocked into some other state.
        assert_eq!(app.view(held.id).unwrap().state, TransactionState::Held);
    }

    #[tokio::test]
    async fn a_held_transfer_can_be_cancelled_immediately() {
        let app = app();
        app.record_inbound(SCAM.into()).unwrap();
        let held = app.propose(transfer_to("08031234567", 500_000)).await.unwrap();
        let cancelled = app.cancel(held.id).unwrap();
        assert_eq!(cancelled.state, TransactionState::Cancelled);
        // Cancelled is terminal — releasing afterwards is not a thing.
        assert!(app.release(held.id).is_err());
    }

    #[tokio::test]
    async fn the_full_number_never_reaches_the_wire() {
        let app = app();
        app.record_inbound(SCAM.into()).unwrap();
        let view = app.propose(transfer_to("08031234567", 500_000)).await.unwrap();
        let json = serde_json::to_string(&view).unwrap();
        assert!(!json.contains("08031234567"), "full msisdn leaked: {json}");
        assert!(json.contains("*******567"));
    }

    #[tokio::test]
    async fn events_are_emitted_for_the_hold_path() {
        let app = app();
        let mut rx = app.events.subscribe();
        app.record_inbound(SCAM.into()).unwrap();
        app.propose(transfer_to("08031234567", 500_000)).await.unwrap();

        let mut seen = Vec::new();
        while let Ok(event) = rx.try_recv() {
            seen.push(event);
        }
        assert!(matches!(
            seen.first(),
            Some(AirlockEvent::StateChanged {
                from: TransactionState::Proposed,
                to: TransactionState::Screening,
                ..
            })
        ));
        assert!(seen.iter().any(|e| matches!(e, AirlockEvent::HoldOpened { .. })));
    }

    #[tokio::test]
    async fn a_dead_reader_emits_screen_failed_for_the_ui() {
        let app = AppState::new(Reader::remote(
            "http://127.0.0.1:1",
            std::time::Duration::from_millis(200),
        ));
        let mut rx = app.events.subscribe();
        app.record_inbound(SCAM.into()).unwrap();
        app.propose(transfer_to("08031234567", 500_000)).await.unwrap();

        let mut seen = Vec::new();
        while let Ok(event) = rx.try_recv() {
            seen.push(event);
        }
        assert!(seen.iter().any(|e| matches!(
            e,
            AirlockEvent::ScreenFailed { component: Component::Reader, .. }
        )));
    }

    #[tokio::test]
    async fn the_snapshot_reflects_what_happened() {
        let app = app();
        let before = app.snapshot().len();
        app.record_inbound(SCAM.into()).unwrap();
        let first = app.propose(transfer_to("08031234567", 500_000)).await.unwrap();
        let second = app.propose(transfer_to("08055512345", 100_000)).await.unwrap();

        let snapshot = app.snapshot();
        assert_eq!(snapshot.len(), before + 2);
        // Newest first, and the seeded history is still behind them.
        assert_eq!(snapshot[0].id, second.id);
        assert_eq!(snapshot[1].id, first.id);
    }

    /// The wallet's "N of your last 12 went straight through" line is
    /// computed from this history, so it has to be there and it has to look
    /// like payments that were never held.
    #[tokio::test]
    async fn the_wallet_starts_with_a_history_of_payments_that_passed() {
        let app = app();
        let history = app.snapshot();
        assert_eq!(history.len(), 12);
        assert!(history.iter().all(|t| t.state == TransactionState::Executed));
        assert!(history.iter().all(|t| t.reason.is_none()));
        assert!(history.iter().all(|t| t.recipient_established));
    }

    /// The hold screen names the impersonated institution, so the record has
    /// to carry it — as a variant, never as text from the message.
    #[tokio::test]
    async fn a_held_transfer_records_who_the_message_claimed_to_be() {
        let app = app();
        app.record_inbound(SCAM.into()).unwrap();
        let view = app.propose(transfer_to("08031234567", 500_000)).await.unwrap();
        assert_eq!(view.state, TransactionState::Held);
        assert_eq!(view.claimed_authority, airlock_core::ClaimedAuthority::Mtn);
        assert_eq!(view.minutes_since_contact, Some(0));
    }

    /// Nothing arrived, so there is nobody to have been impersonated.
    #[tokio::test]
    async fn a_transfer_with_no_message_claims_no_authority() {
        let app = app();
        let view = app.propose(transfer_to("08031234567", 500_000)).await.unwrap();
        assert_eq!(view.claimed_authority, airlock_core::ClaimedAuthority::None);
        assert_eq!(view.minutes_since_contact, None);
    }

    #[tokio::test]
    async fn a_malformed_recipient_is_a_bad_request_not_a_hold() {
        let app = app();
        assert!(matches!(
            app.propose(transfer_to("not-a-number", 500_000)).await,
            Err(ApiErr::BadRequest(_))
        ));
    }

    #[tokio::test]
    async fn a_nonpositive_amount_is_refused() {
        let app = app();
        assert!(matches!(
            app.propose(transfer_to("08031234567", 0)).await,
            Err(ApiErr::BadRequest(_))
        ));
    }

    #[test]
    fn currency_codes_are_checked() {
        assert!(parse_currency("NGN").is_ok());
        assert!(parse_currency("ngn").is_err());
        assert!(parse_currency("NG").is_err());
    }
}
