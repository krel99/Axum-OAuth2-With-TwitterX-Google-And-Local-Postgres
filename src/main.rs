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
    ClientSecret, RedirectUrl, TokenResponse, TokenUrl,
};
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::env;
use std::time::Duration as StdDuration;
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

    let oauth_client_id =
        env::var("GOOGLE_OAUTH_CLIENT_ID").expect("GOOGLE_OAUTH_CLIENT_ID must be set");
    let oauth_client_secret =
        env::var("GOOGLE_OAUTH_CLIENT_SECRET").expect("GOOGLE_OAUTH_CLIENT_SECRET must be set");

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

    // Build OAuth client
    let oauth_client = build_oauth_client(oauth_client_id.clone(), oauth_client_secret);

    // Initialize router
    let app = init_router(state, oauth_client, oauth_client_id);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000")
        .await
        .expect("Failed to bind to port 8000");

    info!("Server running on http://localhost:8000");

    axum::serve(listener, app)
        .await
        .expect("Failed to start server");

    Ok(())
}

fn build_oauth_client(client_id: String, client_secret: String) -> BasicClient {
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

fn init_router(state: AppState, oauth_client: BasicClient, oauth_id: String) -> Router {
    // Auth routes
    let auth_router = Router::new()
        .route("/auth/google_callback", get(google_callback))
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
        .layer(Extension(oauth_client))
        .layer(Extension(oauth_id))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn homepage() -> impl IntoResponse {
    // Serve the enhanced static HTML file if it exists, otherwise fallback to simple HTML
    if std::path::Path::new("static/index.html").exists() {
        match tokio::fs::read_to_string("static/index.html").await {
            Ok(content) => Html(content),
            Err(_) => Html(simple_homepage()),
        }
    } else {
        Html(simple_homepage())
    }
}

fn simple_homepage() -> String {
    format!(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>OAuth Demo</title>
            <style>
                body {{
                    font-family: Arial, sans-serif;
                    max-width: 800px;
                    margin: 50px auto;
                    padding: 20px;
                    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                    min-height: 100vh;
                }}
                .container {{
                    background: white;
                    border-radius: 20px;
                    padding: 40px;
                    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
                }}
                .button {{
                    display: inline-block;
                    padding: 12px 24px;
                    background-color: #4285f4;
                    color: white;
                    text-decoration: none;
                    border-radius: 5px;
                    margin: 10px;
                    font-weight: 500;
                    transition: all 0.3s ease;
                }}
                .button:hover {{
                    background-color: #357ae8;
                    transform: translateY(-2px);
                    box-shadow: 0 10px 20px rgba(66, 133, 244, 0.3);
                }}
            </style>
        </head>
        <body>
            <div class="container">
                <h1>🔐 OAuth Demo</h1>
                <p>Secure OAuth2 authentication with Google.</p>
                <a href="/login" class="button">Sign in with Google</a>
                <a href="/protected" class="button">Access Protected Area</a>
                <a href="/static/index.html" class="button">Interactive Demo</a>
            </div>
        </body>
        </html>
        "#
    )
}

async fn login_page(Extension(oauth_id): Extension<String>) -> Html<String> {
    Html(format!(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Login - OAuth Demo</title>
            <style>
                body {{
                    font-family: Arial, sans-serif;
                    max-width: 500px;
                    margin: 100px auto;
                    padding: 20px;
                    text-align: center;
                }}
                .google-button {{
                    display: inline-block;
                    padding: 12px 24px;
                    background-color: #4285f4;
                    color: white;
                    text-decoration: none;
                    border-radius: 5px;
                    font-size: 16px;
                }}
                .google-button:hover {{
                    background-color: #357ae8;
                }}
            </style>
        </head>
        <body>
            <h2>Login</h2>
            <p>Sign in with your Google account to continue</p>

            <a href="https://accounts.google.com/o/oauth2/v2/auth?scope=openid%20profile%20email&client_id={oauth_id}&response_type=code&redirect_uri=http://localhost:8000/api/auth/google_callback"
               class="google-button">
                Sign in with Google
            </a>
        </body>
        </html>
        "#,
        oauth_id = oauth_id
    ))
}

async fn google_callback(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    Query(query): Query<AuthRequest>,
    Extension(oauth_client): Extension<BasicClient>,
) -> Result<impl IntoResponse, ApiError> {
    // Exchange the authorization code for an access token
    let token = oauth_client
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

    // Calculate session expiry
    let secs = token
        .expires_in()
        .map(|d| d.as_secs() as i64)
        .unwrap_or(3600); // Default to 1 hour if not provided

    let max_age = Local::now().naive_local() + Duration::seconds(secs);

    // Generate a session ID
    let session_id = format!("{}:{}", profile.email, token.access_token().secret());

    // Create secure cookie
    let cookie = Cookie::build(("sid", session_id.clone()))
        .path("/")
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax);

    // Store user in database
    sqlx::query(
        "INSERT INTO users (email) VALUES ($1)
         ON CONFLICT (email) DO UPDATE SET last_updated = CURRENT_TIMESTAMP",
    )
    .bind(&profile.email)
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
    .bind(&profile.email)
    .bind(&session_id)
    .bind(max_age)
    .execute(&state.db)
    .await?;

    Ok((jar.add(cookie), Redirect::to("/protected")))
}

async fn logout(jar: PrivateCookieJar) -> impl IntoResponse {
    let jar = jar.remove(Cookie::from("sid"));
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
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    (StatusCode::OK, axum::Json(health))
}

async fn protected(user: UserProfile) -> Html<String> {
    Html(format!(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Protected Area - OAuth Demo</title>
            <style>
                body {{
                    font-family: Arial, sans-serif;
                    max-width: 800px;
                    margin: 50px auto;
                    padding: 20px;
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
                .button:hover {{
                    background-color: #357ae8;
                }}
                .logout {{
                    background-color: #dc3545;
                }}
                .logout:hover {{
                    background-color: #c82333;
                }}
            </style>
        </head>
        <body>
            <h1>Protected Area</h1>
            <div class="info">
                <h2>Welcome!</h2>
                <p>You are successfully authenticated as: <strong>{}</strong></p>
                <p>This is a protected route that requires authentication.</p>
            </div>

            <a href="/protected/profile" class="button">View Profile</a>
            <a href="/api/auth/logout" class="button logout">Logout</a>
        </body>
        </html>
        "#,
        user.email
    ))
}

async fn get_profile(user: UserProfile) -> impl IntoResponse {
    Html(format!(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>User Profile - OAuth Demo</title>
            <style>
                body {{
                    font-family: Arial, sans-serif;
                    max-width: 600px;
                    margin: 50px auto;
                    padding: 20px;
                }}
                .profile-card {{
                    background-color: #f8f9fa;
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
                <p><strong>Email:</strong> {}</p>
                <p><strong>Status:</strong> Authenticated</p>
                <a href="/protected" class="button">Back to Protected Area</a>
            </div>
        </body>
        </html>
        "#,
        user.email
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
    let result = sqlx::query(
        "SELECT COUNT(*) as count FROM sessions
         WHERE session_id = $1 AND expires_at > NOW()",
    )
    .bind(&cookie)
    .fetch_one(&state.db)
    .await;

    match result {
        Ok(_) => {
            req.extensions_mut().insert(cookie);
            Ok(next.run(req).await)
        }
        Err(_) => Ok(Redirect::to("/login").into_response()),
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
        .await?;

        Ok(user)
    }
}
