use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub role_id: Option<i32>,
    pub department_id: Option<i32>,
    pub is_active: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub role_id: Option<i32>,
    pub department_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserListResponse {
    pub users: Vec<User>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}
