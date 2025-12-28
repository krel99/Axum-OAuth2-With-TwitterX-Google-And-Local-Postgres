use oauth2::basic::BasicClient;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct OAuthClients {
    pub google: BasicClient,
    pub twitter: BasicClient,
}

#[derive(Clone)]
pub struct ClientIds {
    pub google: String,
    #[allow(dead_code)]
    pub twitter: String,
}

// Store PKCE verifiers for Twitter
pub type PkceVerifiers = Arc<tokio::sync::Mutex<HashMap<String, String>>>;

#[derive(Debug, Deserialize)]
pub struct AuthRequest {
    pub code: String,
    #[allow(dead_code)]
    pub state: Option<String>,
}
