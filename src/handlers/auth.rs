use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
    Extension,
};
use axum_extra::extract::cookie::PrivateCookieJar;
use oauth2::{reqwest::async_http_client, AuthorizationCode, PkceCodeChallenge, TokenResponse};

use crate::errors::ApiError;
use crate::oauth::{AuthRequest, GoogleUserInfo, OAuthClients, PkceVerifiers, TwitterUserInfo};
use crate::services::session::store_user_session;
use crate::state::AppState;

pub async fn twitter_login(
    Extension(oauth_clients): Extension<OAuthClients>,
    Extension(pkce_verifiers): Extension<PkceVerifiers>,
) -> impl IntoResponse {
    // Generate PKCE challenge
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    // Store the verifier for later use
    let mut verifiers = pkce_verifiers.lock().await;
    verifiers.insert(
        "twitter_verifier".to_string(),
        pkce_verifier.secret().clone(),
    );

    // Generate authorization URL with PKCE
    let (auth_url, _) = oauth_clients
        .twitter
        .authorize_url(oauth2::CsrfToken::new_random)
        .add_scope(oauth2::Scope::new("tweet.read".to_string()))
        .add_scope(oauth2::Scope::new("users.read".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    Redirect::to(auth_url.as_str())
}

pub async fn google_callback(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    Query(query): Query<AuthRequest>,
    Extension(oauth_clients): Extension<OAuthClients>,
) -> Result<impl IntoResponse, ApiError> {
    // Exchange the authorization code for an access token
    let token = oauth_clients
        .google
        .exchange_code(AuthorizationCode::new(query.code))
        .request_async(async_http_client)
        .await?;

    // Use the access token to get user info
    let profile = state
        .ctx
        .get("https://openidconnect.googleapis.com/v1/userinfo")
        .bearer_auth(token.access_token().secret().to_owned())
        .send()
        .await?
        .json::<GoogleUserInfo>()
        .await?;

    // Store session
    store_user_session(State(state), jar, profile.email, token).await
}

pub async fn twitter_callback(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    Query(query): Query<AuthRequest>,
    Extension(oauth_clients): Extension<OAuthClients>,
    Extension(pkce_verifiers): Extension<PkceVerifiers>,
) -> Result<impl IntoResponse, ApiError> {
    // Retrieve the PKCE verifier
    let mut verifiers = pkce_verifiers.lock().await;
    let pkce_verifier = verifiers
        .remove("twitter_verifier")
        .ok_or_else(|| ApiError::BadRequest("Missing PKCE verifier".to_string()))?;

    // Exchange the authorization code for an access token with PKCE
    let token = oauth_clients
        .twitter
        .exchange_code(AuthorizationCode::new(query.code))
        .set_pkce_verifier(oauth2::PkceCodeVerifier::new(pkce_verifier))
        .request_async(async_http_client)
        .await?;

    // Use the access token to get user info from Twitter
    let profile = state
        .ctx
        .get("https://api.twitter.com/2/users/me")
        .bearer_auth(token.access_token().secret().to_owned())
        .send()
        .await?
        .json::<TwitterUserInfo>()
        .await?;

    // Use Twitter username as email (Twitter doesn't provide email in v2 API easily)
    let email = format!("{}@twitter.local", profile.data.username);

    // Store session
    store_user_session(State(state), jar, email, token).await
}
