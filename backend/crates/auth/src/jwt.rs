//! JWT access + refresh token issue/verify.

use anylive_common::{AppError, ErrorCode};
use anylive_domain::UserId;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Access token lifetime (15 minutes).
pub const ACCESS_TTL_SECS: i64 = 15 * 60;
/// Refresh token lifetime (30 days).
pub const REFRESH_TTL_SECS: i64 = 30 * 24 * 60 * 60;

#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub access_secret: String,
    pub refresh_secret: String,
    pub access_ttl_secs: i64,
    pub refresh_ttl_secs: i64,
    pub issuer: String,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            access_secret: "dev-access-secret-change-me-32chars".into(),
            refresh_secret: "dev-refresh-secret-change-me-32chars".into(),
            access_ttl_secs: ACCESS_TTL_SECS,
            refresh_ttl_secs: REFRESH_TTL_SECS,
            issuer: "anylive".into(),
        }
    }
}

impl JwtConfig {
    pub fn from_env() -> Self {
        let mut cfg = Self::default();
        if let Ok(s) = std::env::var("JWT_ACCESS_SECRET") {
            cfg.access_secret = s;
        }
        if let Ok(s) = std::env::var("JWT_REFRESH_SECRET") {
            cfg.refresh_secret = s;
        }
        cfg
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    /// Access token lifetime in seconds.
    pub expires_in: i64,
}

/// Access JWT claims: `{ sub, exp, typ=access }` (+ iat/iss/email helpers).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccessClaims {
    pub sub: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    pub iss: String,
    pub exp: i64,
    pub iat: i64,
    /// Token type discriminator (`"access"`).
    pub typ: String,
}

/// Refresh JWT claims: `{ sub, exp, jti, typ=refresh }` (+ iat/iss).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RefreshClaims {
    pub sub: Uuid,
    pub jti: Uuid,
    pub iss: String,
    pub exp: i64,
    pub iat: i64,
    /// Token type discriminator (`"refresh"`).
    pub typ: String,
}

#[derive(Clone)]
pub struct JwtService {
    config: JwtConfig,
    access_enc: EncodingKey,
    access_dec: DecodingKey,
    refresh_enc: EncodingKey,
    refresh_dec: DecodingKey,
}

impl JwtService {
    pub fn new(config: JwtConfig) -> Self {
        let access_enc = EncodingKey::from_secret(config.access_secret.as_bytes());
        let access_dec = DecodingKey::from_secret(config.access_secret.as_bytes());
        let refresh_enc = EncodingKey::from_secret(config.refresh_secret.as_bytes());
        let refresh_dec = DecodingKey::from_secret(config.refresh_secret.as_bytes());
        Self {
            config,
            access_enc,
            access_dec,
            refresh_enc,
            refresh_dec,
        }
    }

    pub fn access_ttl_secs(&self) -> i64 {
        self.config.access_ttl_secs
    }

    pub fn issue_pair(&self, user_id: UserId, email: Option<String>) -> Result<IssuedTokens, AppError> {
        let now = Utc::now();
        let iat = now.timestamp();
        let access_exp = (now + Duration::seconds(self.config.access_ttl_secs)).timestamp();
        let refresh_exp = (now + Duration::seconds(self.config.refresh_ttl_secs)).timestamp();
        let jti = Uuid::new_v4();

        let access_claims = AccessClaims {
            sub: user_id.0,
            email,
            iss: self.config.issuer.clone(),
            exp: access_exp,
            iat,
            typ: "access".into(),
        };
        let refresh_claims = RefreshClaims {
            sub: user_id.0,
            jti,
            iss: self.config.issuer.clone(),
            exp: refresh_exp,
            iat,
            typ: "refresh".into(),
        };

        let access_token = encode(&Header::default(), &access_claims, &self.access_enc)
            .map_err(|e| AppError::new(ErrorCode::Internal, format!("encode access: {e}")))?;
        let refresh_token = encode(&Header::default(), &refresh_claims, &self.refresh_enc)
            .map_err(|e| AppError::new(ErrorCode::Internal, format!("encode refresh: {e}")))?;

        Ok(IssuedTokens {
            pair: TokenPair {
                access_token,
                refresh_token,
                expires_in: self.config.access_ttl_secs,
            },
            refresh_jti: jti,
            user_id,
            refresh_exp,
        })
    }

    pub fn verify_access(&self, token: &str) -> Result<AccessClaims, AppError> {
        let mut validation = Validation::default();
        validation.set_issuer(&[&self.config.issuer]);
        validation.validate_exp = true;
        let data = decode::<AccessClaims>(token, &self.access_dec, &validation).map_err(|e| {
            tracing::debug!(error = %e, "access token verify failed");
            AppError::unauthorized("invalid or expired access token")
        })?;
        if data.claims.typ != "access" {
            return Err(AppError::unauthorized("invalid token type"));
        }
        Ok(data.claims)
    }

    pub fn verify_refresh(&self, token: &str) -> Result<RefreshClaims, AppError> {
        let mut validation = Validation::default();
        validation.set_issuer(&[&self.config.issuer]);
        validation.validate_exp = true;
        let data = decode::<RefreshClaims>(token, &self.refresh_dec, &validation).map_err(|e| {
            tracing::debug!(error = %e, "refresh token verify failed");
            AppError::unauthorized("invalid or expired refresh token")
        })?;
        if data.claims.typ != "refresh" {
            return Err(AppError::unauthorized("invalid token type"));
        }
        Ok(data.claims)
    }
}

#[derive(Debug, Clone)]
pub struct IssuedTokens {
    pub pair: TokenPair,
    pub refresh_jti: Uuid,
    pub user_id: UserId,
    pub refresh_exp: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_and_verify_access() {
        let jwt = JwtService::new(JwtConfig::default());
        let uid = UserId::new();
        let issued = jwt
            .issue_pair(uid, Some("a@example.com".into()))
            .unwrap();
        let claims = jwt.verify_access(&issued.pair.access_token).unwrap();
        assert_eq!(claims.sub, uid.0);
        assert_eq!(claims.email.as_deref(), Some("a@example.com"));
        assert_eq!(claims.typ, "access");
        assert_eq!(issued.pair.expires_in, ACCESS_TTL_SECS);
    }

    #[test]
    fn refresh_claims_include_jti() {
        let jwt = JwtService::new(JwtConfig::default());
        let uid = UserId::new();
        let issued = jwt.issue_pair(uid, None).unwrap();
        let claims = jwt.verify_refresh(&issued.pair.refresh_token).unwrap();
        assert_eq!(claims.sub, uid.0);
        assert_eq!(claims.jti, issued.refresh_jti);
        assert_eq!(claims.typ, "refresh");
    }

    #[test]
    fn reject_access_as_refresh() {
        let jwt = JwtService::new(JwtConfig::default());
        let issued = jwt.issue_pair(UserId::new(), None).unwrap();
        assert!(jwt.verify_refresh(&issued.pair.access_token).is_err());
        assert!(jwt.verify_access(&issued.pair.refresh_token).is_err());
    }

    #[test]
    fn reject_tampered_token() {
        let jwt = JwtService::new(JwtConfig::default());
        let issued = jwt.issue_pair(UserId::new(), None).unwrap();
        let mut bad = issued.pair.access_token;
        bad.push('x');
        assert!(jwt.verify_access(&bad).is_err());
    }
}
