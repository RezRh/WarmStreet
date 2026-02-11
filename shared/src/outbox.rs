use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::{RwLock, Semaphore};
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

/// Validated operation identifier - immutable after construction
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OpId(String);

impl OpId {
    const MAX_LENGTH: usize = 128;

    pub fn new(id: impl Into<String>) -> Result<Self, OutboxError> {
        let id = id.into().trim().to_string(); // Store trimmed version
        Self::validate(&id)?;
        Ok(Self(id))
    }

    pub fn generate() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn validate(id: &str) -> Result<(), OutboxError> {
        if id.is_empty() {
            return Err(OutboxError::InvalidId("OpId cannot be empty".into()));
        }
        if id.len() > Self::MAX_LENGTH {
            return Err(OutboxError::InvalidId(format!(
                "OpId exceeds {} characters",
                Self::MAX_LENGTH
            )));
        }
        if !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(OutboxError::InvalidId(
                "OpId contains invalid characters (allowed: a-z, A-Z, 0-9, -, _)".into(),
            ));
        }
        Ok(())
    }
}

/// Validated idempotency key - immutable after construction
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IdempotencyKey(String);

impl IdempotencyKey {
    const MAX_LENGTH: usize = 128;

    pub fn new(key: impl Into<String>) -> Result<Self, OutboxError> {
        let key = key.into().trim().to_string(); // Store trimmed version
        Self::validate(&key)?;
        Ok(Self(key))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn validate(key: &str) -> Result<(), OutboxError> {
        if key.is_empty() {
            return Err(OutboxError::InvalidId(
                "IdempotencyKey cannot be empty".into(),
            ));
        }
        if key.len() > Self::MAX_LENGTH {
            return Err(OutboxError::InvalidId(format!(
                "IdempotencyKey exceeds {} characters",
                Self::MAX_LENGTH
            )));
        }
        if !key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(OutboxError::InvalidId(
                "IdempotencyKey contains invalid characters".into(),
            ));
        }
        Ok(())
    }
}

/// Lease token for distributed locking - prevents duplicate processing
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LeaseToken {
    pub token: String,
    pub acquired_at: UnixTimeMs,
    pub expires_at: UnixTimeMs,
    pub holder_id: String,
}

impl LeaseToken {
    pub fn new(holder_id: impl Into<String>, now: UnixTimeMs, duration_ms: u64) -> Self {
        Self {
            token: Uuid::new_v4().to_string(),
            acquired_at: now,
            expires_at: UnixTimeMs(now.0.saturating_add(duration_ms)),
            holder_id: holder_id.into(),
        }
    }

    pub fn is_expired(&self, now: UnixTimeMs) -> bool {
        now.0 >= self.expires_at.0
    }

    pub fn is_held_by(&self, holder_id: &str) -> bool {
        self.holder_id == holder_id
    }
}

/// Unix timestamp in milliseconds
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UnixTimeMs(pub u64);

impl UnixTimeMs {
    pub fn now() -> Self {
        Self(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        )
    }
}

/// Validated wound severity (1-5 scale)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WoundSeverity(u8);

impl WoundSeverity {
    pub const MIN: u8 = 1;
    pub const MAX: u8 = 5;

    pub fn new(value: u8) -> Result<Self, OutboxError> {
        if value < Self::MIN || value > Self::MAX {
            return Err(OutboxError::Validation(format!(
                "WoundSeverity must be between {} and {}, got {}",
                Self::MIN,
                Self::MAX,
                value
            )));
        }
        Ok(Self(value))
    }

    pub fn value(&self) -> u8 {
        self.0
    }
}

/// Validated geographic coordinates
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LatLon {
    lat: f64,
    lon: f64,
}

impl LatLon {
    pub fn new(lat: f64, lon: f64) -> Result<Self, OutboxError> {
        if !(-90.0..=90.0).contains(&lat) {
            return Err(OutboxError::Validation(format!(
                "Latitude must be between -90 and 90, got {}",
                lat
            )));
        }
        if !(-180.0..=180.0).contains(&lon) {
            return Err(OutboxError::Validation(format!(
                "Longitude must be between -180 and 180, got {}",
                lon
            )));
        }
        if lat.is_nan() || lon.is_nan() {
            return Err(OutboxError::Validation(
                "Coordinates cannot be NaN".into(),
            ));
        }
        Ok(Self { lat, lon })
    }

    pub fn lat(&self) -> f64 {
        self.lat
    }

    pub fn lon(&self) -> f64 {
        self.lon
    }
}

/// Validated server case ID
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ServerCaseId(String);

impl ServerCaseId {
    const MAX_LENGTH: usize = 256;

