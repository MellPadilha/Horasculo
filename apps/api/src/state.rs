use std::{sync::Arc, time::Instant};

use openidconnect::reqwest;
use sqlx::PgPool;
use tokio::sync::Semaphore;

use crate::config::{Config, SharedConfig};

#[derive(Clone)]
pub struct AppState {
    pub started_at: Arc<Instant>,
    pub db: PgPool,
    pub config: SharedConfig,
    pub http: reqwest::Client,
    pub exports: Arc<Semaphore>,
    pub auth_flows: Arc<Semaphore>,
}

impl AppState {
    pub fn new(db: PgPool, config: Config, http: reqwest::Client) -> Self {
        Self {
            started_at: Arc::new(Instant::now()),
            db,
            config: Arc::new(config),
            http,
            exports: Arc::new(Semaphore::new(2)),
            auth_flows: Arc::new(Semaphore::new(16)),
        }
    }
}
