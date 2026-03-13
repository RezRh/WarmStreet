use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, fmt};
use chrono::{DateTime, Utc};
use geo::{Coord, HaversineDistance, Point};

use crate::offline_store::OfflineStore;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CaseId(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LocalOpId(pub String);

/// Use geo::Coord directly - no custom wrapper needed
pub type LatLon = Coord<f64>;

/// Extension trait for LatLon to add domain-specific methods
pub trait LatLonExt {
    fn haversine_distance(&self, other: &Self) -> f64;
    fn to_point(&self) -> Point<f64>;
    fn from_point(point: &Point<f64>) -> Self;
}

impl LatLonExt for LatLon {
    fn haversine_distance(&self, other: &Self) -> f64 {
        self.haversine_distance(other)
    }

    fn to_point(&self) -> Point<f64> {
        Point::new(self.x, self.y)
    }

    fn from_point(point: &Point<f64>) -> Self {
        Coord { x: point.x(), y: point.y() }
    }
}

/// Use chrono::DateTime<Utc> directly - no custom wrapper needed
pub type UnixTimeMs = DateTime<Utc>;

/// Extension trait for UnixTimeMs to add domain-specific methods
pub trait UnixTimeMsExt {
    fn now() -> Self;
    fn to_millis(&self) -> i64;
    fn from_millis(millis: i64) -> Self;
    fn to_humanized(&self) -> String;
}

impl UnixTimeMsExt for UnixTimeMs {
    fn now() -> Self {
        Utc::now()
    }

    fn to_millis(&self) -> i64 {
        self.timestamp_millis()
    }

    fn from_millis(millis: i64) -> Self {
        DateTime::from_timestamp_millis(millis).unwrap_or_else(|| Utc::now())
    }

    fn to_humanized(&self) -> String {
        use chrono_humanize::HumanTime;
        HumanTime::from(*self).to_string()
    }
}

#[derive(Default, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum FeedView {
    #[default]
    Map,
    List,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum AppState {
    Loading,
    Unauthenticated,
    Authenticating,
    OnboardingLocation,
    PinDrop,
    OnboardingRadius,
    CameraCapture,
    Ready,
}

impl Default for AppState {
    fn default() -> Self { Self::Unauthenticated }
}

/// Use single status enum - LocalCaseStatus is sufficient
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum LocalCaseStatus {
    PendingUpload,
    Uploading,
    Synced,
    Failed,
}

impl LocalCaseStatus {
    #[must_use]
    pub const fn is_pending(&self) -> bool {
        matches!(self, Self::PendingUpload | Self::Uploading)
    }

    #[must_use]
    pub const fn is_synced(&self) -> bool {
        matches!(self, Self::Synced)
    }

    #[must_use]
    pub const fn is_failed(&self) -> bool {
        matches!(self, Self::Failed)
    }
}

/// Use single status enum - ServerCaseStatus is sufficient
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServerCaseStatus {
    Open,
    Assigned,
    Resolved,
    Closed,
    Unknown,
}

impl ServerCaseStatus {
    #[must_use]
    pub const fn is_claimable(&self) -> bool {
        matches!(self, Self::Open)
    }

    pub fn valid_transitions(&self) -> Vec<&'static str> {
        match self {
            Self::Open => vec!["assigned", "resolved"],
            Self::Assigned => vec!["resolved", "closed"],
            Self::Resolved => vec!["closed"],
            Self::Closed => vec![],
            Self::Unknown => vec![],
        }
    }

    pub fn validate_transition(&self, next: ServerCaseStatus) -> Result<(), &'static str> {
        let valid = self.valid_transitions();
        let next_str = match next {
            ServerCaseStatus::Open => "open",
            ServerCaseStatus::Assigned => "assigned",
            ServerCaseStatus::Resolved => "resolved",
            ServerCaseStatus::Closed => "closed",
            ServerCaseStatus::Unknown => "unknown",
        };
        if valid.contains(&next_str) {
            Ok(())
        } else {
            Err("Invalid status transition")
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Assigned => "assigned",
            Self::Resolved => "resolved",
            Self::Closed => "closed",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "open" => Some(Self::Open),
            "assigned" => Some(Self::Assigned),
            "resolved" => Some(Self::Resolved),
            "closed" => Some(Self::Closed),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }
}

/// Don't store image bytes. Store a handle/URI/path.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct BlobRef {
    pub uri: String,
    pub size_bytes: Option<u64>,
    pub sha256_hex: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LocalCase {
    pub local_id: LocalOpId,
    pub location: LatLon,
    pub description: Option<String>,
    pub wound_severity: Option<u8>,
    pub status: LocalCaseStatus,
    pub created_at_ms_utc: UnixTimeMs,
    pub photo: Option<BlobRef>,
    pub photo_upload_url: Option<String>,
}

