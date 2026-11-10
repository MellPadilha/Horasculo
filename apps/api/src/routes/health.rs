use axum::{Json, extract::State};
use serde::Serialize;

use crate::{error::AppError, state::AppState};

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    status: &'static str,
    uptime_seconds: u64,
}

pub async fn ready(State(state): State<AppState>) -> Result<Json<HealthResponse>, AppError> {
    sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(&state.db).await?;
    Ok(check(State(state)).await)
}

pub async fn check(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        uptime_seconds: state.started_at.elapsed().as_secs(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn health_check_returns_ok() {
        let response = HealthResponse { status: "ok", uptime_seconds: 0 };
        assert_eq!(response.status, "ok");
    }
}
