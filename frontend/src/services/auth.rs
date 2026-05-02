use crate::models::auth::{LoginRequest, LoginResponse};
use crate::services::api::ApiService;

/// 认证服务
/// 负责用户登录、注销和认证状态检查
#[derive(Clone)]
pub struct AuthService;

impl AuthService {
    /// 创建认证服务实例
    pub fn new() -> Self {
        Self
    }

    /// 用户登录
    /// 
    /// # 参数
    /// * `username` - 用户名
    /// * `password` - 密码
    /// 
    /// # 返回
    /// * `Ok(LoginResponse)` - 登录成功，返回包含 Token 的响应
    /// * `Err(String)` - 登录失败，返回错误信息
    pub async fn login(&self, username: &str, password: &str, totp_token: Option<String>) -> Result<LoginResponse, String> {
        let login_req = LoginRequest {
            username: username.to_string(),
            password: password.to_string(),
            totp_token,
        };

        let payload = serde_json::to_value(&login_req).map_err(|e| e.to_string())?;
        ApiService::post("/auth/login", &payload).await
    }

    /// 用户注销
    /// 调用后端接口并清除本地存储的 Token
    pub async fn logout(&self) -> Result<(), String> {
        let _ = ApiService::post::<serde_json::Value, serde_json::Value>("/auth/logout", &serde_json::json!({})).await;
        crate::utils::storage::Storage::remove_token();
        Ok(())
    }

    /// 刷新令牌
    pub async fn refresh_token(&self) -> Result<String, String> {
        #[derive(serde::Deserialize)]
        struct RefreshResponse {
            token: String,
            expires_in: u64,
        }
        let response: RefreshResponse = ApiService::post("/auth/refresh", &serde_json::json!({})).await?;
        Ok(response.token)
    }

    /// 检查用户是否已认证
    /// 
    /// # 返回
    /// * `true` - 已认证（存在有效 Token）
    /// * `false` - 未认证
    pub fn is_authenticated(&self) -> bool {
        crate::utils::storage::Storage::get_token().is_some()
    }

    pub async fn setup_totp(&self) -> Result<crate::models::auth::TotpSetupResponse, String> {
        ApiService::post("/auth/totp/setup", &serde_json::json!({})).await
    }

    pub async fn enable_totp(&self, token: &str) -> Result<(), String> {
        let _ = ApiService::post::<bool, serde_json::Value>("/auth/totp/enable", &serde_json::json!({ "token": token })).await?;
        Ok(())
    }
}
