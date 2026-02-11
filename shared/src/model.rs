use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, fmt};

use crate::offline_store::OfflineStore;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CaseId(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LocalOpId(pub String);

/// Validated lat/lon
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LatLon {
    pub lat: f64,
    pub lon: f64,
}

impl LatLon {
    pub fn new(lat: f64, lon: f64) -> Option<Self> {
        if !lat.is_finite() || !lon.is_finite() { return None; }
        if !(-90.0..=90.0).contains(&lat) { return None; }
        if !(-180.0..=180.0).contains(&lon) { return None; }
        Some(Self { lat, lon })
    }
}

/// Explicit timestamp unit.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnixTimeMs(pub u64);

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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum LocalCaseStatus {
    PendingUpload,
    Uploading,
    Synced,
    Failed,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServerCaseStatus {
    Open,
    Assigned,
    Resolved,
    Closed,
    Unknown,
}

/// Don’t store image bytes. Store a handle/URI/path.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct BlobRef {
    pub uri: String,
    pub size_bytes: Option<u64>,
    pub sha256_hex: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LocalCase {
    pub local_id: LocalOpId,            // matches op_id/idempotency key
    pub location: LatLon,
    pub description: Option<String>,
    pub wound_severity: Option<u8>,     // constrain further if you know the scale
    pub status: LocalCaseStatus,
    pub created_at_ms_utc: UnixTimeMs,

    /// Transient-ish references; not the raw bytes.
    pub photo: Option<BlobRef>,
    pub photo_upload_url: Option<String>,
}

// Redact debug output because this can contain sensitive user-provided data.
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
    pub gemini_diagnosis: Option<String>, // consider redaction/logging policy
}

/// Runtime-only secrets: do NOT Serialize/Deserialize.
/// Store in keystore; load into memory as needed.
#[derive(Default)]
pub struct RuntimeSecrets {
    pub jwt: Option<secrecy::SecretString>,
    pub fcm_token: Option<secrecy::SecretString>,
}

/// Persisted model: safe-ish to serialize.
/// WARNING: still contains sensitive user content; apply encryption-at-rest if required.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Model {
    pub state: AppState,

    pub user_id: Option<UserId>,

    pub area_center: Option<LatLon>,
    pub area_radius_m: u32,

    // Feed
    pub feed_view: FeedView,
    pub selected_case_id: Option<CaseId>,
    pub map_center: Option<LatLon>,
    pub map_zoom: f64,
    pub is_refreshing: bool,

    // Photo staging (references, not bytes)
    pub staged_photo: Option<BlobRef>,
    pub staged_crop: Option<BlobRef>,
    pub detection_count: usize,
    pub top_confidence: f32,

    // YOLO model: do not keep bytes here; keep a ref/version.
    pub yolo_model: Option<BlobRef>,

    // Offline-first
    pub network_online: bool,
    pub offline_store: OfflineStore,
    pub cases: Vec<ServerCase>,

    // Push (non-secret flag only; token lives in RuntimeSecrets/keystore)
    // Generic UI state
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
            area_radius_m: 1_000, // sane default, not 0
            feed_view: FeedView::Map,
            selected_case_id: None,
            map_center: None,
            map_zoom: 12.0, // sane default
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