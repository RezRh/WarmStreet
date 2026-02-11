// lib.rs - Complete Production Implementation

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::too_many_lines)]

pub mod capabilities;
pub mod vision;
pub mod image_processing;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use thiserror::Error;
use uuid::Uuid;

pub use app::App;
pub use capabilities::Capabilities;
pub use crux_core::{render::Render, App as CruxApp, Effect};

pub const CURRENT_KEY_VERSION: u32 = 1;
pub const DEFAULT_RADIUS_M: u32 = 5000;
pub const MIN_RADIUS_M: u32 = 500;
pub const MAX_RADIUS_M: u32 = 50000;
pub const DEFAULT_MAP_ZOOM: f64 = 14.0;
pub const MIN_ZOOM: f64 = 5.0;
pub const MAX_ZOOM: f64 = 20.0;
pub const FALLBACK_ZOOM: f64 = 10.0;
pub const DESCRIPTION_PREVIEW_LENGTH: usize = 80;
pub const EARTH_RADIUS_M: f64 = 6_371_000.0;
pub const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;
pub const MAX_IMAGE_DIMENSION: u32 = 4096;
pub const MAX_IMAGE_ALLOC: usize = 100 * 1024 * 1024;
pub const MAX_PROCESSED_DIMENSION: u32 = 1920;
pub const MAX_PENDING_LOCAL_CASES: usize = 100;
pub const MAX_OUTBOX_ENTRIES: usize = 50;
pub const MAX_CACHED_SERVER_CASES: usize = 500;
pub const CLAIM_TIMEOUT: Duration = Duration::from_secs(30);
pub const TRANSITION_TIMEOUT: Duration = Duration::from_secs(30);
pub const CREATE_CASE_TIMEOUT: Duration = Duration::from_secs(60);
pub const REFRESH_TIMEOUT: Duration = Duration::from_secs(30);
pub const FCM_SYNC_TIMEOUT: Duration = Duration::from_secs(15);
pub const UPLOAD_TIMEOUT: Duration = Duration::from_secs(120);
pub const MAX_RETRY_ATTEMPTS: u32 = 5;
pub const BASE_RETRY_DELAY_MS: u64 = 1000;
pub const MAX_RETRY_DELAY_MS: u64 = 60000;
pub const JITTER_MAX_MS: u64 = 1000;

pub const RADIUS_ZOOM_MAP: &[(u32, f64)] = &[
    (1000, 16.0),
    (2000, 15.0),
    (5000, 14.0),
    (10000, 13.0),
    (20000, 12.0),
    (50000, 11.0),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Transient,
    Permanent,
    Fatal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorKind {
    Network,
    Timeout,
    Authentication,
    Authorization,
    Validation,
    NotFound,
    Conflict,
    RateLimited,
    QuotaExceeded,
    Storage,
    Serialization,
    Deserialization,
    ImageProcessing,
    ImageTooLarge,
    ImageDimensionsTooLarge,
    ImageFormatUnsupported,
    Camera,
    CameraPermissionDenied,
    Location,
    LocationPermissionDenied,
    Crypto,
    CryptoKeyNotFound,
    FeatureUnavailable,
    InvalidState,
    Internal,
    Unknown,
}

impl ErrorKind {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Network => "NETWORK_ERROR",
            Self::Timeout => "TIMEOUT",
            Self::Authentication => "AUTH_ERROR",
            Self::Authorization => "FORBIDDEN",
            Self::Validation => "VALIDATION_ERROR",
            Self::NotFound => "NOT_FOUND",
            Self::Conflict => "CONFLICT",
            Self::RateLimited => "RATE_LIMITED",
            Self::QuotaExceeded => "QUOTA_EXCEEDED",
            Self::Storage => "STORAGE_ERROR",
            Self::Serialization => "SERIALIZATION_ERROR",
            Self::Deserialization => "DESERIALIZATION_ERROR",
            Self::ImageProcessing => "IMAGE_PROCESSING_ERROR",
            Self::ImageTooLarge => "IMAGE_TOO_LARGE",
            Self::ImageDimensionsTooLarge => "IMAGE_DIMENSIONS_TOO_LARGE",
            Self::ImageFormatUnsupported => "IMAGE_FORMAT_UNSUPPORTED",
            Self::Camera => "CAMERA_ERROR",
            Self::CameraPermissionDenied => "CAMERA_PERMISSION_DENIED",
            Self::Location => "LOCATION_ERROR",
            Self::LocationPermissionDenied => "LOCATION_PERMISSION_DENIED",
            Self::Crypto => "CRYPTO_ERROR",
            Self::CryptoKeyNotFound => "CRYPTO_KEY_NOT_FOUND",
            Self::FeatureUnavailable => "FEATURE_UNAVAILABLE",
            Self::InvalidState => "INVALID_STATE",
            Self::Internal => "INTERNAL_ERROR",
            Self::Unknown => "UNKNOWN_ERROR",
        }
    }

    #[must_use]
    pub const fn default_severity(self) -> ErrorSeverity {
        match self {
            Self::Network
            | Self::Timeout
            | Self::Conflict
            | Self::RateLimited
            | Self::Storage
            | Self::Camera
            | Self::Location => ErrorSeverity::Transient,

            Self::Serialization
            | Self::Deserialization
            | Self::Crypto
            | Self::CryptoKeyNotFound
            | Self::Internal
            | Self::InvalidState => ErrorSeverity::Fatal,

            Self::Authentication
            | Self::Authorization
            | Self::Validation
            | Self::NotFound
            | Self::QuotaExceeded
            | Self::ImageProcessing
            | Self::ImageTooLarge
            | Self::ImageDimensionsTooLarge
            | Self::ImageFormatUnsupported
            | Self::CameraPermissionDenied
            | Self::LocationPermissionDenied
            | Self::FeatureUnavailable
            | Self::Unknown => ErrorSeverity::Permanent,
        }
    }

    #[must_use]
    pub const fn is_retryable(self) -> bool {
        matches!(
            self,
            Self::Network
                | Self::Timeout
                | Self::RateLimited
                | Self::Storage
                | Self::Conflict
                | Self::Camera
                | Self::Location
        )
    }

    #[must_use]
    pub const fn http_status_hint(self) -> Option<u16> {
        match self {
            Self::Authentication => Some(401),
            Self::Authorization => Some(403),
            Self::NotFound => Some(404),
            Self::Conflict => Some(409),
            Self::RateLimited => Some(429),
            Self::QuotaExceeded => Some(402),
            Self::Validation => Some(400),
            Self::Internal => Some(500),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppError {
    pub kind: ErrorKind,
    pub severity: ErrorSeverity,
    pub message: String,
    pub internal_message: Option<String>,
    pub retry_after_ms: Option<u64>,
    pub context: HashMap<String, String>,
}

impl AppError {
    #[must_use]
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            severity: kind.default_severity(),
            message: message.into(),
            internal_message: None,
            retry_after_ms: None,
            context: HashMap::new(),
        }
    }

    #[must_use]
    pub fn with_internal(mut self, internal: impl Into<String>) -> Self {
        self.internal_message = Some(internal.into());
        self
    }

    #[must_use]
    pub fn with_retry_after(mut self, ms: u64) -> Self {
        self.retry_after_ms = Some(ms);
        self
    }

    #[must_use]
    pub fn with_severity(mut self, severity: ErrorSeverity) -> Self {
        self.severity = severity;
        self
    }

    #[must_use]
    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }

    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.kind.code()
    }

    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        self.kind.is_retryable() && !matches!(self.severity, ErrorSeverity::Fatal)
    }

    #[must_use]
    pub fn user_facing_message(&self) -> String {
        match self.kind {
            ErrorKind::Network => {
                "Unable to connect. Please check your internet connection and try again.".into()
            }
            ErrorKind::Timeout => "The request timed out. Please try again.".into(),
            ErrorKind::Authentication => {
                "Your session has expired. Please sign in again.".into()
            }
            ErrorKind::Authorization => {
                "You don't have permission to perform this action.".into()
            }
            ErrorKind::Validation => self.message.clone(),
            ErrorKind::NotFound => "The requested item could not be found.".into(),
            ErrorKind::Conflict => {
                "This action conflicts with a recent change. Please refresh and try again.".into()
            }
            ErrorKind::RateLimited => {
                if let Some(retry_after) = self.retry_after_ms {
                    let seconds = retry_after / 1000;
                    format!("Too many requests. Please wait {} seconds and try again.", seconds)
                } else {
                    "Too many requests. Please wait a moment and try again.".into()
                }
            }
            ErrorKind::QuotaExceeded => {
                "You've reached your usage limit. Please upgrade or wait for the limit to reset."
                    .into()
            }
            ErrorKind::Storage => {
                "Unable to save data locally. Please free up some storage space.".into()
            }
            ErrorKind::Serialization | ErrorKind::Deserialization => {
                "A data error occurred. Please contact support if this persists.".into()
            }
            ErrorKind::ImageProcessing => {
                "Unable to process the image. Please try a different photo.".into()
            }
            ErrorKind::ImageTooLarge => {
                format!(
                    "The image is too large. Please use an image smaller than {} MB.",
                    MAX_IMAGE_BYTES / 1_000_000
                )
            }
            ErrorKind::ImageDimensionsTooLarge => {
                format!(
                    "The image dimensions are too large. Maximum supported is {}x{} pixels.",
                    MAX_IMAGE_DIMENSION, MAX_IMAGE_DIMENSION
                )
            }
            ErrorKind::ImageFormatUnsupported => {
                "This image format is not supported. Please use JPEG, PNG, or WebP.".into()
            }
            ErrorKind::Camera => "Camera error. Please close and reopen the camera.".into(),
            ErrorKind::CameraPermissionDenied => {
                "Camera access is required. Please enable camera permissions in Settings.".into()
            }
            ErrorKind::Location => {
                "Unable to determine your location. Please check your GPS settings.".into()
            }
            ErrorKind::LocationPermissionDenied => {
                "Location access is required. Please enable location permissions in Settings."
                    .into()
            }
            ErrorKind::Crypto | ErrorKind::CryptoKeyNotFound => {
                "A security error occurred. Please sign in again.".into()
            }
            ErrorKind::FeatureUnavailable => self.message.clone(),
            ErrorKind::InvalidState => {
                "The app is in an invalid state. Please restart the app.".into()
            }
            ErrorKind::Internal | ErrorKind::Unknown => {
                "An unexpected error occurred. Please try again or contact support.".into()
            }
        }
    }

    #[must_use]
    pub fn from_http_status(status: u16, body: Option<&[u8]>) -> Self {
        let kind = match status {
            400 => ErrorKind::Validation,
            401 => ErrorKind::Authentication,
            403 => ErrorKind::Authorization,
            404 => ErrorKind::NotFound,
            409 => ErrorKind::Conflict,
            429 => ErrorKind::RateLimited,
            402 => ErrorKind::QuotaExceeded,
            408 => ErrorKind::Timeout,
            500..=599 => ErrorKind::Internal,
            _ => ErrorKind::Unknown,
        };

        let message = body
            .and_then(|b| serde_json::from_slice::<ApiErrorResponse>(b).ok())
            .map(|e| e.message)
            .unwrap_or_else(|| format!("HTTP error: {status}"));

        Self::new(kind, message).with_context("http_status", status.to_string())
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code(), self.message)?;
        if let Some(internal) = &self.internal_message {
            write!(f, " (internal: {internal})")?;
        }
        Ok(())
    }
}

impl std::error::Error for AppError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiErrorResponse {
    #[serde(default)]
    message: String,
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    details: Option<HashMap<String, String>>,
}

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Clone, Error)]
pub enum CoordinateError {
    #[error("Latitude {0} is out of valid range [-90, 90]")]
    LatitudeOutOfRange(f64),
    #[error("Longitude {0} is out of valid range [-180, 180]")]
    LongitudeOutOfRange(f64),
    #[error("Coordinate value is not finite (NaN or Infinity)")]
    NonFinite,
}

impl From<CoordinateError> for AppError {
    fn from(e: CoordinateError) -> Self {
        AppError::new(ErrorKind::Validation, e.to_string())
    }
}

#[derive(Debug, Clone, Error)]
pub enum OutboxError {
    #[error("Outbox is full (maximum {max} entries)")]
    Full { max: usize },
    #[error("Duplicate operation ID: {0}")]
    DuplicateOpId(String),
    #[error("Entry not found: {0}")]
    NotFound(String),
    #[error("Entry is in invalid state for this operation")]
    InvalidState,
}

impl From<OutboxError> for AppError {
    fn from(e: OutboxError) -> Self {
        match e {
            OutboxError::Full { .. } => {
                AppError::new(ErrorKind::QuotaExceeded, "Too many pending operations")
            }
            OutboxError::DuplicateOpId(_) => {
                AppError::new(ErrorKind::Conflict, "Operation already exists")
            }
            OutboxError::NotFound(_) => AppError::new(ErrorKind::NotFound, "Operation not found"),
            OutboxError::InvalidState => {
                AppError::new(ErrorKind::InvalidState, "Invalid operation state")
            }
        }
    }
}

#[derive(Debug, Clone, Error)]
pub enum ImageError {
    #[error("Image size {size} bytes exceeds maximum of {max} bytes")]
    TooLarge { size: usize, max: usize },
    #[error("Image dimensions {width}x{height} exceed maximum of {max}x{max}")]
    DimensionsTooLarge { width: u32, height: u32, max: u32 },
    #[error("Unsupported image format")]
    UnsupportedFormat,
    #[error("Failed to decode image: {0}")]
    DecodeFailed(String),
    #[error("Failed to encode image: {0}")]
    EncodeFailed(String),
    #[error("Image processing failed: {0}")]
    ProcessingFailed(String),
}

impl From<ImageError> for AppError {
    fn from(e: ImageError) -> Self {
        let kind = match &e {
            ImageError::TooLarge { .. } => ErrorKind::ImageTooLarge,
            ImageError::DimensionsTooLarge { .. } => ErrorKind::ImageDimensionsTooLarge,
            ImageError::UnsupportedFormat => ErrorKind::ImageFormatUnsupported,
            ImageError::DecodeFailed(_)
            | ImageError::EncodeFailed(_)
            | ImageError::ProcessingFailed(_) => ErrorKind::ImageProcessing,
        };
        AppError::new(kind, e.to_string())
    }
}

#[derive(Debug, Clone, Error)]
pub enum PersistenceError {
    #[error("No user ID available for key derivation")]
    NoUserId,
    #[error("Serialization failed: {0}")]
    SerializationFailed(String),
    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),
    #[error("Storage write failed: {0}")]
    WriteFailed(String),
    #[error("Storage read failed: {0}")]
    ReadFailed(String),
    #[error("Key not found: {0}")]
    KeyNotFound(String),
}

impl From<PersistenceError> for AppError {
    fn from(e: PersistenceError) -> Self {
        let kind = match &e {
            PersistenceError::NoUserId => ErrorKind::InvalidState,
            PersistenceError::SerializationFailed(_) => ErrorKind::Serialization,
            PersistenceError::DeserializationFailed(_) => ErrorKind::Deserialization,
            PersistenceError::EncryptionFailed(_) | PersistenceError::DecryptionFailed(_) => {
                ErrorKind::Crypto
            }
            PersistenceError::WriteFailed(_) | PersistenceError::ReadFailed(_) => ErrorKind::Storage,
            PersistenceError::KeyNotFound(_) => ErrorKind::CryptoKeyNotFound,
        };
        AppError::new(kind, e.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ValidatedCoordinate {
    lat: f64,
    lon: f64,
}

impl ValidatedCoordinate {
    pub fn new(lat: f64, lon: f64) -> Result<Self, CoordinateError> {
        if !lat.is_finite() {
            return Err(CoordinateError::NonFinite);
        }
        if !lon.is_finite() {
            return Err(CoordinateError::NonFinite);
        }
        if !(-90.0..=90.0).contains(&lat) {
            return Err(CoordinateError::LatitudeOutOfRange(lat));
        }
        if !(-180.0..=180.0).contains(&lon) {
            return Err(CoordinateError::LongitudeOutOfRange(lon));
        }
        Ok(Self { lat, lon })
    }

    #[must_use]
    pub const fn lat(self) -> f64 {
        self.lat
    }

    #[must_use]
    pub const fn lon(self) -> f64 {
        self.lon
    }

    #[must_use]
    pub const fn as_tuple(self) -> (f64, f64) {
        (self.lat, self.lon)
    }

    #[must_use]
    pub fn distance_to(self, other: Self) -> f64 {
        haversine_distance(self, other)
    }
}

impl Default for ValidatedCoordinate {
    fn default() -> Self {
        Self { lat: 0.0, lon: 0.0 }
    }
}

impl TryFrom<(f64, f64)> for ValidatedCoordinate {
    type Error = CoordinateError;

    fn try_from((lat, lon): (f64, f64)) -> Result<Self, Self::Error> {
        Self::new(lat, lon)
    }
}

impl TryFrom<LatLon> for ValidatedCoordinate {
    type Error = CoordinateError;

    fn try_from(value: LatLon) -> Result<Self, Self::Error> {
        Self::new(value.lat, value.lon)
    }
}

impl From<ValidatedCoordinate> for LatLon {
    fn from(coord: ValidatedCoordinate) -> Self {
        Self {
            lat: coord.lat,
            lon: coord.lon,
        }
    }
}

#[must_use]
pub fn haversine_distance(p1: ValidatedCoordinate, p2: ValidatedCoordinate) -> f64 {
    const EPSILON: f64 = 1e-10;

    if (p1.lat - p2.lat).abs() < EPSILON && (p1.lon - p2.lon).abs() < EPSILON {
        return 0.0;
    }

    let lat1_rad = p1.lat.to_radians();
    let lat2_rad = p2.lat.to_radians();
    let delta_lat = (p2.lat - p1.lat).to_radians();
    let delta_lon = (p2.lon - p1.lon).to_radians();

    let a = (delta_lat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);

    let a = a.clamp(0.0, 1.0);

    let c = 2.0 * a.sqrt().asin();

    let result = EARTH_RADIUS_M * c;

    if result.is_finite() {
        result
    } else {
        f64::MAX
    }
}

#[must_use]
pub fn format_distance(meters: f64) -> String {
    if !meters.is_finite() || meters < 0.0 {
        return "Unknown".to_string();
    }

    #[allow(clippy::cast_possible_truncation)]
    if meters < 1000.0 {
        format!("{:.0} m", meters)
    } else if meters < 10_000.0 {
        format!("{:.1} km", meters / 1000.0)
    } else if meters < 100_000.0 {
        format!("{:.0} km", meters / 1000.0)
    } else {
        format!("{:.0} km", (meters / 1000.0).round())
    }
}

#[must_use]
pub fn format_time_ago(timestamp_ms: u64, now_ms: u64) -> String {
    if timestamp_ms > now_ms {
        let future_diff_secs = (timestamp_ms.saturating_sub(now_ms)) / 1000;
        return if future_diff_secs < 60 {
            "Just now".into()
        } else {
            "Upcoming".into()
        };
    }

    let diff_secs = now_ms.saturating_sub(timestamp_ms) / 1000;

    if diff_secs < 5 {
        return "Just now".into();
    }
    if diff_secs < 60 {
        return format!("{diff_secs}s ago");
    }

    let diff_mins = diff_secs / 60;
    if diff_mins < 60 {
        return format!("{diff_mins}m ago");
    }

    let diff_hours = diff_mins / 60;
    if diff_hours < 24 {
        return format!("{diff_hours}h ago");
    }

    let diff_days = diff_hours / 24;
    if diff_days < 7 {
        return format!("{diff_days}d ago");
    }
    if diff_days < 30 {
        return format!("{}w ago", diff_days / 7);
    }
    if diff_days < 365 {
        return format!("{}mo ago", diff_days / 30);
    }

    format!("{}y ago", diff_days / 365)
}

#[must_use]
pub fn calculate_retry_delay(attempt: u32, jitter_ms: u64) -> u64 {
    let base = BASE_RETRY_DELAY_MS;
    let exponential = base.saturating_mul(2u64.saturating_pow(attempt));
    let capped = exponential.min(MAX_RETRY_DELAY_MS);
    capped.saturating_add(jitter_ms)
}

#[must_use]
pub fn generate_jitter() -> u64 {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    let state = RandomState::new();
    let mut hasher = state.build_hasher();
    hasher.write_u64(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0),
    );
    hasher.finish() % JITTER_MAX_MS
}

#[must_use]
pub fn get_current_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[must_use]
pub fn zoom_for_radius(radius_m: u32) -> f64 {
    RADIUS_ZOOM_MAP
        .iter()
        .find(|(r, _)| *r >= radius_m)
        .map(|(_, z)| *z)
        .unwrap_or(FALLBACK_ZOOM)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CaseStatus {
    #[default]
    Pending,
    Claimed,
    EnRoute,
    Arrived,
    Resolved,
    Cancelled,
    Expired,
}

impl CaseStatus {
    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "pending" | "open" => Some(Self::Pending),
            "claimed" | "assigned" => Some(Self::Claimed),
            "en_route" | "enroute" | "on_way" | "onway" => Some(Self::EnRoute),
            "arrived" | "on_site" | "onsite" => Some(Self::Arrived),
            "resolved" | "completed" | "done" | "closed" => Some(Self::Resolved),
            "cancelled" | "canceled" | "aborted" => Some(Self::Cancelled),
            "expired" | "timeout" | "timed_out" => Some(Self::Expired),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Claimed => "claimed",
            Self::EnRoute => "en_route",
            Self::Arrived => "arrived",
            Self::Resolved => "resolved",
            Self::Cancelled => "cancelled",
            Self::Expired => "expired",
        }
    }

    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Pending => "Pending",
            Self::Claimed => "Claimed",
            Self::EnRoute => "En Route",
            Self::Arrived => "Arrived",
            Self::Resolved => "Resolved",
            Self::Cancelled => "Cancelled",
            Self::Expired => "Expired",
        }
    }

    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Resolved | Self::Cancelled | Self::Expired)
    }

    #[must_use]
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Claimed | Self::EnRoute | Self::Arrived)
    }

    #[must_use]
    pub const fn is_claimable(self) -> bool {
        matches!(self, Self::Pending)
    }

    #[must_use]
    pub fn valid_transitions(self) -> Vec<Self> {
        match self {
            Self::Pending => vec![Self::Claimed, Self::Cancelled, Self::Expired],
            Self::Claimed => vec![Self::EnRoute, Self::Cancelled],
            Self::EnRoute => vec![Self::Arrived, Self::Cancelled],
            Self::Arrived => vec![Self::Resolved, Self::Cancelled],
            Self::Resolved | Self::Cancelled | Self::Expired => vec![],
        }
    }

    #[must_use]
    pub fn can_transition_to(self, to: Self) -> bool {
        self.valid_transitions().contains(&to)
    }

    pub fn validate_transition(self, to: Self) -> Result<(), TransitionError> {
        if self == to {
            return Err(TransitionError::SameStatus);
        }
        if self.is_terminal() {
            return Err(TransitionError::FromTerminalStatus { status: self });
        }
        if !self.can_transition_to(to) {
            return Err(TransitionError::InvalidTransition { from: self, to });
        }
        Ok(())
    }
}

