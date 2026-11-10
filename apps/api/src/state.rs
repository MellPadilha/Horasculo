use std::{sync::Arc, time::Instant};

use sqlx::PgPool;

use crate::config::{Config, SharedConfig};

#[derive(Clone)]
pub struct AppState {
    pub started_at: Arc<Instant>,
    pub db: PgPool,
    pub config: SharedConfig,
}

impl AppState {
    pub fn new(db: PgPool, config: Config) -> Self {
        Self {
            started_at: Arc::new(Instant::now()),
            db,
            config: Arc::new(config),
        }
    }
}
