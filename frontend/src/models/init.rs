//! 系统初始化模型
//!
//! 系统初始化相关的数据结构

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
