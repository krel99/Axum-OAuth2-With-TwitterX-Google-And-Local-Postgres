use axum::{middleware, routing::get, Extension, Router};
use tower_http::{cors::CorsLayer, services::ServeDir};

use crate::handlers::{
    get_profile, google_callback, health_check, homepage, login_page, protected, twitter_callback,
    twitter_login,
};
use crate::middleware::check_authenticated;
use crate::oauth::{ClientIds, OAuthClients, PkceVerifiers};
use crate::services::logout;
use crate::state::AppState;

pub fn init_router(
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
