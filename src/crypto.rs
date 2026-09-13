//! Encryption primitives used to protect the vault at rest.
//!
//! The vault (a JSON document) is encrypted as a single blob using AES-256-GCM.
//! The encryption key is derived from the user's master password with Argon2id,
//! using a random salt that is stored alongside the ciphertext.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use argon2::Argon2;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};

const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("failed to derive key from password")]
    KeyDerivation,
    #[error("failed to encrypt data")]
    Encryption,
    #[error("incorrect master password or corrupted vault")]
    Decryption,
}

/// The persisted, encrypted form of the vault. All byte fields are base64
/// encoded so this struct can be stored as-is in SQLite, JSON, or browser
/// storage without any extra conversion layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedBlob {
    pub salt: String,
    pub nonce: String,
    pub ciphertext: String,
}

fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; KEY_LEN], CryptoError> {
    let mut key = [0u8; KEY_LEN];
    Argon2::default()
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|_| CryptoError::KeyDerivation)?;
    Ok(key)
}

/// Encrypt `plaintext` with a key derived from `password`, generating a fresh
/// random salt and nonce.
pub fn encrypt(plaintext: &[u8], password: &str) -> Result<EncryptedBlob, CryptoError> {
    let mut salt = [0u8; SALT_LEN];
    rand::thread_rng().fill_bytes(&mut salt);
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);

    let key_bytes = derive_key(password, &salt)?;
    let cipher = Aes256Gcm::new(&Key::<Aes256Gcm>::from(key_bytes));
    let nonce = Nonce::from(nonce_bytes);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|_| CryptoError::Encryption)?;

    Ok(EncryptedBlob {
        salt: B64.encode(salt),
        nonce: B64.encode(nonce_bytes),
        ciphertext: B64.encode(ciphertext),
    })
}

/// Decrypt a blob previously produced by [`encrypt`] using `password`.
pub fn decrypt(blob: &EncryptedBlob, password: &str) -> Result<Vec<u8>, CryptoError> {
    let salt = B64
        .decode(&blob.salt)
        .map_err(|_| CryptoError::Decryption)?;
    let nonce_bytes = B64
        .decode(&blob.nonce)
        .map_err(|_| CryptoError::Decryption)?;
    let ciphertext = B64
        .decode(&blob.ciphertext)
        .map_err(|_| CryptoError::Decryption)?;

    let nonce_bytes: [u8; NONCE_LEN] = nonce_bytes
        .try_into()
        .map_err(|_| CryptoError::Decryption)?;
    let key_bytes = derive_key(password, &salt)?;
    let cipher = Aes256Gcm::new(&Key::<Aes256Gcm>::from(key_bytes));
    let nonce = Nonce::from(nonce_bytes);
    cipher
        .decrypt(&nonce, ciphertext.as_ref())
        .map_err(|_| CryptoError::Decryption)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_and_decrypt_round_trip() {
        let plaintext = b"{\"entries\":[]}";
        let password = "correct horse battery staple";

        let blob = encrypt(plaintext, password).unwrap();
        let decrypted = decrypt(&blob, password).unwrap();

        assert_eq!(decrypted.as_slice(), plaintext);
        assert!(!blob.salt.is_empty());
        assert!(!blob.nonce.is_empty());
        assert!(!blob.ciphertext.is_empty());
    }

    #[test]
    fn wrong_password_fails() {
        let plaintext = b"{\"entries\":[1,2,3]}";
        let blob = encrypt(plaintext, "correct horse battery staple").unwrap();

        assert!(decrypt(&blob, "wrong password").is_err());
    }
}
