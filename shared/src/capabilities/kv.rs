use crux_kv::KeyValue;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::marker::PhantomData;
use thiserror::Error;

use crate::event::Event;

pub type KvCapability = KeyValue<Event>;

pub const MAX_KEY_LENGTH: usize = 512;
pub const MAX_VALUE_SIZE: usize = 10 * 1024 * 1024;
pub const MAX_PREFIX_LENGTH: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KvKey {
    namespace: KeyNamespace,
    key: String,
}

impl KvKey {
    pub fn new(namespace: KeyNamespace, key: impl Into<String>) -> Result<Self, KvError> {
        let key = key.into();
        Self::validate_key(&key)?;
        Ok(Self { namespace, key })
    }

    pub fn raw(&self) -> String {
        format!("{}:{}", self.namespace.prefix(), self.key)
    }

    pub fn namespace(&self) -> &KeyNamespace {
        &self.namespace
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    fn validate_key(key: &str) -> Result<(), KvError> {
        if key.is_empty() {
            return Err(KvError::InvalidKey {
                key: key.to_string(),
                reason: "key cannot be empty".to_string(),
            });
        }

        if key.len() > MAX_KEY_LENGTH {
            return Err(KvError::InvalidKey {
                key: key.chars().take(50).collect::<String>() + "...",
                reason: format!("key exceeds maximum length of {} bytes", MAX_KEY_LENGTH),
            });
        }

        let trimmed = key.trim();
        if trimmed.is_empty() {
            return Err(KvError::InvalidKey {
                key: key.to_string(),
                reason: "key cannot be only whitespace".to_string(),
            });
        }

        if key.contains('\0') {
            return Err(KvError::InvalidKey {
                key: key.replace('\0', "\\0"),
                reason: "key cannot contain null bytes".to_string(),
            });
        }

        if key.contains("..") {
            return Err(KvError::InvalidKey {
                key: key.to_string(),
                reason: "key cannot contain path traversal sequences".to_string(),
            });
        }

        if key.starts_with('/') || key.starts_with('\\') {
            return Err(KvError::InvalidKey {
                key: key.to_string(),
                reason: "key cannot start with path separator".to_string(),
            });
        }

        for c in key.chars() {
            if c.is_control() && c != '\t' {
                return Err(KvError::InvalidKey {
                    key: key.to_string(),
                    reason: "key contains invalid control characters".to_string(),
                });
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyNamespace {
    Outbox,
    Session,
    Cache,
    UserData,
    Settings,
    Sync,
    Custom(String),
}

impl KeyNamespace {
    pub fn prefix(&self) -> &str {
        match self {
            KeyNamespace::Outbox => "outbox",
            KeyNamespace::Session => "session",
            KeyNamespace::Cache => "cache",
            KeyNamespace::UserData => "userdata",
            KeyNamespace::Settings => "settings",
            KeyNamespace::Sync => "sync",
            KeyNamespace::Custom(s) => s.as_str(),
        }
    }

    pub fn custom(prefix: impl Into<String>) -> Result<Self, KvError> {
        let prefix = prefix.into();
        if prefix.is_empty() {
            return Err(KvError::InvalidKey {
                key: prefix,
                reason: "custom namespace cannot be empty".to_string(),
            });
        }
        if prefix.len() > MAX_PREFIX_LENGTH {
            return Err(KvError::InvalidKey {
                key: prefix,
                reason: format!(
                    "custom namespace exceeds maximum length of {} bytes",
                    MAX_PREFIX_LENGTH
                ),
            });
        }
        if !prefix
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(KvError::InvalidKey {
                key: prefix,
                reason: "custom namespace contains invalid characters".to_string(),
            });
        }
        Ok(KeyNamespace::Custom(prefix))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvValue {
    data: Vec<u8>,
    version: u64,
    created_at: u64,
    updated_at: u64,
}

impl KvValue {
    pub fn new(data: Vec<u8>, now_ms: u64) -> Result<Self, KvError> {
        if data.len() > MAX_VALUE_SIZE {
            return Err(KvError::ValueTooLarge {
                size: data.len(),
                max: MAX_VALUE_SIZE,
            });
        }
        Ok(Self {
            data,
            version: 1,
            created_at: now_ms,
            updated_at: now_ms,
        })
    }

    pub fn from_serializable<T: Serialize>(value: &T, now_ms: u64) -> Result<Self, KvError> {
        let data = serde_json::to_vec(value).map_err(|e| KvError::Serialization {
            message: e.to_string(),
            key: None,
        })?;
        Self::new(data, now_ms)
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn created_at(&self) -> u64 {
        self.created_at
    }

    pub fn updated_at(&self) -> u64 {
        self.updated_at
    }

    pub fn deserialize<T: DeserializeOwned>(&self) -> Result<T, KvError> {
        serde_json::from_slice(&self.data).map_err(|e| KvError::Serialization {
            message: e.to_string(),
            key: None,
        })
    }

    pub fn increment_version(&mut self, now_ms: u64) {
        self.version = self.version.saturating_add(1);
        self.updated_at = now_ms;
    }

    pub fn update_data(&mut self, data: Vec<u8>, now_ms: u64) -> Result<(), KvError> {
        if data.len() > MAX_VALUE_SIZE {
            return Err(KvError::ValueTooLarge {
                size: data.len(),
                max: MAX_VALUE_SIZE,
            });
        }
        self.data = data;
        self.increment_version(now_ms);
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum KvOperation {
    Get {
        key: KvKey,
    },
    Set {
        key: KvKey,
        value: Vec<u8>,
        if_version: Option<u64>,
    },
    Delete {
        key: KvKey,
        if_version: Option<u64>,
    },
    Exists {
        key: KvKey,
    },
    List {
        namespace: KeyNamespace,
        prefix: Option<String>,
        limit: u32,
        cursor: Option<String>,
    },
    GetMulti {
        keys: Vec<KvKey>,
    },
    DeleteMulti {
        keys: Vec<KvKey>,
    },
}

impl KvOperation {
    pub fn get(namespace: KeyNamespace, key: impl Into<String>) -> Result<Self, KvError> {
        Ok(Self::Get {
            key: KvKey::new(namespace, key)?,
        })
    }

    pub fn set(
        namespace: KeyNamespace,
        key: impl Into<String>,
        value: Vec<u8>,
    ) -> Result<Self, KvError> {
        if value.len() > MAX_VALUE_SIZE {
            return Err(KvError::ValueTooLarge {
                size: value.len(),
                max: MAX_VALUE_SIZE,
            });
        }
        Ok(Self::Set {
            key: KvKey::new(namespace, key)?,
            value,
            if_version: None,
        })
    }

    pub fn set_if_version(
        namespace: KeyNamespace,
        key: impl Into<String>,
        value: Vec<u8>,
        expected_version: u64,
    ) -> Result<Self, KvError> {
        if value.len() > MAX_VALUE_SIZE {
            return Err(KvError::ValueTooLarge {
                size: value.len(),
                max: MAX_VALUE_SIZE,
            });
        }
        Ok(Self::Set {
            key: KvKey::new(namespace, key)?,
            value,
            if_version: Some(expected_version),
        })
    }

    pub fn delete(namespace: KeyNamespace, key: impl Into<String>) -> Result<Self, KvError> {
        Ok(Self::Delete {
            key: KvKey::new(namespace, key)?,
            if_version: None,
        })
    }

    pub fn exists(namespace: KeyNamespace, key: impl Into<String>) -> Result<Self, KvError> {
        Ok(Self::Exists {
            key: KvKey::new(namespace, key)?,
        })
    }

    pub fn list(namespace: KeyNamespace, prefix: Option<String>, limit: u32) -> Self {
        Self::List {
            namespace,
            prefix,
            limit: limit.min(1000),
            cursor: None,
        }
    }

    pub fn list_with_cursor(
        namespace: KeyNamespace,
        prefix: Option<String>,
        limit: u32,
        cursor: String,
    ) -> Self {
        Self::List {
            namespace,
            prefix,
            limit: limit.min(1000),
            cursor: Some(cursor),
        }
    }
}

#[derive(Debug, Clone, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum KvError {
    #[error("key not found: {key}")]
    NotFound { key: String },

    #[error("invalid key '{key}': {reason}")]
    InvalidKey { key: String, reason: String },

    #[error("value too large: {size} bytes exceeds maximum of {max} bytes")]
    ValueTooLarge { size: usize, max: usize },

    #[error("version mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: u64, found: u64 },

    #[error("storage error: {message} (code: {code}, retryable: {retryable})")]
    Storage {
        code: StorageErrorCode,
        message: String,
        retryable: bool,
    },

    #[error("serialization error: {message}")]
    Serialization { message: String, key: Option<String> },

    #[error("quota exceeded: {used}/{limit} bytes")]
    QuotaExceeded { used: u64, limit: u64 },

    #[error("operation timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    #[error("too many keys in batch: {count} exceeds maximum of {max}")]
    BatchTooLarge { count: usize, max: usize },
}

impl KvError {
    pub fn is_retryable(&self) -> bool {
        match self {
            KvError::Storage { retryable, .. } => *retryable,
            KvError::Timeout { .. } => true,
            KvError::VersionMismatch { .. } => true,
            _ => false,
        }
    }

    pub fn is_not_found(&self) -> bool {
        matches!(self, KvError::NotFound { .. })
    }

    pub fn storage(code: StorageErrorCode, message: impl Into<String>) -> Self {
        let retryable = code.is_retryable();
        Self::Storage {
            code,
            message: message.into(),
            retryable,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageErrorCode {
    Unknown,
    ConnectionFailed,
    ConnectionLost,
    Corrupted,
    DiskFull,
    PermissionDenied,
    Busy,
    Locked,
    IoError,
}

impl StorageErrorCode {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            StorageErrorCode::ConnectionFailed
                | StorageErrorCode::ConnectionLost
                | StorageErrorCode::Busy
                | StorageErrorCode::Locked
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum KvOutput {
    Value(Option<KvValue>),
    Written { version: u64 },
    Deleted { existed: bool },
    Exists(bool),
    List {
        entries: Vec<KvListEntry>,
        next_cursor: Option<String>,
        has_more: bool,
    },
    Multi(Vec<Option<KvValue>>),
    DeletedMulti { deleted_count: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KvListEntry {
    pub key: String,
    pub version: u64,
    pub size: usize,
    pub updated_at: u64,
}

pub type KvResult = Result<KvOutput, KvError>;

pub struct TypedKvStore<T> {
    namespace: KeyNamespace,
    _phantom: PhantomData<T>,
}

impl<T: Serialize + DeserializeOwned> TypedKvStore<T> {
    pub fn new(namespace: KeyNamespace) -> Self {
        Self {
            namespace,
            _phantom: PhantomData,
        }
    }

    pub fn get_op(&self, key: impl Into<String>) -> Result<KvOperation, KvError> {
        KvOperation::get(self.namespace.clone(), key)
    }

    pub fn set_op(&self, key: impl Into<String>, value: &T) -> Result<KvOperation, KvError> {
        let data = serde_json::to_vec(value).map_err(|e| KvError::Serialization {
            message: e.to_string(),
            key: None,
        })?;
        KvOperation::set(self.namespace.clone(), key, data)
    }

    pub fn delete_op(&self, key: impl Into<String>) -> Result<KvOperation, KvError> {
        KvOperation::delete(self.namespace.clone(), key)
    }

    pub fn parse_value(&self, output: KvOutput) -> Result<Option<T>, KvError> {
        match output {
            KvOutput::Value(Some(kv_value)) => {
                let value = kv_value.deserialize()?;
                Ok(Some(value))
            }
            KvOutput::Value(None) => Ok(None),
            _ => Err(KvError::Storage {
                code: StorageErrorCode::Unknown,
                message: "unexpected output type".to_string(),
                retryable: false,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_validation_empty() {
        let result = KvKey::new(KeyNamespace::Cache, "");
        assert!(result.is_err());
        assert!(matches!(result, Err(KvError::InvalidKey { .. })));
    }

    #[test]
    fn test_key_validation_whitespace() {
        let result = KvKey::new(KeyNamespace::Cache, "   ");
        assert!(result.is_err());
    }

    #[test]
    fn test_key_validation_null_byte() {
        let result = KvKey::new(KeyNamespace::Cache, "key\0value");
        assert!(result.is_err());
    }

    #[test]
    fn test_key_validation_path_traversal() {
        let result = KvKey::new(KeyNamespace::Cache, "../etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn test_key_validation_too_long() {
        let long_key = "a".repeat(MAX_KEY_LENGTH + 1);
        let result = KvKey::new(KeyNamespace::Cache, long_key);
        assert!(result.is_err());
    }

    #[test]
    fn test_key_validation_control_chars() {
        let result = KvKey::new(KeyNamespace::Cache, "key\x01value");
        assert!(result.is_err());
    }

    #[test]
    fn test_key_validation_valid() {
        let result = KvKey::new(KeyNamespace::Cache, "valid-key_123");
        assert!(result.is_ok());
    }

    #[test]
    fn test_key_namespace_prefix() {
        let key = KvKey::new(KeyNamespace::Outbox, "entry-1").unwrap();
        assert_eq!(key.raw(), "outbox:entry-1");
    }

    #[test]
    fn test_custom_namespace() {
        let ns = KeyNamespace::custom("myapp").unwrap();
        assert_eq!(ns.prefix(), "myapp");
    }

    #[test]
    fn test_custom_namespace_invalid() {
        let result = KeyNamespace::custom("");
        assert!(result.is_err());

        let result = KeyNamespace::custom("invalid namespace!");
        assert!(result.is_err());
    }

    #[test]
    fn test_value_size_limit() {
        let large_data = vec![0u8; MAX_VALUE_SIZE + 1];
        let result = KvValue::new(large_data, 0);
        assert!(result.is_err());
        assert!(matches!(result, Err(KvError::ValueTooLarge { .. })));
    }

    #[test]
    fn test_value_serialization() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct TestData {
            name: String,
            count: u32,
        }

        let data = TestData {
            name: "test".to_string(),
            count: 42,
        };

        let value = KvValue::from_serializable(&data, 1000).unwrap();
        let parsed: TestData = value.deserialize().unwrap();

        assert_eq!(data, parsed);
    }

    #[test]
    fn test_value_versioning() {
        let mut value = KvValue::new(vec![1, 2, 3], 1000).unwrap();
        assert_eq!(value.version(), 1);

        value.increment_version(2000);
        assert_eq!(value.version(), 2);
        assert_eq!(value.updated_at(), 2000);
        assert_eq!(value.created_at(), 1000);
    }

    #[test]
    fn test_error_retryable() {
        assert!(KvError::Timeout { timeout_ms: 1000 }.is_retryable());
        assert!(KvError::VersionMismatch {
            expected: 1,
            found: 2
        }
        .is_retryable());
        assert!(KvError::storage(StorageErrorCode::Busy, "busy").is_retryable());
        assert!(!KvError::storage(StorageErrorCode::Corrupted, "bad").is_retryable());
        assert!(!KvError::NotFound {
            key: "x".to_string()
        }
        .is_retryable());
    }

    #[test]
    fn test_operation_builders() {
        let op = KvOperation::get(KeyNamespace::Cache, "key1").unwrap();
        assert!(matches!(op, KvOperation::Get { .. }));

        let op = KvOperation::set(KeyNamespace::Cache, "key1", vec![1, 2, 3]).unwrap();
        assert!(matches!(op, KvOperation::Set { if_version: None, .. }));

        let op = KvOperation::set_if_version(KeyNamespace::Cache, "key1", vec![1], 5).unwrap();
        assert!(matches!(
            op,
            KvOperation::Set {
                if_version: Some(5),
                ..
            }
        ));
    }

    #[test]
    fn test_list_limit_capped() {
        let op = KvOperation::list(KeyNamespace::Cache, None, 9999);
        if let KvOperation::List { limit, .. } = op {
            assert_eq!(limit, 1000);
        } else {
            panic!("expected List operation");
        }
    }

    #[test]
    fn test_typed_store() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct User {
            id: u64,
            name: String,
        }

        let store: TypedKvStore<User> = TypedKvStore::new(KeyNamespace::UserData);

        let user = User {
            id: 1,
            name: "Alice".to_string(),
        };

        let op = store.set_op("user-1", &user).unwrap();
        assert!(matches!(op, KvOperation::Set { .. }));
    }

    #[test]
    fn test_batch_get_operation() {
        let keys = vec![
            KvKey::new(KeyNamespace::Cache, "key1").unwrap(),
            KvKey::new(KeyNamespace::Cache, "key2").unwrap(),
        ];

        let op = KvOperation::GetMulti { keys: keys.clone() };
        if let KvOperation::GetMulti { keys: op_keys } = op {
            assert_eq!(op_keys.len(), 2);
        }
    }
}