use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("HTTP request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("OAuth token error: {0}")]
    TokenError(
        #[from]
        oauth2::RequestTokenError<
            oauth2::reqwest::Error<reqwest::Error>,
            oauth2::StandardErrorResponse<oauth2::basic::BasicErrorResponseType>,
        >,
    ),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Internal server error")]
    InternalServerError,

    #[error("Bad request: {0}")]
    BadRequest(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            Self::Database(e) => {
                tracing::error!("Database error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database error occurred".to_string(),
                )
            }
            Self::Request(e) => {
                tracing::error!("HTTP request error: {}", e);
                (
                    StatusCode::BAD_GATEWAY,
                    "External service error".to_string(),
                )
            }
            Self::TokenError(e) => {
                tracing::error!("OAuth token error: {}", e);
                (
                    StatusCode::UNAUTHORIZED,
                    "Authentication failed".to_string(),
                )
            }
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "You are not authorized to access this resource".to_string(),
            ),
            Self::InternalServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
        };

        (status, error_message).into_response()
    }
}
