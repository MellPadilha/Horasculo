pub mod config;
pub mod error;
pub mod routes;
pub mod state;

use axum::Router;
use state::AppState;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

pub fn app(state: AppState) -> Router {
    Router::new()
        .nest("/api", routes::router())
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}
