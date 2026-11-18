use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{DateTime, FixedOffset, Timelike, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{auth::CurrentUser, error::AppError, state::AppState};

#[derive(FromRow)]
struct EntryRow {
    id: Uuid,
    user_id: Uuid,
    project_id: Uuid,
    title: String,
    starts_at: DateTime<Utc>,
    ends_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
#[derive(Serialize)]
pub struct EntryResponse {
    id: Uuid,
    user_id: Uuid,
    project_id: Uuid,
    title: String,
    starts_at: DateTime<Utc>,
    ends_at: DateTime<Utc>,
    hours: f64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
impl From<EntryRow> for EntryResponse {
    fn from(row: EntryRow) -> Self {
        let hours = (row.ends_at - row.starts_at).num_seconds() as f64 / 3600.0;
        Self {
            id: row.id,
            user_id: row.user_id,
            project_id: row.project_id,
            title: row.title,
            starts_at: row.starts_at,
            ends_at: row.ends_at,
            hours,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}
#[derive(Deserialize)]
pub struct EntryInput {
    title: String,
    project_id: Uuid,
    user_id: Option<Uuid>,
    starts_at: DateTime<FixedOffset>,
    ends_at: DateTime<FixedOffset>,
}
#[derive(Deserialize)]
pub struct EntryUpdate {
    title: Option<String>,
    project_id: Option<Uuid>,
    starts_at: Option<DateTime<FixedOffset>>,
    ends_at: Option<DateTime<FixedOffset>>,
}
#[derive(Deserialize)]
pub struct ListQuery {
    from: DateTime<FixedOffset>,
    to: DateTime<FixedOffset>,
    project_id: Option<Uuid>,
    user_id: Option<Uuid>,
    limit: Option<i64>,
    offset: Option<i64>,
}

const MAX_QUERY_DAYS: i64 = 366;

fn title(value: &str) -> Result<String, AppError> {
    let v = value.trim();
    if v.is_empty() {
        Err(AppError::Validation("título é obrigatório".into()))
    } else if v.chars().count() > 200 {
        Err(AppError::Validation(
            "título deve ter no máximo 200 caracteres".into(),
        ))
    } else {
        Ok(v.to_owned())
    }
}
fn validate_range(start: DateTime<Utc>, end: DateTime<Utc>) -> Result<(), AppError> {
    if start.second() != 0
        || start.nanosecond() != 0
        || end.second() != 0
        || end.nanosecond() != 0
        || !start.minute().is_multiple_of(15)
        || !end.minute().is_multiple_of(15)
    {
        return Err(AppError::Validation(
            "horários devem usar intervalos de 15 minutos".into(),
        ));
    }
    let duration = end - start;
    if duration.num_minutes() <= 0 || duration.num_hours() > 24 || duration.num_minutes() > 1440 {
        return Err(AppError::Validation(
            "o período deve ser maior que zero e ter no máximo 24 horas".into(),
        ));
    }
    Ok(())
}
fn validate_query_range(start: DateTime<Utc>, end: DateTime<Utc>) -> Result<(), AppError> {
    if end <= start || end - start > chrono::Duration::days(MAX_QUERY_DAYS) {
        return Err(AppError::Validation(
            "o período de consulta deve ter no máximo 366 dias".into(),
        ));
    }
    Ok(())
}
async fn active_project(state: &AppState, id: Uuid) -> Result<(), AppError> {
    if !sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM projects WHERE id=$1 AND active=true)",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?
    {
        Err(AppError::Validation("projeto inválido ou arquivado".into()))
    } else {
        Ok(())
    }
}
const SELECT_ENTRY: &str =
    "SELECT id,user_id,project_id,title,starts_at,ends_at,created_at,updated_at FROM time_entries";

pub async fn list(
    State(state): State<AppState>,
    user: CurrentUser,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<EntryResponse>>, AppError> {
    let from = query.from.with_timezone(&Utc);
    let to = query.to.with_timezone(&Utc);
    validate_query_range(from, to)?;
    let limit = query.limit.unwrap_or(200);
    let offset = query.offset.unwrap_or(0);
    if !(1..=500).contains(&limit) || !(0..=10_000).contains(&offset) {
        return Err(AppError::Validation("paginação inválida".into()));
    }
    let requested_user = if user.role == "admin" {
        query.user_id
    } else {
        Some(user.id)
    };
    let rows=sqlx::query_as::<_,EntryRow>(&format!("{SELECT_ENTRY} WHERE starts_at < $2 AND ends_at > $1 AND ($3::uuid IS NULL OR project_id=$3) AND ($4::uuid IS NULL OR user_id=$4) ORDER BY starts_at,id LIMIT $5 OFFSET $6"))
        .bind(from).bind(to).bind(query.project_id).bind(requested_user).bind(limit).bind(offset).fetch_all(&state.db).await?;
    if user.role == "admin" {
        tracing::info!(target: "audit", actor_id=%user.id, action="admin_list_entries", result="success", rows=rows.len(), "security event");
    }
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}
pub async fn get(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<EntryResponse>, AppError> {
    let row = sqlx::query_as::<_, EntryRow>(&format!("{SELECT_ENTRY} WHERE id=$1"))
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;
    if user.role != "admin" && row.user_id != user.id {
        return Err(AppError::NotFound);
    }
    if user.role == "admin" && row.user_id != user.id {
        tracing::info!(target: "audit", actor_id=%user.id, target_id=%row.id, action="admin_get_entry", result="success", "security event");
    }
    Ok(Json(row.into()))
}
pub async fn create(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(input): Json<EntryInput>,
) -> Result<Json<EntryResponse>, AppError> {
    let starts = input.starts_at.with_timezone(&Utc);
    let ends = input.ends_at.with_timezone(&Utc);
    validate_range(starts, ends)?;
    active_project(&state, input.project_id).await?;
    let owner = if user.role == "admin" {
        input.user_id.unwrap_or(user.id)
    } else {
        user.id
    };
    if !sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM users WHERE id=$1 AND active=true)",
    )
    .bind(owner)
    .fetch_one(&state.db)
    .await?
    {
        return Err(AppError::Validation("usuário inválido ou arquivado".into()));
    }
    let row=sqlx::query_as::<_,EntryRow>("INSERT INTO time_entries (id,user_id,project_id,title,starts_at,ends_at) VALUES ($1,$2,$3,$4,$5,$6) RETURNING id,user_id,project_id,title,starts_at,ends_at,created_at,updated_at")
        .bind(Uuid::new_v4()).bind(owner).bind(input.project_id).bind(title(&input.title)?).bind(starts).bind(ends).fetch_one(&state.db).await?;
    tracing::info!(target: "audit", actor_id=%user.id, target_id=%row.id, owner_id=%owner, action="create_entry", result="success", "security event");
    Ok(Json(row.into()))
}
pub async fn update(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(input): Json<EntryUpdate>,
) -> Result<Json<EntryResponse>, AppError> {
    let current = sqlx::query_as::<_, EntryRow>(&format!("{SELECT_ENTRY} WHERE id=$1"))
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;
    if user.role != "admin" && current.user_id != user.id {
        return Err(AppError::NotFound);
    }
    let starts = input
        .starts_at
        .map(|v| v.with_timezone(&Utc))
        .unwrap_or(current.starts_at);
    let ends = input
        .ends_at
        .map(|v| v.with_timezone(&Utc))
        .unwrap_or(current.ends_at);
    validate_range(starts, ends)?;
    let project = input.project_id.unwrap_or(current.project_id);
    if project != current.project_id {
        active_project(&state, project).await?;
    }
    let entry_title = input
        .title
        .as_deref()
        .map(title)
        .transpose()?
        .unwrap_or(current.title);
    let row=sqlx::query_as::<_,EntryRow>("UPDATE time_entries SET project_id=$2,title=$3,starts_at=$4,ends_at=$5,updated_at=now() WHERE id=$1 RETURNING id,user_id,project_id,title,starts_at,ends_at,created_at,updated_at")
        .bind(id).bind(project).bind(entry_title).bind(starts).bind(ends).fetch_one(&state.db).await?;
    tracing::info!(target: "audit", actor_id=%user.id, target_id=%id, action="update_entry", result="success", "security event");
    Ok(Json(row.into()))
}
pub async fn delete(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    let owner = sqlx::query_scalar::<_, Uuid>("SELECT user_id FROM time_entries WHERE id=$1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;
    if user.role != "admin" && owner != user.id {
        return Err(AppError::NotFound);
    }
    sqlx::query("DELETE FROM time_entries WHERE id=$1")
        .bind(id)
        .execute(&state.db)
        .await?;
    tracing::info!(target: "audit", actor_id=%user.id, target_id=%id, action="delete_entry", result="success", "security event");
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::validate_range;

    #[test]
    fn accepts_entries_crossing_midnight_and_with_twenty_four_hours() {
        let start = Utc.with_ymd_and_hms(2026, 11, 9, 23, 0, 0).unwrap();
        let next_day = Utc.with_ymd_and_hms(2026, 11, 10, 1, 30, 0).unwrap();
        assert!(validate_range(start, next_day).is_ok());
        assert!(validate_range(start, start + chrono::Duration::hours(24)).is_ok());
    }

    #[test]
    fn rejects_long_or_non_quarter_entries() {
        let start = Utc.with_ymd_and_hms(2026, 11, 9, 6, 0, 0).unwrap();
        assert!(validate_range(start, start + chrono::Duration::minutes(1441)).is_err());
        let invalid = Utc.with_ymd_and_hms(2026, 11, 9, 6, 10, 0).unwrap();
        assert!(validate_range(invalid, invalid + chrono::Duration::hours(1)).is_err());
    }
}
