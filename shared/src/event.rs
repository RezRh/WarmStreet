use serde::{Deserialize, Serialize};
use std::fmt;

// --- Secret wrapper: redacts Debug, zeroizes on Drop ---

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct Secret(String);

impl Secret {
    pub fn new(s: String) -> Self {
        Self(s)
    }
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        // Best-effort zeroization; use `secrecy` crate in prod for guaranteed behavior.
        unsafe {
            let bytes = self.0.as_bytes_mut();
            std::ptr::write_bytes(bytes.as_mut_ptr(), 0, bytes.len());
        }
    }
}

// --- Typed IDs ---

macro_rules! typed_id {
    ($name:ident) => {
        #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
        pub struct $name(pub String);

        impl $name {
            pub fn new(s: impl Into<String>) -> Self {
                Self(s.into())
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

typed_id!(CaseId);
typed_id!(UserId);
typed_id!(OpId);
typed_id!(LocalId);

// --- Coordinate: validated, NaN-safe ---

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Coordinate {
    lat: f64,
    lng: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("invalid coordinate: lat={0}, lng={1}")]
    InvalidCoordinate(f64, f64),
    #[error("invalid zoom: {0}")]
    InvalidZoom(f64),
    #[error("invalid wound severity: {0}")]
    InvalidWoundSeverity(i32),
    #[error("value too long ({len} > {max})")]
    TooLong { len: usize, max: usize },
    #[error("invalid url: {0}")]
    InvalidUrl(String),
}

impl Coordinate {
    pub fn new(lat: f64, lng: f64) -> Result<Self, ValidationError> {
        if lat.is_nan()
            || lng.is_nan()
            || lat.is_infinite()
            || lng.is_infinite()
            || !(-90.0..=90.0).contains(&lat)
            || !(-180.0..=180.0).contains(&lng)
        {
            return Err(ValidationError::InvalidCoordinate(lat, lng));
        }
        Ok(Self { lat, lng })
    }

    pub fn lat(&self) -> f64 {
        self.lat
    }
    pub fn lng(&self) -> f64 {
        self.lng
    }
}

impl PartialEq for Coordinate {
    fn eq(&self, other: &Self) -> bool {
        self.lat.to_bits() == other.lat.to_bits() && self.lng.to_bits() == other.lng.to_bits()
    }
}

impl Eq for Coordinate {}

// --- Zoom: validated ---

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Zoom(f64);

impl Zoom {
    pub fn new(value: f64) -> Result<Self, ValidationError> {
        if value.is_nan() || value.is_infinite() || !(0.0..=25.0).contains(&value) {
            return Err(ValidationError::InvalidZoom(value));
        }
        Ok(Self(value))
    }
    pub fn value(&self) -> f64 {
        self.0
    }
}

// --- Bounded description ---

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct BoundedText<const MAX: usize>(String);

impl<const MAX: usize> BoundedText<MAX> {
    pub fn new(s: impl Into<String>) -> Result<Self, ValidationError> {
        let s = s.into();
        if s.len() > MAX {
            return Err(ValidationError::TooLong {
                len: s.len(),
                max: MAX,
            });
        }
        Ok(Self(s))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub type Description = BoundedText<4096>;

// --- Validated URL ---

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ValidatedUrl(String);

impl ValidatedUrl {
    pub fn new(s: impl Into<String>) -> Result<Self, ValidationError> {
        let s = s.into();
        if !(s.starts_with("https://") || s.starts_with("http://")) {
            return Err(ValidationError::InvalidUrl(s));
        }
        // Reject known dangerous schemes that could sneak past prefix check.
        if s.contains("javascript:") || s.contains("data:") {
            return Err(ValidationError::InvalidUrl(s));
        }
        Ok(Self(s))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// --- Domain enums replacing stringly-typed fields ---

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum CaseStatus {
    Pending,
    Claimed,
    InProgress,
    Resolved,
    Closed,
}

impl fmt::Display for CaseStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum WoundSeverity {
    Minor,
    Moderate,
    Severe,
    Critical,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum MediaKind {
    Original,
    Thumbnail,
}

// --- Blob handle replaces inline Vec<u8> ---

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct BlobHandle {
    pub local_id: LocalId,
    pub size_bytes: u64,
}

// --- Create case payload ---

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CreateCasePayload {
    pub location: Coordinate,
    pub description: Option<Description>,
    pub wound_severity: Option<WoundSeverity>,
}

// --- Push payload ---

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum PushPayload {
    NewRescue {
        case_id: CaseId,
        location: Coordinate,
    },
    Mute {
        case_id: CaseId,
        claimed_by: UserId,
    },
    CaseUpdate {
        case_id: CaseId,
        new_status: CaseStatus,
    },
}

// --- Bounded error string for server/transport errors ---

pub type ErrorText = BoundedText<2048>;

// --- Event enum: no None variant, large variants boxed ---

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Event {
    // Auth & Onboarding
    LoginRequested,
    LoginCompleted {
        jwt: Secret,
        user_id: UserId,
    },
    LocationPermissionGranted {
        location: Coordinate,
    },
    LocationPinDropped {
        location: Coordinate,
    },
    RadiusSelected {
        meters: u32,
    },
    OnboardingComplete,

    // Offline / Sync
    NetworkStatusChanged {
        online: bool,
    },
    CreateCaseRequested(Box<CreateCasePayload>),
    OutboxFlushRequested,
    OutboxEntryCompleted {
        op_id: OpId,
    },
    OutboxEntryFailed {
        op_id: OpId,
        error: ErrorText,
    },

    // Feed & Map
    SwitchToMap,
    SwitchToList,
    MapMoved {
        center: Coordinate,
        zoom: Zoom,
    },
    CaseMarkerTapped {
        case_id: CaseId,
    },
    CaseDismissed,
    RefreshRequested,

    // Photo Flow
    CapturePhotoRequested,
    PhotoCaptured {
        handle: BlobHandle,
    },
    PhotoCancelled,
    PhotoUploadComplete {
        local_id: LocalId,
    },
    UploadUrlsReceived {
        local_id: LocalId,
        original_url: ValidatedUrl,
    },
    MediaUrlReceived {
        kind: MediaKind,
        url: ValidatedUrl,
    },

    // Claim & Transition Flow
    ClaimRequested {
        case_id: CaseId,
    },
    ClaimSucceeded {
        case_id: CaseId,
    },
    ClaimFailed {
        case_id: CaseId,
        current_status: CaseStatus,
        holder: Option<UserId>,
    },
    ClaimConflict {
        case_id: CaseId,
    },

    TransitionRequested {
        case_id: CaseId,
        next: CaseStatus,
    },
    TransitionSucceeded {
        case_id: CaseId,
        new_status: CaseStatus,
    },
    TransitionFailed {
        case_id: CaseId,
        error: ErrorText,
    },

    // Capability Responses (boxed to keep enum size small)
    HttpResult(Box<crate::capabilities::HttpResult>),
    KeyValueResult(Box<crate::capabilities::KvResult>),
    
    #[cfg(feature = "camera")]
    CameraResult(Box<crate::capabilities::CameraResult>),

    #[cfg(feature = "push")]
    PushResult(Box<crate::capabilities::PushResult>),
    
    #[cfg(feature = "push")]
    PushReceived(Box<PushPayload>),

    CryptoResult(Box<crate::capabilities::CryptoResult>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinate_rejects_nan() {
        assert!(Coordinate::new(f64::NAN, 0.0).is_err());
        assert!(Coordinate::new(0.0, f64::NAN).is_err());
    }

    #[test]
    fn coordinate_rejects_out_of_range() {
        assert!(Coordinate::new(91.0, 0.0).is_err());
        assert!(Coordinate::new(0.0, 181.0).is_err());
        assert!(Coordinate::new(-91.0, 0.0).is_err());
        assert!(Coordinate::new(0.0, -181.0).is_err());
    }

    #[test]
    fn coordinate_accepts_valid() {
        assert!(Coordinate::new(45.0, -73.0).is_ok());
        assert!(Coordinate::new(90.0, 180.0).is_ok());
        assert!(Coordinate::new(-90.0, -180.0).is_ok());
    }

    #[test]
    fn coordinate_rejects_infinity() {
        assert!(Coordinate::new(f64::INFINITY, 0.0).is_err());
        assert!(Coordinate::new(0.0, f64::NEG_INFINITY).is_err());
    }

    #[test]
    fn secret_debug_is_redacted() {
        let s = Secret::new("super_secret".into());
        assert_eq!(format!("{:?}", s), "[REDACTED]");
    }

    #[test]
    fn bounded_text_enforces_limit() {
        assert!(BoundedText::<5>::new("hello").is_ok());
        assert!(BoundedText::<5>::new("toolong").is_err());
    }

    #[test]
    fn validated_url_rejects_javascript() {
        assert!(ValidatedUrl::new("javascript:alert(1)").is_err());
        assert!(ValidatedUrl::new("https://example.com").is_ok());
        assert!(ValidatedUrl::new("ftp://files.com").is_err());
    }

    #[test]
    fn zoom_rejects_invalid() {
        assert!(Zoom::new(-1.0).is_err());
        assert!(Zoom::new(26.0).is_err());
        assert!(Zoom::new(f64::NAN).is_err());
        assert!(Zoom::new(15.0).is_ok());
    }

    #[test]
    fn typed_ids_are_not_interchangeable() {
        let case = CaseId::new("abc");
        let user = UserId::new("abc");
        // These are different types — mixing them is a compile error.
        // This test exists as documentation; the compiler enforces it.
        assert_eq!(case.as_str(), user.as_str());
    }

    #[test]
    fn event_size_is_reasonable() {
        // Ensure boxing keeps the enum small.
        let size = std::mem::size_of::<Event>();
        assert!(
            size <= 128,
            "Event enum is {} bytes — too large, box more variants",
            size
        );
    }
}