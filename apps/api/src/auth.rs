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

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct CurrentUser {
    pub id: Uuid,
    pub username: String,
    pub name: String,
    pub role: String,
}

impl CurrentUser {
    pub fn require_admin(&self) -> Result<(), AppError> {
        if self.role == "admin" {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        }
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
        let token = jar
            .get(state.config.cookie_name())
            .ok_or(AppError::Unauthorized)?
            .value();
        let token_hash = hash_token(token);
        sqlx::query_as::<_, CurrentUser>(
            "WITH valid_session AS (
                UPDATE sessions SET last_seen_at = now()
                WHERE token_hash = $1 AND expires_at > now()
                  AND last_seen_at > now() - interval '30 minutes'
                RETURNING user_id
             )
             SELECT u.id, u.username::text AS username, u.name, u.role::text AS role
             FROM valid_session s JOIN users u ON u.id = s.user_id
             WHERE u.active = true",
        )
        .bind(token_hash)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::Unauthorized)
    }
}

pub fn new_session_token() -> String {
    let mut bytes = [0_u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

pub fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}
