use aead::{AeadInPlace, KeyInit};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use secrecy::{ExposeSecret, Secret};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use thiserror::Error;
use zeroize::Zeroize;

const ENVELOPE_MAGIC: [u8; 8] = *b"WARMCRY1";
const CURRENT_VERSION: u32 = 1;
const MIN_SUPPORTED_VERSION: u32 = 1;
const HEADER_SIZE: usize = 41;
const TAG_SIZE: usize = 16;
const NONCE_SIZE: usize = 24;
const KEY_SIZE: usize = 32;
const MAX_AAD_LEN: usize = 8 * 1024;
const MAX_AAD_FIELD: usize = 1024;
const RESERVED_KEY_ID: u32 = 0;

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_plaintext: usize,
    pub max_ciphertext: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_plaintext: 5 * 1024 * 1024,
            max_ciphertext: 6 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecryptFailure {
    MalformedEnvelope,
    UnsupportedVersion { version: u32 },
    UnsupportedAlgorithm { alg: u8 },
    KeyNotFound { key_id: u32 },
    AuthenticationFailed,
    PayloadTooLarge,
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum CryptoError {
    #[error("invalid key length: expected {expected}, got {actual}")]
    InvalidKeyLength { expected: usize, actual: usize },

    #[error("invalid key id: {0} is reserved")]
    InvalidKeyId(u32),

    #[error("randomness unavailable")]
    RandomUnavailable,

    #[error("plaintext too large: {size} > {max}")]
    PlaintextTooLarge { size: usize, max: usize },

    #[error("ciphertext too large: {size} > {max}")]
    CiphertextTooLarge { size: usize, max: usize },

    #[error("encryption failed")]
    EncryptionFailed,

    #[error("decryption failed: {0:?}")]
    DecryptionFailed(DecryptFailure),

    #[error("aad too large: {size} > {max}")]
    AadTooLarge { size: usize, max: usize },

    #[error("aad field too large: {field} has {size} > {max}")]
    AadFieldTooLarge {
        field: &'static str,
        size: usize,
        max: usize,
    },

    #[error("aad required but empty")]
    AadRequired,

    #[error("no keys available")]
    NoKeysAvailable,

    #[error("cannot remove primary key {0}, set another primary first")]
    CannotRemovePrimaryKey(u32),

    #[error("key not found: {0}")]
    KeyNotFound(u32),

    #[error("lock poisoned")]
    LockPoisoned,
}

pub trait CryptoProvider: Send + Sync {
    fn encrypt(&self, plaintext: &[u8], aad: &[u8]) -> Result<Vec<u8>, CryptoError>;
    fn decrypt(&self, envelope: &[u8], aad: &[u8]) -> Result<Vec<u8>, CryptoError>;
}

pub trait RandomProvider: Send + Sync {
    fn fill(&self, out: &mut [u8]) -> Result<(), CryptoError>;
}

pub struct OsRng;

impl RandomProvider for OsRng {
    fn fill(&self, out: &mut [u8]) -> Result<(), CryptoError> {
        getrandom::getrandom(out).map_err(|_| CryptoError::RandomUnavailable)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum AlgId {
    XChaCha20Poly1305 = 1,
}

impl TryFrom<u8> for AlgId {
    type Error = u8;

    fn try_from(v: u8) -> Result<Self, u8> {
        match v {
            1 => Ok(AlgId::XChaCha20Poly1305),
            other => Err(other),
        }
    }
}

struct KeyEntry {
    secret: Secret<[u8; KEY_SIZE]>,
}

impl Drop for KeyEntry {
    fn drop(&mut self) {
        // Secret handles zeroization
    }
}

struct KeyStore {
    keys: HashMap<u32, KeyEntry>,
    primary_key_id: Option<u32>,
}

impl KeyStore {
    fn new() -> Self {
        Self {
            keys: HashMap::new(),
            primary_key_id: None,
        }
    }
}

pub struct KeyRing<R: RandomProvider = OsRng> {
    store: RwLock<KeyStore>,
    rng: R,
    limits: Limits,
    encrypt_count: AtomicU64,
    decrypt_count: AtomicU64,
    decrypt_failures: AtomicU64,
}

impl KeyRing<OsRng> {
    pub fn with_os_rng(limits: Limits) -> Result<Self, CryptoError> {
        Self::new(OsRng, limits)
    }
}

impl<R: RandomProvider> KeyRing<R> {
    pub fn new(rng: R, limits: Limits) -> Result<Self, CryptoError> {
        Ok(Self {
            store: RwLock::new(KeyStore::new()),
            rng,
            limits,
            encrypt_count: AtomicU64::new(0),
            decrypt_count: AtomicU64::new(0),
            decrypt_failures: AtomicU64::new(0),
        })
    }

    pub fn add_key(&self, key_id: u32, key_bytes: &[u8]) -> Result<(), CryptoError> {
        if key_id == RESERVED_KEY_ID {
            return Err(CryptoError::InvalidKeyId(key_id));
        }

        if key_bytes.len() != KEY_SIZE {
            return Err(CryptoError::InvalidKeyLength {
                expected: KEY_SIZE,
                actual: key_bytes.len(),
            });
        }

        let mut k = [0u8; KEY_SIZE];
        k.copy_from_slice(key_bytes);

        let mut store = self.store.write().map_err(|_| CryptoError::LockPoisoned)?;

        let is_first = store.keys.is_empty();
        store.keys.insert(
            key_id,
            KeyEntry {
                secret: Secret::new(k),
            },
        );

        if is_first {
            store.primary_key_id = Some(key_id);
        }

        k.zeroize();
        Ok(())
    }

    pub fn set_primary(&self, key_id: u32) -> Result<(), CryptoError> {
        let mut store = self.store.write().map_err(|_| CryptoError::LockPoisoned)?;

        if !store.keys.contains_key(&key_id) {
            return Err(CryptoError::KeyNotFound(key_id));
        }

        store.primary_key_id = Some(key_id);
        Ok(())
    }

    pub fn remove_key(&self, key_id: u32) -> Result<(), CryptoError> {
        let mut store = self.store.write().map_err(|_| CryptoError::LockPoisoned)?;

        if store.primary_key_id == Some(key_id) {
            return Err(CryptoError::CannotRemovePrimaryKey(key_id));
        }

        store.keys.remove(&key_id);
        Ok(())
    }

    pub fn has_key(&self, key_id: u32) -> Result<bool, CryptoError> {
        let store = self.store.read().map_err(|_| CryptoError::LockPoisoned)?;
        Ok(store.keys.contains_key(&key_id))
    }

    pub fn primary_key_id(&self) -> Result<Option<u32>, CryptoError> {
        let store = self.store.read().map_err(|_| CryptoError::LockPoisoned)?;
        Ok(store.primary_key_id)
    }

    pub fn key_count(&self) -> Result<usize, CryptoError> {
        let store = self.store.read().map_err(|_| CryptoError::LockPoisoned)?;
        Ok(store.keys.len())
    }

    pub fn stats(&self) -> KeyRingStats {
        KeyRingStats {
            encrypt_count: self.encrypt_count.load(Ordering::Relaxed),
            decrypt_count: self.decrypt_count.load(Ordering::Relaxed),
            decrypt_failures: self.decrypt_failures.load(Ordering::Relaxed),
        }
    }

    fn get_cipher_for_key(
        &self,
        store: &KeyStore,
        key_id: u32,
    ) -> Result<XChaCha20Poly1305, DecryptFailure> {
        let entry = store
            .keys
            .get(&key_id)
            .ok_or(DecryptFailure::KeyNotFound { key_id })?;

        Ok(XChaCha20Poly1305::new(Key::from_slice(
            entry.secret.expose_secret(),
        )))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct KeyRingStats {
    pub encrypt_count: u64,
    pub decrypt_count: u64,
    pub decrypt_failures: u64,
}

impl<R: RandomProvider> CryptoProvider for KeyRing<R> {
    fn encrypt(&self, plaintext: &[u8], aad: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if plaintext.len() > self.limits.max_plaintext {
            return Err(CryptoError::PlaintextTooLarge {
                size: plaintext.len(),
                max: self.limits.max_plaintext,
            });
        }

        if aad.is_empty() {
            return Err(CryptoError::AadRequired);
        }

        if aad.len() > MAX_AAD_LEN {
            return Err(CryptoError::AadTooLarge {
                size: aad.len(),
                max: MAX_AAD_LEN,
            });
        }

        let total_len = HEADER_SIZE + plaintext.len() + TAG_SIZE;
        if total_len > self.limits.max_ciphertext {
            return Err(CryptoError::CiphertextTooLarge {
                size: total_len,
                max: self.limits.max_ciphertext,
            });
        }

        let store = self.store.read().map_err(|_| CryptoError::LockPoisoned)?;

        let key_id = store.primary_key_id.ok_or(CryptoError::NoKeysAvailable)?;

        let entry = store
            .keys
            .get(&key_id)
            .ok_or(CryptoError::NoKeysAvailable)?;

        let cipher = XChaCha20Poly1305::new(Key::from_slice(entry.secret.expose_secret()));
        drop(store);

        let mut nonce_bytes = [0u8; NONCE_SIZE];
        self.rng.fill(&mut nonce_bytes)?;

        let mut out = vec![0u8; total_len];

        out[0..8].copy_from_slice(&ENVELOPE_MAGIC);
        out[8..12].copy_from_slice(&CURRENT_VERSION.to_le_bytes());
        out[12] = AlgId::XChaCha20Poly1305 as u8;
        out[13..17].copy_from_slice(&key_id.to_le_bytes());
        out[17..41].copy_from_slice(&nonce_bytes);

        let pt_end = HEADER_SIZE + plaintext.len();
        out[HEADER_SIZE..pt_end].copy_from_slice(plaintext);

        let tag = cipher
            .encrypt_in_place_detached(
                XNonce::from_slice(&nonce_bytes),
                aad,
                &mut out[HEADER_SIZE..pt_end],
            )
            .map_err(|_| CryptoError::EncryptionFailed)?;

        out[pt_end..].copy_from_slice(&tag);

        self.encrypt_count.fetch_add(1, Ordering::Relaxed);

        Ok(out)
    }

    fn decrypt(&self, envelope: &[u8], aad: &[u8]) -> Result<Vec<u8>, CryptoError> {
        self.decrypt_count.fetch_add(1, Ordering::Relaxed);

        let result = self.decrypt_inner(envelope, aad);

        if result.is_err() {
            self.decrypt_failures.fetch_add(1, Ordering::Relaxed);
        }

        result
    }
}

impl<R: RandomProvider> KeyRing<R> {
    fn decrypt_inner(&self, envelope: &[u8], aad: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if envelope.len() < HEADER_SIZE + TAG_SIZE {
            return Err(CryptoError::DecryptionFailed(
                DecryptFailure::MalformedEnvelope,
            ));
        }

        if envelope.len() > self.limits.max_ciphertext {
            return Err(CryptoError::DecryptionFailed(
                DecryptFailure::PayloadTooLarge,
            ));
        }

        if &envelope[0..8] != ENVELOPE_MAGIC {
            return Err(CryptoError::DecryptionFailed(
                DecryptFailure::MalformedEnvelope,
            ));
        }

        if aad.is_empty() {
            return Err(CryptoError::AadRequired);
        }

        if aad.len() > MAX_AAD_LEN {
            return Err(CryptoError::AadTooLarge {
                size: aad.len(),
                max: MAX_AAD_LEN,
            });
        }

        let version = u32::from_le_bytes(envelope[8..12].try_into().unwrap());

        if version < MIN_SUPPORTED_VERSION || version > CURRENT_VERSION {
            return Err(CryptoError::DecryptionFailed(
                DecryptFailure::UnsupportedVersion { version },
            ));
        }

        let alg_byte = envelope[12];
        let _alg = AlgId::try_from(alg_byte).map_err(|_| {
            CryptoError::DecryptionFailed(DecryptFailure::UnsupportedAlgorithm { alg: alg_byte })
        })?;

        let key_id = u32::from_le_bytes(envelope[13..17].try_into().unwrap());
        let nonce_bytes: [u8; NONCE_SIZE] = envelope[17..41].try_into().unwrap();
        let ciphertext_with_tag = &envelope[HEADER_SIZE..];

        if ciphertext_with_tag.len() < TAG_SIZE {
            return Err(CryptoError::DecryptionFailed(
                DecryptFailure::MalformedEnvelope,
            ));
        }

        let store = self.store.read().map_err(|_| CryptoError::LockPoisoned)?;
        let cipher = self
            .get_cipher_for_key(&store, key_id)
            .map_err(CryptoError::DecryptionFailed)?;
        drop(store);

        let ct_len = ciphertext_with_tag.len() - TAG_SIZE;
        let mut buffer = ciphertext_with_tag[..ct_len].to_vec();
        let tag = &ciphertext_with_tag[ct_len..];

        let result = cipher.decrypt_in_place_detached(
            XNonce::from_slice(&nonce_bytes),
            aad,
            &mut buffer,
            tag.into(),
        );

        if result.is_err() {
            buffer.zeroize();
            return Err(CryptoError::DecryptionFailed(
                DecryptFailure::AuthenticationFailed,
            ));
        }

        if buffer.len() > self.limits.max_plaintext {
            buffer.zeroize();
            return Err(CryptoError::DecryptionFailed(
                DecryptFailure::PayloadTooLarge,
            ));
        }

        Ok(buffer)
    }
}

pub fn build_aad(
    app_ns: &str,
    store_name: &str,
    schema_version: u32,
    user_id: Option<&str>,
) -> Result<Vec<u8>, CryptoError> {
    validate_aad_field("app_ns", app_ns)?;
    validate_aad_field("store_name", store_name)?;

    if let Some(u) = user_id {
        validate_aad_field("user_id", u)?;
    }

    let capacity = 4 + app_ns.len() + 4 + store_name.len() + 4 + 1 + user_id.map_or(0, |u| 4 + u.len());
    let mut aad = Vec::with_capacity(capacity);

    aad.extend_from_slice(&(app_ns.len() as u16).to_le_bytes());
    aad.extend_from_slice(app_ns.as_bytes());

    aad.extend_from_slice(&(store_name.len() as u16).to_le_bytes());
    aad.extend_from_slice(store_name.as_bytes());

    aad.extend_from_slice(&schema_version.to_le_bytes());

    match user_id {
        None => aad.push(0),
        Some(u) => {
            aad.push(1);
            aad.extend_from_slice(&(u.len() as u16).to_le_bytes());
            aad.extend_from_slice(u.as_bytes());
        }
    }

    Ok(aad)
}

fn validate_aad_field(name: &'static str, value: &str) -> Result<(), CryptoError> {
    if value.len() > MAX_AAD_FIELD {
        return Err(CryptoError::AadFieldTooLarge {
            field: name,
            size: value.len(),
            max: MAX_AAD_FIELD,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU64;
    use std::sync::Arc;
    use std::thread;

    struct SequentialRng {
        counter: AtomicU64,
    }

    impl SequentialRng {
        fn new() -> Self {
            Self {
                counter: AtomicU64::new(1),
            }
        }
    }

    impl RandomProvider for SequentialRng {
        fn fill(&self, out: &mut [u8]) -> Result<(), CryptoError> {
            let val = self.counter.fetch_add(1, Ordering::SeqCst);
            for (i, byte) in out.iter_mut().enumerate() {
                *byte = ((val >> ((i % 8) * 8)) ^ (i as u64)) as u8;
            }
            Ok(())
        }
    }

    fn test_keyring() -> KeyRing<SequentialRng> {
        let kr = KeyRing::new(SequentialRng::new(), Limits::default()).unwrap();
        kr.add_key(1, &[7u8; 32]).unwrap();
        kr
    }

    fn test_aad() -> Vec<u8> {
        build_aad("app", "store", 1, Some("user")).unwrap()
    }

    #[test]
    fn roundtrip() {
        let kr = test_keyring();
        let aad = test_aad();
        let enc = kr.encrypt(b"hello", &aad).unwrap();
        assert_eq!(kr.decrypt(&enc, &aad).unwrap(), b"hello");
    }

    #[test]
    fn roundtrip_empty_plaintext() {
        let kr = test_keyring();
        let aad = test_aad();
        let enc = kr.encrypt(b"", &aad).unwrap();
        assert_eq!(kr.decrypt(&enc, &aad).unwrap(), b"");
    }

    #[test]
    fn roundtrip_large_plaintext() {
        let kr = test_keyring();
        let aad = test_aad();
        let data = vec![0xAB; 1024 * 1024];
        let enc = kr.encrypt(&data, &aad).unwrap();
        assert_eq!(kr.decrypt(&enc, &aad).unwrap(), data);
    }

    #[test]
    fn envelope_size_correct() {
        let kr = test_keyring();
        let aad = test_aad();
        let enc = kr.encrypt(b"x", &aad).unwrap();
        assert_eq!(enc.len(), HEADER_SIZE + 1 + TAG_SIZE);
    }

    #[test]
    fn wrong_aad_fails() {
        let kr = test_keyring();
        let aad1 = build_aad("app", "store", 1, Some("user1")).unwrap();
        let aad2 = build_aad("app", "store", 1, Some("user2")).unwrap();
        let enc = kr.encrypt(b"hello", &aad1).unwrap();

        let err = kr.decrypt(&enc, &aad2).unwrap_err();
        assert!(matches!(
            err,
            CryptoError::DecryptionFailed(DecryptFailure::AuthenticationFailed)
        ));
    }

    #[test]
    fn none_vs_empty_user_id_distinct() {
        let aad_none = build_aad("a", "b", 1, None).unwrap();
        let aad_empty = build_aad("a", "b", 1, Some("")).unwrap();
        assert_ne!(aad_none, aad_empty);
    }

    #[test]
    fn aad_field_too_large() {
        let long = "a".repeat(MAX_AAD_FIELD + 1);
        let err = build_aad(&long, "b", 1, None).unwrap_err();
        assert!(matches!(err, CryptoError::AadFieldTooLarge { field: "app_ns", .. }));
    }

    #[test]
    fn empty_aad_rejected() {
        let kr = test_keyring();
        assert!(matches!(
            kr.encrypt(b"hi", b""),
            Err(CryptoError::AadRequired)
        ));
    }

    #[test]
    fn key_id_zero_rejected() {
        let kr = test_keyring();
        let err = kr.add_key(0, &[1u8; 32]).unwrap_err();
        assert!(matches!(err, CryptoError::InvalidKeyId(0)));
    }

    #[test]
    fn invalid_key_length_rejected() {
        let kr = test_keyring();
        let err = kr.add_key(2, &[1u8; 16]).unwrap_err();
        assert!(matches!(
            err,
            CryptoError::InvalidKeyLength { expected: 32, actual: 16 }
        ));
    }

    #[test]
    fn no_keys_fails_encrypt() {
        let kr = KeyRing::new(SequentialRng::new(), Limits::default()).unwrap();
        let err = kr.encrypt(b"test", &test_aad()).unwrap_err();
        assert!(matches!(err, CryptoError::NoKeysAvailable));
    }

    #[test]
    fn key_rotation() {
        let kr = KeyRing::new(SequentialRng::new(), Limits::default()).unwrap();
        kr.add_key(1, &[7u8; 32]).unwrap();
        kr.add_key(2, &[8u8; 32]).unwrap();

        let aad = test_aad();
        let enc1 = kr.encrypt(b"old", &aad).unwrap();

        kr.set_primary(2).unwrap();
        let enc2 = kr.encrypt(b"new", &aad).unwrap();

        assert_eq!(kr.decrypt(&enc1, &aad).unwrap(), b"old");
        assert_eq!(kr.decrypt(&enc2, &aad).unwrap(), b"new");

        assert_eq!(u32::from_le_bytes(enc1[13..17].try_into().unwrap()), 1);
        assert_eq!(u32::from_le_bytes(enc2[13..17].try_into().unwrap()), 2);
    }

    #[test]
    fn cannot_remove_primary_key() {
        let kr = test_keyring();
        let err = kr.remove_key(1).unwrap_err();
        assert!(matches!(err, CryptoError::CannotRemovePrimaryKey(1)));
    }

    #[test]
    fn remove_non_primary_key() {
        let kr = KeyRing::new(SequentialRng::new(), Limits::default()).unwrap();
        kr.add_key(1, &[7u8; 32]).unwrap();
        kr.add_key(2, &[8u8; 32]).unwrap();

        kr.remove_key(2).unwrap();
        assert!(!kr.has_key(2).unwrap());
        assert!(kr.has_key(1).unwrap());
    }

    #[test]
    fn removed_key_fails_decrypt() {
        let kr = KeyRing::new(SequentialRng::new(), Limits::default()).unwrap();
        kr.add_key(1, &[7u8; 32]).unwrap();
        kr.add_key(2, &[8u8; 32]).unwrap();

        let aad = test_aad();
        kr.set_primary(2).unwrap();
        let enc = kr.encrypt(b"test", &aad).unwrap();

        kr.set_primary(1).unwrap();
        kr.remove_key(2).unwrap();

        let err = kr.decrypt(&enc, &aad).unwrap_err();
        assert!(matches!(
            err,
            CryptoError::DecryptionFailed(DecryptFailure::KeyNotFound { key_id: 2 })
        ));
    }

    #[test]
    fn tampered_ciphertext_fails() {
        let kr = test_keyring();
        let aad = test_aad();
        let mut enc = kr.encrypt(b"hello", &aad).unwrap();
        enc[enc.len() - 1] ^= 1;

        let err = kr.decrypt(&enc, &aad).unwrap_err();
        assert!(matches!(
            err,
            CryptoError::DecryptionFailed(DecryptFailure::AuthenticationFailed)
        ));
    }

    #[test]
    fn tampered_header_fails() {
        let kr = test_keyring();
        let aad = test_aad();
        let mut enc = kr.encrypt(b"hello", &aad).unwrap();
        enc[15] ^= 1;

        let err = kr.decrypt(&enc, &aad).unwrap_err();
        assert!(matches!(
            err,
            CryptoError::DecryptionFailed(DecryptFailure::KeyNotFound { .. })
                | CryptoError::DecryptionFailed(DecryptFailure::AuthenticationFailed)
        ));
    }

    #[test]
    fn bad_magic_fails() {
        let kr = test_keyring();
        let aad = test_aad();
        let mut enc = kr.encrypt(b"hello", &aad).unwrap();
        enc[0] = 0xFF;

        let err = kr.decrypt(&enc, &aad).unwrap_err();
        assert!(matches!(
            err,
            CryptoError::DecryptionFailed(DecryptFailure::MalformedEnvelope)
        ));
    }

    #[test]
    fn short_envelope_fails() {
        let kr = test_keyring();
        let err = kr.decrypt(&[0u8; 40], &test_aad()).unwrap_err();
        assert!(matches!(
            err,
            CryptoError::DecryptionFailed(DecryptFailure::MalformedEnvelope)
        ));
    }

    #[test]
    fn unsupported_version_fails() {
        let kr = test_keyring();
        let aad = test_aad();
        let mut enc = kr.encrypt(b"hello", &aad).unwrap();
        enc[8..12].copy_from_slice(&99u32.to_le_bytes());

        let err = kr.decrypt(&enc, &aad).unwrap_err();
        assert!(matches!(
            err,
            CryptoError::DecryptionFailed(DecryptFailure::UnsupportedVersion { version: 99 })
        ));
    }

    #[test]
    fn unsupported_algorithm_fails() {
        let kr = test_keyring();
        let aad = test_aad();
        let mut enc = kr.encrypt(b"hello", &aad).unwrap();
        enc[12] = 99;

        let err = kr.decrypt(&enc, &aad).unwrap_err();
        assert!(matches!(
            err,
            CryptoError::DecryptionFailed(DecryptFailure::UnsupportedAlgorithm { alg: 99 })
        ));
    }

    #[test]
    fn plaintext_too_large() {
        let limits = Limits {
            max_plaintext: 100,
            max_ciphertext: 200,
        };
        let kr = KeyRing::new(SequentialRng::new(), limits).unwrap();
        kr.add_key(1, &[7u8; 32]).unwrap();

        let err = kr.encrypt(&[0u8; 101], &test_aad()).unwrap_err();
        assert!(matches!(
            err,
            CryptoError::PlaintextTooLarge { size: 101, max: 100 }
        ));
    }

    #[test]
    fn ciphertext_too_large_on_decrypt() {
        let limits = Limits {
            max_plaintext: 100,
            max_ciphertext: 100,
        };
        let kr = KeyRing::new(SequentialRng::new(), limits).unwrap();
        kr.add_key(1, &[7u8; 32]).unwrap();

        let err = kr.decrypt(&[0u8; 101], &test_aad()).unwrap_err();
        assert!(matches!(
            err,
            CryptoError::DecryptionFailed(DecryptFailure::PayloadTooLarge)
        ));
    }

    #[test]
    fn stats_tracked() {
        let kr = test_keyring();
        let aad = test_aad();

        let enc = kr.encrypt(b"test", &aad).unwrap();
        kr.decrypt(&enc, &aad).unwrap();
        let _ = kr.decrypt(b"garbage", &aad);

        let stats = kr.stats();
        assert_eq!(stats.encrypt_count, 1);
        assert_eq!(stats.decrypt_count, 2);
        assert_eq!(stats.decrypt_failures, 1);
    }

    #[test]
    fn unique_nonces() {
        let kr = test_keyring();
        let aad = test_aad();

        let enc1 = kr.encrypt(b"same", &aad).unwrap();
        let enc2 = kr.encrypt(b"same", &aad).unwrap();

        assert_ne!(enc1[17..41], enc2[17..41]);
    }

    #[test]
    fn envelope_structure() {
        let kr = test_keyring();
        let enc = kr.encrypt(b"x", &test_aad()).unwrap();

        assert_eq!(&enc[0..8], b"WARMCRY1");
        assert_eq!(u32::from_le_bytes(enc[8..12].try_into().unwrap()), 1);
        assert_eq!(enc[12], 1);
        assert_eq!(u32::from_le_bytes(enc[13..17].try_into().unwrap()), 1);
    }

    #[test]
    fn concurrent_encrypt_decrypt() {
        let kr = Arc::new(KeyRing::with_os_rng(Limits::default()).unwrap());
        kr.add_key(1, &[7u8; 32]).unwrap();
        let aad = test_aad();

        let handles: Vec<_> = (0..8)
            .map(|i| {
                let kr = Arc::clone(&kr);
                let aad = aad.clone();
                thread::spawn(move || {
                    for j in 0..100 {
                        let msg = format!("thread-{}-iter-{}", i, j);
                        let enc = kr.encrypt(msg.as_bytes(), &aad).unwrap();
                        let dec = kr.decrypt(&enc, &aad).unwrap();
                        assert_eq!(dec, msg.as_bytes());
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }
    }

    #[test]
    fn concurrent_rotation() {
        let kr = Arc::new(KeyRing::with_os_rng(Limits::default()).unwrap());
        kr.add_key(1, &[7u8; 32]).unwrap();
        let aad = test_aad();

        let enc_before = kr.encrypt(b"before", &aad).unwrap();

        let kr2 = Arc::clone(&kr);
        let rotator = thread::spawn(move || {
            kr2.add_key(2, &[8u8; 32]).unwrap();
            kr2.set_primary(2).unwrap();
        });

        rotator.join().unwrap();

        assert_eq!(kr.decrypt(&enc_before, &aad).unwrap(), b"before");

        let enc_after = kr.encrypt(b"after", &aad).unwrap();
        assert_eq!(
            u32::from_le_bytes(enc_after[13..17].try_into().unwrap()),
            2
        );
    }

    #[test]
    fn first_key_becomes_primary() {
        let kr = KeyRing::new(SequentialRng::new(), Limits::default()).unwrap();
        assert_eq!(kr.primary_key_id().unwrap(), None);

        kr.add_key(5, &[1u8; 32]).unwrap();
        assert_eq!(kr.primary_key_id().unwrap(), Some(5));

        kr.add_key(10, &[2u8; 32]).unwrap();
        assert_eq!(kr.primary_key_id().unwrap(), Some(5));
    }

    #[test]
    fn set_primary_unknown_key_fails() {
        let kr = test_keyring();
        let err = kr.set_primary(999).unwrap_err();
        assert!(matches!(err, CryptoError::KeyNotFound(999)));
    }

    #[test]
    fn key_count() {
        let kr = KeyRing::new(SequentialRng::new(), Limits::default()).unwrap();
        assert_eq!(kr.key_count().unwrap(), 0);

        kr.add_key(1, &[1u8; 32]).unwrap();
        assert_eq!(kr.key_count().unwrap(), 1);

        kr.add_key(2, &[2u8; 32]).unwrap();
        assert_eq!(kr.key_count().unwrap(), 2);
    }

    #[test]
    fn aad_length_encoding() {
        let aad = build_aad("app", "store", 1, Some("user")).unwrap();

        let app_len = u16::from_le_bytes(aad[0..2].try_into().unwrap());
        assert_eq!(app_len, 3);

        let store_start = 2 + 3;
        let store_len = u16::from_le_bytes(aad[store_start..store_start + 2].try_into().unwrap());
        assert_eq!(store_len, 5);
    }
}