use anyhow::Result;
use oauth2::basic::BasicClient;
use reqwest::Client as ReqwestClient;
use sqlx::postgres::PgPoolOptions;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::Duration as StdDuration;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
use config::init_router;

mod errors;

mod handlers;

mod middleware;

mod oauth;
use oauth::{ClientIds, OAuthClients, PkceVerifiers};

mod services;

mod state;
use state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "oauth_axum=debug,axum::rejection=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load environment variables
    dotenv::dotenv().ok();

    // Database connection
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let db = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(StdDuration::from_secs(3))
        .connect(&database_url)
        .await
        .expect("Failed to connect to database");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .expect("Failed to run migrations");

    // Create HTTP client with timeout
    let ctx = ReqwestClient::builder()
        .timeout(StdDuration::from_secs(30))
        .build()?;

    // Initialize OAuth clients
    let google_client_id =
        env::var("GOOGLE_OAUTH_CLIENT_ID").expect("GOOGLE_OAUTH_CLIENT_ID not set");
    let google_client_secret =
        env::var("GOOGLE_OAUTH_CLIENT_SECRET").expect("GOOGLE_OAUTH_CLIENT_SECRET not set");

    let twitter_client_id =
        env::var("TWITTER_OAUTH_CLIENT_ID").expect("TWITTER_OAUTH_CLIENT_ID not set");
    let twitter_client_secret =
        env::var("TWITTER_OAUTH_CLIENT_SECRET").expect("TWITTER_OAUTH_CLIENT_SECRET not set");

    let google_client = BasicClient::new(
        oauth2::ClientId::new(google_client_id.clone()),
        Some(oauth2::ClientSecret::new(google_client_secret)),
        oauth2::AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())?,
        Some(oauth2::TokenUrl::new(
            "https://oauth2.googleapis.com/token".to_string(),
        )?),
    )
    .set_redirect_uri(oauth2::RedirectUrl::new(
        "http://localhost:8000/api/auth/google_callback".to_string(),
    )?);

    let twitter_client = BasicClient::new(
        oauth2::ClientId::new(twitter_client_id.clone()),
        Some(oauth2::ClientSecret::new(twitter_client_secret)),
        oauth2::AuthUrl::new("https://twitter.com/i/oauth2/authorize".to_string())?,
        Some(oauth2::TokenUrl::new(
            "https://api.twitter.com/2/oauth2/token".to_string(),
        )?),
    )
    .set_redirect_uri(oauth2::RedirectUrl::new(
        "http://localhost:8000/api/auth/twitter_callback".to_string(),
    )?);

    // Generate a secure key for cookie encryption
    let cookie_key = env::var("COOKIE_KEY").unwrap_or_else(|_| {
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string()
    });

    let key = axum_extra::extract::cookie::Key::from(cookie_key.as_bytes());

    // Build app state
    let state = AppState { db, ctx, key };

    let oauth_clients = OAuthClients {
        google: google_client,
        twitter: twitter_client,
    };

    let client_ids = ClientIds {
        google: google_client_id,
        twitter: twitter_client_id,
    };

    let pkce_verifiers: PkceVerifiers = Arc::new(tokio::sync::Mutex::new(HashMap::new()));

    // Build router
    let app = init_router(state.clone(), oauth_clients, client_ids, pkce_verifiers);

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();

    info!("Server running on http://localhost:8000");
    info!("OAuth endpoints:");
    info!("  - Google: http://localhost:8000/api/auth/google_callback");
    info!("  - Twitter: http://localhost:8000/api/auth/twitter_callback");

    axum::serve(listener, app).await.unwrap();

    Ok(())
}
