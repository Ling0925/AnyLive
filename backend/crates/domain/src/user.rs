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
    pub fn new(display_name: impl Into<String>, email: Option<String>) -> Result<Self, UserError> {
        let display_name = display_name.into().trim().to_string();
        if display_name.is_empty() || display_name.len() > 64 {
            return Err(UserError::InvalidDisplayName);
        }
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
}
