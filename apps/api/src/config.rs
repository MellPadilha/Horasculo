use std::{env, net::SocketAddr, sync::Arc};

use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct Config {
    address: SocketAddr,
    pub database_url: String,
    pub production: bool,
    pub session_days: i64,
    pub bootstrap_admin_username: Option<String>,
    pub bootstrap_admin_password: Option<String>,
    pub bootstrap_admin_name: String,
    pub allowed_origin: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self, AppError> {
        let host = env::var("API_HOST").unwrap_or_else(|_| "127.0.0.1".to_owned());
        let port = env::var("API_PORT").unwrap_or_else(|_| "3000".to_owned());
        let address = format!("{host}:{port}")
            .parse()
            .map_err(|_| AppError::Configuration("API_HOST ou API_PORT inválido".into()))?;

        let database_url = env::var("DATABASE_URL")
            .map_err(|_| AppError::Configuration("DATABASE_URL não definida".into()))?;
        let production = env::var("APP_ENV").unwrap_or_else(|_| "development".into()) == "production";
        let session_days = env::var("SESSION_DAYS")
            .unwrap_or_else(|_| "7".into())
            .parse::<i64>()
            .map_err(|_| AppError::Configuration("SESSION_DAYS inválido".into()))?;
        if !(1..=30).contains(&session_days) {
            return Err(AppError::Configuration("SESSION_DAYS deve estar entre 1 e 30".into()));
        }

        let allowed_origin = env::var("ALLOWED_ORIGIN").ok();
        if production && allowed_origin.is_none() {
            return Err(AppError::Configuration("ALLOWED_ORIGIN é obrigatório em produção".into()));
        }

        Ok(Self {
            address,
            database_url,
            production,
            session_days,
            bootstrap_admin_username: env::var("BOOTSTRAP_ADMIN_USERNAME").ok(),
            bootstrap_admin_password: env::var("BOOTSTRAP_ADMIN_PASSWORD").ok(),
            bootstrap_admin_name: env::var("BOOTSTRAP_ADMIN_NAME").unwrap_or_else(|_| "Administrador".into()),
            allowed_origin,
        })
    }

    pub fn address(&self) -> SocketAddr {
        self.address
    }
}

pub type SharedConfig = Arc<Config>;
