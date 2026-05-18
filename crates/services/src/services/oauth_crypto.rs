//! AES-256-GCM at-rest encryption for OAuth credentials.
//!
//! Tokens persisted in `integration_connections.access_token_ciphertext` and
//! `refresh_token_ciphertext` are encrypted with this module before they hit
//! the database. The format is:
//!
//! ```text
//! base64( nonce(12 bytes) || ciphertext_with_tag )
//! ```
//!
//! ## Key management
//!
//! The 32-byte master key is read from `OAUTH_ENCRYPTION_KEY` (base64). If unset
//! the module falls back to a deterministic dev key — fine for local dev, never
//! safe for production. A loud warning fires every time the fallback is used.

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    AeadCore, Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("encryption failed: {0}")]
    Encryption(String),
    #[error("decryption failed: {0}")]
    Decryption(String),
    #[error("base64 decode failed: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("invalid OAUTH_ENCRYPTION_KEY length: expected 32 bytes, got {0}")]
    KeyLength(usize),
}

const NONCE_LEN: usize = 12;

fn load_master_key() -> Result<Key<Aes256Gcm>, CryptoError> {
    let raw = std::env::var("OAUTH_ENCRYPTION_KEY").unwrap_or_else(|_| {
        tracing::warn!(
            "OAUTH_ENCRYPTION_KEY is not set — using insecure dev fallback. \
             Generate a 32-byte key with `openssl rand -base64 32` for production."
        );
        // Deterministic fallback so dev DBs encrypt/decrypt across restarts.
        // Bytes are arbitrary; the only requirement is consistency.
        B64.encode([0x42u8; 32])
    });
    let bytes = B64.decode(raw.trim())?;
    if bytes.len() != 32 {
        return Err(CryptoError::KeyLength(bytes.len()));
    }
    Ok(*Key::<Aes256Gcm>::from_slice(&bytes))
}

pub fn encrypt(plaintext: &str) -> Result<String, CryptoError> {
    let key = load_master_key()?;
    let cipher = Aes256Gcm::new(&key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ct = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| CryptoError::Encryption(e.to_string()))?;

    let mut out = Vec::with_capacity(NONCE_LEN + ct.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    Ok(B64.encode(out))
}

pub fn decrypt(ciphertext_b64: &str) -> Result<String, CryptoError> {
    let key = load_master_key()?;
    let cipher = Aes256Gcm::new(&key);
    let raw = B64.decode(ciphertext_b64)?;
    if raw.len() < NONCE_LEN {
        return Err(CryptoError::Decryption("ciphertext too short".into()));
    }
    let (nonce_bytes, body) = raw.split_at(NONCE_LEN);
    let nonce = Nonce::from_slice(nonce_bytes);
    let pt = cipher
        .decrypt(nonce, body)
        .map_err(|e| CryptoError::Decryption(e.to_string()))?;
    String::from_utf8(pt).map_err(|e| CryptoError::Decryption(e.to_string()))
}

pub fn encrypt_optional(value: Option<&str>) -> Result<Option<String>, CryptoError> {
    value.map(encrypt).transpose()
}

pub fn decrypt_optional(value: Option<&str>) -> Result<Option<String>, CryptoError> {
    value.map(|v| decrypt(v)).transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let pt = "ya29.a0AfH6SMC...example_oauth_token";
        let ct = encrypt(pt).unwrap();
        assert_ne!(ct, pt);
        assert_eq!(decrypt(&ct).unwrap(), pt);
    }

    #[test]
    fn each_encryption_uses_fresh_nonce() {
        let pt = "secret";
        assert_ne!(encrypt(pt).unwrap(), encrypt(pt).unwrap());
    }

    #[test]
    fn tamper_detected() {
        let ct = encrypt("hello").unwrap();
        let mut bytes = B64.decode(&ct).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 0x01;
        let tampered = B64.encode(bytes);
        assert!(decrypt(&tampered).is_err());
    }
}
