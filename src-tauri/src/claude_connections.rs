//! Short-lived native ownership of an observed Claude connection for review.
//!
//! Only the managed discovery command can publish an observation. A renderer
//! supplies an opaque ID, never account digests or model capabilities. This is
//! review state, not a qualified account route or permission to run inference.

use crate::{
    claude_agent::{ClaudeGenerationSettings, ClaudeReviewRuntime},
    claude_connection::ClaudeAuthMethod,
    claude_discovery::ClaudeModelDiscoveryObservation,
    claude_runtime::{
        self, ClaudeCatalogInstallation, ClaudeGenerationAvailability, ClaudeGenerationRuntime,
    },
    claude_worker::{ClaudeWorkerFailure, ClaudeWorkerFailureKind, RUN_TIMEOUT},
    provider, recovery,
};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
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
    operation: Option<ActiveOperation>,
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
    installation: ClaudeCatalogInstallation,
}

/// Display metadata only. ID possession cannot grant generation permission.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClaudeReviewConnection {
    connection_id: String,
    expires_at_unix_ms: u64,
    generation: ClaudeGenerationAvailability,
}

pub(crate) struct ClaudeReviewLease {
    pub runtime: Arc<ClaudeReviewRuntime>,
}

struct ActiveOperation {
    id: Uuid,
    origin_run_id: String,
    access: OperationIdentity,
}

#[derive(Clone)]
struct OperationIdentity {
    run_id: String,
    fingerprint: String,
    runtime: Arc<ClaudeReviewRuntime>,
    cancelled: Arc<AtomicBool>,
    invalidated: Arc<AtomicBool>,
    dispatched_at: Arc<OnceLock<Instant>>,
}

/// Owns only lifecycle, never product/account/isolation qualification. There
/// is exactly one native owner. Dropping its async caller revokes the shared
/// worker signal even if a blocking worker or validator is still running.
pub(crate) struct ClaudeOperationLease {
    access: ClaudeOperationAccess,
}

/// Cloneable access for blocking work. It cannot outlive revocation of the
/// non-cloneable owner or create a new operation/connection/consent.
#[derive(Clone)]
pub(crate) struct ClaudeOperationAccess {
    connections: ClaudeConnections,
    id: Uuid,
    identity: OperationIdentity,
}

impl ClaudeOperationLease {
    pub(crate) fn access(&self) -> ClaudeOperationAccess {
        self.access.clone()
    }
}

impl Drop for ClaudeOperationLease {
    fn drop(&mut self) {
        // Stop the worker before taking a lock that publication may own.
        self.access
            .identity
            .cancelled
            .store(true, Ordering::Release);
        if let Ok(mut state) = self.access.connections.0.lock() {
            if state
                .operation
                .as_ref()
                .is_some_and(|operation| operation.id == self.access.id)
            {
                state.operation = None;
            }
        }
    }
}

impl ClaudeOperationAccess {
    pub(crate) fn runtime(&self) -> &ClaudeReviewRuntime {
        &self.identity.runtime
    }

    pub(crate) fn cancel_signal(&self) -> Arc<AtomicBool> {
        self.identity.cancelled.clone()
    }

    pub(crate) fn started_at(&self) -> Result<Instant, provider::ProviderFailure> {
        self.identity
            .dispatched_at
            .get()
            .copied()
            .ok_or_else(changed)
    }

    /// A dispatched operation owns its inspected identity until completion.
    /// Review/catalog expiry cannot renew admission or invalidate that already
    /// consented run. Refresh, forget, cancellation and owner drop still win.
    pub(crate) fn check_review(
        &self,
        review: &provider::ProviderRequestReview,
    ) -> Result<(), provider::ProviderFailure> {
        let state = self.connections.lock().map_err(lifecycle_failure)?;
        self.check(&state, review, true)
    }

