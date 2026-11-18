pub mod config;
pub mod error;
pub mod routes;
pub mod state;

use axum::Router;
use state::AppState;
use axum::{
    extract::{DefaultBodyLimit, Request, State},
    http::{HeaderName, HeaderValue, Method, StatusCode, header::ORIGIN},
    middleware::{self, Next},
    response::Response,
};
use tower_http::{request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer}, trace::TraceLayer};

pub fn app(state: AppState) -> Router {
    let middleware_state = state.clone();
    Router::new()
        .nest("/api", routes::router())
        .with_state(state)
        .layer(middleware::from_fn(security_headers))
        .layer(middleware::from_fn_with_state(middleware_state, verify_origin))
        .layer(DefaultBodyLimit::max(64 * 1024))
        .layer(PropagateRequestIdLayer::new(HeaderName::from_static("x-request-id")))
        .layer(TraceLayer::new_for_http())
        .layer(SetRequestIdLayer::new(HeaderName::from_static("x-request-id"), MakeRequestUuid))
}

async fn security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert("x-content-type-options", HeaderValue::from_static("nosniff"));
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    headers.insert("permissions-policy", HeaderValue::from_static("camera=(), microphone=(), geolocation=()"));
    headers.insert("content-security-policy", HeaderValue::from_static("default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; font-src 'self'; script-src 'self'; connect-src 'self'"));
    response
}

async fn verify_origin(State(state): State<AppState>, request: Request, next: Next) -> Result<Response, StatusCode> {
    let mutating = matches!(*request.method(), Method::POST | Method::PUT | Method::PATCH | Method::DELETE);
    if mutating {
        if let Some(expected) = state.config.allowed_origin.as_deref() {
            let actual = request.headers().get(ORIGIN).and_then(|value| value.to_str().ok());
            if actual != Some(expected) { return Err(StatusCode::FORBIDDEN); }
        }
    }
    Ok(next.run(request).await)
}
pub mod auth;
pub mod bootstrap;
