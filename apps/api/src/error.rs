use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("configuração inválida: {0}")]
    Configuration(String),
    #[error("{0}")]
    Validation(String),
    #[error("autenticação necessária")]
    Unauthorized,
    #[error("acesso não autorizado")]
    Forbidden,
    #[error("recurso não encontrado")]
    NotFound,
    #[error("{0}")]
    Conflict(String),
    #[error("muitas tentativas; aguarde antes de tentar novamente")]
    RateLimited,
    #[error("erro interno")]
    Internal,
}

#[derive(Serialize)]
struct ErrorResponse {
    code: &'static str,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = match self {
            Self::Configuration(_) | Self::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
            Self::Validation(_) => (StatusCode::BAD_REQUEST, "validation_error"),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            Self::Forbidden => (StatusCode::FORBIDDEN, "forbidden"),
            Self::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            Self::Conflict(_) => (StatusCode::CONFLICT, "conflict"),
            Self::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "rate_limited"),
        };

        (
            status,
            Json(ErrorResponse {
                code,
                message: self.to_string(),
            }),
        )
            .into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        if let sqlx::Error::Database(database) = &error {
            if database.constraint() == Some("time_entries_no_user_overlap") {
                return Self::Conflict("já existe um registro que se sobrepõe a este período".into());
            }
            if database.is_unique_violation() {
                return Self::Conflict("já existe um registro com estes dados".into());
            }
        }
        tracing::error!(?error, "erro de banco de dados");
        Self::Internal
    }
}
