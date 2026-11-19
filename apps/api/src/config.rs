use std::{env, fs, net::SocketAddr, sync::Arc};

use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct Config {
    address: SocketAddr,
    pub database_url: String,
    pub production: bool,
    pub allowed_origin: String,
    pub entra_tenant_id: Uuid,
    pub entra_client_id: String,
    pub entra_client_secret: String,
    pub entra_redirect_uri: String,
}

pub fn read_secret(name: &str) -> Result<String, AppError> {
    let value = if let Ok(path) = env::var(format!("{name}_FILE")) {
        fs::read_to_string(path)
            .map_err(|_| AppError::Configuration(format!("não foi possível ler {name}_FILE")))?
    } else {
        env::var(name).map_err(|_| AppError::Configuration(format!("{name} não definido")))?
    };
    let value = value.trim().to_owned();
    if value.is_empty() {
        return Err(AppError::Configuration(format!("{name} vazio")));
    }
    Ok(value)
}

impl Config {
    #[cfg(test)]
    pub(crate) fn for_test() -> Self {
        Self {
            address: "127.0.0.1:0".parse().expect("endereço de teste"),
            database_url: String::new(),
            production: false,
            allowed_origin: "http://localhost:5173".into(),
            entra_tenant_id: Uuid::nil(),
            entra_client_id: Uuid::nil().to_string(),
            entra_client_secret: String::new(),
            entra_redirect_uri: "http://localhost:5173/api/auth/callback".into(),
        }
    }

    pub fn from_env() -> Result<Self, AppError> {
        let host = env::var("API_HOST").unwrap_or_else(|_| "127.0.0.1".to_owned());
        let port = env::var("API_PORT").unwrap_or_else(|_| "3000".to_owned());
        let address = format!("{host}:{port}")
            .parse()
            .map_err(|_| AppError::Configuration("API_HOST ou API_PORT inválido".into()))?;
        let production =
            env::var("APP_ENV").unwrap_or_else(|_| "development".into()) == "production";
        let allowed_origin = env::var("ALLOWED_ORIGIN")
            .map_err(|_| AppError::Configuration("ALLOWED_ORIGIN é obrigatório".into()))?;
        let origin = openidconnect::url::Url::parse(&allowed_origin)
            .map_err(|_| AppError::Configuration("ALLOWED_ORIGIN inválido".into()))?;
        if !matches!(origin.scheme(), "http" | "https")
            || origin.host_str().is_none()
            || origin.path() != "/"
            || origin.query().is_some()
            || origin.fragment().is_some()
            || (production && origin.scheme() != "https")
        {
            return Err(AppError::Configuration(
                "ALLOWED_ORIGIN deve ser uma origem HTTP(S) válida".into(),
            ));
        }
        let entra_tenant_id = env::var("ENTRA_TENANT_ID")
            .map_err(|_| AppError::Configuration("ENTRA_TENANT_ID não definido".into()))?
            .parse::<Uuid>()
            .map_err(|_| AppError::Configuration("ENTRA_TENANT_ID inválido".into()))?;
        let entra_client_id = env::var("ENTRA_CLIENT_ID")
            .map_err(|_| AppError::Configuration("ENTRA_CLIENT_ID não definido".into()))?;
        Uuid::parse_str(&entra_client_id)
            .map_err(|_| AppError::Configuration("ENTRA_CLIENT_ID inválido".into()))?;
        let allowed_origin = allowed_origin.trim_end_matches('/').to_owned();
        let entra_redirect_uri = format!("{allowed_origin}/api/auth/callback");

        Ok(Self {
            address,
            database_url: read_secret("DATABASE_URL")?,
            production,
            allowed_origin,
            entra_tenant_id,
            entra_client_id,
            entra_client_secret: read_secret("ENTRA_CLIENT_SECRET")?,
            entra_redirect_uri,
        })
    }

    pub fn address(&self) -> SocketAddr {
        self.address
    }
    pub fn issuer(&self) -> String {
        format!(
            "https://login.microsoftonline.com/{}/v2.0",
            self.entra_tenant_id
        )
    }
    pub fn cookie_name(&self) -> &'static str {
        if self.production {
            "__Host-horasculo_session"
        } else {
            "horasculo_session"
        }
    }
}

pub type SharedConfig = Arc<Config>;
