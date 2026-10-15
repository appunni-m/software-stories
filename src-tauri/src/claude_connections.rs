//! Short-lived native ownership of an observed Claude connection for review.
//!
//! Only the managed discovery command can publish an observation. A renderer
//! supplies an opaque ID, never account digests or model capabilities. This is
//! review state, not a qualified account route or permission to run inference.

use crate::{
    claude_agent::{ClaudeGenerationSettings, ClaudeReviewRuntime},
    claude_connection::ClaudeAuthMethod,
    claude_discovery::ClaudeModelDiscoveryObservation,
    claude_worker::{ClaudeWorkerFailure, ClaudeWorkerFailureKind},
    provider,
};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

const CONNECTION_TTL: Duration = Duration::from_secs(5 * 60);
const REFRESH: &str =
    "This Claude connection changed or expired. Check its catalog and prepare a fresh review.";

#[derive(Clone, Default)]
pub(crate) struct ClaudeConnections(Arc<Mutex<ConnectionsState>>);

#[derive(Default)]
struct ConnectionsState {
    checking: Option<InspectionTicket>,
    current: Option<Connection>,
}

#[derive(Clone)]
pub(crate) struct InspectionTicket {
    id: Uuid,
    run_id: String,
}

struct Connection {
    origin_run_id: String,
    created: Instant,
    expires_at_unix_ms: u64,
    runtime: Arc<ClaudeReviewRuntime>,
}

/// Display metadata only. ID possession cannot grant generation permission.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClaudeReviewConnection {
    connection_id: String,
    expires_at_unix_ms: u64,
}

pub(crate) struct ClaudeReviewLease {
    pub runtime: Arc<ClaudeReviewRuntime>,
}

impl ClaudeConnections {
    /// Invalidate before any new check, including one that later fails. An
    /// older completion cannot restore a connection after refresh/disconnect.
    pub(crate) fn begin_inspection(
        &self,
        run_id: &str,
    ) -> Result<InspectionTicket, ClaudeWorkerFailure> {
        if Uuid::parse_str(run_id).is_err() {
            return Err(failure(
                ClaudeWorkerFailureKind::PreparationFailed,
                "The Claude inspection identity is invalid. Start a fresh check.",
            ));
        }
        let ticket = InspectionTicket {
            id: Uuid::new_v4(),
            run_id: run_id.to_owned(),
        };
        let mut state = self.lock()?;
        state.current = None;
        state.checking = Some(ticket.clone());
        Ok(ticket)
    }

    pub(crate) fn finish_installation(
        &self,
        ticket: &InspectionTicket,
    ) -> Result<(), ClaudeWorkerFailure> {
        let mut state = self.lock()?;
        require_ticket(&state, ticket)?;
        state.checking = None;
        Ok(())
    }

    pub(crate) fn finish_catalog(
        &self,
        ticket: &InspectionTicket,
        observation: &ClaudeModelDiscoveryObservation,
    ) -> Result<ClaudeReviewConnection, ClaudeWorkerFailure> {
        use ClaudeWorkerFailureKind as Kind;
        let observed = &observation.connection;
        let account = observed
            .account
            .as_ref()
            .filter(|account| {
                account.logged_in
                    && account.first_party_provider
                    && account.auth_method == ClaudeAuthMethod::ClaudeAccount
            })
            .and_then(|account| account.account_identity_sha256.as_ref())
            .ok_or_else(|| failure(Kind::RuntimeChanged, REFRESH))?;
        if !observed.candidate_version_matches {
            return Err(failure(Kind::RuntimeChanged, REFRESH));
        }
        // These values have already passed native SDK/catalog admission. They
        // describe this observation only; they do not establish entitlement.
        let runtime = Arc::new(ClaudeReviewRuntime {
            connection_id: Uuid::new_v4().to_string(),
            executable_path: PathBuf::from(&observed.executable_path),
            executable_sha256: observed.executable_sha256.clone(),
            runtime_version: observed.runtime_version.clone(),
            account_identity_sha256: account.clone(),
            models: observation.catalog.capabilities(),
        });
        let first_model = runtime.models.first().ok_or_else(|| {
            failure(
                Kind::ProtocolRejected,
                "Claude returned no exact model capabilities. No chapter review is available.",
            )
        })?;
        runtime
            .validate_settings(&ClaudeGenerationSettings {
                model_id: first_model.model_id.clone(),
                effort: None,
                approval_policy: "dontAsk".to_owned(),
                max_turns: 1,
            })
            .map_err(|_| {
                failure(
                    Kind::ProtocolRejected,
                    "Claude connection metadata is incomplete. No chapter review is available.",
                )
            })?;
        let expires_at_unix_ms = SystemTime::now().duration_since(UNIX_EPOCH).ok()
            .and_then(|elapsed| elapsed.checked_add(CONNECTION_TTL))
            .and_then(|elapsed| u64::try_from(elapsed.as_millis()).ok())
            .ok_or_else(|| failure(Kind::PreparationFailed,
                "The local clock could not date this connection. No chapter review is available."))?;
        let mut state = self.lock()?;
        require_ticket(&state, ticket)?;
        let connection = ClaudeReviewConnection {
            connection_id: runtime.connection_id.clone(),
            expires_at_unix_ms,
        };
        state.checking = None;
        state.current = Some(Connection {
            origin_run_id: ticket.run_id.clone(),
            created: Instant::now(),
            expires_at_unix_ms,
            runtime,
        });
        Ok(connection)
    }

