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
        tracing::error!(?error, "erro de banco de dados");
        Self::Internal
    }
}