    /// Consume/journal under the connection lock with fresh admission checks.
    /// This is called only after the native sender checks explicit consent and
    /// revalidates source/settings. It grants no account/inference authority.
    pub(crate) fn consume_review(
        &self,
        review: &provider::ProviderRequestReview,
        reviews: &provider::PendingProviderReviews,
        interruptions: &recovery::InterruptionJournal,
        storage: &Path,
    ) -> Result<provider::PendingProviderRequest, provider::ProviderFailure> {
        let mut state = self.connections.lock().map_err(lifecycle_failure)?;
        self.check(&state, review, false)?;
        if self.identity.dispatched_at.get().is_some() {
            return Err(changed());
        }
        let connection = current(&mut state).map_err(lifecycle_failure)?;
        if !Arc::ptr_eq(&connection.runtime, &self.identity.runtime)
            || review.expires_at_unix_ms <= unix_time_ms()
        {
            return Err(changed());
        }
        let pending = reviews.consume(&review.review_id, &review.fingerprint)?;
        interruptions.record(storage, recovery::new_reviewed_provider_operation(&pending.review))
            .map_err(|_| provider::ProviderFailure {
                kind: provider::ProviderFailureKind::LocalStorageUnavailable,
                message: "Local interruption state could not be saved. No Claude request was sent; prepare a fresh review before retrying.".to_owned(),
            })?;
        // This clock also bounds queue time and result validation/publication,
        // rather than restarting the allowance when a worker thread starts.
        self.identity
            .dispatched_at
            .set(Instant::now())
            .map_err(|_| changed())?;
        Ok(pending)
    }

    /// Final publication is serialized with disconnect/refresh. Validation may
    /// run outside this lock, but creating a pending preview or saving a guide
    /// must happen inside it. If publication won, a subsequent disconnect does
    /// not erase that already-created unaccepted draft/history.
    pub(crate) fn publish<T>(
        &self,
        review: &provider::ProviderRequestReview,
        action: impl FnOnce() -> Result<T, provider::ProviderFailure>,
    ) -> Result<T, provider::ProviderFailure> {
        let state = self.connections.lock().map_err(lifecycle_failure)?;
        self.check(&state, review, true)?;
        action()
    }

