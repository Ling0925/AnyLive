//! JWT access + refresh token issue/verify.
//!
//! Hard requirements:
//! - Algorithm pinned to **HS256 only** (no `none` / alg confusion).
//! - Access and refresh use **separate secrets**.
//! - Secrets must be ≥ 32 bytes when loaded from the environment.

use anylive_common::{AppError, ErrorCode};
use anylive_domain::UserId;
use chrono::{Duration, Utc};
use jsonwebtoken::{
    decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Access token lifetime (15 minutes).
pub const ACCESS_TTL_SECS: i64 = 15 * 60;
/// Refresh token lifetime (30 days).
pub const REFRESH_TTL_SECS: i64 = 30 * 24 * 60 * 60;
/// Minimum secret length accepted from environment / production config.
pub const MIN_SECRET_LEN: usize = 32;

/// Only algorithm we ever encode or accept.
const JWT_ALG: Algorithm = Algorithm::HS256;

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
        // Local/test defaults only — never ship with these in production.
        Self {
            access_secret: "dev-access-secret-change-me-32chars!!".into(),
            refresh_secret: "dev-refresh-secret-change-me-32chars!".into(),
            access_ttl_secs: ACCESS_TTL_SECS,
            refresh_ttl_secs: REFRESH_TTL_SECS,
            issuer: "anylive".into(),
        }
    }
}

impl JwtConfig {
    /// Load secrets from `JWT_ACCESS_SECRET` / `JWT_REFRESH_SECRET`.
    ///
    /// Falls back to dev defaults only when `ALLOW_INSECURE_JWT=1` or
    /// `APP_ENV=local` (or unset treated as local). Staging/prod without secrets
    /// panic. Rejects secrets shorter than [`MIN_SECRET_LEN`]. Access and refresh
    /// secrets must differ.
    pub fn from_env() -> Self {
        let access = std::env::var("JWT_ACCESS_SECRET").ok();
        let refresh = std::env::var("JWT_REFRESH_SECRET").ok();
        let allow_insecure = crate::otp::env_flag_enabled("ALLOW_INSECURE_JWT")
            || is_local_app_env();

        let mut cfg = Self::default();
        match (access, refresh) {
            (Some(a), Some(r)) => {
                cfg.access_secret = a;
                cfg.refresh_secret = r;
            }
            (None, None) if allow_insecure => {
                tracing::warn!(
                    "JWT_ACCESS_SECRET / JWT_REFRESH_SECRET unset; using insecure dev defaults"
                );
            }
            _ => {
                panic!(
                    "JWT_ACCESS_SECRET and JWT_REFRESH_SECRET must both be set \
                     (or set ALLOW_INSECURE_JWT=1 / APP_ENV=local for local only)"
                );
            }
        }
        if let Err(e) = cfg.validate() {
            // Fail closed for misconfigured secrets.
            panic!("invalid JWT config: {e}");
        }
        cfg
    }

    /// Validate secret strength and separation.
    pub fn validate(&self) -> Result<(), String> {
        if self.access_secret.len() < MIN_SECRET_LEN {
            return Err(format!(
                "JWT access secret must be at least {MIN_SECRET_LEN} bytes"
            ));
        }
        if self.refresh_secret.len() < MIN_SECRET_LEN {
            return Err(format!(
                "JWT refresh secret must be at least {MIN_SECRET_LEN} bytes"
            ));
        }
        if self.access_secret == self.refresh_secret {
            return Err("JWT access and refresh secrets must be distinct".into());
        }
        Ok(())
    }
}

fn is_local_app_env() -> bool {
    match std::env::var("APP_ENV") {
        Ok(v) => {
            let v = v.trim().to_ascii_lowercase();
            v.is_empty() || v == "local" || v == "development" || v == "dev" || v == "test"
        }
        // Unset APP_ENV is treated as local for offline/dev ergonomics.
        Err(_) => true,
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
        // Soft-check in constructor; from_env already panics on bad secrets.
        if let Err(e) = config.validate() {
            tracing::warn!(error = %e, "JWT config validation warning");
        }
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

    fn hs256_header() -> Header {
        Header::new(JWT_ALG)
    }

    fn hs256_validation(&self) -> Validation {
        let mut validation = Validation::new(JWT_ALG);
        validation.set_issuer(&[&self.config.issuer]);
        validation.validate_exp = true;
        // Explicit allow-list — never accept `none` or other algs.
        validation.algorithms = vec![JWT_ALG];
        validation
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

        let access_token = encode(&Self::hs256_header(), &access_claims, &self.access_enc)
            .map_err(|e| AppError::new(ErrorCode::Internal, format!("encode access: {e}")))?;
        let refresh_token = encode(&Self::hs256_header(), &refresh_claims, &self.refresh_enc)
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
        let validation = self.hs256_validation();
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
        let validation = self.hs256_validation();
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

    #[test]
    fn reject_wrong_secret() {
        let a = JwtService::new(JwtConfig::default());
        let issued = a.issue_pair(UserId::new(), None).unwrap();
        let mut other = JwtConfig::default();
        other.access_secret = "other-access-secret-at-least-32b!!!!".into();
        other.refresh_secret = "other-refresh-secret-at-least-32b!!!".into();
        let b = JwtService::new(other);
        assert!(b.verify_access(&issued.pair.access_token).is_err());
    }

    #[test]
    fn reject_wrong_issuer() {
        let mut cfg = JwtConfig::default();
        cfg.issuer = "other-iss".into();
        let foreign = JwtService::new(cfg);
        let issued = foreign.issue_pair(UserId::new(), None).unwrap();
        let local = JwtService::new(JwtConfig::default());
        assert!(local.verify_access(&issued.pair.access_token).is_err());
    }

    #[test]
    fn tokens_are_hs256() {
        let jwt = JwtService::new(JwtConfig::default());
        let issued = jwt.issue_pair(UserId::new(), None).unwrap();
        // Standard HS256 JWT header `{"alg":"HS256","typ":"JWT"}` base64url-encodes to this.
        let header = issued.pair.access_token.split('.').next().unwrap();
        assert_eq!(header, "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9");
        let refresh_header = issued.pair.refresh_token.split('.').next().unwrap();
        assert_eq!(refresh_header, "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9");
    }

    #[test]
    fn config_rejects_short_or_shared_secrets() {
        let mut cfg = JwtConfig::default();
        cfg.access_secret = "short".into();
        assert!(cfg.validate().is_err());
        cfg = JwtConfig::default();
        cfg.refresh_secret = cfg.access_secret.clone();
        assert!(cfg.validate().is_err());
        assert!(JwtConfig::default().validate().is_ok());
    }
}
