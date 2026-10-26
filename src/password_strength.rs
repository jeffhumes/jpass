use crate::model::{Vault, VaultEntry};

const COMMON_PASSWORDS: &[&str] = &[
    "12345678",
    "123456789",
    "1234567890",
    "password",
    "password1",
    "password123",
    "qwerty",
    "qwerty123",
    "letmein",
    "welcome",
    "admin",
    "iloveyou",
];

pub fn validate_entry_password(
    password: &str,
    entry: &VaultEntry,
    vault: &Vault,
) -> Result<(), String> {
    if password.is_empty() {
        return Err("Password cannot be empty.".into());
    }
    if password.chars().count() < 8 {
        return Err("Password must be at least 8 characters.".into());
    }

    let normalized = password.to_lowercase();
    if COMMON_PASSWORDS.contains(&normalized.as_str()) {
        return Err("Choose a less common password.".into());
    }
    if normalized == entry.username.to_lowercase() || normalized == entry.title.to_lowercase() {
        return Err("Password must not match the entry title or username.".into());
    }
    if password.chars().all(|character| character == password.chars().next().unwrap()) {
        return Err("Password must not repeat a single character.".into());
    }

    let classes = [
        password.chars().any(|character| character.is_ascii_lowercase()),
        password.chars().any(|character| character.is_ascii_uppercase()),
        password.chars().any(|character| character.is_ascii_digit()),
        password
            .chars()
            .any(|character| !character.is_ascii_alphanumeric()),
    ]
    .into_iter()
    .filter(|present| *present)
    .count();
    if classes < 3 {
        return Err("Use at least three of lowercase, uppercase, numbers, and symbols.".into());
    }

    if vault
        .entries
        .iter()
        .any(|candidate| candidate.id != entry.id && candidate.password == password)
    {
        return Err("Do not reuse a password from another entry.".into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry() -> VaultEntry {
        VaultEntry::new(
            "GitHub".into(),
            "alice".into(),
            "Old-Pass1!".into(),
            String::new(),
            String::new(),
        )
    }

    #[test]
    fn accepts_a_strong_unique_password() {
        let vault = Vault::default();
        assert!(validate_entry_password("S3cure!Vault", &entry(), &vault).is_ok());
    }

    #[test]
    fn rejects_common_and_weak_passwords() {
        let vault = Vault::default();
        assert!(validate_entry_password("password123", &entry(), &vault).is_err());
        assert!(validate_entry_password("aaaaaaaa", &entry(), &vault).is_err());
        assert!(validate_entry_password("lowercase", &entry(), &vault).is_err());
    }

    #[test]
    fn rejects_reuse_but_allows_current_entry_password() {
        let mut vault = Vault::default();
        let current = entry();
        let mut other = entry();
        other.id = uuid::Uuid::new_v4();
        other.password = "S3cure!Vault".into();
        vault.add_entry(other);

        assert!(validate_entry_password("S3cure!Vault", &current, &vault).is_err());
        assert!(validate_entry_password("Old-Pass1!", &current, &vault).is_ok());
    }
}