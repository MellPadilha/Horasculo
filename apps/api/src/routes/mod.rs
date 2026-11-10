mod auth;
mod goals;
mod health;
mod projects;
mod time_entries;
mod users;

use axum::{Router, routing::get};

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health::check))
        .route("/ready", get(health::ready))
        .route("/auth/login", axum::routing::post(auth::login))
        .route("/auth/logout", axum::routing::post(auth::logout))
        .route("/auth/me", get(auth::me))
        .route("/users", get(users::list).post(users::create))
        .route("/users/{id}", axum::routing::patch(users::update).delete(users::archive))
        .route("/projects", get(projects::list).post(projects::create))
        .route("/projects/{id}", axum::routing::patch(projects::update).delete(projects::archive))
        .route("/me/weekly-goal", get(goals::get).put(goals::put))
        .route("/time-entries", get(time_entries::list).post(time_entries::create))
        .route("/time-entries/{id}", get(time_entries::get).patch(time_entries::update).delete(time_entries::delete))
}