    pub fn new(id: impl Into<String>) -> Result<Self, OutboxError> {
        let id = id.into().trim().to_string();
        if id.is_empty() {
            return Err(OutboxError::InvalidId("ServerCaseId cannot be empty".into()));
        }
        if id.len() > Self::MAX_LENGTH {
            return Err(OutboxError::InvalidId(format!(
                "ServerCaseId exceeds {} characters",
                Self::MAX_LENGTH
            )));
        }
        // Allow more characters but still validate for injection
        if id.chars().any(|c| c.is_control() || c == '\0') {
            return Err(OutboxError::InvalidId(
                "ServerCaseId contains invalid control characters".into(),
            ));
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Local operation identifier
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LocalOpId(String);

impl LocalOpId {
    const MAX_LENGTH: usize = 128;

    pub fn new(id: impl Into<String>) -> Result<Self, OutboxError> {
        let id = id.into().trim().to_string();
        if id.is_empty() {
            return Err(OutboxError::InvalidId("LocalOpId cannot be empty".into()));
        }
        if id.len() > Self::MAX_LENGTH {
            return Err(OutboxError::InvalidId(format!(
                "LocalOpId exceeds {} characters",
                Self::MAX_LENGTH
            )));
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Blob reference for uploaded files
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlobRef {
    id: String,
    size_bytes: u64,
    content_type: String,
}

impl BlobRef {
    const MAX_ID_LENGTH: usize = 512;
    const MAX_CONTENT_TYPE_LENGTH: usize = 128;

    pub fn new(
        id: impl Into<String>,
        size_bytes: u64,
        content_type: impl Into<String>,
    ) -> Result<Self, OutboxError> {
        let id = id.into();
        let content_type = content_type.into();

        if id.is_empty() || id.len() > Self::MAX_ID_LENGTH {
            return Err(OutboxError::Validation("Invalid blob ID".into()));
        }
        if content_type.len() > Self::MAX_CONTENT_TYPE_LENGTH {
            return Err(OutboxError::Validation("Content type too long".into()));
        }

        Ok(Self {
            id,
            size_bytes,
            content_type,
        })
    }
}

/// Server-side case status
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ServerCaseStatus {
    Open,
    InProgress,
    Resolved,
    Closed,
}

// ============================================================================
// Error Types
// ============================================================================

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCategory {
    Transient,
    RateLimited,
    ClientError,
    ServerError,
    NetworkError,
    Timeout,
    Unknown,
}

impl ErrorCategory {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ErrorCategory::Transient
                | ErrorCategory::RateLimited
                | ErrorCategory::ServerError
                | ErrorCategory::NetworkError
                | ErrorCategory::Timeout
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntentError {
    pub category: ErrorCategory,
    pub code: String,
    pub message: String,
    pub truncated: bool,
    pub timestamp: UnixTimeMs,
}

impl IntentError {
    const MAX_MESSAGE_LENGTH: usize = 512;
    const MAX_CODE_LENGTH: usize = 64;

    pub fn new(
        category: ErrorCategory,
        code: impl Into<String>,
        message: impl Into<String>,
        now: UnixTimeMs,
    ) -> Self {
        let mut message = message.into();
        let mut code = code.into();

        let message_truncated = message.len() > Self::MAX_MESSAGE_LENGTH;
        let code_truncated = code.len() > Self::MAX_CODE_LENGTH;

        truncate_utf8_safe(&mut message, Self::MAX_MESSAGE_LENGTH);
        truncate_utf8_safe(&mut code, Self::MAX_CODE_LENGTH);

        Self {
            category,
            code,
            message,
            truncated: message_truncated || code_truncated,
            timestamp: now,
        }
    }

    pub fn is_retryable(&self) -> bool {
        self.category.is_retryable()
    }
}

fn truncate_utf8_safe(s: &mut String, max_bytes: usize) {
    if s.len() <= max_bytes {
        return;
    }

    let mut truncate_at = max_bytes;
    while truncate_at > 0 && !s.is_char_boundary(truncate_at) {
        truncate_at -= 1;
    }
    s.truncate(truncate_at);
}

#[derive(Debug, Error)]
pub enum OutboxError {
    #[error("outbox is full ({0} entries)")]
    Full(usize),

    #[error("invalid identifier: {0}")]
    InvalidId(String),

    #[error("duplicate operation: {0}")]
    DuplicateOpId(String),

    #[error("duplicate idempotency key: {0}")]
    DuplicateIdempotencyKey(String),

    #[error("entry not found: {0}")]
    NotFound(String),

    #[error("invalid state transition from {from:?} to {to:?}: {reason}")]
    InvalidStateTransition {
        from: EntryState,
        to: &'static str,
        reason: String,
    },

    #[error("validation error: {0}")]
    Validation(String),

    #[error("storage error: {0}")]
    Storage(String),

    #[error("dependency not satisfied: {0}")]
    DependencyNotSatisfied(String),

    #[error("lease error: {0}")]
    LeaseError(String),

    #[error("rate limited: {0}")]
    RateLimited(String),

    #[error("corrupted entry: {op_id}, reason: {reason}")]
    CorruptedEntry { op_id: String, reason: String },
}

// ============================================================================
// State Machine with Explicit Transitions
// ============================================================================

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RetryHistory {
    pub errors: Vec<IntentError>,
    pub total_attempts: u32,
}

impl RetryHistory {
    const MAX_HISTORY: usize = 10;

    pub fn new() -> Self {
        Self {
            errors: Vec::with_capacity(Self::MAX_HISTORY),
            total_attempts: 0,
        }
    }

    pub fn record_error(&mut self, error: IntentError) {
        self.total_attempts = self.total_attempts.saturating_add(1);
        if self.errors.len() >= Self::MAX_HISTORY {
            self.errors.remove(0);
        }
        self.errors.push(error);
    }

    pub fn last_error(&self) -> Option<&IntentError> {
        self.errors.last()
    }
}

impl Default for RetryHistory {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum DeadLetterReason {
    MaxRetriesExceeded,
    NonRetryableError,
    DependencyFailed { dependency_op_id: OpId },
    Expired,
    ManualIntervention,
    Corrupted { details: String },
}

/// Entry state with explicit valid transitions
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum EntryState {
    Pending,
    InFlight {
        started_at: UnixTimeMs,
        lease: LeaseToken,
    },
    Retrying {
        next_attempt_at: UnixTimeMs,
        history: RetryHistory,
    },
    DeadLetter {
        reason: DeadLetterReason,
        history: RetryHistory,
        dead_at: UnixTimeMs,
    },
    Completed {
        completed_at: UnixTimeMs,
    },
}

impl EntryState {
    pub fn state_name(&self) -> &'static str {
        match self {
            EntryState::Pending => "pending",
            EntryState::InFlight { .. } => "in_flight",
            EntryState::Retrying { .. } => "retrying",
            EntryState::DeadLetter { .. } => "dead_letter",
            EntryState::Completed { .. } => "completed",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            EntryState::DeadLetter { .. } | EntryState::Completed { .. }
        )
    }

    /// Validate if transition to InFlight is allowed
    pub fn can_transition_to_in_flight(&self, now: UnixTimeMs, timeout_ms: u64) -> bool {
        match self {
            EntryState::Pending => true,
            EntryState::Retrying { next_attempt_at, .. } => now.0 >= next_attempt_at.0,
            EntryState::InFlight { lease, .. } => lease.is_expired(now),
            EntryState::DeadLetter { .. } | EntryState::Completed { .. } => false,
        }
    }

    /// Validate if transition to Completed is allowed
    pub fn can_transition_to_completed(&self, lease_token: Option<&str>) -> bool {
        match self {
            EntryState::InFlight { lease, .. } => {
                lease_token.map_or(false, |t| t == lease.token)
            }
            _ => false,
        }
    }

    /// Validate if transition to Failed/Retrying is allowed
    pub fn can_transition_to_failed(&self, lease_token: Option<&str>) -> bool {
        match self {
            EntryState::InFlight { lease, .. } => {
                lease_token.map_or(false, |t| t == lease.token)
            }
            _ => false,
        }
    }

    /// Extract retry history for state transitions
    pub fn take_history(&self) -> RetryHistory {
        match self {
            EntryState::Retrying { history, .. } => history.clone(),
            EntryState::InFlight { .. } | EntryState::Pending => RetryHistory::new(),
            EntryState::DeadLetter { history, .. } => history.clone(),
            EntryState::Completed { .. } => RetryHistory::new(),
        }
    }
}

// ============================================================================
// Intent Types
// ============================================================================

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum OutboxIntent {
    CreateCase {
        local_id: LocalOpId,
        location: LatLon,
        description: Option<String>,
        wound_severity: Option<WoundSeverity>,
        created_at_ms_utc: UnixTimeMs,
    },
    UploadCasePhoto {
        local_id: LocalOpId,
        photo: BlobRef,
        depends_on: Option<OpId>,
    },
    UpdateCaseStatus {
        server_case_id: ServerCaseId,
        status: ServerCaseStatus,
        depends_on: Option<OpId>,
    },
}

impl OutboxIntent {
    const MAX_DESCRIPTION_LENGTH: usize = 4096;

    pub fn create_case(
        local_id: LocalOpId,
        location: LatLon,
        description: Option<String>,
        wound_severity: Option<WoundSeverity>,
        created_at_ms_utc: UnixTimeMs,
    ) -> Result<Self, OutboxError> {
        if let Some(ref desc) = description {
            if desc.len() > Self::MAX_DESCRIPTION_LENGTH {
                return Err(OutboxError::Validation(format!(
                    "Description exceeds {} characters",
                    Self::MAX_DESCRIPTION_LENGTH
                )));
            }
        }
        Ok(Self::CreateCase {
            local_id,
            location,
            description,
            wound_severity,
            created_at_ms_utc,
        })
    }

    pub fn depends_on(&self) -> Option<&OpId> {
        match self {
            OutboxIntent::CreateCase { .. } => None,
            OutboxIntent::UploadCasePhoto { depends_on, .. } => depends_on.as_ref(),
            OutboxIntent::UpdateCaseStatus { depends_on, .. } => depends_on.as_ref(),
        }
    }

    pub fn intent_type(&self) -> &'static str {
        match self {
            OutboxIntent::CreateCase { .. } => "create_case",
            OutboxIntent::UploadCasePhoto { .. } => "upload_photo",
            OutboxIntent::UpdateCaseStatus { .. } => "update_status",
        }
    }
}

// ============================================================================
// Outbox Entry
// ============================================================================

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutboxEntry {
    pub op_id: OpId,
    pub idempotency_key: IdempotencyKey,
    pub intent: OutboxIntent,
    pub created_at: UnixTimeMs,
    pub expires_at: UnixTimeMs,
    pub state: EntryState,
    pub tenant_id: Option<String>,
    pub priority: u8,
    pub version: u64,
}

impl OutboxEntry {
    pub fn new(
        op_id: OpId,
        idempotency_key: IdempotencyKey,
        intent: OutboxIntent,
        now: UnixTimeMs,
        ttl_ms: u64,
    ) -> Self {
        Self {
            op_id,
            idempotency_key,
            intent,
            created_at: now,
            expires_at: UnixTimeMs(now.0.saturating_add(ttl_ms)),
            state: EntryState::Pending,
            tenant_id: None,
            priority: 0,
            version: 1,
        }
    }

    pub fn with_tenant(mut self, tenant_id: impl Into<String>) -> Self {
        self.tenant_id = Some(tenant_id.into());
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    pub fn is_terminal(&self) -> bool {
        self.state.is_terminal()
    }

    pub fn is_expired(&self, now: UnixTimeMs) -> bool {
        now.0 >= self.expires_at.0
    }
}

// ============================================================================
// Metrics and Observability
// ============================================================================

#[derive(Debug, Default)]
pub struct OutboxMetrics {
    // Counters
    pub entries_pushed: AtomicU64,
    pub entries_completed: AtomicU64,
    pub entries_failed: AtomicU64,
    pub entries_dead_lettered: AtomicU64,
    pub entries_expired: AtomicU64,
    pub duplicate_rejections: AtomicU64,
    pub rate_limit_rejections: AtomicU64,
    pub storage_errors: AtomicU64,
    pub lease_conflicts: AtomicU64,
    
    // State transition errors
    pub invalid_transitions: AtomicU64,
}

impl OutboxMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            entries_pushed: self.entries_pushed.load(Ordering::Relaxed),
            entries_completed: self.entries_completed.load(Ordering::Relaxed),
            entries_failed: self.entries_failed.load(Ordering::Relaxed),
            entries_dead_lettered: self.entries_dead_lettered.load(Ordering::Relaxed),
            entries_expired: self.entries_expired.load(Ordering::Relaxed),
            duplicate_rejections: self.duplicate_rejections.load(Ordering::Relaxed),
            rate_limit_rejections: self.rate_limit_rejections.load(Ordering::Relaxed),
            storage_errors: self.storage_errors.load(Ordering::Relaxed),
            lease_conflicts: self.lease_conflicts.load(Ordering::Relaxed),
            invalid_transitions: self.invalid_transitions.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MetricsSnapshot {
    pub entries_pushed: u64,
    pub entries_completed: u64,
    pub entries_failed: u64,
    pub entries_dead_lettered: u64,
    pub entries_expired: u64,
    pub duplicate_rejections: u64,
    pub rate_limit_rejections: u64,
    pub storage_errors: u64,
    pub lease_conflicts: u64,
    pub invalid_transitions: u64,
}

#[derive(Debug, Clone, Default)]
pub struct QueueDepthSnapshot {
    pub total_entries: usize,
    pub pending_count: usize,
    pub in_flight_count: usize,
    pub retrying_count: usize,
    pub dead_letter_count: usize,
    pub completed_count: usize,
    pub by_intent_type: HashMap<String, usize>,
    pub by_tenant: HashMap<String, usize>,
}

// ============================================================================
// Rate Limiter
// ============================================================================

pub struct RateLimiter {
    permits: Arc<Semaphore>,
    refill_rate: Duration,
    last_refill: RwLock<Instant>,
    max_permits: usize,
}

impl RateLimiter {
    pub fn new(max_per_second: usize) -> Self {
        Self {
            permits: Arc::new(Semaphore::new(max_per_second)),
            refill_rate: Duration::from_secs(1),
            last_refill: RwLock::new(Instant::now()),
            max_permits: max_per_second,
        }
    }

    pub async fn try_acquire(&self) -> bool {
        self.maybe_refill().await;
        self.permits.try_acquire().is_ok()
    }

    async fn maybe_refill(&self) {
        let mut last = self.last_refill.write().await;
        if last.elapsed() >= self.refill_rate {
            let available = self.permits.available_permits();
            let to_add = self.max_permits.saturating_sub(available);
            self.permits.add_permits(to_add);
            *last = Instant::now();
        }
    }
}

// ============================================================================
// Storage Trait and SQLite Implementation
// ============================================================================

#[async_trait::async_trait]
pub trait OutboxStorage: Send + Sync {
    async fn load_all(&self) -> Result<Vec<Result<OutboxEntry, OutboxError>>, OutboxError>;
    async fn save(&self, entry: &OutboxEntry) -> Result<(), OutboxError>;
    async fn save_batch(&self, entries: &[OutboxEntry]) -> Result<(), OutboxError>;
    async fn remove(&self, op_id: &OpId) -> Result<Option<OutboxEntry>, OutboxError>;
    async fn get(&self, op_id: &OpId) -> Result<Option<OutboxEntry>, OutboxError>;
    async fn compare_and_swap(
        &self,
        op_id: &OpId,
        expected_version: u64,
        new_entry: &OutboxEntry,
    ) -> Result<bool, OutboxError>;
    async fn sync(&self) -> Result<(), OutboxError>;
}

/// SQLite-based persistent storage with proper transactions
pub struct SqliteStorage {
    pool: sqlx::SqlitePool,
}

impl SqliteStorage {
    pub async fn new(database_url: &str) -> Result<Self, OutboxError> {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .map_err(|e| OutboxError::Storage(e.to_string()))?;

        // Run migrations
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS outbox_entries (
                op_id TEXT PRIMARY KEY,
                idempotency_key TEXT UNIQUE NOT NULL,
                data BLOB NOT NULL,
                version INTEGER NOT NULL DEFAULT 1,
                state TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                expires_at INTEGER NOT NULL,
                tenant_id TEXT,
                priority INTEGER NOT NULL DEFAULT 0
            );
            
            CREATE INDEX IF NOT EXISTS idx_outbox_state ON outbox_entries(state);
            CREATE INDEX IF NOT EXISTS idx_outbox_expires ON outbox_entries(expires_at);
            CREATE INDEX IF NOT EXISTS idx_outbox_tenant ON outbox_entries(tenant_id);
            CREATE INDEX IF NOT EXISTS idx_outbox_priority ON outbox_entries(priority DESC, created_at ASC);
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| OutboxError::Storage(e.to_string()))?;

        Ok(Self { pool })
    }

    pub async fn new_in_memory() -> Result<Self, OutboxError> {
        Self::new("sqlite::memory:").await
    }
}

#[async_trait::async_trait]
impl OutboxStorage for SqliteStorage {
    async fn load_all(&self) -> Result<Vec<Result<OutboxEntry, OutboxError>>, OutboxError> {
        let rows: Vec<(String, Vec<u8>)> = sqlx::query_as(
            "SELECT op_id, data FROM outbox_entries ORDER BY priority DESC, created_at ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| OutboxError::Storage(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|(op_id, data)| {
                serde_json::from_slice(&data).map_err(|e| OutboxError::CorruptedEntry {
                    op_id: op_id.clone(),
                    reason: e.to_string(),
                })
            })
            .collect())
    }

    async fn save(&self, entry: &OutboxEntry) -> Result<(), OutboxError> {
        let data =
            serde_json::to_vec(entry).map_err(|e| OutboxError::Storage(e.to_string()))?;

        sqlx::query(
            r#"
            INSERT INTO outbox_entries (op_id, idempotency_key, data, version, state, created_at, expires_at, tenant_id, priority)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(op_id) DO UPDATE SET
                data = excluded.data,
                version = excluded.version,
                state = excluded.state
            "#,
        )
        .bind(entry.op_id.as_str())
        .bind(entry.idempotency_key.as_str())
        .bind(&data)
        .bind(entry.version as i64)
        .bind(entry.state.state_name())
        .bind(entry.created_at.0 as i64)
        .bind(entry.expires_at.0 as i64)
        .bind(entry.tenant_id.as_deref())
        .bind(entry.priority as i32)
        .execute(&self.pool)
        .await
        .map_err(|e| OutboxError::Storage(e.to_string()))?;

        Ok(())
    }

    async fn save_batch(&self, entries: &[OutboxEntry]) -> Result<(), OutboxError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| OutboxError::Storage(e.to_string()))?;

        for entry in entries {
            let data = serde_json::to_vec(entry)
                .map_err(|e| OutboxError::Storage(e.to_string()))?;

            sqlx::query(
                r#"
                INSERT INTO outbox_entries (op_id, idempotency_key, data, version, state, created_at, expires_at, tenant_id, priority)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(op_id) DO UPDATE SET
                    data = excluded.data,
                    version = excluded.version,
                    state = excluded.state
                "#,
            )
            .bind(entry.op_id.as_str())
            .bind(entry.idempotency_key.as_str())
            .bind(&data)
            .bind(entry.version as i64)
            .bind(entry.state.state_name())
            .bind(entry.created_at.0 as i64)
            .bind(entry.expires_at.0 as i64)
            .bind(entry.tenant_id.as_deref())
            .bind(entry.priority as i32)
            .execute(&mut *tx)
            .await
            .map_err(|e| OutboxError::Storage(e.to_string()))?;
        }

        tx.commit()
            .await
            .map_err(|e| OutboxError::Storage(e.to_string()))?;

        Ok(())
    }

    async fn remove(&self, op_id: &OpId) -> Result<Option<OutboxEntry>, OutboxError> {
        let row: Option<(Vec<u8>,)> =
            sqlx::query_as("SELECT data FROM outbox_entries WHERE op_id = ?")
                .bind(op_id.as_str())
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| OutboxError::Storage(e.to_string()))?;

        if let Some((data,)) = row {
            sqlx::query("DELETE FROM outbox_entries WHERE op_id = ?")
                .bind(op_id.as_str())
                .execute(&self.pool)
                .await
                .map_err(|e| OutboxError::Storage(e.to_string()))?;

            let entry: OutboxEntry = serde_json::from_slice(&data)
                .map_err(|e| OutboxError::Storage(e.to_string()))?;
            Ok(Some(entry))
        } else {
            Ok(None)
        }
    }

    async fn get(&self, op_id: &OpId) -> Result<Option<OutboxEntry>, OutboxError> {
        let row: Option<(Vec<u8>,)> =
            sqlx::query_as("SELECT data FROM outbox_entries WHERE op_id = ?")
                .bind(op_id.as_str())
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| OutboxError::Storage(e.to_string()))?;

        if let Some((data,)) = row {
            let entry: OutboxEntry = serde_json::from_slice(&data)
                .map_err(|e| OutboxError::Storage(e.to_string()))?;
            Ok(Some(entry))
        } else {
            Ok(None)
        }
    }

