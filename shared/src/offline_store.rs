use std::fs::File;
use std::io::Write;
use std::path::Path;

use ciborium;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use thiserror::Error;

use crate::crypto::{CryptoError, CryptoProvider};
use crate::model::LocalCase;
use crate::outbox::{OutboxEntry, OutboxError};

const CURRENT_SCHEMA_VERSION: u32 = 1;
const MAX_STORE_BYTES: usize = 100 * 1024 * 1024;
const MAX_OUTBOX_ENTRIES: usize = 10_000;
const MAX_PENDING_CASES: usize = 1_000;
const STORE_MAGIC: &[u8; 4] = b"OFST";

#[derive(Error, Debug)]
pub enum StoreError {
    #[error("crypto error: {0}")]
    Crypto(#[from] CryptoError),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("outbox error: {0}")]
    Outbox(#[from] OutboxError),

    #[error("corrupted store: {reason}")]
    Corrupted { reason: &'static str },

    #[error("integrity check failed: expected {expected}, got {actual}")]
    IntegrityCheckFailed { expected: String, actual: String },

    #[error("schema version {found} is newer than supported {max}")]
    FutureSchema { found: u32, max: u32 },

    #[error("unknown schema version: {0}")]
    UnknownSchema(u32),

    #[error("store too large: {size} bytes, max {max}")]
    StoreTooLarge { size: usize, max: usize },

    #[error("too many outbox entries: {count}, max {max}")]
    TooManyOutboxEntries { count: usize, max: usize },

    #[error("too many pending cases: {count}, max {max}")]
    TooManyPendingCases { count: usize, max: usize },

    #[error("lock acquisition failed")]
    LockFailed,
}

impl From<ciborium::de::Error<std::io::Error>> for StoreError {
    fn from(e: ciborium::de::Error<std::io::Error>) -> Self {
        StoreError::Serialization(e.to_string())
    }
}

impl From<ciborium::ser::Error<std::io::Error>> for StoreError {
    fn from(e: ciborium::ser::Error<std::io::Error>) -> Self {
        StoreError::Serialization(e.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreContext {
    user_id: String,
    device_id: String,
}

impl StoreContext {
    pub fn new(user_id: impl Into<String>, device_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            device_id: device_id.into(),
        }
    }

    fn to_aad(&self) -> Vec<u8> {
        format!(
            "offline-store:v{}:{}:{}",
            CURRENT_SCHEMA_VERSION, self.user_id, self.device_id
        )
        .into_bytes()
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct StoreEnvelope {
    magic: [u8; 4],
    schema_version: u32,
    checksum: [u8; 32],
    payload: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
struct StorePayload {
    outbox: VecDeque<OutboxEntry>,
    pending_local_cases: Vec<LocalCase>,
}

#[derive(Debug)]
pub struct OfflineStore {
    schema_version: u32,
    outbox: VecDeque<OutboxEntry>,
    pending_local_cases: Vec<LocalCase>,
}

impl Default for OfflineStore {
    fn default() -> Self {
        Self::new()
    }
}

impl OfflineStore {
    pub fn new() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            outbox: VecDeque::new(),
            pending_local_cases: Vec::new(),
        }
    }

    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn outbox(&self) -> &VecDeque<OutboxEntry> {
        &self.outbox
    }

    pub fn outbox_mut(&mut self) -> &mut VecDeque<OutboxEntry> {
        &mut self.outbox
    }

    pub fn pending_cases(&self) -> &[LocalCase] {
        &self.pending_local_cases
    }

    pub fn pending_cases_mut(&mut self) -> &mut Vec<LocalCase> {
        &mut self.pending_local_cases
    }

    pub fn outbox_len(&self) -> usize {
        self.outbox.len()
    }

    pub fn pending_cases_len(&self) -> usize {
        self.pending_local_cases.len()
    }

    pub fn push_outbox(&mut self, entry: OutboxEntry) -> Result<(), StoreError> {
        if self.outbox.len() >= MAX_OUTBOX_ENTRIES {
            return Err(StoreError::TooManyOutboxEntries {
                count: self.outbox.len() + 1,
                max: MAX_OUTBOX_ENTRIES,
            });
        }
        crate::outbox::push_entry(&mut self.outbox, entry)?;
        Ok(())
    }

    pub fn add_pending_case(&mut self, case: LocalCase) -> Result<(), StoreError> {
        if self.pending_local_cases.len() >= MAX_PENDING_CASES {
            return Err(StoreError::TooManyPendingCases {
                count: self.pending_local_cases.len() + 1,
                max: MAX_PENDING_CASES,
            });
        }
        self.pending_local_cases.push(case);
        Ok(())
    }

    pub fn pop_outbox(&mut self) -> Option<OutboxEntry> {
        self.outbox.pop_front()
    }

    pub fn remove_pending_case(&mut self, index: usize) -> Option<LocalCase> {
        if index < self.pending_local_cases.len() {
            Some(self.pending_local_cases.remove(index))
        } else {
            None
        }
    }

    pub fn clear_outbox(&mut self) {
        self.outbox.clear();
    }

    pub fn clear_pending_cases(&mut self) {
        self.pending_local_cases.clear();
    }

    pub fn save_to_path<C: CryptoProvider>(
        &self,
        path: &Path,
        crypto: &C,
        ctx: &StoreContext,
    ) -> Result<(), StoreError> {
        let encrypted = self.serialize_encrypted(crypto, ctx)?;

        let tmp_path = path.with_extension("tmp");

        let mut file = File::create(&tmp_path)?;
        file.write_all(&encrypted)?;
        file.sync_all()?;

        std::fs::rename(&tmp_path, path)?;

        if let Some(parent) = path.parent() {
            if let Ok(dir) = File::open(parent) {
                let _ = dir.sync_all();
            }
        }

        Ok(())
    }

    pub fn load_from_path<C: CryptoProvider>(
        path: &Path,
        crypto: &C,
        ctx: &StoreContext,
    ) -> Result<Self, StoreError> {
        if !path.exists() {
            return Ok(Self::new());
        }

        let encrypted = std::fs::read(path)?;

        if encrypted.is_empty() {
            return Err(StoreError::Corrupted {
                reason: "empty file",
            });
        }

        Self::deserialize_encrypted(&encrypted, crypto, ctx)
    }

    pub fn serialize_encrypted<C: CryptoProvider>(
        &self,
        crypto: &C,
        ctx: &StoreContext,
    ) -> Result<Vec<u8>, StoreError> {
        let payload = StorePayload {
            outbox: self.outbox.clone(),
            pending_local_cases: self.pending_local_cases.clone(),
        };

        let mut payload_bytes = Vec::new();
        ciborium::into_writer(&payload, &mut payload_bytes)?;

        let checksum = blake3::hash(&payload_bytes);

        let envelope = StoreEnvelope {
            magic: *STORE_MAGIC,
            schema_version: self.schema_version,
            checksum: *checksum.as_bytes(),
            payload: payload_bytes,
        };

        let mut envelope_bytes = Vec::new();
        ciborium::into_writer(&envelope, &mut envelope_bytes)?;

        let aad = ctx.to_aad();
        let encrypted = crypto.encrypt(&envelope_bytes, &aad)?;

        Ok(encrypted)
    }

    pub fn deserialize_encrypted<C: CryptoProvider>(
        encrypted: &[u8],
        crypto: &C,
        ctx: &StoreContext,
    ) -> Result<Self, StoreError> {
        if encrypted.len() > MAX_STORE_BYTES {
            return Err(StoreError::StoreTooLarge {
                size: encrypted.len(),
                max: MAX_STORE_BYTES,
            });
        }

        let aad = ctx.to_aad();
        let envelope_bytes = crypto.decrypt(encrypted, &aad)?;

        let envelope: StoreEnvelope = ciborium::from_reader(&envelope_bytes[..])?;

        if envelope.magic != *STORE_MAGIC {
            return Err(StoreError::Corrupted {
                reason: "invalid magic bytes",
            });
        }

        if envelope.schema_version > CURRENT_SCHEMA_VERSION {
            return Err(StoreError::FutureSchema {
                found: envelope.schema_version,
                max: CURRENT_SCHEMA_VERSION,
            });
        }

        let actual_checksum = blake3::hash(&envelope.payload);
        if actual_checksum.as_bytes() != &envelope.checksum {
            return Err(StoreError::IntegrityCheckFailed {
                expected: hex::encode(envelope.checksum),
                actual: hex::encode(actual_checksum.as_bytes()),
            });
        }

        let payload: StorePayload = ciborium::from_reader(&envelope.payload[..])?;

        if payload.outbox.len() > MAX_OUTBOX_ENTRIES {
            return Err(StoreError::TooManyOutboxEntries {
                count: payload.outbox.len(),
                max: MAX_OUTBOX_ENTRIES,
            });
        }

        if payload.pending_local_cases.len() > MAX_PENDING_CASES {
            return Err(StoreError::TooManyPendingCases {
                count: payload.pending_local_cases.len(),
                max: MAX_PENDING_CASES,
            });
        }

        let store = if envelope.schema_version < CURRENT_SCHEMA_VERSION {
            Self::migrate(envelope.schema_version, payload)?
        } else {
            Self {
                schema_version: envelope.schema_version,
                outbox: payload.outbox,
                pending_local_cases: payload.pending_local_cases,
            }
        };

        Ok(store)
    }

    fn migrate(from_version: u32, payload: StorePayload) -> Result<Self, StoreError> {
        match from_version {
            0 => Self::migrate_v0_to_v1(payload),
            _ => Err(StoreError::UnknownSchema(from_version)),
        }
    }

    fn migrate_v0_to_v1(payload: StorePayload) -> Result<Self, StoreError> {
        Ok(Self {
            schema_version: 1,
            outbox: payload.outbox,
            pending_local_cases: payload.pending_local_cases,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::Aes256GcmCryptoProvider;
    use secrecy::SecretVec;
    use tempfile::tempdir;

    fn test_crypto() -> Aes256GcmCryptoProvider {
        let key = SecretVec::new(vec![0u8; 32]);
        Aes256GcmCryptoProvider::new(key).unwrap()
    }

    fn test_context() -> StoreContext {
        StoreContext::new("user123", "device456")
    }

    fn sample_outbox_entry() -> OutboxEntry {
        OutboxEntry {
            id: "entry1".into(),
            payload: vec![1, 2, 3],
            created_at: 1000,
            attempts: 0,
        }
    }

    fn sample_local_case() -> LocalCase {
        LocalCase {
            id: "case1".into(),
            data: vec![4, 5, 6],
        }
    }

    #[test]
    fn new_store_has_current_schema() {
        let store = OfflineStore::new();
        assert_eq!(store.schema_version(), CURRENT_SCHEMA_VERSION);
        assert!(store.outbox().is_empty());
        assert!(store.pending_cases().is_empty());
    }

    #[test]
    fn roundtrip_empty_store() {
        let crypto = test_crypto();
        let ctx = test_context();
        let store = OfflineStore::new();

        let encrypted = store.serialize_encrypted(&crypto, &ctx).unwrap();
        let loaded = OfflineStore::deserialize_encrypted(&encrypted, &crypto, &ctx).unwrap();

        assert_eq!(loaded.schema_version(), store.schema_version());
        assert_eq!(loaded.outbox_len(), 0);
        assert_eq!(loaded.pending_cases_len(), 0);
    }

    #[test]
    fn roundtrip_with_data() {
        let crypto = test_crypto();
        let ctx = test_context();
        let mut store = OfflineStore::new();

        store.push_outbox(sample_outbox_entry()).unwrap();
        store.add_pending_case(sample_local_case()).unwrap();

        let encrypted = store.serialize_encrypted(&crypto, &ctx).unwrap();
        let loaded = OfflineStore::deserialize_encrypted(&encrypted, &crypto, &ctx).unwrap();

        assert_eq!(loaded.outbox_len(), 1);
        assert_eq!(loaded.pending_cases_len(), 1);
        assert_eq!(loaded.outbox().front().unwrap().id, "entry1");
        assert_eq!(loaded.pending_cases()[0].id, "case1");
    }

    #[test]
    fn file_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("store.bin");

        let crypto = test_crypto();
        let ctx = test_context();
        let mut store = OfflineStore::new();

        store.push_outbox(sample_outbox_entry()).unwrap();
        store.save_to_path(&path, &crypto, &ctx).unwrap();

        let loaded = OfflineStore::load_from_path(&path, &crypto, &ctx).unwrap();

        assert_eq!(loaded.outbox_len(), 1);
    }

    #[test]
    fn load_nonexistent_returns_new() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nonexistent.bin");

        let crypto = test_crypto();
        let ctx = test_context();

        let loaded = OfflineStore::load_from_path(&path, &crypto, &ctx).unwrap();

        assert_eq!(loaded.schema_version(), CURRENT_SCHEMA_VERSION);
        assert!(loaded.outbox().is_empty());
    }

    #[test]
    fn empty_file_is_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("empty.bin");
        std::fs::write(&path, b"").unwrap();

        let crypto = test_crypto();
        let ctx = test_context();

        let result = OfflineStore::load_from_path(&path, &crypto, &ctx);

        assert!(matches!(result, Err(StoreError::Corrupted { .. })));
    }

    #[test]
    fn wrong_context_fails_decryption() {
        let crypto = test_crypto();
        let ctx1 = StoreContext::new("user1", "device1");
        let ctx2 = StoreContext::new("user2", "device2");

        let store = OfflineStore::new();
        let encrypted = store.serialize_encrypted(&crypto, &ctx1).unwrap();

        let result = OfflineStore::deserialize_encrypted(&encrypted, &crypto, &ctx2);

        assert!(matches!(result, Err(StoreError::Crypto(_))));
    }

    #[test]
    fn corrupted_checksum_fails() {
        let crypto = test_crypto();
        let ctx = test_context();
        let store = OfflineStore::new();

        let mut encrypted = store.serialize_encrypted(&crypto, &ctx).unwrap();
        if let Some(byte) = encrypted.last_mut() {
            *byte ^= 0xFF;
        }

        let result = OfflineStore::deserialize_encrypted(&encrypted, &crypto, &ctx);

        assert!(result.is_err());
    }

    #[test]
    fn oversized_input_rejected() {
        let crypto = test_crypto();
        let ctx = test_context();

        let oversized = vec![0u8; MAX_STORE_BYTES + 1];
        let result = OfflineStore::deserialize_encrypted(&oversized, &crypto, &ctx);

        assert!(matches!(result, Err(StoreError::StoreTooLarge { .. })));
    }

    #[test]
    fn outbox_limit_enforced() {
        let mut store = OfflineStore::new();

        for i in 0..MAX_OUTBOX_ENTRIES {
            let entry = OutboxEntry {
                id: format!("entry{}", i),
                payload: vec![],
                created_at: i as u64,
                attempts: 0,
            };
            store.push_outbox(entry).unwrap();
        }

        let result = store.push_outbox(sample_outbox_entry());

        assert!(matches!(result, Err(StoreError::TooManyOutboxEntries { .. })));
    }

    #[test]
    fn pending_cases_limit_enforced() {
        let mut store = OfflineStore::new();

        for i in 0..MAX_PENDING_CASES {
            let case = LocalCase {
                id: format!("case{}", i),
                data: vec![],
            };
            store.add_pending_case(case).unwrap();
        }

        let result = store.add_pending_case(sample_local_case());

        assert!(matches!(result, Err(StoreError::TooManyPendingCases { .. })));
    }

    #[test]
    fn atomic_write_leaves_no_tmp_on_success() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("store.bin");
        let tmp_path = path.with_extension("tmp");

        let crypto = test_crypto();
        let ctx = test_context();
        let store = OfflineStore::new();

        store.save_to_path(&path, &crypto, &ctx).unwrap();

        assert!(path.exists());
        assert!(!tmp_path.exists());
    }

    #[test]
    fn pop_outbox_removes_front() {
        let mut store = OfflineStore::new();

        let entry1 = OutboxEntry {
            id: "first".into(),
            payload: vec![],
            created_at: 1,
            attempts: 0,
        };
        let entry2 = OutboxEntry {
            id: "second".into(),
            payload: vec![],
            created_at: 2,
            attempts: 0,
        };

        store.push_outbox(entry1).unwrap();
        store.push_outbox(entry2).unwrap();

        let popped = store.pop_outbox().unwrap();
        assert_eq!(popped.id, "first");
        assert_eq!(store.outbox_len(), 1);
    }

    #[test]
    fn remove_pending_case_by_index() {
        let mut store = OfflineStore::new();
        store.add_pending_case(sample_local_case()).unwrap();

        let removed = store.remove_pending_case(0);

        assert!(removed.is_some());
        assert!(store.pending_cases().is_empty());
    }

    #[test]
    fn remove_pending_case_invalid_index() {
        let mut store = OfflineStore::new();

        let removed = store.remove_pending_case(999);

        assert!(removed.is_none());
    }

    #[test]
    fn clear_operations() {
        let mut store = OfflineStore::new();
        store.push_outbox(sample_outbox_entry()).unwrap();
        store.add_pending_case(sample_local_case()).unwrap();

        store.clear_outbox();
        store.clear_pending_cases();

        assert!(store.outbox().is_empty());
        assert!(store.pending_cases().is_empty());
    }
}