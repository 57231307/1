#![allow(dead_code, unused_imports, unused_variables)]
//! 数据传输对象（DTO）模块
//!
//! 提供通用的 API 响应和分页请求结构

pub mod bpm_dto;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

/// API 统一响应结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// 状态码，0 表示成功
    pub code: i32,
    /// 响应消息
    pub message: String,
    /// 响应数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T> Default for ApiResponse<T> {
    fn default() -> Self {
        Self {
            code: 0,
            message: String::new(),
            data: None,
        }
    }
}

impl<T> ApiResponse<T> {
    /// 创建成功响应
    pub fn success(data: T) -> Self {
        Self {
            code: 0,
            message: "操作成功".to_string(),
            data: Some(data),
        }
    }

    /// 创建成功响应（无数据）
    #[allow(dead_code)]
    pub fn success_no_data() -> Self {
        Self {
            code: 0,
            message: "操作成功".to_string(),
            data: None,
        }
    }

    /// 创建带消息的成功响应
    pub fn success_with_message(data: T, message: &str) -> Self {
        Self {
            code: 0,
            message: message.to_string(),
            data: Some(data),
        }
    }

    /// 创建带消息的成功响应（别名）
    pub fn success_with_msg(data: T, message: &str) -> Self {
        Self {
            code: 0,
            message: message.to_string(),
            data: Some(data),
        }
    }

    /// 创建错误响应
    pub fn error(message: &str) -> Self {
        Self {
            code: -1,
            message: message.to_string(),
            data: None,
        }
    }

    /// 创建带错误码的响应
    #[allow(dead_code)]
    pub fn error_with_code(code: i32, message: &str) -> Self {
        Self {
            code,
            message: message.to_string(),
            data: None,
        }
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        let status = if self.code == 0 {
            StatusCode::OK
        } else {
            StatusCode::BAD_REQUEST
        };
        (status, Json(self)).into_response()
    }
}

/// 分页请求参数
#[derive(Debug, Clone, Deserialize)]
pub struct PageRequest {
    /// 页码（从 1 开始）
    pub page: u64,
    /// 每页数量
    pub page_size: u64,
}

impl Default for PageRequest {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 10,
        }
    }
}

impl PageRequest {
    /// 创建分页请求
    #[allow(dead_code)]
    pub fn new(page: u64, page_size: u64) -> Self {
        Self { page, page_size }
    }

    /// 获取偏移量
    #[allow(dead_code)]
    pub fn offset(&self) -> u64 {
        (self.page.saturating_sub(1)) * self.page_size
    }

    /// 获取每页数量（限制最大 100）
    #[allow(dead_code)]
    pub fn limit(&self) -> u64 {
        self.page_size.min(100)
    }
}

/// 分页响应结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageResponse<T> {
    /// 总记录数
    pub total: u64,
    /// 当前页码
    pub page: u64,
    /// 每页数量
    pub page_size: u64,
    /// 总页数
    pub total_pages: u64,
    /// 数据列表
    pub data: Vec<T>,
}

impl<T> PageResponse<T> {
    /// 创建分页响应
    #[allow(dead_code)]
    pub fn new(data: Vec<T>, total: u64, page: u64, page_size: u64) -> Self {
        let total_pages = if total == 0 {
            0
        } else {
            total.div_ceil(page_size)
        };
        Self {
            total,
            page,
            page_size,
            total_pages,
            data,
        }
    }
}
