use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::cookie::{Cookie, PrivateCookieJar};
use time::Duration as TimeDuration;

use crate::state::AppState;

pub async fn check_authenticated(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    mut req: Request,
    next: middleware::Next,
) -> Result<Response, StatusCode> {
    let Some(cookie) = jar.get("sid").map(|c| c.value().to_owned()) else {
        return Ok(Redirect::to("/login").into_response());
    };

    // Verify session exists and hasn't expired
    let result: Result<(i64,), _> = sqlx::query_as(
        "SELECT COUNT(*) as count FROM sessions
         WHERE session_id = $1 AND expires_at > NOW()",
    )
    .bind(&cookie)
    .fetch_one(&state.db)
    .await;

    match result {
        Ok((count,)) if count > 0 => {
            req.extensions_mut().insert(cookie);
            Ok(next.run(req).await)
        }
        _ => {
            // Invalid or expired session - remove the cookie and redirect
            let removal_cookie = Cookie::build(("sid", ""))
                .path("/")
                .http_only(true)
                .same_site(axum_extra::extract::cookie::SameSite::Lax)
                .max_age(TimeDuration::seconds(-1));

            let jar = jar.add(removal_cookie);
            Ok((jar, Redirect::to("/login")).into_response())
        }
    }
}
