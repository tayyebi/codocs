use actix_web::HttpResponse;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),

    #[error("unauthorized")]
    Unauthorized,

    #[error("forbidden")]
    Forbidden,

    #[error("not found")]
    NotFound,

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl actix_web::ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        match self {
            AppError::Unauthorized => {
                HttpResponse::Unauthorized().json(serde_json::json!({"error": "unauthorized"}))
            }
            AppError::Forbidden => {
                HttpResponse::Forbidden().json(serde_json::json!({"error": "forbidden"}))
            }
            AppError::NotFound => {
                HttpResponse::NotFound().json(serde_json::json!({"error": "not found"}))
            }
            AppError::BadRequest(msg) => {
                HttpResponse::BadRequest().json(serde_json::json!({"error": msg}))
            }
            AppError::Db(e) => {
                tracing::error!("db error: {e}");
                HttpResponse::InternalServerError()
                    .json(serde_json::json!({"error": "database error"}))
            }
            AppError::Internal(msg) => {
                tracing::error!("internal error: {msg}");
                HttpResponse::InternalServerError()
                    .json(serde_json::json!({"error": "internal server error"}))
            }
        }
    }
}
