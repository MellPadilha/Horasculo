use axum::{Json, extract::State};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{auth::CurrentUser, error::AppError, state::AppState};

#[derive(serde::Serialize, FromRow)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub name: String,
    pub role: String,
}

pub async fn list(
    State(state): State<AppState>,
    admin: CurrentUser,
) -> Result<Json<Vec<UserResponse>>, AppError> {
    admin.require_admin()?;
    let users = sqlx::query_as::<_, UserResponse>(
        "SELECT id,username::text AS username,name,role::text AS role
         FROM users WHERE active=true ORDER BY name LIMIT 1000",
    )
    .fetch_all(&state.db)
    .await?;
    tracing::info!(target: "audit", actor_id=%admin.id, action="list_users", result="success", rows=users.len(), "security event");
    Ok(Json(users))
}