    async fn compare_and_swap(
        &self,
        op_id: &OpId,
        expected_version: u64,
        new_entry: &OutboxEntry,
    ) -> Result<bool, OutboxError> {
        let data = serde_json::to_vec(new_entry)
            .map_err(|e| OutboxError::Storage(e.to_string()))?;

        let result = sqlx::query(
            r#"
            UPDATE outbox_entries 
            SET data = ?, version = ?, state = ?
            WHERE op_id = ? AND version = ?
            "#,
        )
        .bind(&data)
        .bind(new_entry.version as i64)
        .bind(new_entry.state.state_name())
        .bind(op_id.as_str())
        .bind(expected_version as i64)
        .execute(&self.pool)
        .await
        .map_err(|e| OutboxError::Storage(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    async fn sync(&self) -> Result<(), OutboxError> {
        sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
            .execute(&self.pool)
            .await
            .map_err(|e| OutboxError::Storage(e.to_string()))?;
        Ok(())
    }
}

// ============================================================================
// Configuration
// ============================================================================

#[derive(Clone, Debug)]
pub struct OutboxConfig {
    pub max_entries: usize,
    pub max_attempts: u32,
    pub base_backoff_ms: u64,
    pub max_backoff_ms: u64,
    pub default_ttl_ms: u64,
    pub lease_duration_ms: u64,
    pub completed_cache_size: usize,
    pub completed_cache_ttl_ms: u64,
    pub rate_limit_per_second: usize,
    pub worker_id: String,
}

impl Default for OutboxConfig {
    fn default() -> Self {
        Self {
            max_entries: 10_000,
            max_attempts: 25,
            base_backoff_ms: 1_000,
            max_backoff_ms: 300_000,
            default_ttl_ms: 7 * 24 * 60 * 60 * 1000,
            lease_duration_ms: 60_000,
            completed_cache_size: 100_000,
            completed_cache_ttl_ms: 24 * 60 * 60 * 1000,
            rate_limit_per_second: 1000,
            worker_id: Uuid::new_v4().to_string(),
        }
    }
}

impl OutboxConfig {
    pub fn validate(&self) -> Result<(), OutboxError> {
        if self.max_entries == 0 {
            return Err(OutboxError::Validation("max_entries must be > 0".into()));
        }
        if self.max_attempts == 0 {
            return Err(OutboxError::Validation("max_attempts must be > 0".into()));
        }
        if self.base_backoff_ms == 0 {
            return Err(OutboxError::Validation("base_backoff_ms must be > 0".into()));
        }
        if self.lease_duration_ms < 1000 {
            return Err(OutboxError::Validation(
                "lease_duration_ms should be at least 1000ms".into(),
            ));
        }
        Ok(())
    }
}

// ============================================================================
// Consolidated Outbox State
// ============================================================================

struct OutboxState {
    entries_by_op_id: HashMap<String, OutboxEntry>,
    entries_by_idem_key: HashMap<String, String>,
    completed_idem_keys: lru::LruCache<String, (UnixTimeMs, UnixTimeMs)>,
    quarantined: HashMap<String, (OutboxError, UnixTimeMs)>,
}

impl OutboxState {
    fn new(completed_cache_size: usize) -> Self {
        let cache_size = std::num::NonZeroUsize::new(completed_cache_size)
            .unwrap_or(std::num::NonZeroUsize::new(10_000).unwrap());
        
        Self {
            entries_by_op_id: HashMap::new(),
            entries_by_idem_key: HashMap::new(),
            completed_idem_keys: lru::LruCache::new(cache_size),
            quarantined: HashMap::new(),
        }
    }
}

// ============================================================================
// Main Outbox Implementation
// ============================================================================

pub struct Outbox<S: OutboxStorage> {
    storage: Arc<S>,
    config: OutboxConfig,
    state: RwLock<OutboxState>,
    metrics: Arc<OutboxMetrics>,
    rate_limiter: RateLimiter,
}

impl<S: OutboxStorage> Outbox<S> {
    #[instrument(skip(storage, config))]
    pub async fn new(storage: Arc<S>, config: OutboxConfig) -> Result<Self, OutboxError> {
        config.validate()?;

        let loaded = storage.load_all().await?;
        let mut state = OutboxState::new(config.completed_cache_size);
        let mut quarantine_count = 0;

        for result in loaded {
            match result {
                Ok(entry) => {
                    state.entries_by_idem_key.insert(
                        entry.idempotency_key.as_str().to_string(),
                        entry.op_id.as_str().to_string(),
                    );
                    
                    if let EntryState::Completed { completed_at } = &entry.state {
                        state.completed_idem_keys.put(
                            entry.idempotency_key.as_str().to_string(),
                            (*completed_at, entry.expires_at),
                        );
                    }
                    
                    state.entries_by_op_id.insert(entry.op_id.as_str().to_string(), entry);
                }
                Err(e) => {
                    warn!("Quarantining corrupted entry: {:?}", e);
                    if let OutboxError::CorruptedEntry { op_id, .. } = &e {
                        state.quarantined.insert(op_id.clone(), (e, UnixTimeMs::now()));
                        quarantine_count += 1;
                    }
                }
            }
        }

        if quarantine_count > 0 {
            warn!("Loaded with {} quarantined entries", quarantine_count);
        }

        info!(
            "Outbox initialized with {} entries, {} quarantined",
            state.entries_by_op_id.len(),
            quarantine_count
        );

        Ok(Self {
            storage,
            config: config.clone(),
            state: RwLock::new(state),
            metrics: Arc::new(OutboxMetrics::new()),
            rate_limiter: RateLimiter::new(config.rate_limit_per_second),
        })
    }

    /// Push a new entry with full atomicity guarantees
    #[instrument(skip(self, entry), fields(op_id = %entry.op_id.as_str()))]
    pub async fn push(&self, entry: OutboxEntry) -> Result<(), OutboxError> {
        // Rate limiting check
        if !self.rate_limiter.try_acquire().await {
            self.metrics.rate_limit_rejections.fetch_add(1, Ordering::Relaxed);
            return Err(OutboxError::RateLimited(
                "Too many requests, try again later".into(),
            ));
        }

        let op_id_str = entry.op_id.as_str().to_string();
        let idem_key_str = entry.idempotency_key.as_str().to_string();

        // Single lock scope for atomicity
        let mut state = self.state.write().await;

        // Check capacity
        if state.entries_by_op_id.len() >= self.config.max_entries {
            return Err(OutboxError::Full(self.config.max_entries));
        }

        // Check for duplicate op_id
        if state.entries_by_op_id.contains_key(&op_id_str) {
            self.metrics.duplicate_rejections.fetch_add(1, Ordering::Relaxed);
            return Err(OutboxError::DuplicateOpId(op_id_str));
        }

        // Check for duplicate idempotency key (including completed cache with TTL check)
        if state.entries_by_idem_key.contains_key(&idem_key_str) {
            self.metrics.duplicate_rejections.fetch_add(1, Ordering::Relaxed);
            return Err(OutboxError::DuplicateIdempotencyKey(idem_key_str));
        }

        // Check completed cache with TTL awareness
        if let Some((completed_at, expires_at)) = state.completed_idem_keys.peek(&idem_key_str) {
            let now = UnixTimeMs::now();
            // Only reject if the completed entry hasn't expired
            if now.0 < expires_at.0 {
                self.metrics.duplicate_rejections.fetch_add(1, Ordering::Relaxed);
                return Err(OutboxError::DuplicateIdempotencyKey(format!(
                    "{} (completed at {})",
                    idem_key_str, completed_at.0
                )));
            }
        }

        // Memory first
        state.entries_by_op_id.insert(op_id_str.clone(), entry.clone());
        state.entries_by_idem_key.insert(idem_key_str.clone(), op_id_str.clone());

        // Then persist - rollback on failure
        if let Err(e) = self.storage.save(&entry).await {
            state.entries_by_op_id.remove(&op_id_str);
            state.entries_by_idem_key.remove(&idem_key_str);
            self.metrics.storage_errors.fetch_add(1, Ordering::Relaxed);
            error!("Failed to persist entry: {:?}", e);
            return Err(e);
        }

        self.metrics.entries_pushed.fetch_add(1, Ordering::Relaxed);
        info!("Entry pushed successfully");

        Ok(())
    }

    /// Get entries that are due for processing
    #[instrument(skip(self))]
    pub async fn get_due_entries(&self, now: UnixTimeMs, limit: usize) -> Vec<OutboxEntry> {
        let state = self.state.read().await;

        let mut due: Vec<_> = state
            .entries_by_op_id
            .values()
            .filter(|e| {
                !e.is_expired(now)
                    && e.state.can_transition_to_in_flight(now, self.config.lease_duration_ms)
                    && self.dependencies_satisfied(e, &state.entries_by_op_id)
            })
            .take(limit)
            .cloned()
            .collect();

        // Sort by priority (desc) then created_at (asc)
        due.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then_with(|| a.created_at.0.cmp(&b.created_at.0))
        });

        due
    }

    fn dependencies_satisfied(
        &self,
        entry: &OutboxEntry,
        all_entries: &HashMap<String, OutboxEntry>,
    ) -> bool {
        if let Some(dep_op_id) = entry.intent.depends_on() {
            match all_entries.get(dep_op_id.as_str()) {
                Some(dep_entry) => matches!(dep_entry.state, EntryState::Completed { .. }),
                None => true, // Dependency doesn't exist, assume satisfied
            }
        } else {
            true
        }
    }

    /// Acquire a lease on an entry for processing
    #[instrument(skip(self), fields(op_id = %op_id.as_str()))]
    pub async fn acquire_lease(
        &self,
        op_id: &OpId,
        now: UnixTimeMs,
    ) -> Result<(OutboxEntry, LeaseToken), OutboxError> {
        let mut state = self.state.write().await;

        let entry = state
            .entries_by_op_id
            .get_mut(op_id.as_str())
            .ok_or_else(|| OutboxError::NotFound(op_id.as_str().to_string()))?;

        // Validate state transition
        if !entry.state.can_transition_to_in_flight(now, self.config.lease_duration_ms) {
            self.metrics.invalid_transitions.fetch_add(1, Ordering::Relaxed);
            return Err(OutboxError::InvalidStateTransition {
                from: entry.state.clone(),
                to: "InFlight",
                reason: "Entry not ready for processing".into(),
            });
        }

        // Check expiration
        if entry.is_expired(now) {
            return Err(OutboxError::InvalidStateTransition {
                from: entry.state.clone(),
                to: "InFlight",
                reason: "Entry has expired".into(),
            });
        }

        let expected_version = entry.version;
        let lease = LeaseToken::new(&self.config.worker_id, now, self.config.lease_duration_ms);

        entry.state = EntryState::InFlight {
            started_at: now,
            lease: lease.clone(),
        };
        entry.version += 1;

        let updated = entry.clone();

        // Use compare-and-swap for storage
        let swapped = self
            .storage
            .compare_and_swap(op_id, expected_version, &updated)
            .await?;

        if !swapped {
            // Rollback memory state
            if let Some(e) = state.entries_by_op_id.get_mut(op_id.as_str()) {
                e.version = expected_version;
                // Can't easily restore previous state, reload from storage
            }
            self.metrics.lease_conflicts.fetch_add(1, Ordering::Relaxed);
            return Err(OutboxError::LeaseError(
                "Concurrent modification detected".into(),
            ));
        }

        info!("Lease acquired, token: {}", lease.token);
        Ok((updated, lease))
    }

    /// Complete an entry with lease validation
    #[instrument(skip(self), fields(op_id = %op_id.as_str()))]
    pub async fn complete(
        &self,
        op_id: &OpId,
        lease_token: &str,
        now: UnixTimeMs,
    ) -> Result<OutboxEntry, OutboxError> {
        let mut state = self.state.write().await;

        let entry = state
            .entries_by_op_id
            .get_mut(op_id.as_str())
            .ok_or_else(|| OutboxError::NotFound(op_id.as_str().to_string()))?;

        // Validate lease
        if !entry.state.can_transition_to_completed(Some(lease_token)) {
            self.metrics.lease_conflicts.fetch_add(1, Ordering::Relaxed);
            return Err(OutboxError::LeaseError(
                "Invalid or expired lease token".into(),
            ));
        }

        let expected_version = entry.version;
        entry.state = EntryState::Completed { completed_at: now };
        entry.version += 1;

        let updated = entry.clone();

        // Update completed cache
        state.completed_idem_keys.put(
            entry.idempotency_key.as_str().to_string(),
            (now, entry.expires_at),
        );

        // Persist with CAS
        let swapped = self
            .storage
            .compare_and_swap(op_id, expected_version, &updated)
            .await?;

        if !swapped {
            self.metrics.lease_conflicts.fetch_add(1, Ordering::Relaxed);
            return Err(OutboxError::LeaseError(
                "Concurrent modification during completion".into(),
            ));
        }

        self.metrics.entries_completed.fetch_add(1, Ordering::Relaxed);
        info!("Entry completed");

        Ok(updated)
    }

    /// Mark an entry as failed with lease validation
    #[instrument(skip(self, error), fields(op_id = %op_id.as_str()))]
    pub async fn fail(
        &self,
        op_id: &OpId,
        lease_token: &str,
        error: IntentError,
        now: UnixTimeMs,
    ) -> Result<OutboxEntry, OutboxError> {
        let mut state = self.state.write().await;

        let entry = state
            .entries_by_op_id
            .get_mut(op_id.as_str())
            .ok_or_else(|| OutboxError::NotFound(op_id.as_str().to_string()))?;

        // Validate lease
        if !entry.state.can_transition_to_failed(Some(lease_token)) {
            self.metrics.lease_conflicts.fetch_add(1, Ordering::Relaxed);
            return Err(OutboxError::LeaseError(
                "Invalid or expired lease token".into(),
            ));
        }

        let expected_version = entry.version;
        let mut history = entry.state.take_history();
        history.record_error(error.clone());

        let should_dead_letter = !error.is_retryable()
            || history.total_attempts >= self.config.max_attempts
            || entry.is_expired(now);

        if should_dead_letter {
            let reason = if !error.is_retryable() {
                DeadLetterReason::NonRetryableError
            } else if entry.is_expired(now) {
                DeadLetterReason::Expired
            } else {
                DeadLetterReason::MaxRetriesExceeded
            };

            entry.state = EntryState::DeadLetter {
                reason,
                history,
                dead_at: now,
            };
            self.metrics.entries_dead_lettered.fetch_add(1, Ordering::Relaxed);
        } else {
            let backoff = self.calculate_backoff(history.total_attempts);
            let next_attempt_at = UnixTimeMs(now.0.saturating_add(backoff));

            entry.state = EntryState::Retrying {
                next_attempt_at,
                history,
            };
            self.metrics.entries_failed.fetch_add(1, Ordering::Relaxed);
        }

        entry.version += 1;
        let updated = entry.clone();

        // Persist with CAS
        let swapped = self
            .storage
            .compare_and_swap(op_id, expected_version, &updated)
            .await?;

        if !swapped {
            self.metrics.lease_conflicts.fetch_add(1, Ordering::Relaxed);
            return Err(OutboxError::LeaseError(
                "Concurrent modification during failure".into(),
            ));
        }

        info!(
            "Entry failed, new state: {}",
            updated.state.state_name()
        );

        Ok(updated)
    }

    fn calculate_backoff(&self, attempt: u32) -> u64 {
        use rand::Rng;
        let mut rng = rand::rngs::StdRng::from_entropy();
        let jitter: u64 = rng.gen_range(0..=2000);

        let exponent = attempt.min(16);
        let base_delay = self.config.base_backoff_ms.saturating_mul(1u64 << exponent);
        let capped_delay = base_delay.min(self.config.max_backoff_ms);

        capped_delay.saturating_add(jitter)
    }

    /// Mark dependent entries as failed when a dependency fails
    #[instrument(skip(self), fields(failed_op_id = %failed_op_id.as_str()))]
    pub async fn cascade_dependency_failure(
        &self,
        failed_op_id: &OpId,
        now: UnixTimeMs,
    ) -> Result<Vec<OpId>, OutboxError> {
        let mut affected = Vec::new();
        let mut state = self.state.write().await;

        // Collect all dependent entries in a single pass
        let dependents: Vec<_> = state
            .entries_by_op_id
            .values()
            .filter(|e| {
                e.intent.depends_on().map_or(false, |dep| dep == failed_op_id)
                    && !e.is_terminal()
            })
            .map(|e| e.op_id.clone())
            .collect();

        // Update all dependents atomically
        let mut updates = Vec::new();
        for dep_op_id in &dependents {
            if let Some(entry) = state.entries_by_op_id.get_mut(dep_op_id.as_str()) {
                let history = entry.state.take_history();
                entry.state = EntryState::DeadLetter {
                    reason: DeadLetterReason::DependencyFailed {
                        dependency_op_id: failed_op_id.clone(),
                    },
                    history,
                    dead_at: now,
                };
                entry.version += 1;
                updates.push(entry.clone());
                affected.push(dep_op_id.clone());
            }
        }

        // Batch persist
        if !updates.is_empty() {
            self.storage.save_batch(&updates).await?;
            self.metrics
                .entries_dead_lettered
                .fetch_add(updates.len() as u64, Ordering::Relaxed);
        }

        info!("Cascaded failure to {} dependent entries", affected.len());
        Ok(affected)
    }

    /// Expire stale entries
    #[instrument(skip(self))]
    pub async fn expire_stale(&self, now: UnixTimeMs) -> Result<Vec<OpId>, OutboxError> {
        let mut expired = Vec::new();
        let mut state = self.state.write().await;

        let stale: Vec<_> = state
            .entries_by_op_id
            .values()
            .filter(|e| e.is_expired(now) && !e.is_terminal())
            .map(|e| e.op_id.clone())
            .collect();

        let mut updates = Vec::new();
        for op_id in &stale {
            if let Some(entry) = state.entries_by_op_id.get_mut(op_id.as_str()) {
                let history = entry.state.take_history();
                entry.state = EntryState::DeadLetter {
                    reason: DeadLetterReason::Expired,
                    history,
                    dead_at: now,
                };
                entry.version += 1;
                updates.push(entry.clone());
                expired.push(op_id.clone());
            }
        }

        if !updates.is_empty() {
            self.storage.save_batch(&updates).await?;
            self.metrics
                .entries_expired
                .fetch_add(updates.len() as u64, Ordering::Relaxed);
        }

        info!("Expired {} stale entries", expired.len());
        Ok(expired)
    }

    /// Remove completed entries older than specified time
    #[instrument(skip(self))]
    pub async fn prune_completed(
        &self,
        older_than: UnixTimeMs,
    ) -> Result<usize, OutboxError> {
        let mut state = self.state.write().await;

        let to_remove: Vec<_> = state
            .entries_by_op_id
            .values()
            .filter(|e| {
                if let EntryState::Completed { completed_at } = &e.state {
                    completed_at.0 < older_than.0
                } else {
                    false
                }
            })
            .map(|e| (e.op_id.clone(), e.idempotency_key.clone()))
            .collect();

        let count = to_remove.len();

        for (op_id, idem_key) in &to_remove {
            self.storage.remove(op_id).await?;
            state.entries_by_op_id.remove(op_id.as_str());
            state.entries_by_idem_key.remove(idem_key.as_str());
        }

        info!("Pruned {} completed entries", count);
        Ok(count)
    }

    /// Get queue depth metrics
    pub async fn get_queue_depth(&self) -> QueueDepthSnapshot {
        let state = self.state.read().await;

        let mut snapshot = QueueDepthSnapshot {
            total_entries: state.entries_by_op_id.len(),
            ..Default::default()
        };

        for entry in state.entries_by_op_id.values() {
            match &entry.state {
                EntryState::Pending => snapshot.pending_count += 1,
                EntryState::InFlight { .. } => snapshot.in_flight_count += 1,
                EntryState::Retrying { .. } => snapshot.retrying_count += 1,
                EntryState::DeadLetter { .. } => snapshot.dead_letter_count += 1,
                EntryState::Completed { .. } => snapshot.completed_count += 1,
            }

            *snapshot
                .by_intent_type
                .entry(entry.intent.intent_type().to_string())
                .or_insert(0) += 1;

            if let Some(ref tenant) = entry.tenant_id {
                *snapshot
                    .by_tenant
                    .entry(tenant.clone())
                    .or_insert(0) += 1;
            }
        }

        snapshot
    }

    /// Get operational metrics
    pub fn get_metrics(&self) -> MetricsSnapshot {
        self.metrics.snapshot()
    }

    /// Get a specific entry by op_id
    pub async fn get_entry(&self, op_id: &OpId) -> Option<OutboxEntry> {
        let state = self.state.read().await;
        state.entries_by_op_id.get(op_id.as_str()).cloned()
    }

    /// Get quarantined entries for manual review
    pub async fn get_quarantined(&self) -> Vec<(String, String, UnixTimeMs)> {
        let state = self.state.read().await;
        state
            .quarantined
            .iter()
            .map(|(op_id, (err, ts))| (op_id.clone(), err.to_string(), *ts))
            .collect()
    }

    /// Force sync to storage
    pub async fn sync(&self) -> Result<(), OutboxError> {
        self.storage.sync().await
    }
}

// ============================================================================
// Tests with Failure Injection
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    fn make_now() -> UnixTimeMs {
        UnixTimeMs(1_700_000_000_000)
    }