    pub(crate) fn abandon_inspection(&self, ticket: &InspectionTicket) {
        if let Ok(mut state) = self.0.lock() {
            if state
                .checking
                .as_ref()
                .is_some_and(|active| active.id == ticket.id)
            {
                state.checking = None;
            }
        }
    }

    /// Cover cancellation after the worker returns but before publication or
    /// UI delivery. A different operation's cancellation cannot clear state.
    pub(crate) fn cancel_operation(&self, run_id: &str) -> bool {
        let Ok(mut state) = self.0.lock() else {
            return false;
        };
        let checking = state
            .checking
            .as_ref()
            .is_some_and(|active| active.run_id == run_id);
        let current = state
            .current
            .as_ref()
            .is_some_and(|current| current.origin_run_id == run_id);
        if checking {
            state.checking = None;
        }
        if current {
            state.current = None;
        }
        checking || current
    }

    /// Compare-and-forget avoids an old window disconnecting a newer check.
    pub(crate) fn forget(&self, connection_id: &str) -> Result<bool, ClaudeWorkerFailure> {
        let mut state = self.lock()?;
        if state
            .current
            .as_ref()
            .is_some_and(|current| current.runtime.connection_id == connection_id)
        {
            state.current = None;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub(crate) fn review_lease(
        &self,
        connection_id: &str,
    ) -> Result<ClaudeReviewLease, ClaudeWorkerFailure> {
        let mut state = self.lock()?;
        let current = current(&mut state)?;
        if current.runtime.connection_id != connection_id {
            return Err(failure(ClaudeWorkerFailureKind::RuntimeChanged, REFRESH));
        }
        Ok(ClaudeReviewLease {
            runtime: current.runtime.clone(),
        })
    }

    /// Recheck after blocking source preparation and insert under the same
    /// connection lock. Disconnect/refresh cannot race a stale review into the
    /// store. The review expires no later than this monotonic-bounded lease.
    pub(crate) fn insert_review(
        &self,
        lease: &ClaudeReviewLease,
        reviews: &provider::PendingProviderReviews,
        mut pending: provider::PendingProviderRequest,
    ) -> Result<provider::ProviderRequestReview, provider::ProviderFailure> {
        let changed = || provider::ProviderFailure {
            kind: provider::ProviderFailureKind::ReviewChanged,
            message: REFRESH.to_owned(),
        };
        let mut state = self.lock().map_err(|_| changed())?;
        let current = current(&mut state).map_err(|_| changed())?;
        if !Arc::ptr_eq(&current.runtime, &lease.runtime)
            || pending.review.provider_id != "claude-agent"
            || pending.review.generation_config["connectionId"] != current.runtime.connection_id
        {
            return Err(changed());
        }
        pending.review.expires_at_unix_ms = pending
            .review
            .expires_at_unix_ms
            .min(current.expires_at_unix_ms);
        reviews.insert(pending)
    }

    fn lock(&self) -> Result<MutexGuard<'_, ConnectionsState>, ClaudeWorkerFailure> {
        self.0.lock().map_err(|_| {
            failure(
                ClaudeWorkerFailureKind::PreparationFailed,
                "Claude connection state is unavailable. Reopen setup and prepare a fresh check.",
            )
        })
    }
}

fn current(state: &mut ConnectionsState) -> Result<&Connection, ClaudeWorkerFailure> {
    if state
        .current
        .as_ref()
        .is_some_and(|current| current.created.elapsed() >= CONNECTION_TTL)
    {
        state.current = None;
    }
    state
        .current
        .as_ref()
        .ok_or_else(|| failure(ClaudeWorkerFailureKind::RuntimeChanged, REFRESH))
}

fn require_ticket(
    state: &ConnectionsState,
    ticket: &InspectionTicket,
) -> Result<(), ClaudeWorkerFailure> {
    if state
        .checking
        .as_ref()
        .is_some_and(|active| active.id == ticket.id)
    {
        Ok(())
    } else {
        Err(failure(
            ClaudeWorkerFailureKind::Cancelled,
            "This Claude check was superseded or cancelled. Its metadata was not retained.",
        ))
    }
}

fn failure(kind: ClaudeWorkerFailureKind, message: &str) -> ClaudeWorkerFailure {
    ClaudeWorkerFailure {
        kind,
        message: message.to_owned(),
    }
}
