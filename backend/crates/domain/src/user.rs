use crate::{Timestamp, UserId};
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct User {
    pub id: UserId,
    pub display_name: String,
    pub email: Option<String>,
    pub created_at: Timestamp,
}

impl User {
    /// Validate and normalize a display name (1..=64 chars after trim).
    pub fn validate_display_name(name: impl Into<String>) -> Result<String, UserError> {
        let display_name = name.into().trim().to_string();
        if display_name.is_empty() || display_name.len() > 64 {
            return Err(UserError::InvalidDisplayName);
        }
        Ok(display_name)
    }

    pub fn new(display_name: impl Into<String>, email: Option<String>) -> Result<Self, UserError> {
        let display_name = Self::validate_display_name(display_name)?;
        if let Some(ref e) = email {
            if !e.contains('@') || e.len() > 254 {
                return Err(UserError::InvalidEmail);
            }
        }
        Ok(Self {
            id: UserId::new(),
            display_name,
            email,
            created_at: Utc::now(),
        })
    }

    /// Apply a display-name change in place (domain-level, no IO).
    pub fn with_display_name(mut self, name: impl Into<String>) -> Result<Self, UserError> {
        self.display_name = Self::validate_display_name(name)?;
        Ok(self)
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum UserError {
    #[error("invalid display name")]
    InvalidDisplayName,
    #[error("invalid email")]
    InvalidEmail,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_user_ok() {
        let u = User::new("Alice", Some("a@example.com".into())).unwrap();
        assert_eq!(u.display_name, "Alice");
    }

    #[test]
    fn reject_bad_email() {
        assert_eq!(
            User::new("Bob", Some("not-an-email".into())).unwrap_err(),
            UserError::InvalidEmail
        );
    }

    #[test]
    fn reject_empty_name() {
        assert!(User::new("  ", None).is_err());
    }

    #[test]
    fn validate_display_name_trims_and_bounds() {
        assert_eq!(
            User::validate_display_name("  Alice  ").unwrap(),
            "Alice"
        );
        assert_eq!(
            User::validate_display_name(""),
            Err(UserError::InvalidDisplayName)
        );
        assert_eq!(
            User::validate_display_name("a".repeat(65)),
            Err(UserError::InvalidDisplayName)
        );
        assert!(User::validate_display_name("a".repeat(64)).is_ok());
    }

    #[test]
    fn with_display_name_updates() {
        let u = User::new("Alice", None)
            .unwrap()
            .with_display_name("Bob")
            .unwrap();
        assert_eq!(u.display_name, "Bob");
        assert!(User::new("Alice", None)
            .unwrap()
            .with_display_name("  ")
            .is_err());
    }
}