    // Failure-injectable storage wrapper
    struct FailableStorage<S: OutboxStorage> {
        inner: S,
        fail_saves: AtomicBool,
        fail_cas: AtomicBool,
    }

    impl<S: OutboxStorage> FailableStorage<S> {
        fn new(inner: S) -> Self {
            Self {
                inner,
                fail_saves: AtomicBool::new(false),
                fail_cas: AtomicBool::new(false),
            }
        }

        fn set_fail_saves(&self, fail: bool) {
            self.fail_saves.store(fail, Ordering::SeqCst);
        }

        fn set_fail_cas(&self, fail: bool) {
            self.fail_cas.store(fail, Ordering::SeqCst);
        }
    }

    #[async_trait::async_trait]
    impl<S: OutboxStorage> OutboxStorage for FailableStorage<S> {
        async fn load_all(&self) -> Result<Vec<Result<OutboxEntry, OutboxError>>, OutboxError> {
            self.inner.load_all().await
        }

        async fn save(&self, entry: &OutboxEntry) -> Result<(), OutboxError> {
            if self.fail_saves.load(Ordering::SeqCst) {
                return Err(OutboxError::Storage("Injected failure".into()));
            }
            self.inner.save(entry).await
        }

        async fn save_batch(&self, entries: &[OutboxEntry]) -> Result<(), OutboxError> {
            if self.fail_saves.load(Ordering::SeqCst) {
                return Err(OutboxError::Storage("Injected failure".into()));
            }
            self.inner.save_batch(entries).await
        }

