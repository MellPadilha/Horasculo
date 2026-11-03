use axum::{Json, extract::State};
use serde::Serialize;

use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    status: &'static str,
    uptime_seconds: u64,
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
        let Json(response) = check(State(AppState::new())).await;

        assert_eq!(response.status, "ok");
    }
}
