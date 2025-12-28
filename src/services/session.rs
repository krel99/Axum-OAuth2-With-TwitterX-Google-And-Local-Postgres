use axum::{
    extract::State,
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::cookie::{Cookie, PrivateCookieJar};
use chrono::{Duration, Local};
use oauth2::TokenResponse;
use time::Duration as TimeDuration;

use crate::errors::ApiError;
use crate::state::AppState;

pub async fn store_user_session(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    email: String,
    token: impl TokenResponse<oauth2::basic::BasicTokenType>,
) -> Result<impl IntoResponse, ApiError> {
    // Calculate session expiry
    let secs = token
        .expires_in()
        .map(|d| d.as_secs() as i64)
        .unwrap_or(3600); // Default to 1 hour if not provided

    let max_age = Local::now().naive_local() + Duration::seconds(secs);

    // Generate a session ID
    let session_id = format!("{}:{}", email, token.access_token().secret());

    // Create secure cookie with expiration
    let cookie = Cookie::build(("sid", session_id.clone()))
        .path("/")
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .max_age(TimeDuration::seconds(secs));

    // Store user in database
    sqlx::query(
        "INSERT INTO users (email) VALUES ($1)
         ON CONFLICT (email) DO UPDATE SET last_updated = CURRENT_TIMESTAMP",
    )
    .bind(&email)
    .execute(&state.db)
    .await?;

    // Store session in database
    sqlx::query(
        "INSERT INTO sessions (user_id, session_id, expires_at) VALUES (
            (SELECT id FROM users WHERE email = $1 LIMIT 1),
            $2, $3
        )
        ON CONFLICT (user_id) DO UPDATE SET
            session_id = excluded.session_id,
            expires_at = excluded.expires_at",
    )
    .bind(&email)
    .bind(&session_id)
    .bind(max_age)
    .execute(&state.db)
    .await?;

    Ok((jar.add(cookie), Redirect::to("/protected")))
}

pub async fn logout(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
) -> Result<impl IntoResponse, ApiError> {
    // Get the session cookie to invalidate it in the database
    if let Some(cookie) = jar.get("sid") {
        let session_id = cookie.value();

        // Remove session from database
        sqlx::query("DELETE FROM sessions WHERE session_id = $1")
            .bind(session_id)
            .execute(&state.db)
            .await?;
    }

    // Remove the cookie
    let removal_cookie = Cookie::build(("sid", ""))
        .path("/")
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .max_age(TimeDuration::seconds(-1));

    Ok((jar.add(removal_cookie), Redirect::to("/")))
}
