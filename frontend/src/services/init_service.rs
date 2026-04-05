//! 系统初始化服务

use crate::models::init::{
    DatabaseConfig, DbTestRequest, DbTestResult, InitRequest, InitResult, InitStatus,
    InitWithDbRequest, ResetPasswordRequest, ResetPasswordResponse,
};
use gloo_net::http::Request;

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
        String::from(crate::services::api::API_BASE)
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

        let response = Request::post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .map_err(|e| InitError::NetworkError(e.to_string()))?
            .send()
            .await
            .map_err(|e| InitError::NetworkError(e.to_string()))?;

        if !response.ok() {
            if let Ok(err_json) = response.json::<serde_json::Value>().await {
                if let Some(msg) = err_json.get("message").and_then(|m| m.as_str()) {
                    return Err(InitError::ServerError(msg.to_string()));
                }
            }
            return Err(InitError::ServerError(format!("请求失败，状态码: {}", response.status())));
        }

        response.json::<InitResult>()
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

        let response = Request::post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .map_err(|e| InitError::NetworkError(e.to_string()))?
            .send()
            .await
            .map_err(|e| InitError::NetworkError(e.to_string()))?;

        if !response.ok() {
            if let Ok(err_json) = response.json::<serde_json::Value>().await {
                if let Some(msg) = err_json.get("message").and_then(|m| m.as_str()) {
                    return Err(InitError::ServerError(msg.to_string()));
                }
            }
            return Err(InitError::ServerError(format!("请求失败，状态码: {}", response.status())));
        }

        response.json::<InitResult>()
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
