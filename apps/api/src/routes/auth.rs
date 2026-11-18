use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::Redirect,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{Duration, Utc};
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointMaybeSet, EndpointNotSet,
    EndpointSet, IssuerUrl, Nonce, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope,
    TokenResponse,
    core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata},
};
use serde::Deserialize;
use sqlx::FromRow;
use time::Duration as CookieDuration;
use uuid::Uuid;

use crate::{
    auth::{CurrentUser, hash_token, new_session_token},
    error::AppError,
    state::AppState,
};

#[derive(FromRow)]
struct AuthFlow {
    binding_hash: String,
    nonce: String,
    pkce_verifier: String,
}

#[derive(Deserialize)]
struct EntraClaims {
    tid: Uuid,
    oid: Uuid,
    #[serde(default)]
    roles: Vec<String>,
    preferred_username: Option<String>,
    name: Option<String>,
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

fn auth_cookie_name(production: bool) -> &'static str {
    if production {
        "__Host-horasculo_auth"
    } else {
        "horasculo_auth"
    }
}

fn cookie(name: &'static str, value: String, production: bool, seconds: i64) -> Cookie<'static> {
    Cookie::build((name, value))
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(production)
        .path("/")
        .max_age(CookieDuration::seconds(seconds))
        .build()
}

type EntraClient = CoreClient<
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;

async fn oidc_client(state: &AppState) -> Result<EntraClient, AppError> {
    let issuer = IssuerUrl::new(state.config.issuer()).map_err(|_| AppError::Internal)?;
    let metadata = CoreProviderMetadata::discover_async(issuer, &state.http)
        .await
        .map_err(|error| {
            tracing::error!(?error, "falha na descoberta OIDC");
            AppError::Internal
        })?;
    let redirect = RedirectUrl::new(state.config.entra_redirect_uri.clone())
        .map_err(|_| AppError::Internal)?;
    Ok(CoreClient::from_provider_metadata(
        metadata,
        ClientId::new(state.config.entra_client_id.clone()),
        Some(ClientSecret::new(state.config.entra_client_secret.clone())),
    )
    .set_redirect_uri(redirect))
}

pub async fn start(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, Redirect), AppError> {
    let _permit = state
        .auth_flows
        .clone()
        .try_acquire_owned()
        .map_err(|_| AppError::RateLimited)?;
    let client = oidc_client(&state).await?;
    let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
    let (url, csrf, nonce) = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("profile".into()))
        .set_pkce_challenge(challenge)
        .url();
    let binding = new_session_token();
    sqlx::query("DELETE FROM auth_flows WHERE expires_at <= now()")
        .execute(&state.db)
        .await?;
    let pending: i64 = sqlx::query_scalar("SELECT count(*) FROM auth_flows")
        .fetch_one(&state.db)
        .await?;
    if pending >= 1000 {
        return Err(AppError::RateLimited);
    }
    sqlx::query("INSERT INTO auth_flows(state_hash,binding_hash,nonce,pkce_verifier,expires_at) VALUES($1,$2,$3,$4,$5)")
        .bind(hash_token(csrf.secret()))
        .bind(hash_token(&binding))
        .bind(nonce.secret())
        .bind(verifier.secret())
        .bind(Utc::now() + Duration::minutes(10))
        .execute(&state.db).await?;
    Ok((
        jar.add(cookie(
            auth_cookie_name(state.config.production),
            binding,
            state.config.production,
            600,
        )),
        Redirect::temporary(url.as_str()),
    ))
}

pub async fn callback(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<CallbackQuery>,
) -> Result<(CookieJar, Redirect), AppError> {
    let result = callback_inner(state, jar, query).await;
    if let Err(error) = &result {
        tracing::warn!(target: "audit", action="login", result="failure", reason=%error, "security event");
    }
    result
}

