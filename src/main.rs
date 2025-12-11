use anyhow::Result;
use axum::{
    extract::{FromRef, FromRequest, FromRequestParts, Query, Request, State},
    http::StatusCode,
    middleware,
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
    Extension, Router,
};
use axum_extra::extract::cookie::{Cookie, Key, PrivateCookieJar};
use chrono::{Duration, Local};
use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthUrl, AuthorizationCode, ClientId,
    ClientSecret, PkceCodeChallenge, RedirectUrl, TokenResponse, TokenUrl,
};
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::Duration as StdDuration;
use time::Duration as TimeDuration;
use tower_http::{cors::CorsLayer, services::ServeDir};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod errors;
use errors::ApiError;

#[derive(Clone)]
pub struct AppState {
    db: PgPool,
    ctx: ReqwestClient,
    key: Key,
}

impl FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.key.clone()
    }
}

#[derive(Clone)]
pub struct OAuthClients {
    google: BasicClient,
    twitter: BasicClient,
}

#[derive(Clone)]
pub struct ClientIds {
    google: String,
    twitter: String,
}

// Store PKCE verifiers for Twitter
type PkceVerifiers = Arc<tokio::sync::Mutex<HashMap<String, String>>>;

#[derive(Debug, Deserialize)]
pub struct AuthRequest {
    code: String,
    state: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, sqlx::FromRow)]
pub struct UserProfile {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct GoogleUserInfo {
    pub email: String,
    pub name: Option<String>,
    pub picture: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TwitterUserInfo {
    pub data: TwitterUserData,
}

#[derive(Debug, Deserialize)]
pub struct TwitterUserData {
    pub id: String,
    pub name: String,
    pub username: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "oauth_axum=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load environment variables
    dotenv::dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:password@localhost/oauth_db".to_string());

    // Google OAuth credentials
    let google_client_id =
        env::var("GOOGLE_OAUTH_CLIENT_ID").expect("GOOGLE_OAUTH_CLIENT_ID must be set");
    let google_client_secret =
        env::var("GOOGLE_OAUTH_CLIENT_SECRET").expect("GOOGLE_OAUTH_CLIENT_SECRET must be set");

    // Twitter OAuth credentials
    let twitter_client_id =
        env::var("TWITTER_OAUTH_CLIENT_ID").expect("TWITTER_OAUTH_CLIENT_ID must be set");
    let twitter_client_secret =
        env::var("TWITTER_OAUTH_CLIENT_SECRET").expect("TWITTER_OAUTH_CLIENT_SECRET must be set");

    info!("Connecting to database...");

    // Create database connection pool
    let db = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(StdDuration::from_secs(3))
        .connect(&database_url)
        .await
        .expect("Failed to connect to database");

    // Run migrations
    info!("Running migrations...");
    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .expect("Failed to run migrations");

    // Create HTTP client
    let ctx = ReqwestClient::new();

    // Generate a secure key for cookie encryption
    let key = Key::generate();

    let state = AppState { db, ctx, key };

    // Build OAuth clients
    let google_oauth_client =
        build_google_oauth_client(google_client_id.clone(), google_client_secret);
    let twitter_oauth_client =
        build_twitter_oauth_client(twitter_client_id.clone(), twitter_client_secret);

    let oauth_clients = OAuthClients {
        google: google_oauth_client,
        twitter: twitter_oauth_client,
    };

    let client_ids = ClientIds {
        google: google_client_id,
        twitter: twitter_client_id,
    };

    // Initialize PKCE verifiers storage
    let pkce_verifiers: PkceVerifiers = Arc::new(tokio::sync::Mutex::new(HashMap::new()));

    // Initialize router
    let app = init_router(state, oauth_clients, client_ids, pkce_verifiers);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000")
        .await
        .expect("Failed to bind to port 8000");

    info!("Server running on http://localhost:8000");

    axum::serve(listener, app)
        .await
        .expect("Failed to start server");

