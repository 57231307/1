use wasm_bindgen_futures::spawn_local;
use gloo_net::http::Request;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitRequest {
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

#[derive(Debug, thiserror::Error)]
pub enum InitError {
    #[error("系统已经初始化")]
    AlreadyInitialized,
    #[error("初始化失败: {0}")]
    InitializationFailed(String),
    #[error("网络错误: {0}")]
    NetworkError(String),
}

pub struct InitService;

impl InitService {
    pub fn new() -> Self {
        Self
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
            .map_err(|e| InitError::NetworkError(e.to_string()))
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
        
        if response.status() == 400 {
            let error: serde_json::Value = response.json().await.map_err(|e| InitError::NetworkError(e.to_string()))?;
            let message = error.get("message").and_then(|v| v.as_str()).unwrap_or("初始化失败");
            return Err(InitError::InitializationFailed(message.to_string()));
        }
        
        response
            .json::<InitResult>()
            .await
            .map_err(|e| InitError::NetworkError(e.to_string()))
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
            .map_err(|e| InitError::NetworkError(e.to_string()))
    }

    fn get_base_url() -> String {
        "http://129.204.17.232:8080/api/v1/erp".to_string()
    }
}
