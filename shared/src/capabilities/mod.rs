mod crypto;
mod http;
mod kv;
mod location;
mod outbox;
mod push;
mod telemetry;

#[cfg(feature = "camera")]
mod camera;

pub use self::crypto::{CryptoError, CryptoOperation, CryptoOutput, CryptoResult, HashAlgorithm, KeyAlgorithm};
pub use self::http::{HttpError, HttpOperation, HttpOutput, HttpResult};
pub use self::kv::{KvError, KvOperation, KvOutput, KvResult};
pub use self::location::{Location, LocationError, LocationOperation, LocationResult, PermissionState};
pub use self::push::{Push, PushError, PushOperation, PushOutput, PushResult};
pub use self::telemetry::{Telemetry, TelemetryOperation};

pub use self::outbox::{
    BlobRef, DeadLetterReason, EntryState, ErrorCategory, IdempotencyKey, IntentError, LatLon,
    LeaseToken, LocalOpId, MetricsSnapshot, OpId, Outbox, OutboxConfig, OutboxEntry, OutboxError,
    OutboxIntent, OutboxStorage, QueueDepthSnapshot, RetryHistory, ServerCaseId, ServerCaseStatus,
    SqliteStorage, UnixTimeMs, WoundSeverity,
};

#[cfg(feature = "camera")]
pub use self::camera::{CameraError, CameraFacing, CameraOperation, CameraOutput, CameraResult};

pub use crux_core::render::Render;
pub use crux_http::Http;
pub use crux_kv::KeyValue;
pub use crux_core::Effect;

use crate::Event;

pub type AppHttp = Http<Event>;
pub type AppKv = KeyValue<Event>;
pub type AppRender = Render<Event>;
pub type AppCrypto = crypto::Crypto<Event>;
pub type AppLocation = Location<Event>;
pub type AppPush = Push<Event>;
pub type AppTelemetry = Telemetry<Event>;

#[cfg(feature = "camera")]
pub type AppCamera = camera::Camera<Event>;

#[derive(Debug, thiserror::Error)]
pub enum CapabilityError {
    #[error("HTTP error: {0}")]
    Http(#[from] HttpError),

    #[error("Storage error: {0}")]
    Kv(#[from] KvError),

    #[error("Crypto error: {0}")]
    Crypto(#[from] CryptoError),

    #[error("Location error: {0}")]
    Location(#[from] LocationError),

    #[error("Push error: {0}")]
    Push(#[from] PushError),

    #[error("Telemetry error")]
    Telemetry,

    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),

    #[cfg(feature = "camera")]
    #[error("Camera error: {0}")]
    Camera(#[from] CameraError),
}

#[derive(crux_core::macros::Effect)]
pub struct Capabilities {
    pub http: crux_http::Http<Event>,
    pub kv: crux_kv::KeyValue<Event>,
    pub render: crux_core::render::Render<Event>,
    pub crypto: crypto::Crypto<Event>,
    pub location: location::Location<Event>,
    pub push: push::Push<Event>,
    pub telemetry: telemetry::Telemetry<Event>,

    #[cfg(feature = "camera")]
    pub camera: camera::Camera<Event>,
}

#[cfg(any(test, feature = "test-utils"))]
pub mod testing {
    use super::*;

    pub fn mock_capabilities() -> Capabilities<Event> {
        Capabilities {
            http: AppHttp::default(),
            kv: AppKv::default(),
            render: AppRender::default(),
            crypto: AppCrypto::default(),
            location: AppLocation::default(),
            push: AppPush::default(),
            telemetry: AppTelemetry::default(),
            #[cfg(feature = "camera")]
            camera: AppCamera::default(),
        }
    }
}

#[cfg(test)]
impl Default for Capabilities<Event> {
    fn default() -> Self {
        testing::mock_capabilities()
    }
}