    Ok(())
}

fn build_google_oauth_client(client_id: String, client_secret: String) -> BasicClient {
    let redirect_url = "http://localhost:8000/api/auth/google_callback".to_string();

    let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
        .expect("Invalid authorization endpoint URL");
    let token_url = TokenUrl::new("https://www.googleapis.com/oauth2/v3/token".to_string())
        .expect("Invalid token endpoint URL");

    BasicClient::new(
        ClientId::new(client_id),
        Some(ClientSecret::new(client_secret)),
        auth_url,
        Some(token_url),
    )
    .set_redirect_uri(RedirectUrl::new(redirect_url).unwrap())
}

fn build_twitter_oauth_client(client_id: String, client_secret: String) -> BasicClient {
    let redirect_url = "http://localhost:8000/api/auth/twitter_callback".to_string();

    let auth_url = AuthUrl::new("https://twitter.com/i/oauth2/authorize".to_string())
        .expect("Invalid authorization endpoint URL");
    let token_url = TokenUrl::new("https://api.twitter.com/2/oauth2/token".to_string())
        .expect("Invalid token endpoint URL");

    BasicClient::new(
        ClientId::new(client_id),
        Some(ClientSecret::new(client_secret)),
        auth_url,
        Some(token_url),
    )
    .set_redirect_uri(RedirectUrl::new(redirect_url).unwrap())
}

fn init_router(
    state: AppState,
    oauth_clients: OAuthClients,
    client_ids: ClientIds,
    pkce_verifiers: PkceVerifiers,
) -> Router {
    // Auth routes
    let auth_router = Router::new()
        .route("/auth/google_callback", get(google_callback))
        .route("/auth/twitter_callback", get(twitter_callback))
        .route("/auth/twitter_login", get(twitter_login))
        .route("/auth/logout", get(logout));

    // Protected routes
    let protected_router = Router::new()
        .route("/", get(protected))
        .route("/profile", get(get_profile))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            check_authenticated,
        ));

    // Public routes
    let public_router = Router::new()
        .route("/", get(homepage))
        .route("/login", get(login_page))
        .route("/health", get(health_check))
        .nest_service("/static", ServeDir::new("static"));