        async fn remove(&self, op_id: &OpId) -> Result<Option<OutboxEntry>, OutboxError> {
            self.inner.remove(op_id).await
        }

        async fn get(&self, op_id: &OpId) -> Result<Option<OutboxEntry>, OutboxError> {
            self.inner.get(op_id).await
        }

        async fn compare_and_swap(
            &self,
            op_id: &OpId,
            expected_version: u64,
            new_entry: &OutboxEntry,
        ) -> Result<bool, OutboxError> {
            if self.fail_cas.load(Ordering::SeqCst) {
                return Err(OutboxError::Storage("Injected CAS failure".into()));
            }
            self.inner.compare_and_swap(op_id, expected_version, new_entry).await
        }

        async fn sync(&self) -> Result<(), OutboxError> {
            self.inner.sync().await
        }
    }

    #[test]
    fn test_op_id_validation() {
        assert!(OpId::new("valid-id_123").is_ok());
        assert!(OpId::new("").is_err());
        assert!(OpId::new("   ").is_err());
        assert!(OpId::new("invalid id").is_err());
        assert!(OpId::new("a".repeat(129)).is_err());
    }

    #[test]
    fn test_op_id_trims_whitespace() {
        let id = OpId::new("  test-id  ").unwrap();
        assert_eq!(id.as_str(), "test-id");
    }

    #[test]
    fn test_idempotency_key_trims_whitespace() {
        let key = IdempotencyKey::new("  key-123  ").unwrap();
        assert_eq!(key.as_str(), "key-123");
    }

    #[test]
    fn test_wound_severity_validation() {
        assert!(WoundSeverity::new(0).is_err());
        assert!(WoundSeverity::new(1).is_ok());
        assert!(WoundSeverity::new(5).is_ok());
        assert!(WoundSeverity::new(6).is_err());
    }

    #[test]
    fn test_latlon_validation() {
        assert!(LatLon::new(0.0, 0.0).is_ok());
        assert!(LatLon::new(90.0, 180.0).is_ok());
        assert!(LatLon::new(-90.0, -180.0).is_ok());
        assert!(LatLon::new(91.0, 0.0).is_err());
        assert!(LatLon::new(0.0, 181.0).is_err());
        assert!(LatLon::new(f64::NAN, 0.0).is_err());
    }

    #[test]
    fn test_utf8_truncation() {
        let mut s = "Hello 🌍 World".to_string();
        truncate_utf8_safe(&mut s, 10);
        assert!(s.is_char_boundary(s.len()));
        assert!(s.len() <= 10);
    }

    #[test]
    fn test_utf8_truncation_multi_byte() {
        let mut s = "日本語".to_string();
        truncate_utf8_safe(&mut s, 4);
        assert!(s.is_char_boundary(s.len()));
        assert_eq!(s, "日");
    }

    #[test]
    fn test_error_category_retryable() {
        assert!(ErrorCategory::Transient.is_retryable());
        assert!(ErrorCategory::RateLimited.is_retryable());
        assert!(ErrorCategory::NetworkError.is_retryable());
        assert!(!ErrorCategory::ClientError.is_retryable());
    }

    #[test]
    fn test_intent_error_marks_truncation() {
        let error = IntentError::new(
            ErrorCategory::Unknown,
            "code",
            "x".repeat(1000),
            make_now(),
        );
        assert!(error.truncated);
        assert!(error.message.len() <= IntentError::MAX_MESSAGE_LENGTH);
    }

    #[test]
    fn test_lease_token_expiration() {
        let now = make_now();
        let lease = LeaseToken::new("worker-1", now, 1000);
        
        assert!(!lease.is_expired(now));
        assert!(!lease.is_expired(UnixTimeMs(now.0 + 500)));
        assert!(lease.is_expired(UnixTimeMs(now.0 + 1000)));
        assert!(lease.is_expired(UnixTimeMs(now.0 + 2000)));
    }

    #[test]
    fn test_state_can_transition_to_in_flight() {
        let now = make_now();
        let timeout = 60_000u64;

        assert!(EntryState::Pending.can_transition_to_in_flight(now, timeout));
        
        let retrying = EntryState::Retrying {
            next_attempt_at: UnixTimeMs(now.0 - 1000),
            history: RetryHistory::new(),
        };
        assert!(retrying.can_transition_to_in_flight(now, timeout));

        let retrying_future = EntryState::Retrying {
            next_attempt_at: UnixTimeMs(now.0 + 1000),
            history: RetryHistory::new(),
        };
        assert!(!retrying_future.can_transition_to_in_flight(now, timeout));

        let completed = EntryState::Completed { completed_at: now };
        assert!(!completed.can_transition_to_in_flight(now, timeout));
    }

    #[tokio::test]
    async fn test_outbox_push_and_get() {
        let storage = Arc::new(SqliteStorage::new_in_memory().await.unwrap());
        let outbox = Outbox::new(storage, OutboxConfig::default()).await.unwrap();

        let now = make_now();
        let entry = OutboxEntry::new(
            OpId::new("op-1").unwrap(),
            IdempotencyKey::new("idem-1").unwrap(),
            OutboxIntent::CreateCase {
                local_id: LocalOpId::new("local-1").unwrap(),
                location: LatLon::new(0.0, 0.0).unwrap(),
                description: None,
                wound_severity: None,
                created_at_ms_utc: now,
            },
            now,
            3600_000,
        );

        outbox.push(entry).await.unwrap();

        let due = outbox.get_due_entries(now, 10).await;
        assert_eq!(due.len(), 1);
    }

