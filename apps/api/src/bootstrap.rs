use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

pub async fn run(db: &PgPool) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO projects (id, name, color, tint)
         SELECT $1, 'JNS', '#4f70b5', '#c9d8f5'
         WHERE NOT EXISTS (SELECT 1 FROM projects)",
    )
    .bind(Uuid::new_v4())
    .execute(db)
    .await?;
    Ok(())
}