    Router::new()
        .nest("/api", auth_router)
        .nest("/protected", protected_router)
        .nest("/", public_router)
        .layer(Extension(oauth_clients))
        .layer(Extension(client_ids))
        .layer(Extension(pkce_verifiers))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn homepage(Extension(client_ids): Extension<ClientIds>) -> Html<String> {
    Html(format!(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>OAuth Demo</title>
            <style>
                body {{
                    font-family: Arial, sans-serif;
                    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                    min-height: 100vh;
                    display: flex;
                    justify-content: center;
                    align-items: center;
                }}
                .container {{
                    background: white;
                    border-radius: 20px;
                    padding: 40px;
                    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
                    text-align: center;
                    max-width: 600px;
                }}
                h1 {{
                    color: #333;
                    margin-bottom: 10px;
                }}
                .subtitle {{
                    color: #666;
                    margin-bottom: 30px;
                }}
                .button-group {{
                    display: flex;
                    gap: 15px;
                    justify-content: center;
                    margin: 20px 0;
                }}
                .button {{
                    display: inline-flex;
                    align-items: center;
                    justify-content: center;
                    padding: 12px 24px;
                    color: white;
                    text-decoration: none;
                    border-radius: 5px;
                    font-weight: 500;
                    transition: all 0.3s ease;
                    flex: 1;
                }}
                .button.google {{
                    background-color: #4285f4;
                }}
                .button.google:hover {{
                    background-color: #357ae8;
                    transform: translateY(-2px);
                    box-shadow: 0 10px 20px rgba(66, 133, 244, 0.3);
                }}
                .button.twitter {{
                    background-color: #1DA1F2;
                }}
                .button.twitter:hover {{
                    background-color: #1a91da;
                    transform: translateY(-2px);
                    box-shadow: 0 10px 20px rgba(29, 161, 242, 0.3);
                }}
                .button.protected {{
                    background-color: #667eea;
                    margin-top: 10px;
                }}
                .button.protected:hover {{
                    background-color: #5a67d8;
                    transform: translateY(-2px);
                }}
            </style>
        </head>
        <body>
            <div class="container">
                <h1>🔐 OAuth Demo</h1>
                <p class="subtitle">Secure OAuth2 authentication with Google and Twitter</p>

                <div class="button-group">
                    <a href="https://accounts.google.com/o/oauth2/v2/auth?scope=openid%20profile%20email&client_id={}&response_type=code&redirect_uri=http://localhost:8000/api/auth/google_callback"
                       class="button google">
                        <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" style="margin-right: 8px;">
                            <path d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"/>
                            <path d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"/>
                            <path d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"/>
                            <path d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"/>
                        </svg>
                        Google
                    </a>

                    <a href="/api/auth/twitter_login"
                       class="button twitter">
                        <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" style="margin-right: 8px;">
                            <path d="M23.643 4.937c-.835.37-1.732.62-2.675.733.962-.576 1.7-1.49 2.048-2.578-.9.534-1.897.922-2.958 1.13-.85-.904-2.06-1.47-3.4-1.47-2.572 0-4.658 2.086-4.658 4.66 0 .364.042.718.12 1.06-3.873-.195-7.304-2.05-9.602-4.868-.4.69-.63 1.49-.63 2.342 0 1.616.823 3.043 2.072 3.878-.764-.025-1.482-.234-2.11-.583v.06c0 2.257 1.605 4.14 3.737 4.568-.392.106-.803.162-1.227.162-.3 0-.593-.028-.877-.082.593 1.85 2.313 3.198 4.352 3.234-1.595 1.25-3.604 1.995-5.786 1.995-.376 0-.747-.022-1.112-.065 2.062 1.323 4.51 2.093 7.14 2.093 8.57 0 13.255-7.098 13.255-13.254 0-.2-.005-.402-.014-.602.91-.658 1.7-1.477 2.323-2.41z"/>
                        </svg>
                        Twitter
                    </a>
                </div>

                <a href="/protected" class="button protected">🔒 Access Protected Area</a>
            </div>
        </body>
        </html>
        "#,
        client_ids.google
    ))
}

async fn login_page(Extension(client_ids): Extension<ClientIds>) -> Html<String> {
    Html(format!(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Login - OAuth Demo</title>
            <style>
                body {{
                    font-family: Arial, sans-serif;
                    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                    min-height: 100vh;
                    display: flex;
                    justify-content: center;
                    align-items: center;
                }}
                .login-container {{
                    background: white;
                    border-radius: 20px;
                    padding: 40px;
                    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
                    text-align: center;
                    max-width: 500px;
                }}
                .oauth-button {{
                    display: flex;
                    align-items: center;
                    justify-content: center;
                    padding: 12px 24px;
                    color: white;
                    text-decoration: none;
                    border-radius: 5px;
                    font-size: 16px;
                    font-weight: 500;
                    margin: 15px 0;
                    transition: all 0.3s ease;
                }}
                .google-button {{
                    background-color: #4285f4;
                }}
                .google-button:hover {{
                    background-color: #357ae8;
                    transform: translateY(-2px);
                }}
                .twitter-button {{
                    background-color: #1DA1F2;
                }}
                .twitter-button:hover {{
                    background-color: #1a91da;
                    transform: translateY(-2px);
                }}
            </style>
        </head>
        <body>
            <div class="login-container">
                <h2>Login</h2>
                <p>Choose your preferred sign-in method</p>

                <div style="display: flex; gap: 15px; margin-top: 20px;">
                    <a href="https://accounts.google.com/o/oauth2/v2/auth?scope=openid%20profile%20email&client_id={google_id}&response_type=code&redirect_uri=http://localhost:8000/api/auth/google_callback"
                       class="oauth-button google-button" style="flex: 1;">
                        <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" style="margin-right: 8px;">
                            <path d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"/>
                            <path d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"/>
                            <path d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"/>
                            <path d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"/>
                        </svg>
                        Sign in with Google
                    </a>

                    <a href="/api/auth/twitter_login"
                       class="oauth-button twitter-button" style="flex: 1;">
                        <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" style="margin-right: 8px;">
                            <path d="M23.643 4.937c-.835.37-1.732.62-2.675.733.962-.576 1.7-1.49 2.048-2.578-.9.534-1.897.922-2.958 1.13-.85-.904-2.06-1.47-3.4-1.47-2.572 0-4.658 2.086-4.658 4.66 0 .364.042.718.12 1.06-3.873-.195-7.304-2.05-9.602-4.868-.4.69-.63 1.49-.63 2.342 0 1.616.823 3.043 2.072 3.878-.764-.025-1.482-.234-2.11-.583v.06c0 2.257 1.605 4.14 3.737 4.568-.392.106-.803.162-1.227.162-.3 0-.593-.028-.877-.082.593 1.85 2.313 3.198 4.352 3.234-1.595 1.25-3.604 1.995-5.786 1.995-.376 0-.747-.022-1.112-.065 2.062 1.323 4.51 2.093 7.14 2.093 8.57 0 13.255-7.098 13.255-13.254 0-.2-.005-.402-.014-.602.91-.658 1.7-1.477 2.323-2.41z"/>
                        </svg>
                        Sign in with Twitter
                    </a>
                </div>
            </div>
        </body>
        </html>
        "#,
        google_id = client_ids.google
    ))
}

// Twitter needs a special login handler for PKCE
async fn twitter_login(
    Extension(oauth_clients): Extension<OAuthClients>,
    Extension(pkce_verifiers): Extension<PkceVerifiers>,
) -> Redirect {
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, _csrf_token) = oauth_clients
        .twitter
        .authorize_url(oauth2::CsrfToken::new_random)
        .add_scope(oauth2::Scope::new("tweet.read".to_string()))
        .add_scope(oauth2::Scope::new("users.read".to_string()))
        .add_scope(oauth2::Scope::new("offline.access".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    // Store the verifier for later use
    let mut verifiers = pkce_verifiers.lock().await;
    verifiers.insert(
        "twitter_verifier".to_string(),
        pkce_verifier.secret().clone(),
    );

    Redirect::to(auth_url.as_str())
}

async fn google_callback(
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

    // Use the access token to get user info from Google
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

async fn twitter_callback(
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

async fn store_user_session(
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

async fn logout(State(state): State<AppState>, jar: PrivateCookieJar) -> impl IntoResponse {
    // Get the session ID from the cookie before removing it
    if let Some(cookie) = jar.get("sid") {
        let session_id = cookie.value();

        // Delete the session from the database
        let _ = sqlx::query("DELETE FROM sessions WHERE session_id = $1")
            .bind(session_id)
            .execute(&state.db)
            .await;
    }

    // Create an expired cookie to properly remove it from the browser
    let removal_cookie = Cookie::build(("sid", ""))
        .path("/")
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .max_age(TimeDuration::seconds(-1)); // Negative max_age expires the cookie

    let jar = jar.add(removal_cookie);
    (jar, Redirect::to("/"))
}

async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    // Check database connection
    let db_status = sqlx::query("SELECT 1")
        .fetch_one(&state.db)
        .await
        .map(|_| "healthy")
        .unwrap_or("unhealthy");

    let health = serde_json::json!({
        "status": "ok",
        "service": "oauth_axum",
        "database": db_status,
        "providers": ["google", "twitter"],
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    (StatusCode::OK, axum::Json(health))
}

async fn protected(user: UserProfile) -> Html<String> {
    let provider = if user.email.ends_with("@twitter.local") {
        "Twitter"
    } else {
        "Google"
    };

    Html(format!(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Protected Area</title>
            <style>
                body {{
                    font-family: Arial, sans-serif;
                    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                    min-height: 100vh;
                    padding: 20px;
                }}
                .container {{
                    max-width: 800px;
                    margin: 0 auto;
                    background: white;
                    border-radius: 20px;
                    padding: 40px;
                    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
                }}
                .info {{
                    background-color: #f0f8ff;
                    padding: 20px;
                    border-radius: 5px;
                    margin: 20px 0;
                }}
                .button {{
                    display: inline-block;
                    padding: 10px 20px;
                    background-color: #4285f4;
                    color: white;
                    text-decoration: none;
                    border-radius: 5px;
                    margin: 10px;
                }}
                .button.logout {{
                    background-color: #dc3545;
                }}
            </style>
        </head>
        <body>
            <div class="container">
                <h1>Protected Area</h1>
                <div class="info">
                    <h2>Welcome!</h2>
                    <p>You are authenticated as: <strong>{}</strong></p>
                    <p>Provider: <strong>{}</strong></p>
                </div>
                <a href="/protected/profile" class="button">View Profile</a>
                <a href="/api/auth/logout" class="button logout">Logout</a>
            </div>
        </body>
        </html>
        "#,
        user.email, provider
    ))
}

async fn get_profile(user: UserProfile) -> impl IntoResponse {
    let (provider, display_name) = if user.email.ends_with("@twitter.local") {
        ("Twitter", user.email.replace("@twitter.local", ""))
    } else {
        ("Google", user.email.clone())
    };

    Html(format!(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>User Profile</title>
            <style>
                body {{
                    font-family: Arial, sans-serif;
                    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                    min-height: 100vh;
                    padding: 20px;
                }}
                .profile-card {{
                    max-width: 600px;
                    margin: 0 auto;
                    background: white;
                    padding: 30px;
                    border-radius: 10px;
                    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
                }}
                .button {{
                    display: inline-block;
                    padding: 10px 20px;
                    background-color: #4285f4;
                    color: white;
                    text-decoration: none;
                    border-radius: 5px;
                    margin-top: 20px;
                }}
            </style>
        </head>
        <body>
            <div class="profile-card">
                <h2>User Profile</h2>
                <p><strong>Provider:</strong> {}</p>
                <p><strong>Display Name:</strong> {}</p>
                <p><strong>Email/ID:</strong> {}</p>
                <a href="/protected" class="button">Back to Protected Area</a>
            </div>
        </body>
        </html>
        "#,
        provider, display_name, user.email
    ))
}

async fn check_authenticated(
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
