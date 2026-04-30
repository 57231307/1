use crate::models::user::{CreateUserRequest, User, UserListResponse};
use crate::services::api::ApiService;

/// 用户服务
/// 负责用户相关的 API 调用（CRUD 操作）
#[derive(Clone)]
pub struct UserService;

impl UserService {
    /// 创建用户服务实例
    pub fn new() -> Self {
        Self
    }

    pub async fn list_users(&self, page: u64, page_size: u64) -> Result<UserListResponse, String> {
        let url = format!("/users?page={}&page_size={}", page, page_size);
        ApiService::get(&url).await
    }

    pub async fn get_user(&self, id: i32) -> Result<User, String> {
        ApiService::get(&format!("/users/{}", id)).await
    }

    pub async fn create_user(&self, user: &CreateUserRequest) -> Result<User, String> {
        let payload = serde_json::to_value(user).map_err(|e| e.to_string())?;
        ApiService::post("/users", &payload).await
    }

    pub async fn update_user(&self, id: i32, user: &serde_json::Value) -> Result<User, String> {
        ApiService::put(&format!("/users/{}", id), user).await
    }

    pub async fn change_password(
        &self,
        id: i32,
        payload: &serde_json::Value,
    ) -> Result<(), String> {
        ApiService::post(&format!("/users/{}/password", id), payload).await
    }

    pub async fn delete_user(&self, id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/users/{}", id)).await
    }
}
