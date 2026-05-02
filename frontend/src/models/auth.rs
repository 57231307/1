use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub totp_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: i32,
    pub username: String,
    pub email: Option<String>,
    pub role_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotpSetupResponse {
    pub secret: String,
    pub qr_code: String,
}
