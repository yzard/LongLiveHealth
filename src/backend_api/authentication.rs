use crate::{
    app::AppState,
    error::AppError,
    models::{LoginRequest, LoginResponse, User},
};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use axum::{extract::FromRequestParts, http::request::Parts};
use rand::{RngCore, rngs::OsRng};
use sha2::{Digest, Sha256};
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::Semaphore;

pub fn now() -> Result<i64, AppError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| AppError::Internal)?
        .as_secs()
        .try_into()
        .map_err(|_| AppError::Internal)
}

pub fn digest(secret: &[u8]) -> String {
    hex::encode(Sha256::digest(secret))
}

pub fn normalize_username(username: &str) -> Result<String, AppError> {
    let username = username.trim().to_ascii_lowercase();
    if !(3..=64).contains(&username.len())
        || !username
            .bytes()
            .all(|character| character.is_ascii_alphanumeric() || b"._-@".contains(&character))
    {
        return Err(AppError::Invalid(
            "Username must be 3-64 ASCII letters, digits or . _ - @",
        ));
    }
    Ok(username)
}

pub(crate) fn validated_password_hash(password: &str) -> Result<String, AppError> {
    if !(12..=1024).contains(&password.len()) {
        return Err(AppError::Invalid("Password must be 12-1024 bytes"));
    }
    encode_password(password)
}

fn encode_password(password: &str) -> Result<String, AppError> {
    Argon2::default()
        .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
        .map(|hashed| hashed.to_string())
        .map_err(|_| AppError::Internal)
}

pub async fn hash_password(password: String) -> Result<String, AppError> {
    tokio::task::spawn_blocking(move || validated_password_hash(&password))
        .await
        .map_err(|_| AppError::Internal)?
}

async fn hash_password_value(password: String) -> Result<String, AppError> {
    tokio::task::spawn_blocking(move || encode_password(&password))
        .await
        .map_err(|_| AppError::Internal)?
}

pub async fn login(state: &AppState, request: LoginRequest) -> Result<LoginResponse, AppError> {
    let username = normalize_username(&request.username).map_err(|_| AppError::Unauthorized)?;
    if request.password.len() > 1024 {
        return Err(AppError::Unauthorized);
    }
    let permit = Arc::clone(&state.password_checks)
        .try_acquire_owned()
        .map_err(|_| AppError::RateLimited)?;
    let current_time = now()?;
    state
        .database
        .reserve_login(
            &digest(username.as_bytes()),
            current_time,
            state.config.security.login_window_seconds,
            state.config.security.login_attempts_per_window,
        )
        .await?;
    let credentials = state.database.credentials(&username).await?;
    let password_hash = credentials
        .as_ref()
        .map(|record| record.password_hash.clone())
        .unwrap_or_else(|| state.dummy_password_hash.clone());
    let matches = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let parsed = PasswordHash::new(&password_hash).map_err(|_| AppError::Internal)?;
        Ok::<bool, AppError>(
            Argon2::default()
                .verify_password(request.password.as_bytes(), &parsed)
                .is_ok(),
        )
    })
    .await
    .map_err(|_| AppError::Internal)??;
    if !matches {
        return Err(AppError::Unauthorized);
    }
    let credentials = credentials.ok_or(AppError::Unauthorized)?;
    let mut random = [0u8; 32];
    OsRng.fill_bytes(&mut random);
    let token = hex::encode(random);
    let expires_at = current_time + state.config.security.session_ttl_seconds;
    OsRng.fill_bytes(&mut random);
    let refresh_token = hex::encode(random);
    let refresh_expires_at = current_time + 90 * 86400;
    state
        .database
        .create_session(
            &digest(token.as_bytes()),
            expires_at,
            Some((&digest(refresh_token.as_bytes()), refresh_expires_at)),
            &credentials,
            current_time,
        )
        .await?;
    Ok(LoginResponse {
        token,
        expires_at,
        refresh_token,
        refresh_expires_at,
        refresh_after: current_time + (state.config.security.session_ttl_seconds / 2).min(1800),
        user: User {
            user_id: credentials.user_id,
            is_admin: credentials.username == "admin",
            must_change_password: credentials.must_change_password,
            username: credentials.username,
        },
    })
}

pub struct CurrentUser {
    pub user: User,
    pub token_hash: String,
}

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = AppError;
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|header| header.to_str().ok())
            .and_then(|header| header.strip_prefix("Bearer "))
            .ok_or(AppError::Unauthorized)?;
        if token.len() != 64 || !token.bytes().all(|character| character.is_ascii_hexdigit()) {
            return Err(AppError::Unauthorized);
        }
        let token_hash = digest(token.as_bytes());
        let user = state.database.session_user(&token_hash, now()?).await?;
        if user.must_change_password
            && !matches!(
                parts.uri.path(),
                "/api/v1/user/password/change" | "/api/v1/user/get" | "/api/v1/session/logout"
            )
        {
            return Err(AppError::PasswordChangeRequired);
        }
        Ok(Self { user, token_hash })
    }
}

pub fn password_semaphore(maximum: usize) -> Arc<Semaphore> {
    Arc::new(Semaphore::new(maximum))
}

/// Called once at service startup, before accepting connections.
pub async fn initialize_admin(
    database: &crate::database::Database,
    reset: bool,
) -> Result<(), AppError> {
    let password_hash = hash_password_value("admin".into()).await?;
    database
        .initialize_admin(&password_hash, reset, now()?)
        .await
}

/// The successor secret is persisted by the client before sending. A lost response
/// can be retried with the same pair without losing the session or storing raw tokens.
pub async fn refresh(
    state: &AppState,
    request: crate::models::RefreshRequest,
) -> Result<LoginResponse, AppError> {
    for secret in [&request.refresh_token, &request.next_refresh_token] {
        if secret.len() != 64 || !secret.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(AppError::Unauthorized);
        }
    }
    if request.refresh_token == request.next_refresh_token {
        return Err(AppError::Unauthorized);
    }
    let time = now()?;
    let token =
        digest(format!("longlivehealth-access-v1:{}", request.next_refresh_token).as_bytes());
    let expires_at = time + state.config.security.session_ttl_seconds.min(3600);
    let refresh_expires_at = time + 90 * 86400;
    let user = state
        .database
        .rotate_session(
            &digest(request.refresh_token.as_bytes()),
            &digest(request.next_refresh_token.as_bytes()),
            &digest(token.as_bytes()),
            expires_at,
            refresh_expires_at,
            time,
        )
        .await?;
    Ok(LoginResponse {
        token,
        expires_at,
        refresh_token: request.next_refresh_token,
        refresh_expires_at,
        refresh_after: time + (state.config.security.session_ttl_seconds / 2).min(1800),
        user,
    })
}
