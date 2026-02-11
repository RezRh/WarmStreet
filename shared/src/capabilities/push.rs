use crux_core::capability::{Capability, CapabilityContext};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PermissionState {
    NotDetermined,
    Denied,
    Authorized,
    Provisional,
    Ephemeral,
}

impl PermissionState {
    #[must_use]
    pub const fn is_authorized(self) -> bool {
        matches!(self, Self::Authorized | Self::Provisional | Self::Ephemeral)
    }

    #[must_use]
    pub const fn is_denied(self) -> bool {
        matches!(self, Self::Denied)
    }

    #[must_use]
    pub const fn needs_request(self) -> bool {
        matches!(self, Self::NotDetermined)
    }
}

impl Default for PermissionState {
    fn default() -> Self {
        Self::NotDetermined
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RegisterOptions {
    #[serde(default)]
    pub force_refresh: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "op", content = "data")]
pub enum PushOperation {
    CheckAvailability,
    GetPermissionStatus,
    RequestPermission,
    Register(RegisterOptions),
    Unregister,
    GetToken,
}

#[derive(Debug, Clone, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum PushError {
    #[error("push notifications not available on this platform")]
    NotAvailable,

    #[error("permission denied by user")]
    PermissionDenied,

    #[error("permission not yet requested")]
    PermissionNotRequested,

    #[error("registration failed: {reason}")]
    RegistrationFailed {
        reason: String,
        #[serde(default)]
        platform_code: Option<String>,
        #[serde(default)]
        is_retryable: bool,
    },

    #[error("not registered for push notifications")]
    NotRegistered,

    #[error("token expired or invalidated")]
    TokenExpired,

    #[error("network error: {message}")]
    Network {
        message: String,
        #[serde(default)]
        is_retryable: bool,
    },

    #[error("rate limited, retry after {retry_after_secs} seconds")]
    RateLimited { retry_after_secs: u64 },

    #[error("operation timed out")]
    Timeout,

    #[error("unknown error: {message}")]
    Unknown { message: String },
}

impl PushError {
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        match self {
            Self::Network { is_retryable, .. } => *is_retryable,
            Self::RegistrationFailed { is_retryable, .. } => *is_retryable,
            Self::RateLimited { .. } | Self::Timeout => true,
            Self::NotAvailable
            | Self::PermissionDenied
            | Self::PermissionNotRequested
            | Self::NotRegistered
            | Self::TokenExpired
            | Self::Unknown { .. } => false,
        }
    }

    #[must_use]
    pub fn network(message: impl Into<String>) -> Self {
        Self::Network {
            message: message.into(),
            is_retryable: true,
        }
    }

    #[must_use]
    pub fn registration_failed(reason: impl Into<String>) -> Self {
        Self::RegistrationFailed {
            reason: reason.into(),
            platform_code: None,
            is_retryable: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "data")]
pub enum PushOutput {
    Available(bool),
    PermissionStatus(PermissionState),
    Registered { token: String },
    Unregistered,
    Token(Option<String>),
    TokenRefreshed { old_token: Option<String>, new_token: String },
}

impl PushOutput {
    #[must_use]
    pub fn token(&self) -> Option<&str> {
        match self {
            Self::Registered { token } | Self::Token(Some(token)) => Some(token),
            Self::TokenRefreshed { new_token, .. } => Some(new_token),
            _ => None,
        }
    }

    #[must_use]
    pub const fn is_available(&self) -> Option<bool> {
        match self {
            Self::Available(available) => Some(*available),
            _ => None,
        }
    }

    #[must_use]
    pub const fn permission_status(&self) -> Option<PermissionState> {
        match self {
            Self::PermissionStatus(state) => Some(*state),
            _ => None,
        }
    }
}

#[must_use = "push operation results must be handled"]
pub type PushResult = Result<PushOutput, PushError>;

#[derive(Debug, Clone)]
pub struct Push<E> {
    context: CapabilityContext<PushOperation, E>,
}

impl<Ev> Capability<Ev> for Push<Ev> {
    type Operation = PushOperation;
    type MappedSelf<MappedEv> = Push<MappedEv>;

    fn map_event<F, NewEv>(&self, f: F) -> Self::MappedSelf<NewEv>
    where
        F: Fn(NewEv) -> Ev + Send + Sync + Copy + 'static,
        Ev: 'static,
        NewEv: 'static,
    {
        Push::new(self.context.map_event(f))
    }
}

impl<E> Push<E> {
    pub fn new(context: CapabilityContext<PushOperation, E>) -> Self {
        Self { context }
    }