    fn check(
        &self,
        state: &ConnectionsState,
        review: &provider::ProviderRequestReview,
        require_dispatch: bool,
    ) -> Result<(), provider::ProviderFailure> {
        if self.identity.invalidated.load(Ordering::Acquire)
            || !state.operation.as_ref().is_some_and(|operation| {
                operation.id == self.id
                    && Arc::ptr_eq(&operation.access.cancelled, &self.identity.cancelled)
                    && Arc::ptr_eq(&operation.access.runtime, &self.identity.runtime)
            })
        {
            return Err(changed());
        }
        if self.identity.cancelled.load(Ordering::Acquire) {
            return Err(provider::ProviderFailure {
                kind: provider::ProviderFailureKind::RequestCancelled,
                message: "The Claude operation was cancelled. No late response can be published; account usage may already have occurred.".to_owned(),
            });
        }
        if review.provider_id != "claude-agent"
            || review.review_id != self.identity.run_id
            || review.fingerprint != self.identity.fingerprint
            || review.generation_config["connectionId"] != self.identity.runtime.connection_id
        {
            return Err(changed());
        }
        if require_dispatch && self.started_at()?.elapsed() >= RUN_TIMEOUT {
            return Err(provider::ProviderFailure {
                kind: provider::ProviderFailureKind::RequestTimedOut,
                message: "The Claude operation exceeded ten minutes after review consumption. No late result can be published; account usage may already have occurred.".to_owned(),
            });
        }
        Ok(())
    }
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
        invalidate_operation(&state);
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
        let installation = ClaudeCatalogInstallation::observed(observation);
        let connection = ClaudeReviewConnection {
            connection_id: runtime.connection_id.clone(),
            expires_at_unix_ms,
            generation: claude_runtime::availability(&runtime, &installation),
        };
        state.checking = None;
        state.current = Some(Connection {
            origin_run_id: ticket.run_id.clone(),
            created: Instant::now(),
            expires_at_unix_ms,
            runtime,
            installation,
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
        let mut operation_cancelled = false;
        if let Some(operation) = &state.operation {
            if operation.origin_run_id == run_id {
                operation.access.invalidated.store(true, Ordering::Release);
                operation.access.cancelled.store(true, Ordering::Release);
                operation_cancelled = true;
            } else if operation.access.run_id == run_id {
                operation.access.cancelled.store(true, Ordering::Release);
                operation_cancelled = true;
            }
        }
        if checking {
            state.checking = None;
        }
        if current {
            state.current = None;
        }
        checking || current || operation_cancelled
    }

    /// Compare-and-forget avoids an old window disconnecting a newer check.
    pub(crate) fn forget(&self, connection_id: &str) -> Result<bool, ClaudeWorkerFailure> {
        let mut state = self.lock()?;
        let operation_matches = state
            .operation
            .as_ref()
            .is_some_and(|operation| operation.access.runtime.connection_id == connection_id);
        if operation_matches {
            invalidate_operation(&state);
        }
        let current_matches = state
            .current
            .as_ref()
            .is_some_and(|current| current.runtime.connection_id == connection_id);
        if current_matches {
            state.current = None;
        }
        Ok(current_matches || operation_matches)
    }

    /// Reserve native lifecycle state using exactly the observed runtime Arc.
    /// The caller still owns account/runtime/isolation qualification and must
    /// check explicit consent before reaching this method. Observations and a
    /// lifecycle lease alone cannot enable an inference command.
    pub(crate) fn begin_operation(
        &self,
        runtime: &Arc<ClaudeReviewRuntime>,
        review: &provider::ProviderRequestReview,
    ) -> Result<ClaudeOperationLease, provider::ProviderFailure> {
        let mut state = self.lock().map_err(lifecycle_failure)?;
        if state.operation.is_some() {
            return Err(provider::ProviderFailure {
                kind: provider::ProviderFailureKind::ReviewUnavailable,
                message: "Another Claude operation or result is still active. Wait for it to finish or cancel before preparing a new run.".to_owned(),
            });
        }
        let current = current(&mut state).map_err(lifecycle_failure)?;
        if !Arc::ptr_eq(&current.runtime, runtime)
            || review.provider_id != "claude-agent"
            || review.generation_config["connectionId"] != runtime.connection_id
            || Uuid::parse_str(&review.review_id).is_err()
            || review.expires_at_unix_ms <= unix_time_ms()
        {
            return Err(changed());
        }
        let origin_run_id = current.origin_run_id.clone();
        let id = Uuid::new_v4();
        let identity = OperationIdentity {
            run_id: review.review_id.clone(),
            fingerprint: review.fingerprint.clone(),
            runtime: runtime.clone(),
            cancelled: Arc::new(AtomicBool::new(false)),
            invalidated: Arc::new(AtomicBool::new(false)),
            dispatched_at: Arc::new(OnceLock::new()),
        };
        state.operation = Some(ActiveOperation {
            id,
            origin_run_id,
            access: identity.clone(),
        });
        Ok(ClaudeOperationLease {
            access: ClaudeOperationAccess {
                connections: self.clone(),
                id,
                identity,
            },
        })
    }

    /// Resolve the current native observation and the release-owned admission
    /// policy. Neither connection IDs nor observed metadata mint permission.
    /// The pipeline rechecks this exact Arc under its revocable owner before
    /// consuming the separately consented review.
    pub(crate) fn generation_runtime(
        &self,
        review: &provider::ProviderRequestReview,
    ) -> Result<ClaudeGenerationRuntime, provider::ProviderFailure> {
        let mut state = self.lock().map_err(lifecycle_failure)?;
        let current = current(&mut state).map_err(lifecycle_failure)?;
        if review.provider_id != "claude-agent"
            || review.generation_config["connectionId"] != current.runtime.connection_id
            || review.expires_at_unix_ms <= unix_time_ms()
        {
            return Err(changed());
        }
        claude_runtime::admit(&current.runtime, &current.installation)
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

fn invalidate_operation(state: &ConnectionsState) {
    if let Some(operation) = &state.operation {
        operation.access.invalidated.store(true, Ordering::Release);
        operation.access.cancelled.store(true, Ordering::Release);
    }
}

fn changed() -> provider::ProviderFailure {
    provider::ProviderFailure {
        kind: provider::ProviderFailureKind::ReviewChanged,
        message: REFRESH.to_owned(),
    }
}

fn lifecycle_failure(error: ClaudeWorkerFailure) -> provider::ProviderFailure {
    let kind = match error.kind {
        ClaudeWorkerFailureKind::Cancelled => provider::ProviderFailureKind::RequestCancelled,
        ClaudeWorkerFailureKind::RuntimeChanged | ClaudeWorkerFailureKind::ReviewChanged => {
            provider::ProviderFailureKind::ReviewChanged
        }
        _ => provider::ProviderFailureKind::ReviewUnavailable,
    };
    provider::ProviderFailure {
        kind,
        message: error.message,
    }
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| u64::try_from(elapsed.as_millis()).ok())
        .unwrap_or(u64::MAX)
}

fn failure(kind: ClaudeWorkerFailureKind, message: &str) -> ClaudeWorkerFailure {
    ClaudeWorkerFailure {
        kind,
        message: message.to_owned(),
    }
}
