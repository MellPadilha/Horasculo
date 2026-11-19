pub mod config;
pub mod error;
pub mod routes;
pub mod state;

use axum::Router;
use axum::{
    extract::{DefaultBodyLimit, Request, State},
    http::{HeaderName, HeaderValue, Method, StatusCode, header::ORIGIN},
    middleware::{self, Next},
    response::Response,
};
use state::AppState;
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};

pub fn app(state: AppState) -> Router {
    let middleware_state = state.clone();
    Router::new()
        .nest("/api", routes::router())
        .with_state(state)
        .layer(middleware::from_fn(security_headers))
        .layer(middleware::from_fn_with_state(
            middleware_state,
            verify_origin,
        ))
        .layer(DefaultBodyLimit::max(64 * 1024))
        .layer(PropagateRequestIdLayer::new(HeaderName::from_static(
            "x-request-id",
        )))
        .layer(TraceLayer::new_for_http().make_span_with(|request: &Request| {
            tracing::info_span!("http_request", method = %request.method(), path = request.uri().path())
        }))
        .layer(SetRequestIdLayer::new(
            HeaderName::from_static("x-request-id"),
            MakeRequestUuid,
        ))
}

async fn security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    headers.insert(
        "permissions-policy",
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );
    headers.insert("content-security-policy", HeaderValue::from_static("default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; font-src 'self'; script-src 'self'; connect-src 'self'"));
    headers.insert("cache-control", HeaderValue::from_static("no-store"));
    response
}

async fn verify_origin(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let mutating = matches!(
        *request.method(),
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    );
    if mutating {
        let actual = request
            .headers()
            .get(ORIGIN)
            .and_then(|value| value.to_str().ok());
        if actual != Some(state.config.allowed_origin.as_str()) {
            return Err(StatusCode::FORBIDDEN);
        }
    }
    Ok(next.run(request).await)
}

#[cfg(test)]
mod permission_tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use chrono::{Duration, Utc};
    use sqlx::PgPool;
    use tower::ServiceExt;
    use uuid::Uuid;

    use crate::{app, auth::hash_token, config::Config, state::AppState};

    #[sqlx::test(migrations = "./migrations")]
    async fn enforces_admin_and_entry_ownership(db: PgPool) {
        let regular = Uuid::new_v4();
        let admin = Uuid::new_v4();
        let project = Uuid::new_v4();
        let entry = Uuid::new_v4();
        for (id, role) in [(regular, "user"), (admin, "admin")] {
            sqlx::query("INSERT INTO users(id,entra_tid,entra_oid,username,name,role) VALUES($1,$2,$3,$4,$5,$6::user_role)")
                .bind(id).bind(Uuid::nil()).bind(id).bind(id.to_string()).bind("Test User").bind(role)
                .execute(&db).await.unwrap();
            let token = id.to_string();
            sqlx::query(
                "INSERT INTO sessions(id,user_id,token_hash,expires_at) VALUES($1,$2,$3,$4)",
            )
            .bind(Uuid::new_v4())
            .bind(id)
            .bind(hash_token(&token))
            .bind(Utc::now() + Duration::hours(1))
            .execute(&db)
            .await
            .unwrap();
        }
        sqlx::query(
            "INSERT INTO projects(id,name,color,tint) VALUES($1,'Test','#000000','#ffffff')",
        )
        .bind(project)
        .execute(&db)
        .await
        .unwrap();
        sqlx::query("INSERT INTO time_entries(id,user_id,project_id,title,starts_at,ends_at) VALUES($1,$2,$3,'Private','2026-11-01T10:00:00Z','2026-11-01T11:00:00Z')")
            .bind(entry).bind(admin).bind(project).execute(&db).await.unwrap();

        let state = AppState::new(
            db.clone(),
            Config::for_test(),
            openidconnect::reqwest::Client::new(),
        );
        let router = app(state);
        let request = |uri: String, user: Uuid| {
            Request::builder()
                .uri(uri)
                .header("cookie", format!("horasculo_session={user}"))
                .body(Body::empty())
                .unwrap()
        };

        let response = router
            .clone()
            .oneshot(request("/api/users".into(), regular))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let response = router
            .clone()
            .oneshot(request("/api/users".into(), admin))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let list_uri = format!(
            "/api/time-entries?from=2026-11-01T00%3A00%3A00Z&to=2026-11-02T00%3A00%3A00Z&user_id={admin}"
        );
        let response = router
            .clone()
            .oneshot(request(list_uri.clone(), regular))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        assert_eq!(bytes.as_ref(), b"[]");
        let response = router
            .clone()
            .oneshot(request(list_uri, admin))
            .await
            .unwrap();
        let bytes = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        assert!(String::from_utf8_lossy(&bytes).contains(&entry.to_string()));

        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/users/{admin}/sessions"))
                    .header("origin", "http://localhost:5173")
                    .header("cookie", format!("horasculo_session={regular}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/users/{regular}/sessions"))
                    .header("origin", "http://localhost:5173")
                    .header("cookie", format!("horasculo_session={admin}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let response = router
            .clone()
            .oneshot(request("/api/auth/me".into(), regular))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        sqlx::query(
            "UPDATE sessions SET last_seen_at=now()-interval '31 minutes' WHERE user_id=$1",
        )
        .bind(admin)
        .execute(&db)
        .await
        .unwrap();
        let response = router
            .clone()
            .oneshot(request("/api/auth/me".into(), admin))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        sqlx::query("UPDATE sessions SET last_seen_at=now(), expires_at=now()-interval '1 second' WHERE user_id=$1")
            .bind(admin).execute(&db).await.unwrap();
        let response = router
            .oneshot(request("/api/auth/me".into(), admin))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
pub mod auth;
pub mod bootstrap;