async fn callback_inner(
    state: AppState,
    jar: CookieJar,
    query: CallbackQuery,
) -> Result<(CookieJar, Redirect), AppError> {
    let _permit = state
        .auth_flows
        .clone()
        .try_acquire_owned()
        .map_err(|_| AppError::RateLimited)?;
    if query.error.is_some() {
        return Err(AppError::Unauthorized);
    }
    let code = query.code.ok_or(AppError::Unauthorized)?;
    let callback_state = query.state.ok_or(AppError::Unauthorized)?;
    let binding = jar
        .get(auth_cookie_name(state.config.production))
        .ok_or(AppError::Unauthorized)?
        .value();
    let flow = sqlx::query_as::<_, AuthFlow>(
        "DELETE FROM auth_flows WHERE state_hash=$1 AND expires_at>now()
         RETURNING binding_hash,nonce,pkce_verifier",
    )
    .bind(hash_token(&callback_state))
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::Unauthorized)?;
    if hash_token(binding) != flow.binding_hash {
        return Err(AppError::Unauthorized);
    }

    let client = oidc_client(&state).await?;
    let token = client
        .exchange_code(AuthorizationCode::new(code))
        .map_err(|_| AppError::Internal)?
        .set_pkce_verifier(PkceCodeVerifier::new(flow.pkce_verifier))
        .request_async(&state.http)
        .await
        .map_err(|_| {
            tracing::warn!("falha na troca do código OIDC");
            AppError::Unauthorized
        })?;
    let id_token = token.id_token().ok_or(AppError::Unauthorized)?;
    id_token
        .claims(&client.id_token_verifier(), &Nonce::new(flow.nonce))
        .map_err(|_| {
            tracing::warn!("ID token inválido");
            AppError::Unauthorized
        })?;
    let encoded = id_token.to_string();
    let payload = encoded.split('.').nth(1).ok_or(AppError::Unauthorized)?;
    let decoded = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| AppError::Unauthorized)?;
    let claims: EntraClaims =
        serde_json::from_slice(&decoded).map_err(|_| AppError::Unauthorized)?;
    if claims.tid != state.config.entra_tenant_id
        || !claims.roles.iter().any(|role| role == "Horasculo.User")
    {
        tracing::warn!(tenant = %claims.tid, subject = %claims.oid, "conta sem atribuição de usuário");
        return Err(AppError::Forbidden);
    }
    let role = if claims.roles.iter().any(|role| role == "Horasculo.Admin") {
        "admin"
    } else {
        "user"
    };
    let username = claims
        .preferred_username
        .unwrap_or_else(|| claims.oid.to_string());
    let name = claims.name.unwrap_or_else(|| username.clone());
    let user = sqlx::query_as::<_, CurrentUser>(
        "INSERT INTO users(id,entra_tid,entra_oid,username,name,role)
         VALUES($1,$2,$3,$4,$5,$6::user_role)
         ON CONFLICT(entra_tid,entra_oid) DO UPDATE
         SET username=EXCLUDED.username,name=EXCLUDED.name,role=EXCLUDED.role,updated_at=now()
         WHERE users.active=true
         RETURNING id,username::text AS username,name,role::text AS role",
    )
    .bind(Uuid::new_v4())
    .bind(claims.tid)
    .bind(claims.oid)
    .bind(username.chars().take(200).collect::<String>())
    .bind(name.chars().take(200).collect::<String>())
    .bind(role)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::Forbidden)?;
    sqlx::query(
        "INSERT INTO weekly_goals(user_id,hours) VALUES($1,40) ON CONFLICT(user_id) DO NOTHING",
    )
    .bind(user.id)
    .execute(&state.db)
    .await?;
    let session = new_session_token();
    sqlx::query("INSERT INTO sessions(id,user_id,token_hash,expires_at) VALUES($1,$2,$3,$4)")
        .bind(Uuid::new_v4())
        .bind(user.id)
        .bind(hash_token(&session))
        .bind(Utc::now() + Duration::hours(8))
        .execute(&state.db)
        .await?;
    tracing::info!(target: "audit", actor_id=%user.id, action="login", result="success", "security event");
    let jar = jar.remove(cookie(
        auth_cookie_name(state.config.production),
        String::new(),
        state.config.production,
        0,
    ));
    Ok((
        jar.add(cookie(
            state.config.cookie_name(),
            session,
            state.config.production,
            28_800,
        )),
        Redirect::to(&state.config.allowed_origin),
    ))
}

pub async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, StatusCode), AppError> {
    if let Some(current) = jar.get(state.config.cookie_name()) {
        let result = sqlx::query("DELETE FROM sessions WHERE token_hash=$1 RETURNING user_id")
            .bind(hash_token(current.value()))
            .fetch_optional(&state.db)
            .await?;
        if let Some(row) = result {
            use sqlx::Row;
            let user_id: Uuid = row.try_get("user_id")?;
            tracing::info!(target: "audit", actor_id=%user_id, action="logout", result="success", "security event");
        }
    }
    Ok((
        jar.remove(cookie(
            state.config.cookie_name(),
            String::new(),
            state.config.production,
            0,
        )),
        StatusCode::NO_CONTENT,
    ))
}

pub async fn me(user: CurrentUser) -> Json<CurrentUser> {
    Json(user)
}

pub async fn revoke_sessions(
    State(state): State<AppState>,
    admin: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    admin.require_admin()?;
    sqlx::query("DELETE FROM sessions WHERE user_id=$1")
        .bind(id)
        .execute(&state.db)
        .await?;
    tracing::info!(target: "audit", actor_id=%admin.id, target_id=%id, action="revoke_sessions", result="success", "security event");
    Ok(StatusCode::NO_CONTENT)
}
