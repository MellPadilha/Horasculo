use sqlx::PgPool;
use uuid::Uuid;

use crate::{auth::hash_password, config::Config, error::AppError};

pub async fn run(db: &PgPool, config: &Config) -> Result<(), AppError> {
    let user_count: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
        .fetch_one(db)
        .await?;

    if user_count == 0 {
        let username = config.bootstrap_admin_username.as_deref().ok_or_else(|| {
            AppError::Configuration("BOOTSTRAP_ADMIN_USERNAME é obrigatório no primeiro boot".into())
        })?;
        let password = config.bootstrap_admin_password.as_deref().ok_or_else(|| {
            AppError::Configuration("BOOTSTRAP_ADMIN_PASSWORD é obrigatório no primeiro boot".into())
        })?;
        if username.trim().is_empty() || password.len() < 12 {
            return Err(AppError::Configuration(
                "administrador inicial exige usuário e senha com pelo menos 12 caracteres".into(),
            ));
        }
        let password_hash = hash_password(password.to_owned()).await?;
        let user_id = Uuid::new_v4();
        let mut transaction = db.begin().await?;
        sqlx::query(
            "INSERT INTO users (id, username, name, password_hash, role) VALUES ($1, $2, $3, $4, 'admin')",
        )
        .bind(user_id)
        .bind(username.trim().to_lowercase())
        .bind(config.bootstrap_admin_name.trim())
        .bind(password_hash)
        .execute(&mut *transaction)
        .await?;
        sqlx::query("INSERT INTO weekly_goals (user_id, hours) VALUES ($1, 40)")
            .bind(user_id)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        tracing::info!("administrador inicial criado");
    }

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
