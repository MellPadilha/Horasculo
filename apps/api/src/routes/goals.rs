use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};

use crate::{auth::CurrentUser, error::AppError, state::AppState};

#[derive(Serialize, Deserialize)]
pub struct WeeklyGoal { hours: f64 }

pub async fn get(State(state): State<AppState>, user: CurrentUser) -> Result<Json<WeeklyGoal>, AppError> {
    let hours = sqlx::query_scalar::<_, f64>("SELECT hours FROM weekly_goals WHERE user_id=$1").bind(user.id).fetch_optional(&state.db).await?.unwrap_or(40.0);
    Ok(Json(WeeklyGoal { hours }))
}
pub async fn put(State(state): State<AppState>, user: CurrentUser, Json(input): Json<WeeklyGoal>) -> Result<Json<WeeklyGoal>, AppError> {
    let quarters = input.hours * 4.0;
    if !input.hours.is_finite() || !(0.25..=168.0).contains(&input.hours) || (quarters - quarters.round()).abs() > 1e-9 {
        return Err(AppError::Validation("a meta deve estar entre 0,25 e 168 horas em intervalos de 0,25".into()));
    }
    sqlx::query("INSERT INTO weekly_goals (user_id,hours) VALUES ($1,$2) ON CONFLICT (user_id) DO UPDATE SET hours=$2,updated_at=now()")
        .bind(user.id).bind(input.hours).execute(&state.db).await?;
    Ok(Json(input))
}
