use axum::{Json, extract::State};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::{Duration, Utc};
use serde::Deserialize;
use time::Duration as CookieDuration;
use uuid::Uuid;

use crate::{
    auth::{CurrentUser, LoginUser, SESSION_COOKIE, hash_token, new_session_token, verify_password},
    error::AppError,
    state::AppState,
};

#[derive(Deserialize)]
pub struct LoginInput { username: String, password: String }

pub async fn login(State(state): State<AppState>, jar: CookieJar, Json(input): Json<LoginInput>) -> Result<(CookieJar, Json<CurrentUser>), AppError> {
    let candidate = sqlx::query_as::<_, LoginUser>(
        "SELECT id, username::text AS username, name, role::text AS role, password_hash FROM users WHERE username = $1 AND active = true",
    ).bind(input.username.trim().to_lowercase()).fetch_optional(&state.db).await?;
    let Some(candidate) = candidate else { return Err(AppError::Unauthorized) };
    if !verify_password(input.password, candidate.password_hash).await? { return Err(AppError::Unauthorized); }

    let token = new_session_token();
    let expires_at = Utc::now() + Duration::days(state.config.session_days);
    sqlx::query("INSERT INTO sessions (id, user_id, token_hash, expires_at) VALUES ($1, $2, $3, $4)")
        .bind(Uuid::new_v4()).bind(candidate.id).bind(hash_token(&token)).bind(expires_at).execute(&state.db).await?;
    let cookie = Cookie::build((SESSION_COOKIE, token)).http_only(true).same_site(SameSite::Lax)
        .secure(state.config.production).path("/").max_age(CookieDuration::days(state.config.session_days)).build();
    let user = CurrentUser { id: candidate.id, username: candidate.username, name: candidate.name, role: candidate.role };
    Ok((jar.add(cookie), Json(user)))
}

pub async fn logout(State(state): State<AppState>, jar: CookieJar) -> Result<CookieJar, AppError> {
    if let Some(cookie) = jar.get(SESSION_COOKIE) {
        sqlx::query("DELETE FROM sessions WHERE token_hash = $1").bind(hash_token(cookie.value())).execute(&state.db).await?;
    }
    Ok(jar.remove(Cookie::build((SESSION_COOKIE, "")).path("/").build()))
}

pub async fn me(user: CurrentUser) -> Json<CurrentUser> { Json(user) }
