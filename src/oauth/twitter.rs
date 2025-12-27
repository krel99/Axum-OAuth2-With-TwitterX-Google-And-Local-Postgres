use oauth2::{basic::BasicClient, AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};
use serde::Deserialize;

pub fn build_twitter_oauth_client(client_id: String, client_secret: String) -> BasicClient {
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

#[derive(Debug, Deserialize)]
pub struct TwitterUserData {
    pub id: String,
    pub name: String,
    pub username: String,
}

#[derive(Debug, Deserialize)]
pub struct TwitterUserInfo {
    pub data: TwitterUserData,
}
