use axum::{Json, extract::State, http::HeaderMap};
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

async fn login_allowed(state: &AppState, identifier: &str) -> Result<bool, AppError> {
    Ok(sqlx::query_scalar::<_, bool>(
        "SELECT NOT EXISTS(SELECT 1 FROM login_attempts WHERE identifier=$1 AND attempts >= 5 AND window_started > now() - interval '15 minutes')",
    ).bind(identifier).fetch_one(&state.db).await?)
}

async fn failed_login(state: &AppState, identifier: &str) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO login_attempts(identifier,attempts,window_started) VALUES($1,1,now())
         ON CONFLICT(identifier) DO UPDATE SET
           attempts=CASE WHEN login_attempts.window_started <= now()-interval '15 minutes' THEN 1 ELSE login_attempts.attempts+1 END,
           window_started=CASE WHEN login_attempts.window_started <= now()-interval '15 minutes' THEN now() ELSE login_attempts.window_started END",
    ).bind(identifier).execute(&state.db).await?;
    Ok(())
}

pub async fn login(State(state): State<AppState>, headers: HeaderMap, jar: CookieJar, Json(input): Json<LoginInput>) -> Result<(CookieJar, Json<CurrentUser>), AppError> {
    sqlx::query("DELETE FROM sessions WHERE expires_at <= now()").execute(&state.db).await?;
    let username = input.username.trim().to_lowercase();
    let client = headers.get("x-forwarded-for").and_then(|value| value.to_str().ok())
        .and_then(|value| value.rsplit(',').next()).map(str::trim).unwrap_or("direct");
    let identifier = format!("{username}|{client}");
    if !login_allowed(&state, &identifier).await? { return Err(AppError::RateLimited); }
    let candidate = sqlx::query_as::<_, LoginUser>(
        "SELECT id, username::text AS username, name, role::text AS role, password_hash FROM users WHERE username = $1 AND active = true",
    ).bind(&username).fetch_optional(&state.db).await?;
    let Some(candidate) = candidate else { failed_login(&state,&identifier).await?; return Err(AppError::Unauthorized) };
    if !verify_password(input.password, candidate.password_hash).await? { failed_login(&state,&identifier).await?; return Err(AppError::Unauthorized); }
    sqlx::query("DELETE FROM login_attempts WHERE identifier=$1").bind(&identifier).execute(&state.db).await?;

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