    pub fn check_availability<F>(&self, callback: F)
    where
        F: Fn(bool) -> E + Send + Sync + 'static,
    {
        let wrapper = move |result: PushResult| {
            let available = result
                .ok()
                .and_then(|o| o.is_available())
                .unwrap_or(false);
            callback(available)
        };
        self.context.request_from_shell(PushOperation::CheckAvailability, wrapper);
    }

    pub fn get_permission_status<F>(&self, callback: F)
    where
        F: Fn(PermissionState) -> E + Send + Sync + 'static,
    {
        let wrapper = move |result: PushResult| {
            let status = result
                .ok()
                .and_then(|o| o.permission_status())
                .unwrap_or(PermissionState::NotDetermined);
            callback(status)
        };
        self.context.request_from_shell(PushOperation::GetPermissionStatus, wrapper);
    }

    pub fn request_permission<F>(&self, callback: F)
    where
        F: Fn(PushResult) -> E + Send + Sync + 'static,
    {
        self.context.request_from_shell(PushOperation::RequestPermission, callback);
    }

    pub fn register<F>(&self, callback: F)
    where
        F: Fn(PushResult) -> E + Send + Sync + 'static,
    {
        self.context.request_from_shell(
            PushOperation::Register(RegisterOptions::default()),
            callback,
        );
    }

    pub fn register_with_options<F>(&self, options: RegisterOptions, callback: F)
    where
        F: Fn(PushResult) -> E + Send + Sync + 'static,
    {
        self.context.request_from_shell(PushOperation::Register(options), callback);
    }

    pub fn unregister<F>(&self, callback: F)
    where
        F: Fn(PushResult) -> E + Send + Sync + 'static,
    {
        self.context.request_from_shell(PushOperation::Unregister, callback);
    }

    pub fn get_token<F>(&self, callback: F)
    where
        F: Fn(PushResult) -> E + Send + Sync + 'static,
    {
        self.context.request_from_shell(PushOperation::GetToken, callback);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_state_checks() {
        assert!(PermissionState::Authorized.is_authorized());
        assert!(PermissionState::Provisional.is_authorized());
        assert!(PermissionState::Ephemeral.is_authorized());
        assert!(!PermissionState::Denied.is_authorized());
        assert!(!PermissionState::NotDetermined.is_authorized());

        assert!(PermissionState::Denied.is_denied());
        assert!(!PermissionState::Authorized.is_denied());

        assert!(PermissionState::NotDetermined.needs_request());
        assert!(!PermissionState::Authorized.needs_request());
        assert!(!PermissionState::Denied.needs_request());
    }

    #[test]
    fn test_push_error_is_retryable() {
        assert!(PushError::network("test").is_retryable());
        assert!(PushError::RateLimited { retry_after_secs: 60 }.is_retryable());
        assert!(PushError::Timeout.is_retryable());
        assert!(!PushError::PermissionDenied.is_retryable());
        assert!(!PushError::NotAvailable.is_retryable());
        assert!(!PushError::registration_failed("test").is_retryable());
    }

    #[test]
    fn test_push_output_token_extraction() {
        let registered = PushOutput::Registered {
            token: "abc123".into(),
        };
        assert_eq!(registered.token(), Some("abc123"));

        let token = PushOutput::Token(Some("def456".into()));
        assert_eq!(token.token(), Some("def456"));

        let no_token = PushOutput::Token(None);
        assert_eq!(no_token.token(), None);

        let unregistered = PushOutput::Unregistered;
        assert_eq!(unregistered.token(), None);

        let refreshed = PushOutput::TokenRefreshed {
            old_token: Some("old".into()),
            new_token: "new".into(),
        };
        assert_eq!(refreshed.token(), Some("new"));
    }

    #[test]
    fn test_push_output_available() {
        let available = PushOutput::Available(true);
        assert_eq!(available.is_available(), Some(true));

        let not_available = PushOutput::Available(false);
        assert_eq!(not_available.is_available(), Some(false));

        let other = PushOutput::Unregistered;
        assert_eq!(other.is_available(), None);
    }

    #[test]
    fn test_push_operation_serialization() {
        let op = PushOperation::Register(RegisterOptions { force_refresh: true });
        let json = serde_json::to_string(&op).unwrap();
        let deserialized: PushOperation = serde_json::from_str(&json).unwrap();
        assert_eq!(op, deserialized);
    }

    #[test]
    fn test_push_error_serialization() {
        let error = PushError::RegistrationFailed {
            reason: "test".into(),
            platform_code: Some("APNS_001".into()),
            is_retryable: true,
        };
        let json = serde_json::to_string(&error).unwrap();
        let deserialized: PushError = serde_json::from_str(&json).unwrap();
        assert_eq!(error, deserialized);
    }

    #[test]
    fn test_register_options_default() {
        let options = RegisterOptions::default();
        assert!(!options.force_refresh);
    }
}