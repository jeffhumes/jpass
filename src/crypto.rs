//! Encryption primitives used to protect the vault at rest.
//!
//! The vault (a JSON document) is encrypted as a single blob using AES-256-GCM.
//! The encryption key is derived from the user's master password with Argon2id,
//! using a random salt that is stored alongside the ciphertext.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
pub use jpass_core::EncryptedBlob;
use rand::{rngs::OsRng, RngCore};
use zeroize::Zeroize;

const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;
const ARGON2_MEMORY_KIB: u32 = 64 * 1024;
const ARGON2_TIME_COST: u32 = 3;
const ARGON2_PARALLELISM: u32 = 1;
const LEGACY_ARGON2_MEMORY_KIB: u32 = 19_456;
const LEGACY_ARGON2_TIME_COST: u32 = 2;
const LEGACY_ARGON2_PARALLELISM: u32 = 1;

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("failed to derive key from password")]
    KeyDerivation,
    #[error("failed to encrypt data")]
    Encryption,
    #[error("incorrect master password or corrupted vault")]
    Decryption,
}

fn derive_key_with_params(
    password: &str,
    salt: &[u8],
    memory_kib: u32,
    time_cost: u32,
    parallelism: u32,
) -> Result<[u8; KEY_LEN], CryptoError> {
    let mut key = [0u8; KEY_LEN];
    let params = Params::new(memory_kib, time_cost, parallelism, Some(KEY_LEN))
        .map_err(|_| CryptoError::KeyDerivation)?;
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|_| CryptoError::KeyDerivation)?;
    Ok(key)
}

fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; KEY_LEN], CryptoError> {
    derive_key_with_params(
        password,
        salt,
        ARGON2_MEMORY_KIB,
        ARGON2_TIME_COST,
        ARGON2_PARALLELISM,
    )
}

/// Encrypt `plaintext` with a key derived from `password`, generating a fresh
/// random salt and nonce.
pub fn encrypt(plaintext: &[u8], password: &str) -> Result<EncryptedBlob, CryptoError> {
    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);

    let mut key_bytes = derive_key(password, &salt)?;
    let cipher = Aes256Gcm::new(&Key::<Aes256Gcm>::from(key_bytes));
    let nonce = Nonce::from(nonce_bytes);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|_| CryptoError::Encryption)?;
    key_bytes.zeroize();

    Ok(EncryptedBlob {
        salt: B64.encode(salt),
        nonce: B64.encode(nonce_bytes),
        ciphertext: B64.encode(ciphertext),
        vault_id: None,
        vault_name: None,
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
    let mut key_bytes = derive_key(password, &salt)?;
    let cipher = Aes256Gcm::new(&Key::<Aes256Gcm>::from(key_bytes));
    let nonce = Nonce::from(nonce_bytes);
    let plaintext = cipher.decrypt(&nonce, ciphertext.as_ref());
    key_bytes.zeroize();
    if let Ok(plaintext) = plaintext {
        return Ok(plaintext);
    }

    // Older vaults used Argon2's default cost. Keep them readable so the app
    // can unlock and re-save them using the stronger current policy.
    let mut legacy_key = derive_key_with_params(
        password,
        &salt,
        LEGACY_ARGON2_MEMORY_KIB,
        LEGACY_ARGON2_TIME_COST,
        LEGACY_ARGON2_PARALLELISM,
    )?;
    let legacy_cipher = Aes256Gcm::new(&Key::<Aes256Gcm>::from(legacy_key));
    let plaintext = legacy_cipher
        .decrypt(&nonce, ciphertext.as_ref())
        .map_err(|_| CryptoError::Decryption);
    legacy_key.zeroize();
    plaintext
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

    #[test]
    fn tampered_ciphertext_fails_authentication() {
        let mut blob = encrypt(b"sensitive vault data", "strong test password").unwrap();
        let mut ciphertext = B64.decode(&blob.ciphertext).unwrap();
        ciphertext[0] ^= 0x01;
        blob.ciphertext = B64.encode(ciphertext);

        assert!(decrypt(&blob, "strong test password").is_err());
    }
}
