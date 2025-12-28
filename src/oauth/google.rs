use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GoogleUserInfo {
    pub email: String,
    #[allow(dead_code)]
    pub name: Option<String>,
    #[allow(dead_code)]
    pub picture: Option<String>,
}
