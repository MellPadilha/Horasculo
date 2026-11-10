use axum::{Json, extract::{Path, State}};
use serde::Deserialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::{auth::{CurrentUser, hash_password}, error::AppError, state::AppState};

#[derive(serde::Serialize, FromRow)]
pub struct UserResponse { pub id: Uuid, pub username: String, pub name: String, pub role: String }
#[derive(Deserialize)]
pub struct CreateUser { username: String, name: String, password: String, role: Option<String> }
#[derive(Deserialize)]
pub struct UpdateUser { username: Option<String>, name: Option<String>, password: Option<String>, role: Option<String> }

fn validate_role(role: &str) -> Result<(), AppError> {
    if matches!(role, "admin" | "user") { Ok(()) } else { Err(AppError::Validation("papel inválido".into())) }
}
fn required(value: &str, field: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() { Err(AppError::Validation(format!("{field} é obrigatório"))) } else { Ok(value.to_owned()) }
}

pub async fn list(State(state): State<AppState>, admin: CurrentUser) -> Result<Json<Vec<UserResponse>>, AppError> {
    admin.require_admin()?;
    Ok(Json(sqlx::query_as::<_, UserResponse>("SELECT id, username::text AS username, name, role::text AS role FROM users WHERE active = true ORDER BY name").fetch_all(&state.db).await?))
}

pub async fn create(State(state): State<AppState>, admin: CurrentUser, Json(input): Json<CreateUser>) -> Result<Json<UserResponse>, AppError> {
    admin.require_admin()?;
    let username = required(&input.username, "username")?.to_lowercase();
    let name = required(&input.name, "nome")?;
    if input.password.len() < 8 { return Err(AppError::Validation("a senha deve ter pelo menos 8 caracteres".into())); }
    let role = input.role.unwrap_or_else(|| "user".into()); validate_role(&role)?;
    if sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)").bind(&username).fetch_one(&state.db).await? {
        return Err(AppError::Conflict("este nome de usuário já está em uso".into()));
    }
    let id = Uuid::new_v4(); let hash = hash_password(input.password).await?; let mut tx = state.db.begin().await?;
    let user = sqlx::query_as::<_, UserResponse>("INSERT INTO users (id, username, name, password_hash, role) VALUES ($1,$2,$3,$4,$5::user_role) RETURNING id, username::text AS username, name, role::text AS role")
        .bind(id).bind(username).bind(name).bind(hash).bind(role).fetch_one(&mut *tx).await?;
    sqlx::query("INSERT INTO weekly_goals (user_id, hours) VALUES ($1,40)").bind(id).execute(&mut *tx).await?;
    tx.commit().await?; Ok(Json(user))
}

pub async fn update(State(state): State<AppState>, admin: CurrentUser, Path(id): Path<Uuid>, Json(input): Json<UpdateUser>) -> Result<Json<UserResponse>, AppError> {
    admin.require_admin()?;
    if id == admin.id && input.role.as_deref().is_some_and(|role| role != "admin") { return Err(AppError::Validation("não é possível remover seu próprio papel administrativo".into())); }
    let username = input.username.as_deref().map(|v| required(v,"username")).transpose()?.map(|v| v.to_lowercase());
    let name = input.name.as_deref().map(|v| required(v,"nome")).transpose()?;
    if let Some(role) = input.role.as_deref() { validate_role(role)?; }
    if input.password.as_deref().is_some_and(|p| p.len() < 8) { return Err(AppError::Validation("a senha deve ter pelo menos 8 caracteres".into())); }
    let password_changed = input.password.is_some();
    if let Some(candidate) = username.as_deref() {
        if sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM users WHERE username=$1 AND id<>$2)").bind(candidate).bind(id).fetch_one(&state.db).await? {
            return Err(AppError::Conflict("este nome de usuário já está em uso".into()));
        }
    }
    let password_hash = match input.password { Some(password) => Some(hash_password(password).await?), None => None };
    let user = sqlx::query_as::<_, UserResponse>("UPDATE users SET username=COALESCE($2,username), name=COALESCE($3,name), role=COALESCE($4::user_role,role), password_hash=COALESCE($5,password_hash), updated_at=now() WHERE id=$1 AND active=true RETURNING id, username::text AS username, name, role::text AS role")
        .bind(id).bind(username).bind(name).bind(input.role).bind(password_hash).fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;
    if password_changed { sqlx::query("DELETE FROM sessions WHERE user_id=$1").bind(id).execute(&state.db).await?; }
    Ok(Json(user))
}

pub async fn archive(State(state): State<AppState>, admin: CurrentUser, Path(id): Path<Uuid>) -> Result<(), AppError> {
    admin.require_admin()?; if id == admin.id { return Err(AppError::Validation("não é possível arquivar o usuário autenticado".into())); }
    let mut tx = state.db.begin().await?;
    if sqlx::query("UPDATE users SET active=false, updated_at=now() WHERE id=$1 AND active=true").bind(id).execute(&mut *tx).await?.rows_affected() == 0 { return Err(AppError::NotFound); }
    sqlx::query("DELETE FROM sessions WHERE user_id=$1").bind(id).execute(&mut *tx).await?; tx.commit().await?; Ok(())
}
