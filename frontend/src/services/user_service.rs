use gloo_net::http::Request;
use crate::models::user::{User, CreateUserRequest, UserListResponse};

/// 用户服务
/// 负责用户相关的 API 调用（CRUD 操作）
#[derive(Clone)]
pub struct UserService {
    api_base_url: String,
}

impl UserService {
    /// 创建用户服务实例
    /// API 基础路径：/api/v1/erp（根据项目规则统一接口路径）
    pub fn new() -> Self {
        Self {
            api_base_url: "/api/v1/erp".to_string(),
        }
    }

    /// 获取认证头
    /// 从本地存储获取 Token 并格式化为 Authorization 头
    fn get_auth_header() -> Option<String> {
        crate::utils::storage::Storage::get_token()
            .map(|token| format!("Bearer {}", token))
    }

    pub async fn list_users(&self, page: u64, page_size: u64) -> Result<UserListResponse, String> {
        let url = format!(
            "{}/users?page={}&page_size={}",
            self.api_base_url, page, page_size
        );

        let mut request = Request::get(&url);
        
        if let Some(token) = Self::get_auth_header() {
            request = request.header("Authorization", &token);
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("网络错误：{}", e))?;

        if response.ok() {
            let result: UserListResponse = response
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
            Err(format!("获取用户列表失败 ({}): {}", status, error_text))
        }
    }

    pub async fn get_user(&self, id: i32) -> Result<User, String> {
        let url = format!("{}/users/{}", self.api_base_url, id);

        let mut request = Request::get(&url);
        
        if let Some(token) = Self::get_auth_header() {
            request = request.header("Authorization", &token);
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("网络错误：{}", e))?;

        if response.ok() {
            let result: User = response
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
            Err(format!("获取用户详情失败 ({}): {}", status, error_text))
        }
    }

    pub async fn create_user(&self, user: &CreateUserRequest) -> Result<User, String> {
        let url = format!("{}/users", self.api_base_url);

        let mut request = Request::post(&url)
            .header("Content-Type", "application/json");
        
        if let Some(token) = Self::get_auth_header() {
            request = request.header("Authorization", &token);
        }

        let response = request
            .json(user)
            .map_err(|e| format!("序列化请求失败：{}", e))?
            .send()
            .await
            .map_err(|e| format!("网络错误：{}", e))?;

        if response.ok() {
            let result: User = response
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
            Err(format!("创建用户失败 ({}): {}", status, error_text))
        }
    }

    pub async fn delete_user(&self, id: i32) -> Result<(), String> {
        let url = format!("{}/users/{}", self.api_base_url, id);

        let mut request = Request::delete(&url);
        
        if let Some(token) = Self::get_auth_header() {
            request = request.header("Authorization", &token);
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("网络错误：{}", e))?;

        if response.ok() {
            Ok(())
        } else {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "未知错误".to_string());
            Err(format!("删除用户失败 ({}): {}", status, error_text))
        }
    }
}
