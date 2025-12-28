use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde_json::json;

use crate::errors::ApiError;
use crate::state::AppState;

pub async fn health_check(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    // Test database connection
    let result: Result<(i64,), sqlx::Error> = sqlx::query_as("SELECT 1 as health_check")
        .fetch_one(&state.db)
        .await;

    let health = match result {
        Ok((1,)) => json!({
            "status": "healthy",
            "database": "connected"
        }),
        _ => json!({
            "status": "unhealthy",
            "database": "disconnected"
        }),
    };

    Ok((StatusCode::OK, Json(health)))
}
