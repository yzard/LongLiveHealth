use crate::{
    app::AppState,
    authentication::{self, CurrentUser},
    error::AppError,
    models::{ListRequest, LoginRequest},
};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{Json, extract::State};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PasswordChange {
    current_password: String,
    new_password: String,
}

pub async fn change_password(
    State(state): State<AppState>,
    current: CurrentUser,
    Json(request): Json<PasswordChange>,
) -> Result<Json<Value>, AppError> {
    if request.current_password.len() > 1024 || request.new_password == request.current_password {
        return Err(AppError::Invalid("Choose a different password"));
    }
    let permit = state
        .password_checks
        .clone()
        .try_acquire_owned()
        .map_err(|_| AppError::RateLimited)?;
    state
        .database
        .reserve_login(
            &authentication::digest(current.user.username.as_bytes()),
            authentication::now()?,
            state.config.security.login_window_seconds,
            state.config.security.login_attempts_per_window,
        )
        .await?;
    let credentials = state
        .database
        .credentials(&current.user.username)
        .await?
        .ok_or(AppError::Unauthorized)?;
    let hash = credentials.password_hash;
    let new_hash = tokio::task::spawn_blocking(move || {
        // The CPU operation owns its slot even if the HTTP request is canceled.
        let _permit = permit;
        let parsed = PasswordHash::new(&hash).map_err(|_| AppError::Internal)?;
        if Argon2::default()
            .verify_password(request.current_password.as_bytes(), &parsed)
            .is_err()
        {
            return Err(AppError::Unauthorized);
        }
        authentication::validated_password_hash(&request.new_password)
    })
    .await
    .map_err(|_| AppError::Internal)??;
    state
        .database
        .change_own_password(
            &current.user.user_id,
            credentials.credential_version,
            &current.token_hash,
            &new_hash,
            authentication::now()?,
        )
        .await?;
    Ok(Json(json!({"changed": true, "sessions_revoked": true})))
}

fn require_admin(current: &CurrentUser) -> Result<(), AppError> {
    if !current.user.is_admin {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

pub async fn list(
    State(state): State<AppState>,
    current: CurrentUser,
    Json(request): Json<ListRequest>,
) -> Result<Json<Value>, AppError> {
    require_admin(&current)?;
    Ok(Json(
        json!({"users": state.database.users(&request).await?}),
    ))
}

pub async fn create(
    State(state): State<AppState>,
    current: CurrentUser,
    Json(request): Json<LoginRequest>,
) -> Result<Json<crate::models::User>, AppError> {
    require_admin(&current)?;
    let username = authentication::normalize_username(&request.username)?;
    if username == "admin" {
        return Err(AppError::Forbidden);
    }
    let permit = state
        .password_checks
        .clone()
        .try_acquire_owned()
        .map_err(|_| AppError::RateLimited)?;
    let hash = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        authentication::validated_password_hash(&request.password)
    })
    .await
    .map_err(|_| AppError::Internal)??;
    Ok(Json(
        state
            .database
            .create_user(&username, &hash, authentication::now()?)
            .await?,
    ))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeleteUser {
    user_id: String,
    confirmation: String,
}

pub async fn delete(
    State(state): State<AppState>,
    current: CurrentUser,
    Json(request): Json<DeleteUser>,
) -> Result<Json<Value>, AppError> {
    require_admin(&current)?;
    let _guard = state.file_mutation.lock().await;
    let username = state.database.username_for_id(&request.user_id).await?;
    if username != request.confirmation {
        return Err(AppError::Invalid("Confirm the username to delete"));
    }
    state.database.delete_account(&request.user_id).await?;
    Ok(Json(
        json!({"user_id":request.user_id,"cleanup_queued":true,"sessions_revoked":true}),
    ))
}
