use std::{env, net::SocketAddr};

use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct Config {
    address: SocketAddr,
}

impl Config {
    pub fn from_env() -> Result<Self, AppError> {
        let host = env::var("API_HOST").unwrap_or_else(|_| "127.0.0.1".to_owned());
        let port = env::var("API_PORT").unwrap_or_else(|_| "3000".to_owned());
        let address = format!("{host}:{port}")
            .parse()
            .map_err(|_| AppError::Configuration("API_HOST ou API_PORT inválido".into()))?;

        Ok(Self { address })
    }

    pub fn address(&self) -> SocketAddr {
        self.address
    }
}
