use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use axum_extra::extract::cookie::CookieJar;
use rand::RngCore;
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{error::AppError, state::AppState};

pub const SESSION_COOKIE: &str = "horasculo_session";

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct CurrentUser {
    pub id: Uuid,
    pub username: String,
    pub name: String,
    pub role: String,
}

impl CurrentUser {
    pub fn require_admin(&self) -> Result<(), AppError> {
        if self.role == "admin" { Ok(()) } else { Err(AppError::Forbidden) }
    }
}

impl<S> FromRequestParts<S> for CurrentUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let state = AppState::from_ref(state);
        let jar = CookieJar::from_headers(&parts.headers);
        let token = jar.get(SESSION_COOKIE).ok_or(AppError::Unauthorized)?.value();
        let token_hash = hash_token(token);
        sqlx::query_as::<_, CurrentUser>(
            "SELECT u.id, u.username::text AS username, u.name, u.role::text AS role
             FROM sessions s JOIN users u ON u.id = s.user_id
             WHERE s.token_hash = $1 AND s.expires_at > now() AND u.active = true",
        )
        .bind(token_hash)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::Unauthorized)
    }
}

pub async fn hash_password(password: String) -> Result<String, AppError> {
    tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|error| {
                tracing::error!(?error, "falha ao gerar hash de senha");
                AppError::Internal
            })
    })
    .await
    .map_err(|error| {
        tracing::error!(?error, "falha na tarefa de hash");
        AppError::Internal
    })?
}

pub async fn verify_password(password: String, encoded: String) -> Result<bool, AppError> {
    tokio::task::spawn_blocking(move || {
        let parsed = PasswordHash::new(&encoded).map_err(|error| {
            tracing::error!(?error, "hash de senha inválido no banco");
            AppError::Internal
        })?;
        Ok(Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok())
    })
    .await
    .map_err(|error| {
        tracing::error!(?error, "falha na tarefa de verificação");
        AppError::Internal
    })?
}

pub fn new_session_token() -> String {
    let mut bytes = [0_u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

pub fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

#[derive(FromRow)]
pub struct LoginUser {
    pub id: Uuid,
    pub username: String,
    pub name: String,
    pub role: String,
    pub password_hash: String,
}

