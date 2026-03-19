// types.rs - Shared types without Crux dependencies
// These types are used by the Tauri backend and shared with the frontend

#![allow(clippy::module_name_repetitions)]
#![allow(clippy::too_many_lines)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

// ============================================================================
// Constants
// ============================================================================

pub const DEFAULT_RADIUS_M: u32 = 5000;
pub const MIN_RADIUS_M: u32 = 500;
pub const MAX_RADIUS_M: u32 = 50000;
pub const DEFAULT_MAP_ZOOM: f64 = 14.0;
pub const MIN_ZOOM: f64 = 5.0;
pub const MAX_ZOOM: f64 = 20.0;
pub const FALLBACK_ZOOM: f64 = 10.0;
pub const EARTH_RADIUS_M: f64 = 6_371_000.0;
pub const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;
pub const MAX_IMAGE_DIMENSION: u32 = 4096;
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

// ============================================================================
// Error Types
// ============================================================================

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
    pub fn user_facing_message(&self) -> String {
        match self.kind {
            ErrorKind::Network => "Unable to connect. Please check your internet connection.".into(),
            ErrorKind::Timeout => "The request timed out. Please try again.".into(),
            ErrorKind::Authentication => "Your session has expired. Please sign in again.".into(),
            ErrorKind::Authorization => "You don't have permission to perform this action.".into(),
            ErrorKind::Validation => self.message.clone(),
            ErrorKind::NotFound => "The requested item could not be found.".into(),
            ErrorKind::Conflict => "This action conflicts with a recent change. Please refresh.".into(),
            ErrorKind::RateLimited => "Too many requests. Please wait a moment.".into(),
            ErrorKind::QuotaExceeded => "You've reached your usage limit.".into(),
            ErrorKind::Storage => "Unable to save data locally.".into(),
            ErrorKind::Serialization | ErrorKind::Deserialization => "A data error occurred.".into(),
            ErrorKind::ImageProcessing => "Unable to process the image.".into(),
            ErrorKind::ImageTooLarge => "The image is too large.".into(),
            ErrorKind::ImageDimensionsTooLarge => "Image dimensions are too large.".into(),
            ErrorKind::ImageFormatUnsupported => "This image format is not supported.".into(),
            ErrorKind::Camera => "Camera error.".into(),
            ErrorKind::CameraPermissionDenied => "Camera permission denied.".into(),
            ErrorKind::Location => "Unable to determine location.".into(),
            ErrorKind::LocationPermissionDenied => "Location permission denied.".into(),
            ErrorKind::Crypto | ErrorKind::CryptoKeyNotFound => "A security error occurred.".into(),
            ErrorKind::FeatureUnavailable => self.message.clone(),
            ErrorKind::InvalidState => "Invalid state. Please restart.".into(),
            ErrorKind::Internal | ErrorKind::Unknown => "An unexpected error occurred.".into(),
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.kind.code(), self.message)
    }
}

impl std::error::Error for AppError {}

// ============================================================================
// Coordinate Types
// ============================================================================

/// Simple coordinate type that serializes to { lat, lon }
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Coordinate {
    pub lat: f64,
    pub lon: f64,
}

impl Default for Coordinate {
    fn default() -> Self {
        Self { lat: 0.0, lon: 0.0 }
    }
}

pub type LatLon = Coordinate;

#[must_use]
pub fn haversine_distance(p1: Coordinate, p2: Coordinate) -> f64 {
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
    } else {
        format!("{:.0} km", meters / 1000.0)
    }
}

#[must_use]
pub fn format_time_ago(timestamp_ms: u64, now_ms: u64) -> String {
    if timestamp_ms > now_ms {
        return "Just now".into();
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

#[must_use]
pub const fn severity_label(score: u8) -> &'static str {
    match score {
        1..=2 => "Low",
        3 => "Moderate",
        4 => "High",
        _ => "Critical",
    }
}

// ============================================================================
// Case Status Types
// ============================================================================

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
}

// ============================================================================
// User Profile
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub name: String,
    pub email: String,
    pub phone: Option<String>,
    pub member_type: String,
    pub karma: i32,
    pub rescues: i32,
    pub verification_level: String,
}

// ============================================================================
// Case Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Case {
    pub id: String,
    pub location: LatLon,
    pub description: String,
    pub status: CaseStatus,
    pub severity: u8,
    #[serde(rename = "type")]
    pub case_type: String,
    pub age: Option<String>,
    pub breed: Option<String>,
    pub image_url: Option<String>,
    pub created_at: UnixTimeMs,
    pub reporter_id: Option<String>,
    pub assigned_rescuer_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapPin {
    pub case_id: String,
    pub lat: f64,
    pub lon: f64,
    pub severity: u8,
    pub status: CaseStatus,
}

// ============================================================================
// App State
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum FeedView {
    #[default]
    Map,
    List,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum AppState {
    #[default]
    Loading,
    Unauthenticated,
    Authenticating,
    Ready,
    MfaVerification,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewModel {
    pub status: AppState,
    pub feed_view: FeedView,
    pub cases: Vec<Case>,
    pub map_pins: Vec<MapPin>,
    pub selected_case: Option<Case>,
    pub is_refreshing: bool,
    pub error: Option<String>,
    pub toast: Option<String>,
    pub profile: Option<UserProfile>,
    pub community_members: Vec<CommunityMember>,
    pub is_loading_community: bool,
    pub active_chat_member: Option<CommunityMember>,
}

impl Default for ViewModel {
    fn default() -> Self {
        Self {
            status: AppState::Loading,
            feed_view: FeedView::Map,
            cases: Vec::new(),
            map_pins: Vec::new(),
            selected_case: None,
            is_refreshing: false,
            error: None,
            toast: None,
            profile: None,
            community_members: Vec::new(),
            is_loading_community: false,
            active_chat_member: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityMember {
    pub id: String,
    pub name: String,
    pub member_type: String,
    pub karma: i32,
    pub last_active: String,
}

// ============================================================================
// Type Aliases
// ============================================================================

pub type UnixTimeMs = DateTime<Utc>;

// ============================================================================
// Image Processing Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Detection {
    pub bbox: [f32; 4],
    pub confidence: f32,
    pub class_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionResult {
    pub detections: Vec<Detection>,
    pub truncated: bool,
    pub candidates_before_nms: usize,
    pub preprocess_ms: f64,
    pub inference_ms: f64,
    pub postprocess_ms: f64,
}
