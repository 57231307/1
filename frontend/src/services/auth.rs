use gloo_net::http::Request;
use crate::models::auth::{LoginRequest, LoginResponse};

/// 认证服务
/// 负责用户登录、注销和认证状态检查
#[derive(Clone)]
pub struct AuthService {
    api_base_url: String,
}

impl AuthService {
    /// 创建认证服务实例
    /// API 基础路径：/api/v1/erp（根据项目规则统一接口路径）
    pub fn new() -> Self {
        Self {
            api_base_url: "/api/v1/erp".to_string(),
        }
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
    pub async fn login(&self, username: &str, password: &str) -> Result<LoginResponse, String> {
        let login_req = LoginRequest {
            username: username.to_string(),
            password: password.to_string(),
        };

        let url = format!("{}/auth/login", self.api_base_url);
        
        let response = Request::post(&url)
            .header("Content-Type", "application/json")
            .json(&login_req)
            .map_err(|e| format!("请求失败：{}", e))?
            .send()
            .await
            .map_err(|e| format!("网络错误：{}", e))?;

        if response.ok() {
            let result: LoginResponse = response
                .json()
                .await
                .map_err(|e| format!("解析响应失败：{}", e))?;
            Ok(result)
        } else {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "未知错误".to_string());
            Err(format!("登录失败 ({}): {}", status, error_text))
        }
    }

    /// 用户注销
    /// 清除本地存储的 Token
    pub fn logout(&self) {
        crate::utils::storage::Storage::remove_token();
    }

    /// 检查用户是否已认证
    /// 
    /// # 返回
    /// * `true` - 已认证（存在有效 Token）
    /// * `false` - 未认证
    pub fn is_authenticated(&self) -> bool {
        crate::utils::storage::Storage::get_token().is_some()
    }
}