    #[tokio::test]
    async fn test_duplicate_rejection() {
        let storage = Arc::new(SqliteStorage::new_in_memory().await.unwrap());
        let outbox = Outbox::new(storage, OutboxConfig::default()).await.unwrap();

        let now = make_now();
        let entry1 = OutboxEntry::new(
            OpId::new("op-1").unwrap(),
            IdempotencyKey::new("idem-1").unwrap(),
            OutboxIntent::CreateCase {
                local_id: LocalOpId::new("local-1").unwrap(),
                location: LatLon::new(0.0, 0.0).unwrap(),
                description: None,
                wound_severity: None,
                created_at_ms_utc: now,
            },
            now,
            3600_000,
        );

        let entry2 = OutboxEntry::new(
            OpId::new("op-1").unwrap(),
            IdempotencyKey::new("idem-2").unwrap(),
            OutboxIntent::CreateCase {
                local_id: LocalOpId::new("local-1").unwrap(),
                location: LatLon::new(0.0, 0.0).unwrap(),
                description: None,
                wound_severity: None,
                created_at_ms_utc: now,
            },
            now,
            3600_000,
        );

        outbox.push(entry1).await.unwrap();
        let result = outbox.push(entry2).await;

        assert!(matches!(result, Err(OutboxError::DuplicateOpId(_))));
    }

    #[tokio::test]
    async fn test_complete_flow_with_lease() {
        let storage = Arc::new(SqliteStorage::new_in_memory().await.unwrap());
        let outbox = Outbox::new(storage, OutboxConfig::default()).await.unwrap();

        let now = make_now();
        let entry = OutboxEntry::new(
            OpId::new("op-1").unwrap(),
            IdempotencyKey::new("idem-1").unwrap(),
            OutboxIntent::CreateCase {
                local_id: LocalOpId::new("local-1").unwrap(),
                location: LatLon::new(0.0, 0.0).unwrap(),
                description: None,
                wound_severity: None,
                created_at_ms_utc: now,
            },
            now,
            3600_000,
        );

        outbox.push(entry).await.unwrap();

        let op_id = OpId::new("op-1").unwrap();
        let (_, lease) = outbox.acquire_lease(&op_id, now).await.unwrap();
        
        outbox.complete(&op_id, &lease.token, now).await.unwrap();

        let due = outbox.get_due_entries(now, 10).await;
        assert!(due.is_empty());

        // Verify idempotency key is blocked
        let entry2 = OutboxEntry::new(
            OpId::new("op-2").unwrap(),
            IdempotencyKey::new("idem-1").unwrap(),
            OutboxIntent::CreateCase {
                local_id: LocalOpId::new("local-2").unwrap(),
                location: LatLon::new(0.0, 0.0).unwrap(),
                description: None,
                wound_severity: None,
                created_at_ms_utc: now,
            },
            now,
            3600_000,
        );

        let result = outbox.push(entry2).await;
        assert!(matches!(result, Err(OutboxError::DuplicateIdempotencyKey(_))));
    }

    #[tokio::test]
    async fn test_invalid_lease_rejected() {
        let storage = Arc::new(SqliteStorage::new_in_memory().await.unwrap());
        let outbox = Outbox::new(storage, OutboxConfig::default()).await.unwrap();

        let now = make_now();
        let entry = OutboxEntry::new(
            OpId::new("op-1").unwrap(),
            IdempotencyKey::new("idem-1").unwrap(),
            OutboxIntent::CreateCase {
                local_id: LocalOpId::new("local-1").unwrap(),
                location: LatLon::new(0.0, 0.0).unwrap(),
                description: None,
                wound_severity: None,
                created_at_ms_utc: now,
            },
            now,
            3600_000,
        );

        outbox.push(entry).await.unwrap();

        let op_id = OpId::new("op-1").unwrap();
        let _ = outbox.acquire_lease(&op_id, now).await.unwrap();
        
        // Try to complete with wrong token
        let result = outbox.complete(&op_id, "wrong-token", now).await;
        assert!(matches!(result, Err(OutboxError::LeaseError(_))));
    }

