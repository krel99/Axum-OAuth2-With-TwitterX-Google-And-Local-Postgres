use serde::{Deserialize, Serialize};

use axum::extract::{FromRequest, Request};

use crate::errors::ApiError;
use crate::state::AppState;
use axum::extract::FromRequestParts;
use axum_extra::extract::cookie::{Key, PrivateCookieJar};

#[derive(Debug, Deserialize, Serialize, Clone, sqlx::FromRow)]
pub struct UserProfile {
    pub email: String,
}

#[axum::async_trait]
impl FromRequest<AppState> for UserProfile {
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &AppState) -> Result<Self, Self::Rejection> {
        let (mut parts, _body) = req.into_parts();

        let jar: PrivateCookieJar<Key> = PrivateCookieJar::from_request_parts(&mut parts, state)
            .await
            .map_err(|_| ApiError::Unauthorized)?;

        let Some(cookie) = jar.get("sid").map(|cookie| cookie.value().to_owned()) else {
            return Err(ApiError::Unauthorized);
        };

        let user = sqlx::query_as::<_, UserProfile>(
            "SELECT users.email
             FROM sessions
             LEFT JOIN users ON sessions.user_id = users.id
             WHERE sessions.session_id = $1 AND sessions.expires_at > NOW()
             LIMIT 1",
        )
        .bind(cookie)
        .fetch_one(&state.db)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => ApiError::Unauthorized,
            _ => ApiError::Database(e),
        })?;

        Ok(user)
    }
}
