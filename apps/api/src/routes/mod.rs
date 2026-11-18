mod auth;
mod goals;
mod health;
mod projects;
mod reports;
mod time_entries;
mod users;

use axum::{Router, routing::get};

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health::check))
        .route("/ready", get(health::ready))
        .route("/auth/start", get(auth::start))
        .route("/auth/callback", get(auth::callback))
        .route("/auth/logout", axum::routing::post(auth::logout))
        .route("/auth/me", get(auth::me))
        .route("/users", get(users::list))
        .route(
            "/users/{id}/sessions",
            axum::routing::delete(auth::revoke_sessions),
        )
        .route("/projects", get(projects::list).post(projects::create))
        .route(
            "/projects/{id}",
            axum::routing::patch(projects::update).delete(projects::archive),
        )
        .route("/me/weekly-goal", get(goals::get).put(goals::put))
        .route(
            "/time-entries",
            get(time_entries::list).post(time_entries::create),
        )
        .route(
            "/time-entries/{id}",
            get(time_entries::get)
                .patch(time_entries::update)
                .delete(time_entries::delete),
        )
        .route("/reports/summary", get(reports::summary))
        .route("/reports/time-entries.xlsx", get(reports::xlsx))
}
