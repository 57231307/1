use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
    pub error: Option<String>,
    #[serde(skip)]
    pub status_code: Option<StatusCode>,
}

impl<T> Default for ApiResponse<T> {
    fn default() -> Self {
        Self {
            success: false,
            data: None,
            message: None,
            error: None,
            status_code: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<T>,  // 可选字段，兼容旧代码
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

impl<T> Default for PaginatedResponse<T> {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            items: Vec::new(),
            total: 0,
            page: 1,
            page_size: 10,
        }
    }
}

impl<T: Clone> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, total: u64, page: u64, page_size: u64) -> Self {
        Self {
            data: data.clone(),
            items: data,
            total,
            page,
            page_size,
        }
    }

    #[allow(dead_code)]
    pub fn from_data(data: Vec<T>, total: u64, page: u64, page_size: u64) -> Self {
        Self {
            data: data.clone(),
            items: data,
            total,
            page,
            page_size,
        }
    }
}

// 为 PaginatedResponse 实现 Into<ApiResponse<Vec<T>>> 用于列表响应
impl<T> From<PaginatedResponse<T>> for ApiResponse<Vec<T>> {
    fn from(paginated: PaginatedResponse<T>) -> Self {
        ApiResponse {
            success: true,
            data: Some(paginated.data),
            message: None,
            error: None,
            status_code: None,
        }
    }
}

// 实现 ApiResponse<PaginatedResponse<T>> 到 ApiResponse<Vec<T>> 的转换
impl<T: Clone> From<ApiResponse<PaginatedResponse<T>>> for ApiResponse<Vec<T>> {
    fn from(response: ApiResponse<PaginatedResponse<T>>) -> Self {
        ApiResponse {
            success: response.success,
            data: response.data.map(|p| p.data),
            message: response.message,
            error: response.error,
            status_code: response.status_code,
        }
    }
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
            error: None,
            status_code: None,
        }
    }

    #[allow(dead_code)]
    /// 创建分页成功响应
    pub fn success_paginated(data: Vec<T>, total: u64, page: u64, page_size: u64) -> ApiResponse<Vec<T>> {
        ApiResponse {
            success: true,
            data: Some(data),
            message: Some(format!("共 {} 条记录，第 {}/{} 页", total, page, page_size)),
            error: None,
            status_code: None,
        }
    }

    #[allow(dead_code)]
    pub fn success_opt(data: T, message: Option<String>) -> Self {
        Self {
            success: true,
            data: Some(data),
            message,
            error: None,
            status_code: None,
        }
    }

    #[allow(dead_code)]
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
            error: None,
            status_code: None,
        }
    }

    pub fn success_with_message(data: T, message: &str) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: Some(message.to_string()),
            error: None,
            status_code: None,
        }
    }

    #[allow(dead_code)]
    pub fn success_with_msg(data: T, message: &str) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: Some(message.to_string()),
            error: None,
            status_code: None,
        }
    }

    #[allow(dead_code)]
    pub fn success_with_data(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
            error: None,
            status_code: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            message: None,
            error: Some(message.into()),
            status_code: None,
        }
    }

    #[allow(dead_code)]
    pub fn error_msg(message: &str) -> Self {
        Self {
            success: false,
            data: None,
            message: None,
            error: Some(message.to_string()),
            status_code: None,
        }
    }

    #[allow(dead_code)]
    pub fn error_with_status(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            message: None,
            error: Some(message.into()),
            status_code: Some(status),
        }
    }

    #[allow(dead_code)]
    /// 创建错误响应（泛型版本，用于与成功响应配对）
    pub fn error_response(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            message: None,
            error: Some(message.into()),
            status_code: None,
        }
    }
}

// 为 PaginatedResponse<T> 实现 IntoResponse
impl<T: Serialize> IntoResponse for PaginatedResponse<T> {
    fn into_response(self) -> Response {
        ApiResponse {
            success: true,
            data: Some(self.data),
            message: Some(format!("共 {} 条记录，第 {}/{} 页", self.total, self.page, self.page_size)),
            error: None,
            status_code: None,
        }.into_response()
    }
}

impl<T: Serialize> From<T> for ApiResponse<T> {
    fn from(data: T) -> Self {
        Self::success(data)
    }
}

// 实现 IntoResponse trait，让 ApiResponse 可以直接作为返回值
impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        let status_code = self.status_code.unwrap_or_else(|| {
            if self.success {
                StatusCode::OK
            } else {
                StatusCode::BAD_REQUEST
            }
        });
        (status_code, Json(self)).into_response()
    }
}
