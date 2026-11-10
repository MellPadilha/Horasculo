use axum::{Json, extract::{Path, State}};
use serde::Deserialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::{auth::CurrentUser, error::AppError, state::AppState};

#[derive(serde::Serialize, FromRow)]
pub struct ProjectResponse { pub id: Uuid, pub name: String, pub color: String, pub tint: String }
#[derive(Deserialize)]
pub struct ProjectInput { name: String, color: String, tint: String }
#[derive(Deserialize)]
pub struct ProjectUpdate { name: Option<String>, color: Option<String>, tint: Option<String> }

fn color(value: &str) -> Result<String, AppError> {
    if value.len() == 7 && value.starts_with('#') && value[1..].chars().all(|c| c.is_ascii_hexdigit()) { Ok(value.to_lowercase()) }
    else { Err(AppError::Validation("cor deve usar o formato #RRGGBB".into())) }
}
fn name(value: &str) -> Result<String, AppError> {
    let value = value.trim(); if value.is_empty() { Err(AppError::Validation("nome é obrigatório".into())) } else { Ok(value.to_owned()) }
}

pub async fn list(State(state): State<AppState>, _user: CurrentUser) -> Result<Json<Vec<ProjectResponse>>, AppError> {
    Ok(Json(sqlx::query_as::<_, ProjectResponse>("SELECT id,name,color::text AS color,tint::text AS tint FROM projects WHERE active=true ORDER BY name").fetch_all(&state.db).await?))
}
pub async fn create(State(state): State<AppState>, admin: CurrentUser, Json(input): Json<ProjectInput>) -> Result<Json<ProjectResponse>, AppError> {
    admin.require_admin()?;
    let project = sqlx::query_as::<_, ProjectResponse>("INSERT INTO projects (id,name,color,tint) VALUES ($1,$2,$3,$4) RETURNING id,name,color::text AS color,tint::text AS tint")
        .bind(Uuid::new_v4()).bind(name(&input.name)?).bind(color(&input.color)?).bind(color(&input.tint)?).fetch_one(&state.db).await?;
    Ok(Json(project))
}
pub async fn update(State(state): State<AppState>, admin: CurrentUser, Path(id): Path<Uuid>, Json(input): Json<ProjectUpdate>) -> Result<Json<ProjectResponse>, AppError> {
    admin.require_admin()?;
    let project = sqlx::query_as::<_, ProjectResponse>("UPDATE projects SET name=COALESCE($2,name),color=COALESCE($3,color),tint=COALESCE($4,tint),updated_at=now() WHERE id=$1 AND active=true RETURNING id,name,color::text AS color,tint::text AS tint")
        .bind(id).bind(input.name.as_deref().map(name).transpose()?).bind(input.color.as_deref().map(color).transpose()?).bind(input.tint.as_deref().map(color).transpose()?)
        .fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;
    Ok(Json(project))
}
pub async fn archive(State(state): State<AppState>, admin: CurrentUser, Path(id): Path<Uuid>) -> Result<(), AppError> {
    admin.require_admin()?;
    if sqlx::query("UPDATE projects SET active=false,updated_at=now() WHERE id=$1 AND active=true").bind(id).execute(&state.db).await?.rows_affected() == 0 { return Err(AppError::NotFound); }
    Ok(())
}
