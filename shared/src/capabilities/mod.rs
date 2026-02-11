mod crypto;
mod http;
mod kv;
mod outbox;

#[cfg(feature = "camera")]
mod camera;

#[cfg(feature = "push")]
mod push;

pub use self::crypto::{
    CryptoError, CryptoOperation, CryptoOutput, CryptoResult, HashAlgorithm, KeyAlgorithm,
};
pub use self::http::{HttpError, HttpOperation, HttpOutput, HttpResult};
pub use self::kv::{KvError, KvOperation, KvOutput, KvResult};

pub use self::outbox::{
    BlobRef, DeadLetterReason, EntryState, ErrorCategory, IdempotencyKey, IntentError, LatLon,
    LeaseToken, LocalOpId, MetricsSnapshot, OpId, Outbox, OutboxConfig, OutboxEntry, OutboxError,
    OutboxIntent, OutboxStorage, QueueDepthSnapshot, RetryHistory, ServerCaseId, ServerCaseStatus,
    SqliteStorage, UnixTimeMs, WoundSeverity,
};

#[cfg(feature = "camera")]
pub use self::camera::{CameraError, CameraFacing, CameraOperation, CameraOutput, CameraResult};

#[cfg(feature = "push")]
pub use self::push::{PushError, PushOperation, PushOutput, PushResult};

//! Render capability re-export.
//!
//! We use Crux's built-in Render capability directly because it provides
//! all necessary functionality for triggering view updates.
pub use crux_core::render::Render;
pub use crux_http::Http;
pub use crux_kv::KeyValue;

use crate::event::Event;

pub type AppHttp = Http<Event>;
pub type AppKv = KeyValue<Event>;
pub type AppRender = Render<Event>;
pub type AppCrypto = crypto::Crypto<Event>;

#[cfg(feature = "camera")]
pub type AppCamera = camera::Camera<Event>;

#[cfg(feature = "push")]
pub type AppPush = push::Push<Event>;

#[derive(Debug, thiserror::Error)]
pub enum CapabilityError {
    #[error("HTTP error: {0}")]
    Http(#[from] HttpError),

    #[error("Storage error: {0}")]
    Kv(#[from] KvError),

    #[error("Crypto error: {0}")]
    Crypto(#[from] CryptoError),

    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),

    #[cfg(feature = "camera")]
    #[error("Camera error: {0}")]
    Camera(#[from] CameraError),

    #[cfg(feature = "push")]
    #[error("Push error: {0}")]
    Push(#[from] PushError),
}

#[derive(crux_core::macros::Effect)]
pub struct Capabilities {
    pub http: AppHttp,
    pub kv: AppKv,
    pub render: AppRender,
    pub crypto: AppCrypto,

    #[cfg(feature = "camera")]
    pub camera: AppCamera,

    #[cfg(feature = "push")]
    pub push: AppPush,
}

#[cfg(any(test, feature = "test-utils"))]
pub mod testing {
    use super::*;

    pub fn mock_capabilities() -> Capabilities {
        Capabilities {
            http: AppHttp::default(),
            kv: AppKv::default(),
            render: AppRender::default(),
            crypto: AppCrypto::default(),
            #[cfg(feature = "camera")]
            camera: AppCamera::default(),
            #[cfg(feature = "push")]
            push: AppPush::default(),
        }
    }
}

#[cfg(test)]
impl Default for Capabilities {
    fn default() -> Self {
        testing::mock_capabilities()
    }
}