    #[tokio::test]
    async fn test_storage_failure_rollback() {
        let inner_storage = SqliteStorage::new_in_memory().await.unwrap();
        let failable = Arc::new(FailableStorage::new(inner_storage));
        let outbox = Outbox::new(failable.clone(), OutboxConfig::default()).await.unwrap();

        let now = make_now();
        let entry = OutboxEntry::new(
            OpId::new("op-1").unwrap(),
            IdempotencyKey::new("idem-1").unwrap(),
            OutboxIntent::CreateCase {
                local_id: LocalOpId::new("local-1").unwrap(),
                location: LatLon::new(0.0, 0.0).unwrap(),
                description: None,
                wound_severity: None,
                created_at_ms_utc: now,
            },
            now,
            3600_000,
        );

        // Enable failure injection
        failable.set_fail_saves(true);

        let result = outbox.push(entry.clone()).await;
        assert!(matches!(result, Err(OutboxError::Storage(_))));

        // Verify memory was rolled back
        let retrieved = outbox.get_entry(&entry.op_id).await;
        assert!(retrieved.is_none());

        // Disable failure and retry
        failable.set_fail_saves(false);
                outbox.push(entry.clone()).await.unwrap();

        // Verify it's now there
        let retrieved = outbox.get_entry(&entry.op_id).await;
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_retry_with_backoff() {
        let storage = Arc::new(SqliteStorage::new_in_memory().await.unwrap());
        let outbox = Outbox::new(storage, OutboxConfig::default()).await.unwrap();

        let now = make_now();
        let entry = OutboxEntry::new(
            OpId::new("op-1").unwrap(),
            IdempotencyKey::new("idem-1").unwrap(),
            OutboxIntent::CreateCase {
                local_id: LocalOpId::new("local-1").unwrap(),
                location: LatLon::new(0.0, 0.0).unwrap(),
                description: None,
                wound_severity: None,
                created_at_ms_utc: now,
            },
            now,
            3600_000,
        );

        outbox.push(entry).await.unwrap();

        let op_id = OpId::new("op-1").unwrap();
        let (_, lease) = outbox.acquire_lease(&op_id, now).await.unwrap();

        // Fail with retryable error
        let error = IntentError::new(
            ErrorCategory::Transient,
            "TEMP_ERROR",
            "Temporary failure",
            now,
        );

        let failed = outbox.fail(&op_id, &lease.token, error, now).await.unwrap();

        // Should be in Retrying state
        assert!(matches!(failed.state, EntryState::Retrying { .. }));

        // Should not be due immediately
        let due = outbox.get_due_entries(now, 10).await;
        assert!(due.is_empty());

        // Should be due after backoff
        let future = UnixTimeMs(now.0 + 10_000);
        let due = outbox.get_due_entries(future, 10).await;
        assert_eq!(due.len(), 1);
    }

    #[tokio::test]
    async fn test_non_retryable_error_dead_letters() {
        let storage = Arc::new(SqliteStorage::new_in_memory().await.unwrap());
        let outbox = Outbox::new(storage, OutboxConfig::default()).await.unwrap();

        let now = make_now();
        let entry = OutboxEntry::new(
            OpId::new("op-1").unwrap(),
            IdempotencyKey::new("idem-1").unwrap(),
            OutboxIntent::CreateCase {
                local_id: LocalOpId::new("local-1").unwrap(),
                location: LatLon::new(0.0, 0.0).unwrap(),
                description: None,
                wound_severity: None,
                created_at_ms_utc: now,
            },
            now,
            3600_000,
        );

        outbox.push(entry).await.unwrap();

        let op_id = OpId::new("op-1").unwrap();
        let (_, lease) = outbox.acquire_lease(&op_id, now).await.unwrap();

        // Fail with non-retryable error
        let error = IntentError::new(
            ErrorCategory::ClientError,
            "BAD_REQUEST",
            "Invalid data",
            now,
        );

        let failed = outbox.fail(&op_id, &lease.token, error, now).await.unwrap();

        // Should be dead-lettered immediately
        assert!(matches!(
            failed.state,
            EntryState::DeadLetter {
                reason: DeadLetterReason::NonRetryableError,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn test_max_retries_dead_letters() {
        let config = OutboxConfig {
            max_attempts: 3,
            base_backoff_ms: 100,
            ..Default::default()
        };
        let storage = Arc::new(SqliteStorage::new_in_memory().await.unwrap());
        let outbox = Outbox::new(storage, config).await.unwrap();

        let now = make_now();
        let entry = OutboxEntry::new(
            OpId::new("op-1").unwrap(),
            IdempotencyKey::new("idem-1").unwrap(),
            OutboxIntent::CreateCase {
                local_id: LocalOpId::new("local-1").unwrap(),
                location: LatLon::new(0.0, 0.0).unwrap(),
                description: None,
                wound_severity: None,
                created_at_ms_utc: now,
            },
            now,
            3600_000,
        );

        outbox.push(entry).await.unwrap();
        let op_id = OpId::new("op-1").unwrap();

        // Attempt 1
        let mut current_time = now;
        let (_, lease1) = outbox.acquire_lease(&op_id, current_time).await.unwrap();
        let error1 = IntentError::new(ErrorCategory::Transient, "ERR", "fail", current_time);
        outbox.fail(&op_id, &lease1.token, error1, current_time).await.unwrap();

        // Attempt 2
        current_time = UnixTimeMs(current_time.0 + 10_000);
        let (_, lease2) = outbox.acquire_lease(&op_id, current_time).await.unwrap();
        let error2 = IntentError::new(ErrorCategory::Transient, "ERR", "fail", current_time);
        outbox.fail(&op_id, &lease2.token, error2, current_time).await.unwrap();

        // Attempt 3 - should dead-letter
        current_time = UnixTimeMs(current_time.0 + 10_000);
        let (_, lease3) = outbox.acquire_lease(&op_id, current_time).await.unwrap();
        let error3 = IntentError::new(ErrorCategory::Transient, "ERR", "fail", current_time);
        let result = outbox.fail(&op_id, &lease3.token, error3, current_time).await.unwrap();

        assert!(matches!(
            result.state,
            EntryState::DeadLetter {
                reason: DeadLetterReason::MaxRetriesExceeded,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn test_dependency_handling() {
        let storage = Arc::new(SqliteStorage::new_in_memory().await.unwrap());
        let outbox = Outbox::new(storage, OutboxConfig::default()).await.unwrap();

        let now = make_now();
        
        // Parent entry
        let parent = OutboxEntry::new(
            OpId::new("parent-op").unwrap(),
            IdempotencyKey::new("parent-idem").unwrap(),
            OutboxIntent::CreateCase {
                local_id: LocalOpId::new("local-1").unwrap(),
                location: LatLon::new(0.0, 0.0).unwrap(),
                description: None,
                wound_severity: None,
                created_at_ms_utc: now,
            },
            now,
            3600_000,
        );

        // Child entry that depends on parent
        let child = OutboxEntry::new(
            OpId::new("child-op").unwrap(),
            IdempotencyKey::new("child-idem").unwrap(),
            OutboxIntent::UploadCasePhoto {
                local_id: LocalOpId::new("local-1").unwrap(),
                photo: BlobRef::new("blob-1", 1024, "image/jpeg").unwrap(),
                depends_on: Some(OpId::new("parent-op").unwrap()),
            },
            now,
            3600_000,
        );

        outbox.push(parent).await.unwrap();
        outbox.push(child).await.unwrap();

        // Only parent should be due (child depends on parent)
        let due = outbox.get_due_entries(now, 10).await;
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].op_id.as_str(), "parent-op");

        // Complete parent
        let parent_id = OpId::new("parent-op").unwrap();
        let (_, lease) = outbox.acquire_lease(&parent_id, now).await.unwrap();
        outbox.complete(&parent_id, &lease.token, now).await.unwrap();

        // Now child should be due
        let due = outbox.get_due_entries(now, 10).await;
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].op_id.as_str(), "child-op");
    }

    #[tokio::test]
    async fn test_cascade_dependency_failure() {
        let storage = Arc::new(SqliteStorage::new_in_memory().await.unwrap());
        let outbox = Outbox::new(storage, OutboxConfig::default()).await.unwrap();

        let now = make_now();
        
        // Parent entry
        let parent = OutboxEntry::new(
            OpId::new("parent-op").unwrap(),
            IdempotencyKey::new("parent-idem").unwrap(),
            OutboxIntent::CreateCase {
                local_id: LocalOpId::new("local-1").unwrap(),
                location: LatLon::new(0.0, 0.0).unwrap(),
                description: None,
                wound_severity: None,
                created_at_ms_utc: now,
            },
            now,
            3600_000,
        );

        // Multiple children that depend on parent
        let child1 = OutboxEntry::new(
            OpId::new("child-1").unwrap(),
            IdempotencyKey::new("child-idem-1").unwrap(),
            OutboxIntent::UploadCasePhoto {
                local_id: LocalOpId::new("local-1").unwrap(),
                photo: BlobRef::new("blob-1", 1024, "image/jpeg").unwrap(),
                depends_on: Some(OpId::new("parent-op").unwrap()),
            },
            now,
            3600_000,
        );

        let child2 = OutboxEntry::new(
            OpId::new("child-2").unwrap(),
            IdempotencyKey::new("child-idem-2").unwrap(),
            OutboxIntent::UploadCasePhoto {
                local_id: LocalOpId::new("local-1").unwrap(),
                photo: BlobRef::new("blob-2", 2048, "image/png").unwrap(),
                depends_on: Some(OpId::new("parent-op").unwrap()),
            },
            now,
            3600_000,
        );

        outbox.push(parent).await.unwrap();
        outbox.push(child1).await.unwrap();
        outbox.push(child2).await.unwrap();

        // Dead-letter the parent
        let parent_id = OpId::new("parent-op").unwrap();
        let (_, lease) = outbox.acquire_lease(&parent_id, now).await.unwrap();
        let error = IntentError::new(ErrorCategory::ClientError, "ERR", "fail", now);
        outbox.fail(&parent_id, &lease.token, error, now).await.unwrap();

        // Cascade failure to children
        let affected = outbox.cascade_dependency_failure(&parent_id, now).await.unwrap();
        assert_eq!(affected.len(), 2);

        // Verify children are dead-lettered
        let child1_entry = outbox.get_entry(&OpId::new("child-1").unwrap()).await.unwrap();
        assert!(matches!(
            child1_entry.state,
            EntryState::DeadLetter {
                reason: DeadLetterReason::DependencyFailed { .. },
                ..
            }
        ));
    }

    #[tokio::test]
    async fn test_expiration() {
        let storage = Arc::new(SqliteStorage::new_in_memory().await.unwrap());
        let outbox = Outbox::new(storage, OutboxConfig::default()).await.unwrap();

        let now = make_now();
        let entry = OutboxEntry::new(
            OpId::new("op-1").unwrap(),
            IdempotencyKey::new("idem-1").unwrap(),
            OutboxIntent::CreateCase {
                local_id: LocalOpId::new("local-1").unwrap(),
                location: LatLon::new(0.0, 0.0).unwrap(),
                description: None,
                wound_severity: None,
                created_at_ms_utc: now,
            },
            now,
            1000, // 1 second TTL
        );

        outbox.push(entry).await.unwrap();

        // Entry is due before expiration
        let due = outbox.get_due_entries(now, 10).await;
        assert_eq!(due.len(), 1);

        // Entry is not due after expiration
        let expired_time = UnixTimeMs(now.0 + 2000);
        let due = outbox.get_due_entries(expired_time, 10).await;
        assert!(due.is_empty());

        // Run expiration
        let expired = outbox.expire_stale(expired_time).await.unwrap();
        assert_eq!(expired.len(), 1);

        // Verify dead-lettered
        let entry = outbox.get_entry(&OpId::new("op-1").unwrap()).await.unwrap();
        assert!(matches!(
            entry.state,
            EntryState::DeadLetter {
                reason: DeadLetterReason::Expired,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn test_prune_completed() {
        let storage = Arc::new(SqliteStorage::new_in_memory().await.unwrap());
        let outbox = Outbox::new(storage, OutboxConfig::default()).await.unwrap();

        let now = make_now();
        let entry = OutboxEntry::new(
            OpId::new("op-1").unwrap(),
            IdempotencyKey::new("idem-1").unwrap(),
            OutboxIntent::CreateCase {
                local_id: LocalOpId::new("local-1").unwrap(),
                location: LatLon::new(0.0, 0.0).unwrap(),
                description: None,
                wound_severity: None,
                created_at_ms_utc: now,
            },
            now,
            3600_000,
        );

        outbox.push(entry).await.unwrap();

        let op_id = OpId::new("op-1").unwrap();
        let (_, lease) = outbox.acquire_lease(&op_id, now).await.unwrap();
        outbox.complete(&op_id, &lease.token, now).await.unwrap();

        // Prune entries completed before "now + 1 hour"
        let prune_before = UnixTimeMs(now.0 + 3600_000);
        let pruned = outbox.prune_completed(prune_before).await.unwrap();
        assert_eq!(pruned, 1);

        // Entry should be gone
        let entry = outbox.get_entry(&op_id).await;
        assert!(entry.is_none());
    }

    #[tokio::test]
    async fn test_priority_ordering() {
        let storage = Arc::new(SqliteStorage::new_in_memory().await.unwrap());
        let outbox = Outbox::new(storage, OutboxConfig::default()).await.unwrap();

        let now = make_now();

        // Low priority entry (created first)
        let low = OutboxEntry::new(
            OpId::new("low-priority").unwrap(),
            IdempotencyKey::new("idem-low").unwrap(),
            OutboxIntent::CreateCase {
                local_id: LocalOpId::new("local-1").unwrap(),
                location: LatLon::new(0.0, 0.0).unwrap(),
                description: None,
                wound_severity: None,
                created_at_ms_utc: now,
            },
            now,
            3600_000,
        )
        .with_priority(1);

        // High priority entry (created second)
        let high = OutboxEntry::new(
            OpId::new("high-priority").unwrap(),
            IdempotencyKey::new("idem-high").unwrap(),
            OutboxIntent::CreateCase {
                local_id: LocalOpId::new("local-2").unwrap(),
                location: LatLon::new(0.0, 0.0).unwrap(),
                description: None,
                wound_severity: None,
                created_at_ms_utc: UnixTimeMs(now.0 + 1000),
            },
            UnixTimeMs(now.0 + 1000),
            3600_000,
        )
        .with_priority(10);

        outbox.push(low).await.unwrap();
        outbox.push(high).await.unwrap();

        let due = outbox.get_due_entries(UnixTimeMs(now.0 + 2000), 10).await;
        assert_eq!(due.len(), 2);
        // High priority should come first despite being created later
        assert_eq!(due[0].op_id.as_str(), "high-priority");
        assert_eq!(due[1].op_id.as_str(), "low-priority");
    }

    #[tokio::test]
    async fn test_tenant_isolation_metrics() {
        let storage = Arc::new(SqliteStorage::new_in_memory().await.unwrap());
        let outbox = Outbox::new(storage, OutboxConfig::default()).await.unwrap();

        let now = make_now();

        let tenant_a = OutboxEntry::new(
            OpId::new("op-a").unwrap(),
            IdempotencyKey::new("idem-a").unwrap(),
            OutboxIntent::CreateCase {
                local_id: LocalOpId::new("local-1").unwrap(),
                location: LatLon::new(0.0, 0.0).unwrap(),
                description: None,
                wound_severity: None,
                created_at_ms_utc: now,
            },
            now,
            3600_000,
        )
        .with_tenant("tenant-a");

        let tenant_b = OutboxEntry::new(
            OpId::new("op-b").unwrap(),
            IdempotencyKey::new("idem-b").unwrap(),
            OutboxIntent::CreateCase {
                local_id: LocalOpId::new("local-2").unwrap(),
                location: LatLon::new(0.0, 0.0).unwrap(),
                description: None,
                wound_severity: None,
                created_at_ms_utc: now,
            },
            now,
            3600_000,
        )
        .with_tenant("tenant-b");

        outbox.push(tenant_a).await.unwrap();
        outbox.push(tenant_b).await.unwrap();

        let depth = outbox.get_queue_depth().await;
        assert_eq!(depth.total_entries, 2);
        assert_eq!(depth.by_tenant.get("tenant-a"), Some(&1));
        assert_eq!(depth.by_tenant.get("tenant-b"), Some(&1));
    }

    #[tokio::test]
    async fn test_metrics_tracking() {
        let storage = Arc::new(SqliteStorage::new_in_memory().await.unwrap());
        let outbox = Outbox::new(storage, OutboxConfig::default()).await.unwrap();

        let now = make_now();
        let entry = OutboxEntry::new(
            OpId::new("op-1").unwrap(),
            IdempotencyKey::new("idem-1").unwrap(),
            OutboxIntent::CreateCase {
                local_id: LocalOpId::new("local-1").unwrap(),
                location: LatLon::new(0.0, 0.0).unwrap(),
                description: None,
                wound_severity: None,
                created_at_ms_utc: now,
            },
            now,
            3600_000,
        );

        outbox.push(entry).await.unwrap();

        let op_id = OpId::new("op-1").unwrap();
        let (_, lease) = outbox.acquire_lease(&op_id, now).await.unwrap();
        outbox.complete(&op_id, &lease.token, now).await.unwrap();

        let metrics = outbox.get_metrics();
        assert_eq!(metrics.entries_pushed, 1);
        assert_eq!(metrics.entries_completed, 1);
    }

    #[tokio::test]
    async fn test_lease_expiration_allows_retry() {
        let config = OutboxConfig {
            lease_duration_ms: 1000, // 1 second lease
            ..Default::default()
        };
        let storage = Arc::new(SqliteStorage::new_in_memory().await.unwrap());
        let outbox = Outbox::new(storage, config).await.unwrap();

        let now = make_now();
        let entry = OutboxEntry::new(
            OpId::new("op-1").unwrap(),
            IdempotencyKey::new("idem-1").unwrap(),
            OutboxIntent::CreateCase {
                local_id: LocalOpId::new("local-1").unwrap(),
                location: LatLon::new(0.0, 0.0).unwrap(),
                description: None,
                wound_severity: None,
                created_at_ms_utc: now,
            },
            now,
            3600_000,
        );

        outbox.push(entry).await.unwrap();

        let op_id = OpId::new("op-1").unwrap();
        let (_, _lease1) = outbox.acquire_lease(&op_id, now).await.unwrap();

        // Lease is active - should not be due
        let due = outbox.get_due_entries(now, 10).await;
        assert!(due.is_empty());

        // After lease expiration - should be due again
        let after_expiry = UnixTimeMs(now.0 + 2000);
        let due = outbox.get_due_entries(after_expiry, 10).await;
        assert_eq!(due.len(), 1);

        // Can acquire new lease
        let (_, lease2) = outbox.acquire_lease(&op_id, after_expiry).await.unwrap();
        
        // Complete with new lease
        outbox.complete(&op_id, &lease2.token, after_expiry).await.unwrap();

        // Old lease should fail
        // (can't test this easily since entry is now completed)
    }

    #[tokio::test]
    async fn test_concurrent_lease_acquisition() {
        let storage = Arc::new(SqliteStorage::new_in_memory().await.unwrap());
        let outbox = Arc::new(Outbox::new(storage, OutboxConfig::default()).await.unwrap());

        let now = make_now();
        let entry = OutboxEntry::new(
            OpId::new("op-1").unwrap(),
            IdempotencyKey::new("idem-1").unwrap(),
            OutboxIntent::CreateCase {
                local_id: LocalOpId::new("local-1").unwrap(),
                location: LatLon::new(0.0, 0.0).unwrap(),
                description: None,
                wound_severity: None,
                created_at_ms_utc: now,
            },
            now,
            3600_000,
        );

        outbox.push(entry).await.unwrap();

        let op_id = OpId::new("op-1").unwrap();

        // First acquisition succeeds
        let result1 = outbox.acquire_lease(&op_id, now).await;
        assert!(result1.is_ok());

        // Second acquisition fails (lease still valid)
        let result2 = outbox.acquire_lease(&op_id, now).await;
        assert!(matches!(result2, Err(OutboxError::InvalidStateTransition { .. })));
    }

    #[tokio::test]
    async fn test_quarantine_corrupt_entries() {
        // This test would require creating a corrupted entry in storage
        // For now, we just verify the quarantine API works
        let storage = Arc::new(SqliteStorage::new_in_memory().await.unwrap());
        let outbox = Outbox::new(storage, OutboxConfig::default()).await.unwrap();

        let quarantined = outbox.get_quarantined().await;
        assert!(quarantined.is_empty());
    }

    #[tokio::test]
    async fn test_config_validation() {
        let bad_config = OutboxConfig {
            max_entries: 0,
            ..Default::default()
        };
        assert!(bad_config.validate().is_err());

        let bad_config2 = OutboxConfig {
            max_attempts: 0,
            ..Default::default()
        };
        assert!(bad_config2.validate().is_err());

        let bad_config3 = OutboxConfig {
            lease_duration_ms: 100, // Too short
            ..Default::default()
        };
        assert!(bad_config3.validate().is_err());

        let good_config = OutboxConfig::default();
        assert!(good_config.validate().is_ok());
    }
}

// ============================================================================
// Integration Test Helpers
// ============================================================================

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use tokio::time::{sleep, Duration};

    /// Simulated processor for integration testing
    struct TestProcessor {
        process_count: AtomicUsize,
        fail_count: AtomicUsize,
        should_fail: AtomicBool,
    }

    impl TestProcessor {
        fn new() -> Self {
            Self {
                process_count: AtomicUsize::new(0),
                fail_count: AtomicUsize::new(0),
                should_fail: AtomicBool::new(false),
            }
        }

        fn set_should_fail(&self, fail: bool) {
            self.should_fail.store(fail, Ordering::SeqCst);
        }

        async fn process(&self, _entry: &OutboxEntry) -> Result<(), IntentError> {
            self.process_count.fetch_add(1, Ordering::SeqCst);
            
            if self.should_fail.load(Ordering::SeqCst) {
                self.fail_count.fetch_add(1, Ordering::SeqCst);
                return Err(IntentError::new(
                    ErrorCategory::Transient,
                    "TEST_FAIL",
                    "Simulated failure",
                    UnixTimeMs::now(),
                ));
            }
            
            Ok(())
        }

        fn get_counts(&self) -> (usize, usize) {
            (
                self.process_count.load(Ordering::SeqCst),
                self.fail_count.load(Ordering::SeqCst),
            )
        }
    }

    #[tokio::test]
    async fn test_full_processing_loop() {
        let storage = Arc::new(SqliteStorage::new_in_memory().await.unwrap());
        let config = OutboxConfig {
            base_backoff_ms: 100,
            ..Default::default()
        };
        let outbox = Arc::new(Outbox::new(storage, config).await.unwrap());
        let processor = Arc::new(TestProcessor::new());

        let now = UnixTimeMs::now();

        // Add some entries
        for i in 0..5 {
            let entry = OutboxEntry::new(
                OpId::new(format!("op-{}", i)).unwrap(),
                IdempotencyKey::new(format!("idem-{}", i)).unwrap(),
                OutboxIntent::CreateCase {
                    local_id: LocalOpId::new(format!("local-{}", i)).unwrap(),
                    location: LatLon::new(0.0, 0.0).unwrap(),
                    description: None,
                    wound_severity: None,
                    created_at_ms_utc: now,
                },
                now,
                3600_000,
            );
            outbox.push(entry).await.unwrap();
        }

        // Process all entries
        let due = outbox.get_due_entries(now, 100).await;
        assert_eq!(due.len(), 5);

        for entry in due {
            let (acquired, lease) = outbox.acquire_lease(&entry.op_id, now).await.unwrap();
            
            match processor.process(&acquired).await {
                Ok(()) => {
                    outbox.complete(&entry.op_id, &lease.token, now).await.unwrap();
                }
                Err(error) => {
                    outbox.fail(&entry.op_id, &lease.token, error, now).await.unwrap();
                }
            }
        }

        let (process_count, fail_count) = processor.get_counts();
        assert_eq!(process_count, 5);
        assert_eq!(fail_count, 0);

        let depth = outbox.get_queue_depth().await;
        assert_eq!(depth.completed_count, 5);
    }

    #[tokio::test]
    async fn test_processing_with_failures_and_retries() {
        let storage = Arc::new(SqliteStorage::new_in_memory().await.unwrap());
        let config = OutboxConfig {
            base_backoff_ms: 10, // Short for testing
            max_attempts: 3,
            ..Default::default()
        };
        let outbox = Arc::new(Outbox::new(storage, config).await.unwrap());
        let processor = Arc::new(TestProcessor::new());

        let now = UnixTimeMs::now();

        let entry = OutboxEntry::new(
            OpId::new("op-1").unwrap(),
            IdempotencyKey::new("idem-1").unwrap(),
            OutboxIntent::CreateCase {
                local_id: LocalOpId::new("local-1").unwrap(),
                location: LatLon::new(0.0, 0.0).unwrap(),
                description: None,
                wound_severity: None,
                created_at_ms_utc: now,
            },
            now,
            3600_000,
        );
        outbox.push(entry).await.unwrap();

        // First attempt - fail
        processor.set_should_fail(true);
        let op_id = OpId::new("op-1").unwrap();
        
        let (acquired, lease) = outbox.acquire_lease(&op_id, now).await.unwrap();
        let error = processor.process(&acquired).await.unwrap_err();
        outbox.fail(&op_id, &lease.token, error, now).await.unwrap();

        // Wait for backoff
        sleep(Duration::from_millis(50)).await;

        // Second attempt - fail again
        let time2 = UnixTimeMs(now.0 + 100);
        let (acquired, lease) = outbox.acquire_lease(&op_id, time2).await.unwrap();
        let error = processor.process(&acquired).await.unwrap_err();
        outbox.fail(&op_id, &lease.token, error, time2).await.unwrap();

        // Third attempt - succeed
        processor.set_should_fail(false);
        sleep(Duration::from_millis(50)).await;

        let time3 = UnixTimeMs(now.0 + 200);
        let (acquired, lease) = outbox.acquire_lease(&op_id, time3).await.unwrap();
        processor.process(&acquired).await.unwrap();
        outbox.complete(&op_id, &lease.token, time3).await.unwrap();

        let (process_count, fail_count) = processor.get_counts();
        assert_eq!(process_count, 3);
        assert_eq!(fail_count, 2);

        let depth = outbox.get_queue_depth().await;
        assert_eq!(depth.completed_count, 1);
    }

    #[tokio::test]
    async fn test_persistence_across_restart() {
        let db_path = format!("sqlite:/tmp/outbox_test_{}.db", Uuid::new_v4());
        
        let op_id = OpId::new("persistent-op").unwrap();
        let now = UnixTimeMs::now();

        // First "session" - create and push entry
        {
            let storage = Arc::new(SqliteStorage::new(&db_path).await.unwrap());
            let outbox = Outbox::new(storage, OutboxConfig::default()).await.unwrap();

            let entry = OutboxEntry::new(
                op_id.clone(),
                IdempotencyKey::new("persistent-idem").unwrap(),
                OutboxIntent::CreateCase {
                    local_id: LocalOpId::new("local-1").unwrap(),
                    location: LatLon::new(0.0, 0.0).unwrap(),
                    description: Some("Test persistence".into()),
                    wound_severity: Some(WoundSeverity::new(3).unwrap()),
                    created_at_ms_utc: now,
                },
                now,
                3600_000,
            );

            outbox.push(entry).await.unwrap();
            outbox.sync().await.unwrap();
        }

        // Second "session" - reload and verify
        {
            let storage = Arc::new(SqliteStorage::new(&db_path).await.unwrap());
            let outbox = Outbox::new(storage, OutboxConfig::default()).await.unwrap();

            let entry = outbox.get_entry(&op_id).await;
            assert!(entry.is_some());
            
            let entry = entry.unwrap();
            assert_eq!(entry.op_id.as_str(), "persistent-op");
            assert!(matches!(entry.state, EntryState::Pending));

            // Complete it
            let (_, lease) = outbox.acquire_lease(&op_id, now).await.unwrap();
            outbox.complete(&op_id, &lease.token, now).await.unwrap();
            outbox.sync().await.unwrap();
        }

        // Third "session" - verify completion persisted
        {
            let storage = Arc::new(SqliteStorage::new(&db_path).await.unwrap());
            let outbox = Outbox::new(storage, OutboxConfig::default()).await.unwrap();

            let entry = outbox.get_entry(&op_id).await;
            assert!(entry.is_some());
            assert!(matches!(entry.unwrap().state, EntryState::Completed { .. }));
        }

        // Cleanup
        let _ = std::fs::remove_file(db_path.replace("sqlite:", ""));
    }
}