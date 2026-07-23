use crate::{Timestamp, UserId};
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Account lifecycle status (login gate).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum UserStatus {
    #[default]
    Active,
    Disabled,
    Deleted,
}

impl UserStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Disabled => "disabled",
            Self::Deleted => "deleted",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "active" => Some(Self::Active),
            "disabled" => Some(Self::Disabled),
            "deleted" => Some(Self::Deleted),
            _ => None,
        }
    }

    pub fn can_login(self) -> bool {
        matches!(self, Self::Active)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct User {
    pub id: UserId,
    pub display_name: String,
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default)]
    pub status: UserStatus,
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

    /// Username: 3..=32, ascii alphanumeric + `_` `.` `-`, not email-shaped.
    pub fn validate_username(name: impl Into<String>) -> Result<String, UserError> {
        let username = name.into().trim().to_ascii_lowercase();
        if username.len() < 3 || username.len() > 32 {
            return Err(UserError::InvalidUsername);
        }
        if username.contains('@') {
            return Err(UserError::InvalidUsername);
        }
        if !username
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-')
        {
            return Err(UserError::InvalidUsername);
        }
        if !username.chars().next().is_some_and(|c| c.is_ascii_alphanumeric()) {
            return Err(UserError::InvalidUsername);
        }
        Ok(username)
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
            username: None,
            status: UserStatus::Active,
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
    #[error("invalid username")]
    InvalidUsername,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_user_ok() {
        let u = User::new("Alice", Some("a@example.com".into())).unwrap();
        assert_eq!(u.display_name, "Alice");
        assert_eq!(u.status, UserStatus::Active);
        assert!(u.username.is_none());
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

    #[test]
    fn username_rules() {
        assert_eq!(User::validate_username("Ab_C1").unwrap(), "ab_c1");
        assert!(User::validate_username("ab").is_err());
        assert!(User::validate_username("a@b.com").is_err());
        assert!(User::validate_username("_leading").is_err());
    }

    #[test]
    fn status_can_login() {
        assert!(UserStatus::Active.can_login());
        assert!(!UserStatus::Disabled.can_login());
        assert!(!UserStatus::Deleted.can_login());
    }
}
