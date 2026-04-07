#![allow(dead_code, unused_variables, unused_imports, unused_mut)]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn into_result(self) -> Result<T, String> {
        if self.success {
            match self.data {
                Some(data) => Ok(data),
                None => Err("未返回任何数据".to_string()),
            }
        } else {
            Err(self.error.unwrap_or("未知错误".to_string()))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub items: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}
