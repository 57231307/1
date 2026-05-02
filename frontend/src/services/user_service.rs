use crate::models::user::{User, CreateUserRequest, UserListResponse};
use crate::services::api::ApiService;
use crate::services::crud_service::CrudService;
use serde::Serialize;

#[derive(Serialize)]
pub struct UserQuery {
    pub page: u64,
    pub page_size: u64,
}

/// 用户服务
/// 负责用户相关的 API 调用（CRUD 操作）
#[derive(Clone)]
pub struct UserService;

impl CrudService for UserService {
    type Model = User;
    type ListResponse = UserListResponse;
    type CreateRequest = CreateUserRequest;
    type UpdateRequest = serde_json::Value;

    fn base_path() -> &'static str {
        "/users"
    }
}

impl UserService {
    pub fn new() -> Self {
        Self
    }

    pub async fn change_password(id: i32, payload: &serde_json::Value) -> Result<(), String> {
        ApiService::post(&format!("/users/{}/password", id), payload).await
    }
}