impl std::fmt::Display for CaseStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TransitionError {
    #[error("Cannot transition to the same status")]
    SameStatus,
    #[error("Cannot transition from terminal status: {status}")]
    FromTerminalStatus { status: CaseStatus },
    #[error("Invalid transition from {from} to {to}")]
    InvalidTransition { from: CaseStatus, to: CaseStatus },
}

impl From<TransitionError> for AppError {
    fn from(e: TransitionError) -> Self {
        AppError::new(ErrorKind::Validation, e.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub String);

impl UserId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CaseId(pub String);

impl CaseId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn generate() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl std::fmt::Display for CaseId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LocalOpId(pub String);

impl LocalOpId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn generate() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl std::fmt::Display for LocalOpId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IdempotencyKey(pub String);

impl IdempotencyKey {
    #[must_use]
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn generate() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl std::fmt::Display for IdempotencyKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OpId(pub String);

impl OpId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn generate() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl std::fmt::Display for OpId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UnixTimeMs(pub u64);

impl UnixTimeMs {
    #[must_use]
    pub fn now() -> Self {
        Self(get_current_time_ms())
    }

    #[must_use]
    pub const fn as_millis(self) -> u64 {
        self.0
    }

    #[must_use]
    pub const fn as_secs(self) -> u64 {
        self.0 / 1000
    }

    #[must_use]
    pub fn elapsed_since(self, earlier: Self) -> u64 {
        self.0.saturating_sub(earlier.0)
    }

    #[must_use]
    pub fn add_millis(self, ms: u64) -> Self {
        Self(self.0.saturating_add(ms))
    }

    #[must_use]
    pub fn is_before(self, other: Self) -> bool {
        self.0 < other.0
    }

    #[must_use]
    pub fn is_after(self, other: Self) -> bool {
        self.0 > other.0
    }
}

impl Default for UnixTimeMs {
    fn default() -> Self {
        Self::now()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub struct LatLon {
    pub lat: f64,
    pub lon: f64,
}

impl LatLon {
    #[must_use]
    pub const fn new(lat: f64, lon: f64) -> Self {
        Self { lat, lon }
    }

    pub fn validate(self) -> Result<ValidatedCoordinate, CoordinateError> {
        ValidatedCoordinate::new(self.lat, self.lon)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FeedView {
    #[default]
    Map,
    List,
}

impl FeedView {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Map => "map",
            Self::List => "list",
        }
    }

    #[must_use]
    pub fn toggle(self) -> Self {
        match self {
            Self::Map => Self::List,
            Self::List => Self::Map,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AppState {
    #[default]
    Loading,
    Unauthenticated,
    Authenticating,
    OnboardingLocation,
    PinDrop,
    OnboardingRadius,
    CameraCapture,
    Ready,
    Error,
}

impl AppState {
    #[must_use]
    pub const fn requires_auth(self) -> bool {
        matches!(
            self,
            Self::OnboardingLocation
                | Self::PinDrop
                | Self::OnboardingRadius
                | Self::CameraCapture
                | Self::Ready
        )
    }

    #[must_use]
    pub const fn is_onboarding(self) -> bool {
        matches!(
            self,
            Self::OnboardingLocation | Self::PinDrop | Self::OnboardingRadius
        )
    }

    #[must_use]
    pub const fn can_capture_photo(self) -> bool {
        matches!(self, Self::Ready | Self::CameraCapture)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalCaseStatus {
    PendingUpload,
    Uploading,
    UploadingPhoto,
    Synced,
    Failed,
    PermanentlyFailed,
}

impl LocalCaseStatus {
    #[must_use]
    pub const fn is_pending(self) -> bool {
        matches!(self, Self::PendingUpload | Self::Uploading | Self::UploadingPhoto)
    }

    #[must_use]
    pub const fn is_synced(self) -> bool {
        matches!(self, Self::Synced)
    }

    #[must_use]
    pub const fn is_failed(self) -> bool {
        matches!(self, Self::Failed | Self::PermanentlyFailed)
    }

    #[must_use]
    pub const fn can_retry(self) -> bool {
        matches!(self, Self::Failed)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalCase {
    pub local_id: LocalOpId,
    pub location: LatLon,
    pub description: Option<String>,
    pub landmark_hint: Option<String>,
    pub wound_severity: Option<u8>,
    pub status: LocalCaseStatus,
    pub created_at_ms_utc: UnixTimeMs,
    pub updated_at_ms_utc: UnixTimeMs,
    pub photo_data: Option<Vec<u8>>,
    pub photo_upload_url: Option<String>,
    pub server_id: Option<CaseId>,
    pub sync_error: Option<String>,
    pub retry_count: u32,
}

impl LocalCase {
    #[must_use]
    pub fn new(location: LatLon, description: Option<String>, wound_severity: Option<u8>) -> Self {
        let now = UnixTimeMs::now();
        Self {
            local_id: LocalOpId::generate(),
            location,
            description,
            landmark_hint: None,
            wound_severity,
            status: LocalCaseStatus::PendingUpload,
            created_at_ms_utc: now,
            updated_at_ms_utc: now,
            photo_data: None,
            photo_upload_url: None,
            server_id: None,
            sync_error: None,
            retry_count: 0,
        }
    }

    pub fn mark_synced(&mut self, server_id: CaseId) {
        self.server_id = Some(server_id);
        self.status = LocalCaseStatus::Synced;
        self.updated_at_ms_utc = UnixTimeMs::now();
        self.sync_error = None;
        self.photo_data = None;
    }

    pub fn mark_failed(&mut self, error: impl Into<String>) {
        self.status = if self.retry_count >= MAX_RETRY_ATTEMPTS {
            LocalCaseStatus::PermanentlyFailed
        } else {
            LocalCaseStatus::Failed
        };
        self.sync_error = Some(error.into());
        self.updated_at_ms_utc = UnixTimeMs::now();
        self.retry_count += 1;
    }

    pub fn mark_uploading(&mut self) {
        self.status = LocalCaseStatus::Uploading;
        self.updated_at_ms_utc = UnixTimeMs::now();
    }

    pub fn mark_uploading_photo(&mut self) {
        self.status = LocalCaseStatus::UploadingPhoto;
        self.updated_at_ms_utc = UnixTimeMs::now();
    }

    #[must_use]
    pub fn description_preview(&self, max_len: usize) -> String {
        self.description
            .as_ref()
            .map(|d| {
                if d.len() <= max_len {
                    d.clone()
                } else {
                    let mut preview: String = d.chars().take(max_len.saturating_sub(3)).collect();
                    preview.push_str("...");
                    preview
                }
            })
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCase {
    pub id: CaseId,
    pub location: LatLon,
    pub description: Option<String>,
    pub landmark_hint: Option<String>,
    pub wound_severity: Option<u8>,
    pub status: CaseStatus,
    pub created_at_ms_utc: UnixTimeMs,
    pub updated_at_ms_utc: UnixTimeMs,
    pub reporter_id: UserId,
    pub assigned_rescuer_id: Option<UserId>,
    pub photo_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub gemini_diagnosis: Option<String>,
    pub species_guess: Option<String>,
    pub distance_meters: Option<f64>,
}

impl ServerCase {
    #[must_use]
    pub fn is_owned_by(&self, user_id: &UserId) -> bool {
        self.assigned_rescuer_id.as_ref() == Some(user_id)
    }

    #[must_use]
    pub fn is_reported_by(&self, user_id: &UserId) -> bool {
        &self.reporter_id == user_id
    }

    #[must_use]
    pub fn description_preview(&self, max_len: usize) -> String {
        self.description
            .as_ref()
            .map(|d| {
                if d.len() <= max_len {
                    d.clone()
                } else {
                    let mut preview: String = d.chars().take(max_len.saturating_sub(3)).collect();
                    preview.push_str("...");
                    preview
                }
            })
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCaseRequest {
    pub location: LatLon,
    pub description: Option<String>,
    pub landmark_hint: Option<String>,
    pub wound_severity: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub photo_mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCaseResponse {
    pub id: String,
    pub created_at: String,
    #[serde(default)]
    pub photo_upload_url: Option<String>,
    #[serde(default)]
    pub photo_upload_headers: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimCaseResponse {
    pub success: bool,
    pub case: Option<ServerCase>,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionCaseRequest {
    pub next_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionCaseResponse {
    pub success: bool,
    pub case: Option<ServerCase>,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListCasesResponse {
    pub cases: Vec<ServerCase>,
    #[serde(default)]
    pub next_cursor: Option<String>,
    #[serde(default)]
    pub total_count: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct PendingClaim {
    pub case_id: CaseId,
    pub idempotency_key: IdempotencyKey,
    pub original_status: CaseStatus,
    pub original_assignee: Option<UserId>,
    pub mutation_id: String,
    pub created_at_ms: u64,
    pub attempt_count: u32,
}

impl PendingClaim {
    #[must_use]
    pub fn new(
        case_id: CaseId,
        original_status: CaseStatus,
        original_assignee: Option<UserId>,
    ) -> Self {
        Self {
            case_id,
            idempotency_key: IdempotencyKey::generate(),
            original_status,
            original_assignee,
            mutation_id: Uuid::new_v4().to_string(),
            created_at_ms: get_current_time_ms(),
            attempt_count: 1,
        }
    }

    pub fn increment_attempt(&mut self) {
        self.attempt_count += 1;
    }
}

#[derive(Debug, Clone)]
pub struct OptimisticMutation {
    pub mutation_id: String,
    pub case_id: CaseId,
    pub original_status: CaseStatus,
    pub original_assignee: Option<UserId>,
    pub new_status: CaseStatus,
    pub created_at_ms: u64,
}

impl OptimisticMutation {
    #[must_use]
    pub fn new(
        case_id: CaseId,
        original_status: CaseStatus,
        original_assignee: Option<UserId>,
        new_status: CaseStatus,
    ) -> Self {
        Self {
            mutation_id: Uuid::new_v4().to_string(),
            case_id,
            original_status,
            original_assignee,
            new_status,
            created_at_ms: get_current_time_ms(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutboxIntent {
    CreateCase {
        local_id: LocalOpId,
        location: LatLon,
        description: Option<String>,
        landmark_hint: Option<String>,
        wound_severity: Option<u8>,
        has_photo: bool,
        created_at_ms_utc: UnixTimeMs,
    },
    UploadPhoto {
        local_id: LocalOpId,
        upload_url: String,
        upload_headers: HashMap<String, String>,
    },
    ClaimCase {
        case_id: CaseId,
    },
    TransitionCase {
        case_id: CaseId,
        next_status: CaseStatus,
        notes: Option<String>,
    },
    SyncFcmToken {
        token: String,
    },
}

impl OutboxIntent {
    #[must_use]
    pub const fn intent_type(&self) -> &'static str {
        match self {
            Self::CreateCase { .. } => "create_case",
            Self::UploadPhoto { .. } => "upload_photo",
            Self::ClaimCase { .. } => "claim_case",
            Self::TransitionCase { .. } => "transition_case",
            Self::SyncFcmToken { .. } => "sync_fcm_token",
        }
    }

    #[must_use]
    pub const fn default_timeout(&self) -> Duration {
        match self {
            Self::CreateCase { .. } => CREATE_CASE_TIMEOUT,
            Self::UploadPhoto { .. } => UPLOAD_TIMEOUT,
            Self::ClaimCase { .. } => CLAIM_TIMEOUT,
            Self::TransitionCase { .. } => TRANSITION_TIMEOUT,
            Self::SyncFcmToken { .. } => FCM_SYNC_TIMEOUT,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxEntryError {
    pub code: String,
    pub message: Option<String>,
    pub http_status: Option<u16>,
    pub is_permanent: bool,
}

impl OutboxEntryError {
    #[must_use]
    pub fn new(code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: None,
            http_status: None,
            is_permanent: false,
        }
    }

    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    #[must_use]
    pub fn with_http_status(mut self, status: u16) -> Self {
        self.http_status = Some(status);
        self.is_permanent = matches!(status, 400..=499) && status != 429 && status != 408;
        self
    }

    #[must_use]
    pub fn permanent(mut self) -> Self {
        self.is_permanent = true;
        self
    }

    #[must_use]
    pub fn network_error(message: impl Into<String>) -> Self {
        Self::new("NETWORK_ERROR").with_message(message)
    }

    #[must_use]
    pub fn timeout_error() -> Self {
        Self::new("TIMEOUT")
    }

    #[must_use]
    pub fn server_error(status: u16, message: Option<String>) -> Self {
        let mut error = Self::new(format!("HTTP_{status}")).with_http_status(status);
        if let Some(msg) = message {
            error = error.with_message(msg);
        }
        error
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RetryState {
    #[default]
    Pending,
    InFlight,
    Completed,
    Failed,
    PermanentlyFailed,
    RateLimited,
}

impl RetryState {
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::PermanentlyFailed)
    }

    #[must_use]
    pub const fn can_retry(self) -> bool {
        matches!(self, Self::Pending | Self::Failed | Self::RateLimited)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxEntry {
    pub op_id: OpId,
    pub idempotency_key: IdempotencyKey,
    pub intent: OutboxIntent,
    pub created_at: UnixTimeMs,
    pub updated_at: UnixTimeMs,
    pub retry_state: RetryState,
    pub attempt_count: u32,
    pub last_attempt_at: Option<UnixTimeMs>,
    pub next_retry_at: Option<UnixTimeMs>,
    pub last_error: Option<OutboxEntryError>,
}

impl OutboxEntry {
    #[must_use]
    pub fn new(intent: OutboxIntent) -> Self {
        let now = UnixTimeMs::now();
        Self {
            op_id: OpId::generate(),
            idempotency_key: IdempotencyKey::generate(),
            intent,
            created_at: now,
            updated_at: now,
            retry_state: RetryState::Pending,
            attempt_count: 0,
            last_attempt_at: None,
            next_retry_at: None,
            last_error: None,
        }
    }

    #[must_use]
    pub fn with_idempotency_key(mut self, key: IdempotencyKey) -> Self {
        self.idempotency_key = key;
        self
    }

    #[must_use]
    pub fn is_ready_for_retry(&self, now_ms: u64) -> bool {
        match self.retry_state {
            RetryState::Pending => true,
            RetryState::Failed | RetryState::RateLimited => {
                self.next_retry_at.map_or(true, |t| now_ms >= t.0)
            }
            _ => false,
        }
    }

    #[must_use]
    pub const fn is_completed(&self) -> bool {
        matches!(self.retry_state, RetryState::Completed)
    }

    #[must_use]
    pub const fn is_permanently_failed(&self) -> bool {
        matches!(self.retry_state, RetryState::PermanentlyFailed)
    }

    #[must_use]
    pub const fn is_in_flight(&self) -> bool {
        matches!(self.retry_state, RetryState::InFlight)
    }

    pub fn mark_in_flight(&mut self) {
        let now = UnixTimeMs::now();
        self.retry_state = RetryState::InFlight;
        self.last_attempt_at = Some(now);
        self.updated_at = now;
        self.attempt_count += 1;
    }

    pub fn mark_completed(&mut self) {
        self.retry_state = RetryState::Completed;
        self.updated_at = UnixTimeMs::now();
        self.last_error = None;
        self.next_retry_at = None;
    }

    pub fn mark_failed(&mut self, error: OutboxEntryError) {
        let now = UnixTimeMs::now();
        self.updated_at = now;
        
        if error.is_permanent || self.attempt_count >= MAX_RETRY_ATTEMPTS {
            self.retry_state = RetryState::PermanentlyFailed;
        } else {
            self.retry_state = RetryState::Failed;
            let jitter = generate_jitter();
            let delay = calculate_retry_delay(self.attempt_count, jitter);
            self.next_retry_at = Some(now.add_millis(delay));
        }
        
        self.last_error = Some(error);
    }

    pub fn mark_rate_limited(&mut self, retry_after_ms: u64) {
        let now = UnixTimeMs::now();
        self.retry_state = RetryState::RateLimited;
        self.updated_at = now;
        self.next_retry_at = Some(now.add_millis(retry_after_ms));
        self.last_error = Some(OutboxEntryError::new("RATE_LIMITED"));
    }

    pub fn mark_permanently_failed(&mut self, error: OutboxEntryError) {
        self.retry_state = RetryState::PermanentlyFailed;
        self.updated_at = UnixTimeMs::now();
        self.last_error = Some(error.permanent());
        self.next_retry_at = None;
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfflineStore {
    pub pending_local_cases: Vec<LocalCase>,
    pub outbox: Vec<OutboxEntry>,
    pub last_sync_ms: Option<u64>,
    pub last_cases_refresh_ms: Option<u64>,
    pub schema_version: u32,
}

impl OfflineStore {
    pub const CURRENT_SCHEMA_VERSION: u32 = 1;

    #[must_use]
    pub fn new() -> Self {
        Self {
            pending_local_cases: Vec::new(),
            outbox: Vec::new(),
            last_sync_ms: None,
            last_cases_refresh_ms: None,
            schema_version: Self::CURRENT_SCHEMA_VERSION,
        }
    }

    pub fn push_local_case(&mut self, case: LocalCase) -> Result<(), OutboxError> {
        if self.pending_local_cases.len() >= MAX_PENDING_LOCAL_CASES {
            self.evict_synced_cases(1);
            if self.pending_local_cases.len() >= MAX_PENDING_LOCAL_CASES {
                return Err(OutboxError::Full {
                    max: MAX_PENDING_LOCAL_CASES,
                });
            }
        }

        if self.pending_local_cases.iter().any(|c| c.local_id == case.local_id) {
            return Err(OutboxError::DuplicateOpId(case.local_id.0.clone()));
        }

        self.pending_local_cases.push(case);
        Ok(())
    }

    pub fn push_outbox(&mut self, entry: OutboxEntry) -> Result<(), OutboxError> {
        if self.outbox.len() >= MAX_OUTBOX_ENTRIES {
            self.cleanup_completed_outbox();
            if self.outbox.len() >= MAX_OUTBOX_ENTRIES {
                return Err(OutboxError::Full {
                    max: MAX_OUTBOX_ENTRIES,
                });
            }
        }

        if self.outbox.iter().any(|e| e.op_id == entry.op_id) {
            return Err(OutboxError::DuplicateOpId(entry.op_id.0.clone()));
        }

        self.outbox.push(entry);
        Ok(())
    }

    #[must_use]
    pub fn get_next_pending_entry(&self, now_ms: u64) -> Option<&OutboxEntry> {
        self.outbox
            .iter()
            .filter(|e| !e.is_completed() && !e.is_permanently_failed() && !e.is_in_flight())
            .find(|e| e.is_ready_for_retry(now_ms))
    }

    #[must_use]
    pub fn get_entry_mut(&mut self, op_id: &OpId) -> Option<&mut OutboxEntry> {
        self.outbox.iter_mut().find(|e| &e.op_id == op_id)
    }

    #[must_use]
    pub fn get_local_case_mut(&mut self, local_id: &LocalOpId) -> Option<&mut LocalCase> {
        self.pending_local_cases.iter_mut().find(|c| &c.local_id == local_id)
    }

    pub fn mark_entry_completed(&mut self, op_id: &OpId) {
        if let Some(entry) = self.get_entry_mut(op_id) {
            entry.mark_completed();
        }
    }

    pub fn mark_entry_failed(&mut self, op_id: &OpId, error: OutboxEntryError) {
        if let Some(entry) = self.get_entry_mut(op_id) {
            entry.mark_failed(error);
        }
    }

    pub fn mark_entry_permanently_failed(&mut self, op_id: &OpId, error: OutboxEntryError) {
        if let Some(entry) = self.get_entry_mut(op_id) {
            entry.mark_permanently_failed(error);
        }
    }

    #[must_use]
    pub fn pending_sync_count(&self) -> usize {
        let outbox_pending = self
            .outbox
            .iter()
            .filter(|e| !e.is_completed() && !e.is_permanently_failed())
            .count();
        
        let cases_pending = self
            .pending_local_cases
            .iter()
            .filter(|c| c.status.is_pending())
            .count();
        
        outbox_pending + cases_pending
    }

    #[must_use]
    pub fn failed_count(&self) -> usize {
        self.pending_local_cases
            .iter()
            .filter(|c| c.status.is_failed())
            .count()
    }

    pub fn evict_synced_cases(&mut self, count: usize) {
        let mut to_remove = Vec::new();
        let mut removed = 0;

        for (i, case) in self.pending_local_cases.iter().enumerate() {
            if removed >= count {
                break;
            }
            if case.status.is_synced() {
                to_remove.push(i);
                removed += 1;
            }
        }

        for i in to_remove.into_iter().rev() {
            self.pending_local_cases.remove(i);
        }
    }

    pub fn cleanup_completed_outbox(&mut self) {
        self.outbox.retain(|e| !e.is_completed());
    }

    pub fn cleanup_permanently_failed(&mut self) {
        self.outbox.retain(|e| !e.is_permanently_failed());
    }

    pub fn update_last_sync(&mut self) {
        self.last_sync_ms = Some(get_current_time_ms());
    }

    pub fn update_last_refresh(&mut self) {
        self.last_cases_refresh_ms = Some(get_current_time_ms());
    }
}

pub struct Model {
    pub state: AppState,
    pub user_id: Option<UserId>,
    pub jwt_token: Option<String>,
    pub area_center: Option<ValidatedCoordinate>,
    pub area_radius_m: u32,
    pub map_center: Option<ValidatedCoordinate>,
    pub map_zoom: f64,
    pub feed_view: FeedView,
    pub cases: Vec<ServerCase>,
    pub cases_cursor: Option<String>,
    pub selected_case_id: Option<CaseId>,
    pub offline_store: OfflineStore,
    pub network_online: bool,
    pub is_refreshing: bool,
    pub is_loading: bool,
    pub push_permission_granted: bool,
    pub push_token: Option<String>,
    pub staged_photo: Option<StagedPhoto>,
    pub yolo_detector: Option<crate::vision::YoloDetector>,
    pub active_error: Option<AppError>,
    pub active_toast: Option<ToastMessage>,
    pub pending_claims: HashMap<CaseId, PendingClaim>,
    pub pending_mutations: HashMap<String, OptimisticMutation>,
    pub view_timestamp_ms: u64,
    pub location_permission_state: PermissionState,
    pub camera_permission_state: PermissionState,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            state: AppState::Loading,
            user_id: None,
            jwt_token: None,
            area_center: None,
            area_radius_m: DEFAULT_RADIUS_M,
            map_center: None,
            map_zoom: DEFAULT_MAP_ZOOM,
            feed_view: FeedView::default(),
            cases: Vec::new(),
            cases_cursor: None,
            selected_case_id: None,
            offline_store: OfflineStore::new(),
            network_online: true,
            is_refreshing: false,
            is_loading: false,
            push_permission_granted: false,
            push_token: None,
            staged_photo: None,
            yolo_detector: None,
            active_error: None,
            active_toast: None,
            pending_claims: HashMap::new(),
            pending_mutations: HashMap::new(),
            view_timestamp_ms: get_current_time_ms(),
            location_permission_state: PermissionState::Unknown,
            camera_permission_state: PermissionState::Unknown,
        }
    }
}

impl Model {
    pub fn update_timestamp(&mut self) {
        self.view_timestamp_ms = get_current_time_ms();
    }

    pub fn set_error(&mut self, error: AppError) {
        self.active_error = Some(error);
    }

    pub fn clear_error(&mut self) {
        self.active_error = None;
    }

    pub fn show_toast(&mut self, message: impl Into<String>, kind: ToastKind) {
        self.active_toast = Some(ToastMessage::new(message, kind));
    }

    pub fn clear_toast(&mut self) {
        self.active_toast = None;
    }

    #[must_use]
    pub fn is_authenticated(&self) -> bool {
        self.user_id.is_some()
    }

    #[must_use]
    pub fn can_claim_case(&self, case: &ServerCase) -> bool {
        case.status.is_claimable()
            && !self.pending_claims.contains_key(&case.id)
            && case.assigned_rescuer_id.is_none()
    }

    pub fn store_optimistic_mutation(
        &mut self,
        case_id: CaseId,
        original_status: CaseStatus,
        original_assignee: Option<UserId>,
        new_status: CaseStatus,
    ) -> String {
        let mutation = OptimisticMutation::new(case_id, original_status, original_assignee, new_status);
        let mutation_id = mutation.mutation_id.clone();
        self.pending_mutations.insert(mutation_id.clone(), mutation);
        mutation_id
    }

    pub fn rollback_mutation(&mut self, mutation_id: &str) -> bool {
        if let Some(mutation) = self.pending_mutations.remove(mutation_id) {
            if let Some(case) = self.cases.iter_mut().find(|c| c.id == mutation.case_id) {
                case.status = mutation.original_status;
                case.assigned_rescuer_id = mutation.original_assignee;
                return true;
            }
        }
        false
    }

    pub fn commit_mutation(&mut self, mutation_id: &str) {
        self.pending_mutations.remove(mutation_id);
    }

    pub fn enforce_collection_limits(&mut self) {
        while self.offline_store.pending_local_cases.len() > MAX_PENDING_LOCAL_CASES {
            self.offline_store.evict_synced_cases(1);
            if self.offline_store.pending_local_cases.len() > MAX_PENDING_LOCAL_CASES {
                self.offline_store.pending_local_cases.remove(0);
            }
        }

        if self.cases.len() > MAX_CACHED_SERVER_CASES {
            self.cases.sort_by(|a, b| b.created_at_ms_utc.0.cmp(&a.created_at_ms_utc.0));
            self.cases.truncate(MAX_CACHED_SERVER_CASES);
        }

        self.offline_store.cleanup_completed_outbox();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StagedPhoto {
    pub original_data: Vec<u8>,
    pub processed_data: Vec<u8>,
    pub cropped_data: Option<Vec<u8>>,
    pub width: u32,
    pub height: u32,
    pub mime_type: String,
    pub detection_count: usize,
    pub top_confidence: f32,
    pub detections: Vec<crate::vision::Detection>,
}

impl StagedPhoto {
    #[must_use]
    pub fn has_detections(&self) -> bool {
        self.detection_count > 0
    }

    #[must_use]
    pub fn best_data_for_upload(&self) -> &[u8] {
        self.cropped_data.as_ref().unwrap_or(&self.processed_data)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PermissionState {
    #[default]
    Unknown,
    Requesting,
    Granted,
    Denied,
    Restricted,
}

impl PermissionState {
    #[must_use]
    pub const fn is_granted(self) -> bool {
        matches!(self, Self::Granted)
    }

    #[must_use]
    pub const fn is_denied(self) -> bool {
        matches!(self, Self::Denied | Self::Restricted)
    }

    #[must_use]
    pub const fn is_unknown(self) -> bool {
        matches!(self, Self::Unknown)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToastMessage {
    pub message: String,
    pub kind: ToastKind,
    pub created_at_ms: u64,
    pub duration_ms: u64,
}

impl ToastMessage {
    #[must_use]
    pub fn new(message: impl Into<String>, kind: ToastKind) -> Self {
        Self {
            message: message.into(),
            kind,
            created_at_ms: get_current_time_ms(),
            duration_ms: kind.default_duration_ms(),
        }
    }

    #[must_use]
    pub fn is_expired(&self, now_ms: u64) -> bool {
        now_ms.saturating_sub(self.created_at_ms) > self.duration_ms
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ToastKind {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

impl ToastKind {
    #[must_use]
    pub const fn default_duration_ms(self) -> u64 {
        match self {
            Self::Info => 3000,
            Self::Success => 2000,
            Self::Warning => 4000,
            Self::Error => 5000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCasePayload {
    pub location: (f64, f64),
    pub description: Option<String>,
    pub landmark_hint: Option<String>,
    pub wound_severity: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PushPayload {
    NewCase {
        case_id: String,
        lat: f64,
        lng: f64,
        #[serde(default)]
        severity: Option<u8>,
    },
    CaseClaimed {
        case_id: String,
        claimed_by: String,
    },
    CaseUpdated {
        case_id: String,
        new_status: String,
        #[serde(default)]
        updated_by: Option<String>,
    },
    CaseResolved {
        case_id: String,
    },
    CaseCancelled {
        case_id: String,
        #[serde(default)]
        reason: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapCenter {
    pub lat: f64,
    pub lng: f64,
}

impl MapCenter {
    #[must_use]
    pub const fn new(lat: f64, lng: f64) -> Self {
        Self { lat, lng }
    }

    #[must_use]
    pub const fn lat(&self) -> f64 {
        self.lat
    }

    #[must_use]
    pub const fn lng(&self) -> f64 {
        self.lng
    }

    pub fn to_validated(&self) -> Result<ValidatedCoordinate, CoordinateError> {
        ValidatedCoordinate::new(self.lat, self.lng)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ZoomLevel {
    value: f64,
}

impl ZoomLevel {
    #[must_use]
    pub fn new(value: f64) -> Self {
        Self {
            value: value.clamp(MIN_ZOOM, MAX_ZOOM),
        }
    }

    #[must_use]
    pub const fn value(self) -> f64 {
        self.value
    }
}

impl Default for ZoomLevel {
    fn default() -> Self {
        Self::new(DEFAULT_MAP_ZOOM)
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Noop,

    AppStarted,
    AppBackgrounded,
    AppForegrounded,

    LoginRequested,
    LoginCompleted {
        jwt: String,
        user_id: String,
    },
    LoginFailed {
        error: String,
    },
    LogoutRequested,
    LogoutCompleted,
    TokenRefreshRequired,
    TokenRefreshed {
        jwt: String,
    },
    TokenRefreshFailed {
        error: String,
    },

    LocationPermissionRequested,
    LocationPermissionResult {
        granted: bool,
    },
    LocationReceived {
        lat: f64,
        lng: f64,
        accuracy: Option<f64>,
    },
    LocationFailed {
        error: String,
    },
    LocationPinDropped {
        lat: f64,
        lng: f64,
    },

    RadiusSelected {
        meters: u32,
    },
    OnboardingComplete,

    NetworkStatusChanged {
        online: bool,
    },

    CameraPermissionRequested,
    CameraPermissionResult {
        granted: bool,
    },
    CapturePhotoRequested,
    CameraResult(Box<Result<crate::capabilities::CameraOutput, crate::capabilities::CameraError>>),
    ClearStagedPhoto,
    PhotoProcessed {
        staged_photo: StagedPhoto,
    },
    PhotoProcessingFailed {
        error: String,
    },

    CreateCaseRequested(CreateCasePayload),
    CreateCaseResponse {
        op_id: String,
        result: Box<Result<crate::capabilities::HttpOutput, crate::capabilities::HttpError>>,
    },
    PhotoUploadResponse {
        local_id: String,
        result: Box<Result<crate::capabilities::HttpOutput, crate::capabilities::HttpError>>,
    },

    WriteEncryptedStore {
        key_id: String,
        data: Vec<u8>,
    },
    PersistenceSucceeded,
    PersistenceFailed {
        error: String,
    },
    RestoreStateRequested,
        RestoreStateResponse {
        result: Box<Result<Vec<u8>, crate::capabilities::KvError>>,
    },
    StateDecrypted {
        data: Vec<u8>,
    },
    StateDecryptionFailed {
        error: String,
    },

    OutboxFlushRequested,
    OutboxEntryCompleted {
        op_id: String,
    },
    OutboxEntryFailed {
        op_id: String,
        error: String,
        is_permanent: bool,
    },

    SwitchToMap,
    SwitchToList,
    ToggleFeedView,
    MapMoved {
        center: MapCenter,
        zoom: ZoomLevel,
    },

    CaseSelected {
        case_id: String,
    },
    CaseDeselected,

    ClaimRequested {
        case_id: String,
    },
    ClaimResponse {
        case_id: String,
        mutation_id: String,
        result: Box<Result<crate::capabilities::HttpOutput, crate::capabilities::HttpError>>,
    },

    TransitionRequested {
        case_id: String,
        next_status: String,
        notes: Option<String>,
    },
    TransitionResponse {
        case_id: String,
        mutation_id: String,
        result: Box<Result<crate::capabilities::HttpOutput, crate::capabilities::HttpError>>,
    },

    RefreshRequested,
    RefreshResponse(Box<Result<crate::capabilities::HttpOutput, crate::capabilities::HttpError>>),
    LoadMoreCases,
    LoadMoreResponse(Box<Result<crate::capabilities::HttpOutput, crate::capabilities::HttpError>>),

    PushPermissionRequested,
    PushPermissionResult {
        granted: bool,
    },
    PushTokenReceived {
        token: String,
    },
    PushTokenFailed {
        error: String,
    },
    PushReceived(PushPayload),
    FcmSyncResponse {
        result: Box<Result<crate::capabilities::HttpOutput, crate::capabilities::HttpError>>,
    },

    DismissError,
    DismissToast,
    ShowToast {
        message: String,
        kind: ToastKind,
    },

    TimerTick,
    RetryFailedOperations,
}

impl Event {
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Noop => "noop",
            Self::AppStarted => "app_started",
            Self::AppBackgrounded => "app_backgrounded",
            Self::AppForegrounded => "app_foregrounded",
            Self::LoginRequested => "login_requested",
            Self::LoginCompleted { .. } => "login_completed",
            Self::LoginFailed { .. } => "login_failed",
            Self::LogoutRequested => "logout_requested",
            Self::LogoutCompleted => "logout_completed",
            Self::TokenRefreshRequired => "token_refresh_required",
            Self::TokenRefreshed { .. } => "token_refreshed",
            Self::TokenRefreshFailed { .. } => "token_refresh_failed",
            Self::LocationPermissionRequested => "location_permission_requested",
            Self::LocationPermissionResult { .. } => "location_permission_result",
            Self::LocationReceived { .. } => "location_received",
            Self::LocationFailed { .. } => "location_failed",
            Self::LocationPinDropped { .. } => "location_pin_dropped",
            Self::RadiusSelected { .. } => "radius_selected",
            Self::OnboardingComplete => "onboarding_complete",
            Self::NetworkStatusChanged { .. } => "network_status_changed",
            Self::CameraPermissionRequested => "camera_permission_requested",
            Self::CameraPermissionResult { .. } => "camera_permission_result",
            Self::CapturePhotoRequested => "capture_photo_requested",
            Self::CameraResult(_) => "camera_result",
            Self::ClearStagedPhoto => "clear_staged_photo",
            Self::PhotoProcessed { .. } => "photo_processed",
            Self::PhotoProcessingFailed { .. } => "photo_processing_failed",
            Self::CreateCaseRequested(_) => "create_case_requested",
            Self::CreateCaseResponse { .. } => "create_case_response",
            Self::PhotoUploadResponse { .. } => "photo_upload_response",
            Self::WriteEncryptedStore { .. } => "write_encrypted_store",
            Self::PersistenceSucceeded => "persistence_succeeded",
            Self::PersistenceFailed { .. } => "persistence_failed",
            Self::RestoreStateRequested => "restore_state_requested",
            Self::RestoreStateResponse { .. } => "restore_state_response",
            Self::StateDecrypted { .. } => "state_decrypted",
            Self::StateDecryptionFailed { .. } => "state_decryption_failed",
            Self::OutboxFlushRequested => "outbox_flush_requested",
            Self::OutboxEntryCompleted { .. } => "outbox_entry_completed",
            Self::OutboxEntryFailed { .. } => "outbox_entry_failed",
            Self::SwitchToMap => "switch_to_map",
            Self::SwitchToList => "switch_to_list",
            Self::ToggleFeedView => "toggle_feed_view",
            Self::MapMoved { .. } => "map_moved",
            Self::CaseSelected { .. } => "case_selected",
            Self::CaseDeselected => "case_deselected",
            Self::ClaimRequested { .. } => "claim_requested",
            Self::ClaimResponse { .. } => "claim_response",
            Self::TransitionRequested { .. } => "transition_requested",
            Self::TransitionResponse { .. } => "transition_response",
            Self::RefreshRequested => "refresh_requested",
            Self::RefreshResponse(_) => "refresh_response",
            Self::LoadMoreCases => "load_more_cases",
            Self::LoadMoreResponse(_) => "load_more_response",
            Self::PushPermissionRequested => "push_permission_requested",
            Self::PushPermissionResult { .. } => "push_permission_result",
            Self::PushTokenReceived { .. } => "push_token_received",
            Self::PushTokenFailed { .. } => "push_token_failed",
            Self::PushReceived(_) => "push_received",
            Self::FcmSyncResponse { .. } => "fcm_sync_response",
            Self::DismissError => "dismiss_error",
            Self::DismissToast => "dismiss_toast",
            Self::ShowToast { .. } => "show_toast",
            Self::TimerTick => "timer_tick",
            Self::RetryFailedOperations => "retry_failed_operations",
        }
    }

    #[must_use]
    pub const fn is_user_initiated(&self) -> bool {
        matches!(
            self,
            Self::LoginRequested
                | Self::LogoutRequested
                | Self::LocationPermissionRequested
                | Self::LocationPinDropped { .. }
                | Self::RadiusSelected { .. }
                | Self::CapturePhotoRequested
                | Self::ClearStagedPhoto
                | Self::CreateCaseRequested(_)
                | Self::SwitchToMap
                | Self::SwitchToList
                | Self::ToggleFeedView
                | Self::CaseSelected { .. }
                | Self::CaseDeselected
                | Self::ClaimRequested { .. }
                | Self::TransitionRequested { .. }
                | Self::RefreshRequested
                | Self::LoadMoreCases
                | Self::DismissError
                | Self::DismissToast
        )
    }
}

impl Default for Event {
    fn default() -> Self {
        Self::Noop
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CaptureConfig {
    pub aspect_ratio: String,
    pub max_dimension: u32,
    pub quality: u8,
    pub format: String,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            aspect_ratio: "4:3".into(),
            max_dimension: MAX_IMAGE_DIMENSION,
            quality: 85,
            format: "jpeg".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CasePin {
    pub id: String,
    pub lat: f64,
    pub lon: f64,
    pub status: CaseStatus,
    pub is_mine: bool,
    pub is_local: bool,
    pub wound_severity: Option<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CaseListItem {
    pub id: String,
    pub description_preview: String,
    pub status: CaseStatus,
    pub distance_meters: f64,
    pub distance_text: String,
    pub time_ago: String,
    pub created_at_ms: u64,
    pub wound_severity: Option<u8>,
    pub is_mine: bool,
    pub is_local: bool,
    pub has_photo: bool,
    pub sync_status: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClaimState {
    Available,
    Claiming,
    ClaimedByMe,
    ClaimedByOther,
    NotClaimable,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CaseDetail {
    pub id: String,
    pub description: Option<String>,
    pub landmark_hint: Option<String>,
    pub status: CaseStatus,
    pub wound_severity: Option<u8>,
    pub species_guess: Option<String>,
    pub lat: f64,
    pub lon: f64,
    pub distance_text: String,
    pub time_ago: String,
    pub created_at_ms: u64,
    pub can_claim: bool,
    pub claim_state: ClaimState,
    pub available_transitions: Vec<CaseStatus>,
    pub photo_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub gemini_diagnosis: Option<String>,
    pub reporter_is_me: bool,
    pub is_local: bool,
    pub sync_status: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct StagedPhotoView {
    pub has_photo: bool,
    pub detection_count: usize,
    pub top_confidence: f32,
    pub has_detections: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ViewState {
    Loading {
        message: Option<String>,
    },
    Unauthenticated,
    Authenticating,
    OnboardingLocation {
        permission_state: PermissionState,
    },
    PinDrop {
        initial_lat: Option<f64>,
        initial_lon: Option<f64>,
    },
    OnboardingRadius {
        lat: f64,
        lon: f64,
        radius: u32,
        selected_radius: u32,
    },
    CameraCapture {
        config: CaptureConfig,
    },
    Ready {
        feed_view: FeedView,
        pins: Vec<CasePin>,
        list_items: Vec<CaseListItem>,
        selected_detail: Option<CaseDetail>,
        map_center_lat: f64,
        map_center_lon: f64,
        map_zoom: f64,
        is_refreshing: bool,
        online: bool,
        pending_sync_count: usize,
        failed_sync_count: usize,
        staged_photo: Option<StagedPhotoView>,
        has_more_cases: bool,
    },
    Error {
        title: String,
        message: String,
        is_retryable: bool,
        retry_event: Option<String>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct UserFacingError {
    pub message: String,
    pub is_transient: bool,
    pub is_retryable: bool,
    pub error_code: String,
}

impl From<&AppError> for UserFacingError {
    fn from(e: &AppError) -> Self {
        Self {
            message: e.user_facing_message(),
            is_transient: e.severity == ErrorSeverity::Transient,
            is_retryable: e.is_retryable(),
            error_code: e.code().to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ToastView {
    pub message: String,
    pub kind: ToastKind,
    pub duration_ms: u64,
}

impl From<&ToastMessage> for ToastView {
    fn from(t: &ToastMessage) -> Self {
        Self {
            message: t.message.clone(),
            kind: t.kind,
            duration_ms: t.duration_ms,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ViewModel {
    pub state: ViewState,
    pub error: Option<UserFacingError>,
    pub toast: Option<ToastView>,
    pub is_global_loading: bool,
    pub offline_queue_count: usize,
    pub is_authenticated: bool,
    pub user_id: Option<String>,
}

pub mod app {
    use super::*;
    use crate::capabilities::{
        CameraError, CameraOutput, Capabilities, CryptoOutput, HttpError, HttpOutput, KvError,
    };

    #[derive(Default)]
    pub struct App;

    impl App {
        fn derive_store_key_id(user_id: &UserId) -> String {
            let hash = blake3::hash(user_id.0.as_bytes());
            format!("offline_store_v{}_{}", CURRENT_KEY_VERSION, &hash.to_hex()[..16])
        }

        fn persist_store(model: &Model, caps: &Capabilities) {
            let user_id = match &model.user_id {
                Some(id) => id.clone(),
                None => {
                    caps.telemetry().error("persist_no_user", "Cannot persist without user_id");
                    return;
                }
            };

            let key_id = Self::derive_store_key_id(&user_id);

            let serialized = match serde_cbor::to_vec(&model.offline_store) {
                Ok(bytes) => bytes,
                Err(e) => {
                    caps.telemetry().error("persist_serialize_failed", &e.to_string());
                    return;
                }
            };

            caps.telemetry().gauge("offline_store_bytes", serialized.len() as f64);

            let key_id_for_closure = key_id.clone();
            caps.crypto().encrypt(
                key_id,
                serialized,
                move |result| match result {
                    Ok(CryptoOutput::Encrypted(data)) => Event::WriteEncryptedStore {
                        key_id: key_id_for_closure,
                        data,
                    },
                    Ok(_) => Event::PersistenceFailed {
                        error: "Unexpected crypto output".into(),
                    },
                    Err(e) => Event::PersistenceFailed {
                        error: format!("{e:?}"),
                    },
                },
            );
        }

        fn validate_coordinates(lat: f64, lng: f64) -> Result<ValidatedCoordinate, AppError> {
            ValidatedCoordinate::new(lat, lng).map_err(|e| {
                AppError::new(ErrorKind::Validation, e.to_string())
                    .with_context("lat", lat.to_string())
                    .with_context("lng", lng.to_string())
            })
        }

        fn build_case_pins(model: &Model) -> Vec<CasePin> {
            let user_id = model.user_id.as_ref();
            let mut pins = Vec::with_capacity(
                model.offline_store.pending_local_cases.len() + model.cases.len(),
            );

            for case in &model.offline_store.pending_local_cases {
                pins.push(CasePin {
                    id: case.local_id.0.clone(),
                    lat: case.location.lat,
                    lon: case.location.lon,
                    status: CaseStatus::Pending,
                    is_mine: true,
                    is_local: true,
                    wound_severity: case.wound_severity,
                });
            }

            for case in &model.cases {
                let is_mine = user_id
                    .map(|uid| case.assigned_rescuer_id.as_ref() == Some(uid))
                    .unwrap_or(false);

                pins.push(CasePin {
                    id: case.id.0.clone(),
                    lat: case.location.lat,
                    lon: case.location.lon,
                    status: case.status,
                    is_mine,
                    is_local: false,
                    wound_severity: case.wound_severity,
                });
            }

            pins
        }

        fn build_list_items(model: &Model, now_ms: u64) -> Vec<CaseListItem> {
            let user_loc = match model.area_center {
                Some(loc) => loc,
                None => return Vec::new(),
            };

            let user_id = model.user_id.as_ref();
            let mut items = Vec::with_capacity(
                model.offline_store.pending_local_cases.len() + model.cases.len(),
            );

            for case in &model.offline_store.pending_local_cases {
                let distance = if let Ok(case_coord) =
                    ValidatedCoordinate::new(case.location.lat, case.location.lon)
                {
                    haversine_distance(user_loc, case_coord)
                } else {
                    f64::MAX
                };

                let sync_status = match case.status {
                    LocalCaseStatus::PendingUpload => Some("Pending sync".into()),
                    LocalCaseStatus::Uploading => Some("Syncing...".into()),
                    LocalCaseStatus::UploadingPhoto => Some("Uploading photo...".into()),
                    LocalCaseStatus::Failed => Some("Sync failed - tap to retry".into()),
                    LocalCaseStatus::PermanentlyFailed => Some("Sync failed".into()),
                    LocalCaseStatus::Synced => None,
                };

                items.push(CaseListItem {
                    id: case.local_id.0.clone(),
                    description_preview: case.description_preview(DESCRIPTION_PREVIEW_LENGTH),
                    status: CaseStatus::Pending,
                    distance_meters: distance,
                    distance_text: format_distance(distance),
                    time_ago: format_time_ago(case.created_at_ms_utc.0, now_ms),
                    created_at_ms: case.created_at_ms_utc.0,
                    wound_severity: case.wound_severity,
                    is_mine: true,
                    is_local: true,
                    has_photo: case.photo_data.is_some(),
                    sync_status,
                });
            }

            for case in &model.cases {
                let distance = case.distance_meters.unwrap_or_else(|| {
                    if let Ok(case_coord) =
                        ValidatedCoordinate::new(case.location.lat, case.location.lon)
                    {
                        haversine_distance(user_loc, case_coord)
                    } else {
                        f64::MAX
                    }
                });

                let is_mine = user_id
                    .map(|uid| case.assigned_rescuer_id.as_ref() == Some(uid))
                    .unwrap_or(false);

                items.push(CaseListItem {
                    id: case.id.0.clone(),
                    description_preview: case.description_preview(DESCRIPTION_PREVIEW_LENGTH),
                    status: case.status,
                    distance_meters: distance,
                    distance_text: format_distance(distance),
                    time_ago: format_time_ago(case.created_at_ms_utc.0, now_ms),
                    created_at_ms: case.created_at_ms_utc.0,
                    wound_severity: case.wound_severity,
                    is_mine,
                    is_local: false,
                    has_photo: case.photo_url.is_some(),
                    sync_status: None,
                });
            }

            items.sort_by(|a, b| {
                a.distance_meters
                    .partial_cmp(&b.distance_meters)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            items
        }

        fn build_case_detail(model: &Model, case_id: &str, now_ms: u64) -> Option<CaseDetail> {
            let user_loc = model.area_center?;
            let user_id = model.user_id.as_ref();

            if let Some(local_case) = model
                .offline_store
                .pending_local_cases
                .iter()
                .find(|c| c.local_id.0 == case_id)
            {
                let distance = ValidatedCoordinate::new(local_case.location.lat, local_case.location.lon)
                    .map(|coord| haversine_distance(user_loc, coord))
                    .unwrap_or(f64::MAX);

                let sync_status = match local_case.status {
                    LocalCaseStatus::PendingUpload => Some("Pending sync".into()),
                    LocalCaseStatus::Uploading => Some("Syncing...".into()),
                    LocalCaseStatus::UploadingPhoto => Some("Uploading photo...".into()),
                    LocalCaseStatus::Failed => Some("Sync failed - tap to retry".into()),
                    LocalCaseStatus::PermanentlyFailed => Some("Sync failed permanently".into()),
                    LocalCaseStatus::Synced => None,
                };

                return Some(CaseDetail {
                    id: local_case.local_id.0.clone(),
                    description: local_case.description.clone(),
                    landmark_hint: local_case.landmark_hint.clone(),
                    status: CaseStatus::Pending,
                    wound_severity: local_case.wound_severity,
                    species_guess: None,
                    lat: local_case.location.lat,
                    lon: local_case.location.lon,
                    distance_text: format_distance(distance),
                    time_ago: format_time_ago(local_case.created_at_ms_utc.0, now_ms),
                    created_at_ms: local_case.created_at_ms_utc.0,
                    can_claim: false,
                    claim_state: ClaimState::ClaimedByMe,
                    available_transitions: vec![],
                    photo_url: None,
                    thumbnail_url: None,
                    gemini_diagnosis: None,
                    reporter_is_me: true,
                    is_local: true,
                    sync_status,
                });
            }

            let case = model.cases.iter().find(|c| c.id.0 == case_id)?;

            let distance = case.distance_meters.unwrap_or_else(|| {
                ValidatedCoordinate::new(case.location.lat, case.location.lon)
                    .map(|coord| haversine_distance(user_loc, coord))
                    .unwrap_or(f64::MAX)
            });

            let is_reporter = user_id.map(|uid| &case.reporter_id == uid).unwrap_or(false);

            let claim_state = if !case.status.is_claimable() {
                ClaimState::NotClaimable
            } else if model.pending_claims.contains_key(&case.id) {
                ClaimState::Claiming
            } else {
                match &case.assigned_rescuer_id {
                    Some(assignee) if user_id == Some(assignee) => ClaimState::ClaimedByMe,
                    Some(_) => ClaimState::ClaimedByOther,
                    None => ClaimState::Available,
                }
            };

            let can_claim = claim_state == ClaimState::Available && model.is_authenticated();

            let available_transitions = if user_id
                .map(|uid| case.assigned_rescuer_id.as_ref() == Some(uid))
                .unwrap_or(false)
            {
                case.status.valid_transitions()
            } else {
                vec![]
            };

            Some(CaseDetail {
                id: case.id.0.clone(),
                description: case.description.clone(),
                landmark_hint: case.landmark_hint.clone(),
                status: case.status,
                wound_severity: case.wound_severity,
                species_guess: case.species_guess.clone(),
                lat: case.location.lat,
                lon: case.location.lon,
                distance_text: format_distance(distance),
                time_ago: format_time_ago(case.created_at_ms_utc.0, now_ms),
                created_at_ms: case.created_at_ms_utc.0,
                can_claim,
                claim_state,
                available_transitions,
                photo_url: case.photo_url.clone(),
                thumbnail_url: case.thumbnail_url.clone(),
                gemini_diagnosis: case.gemini_diagnosis.clone(),
                reporter_is_me: is_reporter,
                is_local: false,
                sync_status: None,
            })
        }

        fn process_camera_image(
            data: Vec<u8>,
            model: &mut Model,
            caps: &Capabilities,
        ) -> Result<StagedPhoto, AppError> {
            if data.len() > MAX_IMAGE_BYTES {
                return Err(AppError::new(
                    ErrorKind::ImageTooLarge,
                    format!(
                        "Image size {} MB exceeds maximum {} MB",
                        data.len() / 1_000_000,
                        MAX_IMAGE_BYTES / 1_000_000
                    ),
                ));
            }

            let format = image::guess_format(&data).map_err(|e| {
                AppError::new(ErrorKind::ImageFormatUnsupported, e.to_string())
            })?;

            let reader = image::io::Reader::with_format(std::io::Cursor::new(&data), format);

            let limits = image::io::Limits {
                max_image_width: Some(MAX_IMAGE_DIMENSION),
                max_image_height: Some(MAX_IMAGE_DIMENSION),
                max_alloc: Some(MAX_IMAGE_ALLOC),
            };

            let img = reader
                .with_limits(limits)
                .decode()
                .map_err(|e| AppError::new(ErrorKind::ImageProcessing, e.to_string()))?;

            let (width, height) = (img.width(), img.height());

            caps.telemetry().event(
                "image_decoded",
                &[
                    ("width", &width.to_string()),
                    ("height", &height.to_string()),
                    ("format", &format!("{format:?}")),
                ],
            );

            let processed_img = if width > MAX_PROCESSED_DIMENSION || height > MAX_PROCESSED_DIMENSION {
                img.resize(
                    MAX_PROCESSED_DIMENSION,
                    MAX_PROCESSED_DIMENSION,
                    image::imageops::FilterType::Lanczos3,
                )
            } else {
                img.clone()
            };

            let mut processed_data = Vec::new();
            processed_img
                .write_to(
                    &mut std::io::Cursor::new(&mut processed_data),
                    image::ImageFormat::WebP,
                )
                .map_err(|e| AppError::new(ErrorKind::ImageProcessing, e.to_string()))?;

            let (detections, cropped_data) = if let Some(detector) = &mut model.yolo_detector {
                let raw_pixels: Vec<u8> = img.to_rgb8().into_raw();
                let dets = detector.detect(&raw_pixels, width, height);

                let cropped = if !dets.is_empty() {
                    let merged = crate::image_processing::merge_bboxes(&dets);
                    let padded = crate::image_processing::pad_bbox(merged, 0.15, width, height);

                    let cropped_img = crate::image_processing::crop_image(&img, padded);

                    let mut cropped_bytes = Vec::new();
                    cropped_img
                        .write_to(
                            &mut std::io::Cursor::new(&mut cropped_bytes),
                            image::ImageFormat::WebP,
                        )
                        .ok();

                    if cropped_bytes.is_empty() {
                        None
                    } else {
                        Some(cropped_bytes)
                    }
                } else {
                    None
                };

                (dets, cropped)
            } else {
                (vec![], None)
            };

            let detection_count = detections.len();
            let top_confidence = detections
                .iter()
                .map(|d| d.confidence)
                .fold(0.0f32, f32::max);

            caps.telemetry().event(
                "image_processed",
                &[
                    ("detection_count", &detection_count.to_string()),
                    ("top_confidence", &format!("{top_confidence:.3}")),
                    ("has_crop", &cropped_data.is_some().to_string()),
                ],
            );

            Ok(StagedPhoto {
                original_data: data,
                processed_data,
                cropped_data,
                width,
                height,
                mime_type: "image/webp".into(),
                detection_count,
                top_confidence,
                detections,
            })
        }

        fn send_create_case_request(
            entry: &OutboxEntry,
            model: &Model,
            caps: &Capabilities,
        ) {
            let OutboxIntent::CreateCase {
                local_id,
                location,
                description,
                landmark_hint,
                wound_severity,
                has_photo,
                ..
            } = &entry.intent
            else {
                return;
            };

            let request = CreateCaseRequest {
                location: *location,
                description: description.clone(),
                landmark_hint: landmark_hint.clone(),
                wound_severity: *wound_severity,
                photo_mime_type: if *has_photo {
                    Some("image/webp".into())
                } else {
                    None
                },
            };

            let body = match serde_json::to_vec(&request) {
                Ok(b) => b,
                Err(e) => {
                    caps.telemetry().error("create_case_serialize_failed", &e.to_string());
                    return;
                }
            };

            let op_id = entry.op_id.0.clone();
            let idempotency_key = entry.idempotency_key.0.clone();
            let timeout = entry.intent.default_timeout();

            let mut builder = caps.http().post("/api/v1/cases");
            builder = builder
                .header("Content-Type", "application/json")
                .header("Idempotency-Key", &idempotency_key)
                .timeout(timeout)
                .body(body);

            if let Some(token) = &model.jwt_token {
                builder = builder.header("Authorization", &format!("Bearer {token}"));
            }

            builder.send(move |result| Event::CreateCaseResponse {
                op_id,
                result: Box::new(result),
            });
        }

        fn send_photo_upload(
            local_id: &LocalOpId,
            upload_url: &str,
            upload_headers: &HashMap<String, String>,
            photo_data: &[u8],
            caps: &Capabilities,
        ) {
            let local_id_str = local_id.0.clone();

            let mut builder = caps.http().put(upload_url);
            builder = builder
                .timeout(UPLOAD_TIMEOUT)
                .body(photo_data.to_vec());

            for (key, value) in upload_headers {
                builder = builder.header(key, value);
            }

            builder.send(move |result| Event::PhotoUploadResponse {
                local_id: local_id_str,
                result: Box::new(result),
            });
        }

        fn send_claim_request(
            case_id: &CaseId,
            pending_claim: &PendingClaim,
            model: &Model,
            caps: &Capabilities,
        ) {
            let case_id_str = case_id.0.clone();
            let mutation_id = pending_claim.mutation_id.clone();
            let idempotency_key = pending_claim.idempotency_key.0.clone();

            let url = format!("/api/v1/cases/{}/claim", case_id.0);

            let mut builder = caps.http().post(&url);
            builder = builder
                .header("Idempotency-Key", &idempotency_key)
                .timeout(CLAIM_TIMEOUT);

            if let Some(token) = &model.jwt_token {
                builder = builder.header("Authorization", &format!("Bearer {token}"));
            }

            builder.send(move |result| Event::ClaimResponse {
                case_id: case_id_str,
                mutation_id,
                result: Box::new(result),
            });
        }

        fn send_transition_request(
            case_id: &CaseId,
            mutation_id: &str,
            next_status: CaseStatus,
            notes: Option<String>,
            model: &Model,
            caps: &Capabilities,
        ) {
            let case_id_str = case_id.0.clone();
            let mutation_id_str = mutation_id.to_string();

            let request = TransitionCaseRequest {
                next_status: next_status.as_str().to_string(),
                notes,
            };

            let body = match serde_json::to_vec(&request) {
                Ok(b) => b,
                Err(e) => {
                    caps.telemetry().error("transition_serialize_failed", &e.to_string());
                    return;
                }
            };

            let url = format!("/api/v1/cases/{}/transition", case_id.0);
            let idempotency_key = Uuid::new_v4().to_string();

            let mut builder = caps.http().post(&url);
            builder = builder
                .header("Content-Type", "application/json")
                .header("Idempotency-Key", &idempotency_key)
                .timeout(TRANSITION_TIMEOUT)
                .body(body);

            if let Some(token) = &model.jwt_token {
                builder = builder.header("Authorization", &format!("Bearer {token}"));
            }

            builder.send(move |result| Event::TransitionResponse {
                case_id: case_id_str,
                mutation_id: mutation_id_str,
                result: Box::new(result),
            });
        }

        fn send_refresh_request(model: &Model, caps: &Capabilities, cursor: Option<&str>) {
            let center = match model.area_center {
                Some(c) => c,
                None => return,
            };

            let mut url = format!(
                "/api/v1/cases?lat={}&lng={}&radius={}",
                center.lat(),
                center.lon(),
                model.area_radius_m
            );

            if let Some(c) = cursor {
                url.push_str(&format!("&cursor={c}"));
            }

            let mut builder = caps.http().get(&url);
            builder = builder.timeout(REFRESH_TIMEOUT);

            if let Some(token) = &model.jwt_token {
                builder = builder.header("Authorization", &format!("Bearer {token}"));
            }

            if cursor.is_some() {
                builder.send(|result| Event::LoadMoreResponse(Box::new(result)));
            } else {
                builder.send(|result| Event::RefreshResponse(Box::new(result)));
            }
        }

        fn send_fcm_token(token: &str, model: &Model, caps: &Capabilities) {
            let body = match serde_json::to_vec(&serde_json::json!({ "token": token })) {
                Ok(b) => b,
                Err(_) => return,
            };

            let mut builder = caps.http().post("/api/v1/profile/fcm-token");
            builder = builder
                .header("Content-Type", "application/json")
                .timeout(FCM_SYNC_TIMEOUT)
                .body(body);

            if let Some(jwt) = &model.jwt_token {
                builder = builder.header("Authorization", &format!("Bearer {jwt}"));
            }

            builder.send(|result| Event::FcmSyncResponse {
                result: Box::new(result),
            });
        }

        fn handle_http_error(error: &HttpError) -> AppError {
            match error {
                HttpError::Network(msg) => {
                    AppError::new(ErrorKind::Network, "Network error").with_internal(msg)
                }
                HttpError::Timeout => AppError::new(ErrorKind::Timeout, "Request timed out"),
                HttpError::Status { code, body } => {
                    AppError::from_http_status(*code, body.as_deref())
                }
                HttpError::Other(msg) => {
                    AppError::new(ErrorKind::Unknown, "Request failed").with_internal(msg)
                }
            }
        }

        fn handle_create_case_response(
            op_id: &str,
            result: &Result<HttpOutput, HttpError>,
            model: &mut Model,
            caps: &Capabilities,
        ) {
            let op_id_typed = OpId::new(op_id);

            match result {
                Ok(output) if output.is_success() => {
                    match serde_json::from_slice::<CreateCaseResponse>(&output.body) {
                        Ok(response) => {
                            if let Some(local_case) = model
                                .offline_store
                                .pending_local_cases
                                .iter_mut()
                                .find(|c| c.local_id.0 == op_id)
                            {
                                local_case.server_id = Some(CaseId::new(&response.id));

                                if let Some(upload_url) = &response.photo_upload_url {
                                    if local_case.photo_data.is_some() {
                                        local_case.photo_upload_url = Some(upload_url.clone());
                                        local_case.mark_uploading_photo();

                                        let headers = response.photo_upload_headers.clone().unwrap_or_default();
                                        Self::send_photo_upload(
                                            &local_case.local_id,
                                            upload_url,
                                            &headers,
                                            local_case.photo_data.as_ref().unwrap(),
                                            caps,
                                        );
                                    } else {
                                        local_case.mark_synced(CaseId::new(&response.id));
                                        model.offline_store.mark_entry_completed(&op_id_typed);
                                    }
                                } else {
                                    local_case.mark_synced(CaseId::new(&response.id));
                                    model.offline_store.mark_entry_completed(&op_id_typed);
                                }
                            } else {
                                model.offline_store.mark_entry_completed(&op_id_typed);
                            }

                            caps.telemetry().event("case_created_success", &[("server_id", &response.id)]);
                        }
                        Err(e) => {
                            caps.telemetry().error("case_response_parse_failed", &e.to_string());
                            model.offline_store.mark_entry_failed(
                                &op_id_typed,
                                OutboxEntryError::new("PARSE_ERROR").with_message(e.to_string()),
                            );
                        }
                    }
                }
                Ok(output) if output.status == 409 => {
                    caps.telemetry().warn("case_create_conflict", op_id);
                    model.offline_store.mark_entry_completed(&op_id_typed);
                }
                Ok(output) if output.status == 429 => {
                    let retry_after = output
                        .header("Retry-After")
                        .and_then(|v| v.parse::<u64>().ok())
                        .map(|s| s * 1000)
                        .unwrap_or(60_000);

                    if let Some(entry) = model.offline_store.get_entry_mut(&op_id_typed) {
                        entry.mark_rate_limited(retry_after);
                    }
                    caps.telemetry().warn("case_create_rate_limited", op_id);
                }
                Ok(output) if output.status >= 400 && output.status < 500 => {
                    let error = OutboxEntryError::server_error(output.status, None);
                    model.offline_store.mark_entry_permanently_failed(&op_id_typed, error);

                    if let Some(local_case) = model
                        .offline_store
                        .pending_local_cases
                        .iter_mut()
                        .find(|c| c.local_id.0 == op_id)
                    {
                        local_case.mark_failed(format!("Server error: {}", output.status));
                    }

                    caps.telemetry().error("case_create_client_error", &output.status.to_string());
                }
                Ok(output) => {
                    let error = OutboxEntryError::server_error(output.status, None);
                    model.offline_store.mark_entry_failed(&op_id_typed, error);
                    caps.telemetry().warn("case_create_server_error", &output.status.to_string());
                }
                Err(e) => {
                    let error = match e {
                        HttpError::Timeout => OutboxEntryError::timeout_error(),
                        _ => OutboxEntryError::network_error(format!("{e:?}")),
                    };
                    model.offline_store.mark_entry_failed(&op_id_typed, error);
                    caps.telemetry().warn("case_create_network_error", &format!("{e:?}"));
                }
            }

            Self::persist_store(model, caps);
        }

        fn handle_photo_upload_response(
            local_id: &str,
            result: &Result<HttpOutput, HttpError>,
            model: &mut Model,
            caps: &Capabilities,
        ) {
            let local_case = match model
                .offline_store
                .pending_local_cases
                .iter_mut()
                .find(|c| c.local_id.0 == local_id)
            {
                Some(c) => c,
                None => return,
            };

            match result {
                Ok(output) if output.is_success() => {
                    if let Some(server_id) = local_case.server_id.clone() {
                        local_case.mark_synced(server_id);
                    }
                    local_case.photo_data = None;

                    if let Some(entry) = model
                        .offline_store
                        .outbox
                        .iter()
                        .find(|e| {
                            matches!(&e.intent, OutboxIntent::CreateCase { local_id: lid, .. } if lid.0 == local_id)
                        })
                    {
                        let op_id = entry.op_id.clone();
                        model.offline_store.mark_entry_completed(&op_id);
                    }

                    caps.telemetry().event("photo_upload_success", &[("local_id", local_id)]);
                }
                Ok(output) => {
                    local_case.mark_failed(format!("Upload failed: {}", output.status));
                    caps.telemetry().error("photo_upload_failed", &output.status.to_string());
                }
                Err(e) => {
                    local_case.mark_failed(format!("Upload error: {e:?}"));
                    caps.telemetry().error("photo_upload_error", &format!("{e:?}"));
                }
            }

            Self::persist_store(model, caps);
        }

        fn handle_claim_response(
            case_id: &str,
            mutation_id: &str,
            result: &Result<HttpOutput, HttpError>,
            model: &mut Model,
            caps: &Capabilities,
        ) {
            let case_id_typed = CaseId::new(case_id);
            model.pending_claims.remove(&case_id_typed);

            match result {
                Ok(output) if output.is_success() => {
                    model.commit_mutation(mutation_id);

                    if let Ok(response) = serde_json::from_slice::<ClaimCaseResponse>(&output.body) {
                        if let Some(updated_case) = response.case {
                            if let Some(case) = model.cases.iter_mut().find(|c| c.id.0 == case_id) {
                                *case = updated_case;
                            }
                        }
                    }

                    model.show_toast("Case claimed successfully", ToastKind::Success);
                    caps.telemetry().event("claim_success", &[("case_id", case_id)]);
                }
                Ok(output) if output.status == 409 => {
                    model.rollback_mutation(mutation_id);
                    model.show_toast("Case was claimed by another rescuer", ToastKind::Warning);
                    caps.telemetry().warn("claim_conflict", case_id);
                }
                Ok(output) => {
                    model.rollback_mutation(mutation_id);
                    let error = Self::handle_http_error(&HttpError::Status {
                        code: output.status,
                        body: Some(output.body.clone()),
                    });
                    model.set_error(error);
                    caps.telemetry().error("claim_failed", &output.status.to_string());
                }
                Err(e) => {
                    model.rollback_mutation(mutation_id);
                    model.set_error(Self::handle_http_error(e));
                    caps.telemetry().error("claim_error", &format!("{e:?}"));
                }
            }
        }

        fn handle_transition_response(
            case_id: &str,
            mutation_id: &str,
            result: &Result<HttpOutput, HttpError>,
            model: &mut Model,
            caps: &Capabilities,
        ) {
            match result {
                Ok(output) if output.is_success() => {
                    model.commit_mutation(mutation_id);

                    if let Ok(response) = serde_json::from_slice::<TransitionCaseResponse>(&output.body) {
                        if let Some(updated_case) = response.case {
                            if let Some(case) = model.cases.iter_mut().find(|c| c.id.0 == case_id) {
                                *case = updated_case;
                            }
                        }
                    }

                    model.show_toast("Status updated", ToastKind::Success);
                    caps.telemetry().event("transition_success", &[("case_id", case_id)]);
                }
                Ok(output) if output.status == 409 => {
                    model.rollback_mutation(mutation_id);
                    model.show_toast("Status was changed by someone else", ToastKind::Warning);
                    caps.telemetry().warn("transition_conflict", case_id);
                }
                Ok(output) => {
                    model.rollback_mutation(mutation_id);
                    let error = Self::handle_http_error(&HttpError::Status {
                        code: output.status,
                        body: Some(output.body.clone()),
                    });
                    model.set_error(error);
                    caps.telemetry().error("transition_failed", &output.status.to_string());
                }
                Err(e) => {
                    model.rollback_mutation(mutation_id);
                    model.set_error(Self::handle_http_error(e));
                    caps.telemetry().error("transition_error", &format!("{e:?}"));
                }
            }
        }

        fn handle_refresh_response(
            result: &Result<HttpOutput, HttpError>,
            model: &mut Model,
            caps: &Capabilities,
            is_load_more: bool,
        ) {
            model.is_refreshing = false;

            match result {
                Ok(output) if output.is_success() => {
                    match serde_json::from_slice::<ListCasesResponse>(&output.body) {
                        Ok(response) => {
                            if is_load_more {
                                model.cases.extend(response.cases);
                            } else {
                                model.cases = response.cases;
                            }
                            model.cases_cursor = response.next_cursor;
                            model.offline_store.update_last_refresh();
                            model.enforce_collection_limits();

                            caps.telemetry().event(
                                if is_load_more { "load_more_success" } else { "refresh_success" },
                                &[("count", &model.cases.len().to_string())],
                            );
                        }
                        Err(e) => {
                            caps.telemetry().error("refresh_parse_failed", &e.to_string());
                        }
                    }
                }
                Ok(output) => {
                    caps.telemetry().warn("refresh_failed", &output.status.to_string());
                }
                Err(e) => {
                    caps.telemetry().warn("refresh_error", &format!("{e:?}"));
                }
            }
        }
    }

    impl crux_core::App for App {
        type Event = Event;
        type Model = Model;
        type ViewModel = ViewModel;
        type Capabilities = Capabilities;

        fn update(&self, event: Event, model: &mut Model, caps: &Capabilities) {
            model.update_timestamp();

            let event_name = event.name();
            caps.telemetry().counter(&format!("event.{event_name}"), 1);

            if event.is_user_initiated() {
                caps.telemetry().event("user_action", &[("event", event_name)]);
            }

            match event {
                Event::Noop => {}

                Event::AppStarted => {
                    model.state = AppState::Loading;

                    if let Some(model_bytes) = crate::vision::load_bundled_model() {
                        match crate::vision::YoloDetector::new(&model_bytes) {
                            Ok(detector) => {
                                model.yolo_detector = Some(detector);
                                caps.telemetry().event("yolo_initialized", &[]);
                            }
                            Err(e) => {
                                caps.telemetry().error("yolo_init_failed", &e.to_string());
                            }
                        }
                    }

                    caps.render().render();
                }

                Event::AppBackgrounded => {
                    Self::persist_store(model, caps);
                    caps.telemetry().event("app_backgrounded", &[]);
                }

                Event::AppForegrounded => {
                    model.update_timestamp();

                    if model.state == AppState::Ready && model.network_online {
                        Self::send_refresh_request(model, caps, None);
                        model.is_refreshing = true;
                    }

                    caps.telemetry().event("app_foregrounded", &[]);
                    caps.render().render();
                }

                Event::LoginRequested => {
                    model.state = AppState::Authenticating;
                    caps.render().render();
                }

                Event::LoginCompleted { jwt, user_id } => {
                    model.user_id = Some(UserId::new(&user_id));
                    model.jwt_token = Some(jwt);
                    model.state = AppState::OnboardingLocation;

                    caps.telemetry().event("login_success", &[]);
                    caps.render().render();
                }

                Event::LoginFailed { error } => {
                    model.state = AppState::Unauthenticated;
                    model.set_error(AppError::new(ErrorKind::Authentication, &error));

                    caps.telemetry().error("login_failed", &error);
                    caps.render().render();
                }

                Event::LogoutRequested => {
                    model.user_id = None;
                    model.jwt_token = None;
                    model.state = AppState::Unauthenticated;
                    model.cases.clear();
                    model.offline_store = OfflineStore::new();
                    model.pending_claims.clear();
                    model.pending_mutations.clear();
                    model.staged_photo = None;
                    model.selected_case_id = None;

                    caps.telemetry().event("logout", &[]);
                    caps.render().render();
                }

                Event::LogoutCompleted => {
                    caps.render().render();
                }

                Event::TokenRefreshRequired => {
                    caps.telemetry().event("token_refresh_required", &[]);
                }

                Event::TokenRefreshed { jwt } => {
                    model.jwt_token = Some(jwt);
                    caps.telemetry().event("token_refreshed", &[]);
                }

                Event::TokenRefreshFailed { error } => {
                    model.jwt_token = None;
                    model.state = AppState::Unauthenticated;
                    model.set_error(AppError::new(ErrorKind::Authentication, "Session expired"));

                    caps.telemetry().error("token_refresh_failed", &error);
                    caps.render().render();
                }

                Event::LocationPermissionRequested => {
                    model.location_permission_state = PermissionState::Requesting;
                    caps.location().request_permission(|granted| Event::LocationPermissionResult { granted });
                    caps.render().render();
                }

                Event::LocationPermissionResult { granted } => {
                    model.location_permission_state = if granted {
                        PermissionState::Granted
                    } else {
                        PermissionState::Denied
                    };

                    if granted {
                        caps.location().get_current(|result| match result {
                            Ok((lat, lng, accuracy)) => Event::LocationReceived { lat, lng, accuracy },
                            Err(e) => Event::LocationFailed { error: e },
                        });
                    } else if model.state == AppState::OnboardingLocation {
                        model.state = AppState::PinDrop;
                    }

                    caps.telemetry().event(
                        "location_permission",
                        &[("granted", &granted.to_string())],
                    );
                    caps.render().render();
                }

                Event::LocationReceived { lat, lng, accuracy: _ } => {
                    match Self::validate_coordinates(lat, lng) {
                        Ok(coord) => {
                            model.area_center = Some(coord);
                            model.map_center = Some(coord);
                            model.map_zoom = DEFAULT_MAP_ZOOM;

                            if model.state == AppState::OnboardingLocation {
                                model.state = AppState::OnboardingRadius;
                            }

                            caps.telemetry().event("location_received", &[]);
                        }
                        Err(e) => {
                            model.set_error(e);
                            caps.telemetry().error("location_invalid", &format!("{lat}, {lng}"));
                        }
                    }
                    caps.render().render();
                }

                Event::LocationFailed { error } => {
                    if model.state == AppState::OnboardingLocation {
                        model.state = AppState::PinDrop;
                    }

                    caps.telemetry().error("location_failed", &error);
                    caps.render().render();
                }

                Event::LocationPinDropped { lat, lng } => {
                    match Self::validate_coordinates(lat, lng) {
                        Ok(coord) => {
                            model.area_center = Some(coord);
                            model.map_center = Some(coord);
                            model.map_zoom = DEFAULT_MAP_ZOOM;

                            if model.state == AppState::PinDrop {
                                model.state = AppState::OnboardingRadius;
                            }

                            caps.telemetry().event("pin_dropped", &[]);
                        }
                        Err(e) => {
                            model.set_error(e);
                        }
                    }
                    caps.render().render();
                }

                Event::RadiusSelected { meters } => {
                    let radius = meters.clamp(MIN_RADIUS_M, MAX_RADIUS_M);
                    let radius = if radius == 0 { DEFAULT_RADIUS_M } else { radius };

                    model.area_radius_m = radius;
                    model.map_zoom = zoom_for_radius(radius);

                    if model.state == AppState::OnboardingRadius {
                        model.state = AppState::Ready;

                        caps.push().request_permission(|granted| {
                            Event::PushPermissionResult { granted }
                        });

                        if model.network_online {
                            Self::send_refresh_request(model, caps, None);
                            model.is_refreshing = true;
                        }
                    }

                    caps.telemetry().event("radius_selected", &[("meters", &radius.to_string())]);
                    caps.render().render();
                }

                Event::OnboardingComplete => {
                    model.state = AppState::Ready;
                    caps.render().render();
                }

                Event::NetworkStatusChanged { online } => {
                    let was_offline = !model.network_online;
                    model.network_online = online;

                    if online && was_offline {
                        self.update(Event::OutboxFlushRequested, model, caps);

                        if model.state == AppState::Ready {
                            Self::send_refresh_request(model, caps, None);
                            model.is_refreshing = true;
                        }
                    }

                    caps.telemetry().event(
                        "network_changed",
                        &[("online", &online.to_string())],
                    );
                    caps.render().render();
                }

                Event::CameraPermissionRequested => {
                    model.camera_permission_state = PermissionState::Requesting;
                    caps.camera().request_permission(|granted| {
                        Event::CameraPermissionResult { granted }
                    });
                }

                Event::CameraPermissionResult { granted } => {
                    model.camera_permission_state = if granted {
                        PermissionState::Granted
                    } else {
                        PermissionState::Denied
                    };

                    if granted {
                        self.update(Event::CapturePhotoRequested, model, caps);
                    } else {
                        model.set_error(AppError::new(
                            ErrorKind::CameraPermissionDenied,
                            "Camera permission required",
                        ));
                    }

                    caps.render().render();
                }

                Event::CapturePhotoRequested => {
                    if !model.camera_permission_state.is_granted() {
                        self.update(Event::CameraPermissionRequested, model, caps);
                        return;
                    }

                    model.state = AppState::CameraCapture;
                    caps.camera().capture(|result| Event::CameraResult(Box::new(result)));
                    caps.render().render();
                }

                Event::CameraResult(result) => {
                    model.state = AppState::Ready;

                    match *result {
                        Ok(CameraOutput::Photo { data, mime_type: _ }) => {
                            match Self::process_camera_image(data, model, caps) {
                                Ok(staged) => {
                                    model.staged_photo = Some(staged);
                                }
                                Err(e) => {
                                    model.set_error(e);
                                }
                            }
                        }
                        Ok(CameraOutput::Cancelled) => {
                            caps.telemetry().event("camera_cancelled", &[]);
                        }
                        Err(e) => {
                            let error = match e {
                                CameraError::PermissionDenied => AppError::new(
                                    ErrorKind::CameraPermissionDenied,
                                    "Camera permission denied",
                                ),
                                CameraError::Unavailable => AppError::new(
                                    ErrorKind::FeatureUnavailable,
                                    "Camera unavailable",
                                ),
                                CameraError::Failed(msg) => {
                                    AppError::new(ErrorKind::Camera, msg)
                                }
                            };
                            model.set_error(error);
                            caps.telemetry().error("camera_error", &format!("{e:?}"));
                        }
                    }

                    caps.render().render();
                }

                Event::ClearStagedPhoto => {
                    model.staged_photo = None;
                    caps.render().render();
                }

                Event::PhotoProcessed { staged_photo } => {
                    model.staged_photo = Some(staged_photo);
                    caps.render().render();
                }

                Event::PhotoProcessingFailed { error } => {
                    model.set_error(AppError::new(ErrorKind::ImageProcessing, error));
                    caps.render().render();
                }

                Event::CreateCaseRequested(payload) => {
                    let coord = match Self::validate_coordinates(payload.location.0, payload.location.1) {
                        Ok(c) => c,
                        Err(e) => {
                            model.set_error(e);
                            caps.render().render();
                            return;
                        }
                    };

                    let has_photo = model.staged_photo.is_some();
                    let photo_data = model.staged_photo.as_ref().map(|p| p.best_data_for_upload().to_vec());

                    let mut local_case = LocalCase::new(
                        coord.into(),
                        payload.description.clone(),
                        payload.wound_severity,
                    );
                    local_case.landmark_hint = payload.landmark_hint.clone();
                    local_case.photo_data = photo_data;

                    let local_id = local_case.local_id.clone();

                    if let Err(e) = model.offline_store.push_local_case(local_case) {
                        model.set_error(e.into());
                        caps.render().render();
                        return;
                    }

                    let intent = OutboxIntent::CreateCase {
                        local_id: local_id.clone(),
                        location: coord.into(),
                        description: payload.description,
                        landmark_hint: payload.landmark_hint,
                        wound_severity: payload.wound_severity,
                        has_photo,
                        created_at_ms_utc: UnixTimeMs::now(),
                    };

                    let entry = OutboxEntry::new(intent);

                    if let Err(e) = model.offline_store.push_outbox(entry) {
                        model.set_error(e.into());
                        caps.render().render();
                        return;
                    }

                    model.staged_photo = None;
                    model.map_center = Some(coord);

                    Self::persist_store(model, caps);

                    model.show_toast("Case created", ToastKind::Success);
                    caps.telemetry().event("case_created_local", &[("local_id", &local_id.0)]);

                    caps.render().render();

                    if model.network_online {
                        self.update(Event::OutboxFlushRequested, model, caps);
                    }
                }

                Event::CreateCaseResponse { op_id, result } => {
                    Self::handle_create_case_response(&op_id, &result, model, caps);
                    caps.render().render();

                    self.update(Event::OutboxFlushRequested, model, caps);
                }

                Event::PhotoUploadResponse { local_id, result } => {
                    Self::handle_photo_upload_response(&local_id, &result, model, caps);
                    caps.render().render();
                }

                Event::WriteEncryptedStore { key_id, data } => {
                    caps.kv().set(&key_id, data, |result| match result {
                        Ok(()) => Event::PersistenceSucceeded,
                        Err(e) => Event::PersistenceFailed {
                            error: format!("{e:?}"),
                        },
                    });
                }

                Event::PersistenceSucceeded => {
                    model.offline_store.update_last_sync();
                    caps.telemetry().event("persistence_success", &[]);
                }

                Event::PersistenceFailed { error } => {
                    caps.telemetry().error("persistence_failed", &error);
                }

                Event::RestoreStateRequested => {
                    if let Some(user_id) = &model.user_id {
                        let key_id = Self::derive_store_key_id(user_id);
                        caps.kv().get(&key_id, |result| Event::RestoreStateResponse {
                            result: Box::new(result),
                        });
                    }
                }

                Event::RestoreStateResponse { result } => {
                    match *result {
                        Ok(data) => {
                            if let Some(user_id) = &model.user_id {
                                let key_id = Self::derive_store_key_id(user_id);
                                caps.crypto().decrypt(key_id, data, |result| match result {
                                    Ok(CryptoOutput::Decrypted(bytes)) => {
                                        Event::StateDecrypted { data: bytes }
                                    }
                                    _ => Event::StateDecryptionFailed {
                                        error: "Decryption failed".into(),
                                    },
                                });
                            }
                        }
                        Err(KvError::NotFound) => {
                            caps.telemetry().event("no_stored_state", &[]);
                        }
                        Err(e) => {
                            caps.telemetry().error("state_load_failed", &format!("{e:?}"));
                        }
                    }
                }

                Event::StateDecrypted { data } => {
                    match serde_cbor::from_slice::<OfflineStore>(&data) {
                        Ok(store) => {
                            model.offline_store = store;
                            caps.telemetry().event("state_restored", &[]);
                        }
                        Err(e) => {
                            caps.telemetry().error("state_deserialize_failed", &e.to_string());
                        }
                    }
                    caps.render().render();
                }

                Event::StateDecryptionFailed { error } => {
                    caps.telemetry().error("state_decryption_failed", &error);
                }

                Event::OutboxFlushRequested => {
                    if !model.network_online {
                        return;
                    }

                    let now_ms = get_current_time_ms();

                    if let Some(entry) = model.offline_store.get_next_pending_entry(now_ms) {
                        let entry = entry.clone();

                        if let Some(e) = model.offline_store.get_entry_mut(&entry.op_id) {
                            e.mark_in_flight();
                        }

                        match &entry.intent {
                            OutboxIntent::CreateCase { .. } => {
                                Self::send_create_case_request(&entry, model, caps);
                            }
                            OutboxIntent::UploadPhoto {
                                local_id,
                                upload_url,
                                upload_headers,
                            } => {
                                if let Some(local_case) = model
                                    .offline_store
                                    .pending_local_cases
                                    .iter()
                                    .find(|c| &c.local_id == local_id)
                                {
                                    if let Some(photo_data) = &local_case.photo_data {
                                        Self::send_photo_upload(
                                            local_id,
                                            upload_url,
                                            upload_headers,
                                            photo_data,
                                            caps,
                                        );
                                    }
                                }
                            }
                            OutboxIntent::ClaimCase { case_id } => {
                                if let Some(pending) = model.pending_claims.get(case_id) {
                                    Self::send_claim_request(case_id, pending, model, caps);
                                }
                            }
                            OutboxIntent::TransitionCase {
                                case_id,
                                next_status,
                                notes,
                            } => {
                                let mutation_id = Uuid::new_v4().to_string();
                                Self::send_transition_request(
                                    case_id,
                                    &mutation_id,
                                    *next_status,
                                    notes.clone(),
                                    model,
                                    caps,
                                );
                            }
                            OutboxIntent::SyncFcmToken { token } => {
                                Self::send_fcm_token(token, model, caps);
                            }
                        }

                        caps.telemetry().event(
                            "outbox_processing",
                            &[
                                ("op_id", &entry.op_id.0),
                                ("intent", entry.intent.intent_type()),
                            ],
                        );
                    }
                }

                Event::OutboxEntryCompleted { op_id } => {
                    model.offline_store.mark_entry_completed(&OpId::new(&op_id));
                    Self::persist_store(model, caps);
                    caps.render().render();

                    self.update(Event::OutboxFlushRequested, model, caps);
                }

                Event::OutboxEntryFailed {
                    op_id,
                    error,
                    is_permanent,
                } => {
                    let op_id_typed = OpId::new(&op_id);
                    let err = OutboxEntryError::new("FAILED").with_message(&error);

                    if is_permanent {
                        model.offline_store.mark_entry_permanently_failed(&op_id_typed, err);
                    } else {
                        model.offline_store.mark_entry_failed(&op_id_typed, err);
                    }

                    Self::persist_store(model, caps);
                    caps.render().render();
                }

                Event::SwitchToMap => {
                    model.feed_view = FeedView::Map;
                    caps.render().render();
                }

                Event::SwitchToList => {
                    model.feed_view = FeedView::List;
                    caps.render().render();
                }

                Event::ToggleFeedView => {
                    model.feed_view = model.feed_view.toggle();
                    caps.render().render();
                }

                Event::MapMoved { center, zoom } => {
                    if let Ok(coord) = center.to_validated() {
                        model.map_center = Some(coord);
                    }
                    model.map_zoom = zoom.value();
                }

                Event::CaseSelected { case_id } => {
                    model.selected_case_id = Some(CaseId::new(&case_id));
                    caps.telemetry().event("case_selected", &[("case_id", &case_id)]);
                    caps.render().render();
                }

                Event::CaseDeselected => {
                    model.selected_case_id = None;
                    caps.render().render();
                }

                Event::ClaimRequested { case_id } => {
                    let case_id_typed = CaseId::new(&case_id);

                    let case = match model.cases.iter().find(|c| c.id.0 == case_id) {
                        Some(c) => c,
                        None => {
                            caps.telemetry().warn("claim_case_not_found", &case_id);
                            return;
                        }
                    };

                    if !case.status.is_claimable() {
                        model.show_toast("Case cannot be claimed", ToastKind::Warning);
                        return;
                    }

                    if model.pending_claims.contains_key(&case_id_typed) {
                        return;
                    }

                    let pending = PendingClaim::new(
                        case_id_typed.clone(),
                        case.status,
                        case.assigned_rescuer_id.clone(),
                    );

                    let mutation_id = model.store_optimistic_mutation(
                        case_id_typed.clone(),
                        case.status,
                        case.assigned_rescuer_id.clone(),
                        CaseStatus::Claimed,
                    );

                    let mut pending = pending;
                    pending.mutation_id = mutation_id;

                    model.pending_claims.insert(case_id_typed.clone(), pending.clone());

                    if let Some(case) = model.cases.iter_mut().find(|c| c.id.0 == case_id) {
                        case.status = CaseStatus::Claimed;
                        case.assigned_rescuer_id = model.user_id.clone();
                    }

                    caps.render().render();

                    Self::send_claim_request(&case_id_typed, &pending, model, caps);
                    caps.telemetry().event("claim_requested", &[("case_id", &case_id)]);
                }

                Event::ClaimResponse {
                    case_id,
                    mutation_id,
                    result,
                } => {
                    Self::handle_claim_response(&case_id, &mutation_id, &result, model, caps);
                    caps.render().render();
                }

                Event::TransitionRequested {
                    case_id,
                    next_status,
                    notes,
                } => {
                    let next = match CaseStatus::from_str(&next_status) {
                        Some(s) => s,
                        None => {
                            model.set_error(AppError::new(
                                ErrorKind::Validation,
                                format!("Invalid status: {next_status}"),
                            ));
                            caps.render().render();
                            return;
                        }
                    };

                    let case = match model.cases.iter().find(|c| c.id.0 == case_id) {
                        Some(c) => c,
                        None => {
                            caps.telemetry().warn("transition_case_not_found", &case_id);
                            return;
                        }
                    };

                    if let Err(e) = case.status.validate_transition(next) {
                        model.set_error(e.into());
                        caps.render().render();
                        return;
                    }

                    let mutation_id = model.store_optimistic_mutation(
                        CaseId::new(&case_id),
                        case.status,
                        case.assigned_rescuer_id.clone(),
                        next,
                    );

                    if let Some(case) = model.cases.iter_mut().find(|c| c.id.0 == case_id) {
                        case.status = next;
                    }

                    caps.render().render();

                    Self::send_transition_request(
                        &CaseId::new(&case_id),
                        &mutation_id,
                        next,
                        notes,
                        model,
                        caps,
                    );

                    caps.telemetry().event(
                        "transition_requested",
                        &[("case_id", &case_id), ("next", next.as_str())],
                    );
                }

                Event::TransitionResponse {
                    case_id,
                    mutation_id,
                    result,
                } => {
                    Self::handle_transition_response(&case_id, &mutation_id, &result, model, caps);
                    caps.render().render();
                }

                Event::RefreshRequested => {
                    if !model.network_online {
                        model.show_toast("No internet connection", ToastKind::Warning);
                                                caps.render().render();
                        return;
                    }

                    if model.is_refreshing {
                        return;
                    }

                    model.is_refreshing = true;
                    caps.render().render();

                    Self::send_refresh_request(model, caps, None);
                    caps.telemetry().event("refresh_requested", &[]);
                }

                Event::RefreshResponse(result) => {
                    Self::handle_refresh_response(&result, model, caps, false);
                    caps.render().render();
                }

                Event::LoadMoreCases => {
                    if !model.network_online || model.is_refreshing {
                        return;
                    }

                    if let Some(cursor) = &model.cases_cursor.clone() {
                        model.is_refreshing = true;
                        caps.render().render();

                        Self::send_refresh_request(model, caps, Some(cursor));
                        caps.telemetry().event("load_more_requested", &[]);
                    }
                }

                Event::LoadMoreResponse(result) => {
                    Self::handle_refresh_response(&result, model, caps, true);
                    caps.render().render();
                }

                Event::PushPermissionRequested => {
                    caps.push().request_permission(|granted| {
                        Event::PushPermissionResult { granted }
                    });
                }

                Event::PushPermissionResult { granted } => {
                    model.push_permission_granted = granted;

                    if granted {
                        caps.push().get_token(|result| match result {
                            Ok(token) => Event::PushTokenReceived { token },
                            Err(e) => Event::PushTokenFailed { error: e },
                        });
                    }

                    caps.telemetry().event(
                        "push_permission",
                        &[("granted", &granted.to_string())],
                    );
                    caps.render().render();
                }

                Event::PushTokenReceived { token } => {
                    model.push_token = Some(token.clone());

                    if model.network_online {
                        Self::send_fcm_token(&token, model, caps);
                    } else {
                        let intent = OutboxIntent::SyncFcmToken { token };
                        let entry = OutboxEntry::new(intent);
                        let _ = model.offline_store.push_outbox(entry);
                    }

                    caps.telemetry().event("push_token_received", &[]);
                }

                Event::PushTokenFailed { error } => {
                    caps.telemetry().error("push_token_failed", &error);
                }

                Event::PushReceived(payload) => {
                    match payload {
                        PushPayload::NewCase { case_id, lat, lng, severity } => {
                            caps.telemetry().event(
                                "push_new_case",
                                &[("case_id", &case_id)],
                            );

                            if let Ok(coord) = ValidatedCoordinate::new(lat, lng) {
                                if let Some(center) = model.area_center {
                                    let distance = haversine_distance(center, coord);
                                    if distance <= f64::from(model.area_radius_m) {
                                        Self::send_refresh_request(model, caps, None);
                                        model.is_refreshing = true;
                                    }
                                }
                            }
                        }
                        PushPayload::CaseClaimed { case_id, claimed_by } => {
                            if let Some(case) = model.cases.iter_mut().find(|c| c.id.0 == case_id) {
                                case.status = CaseStatus::Claimed;
                                case.assigned_rescuer_id = Some(UserId::new(&claimed_by));
                            }

                            let dominated_by_other = model.user_id.as_ref()
                                .map(|uid| uid.0 != claimed_by)
                                .unwrap_or(true);

                            if dominated_by_other {
                                if model.selected_case_id.as_ref().map(|id| id.0 == case_id).unwrap_or(false) {
                                    model.show_toast("Case claimed by another rescuer", ToastKind::Info);
                                }
                            }

                            caps.telemetry().event(
                                "push_case_claimed",
                                &[("case_id", &case_id), ("claimed_by", &claimed_by)],
                            );
                        }
                        PushPayload::CaseUpdated { case_id, new_status, updated_by: _ } => {
                            if let Some(status) = CaseStatus::from_str(&new_status) {
                                if let Some(case) = model.cases.iter_mut().find(|c| c.id.0 == case_id) {
                                    case.status = status;
                                }
                            }

                            caps.telemetry().event(
                                "push_case_updated",
                                &[("case_id", &case_id), ("status", &new_status)],
                            );
                        }
                        PushPayload::CaseResolved { case_id } => {
                            if let Some(case) = model.cases.iter_mut().find(|c| c.id.0 == case_id) {
                                case.status = CaseStatus::Resolved;
                            }

                            caps.telemetry().event("push_case_resolved", &[("case_id", &case_id)]);
                        }
                        PushPayload::CaseCancelled { case_id, reason: _ } => {
                            if let Some(case) = model.cases.iter_mut().find(|c| c.id.0 == case_id) {
                                case.status = CaseStatus::Cancelled;
                            }

                            caps.telemetry().event("push_case_cancelled", &[("case_id", &case_id)]);
                        }
                    }

                    caps.render().render();
                }

                Event::FcmSyncResponse { result } => {
                    match &*result {
                        Ok(output) if output.is_success() => {
                            caps.telemetry().event("fcm_sync_success", &[]);
                        }
                        Ok(output) => {
                            caps.telemetry().warn("fcm_sync_failed", &output.status.to_string());
                        }
                        Err(e) => {
                            caps.telemetry().warn("fcm_sync_error", &format!("{e:?}"));
                        }
                    }
                }

                Event::DismissError => {
                    model.clear_error();
                    caps.render().render();
                }

                Event::DismissToast => {
                    model.clear_toast();
                    caps.render().render();
                }

                Event::ShowToast { message, kind } => {
                    model.show_toast(message, kind);
                    caps.render().render();
                }

                Event::TimerTick => {
                    model.update_timestamp();

                    if let Some(toast) = &model.active_toast {
                        if toast.is_expired(model.view_timestamp_ms) {
                            model.clear_toast();
                            caps.render().render();
                        }
                    }

                    for mutation_id in model
                        .pending_mutations
                        .iter()
                        .filter(|(_, m)| {
                            model.view_timestamp_ms.saturating_sub(m.created_at_ms) > 30_000
                        })
                        .map(|(id, _)| id.clone())
                        .collect::<Vec<_>>()
                    {
                        model.rollback_mutation(&mutation_id);
                        caps.telemetry().warn("mutation_timeout", &mutation_id);
                    }

                    for case_id in model
                        .pending_claims
                        .iter()
                        .filter(|(_, c)| {
                            model.view_timestamp_ms.saturating_sub(c.created_at_ms) > 30_000
                        })
                        .map(|(id, _)| id.clone())
                        .collect::<Vec<_>>()
                    {
                        if let Some(pending) = model.pending_claims.remove(&case_id) {
                            model.rollback_mutation(&pending.mutation_id);
                        }
                        caps.telemetry().warn("claim_timeout", &case_id.0);
                    }
                }

                Event::RetryFailedOperations => {
                    for case in &mut model.offline_store.pending_local_cases {
                        if case.status == LocalCaseStatus::Failed {
                            case.status = LocalCaseStatus::PendingUpload;
                        }
                    }

                    for entry in &mut model.offline_store.outbox {
                        if entry.retry_state == RetryState::Failed {
                            entry.retry_state = RetryState::Pending;
                            entry.next_retry_at = None;
                        }
                    }

                    Self::persist_store(model, caps);

                    if model.network_online {
                        self.update(Event::OutboxFlushRequested, model, caps);
                    }

                    caps.telemetry().event("retry_failed_requested", &[]);
                    caps.render().render();
                }
            }
        }

        fn view(&self, model: &Model) -> ViewModel {
            let now_ms = model.view_timestamp_ms;

            let state = match model.state {
                AppState::Loading => ViewState::Loading { message: None },

                AppState::Unauthenticated => ViewState::Unauthenticated,

                AppState::Authenticating => ViewState::Authenticating,

                AppState::OnboardingLocation => ViewState::OnboardingLocation {
                    permission_state: model.location_permission_state,
                },

                AppState::PinDrop => ViewState::PinDrop {
                    initial_lat: model.area_center.map(|c| c.lat()),
                    initial_lon: model.area_center.map(|c| c.lon()),
                },

                AppState::OnboardingRadius => {
                    match model.area_center {
                        Some(center) => ViewState::OnboardingRadius {
                            lat: center.lat(),
                            lon: center.lon(),
                            radius: model.area_radius_m,
                            selected_radius: model.area_radius_m,
                        },
                        None => ViewState::Error {
                            title: "Location Required".into(),
                            message: "Please set your location first".into(),
                            is_retryable: true,
                            retry_event: Some("location_permission_requested".into()),
                        },
                    }
                }

                AppState::CameraCapture => ViewState::CameraCapture {
                    config: CaptureConfig::default(),
                },

                AppState::Ready => {
                    match model.area_center {
                        Some(area_center) => {
                            let pins = Self::build_case_pins(model);
                            let list_items = Self::build_list_items(model, now_ms);

                            let selected_detail = model
                                .selected_case_id
                                .as_ref()
                                .and_then(|id| Self::build_case_detail(model, &id.0, now_ms));

                            let map_center = model.map_center.unwrap_or(area_center);

                            let staged_photo = model.staged_photo.as_ref().map(|p| StagedPhotoView {
                                has_photo: true,
                                detection_count: p.detection_count,
                                top_confidence: p.top_confidence,
                                has_detections: p.has_detections(),
                            });

                            ViewState::Ready {
                                feed_view: model.feed_view,
                                pins,
                                list_items,
                                selected_detail,
                                map_center_lat: map_center.lat(),
                                map_center_lon: map_center.lon(),
                                map_zoom: model.map_zoom,
                                is_refreshing: model.is_refreshing,
                                online: model.network_online,
                                pending_sync_count: model.offline_store.pending_sync_count(),
                                failed_sync_count: model.offline_store.failed_count(),
                                staged_photo,
                                has_more_cases: model.cases_cursor.is_some(),
                            }
                        }
                        None => ViewState::Error {
                            title: "Location Required".into(),
                            message: "Please set your location to continue".into(),
                            is_retryable: true,
                            retry_event: Some("location_permission_requested".into()),
                        },
                    }
                }

                AppState::Error => ViewState::Error {
                    title: "Error".into(),
                    message: model
                        .active_error
                        .as_ref()
                        .map(|e| e.user_facing_message())
                        .unwrap_or_else(|| "An unknown error occurred".into()),
                    is_retryable: model
                        .active_error
                        .as_ref()
                        .map(|e| e.is_retryable())
                        .unwrap_or(false),
                    retry_event: None,
                },
            };

            ViewModel {
                state,
                error: model.active_error.as_ref().map(UserFacingError::from),
                toast: model.active_toast.as_ref().map(ToastView::from),
                is_global_loading: model.is_loading,
                offline_queue_count: model.offline_store.pending_sync_count(),
                is_authenticated: model.is_authenticated(),
                user_id: model.user_id.as_ref().map(|u| u.0.clone()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod coordinate_tests {
        use super::*;

        #[test]
        fn test_valid_coordinates() {
            assert!(ValidatedCoordinate::new(0.0, 0.0).is_ok());
            assert!(ValidatedCoordinate::new(90.0, 180.0).is_ok());
            assert!(ValidatedCoordinate::new(-90.0, -180.0).is_ok());
            assert!(ValidatedCoordinate::new(51.5074, -0.1278).is_ok());
        }

        #[test]
        fn test_invalid_latitude() {
            assert!(matches!(
                ValidatedCoordinate::new(91.0, 0.0),
                Err(CoordinateError::LatitudeOutOfRange(_))
            ));
            assert!(matches!(
                ValidatedCoordinate::new(-91.0, 0.0),
                Err(CoordinateError::LatitudeOutOfRange(_))
            ));
        }

        #[test]
        fn test_invalid_longitude() {
            assert!(matches!(
                ValidatedCoordinate::new(0.0, 181.0),
                Err(CoordinateError::LongitudeOutOfRange(_))
            ));
            assert!(matches!(
                ValidatedCoordinate::new(0.0, -181.0),
                Err(CoordinateError::LongitudeOutOfRange(_))
            ));
        }

        #[test]
        fn test_non_finite_coordinates() {
            assert!(matches!(
                ValidatedCoordinate::new(f64::NAN, 0.0),
                Err(CoordinateError::NonFinite)
            ));
            assert!(matches!(
                ValidatedCoordinate::new(0.0, f64::INFINITY),
                Err(CoordinateError::NonFinite)
            ));
            assert!(matches!(
                ValidatedCoordinate::new(f64::NEG_INFINITY, 0.0),
                Err(CoordinateError::NonFinite)
            ));
        }
    }

    mod distance_tests {
        use super::*;

        #[test]
        fn test_same_point_distance() {
            let p = ValidatedCoordinate::new(51.5074, -0.1278).unwrap();
            assert_eq!(haversine_distance(p, p), 0.0);
        }

        #[test]
        fn test_near_zero_distance() {
            let p1 = ValidatedCoordinate::new(51.5074, -0.1278).unwrap();
            let p2 = ValidatedCoordinate::new(51.5074000001, -0.1278000001).unwrap();
            let dist = haversine_distance(p1, p2);
            assert!(dist < 1.0);
        }

        #[test]
        fn test_london_paris_distance() {
            let london = ValidatedCoordinate::new(51.5074, -0.1278).unwrap();
            let paris = ValidatedCoordinate::new(48.8566, 2.3522).unwrap();
            let distance = haversine_distance(london, paris);
            assert!((distance - 343_500.0).abs() < 10_000.0);
        }

        #[test]
        fn test_antipodal_distance() {
            let p1 = ValidatedCoordinate::new(0.0, 0.0).unwrap();
            let p2 = ValidatedCoordinate::new(0.0, 180.0).unwrap();
            let distance = haversine_distance(p1, p2);
            let expected = std::f64::consts::PI * EARTH_RADIUS_M;
            assert!((distance - expected).abs() < 1000.0);
        }
    }

    mod format_tests {
        use super::*;

        #[test]
        fn test_format_distance_meters() {
            assert_eq!(format_distance(0.0), "0 m");
            assert_eq!(format_distance(500.0), "500 m");
            assert_eq!(format_distance(999.0), "999 m");
        }

        #[test]
        fn test_format_distance_kilometers() {
            assert_eq!(format_distance(1000.0), "1.0 km");
            assert_eq!(format_distance(1500.0), "1.5 km");
            assert_eq!(format_distance(9999.0), "10.0 km");
            assert_eq!(format_distance(15000.0), "15 km");
            assert_eq!(format_distance(150000.0), "150 km");
        }

        #[test]
        fn test_format_distance_invalid() {
            assert_eq!(format_distance(f64::NAN), "Unknown");
            assert_eq!(format_distance(f64::INFINITY), "Unknown");
            assert_eq!(format_distance(-100.0), "Unknown");
        }

        #[test]
        fn test_format_time_ago_just_now() {
            assert_eq!(format_time_ago(1000, 1000), "Just now");
            assert_eq!(format_time_ago(1000, 1004), "Just now");
            assert_eq!(format_time_ago(1000, 4999), "Just now");
        }

        #[test]
        fn test_format_time_ago_seconds() {
            assert_eq!(format_time_ago(0, 10_000), "10s ago");
            assert_eq!(format_time_ago(0, 59_000), "59s ago");
        }

        #[test]
        fn test_format_time_ago_minutes() {
            assert_eq!(format_time_ago(0, 60_000), "1m ago");
            assert_eq!(format_time_ago(0, 300_000), "5m ago");
            assert_eq!(format_time_ago(0, 3_599_000), "59m ago");
        }

        #[test]
        fn test_format_time_ago_hours() {
            assert_eq!(format_time_ago(0, 3_600_000), "1h ago");
            assert_eq!(format_time_ago(0, 7_200_000), "2h ago");
            assert_eq!(format_time_ago(0, 86_399_000), "23h ago");
        }

        #[test]
        fn test_format_time_ago_days() {
            assert_eq!(format_time_ago(0, 86_400_000), "1d ago");
            assert_eq!(format_time_ago(0, 172_800_000), "2d ago");
            assert_eq!(format_time_ago(0, 604_799_000), "6d ago");
        }

        #[test]
        fn test_format_time_ago_weeks() {
            assert_eq!(format_time_ago(0, 604_800_000), "1w ago");
            assert_eq!(format_time_ago(0, 2_419_200_000), "4w ago");
        }

        #[test]
        fn test_format_time_ago_future() {
            assert_eq!(format_time_ago(2000, 1000), "Just now");
            assert_eq!(format_time_ago(120_000, 1000), "Upcoming");
        }
    }

    mod case_status_tests {
        use super::*;

        #[test]
        fn test_status_from_str() {
            assert_eq!(CaseStatus::from_str("pending"), Some(CaseStatus::Pending));
            assert_eq!(CaseStatus::from_str("PENDING"), Some(CaseStatus::Pending));
            assert_eq!(CaseStatus::from_str("Pending"), Some(CaseStatus::Pending));
            assert_eq!(CaseStatus::from_str("open"), Some(CaseStatus::Pending));
            assert_eq!(CaseStatus::from_str("claimed"), Some(CaseStatus::Claimed));
            assert_eq!(CaseStatus::from_str("en_route"), Some(CaseStatus::EnRoute));
            assert_eq!(CaseStatus::from_str("enroute"), Some(CaseStatus::EnRoute));
            assert_eq!(CaseStatus::from_str("arrived"), Some(CaseStatus::Arrived));
            assert_eq!(CaseStatus::from_str("resolved"), Some(CaseStatus::Resolved));
            assert_eq!(CaseStatus::from_str("completed"), Some(CaseStatus::Resolved));
            assert_eq!(CaseStatus::from_str("cancelled"), Some(CaseStatus::Cancelled));
            assert_eq!(CaseStatus::from_str("canceled"), Some(CaseStatus::Cancelled));
            assert_eq!(CaseStatus::from_str("expired"), Some(CaseStatus::Expired));
            assert_eq!(CaseStatus::from_str("invalid"), None);
            assert_eq!(CaseStatus::from_str(""), None);
        }

        #[test]
        fn test_status_as_str() {
            assert_eq!(CaseStatus::Pending.as_str(), "pending");
            assert_eq!(CaseStatus::Claimed.as_str(), "claimed");
            assert_eq!(CaseStatus::EnRoute.as_str(), "en_route");
            assert_eq!(CaseStatus::Arrived.as_str(), "arrived");
            assert_eq!(CaseStatus::Resolved.as_str(), "resolved");
            assert_eq!(CaseStatus::Cancelled.as_str(), "cancelled");
            assert_eq!(CaseStatus::Expired.as_str(), "expired");
        }

        #[test]
        fn test_terminal_status() {
            assert!(!CaseStatus::Pending.is_terminal());
            assert!(!CaseStatus::Claimed.is_terminal());
            assert!(!CaseStatus::EnRoute.is_terminal());
            assert!(!CaseStatus::Arrived.is_terminal());
            assert!(CaseStatus::Resolved.is_terminal());
            assert!(CaseStatus::Cancelled.is_terminal());
            assert!(CaseStatus::Expired.is_terminal());
        }

        #[test]
        fn test_claimable_status() {
            assert!(CaseStatus::Pending.is_claimable());
            assert!(!CaseStatus::Claimed.is_claimable());
            assert!(!CaseStatus::EnRoute.is_claimable());
            assert!(!CaseStatus::Arrived.is_claimable());
            assert!(!CaseStatus::Resolved.is_claimable());
            assert!(!CaseStatus::Cancelled.is_claimable());
            assert!(!CaseStatus::Expired.is_claimable());
        }

        #[test]
        fn test_valid_transitions_from_pending() {
            let transitions = CaseStatus::Pending.valid_transitions();
            assert!(transitions.contains(&CaseStatus::Claimed));
            assert!(transitions.contains(&CaseStatus::Cancelled));
            assert!(transitions.contains(&CaseStatus::Expired));
            assert!(!transitions.contains(&CaseStatus::EnRoute));
            assert!(!transitions.contains(&CaseStatus::Resolved));
        }

        #[test]
        fn test_valid_transitions_from_claimed() {
            let transitions = CaseStatus::Claimed.valid_transitions();
            assert!(transitions.contains(&CaseStatus::EnRoute));
            assert!(transitions.contains(&CaseStatus::Cancelled));
            assert!(!transitions.contains(&CaseStatus::Pending));
            assert!(!transitions.contains(&CaseStatus::Resolved));
        }

        #[test]
        fn test_valid_transitions_from_en_route() {
            let transitions = CaseStatus::EnRoute.valid_transitions();
            assert!(transitions.contains(&CaseStatus::Arrived));
            assert!(transitions.contains(&CaseStatus::Cancelled));
            assert!(!transitions.contains(&CaseStatus::Pending));
            assert!(!transitions.contains(&CaseStatus::Claimed));
        }

        #[test]
        fn test_valid_transitions_from_arrived() {
            let transitions = CaseStatus::Arrived.valid_transitions();
            assert!(transitions.contains(&CaseStatus::Resolved));
            assert!(transitions.contains(&CaseStatus::Cancelled));
            assert!(!transitions.contains(&CaseStatus::EnRoute));
        }

        #[test]
        fn test_terminal_status_no_transitions() {
            assert!(CaseStatus::Resolved.valid_transitions().is_empty());
            assert!(CaseStatus::Cancelled.valid_transitions().is_empty());
            assert!(CaseStatus::Expired.valid_transitions().is_empty());
        }

        #[test]
        fn test_can_transition_to() {
            assert!(CaseStatus::Pending.can_transition_to(CaseStatus::Claimed));
            assert!(!CaseStatus::Pending.can_transition_to(CaseStatus::Resolved));
            assert!(CaseStatus::Claimed.can_transition_to(CaseStatus::EnRoute));
            assert!(!CaseStatus::Resolved.can_transition_to(CaseStatus::Pending));
        }

        #[test]
        fn test_validate_transition_success() {
            assert!(CaseStatus::Pending.validate_transition(CaseStatus::Claimed).is_ok());
            assert!(CaseStatus::Claimed.validate_transition(CaseStatus::EnRoute).is_ok());
            assert!(CaseStatus::EnRoute.validate_transition(CaseStatus::Arrived).is_ok());
            assert!(CaseStatus::Arrived.validate_transition(CaseStatus::Resolved).is_ok());
        }

        #[test]
        fn test_validate_transition_same_status() {
            assert!(matches!(
                CaseStatus::Pending.validate_transition(CaseStatus::Pending),
                Err(TransitionError::SameStatus)
            ));
        }

        #[test]
        fn test_validate_transition_from_terminal() {
            assert!(matches!(
                CaseStatus::Resolved.validate_transition(CaseStatus::Pending),
                Err(TransitionError::FromTerminalStatus { .. })
            ));
        }

        #[test]
        fn test_validate_transition_invalid() {
            assert!(matches!(
                CaseStatus::Pending.validate_transition(CaseStatus::Resolved),
                Err(TransitionError::InvalidTransition { .. })
            ));
        }
    }

    mod retry_tests {
        use super::*;

        #[test]
        fn test_calculate_retry_delay_exponential() {
            assert_eq!(calculate_retry_delay(0, 0), BASE_RETRY_DELAY_MS);
            assert_eq!(calculate_retry_delay(1, 0), BASE_RETRY_DELAY_MS * 2);
            assert_eq!(calculate_retry_delay(2, 0), BASE_RETRY_DELAY_MS * 4);
            assert_eq!(calculate_retry_delay(3, 0), BASE_RETRY_DELAY_MS * 8);
        }

        #[test]
        fn test_calculate_retry_delay_capped() {
            let delay = calculate_retry_delay(20, 0);
            assert!(delay <= MAX_RETRY_DELAY_MS);
        }

        #[test]
        fn test_calculate_retry_delay_with_jitter() {
            let delay = calculate_retry_delay(0, 500);
            assert_eq!(delay, BASE_RETRY_DELAY_MS + 500);
        }
    }

    mod outbox_tests {
        use super::*;

        #[test]
        fn test_outbox_entry_new() {
            let intent = OutboxIntent::SyncFcmToken {
                token: "test_token".into(),
            };
            let entry = OutboxEntry::new(intent);

            assert!(!entry.op_id.0.is_empty());
            assert!(!entry.idempotency_key.0.is_empty());
            assert_eq!(entry.retry_state, RetryState::Pending);
            assert_eq!(entry.attempt_count, 0);
            assert!(entry.last_attempt_at.is_none());
            assert!(entry.next_retry_at.is_none());
            assert!(entry.last_error.is_none());
        }

        #[test]
        fn test_outbox_entry_mark_in_flight() {
            let intent = OutboxIntent::SyncFcmToken {
                token: "test".into(),
            };
            let mut entry = OutboxEntry::new(intent);

            entry.mark_in_flight();

            assert_eq!(entry.retry_state, RetryState::InFlight);
            assert_eq!(entry.attempt_count, 1);
            assert!(entry.last_attempt_at.is_some());
        }

        #[test]
        fn test_outbox_entry_mark_completed() {
            let intent = OutboxIntent::SyncFcmToken {
                token: "test".into(),
            };
            let mut entry = OutboxEntry::new(intent);

            entry.mark_in_flight();
            entry.mark_completed();

            assert_eq!(entry.retry_state, RetryState::Completed);
            assert!(entry.is_completed());
            assert!(entry.last_error.is_none());
        }

        #[test]
        fn test_outbox_entry_mark_failed() {
            let intent = OutboxIntent::SyncFcmToken {
                token: "test".into(),
            };
            let mut entry = OutboxEntry::new(intent);

            entry.mark_in_flight();
            entry.mark_failed(OutboxEntryError::network_error("test error"));

            assert_eq!(entry.retry_state, RetryState::Failed);
            assert!(!entry.is_completed());
            assert!(!entry.is_permanently_failed());
            assert!(entry.last_error.is_some());
            assert!(entry.next_retry_at.is_some());
        }

        #[test]
        fn test_outbox_entry_permanent_failure_after_max_attempts() {
            let intent = OutboxIntent::SyncFcmToken {
                token: "test".into(),
            };
            let mut entry = OutboxEntry::new(intent);

            for _ in 0..MAX_RETRY_ATTEMPTS {
                entry.mark_in_flight();
                entry.mark_failed(OutboxEntryError::network_error("test error"));
            }

            assert_eq!(entry.retry_state, RetryState::PermanentlyFailed);
            assert!(entry.is_permanently_failed());
        }

        #[test]
        fn test_outbox_entry_rate_limited() {
            let intent = OutboxIntent::SyncFcmToken {
                token: "test".into(),
            };
            let mut entry = OutboxEntry::new(intent);

            entry.mark_rate_limited(60_000);

            assert_eq!(entry.retry_state, RetryState::RateLimited);
            assert!(entry.next_retry_at.is_some());
        }

        #[test]
        fn test_outbox_entry_is_ready_for_retry() {
            let intent = OutboxIntent::SyncFcmToken {
                token: "test".into(),
            };
            let mut entry = OutboxEntry::new(intent);

            assert!(entry.is_ready_for_retry(0));

            entry.mark_in_flight();
            assert!(!entry.is_ready_for_retry(0));

            entry.mark_completed();
            assert!(!entry.is_ready_for_retry(0));
        }

        #[test]
        fn test_offline_store_push_outbox() {
            let mut store = OfflineStore::new();

            let intent = OutboxIntent::SyncFcmToken {
                token: "test".into(),
            };
            let entry = OutboxEntry::new(intent);
            let op_id = entry.op_id.clone();

            assert!(store.push_outbox(entry).is_ok());
            assert_eq!(store.outbox.len(), 1);

            let duplicate_entry = OutboxEntry {
                op_id: op_id.clone(),
                idempotency_key: IdempotencyKey::generate(),
                intent: OutboxIntent::SyncFcmToken {
                    token: "test2".into(),
                },
                created_at: UnixTimeMs::now(),
                updated_at: UnixTimeMs::now(),
                retry_state: RetryState::Pending,
                attempt_count: 0,
                last_attempt_at: None,
                next_retry_at: None,
                last_error: None,
            };
            assert!(matches!(
                store.push_outbox(duplicate_entry),
                Err(OutboxError::DuplicateOpId(_))
            ));
        }

        #[test]
        fn test_offline_store_pending_count() {
            let mut store = OfflineStore::new();

            assert_eq!(store.pending_sync_count(), 0);

            let intent = OutboxIntent::SyncFcmToken {
                token: "test".into(),
            };
            store.push_outbox(OutboxEntry::new(intent)).unwrap();

            assert_eq!(store.pending_sync_count(), 1);

            store.outbox[0].mark_completed();
            assert_eq!(store.pending_sync_count(), 0);
        }
    }

    mod error_tests {
        use super::*;

        #[test]
        fn test_app_error_new() {
            let error = AppError::new(ErrorKind::Network, "Test error");

            assert_eq!(error.kind, ErrorKind::Network);
            assert_eq!(error.severity, ErrorSeverity::Transient);
            assert_eq!(error.message, "Test error");
            assert!(error.internal_message.is_none());
            assert!(error.is_retryable());
        }

        #[test]
        fn test_app_error_with_internal() {
            let error = AppError::new(ErrorKind::Network, "User message")
                .with_internal("Internal details");

            assert_eq!(error.internal_message, Some("Internal details".into()));
        }

        #[test]
        fn test_app_error_with_context() {
            let error = AppError::new(ErrorKind::Validation, "Invalid input")
                .with_context("field", "email")
                .with_context("value", "invalid@");

            assert_eq!(error.context.get("field"), Some(&"email".to_string()));
            assert_eq!(error.context.get("value"), Some(&"invalid@".to_string()));
        }

        #[test]
        fn test_app_error_from_http_status() {
            assert_eq!(
                AppError::from_http_status(400, None).kind,
                ErrorKind::Validation
            );
            assert_eq!(
                AppError::from_http_status(401, None).kind,
                ErrorKind::Authentication
            );
            assert_eq!(
                AppError::from_http_status(403, None).kind,
                ErrorKind::Authorization
            );
            assert_eq!(
                AppError::from_http_status(404, None).kind,
                ErrorKind::NotFound
            );
            assert_eq!(
                AppError::from_http_status(409, None).kind,
                ErrorKind::Conflict
            );
            assert_eq!(
                AppError::from_http_status(429, None).kind,
                ErrorKind::RateLimited
            );
            assert_eq!(
                AppError::from_http_status(500, None).kind,
                ErrorKind::Internal
            );
        }

        #[test]
        fn test_error_kind_retryable() {
            assert!(ErrorKind::Network.is_retryable());
            assert!(ErrorKind::Timeout.is_retryable());
            assert!(ErrorKind::RateLimited.is_retryable());
            assert!(ErrorKind::Conflict.is_retryable());
            assert!(!ErrorKind::Authentication.is_retryable());
            assert!(!ErrorKind::Validation.is_retryable());
            assert!(!ErrorKind::Internal.is_retryable());
        }

        #[test]
        fn test_user_facing_message() {
            let network_error = AppError::new(ErrorKind::Network, "Connection failed");
            assert!(network_error.user_facing_message().contains("internet"));

            let validation_error = AppError::new(ErrorKind::Validation, "Email is invalid");
            assert_eq!(validation_error.user_facing_message(), "Email is invalid");

            let internal_error = AppError::new(ErrorKind::Internal, "Database error");
            assert!(internal_error.user_facing_message().contains("unexpected"));
        }
    }

    mod local_case_tests {
        use super::*;

        #[test]
        fn test_local_case_new() {
            let location = LatLon::new(51.5074, -0.1278);
            let case = LocalCase::new(location, Some("Test description".into()), Some(3));

            assert!(!case.local_id.0.is_empty());
            assert_eq!(case.location, location);
            assert_eq!(case.description, Some("Test description".into()));
            assert_eq!(case.wound_severity, Some(3));
            assert_eq!(case.status, LocalCaseStatus::PendingUpload);
            assert!(case.server_id.is_none());
            assert!(case.photo_data.is_none());
        }

        #[test]
        fn test_local_case_mark_synced() {
            let mut case = LocalCase::new(LatLon::new(0.0, 0.0), None, None);
            case.photo_data = Some(vec![1, 2, 3]);

            case.mark_synced(CaseId::new("server123"));

            assert_eq!(case.status, LocalCaseStatus::Synced);
            assert_eq!(case.server_id, Some(CaseId::new("server123")));
            assert!(case.sync_error.is_none());
            assert!(case.photo_data.is_none());
        }

        #[test]
        fn test_local_case_mark_failed() {
            let mut case = LocalCase::new(LatLon::new(0.0, 0.0), None, None);

            case.mark_failed("Connection timeout");

            assert_eq!(case.status, LocalCaseStatus::Failed);
            assert_eq!(case.sync_error, Some("Connection timeout".into()));
            assert_eq!(case.retry_count, 1);
        }

        #[test]
        fn test_local_case_permanent_failure() {
            let mut case = LocalCase::new(LatLon::new(0.0, 0.0), None, None);

            for _ in 0..=MAX_RETRY_ATTEMPTS {
                case.mark_failed("Error");
            }

            assert_eq!(case.status, LocalCaseStatus::PermanentlyFailed);
        }

        #[test]
        fn test_local_case_description_preview() {
            let case = LocalCase::new(
                LatLon::new(0.0, 0.0),
                Some("This is a very long description that should be truncated".into()),
                None,
            );

            let preview = case.description_preview(20);
            assert_eq!(preview.len(), 20);
            assert!(preview.ends_with("..."));
        }

        #[test]
        fn test_local_case_description_preview_short() {
            let case = LocalCase::new(LatLon::new(0.0, 0.0), Some("Short".into()), None);

            let preview = case.description_preview(20);
            assert_eq!(preview, "Short");
        }
    }

    mod model_tests {
        use super::*;

        #[test]
        fn test_model_default() {
            let model = Model::default();

            assert_eq!(model.state, AppState::Loading);
            assert!(model.user_id.is_none());
            assert!(model.area_center.is_none());
            assert_eq!(model.area_radius_m, DEFAULT_RADIUS_M);
            assert_eq!(model.map_zoom, DEFAULT_MAP_ZOOM);
            assert!(model.cases.is_empty());
            assert!(model.network_online);
            assert!(!model.is_refreshing);
        }

        #[test]
        fn test_model_is_authenticated() {
            let mut model = Model::default();

            assert!(!model.is_authenticated());

            model.user_id = Some(UserId::new("user123"));
            assert!(model.is_authenticated());
        }

        #[test]
        fn test_model_show_toast() {
            let mut model = Model::default();

            model.show_toast("Test message", ToastKind::Success);

            assert!(model.active_toast.is_some());
            let toast = model.active_toast.as_ref().unwrap();
            assert_eq!(toast.message, "Test message");
            assert_eq!(toast.kind, ToastKind::Success);
        }

        #[test]
        fn test_model_set_clear_error() {
            let mut model = Model::default();

            model.set_error(AppError::new(ErrorKind::Network, "Test error"));
            assert!(model.active_error.is_some());

            model.clear_error();
            assert!(model.active_error.is_none());
        }

        #[test]
        fn test_model_optimistic_mutation() {
            let mut model = Model::default();
            model.user_id = Some(UserId::new("user123"));

            let case_id = CaseId::new("case123");

            model.cases.push(ServerCase {
                id: case_id.clone(),
                location: LatLon::new(0.0, 0.0),
                description: None,
                landmark_hint: None,
                wound_severity: None,
                status: CaseStatus::Pending,
                created_at_ms_utc: UnixTimeMs::now(),
                updated_at_ms_utc: UnixTimeMs::now(),
                reporter_id: UserId::new("other"),
                assigned_rescuer_id: None,
                photo_url: None,
                thumbnail_url: None,
                gemini_diagnosis: None,
                species_guess: None,
                distance_meters: None,
            });

            let mutation_id = model.store_optimistic_mutation(
                case_id.clone(),
                CaseStatus::Pending,
                None,
                CaseStatus::Claimed,
            );

            assert!(model.pending_mutations.contains_key(&mutation_id));

            if let Some(case) = model.cases.iter_mut().find(|c| c.id == case_id) {
                case.status = CaseStatus::Claimed;
                case.assigned_rescuer_id = model.user_id.clone();
            }

            let rolled_back = model.rollback_mutation(&mutation_id);
            assert!(rolled_back);

            let case = model.cases.iter().find(|c| c.id == case_id).unwrap();
            assert_eq!(case.status, CaseStatus::Pending);
            assert!(case.assigned_rescuer_id.is_none());
        }
    }

    mod zoom_tests {
        use super::*;

        #[test]
        fn test_zoom_for_radius() {
            assert_eq!(zoom_for_radius(500), 16.0);
            assert_eq!(zoom_for_radius(1000), 16.0);
            assert_eq!(zoom_for_radius(2000), 15.0);
            assert_eq!(zoom_for_radius(5000), 14.0);
            assert_eq!(zoom_for_radius(10000), 13.0);
            assert_eq!(zoom_for_radius(20000), 12.0);
            assert_eq!(zoom_for_radius(50000), 11.0);
            assert_eq!(zoom_for_radius(100000), FALLBACK_ZOOM);
        }
    }

    mod event_tests {
        use super::*;

        #[test]
        fn test_event_default() {
            let event = Event::default();
            assert!(matches!(event, Event::Noop));
        }

        #[test]
        fn test_event_name() {
            assert_eq!(Event::Noop.name(), "noop");
            assert_eq!(Event::AppStarted.name(), "app_started");
            assert_eq!(Event::LoginRequested.name(), "login_requested");
            assert_eq!(Event::RefreshRequested.name(), "refresh_requested");
        }

        #[test]
        fn test_event_is_user_initiated() {
            assert!(!Event::Noop.is_user_initiated());
            assert!(!Event::AppStarted.is_user_initiated());
            assert!(Event::LoginRequested.is_user_initiated());
            assert!(Event::RefreshRequested.is_user_initiated());
            assert!(Event::CapturePhotoRequested.is_user_initiated());
            assert!(Event::ClaimRequested {
                case_id: "test".into()
            }
            .is_user_initiated());
        }
    }

    mod toast_tests {
        use super::*;

        #[test]
        fn test_toast_message_new() {
            let toast = ToastMessage::new("Test message", ToastKind::Info);

            assert_eq!(toast.message, "Test message");
            assert_eq!(toast.kind, ToastKind::Info);
            assert_eq!(toast.duration_ms, 3000);
        }

        #[test]
        fn test_toast_kind_duration() {
            assert_eq!(ToastKind::Info.default_duration_ms(), 3000);
            assert_eq!(ToastKind::Success.default_duration_ms(), 2000);
            assert_eq!(ToastKind::Warning.default_duration_ms(), 4000);
            assert_eq!(ToastKind::Error.default_duration_ms(), 5000);
        }

        #[test]
        fn test_toast_is_expired() {
            let toast = ToastMessage::new("Test", ToastKind::Info);
            let created = toast.created_at_ms;

            assert!(!toast.is_expired(created));
            assert!(!toast.is_expired(created + 2999));
            assert!(toast.is_expired(created + 3001));
        }
    }

    mod permission_state_tests {
        use super::*;

        #[test]
        fn test_permission_state() {
            assert!(PermissionState::Unknown.is_unknown());
            assert!(!PermissionState::Unknown.is_granted());
            assert!(!PermissionState::Unknown.is_denied());

            assert!(PermissionState::Granted.is_granted());
            assert!(!PermissionState::Granted.is_unknown());
            assert!(!PermissionState::Granted.is_denied());

            assert!(PermissionState::Denied.is_denied());
            assert!(!PermissionState::Denied.is_granted());
            assert!(!PermissionState::Denied.is_unknown());

            assert!(PermissionState::Restricted.is_denied());
        }
    }

    mod feed_view_tests {
        use super::*;

        #[test]
        fn test_feed_view_toggle() {
            assert_eq!(FeedView::Map.toggle(), FeedView::List);
            assert_eq!(FeedView::List.toggle(), FeedView::Map);
        }

        #[test]
        fn test_feed_view_as_str() {
            assert_eq!(FeedView::Map.as_str(), "map");
            assert_eq!(FeedView::List.as_str(), "list");
        }
    }

    mod id_tests {
        use super::*;

        #[test]
        fn test_user_id() {
            let id = UserId::new("user123");
            assert_eq!(id.as_str(), "user123");
            assert_eq!(id.to_string(), "user123");
        }

        #[test]
        fn test_case_id_generate() {
            let id1 = CaseId::generate();
            let id2 = CaseId::generate();
            assert_ne!(id1.0, id2.0);
            assert!(!id1.0.is_empty());
        }

        #[test]
        fn test_local_op_id_generate() {
            let id1 = LocalOpId::generate();
            let id2 = LocalOpId::generate();
            assert_ne!(id1.0, id2.0);
        }

        #[test]
        fn test_idempotency_key_generate() {
            let key1 = IdempotencyKey::generate();
            let key2 = IdempotencyKey::generate();
            assert_ne!(key1.0, key2.0);
        }

        #[test]
        fn test_op_id_generate() {
            let id1 = OpId::generate();
            let id2 = OpId::generate();
            assert_ne!(id1.0, id2.0);
        }
    }

    mod unix_time_tests {
        use super::*;

        #[test]
        fn test_unix_time_now() {
            let time1 = UnixTimeMs::now();
            std::thread::sleep(std::time::Duration::from_millis(10));
            let time2 = UnixTimeMs::now();

            assert!(time2.0 > time1.0);
        }

        #[test]
        fn test_unix_time_operations() {
            let time = UnixTimeMs(1000);

            assert_eq!(time.as_millis(), 1000);
            assert_eq!(time.as_secs(), 1);
            assert_eq!(time.add_millis(500).0, 1500);
        }

        #[test]
        fn test_unix_time_comparison() {
            let earlier = UnixTimeMs(1000);
            let later = UnixTimeMs(2000);

            assert!(earlier.is_before(later));
            assert!(later.is_after(earlier));
            assert_eq!(later.elapsed_since(earlier), 1000);
        }
    }

    mod lat_lon_tests {
        use super::*;

        #[test]
        fn test_lat_lon_new() {
            let loc = LatLon::new(51.5074, -0.1278);
            assert_eq!(loc.lat, 51.5074);
            assert_eq!(loc.lon, -0.1278);
        }

        #[test]
        fn test_lat_lon_validate() {
            let valid = LatLon::new(51.5074, -0.1278);
            assert!(valid.validate().is_ok());

            let invalid = LatLon::new(91.0, 0.0);
            assert!(invalid.validate().is_err());
        }

        #[test]
        fn test_lat_lon_conversion() {
            let coord = ValidatedCoordinate::new(51.5074, -0.1278).unwrap();
            let lat_lon: LatLon = coord.into();

            assert_eq!(lat_lon.lat, 51.5074);
            assert_eq!(lat_lon.lon, -0.1278);
        }
    }
}
