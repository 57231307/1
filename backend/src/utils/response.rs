use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiResponse<T> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<u16>,
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl<T> Default for ApiResponse<T> {
    fn default() -> Self {
        Self {
            code: Some(500),
            data: None,
            message: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PaginatedResponse<T> {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

impl<T> Default for PaginatedResponse<T> {
    fn default() -> Self {
        Self {
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
            items: data,
            total,
            page,
            page_size,
        }
    }
}

// 将 PaginatedResponse 转换为 ApiResponse<Vec<T>>（保留兼容性）
// 注意：这会丢弃分页元数据，新代码应使用 ApiResponse::success(PaginatedResponse<T>)
impl<T> From<PaginatedResponse<T>> for ApiResponse<Vec<T>> {
    fn from(paginated: PaginatedResponse<T>) -> Self {
        ApiResponse {
            code: Some(200),
            data: Some(paginated.items),
            message: None,
        }
    }
}

// P2 2-10 修复：移除 PaginatedResponse<T> 的 IntoResponse 实现
// 原实现将 items 放入 data、total/page 拼接到 message，丢失结构化分页信息。
// 新代码必须使用 ApiResponse::success(PaginatedResponse<T>) 或 ApiResponse::success_paginated，
// 确保分页元数据（total/page/page_size）以结构化形式保留在 data 字段中。

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            code: Some(200),
            data: Some(data),
            message: None,
        }
    }

    pub fn success_paginated(
        data: Vec<T>,
        total: u64,
        page: u64,
        page_size: u64,
    ) -> ApiResponse<PaginatedResponse<T>> {
        // data 字段统一放置 PaginatedResponse 结构，便于前端直接消费分页元数据
        ApiResponse {
            code: Some(200),
            data: Some(PaginatedResponse {
                items: data,
                total,
                page,
                page_size,
            }),
            message: None,
        }
    }

    pub fn success_with_message(data: T, message: &str) -> Self {
        Self {
            code: Some(200),
            data: Some(data),
            message: Some(message.to_string()),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            code: Some(500),
            data: None,
            message: Some(message.into()),
        }
    }

    pub fn error_with_status(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            code: Some(status.as_u16()),
            data: None,
            message: Some(message.into()),
        }
    }
}

impl<T: Serialize> From<T> for ApiResponse<T> {
    fn from(data: T) -> Self {
        Self::success(data)
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        let status_code = StatusCode::from_u16(self.code.unwrap_or(200)).unwrap_or(StatusCode::OK);
        (status_code, Json(self)).into_response()
    }
}

pub fn unauthorized_response(message: &str) -> Response {
    let body = serde_json::json!({
        "code": 401,
        "message": message,
        "data": null
    });
    (StatusCode::UNAUTHORIZED, Json(body)).into_response()
}

pub fn forbidden_response(message: &str) -> Response {
    let body = serde_json::json!({
        "code": 403,
        "message": message,
        "data": null
    });
    (StatusCode::FORBIDDEN, Json(body)).into_response()
}