impl LocalCase {
    pub fn new(location: LatLon, description: Option<String>, wound_severity: Option<u8>) -> Self {
        Self {
            local_id: LocalOpId(uuid::Uuid::new_v4().to_string()),
            location,
            description,
            wound_severity,
            status: LocalCaseStatus::PendingUpload,
            created_at_ms_utc: UnixTimeMs::now(),
            photo: None,
            photo_upload_url: None,
        }
    }

    pub fn mark_failed(&mut self, _error: String) {
        self.status = LocalCaseStatus::Failed;
    }
}

impl fmt::Debug for LocalCase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalCase")
            .field("local_id", &self.local_id)
            .field("location", &self.location)
            .field("description_present", &self.description.as_ref().map(|_| true))
            .field("wound_severity", &self.wound_severity)
            .field("status", &self.status)
            .field("created_at_ms_utc", &self.created_at_ms_utc)
            .field("photo_present", &self.photo.as_ref().map(|_| true))
            .field("photo_upload_url_present", &self.photo_upload_url.as_ref().map(|_| true))
            .finish()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ServerCase {
    pub id: CaseId,
    pub location: LatLon,
    pub description: Option<String>,
    pub wound_severity: Option<u8>,
    pub status: ServerCaseStatus,
    pub created_at_ms_utc: UnixTimeMs,
    pub reporter_id: UserId,
    pub assigned_rescuer_id: Option<UserId>,
    pub photo_url: Option<String>,
    pub crop_url: Option<String>,
    pub gemini_diagnosis: Option<String>,
}

impl ServerCase {
    pub fn description_preview(&self, max_len: usize) -> String {
        self.description
            .as_ref()
            .map(|d| {
                if d.len() > max_len {
                    format!("{}...", &d[..max_len])
                } else {
                    d.clone()
                }
            })
            .unwrap_or_default()
    }
}

/// Runtime-only secrets: do NOT Serialize/Deserialize.
#[derive(Default)]
pub struct RuntimeSecrets {
    pub jwt: Option<secrecy::SecretString>,
    pub fcm_token: Option<secrecy::SecretString>,
}

/// Persisted model
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Model {
    pub state: AppState,
    pub user_id: Option<UserId>,
    pub area_center: Option<LatLon>,
    pub area_radius_m: u32,
    pub feed_view: FeedView,
    pub selected_case_id: Option<CaseId>,
    pub map_center: Option<LatLon>,
    pub map_zoom: f64,
    pub is_refreshing: bool,
    pub staged_photo: Option<BlobRef>,
    pub staged_crop: Option<BlobRef>,
    pub detection_count: usize,
    pub top_confidence: f32,
    pub yolo_model: Option<BlobRef>,
    pub network_online: bool,
    pub offline_store: OfflineStore,
    pub cases: Vec<ServerCase>,
    pub is_loading: bool,
    pub active_error: Option<crate::lib::AppError>,
    pub active_toast: Option<String>,
    pub push_permission_granted: bool,
}

impl Model {
    pub fn new() -> Self {
        Self {
            state: AppState::Unauthenticated,
            user_id: None,
            area_center: None,
            area_radius_m: 1_000,
            feed_view: FeedView::Map,
            selected_case_id: None,
            map_center: None,
            map_zoom: 12.0,
            is_refreshing: false,
            staged_photo: None,
            staged_crop: None,
            detection_count: 0,
            top_confidence: 0.0,
            yolo_model: None,
            network_online: true,
            offline_store: OfflineStore::new(),
            cases: Vec::new(),
            is_loading: false,
            active_error: None,
            active_toast: None,
            push_permission_granted: false,
        }
    }
}

// ============================================================================
// Vision Types (moved from vision.rs for centralization)
// ============================================================================

/// Bounding box detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Detection {
    /// Bounding box [x1, y1, x2, y2] normalized to original image (0.0..1.0)
    pub bbox: [f32; 4],
    /// Detection confidence score (0.0..1.0)
    pub confidence: f32,
    /// Class ID from model
    pub class_id: u32,
}

/// Normalized bounding box for general use - use array directly instead
pub type NormalizedBbox = [f32; 4];

/// Detection result with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionResult {
    pub detections: Vec<Detection>,
    pub truncated: bool,
    pub candidates_before_nms: usize,
    pub preprocess_ms: f64,
    pub inference_ms: f64,
    pub postprocess_ms: f64,
}
