//! 系统初始化服务

use gloo_net::http::Request;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: String,
    pub name: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitRequest {
    pub admin_username: String,
    pub admin_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitWithDbRequest {
    pub db_config: DatabaseConfig,
    pub admin_username: String,
    pub admin_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitStatus {
    pub initialized: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitResult {
    pub success: bool,
    pub message: String,
    pub admin_username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetPasswordRequest {
    pub username: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetPasswordResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbTestRequest {
    pub host: String,
    pub port: String,
    pub name: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbTestResult {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum InitError {
    NetworkError(String),
    ServerError(String),
    ParseError(String),
}

impl std::fmt::Display for InitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InitError::NetworkError(msg) => write!(f, "网络错误: {}", msg),
            InitError::ServerError(msg) => write!(f, "服务器错误: {}", msg),
            InitError::ParseError(msg) => write!(f, "解析错误: {}", msg),
        }
    }
}

pub struct InitService;

impl InitService {
    fn get_base_url() -> String {
        super::api::API_BASE.to_string()
    }

    pub async fn check_status() -> Result<InitStatus, InitError> {
        let base_url = Self::get_base_url();
        let url = format!("{}/init/status", base_url);

        Request::get(&url)
            .send()
            .await
            .map_err(|e| InitError::NetworkError(e.to_string()))?
            .json::<InitStatus>()
            .await
            .map_err(|e| InitError::ParseError(e.to_string()))
    }

    pub async fn test_database(config: &DatabaseConfig) -> Result<DbTestResult, InitError> {
        let base_url = Self::get_base_url();
        let url = format!("{}/init/test-database", base_url);

        let request_body = DbTestRequest {
            host: config.host.clone(),
            port: config.port.clone(),
            name: config.name.clone(),
            username: config.username.clone(),
            password: config.password.clone(),
        };

        Request::post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .map_err(|e| InitError::NetworkError(e.to_string()))?
            .send()
            .await
            .map_err(|e| InitError::NetworkError(e.to_string()))?
            .json::<DbTestResult>()
            .await
            .map_err(|e| InitError::ParseError(e.to_string()))
    }

    pub async fn initialize(admin_username: &str, admin_password: &str) -> Result<InitResult, InitError> {
        let base_url = Self::get_base_url();
        let url = format!("{}/init/initialize", base_url);

        let request_body = InitRequest {
            admin_username: admin_username.to_string(),
            admin_password: admin_password.to_string(),
        };

        Request::post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .map_err(|e| InitError::NetworkError(e.to_string()))?
            .send()
            .await
            .map_err(|e| InitError::NetworkError(e.to_string()))?
            .json::<InitResult>()
            .await
            .map_err(|e| InitError::ParseError(e.to_string()))
    }

    pub async fn initialize_with_db(
        db_config: &DatabaseConfig,
        admin_username: &str,
        admin_password: &str,
    ) -> Result<InitResult, InitError> {
        let base_url = Self::get_base_url();
        let url = format!("{}/init/initialize-with-db", base_url);

        let request_body = InitWithDbRequest {
            db_config: db_config.clone(),
            admin_username: admin_username.to_string(),
            admin_password: admin_password.to_string(),
        };

        Request::post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .map_err(|e| InitError::NetworkError(e.to_string()))?
            .send()
            .await
            .map_err(|e| InitError::NetworkError(e.to_string()))?
            .json::<InitResult>()
            .await
            .map_err(|e| InitError::ParseError(e.to_string()))
    }

    pub async fn reset_password(username: &str, new_password: &str) -> Result<ResetPasswordResponse, InitError> {
        let base_url = Self::get_base_url();
        let url = format!("{}/init/reset-password", base_url);

        let request_body = ResetPasswordRequest {
            username: username.to_string(),
            new_password: new_password.to_string(),
        };

        Request::post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .map_err(|e| InitError::NetworkError(e.to_string()))?
            .send()
            .await
            .map_err(|e| InitError::NetworkError(e.to_string()))?
            .json::<ResetPasswordResponse>()
            .await
            .map_err(|e| InitError::ParseError(e.to_string()))
    }
}
