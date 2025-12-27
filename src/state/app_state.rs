use axum::extract::FromRef;
use axum_extra::extract::cookie::Key;
use reqwest::Client as ReqwestClient;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub ctx: ReqwestClient,
    pub key: Key, // TODO may want to make this private; add handler
}

impl FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.key.clone()
    }
}
