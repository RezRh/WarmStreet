use serde::{Deserialize, Serialize};
use thiserror::Error;
use crux_core::capability::{CapabilityContext, Capability};

use crate::event::Event;

#[derive(Debug, Clone)]
pub struct Crypto<E> {
    context: CapabilityContext<CryptoOperation, E>,
}

impl<Ev> Capability<Ev> for Crypto<Ev> {
    type Operation = CryptoOperation;
    type MappedSelf<MappedEv> = Crypto<MappedEv>;

    fn map_event<F, NewEv>(&self, f: F) -> Self::MappedSelf<NewEv>
    where
        F: Fn(NewEv) -> Ev + Send + Sync + Copy + 'static,
        Ev: 'static,
        NewEv: 'static,
    {
        Crypto::new(self.context.map_event(f))
    }
}

impl<E> Crypto<E> {
    pub fn new(context: CapabilityContext<CryptoOperation, E>) -> Self {
        Self { context }
    }

    pub fn encrypt<F>(&self, key_id: String, plaintext: Vec<u8>, callback: F)
    where
        F: Fn(CryptoResult) -> E + Send + Sync + 'static,
    {
        self.context.request_from_shell(CryptoOperation::Encrypt { key_id, plaintext }, callback);
    }

    pub fn decrypt<F>(&self, key_id: String, ciphertext: Vec<u8>, callback: F)
    where
        F: Fn(CryptoResult) -> E + Send + Sync + 'static,
    {
        self.context.request_from_shell(CryptoOperation::Decrypt { key_id, ciphertext }, callback);
    }
}

pub type CryptoCapability = Crypto<Event>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CryptoOperation {
    GenerateKeyPair { algorithm: KeyAlgorithm },
    Sign { key_id: String, data: Vec<u8> },
    Verify { key_id: String, data: Vec<u8>, signature: Vec<u8> },
    Encrypt { key_id: String, plaintext: Vec<u8> },
    Decrypt { key_id: String, ciphertext: Vec<u8> },
    Hash { algorithm: HashAlgorithm, data: Vec<u8> },
    GenerateRandom { length: usize },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum KeyAlgorithm {
    Ed25519,
    P256,
    Rsa2048,
    Rsa4096,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HashAlgorithm {
    Sha256,
    Sha384,
    Sha512,
    Blake3,
}

#[derive(Debug, Clone, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum CryptoError {
    #[error("key not found: {key_id}")]
    KeyNotFound { key_id: String },

    #[error("invalid key: {reason}")]
    InvalidKey { reason: String },

    #[error("signature verification failed")]
    VerificationFailed,

    #[error("decryption failed: {reason}")]
    DecryptionFailed { reason: String },

    #[error("algorithm not supported: {algorithm}")]
    UnsupportedAlgorithm { algorithm: String },

    #[error("secure random unavailable")]
    RandomUnavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CryptoOutput {
    KeyPair { key_id: String, public_key: Vec<u8> },
    Signature(Vec<u8>),
    Verified(bool),
    Encrypted(Vec<u8>),
    Decrypted(Vec<u8>),
    Hash(Vec<u8>),
    Random(Vec<u8>),
}

pub type CryptoResult = Result<CryptoOutput, CryptoError>;
