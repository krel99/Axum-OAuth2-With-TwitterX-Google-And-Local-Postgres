use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TwitterUserData {
    #[allow(dead_code)]
    pub id: String,
    #[allow(dead_code)]
    pub name: String,
    pub username: String,
}

#[derive(Debug, Deserialize)]
pub struct TwitterUserInfo {
    pub data: TwitterUserData,
}
