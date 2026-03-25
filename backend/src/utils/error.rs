use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::fmt;

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub enum AppError {
    DatabaseError(String),
    ValidationError(String),
    NotFound(String),
    ResourceNotFound(String),
    BusinessError(String),
    Unauthorized(String),
    InternalError(String),
    BadRequest(String),
    PermissionDenied(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AppError::DatabaseError(msg) => write!(f, "数据库错误：{}", msg),
            AppError::ValidationError(msg) => write!(f, "验证错误：{}", msg),
            AppError::NotFound(msg) => write!(f, "未找到：{}", msg),
            AppError::ResourceNotFound(msg) => write!(f, "资源不存在：{}", msg),
            AppError::BusinessError(msg) => write!(f, "业务错误：{}", msg),
            AppError::Unauthorized(msg) => write!(f, "未授权：{}", msg),
            AppError::InternalError(msg) => write!(f, "内部错误：{}", msg),
            AppError::BadRequest(msg) => write!(f, "请求错误：{}", msg),
            AppError::PermissionDenied(msg) => write!(f, "权限不足：{}", msg),
        }
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_type, error_message) = match &self {
            AppError::DatabaseError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "DatabaseError", "数据库操作失败"),
            AppError::ValidationError(_) => (StatusCode::BAD_REQUEST, "ValidationError", "请求参数验证失败"),
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, "NotFound", "未找到"),
            AppError::ResourceNotFound(_) => (StatusCode::NOT_FOUND, "ResourceNotFound", "资源不存在"),
            AppError::BusinessError(_) => (StatusCode::BAD_REQUEST, "BusinessError", "业务错误"),
            AppError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "Unauthorized", "未授权访问"),
            AppError::InternalError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "InternalError", "服务器内部错误"),
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, "BadRequest", "请求错误"),
            AppError::PermissionDenied(_) => (StatusCode::FORBIDDEN, "PermissionDenied", "权限不足"),
        };

        let body = ErrorResponse {
            error: error_type.to_string(),
            message: error_message.to_string(),
        };

        (status, Json(body)).into_response()
    }
}

impl From<sea_orm::DbErr> for AppError {
    fn from(err: sea_orm::DbErr) -> Self {
        tracing::error!("数据库错误：{}", err);
        AppError::DatabaseError(err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::InternalError(format!("JSON 序列化错误：{}", err))
    }
}

impl From<(StatusCode, String)> for AppError {
    fn from((status, msg): (StatusCode, String)) -> Self {
        match status {
            StatusCode::NOT_FOUND => AppError::NotFound(msg),
            StatusCode::BAD_REQUEST => AppError::BadRequest(msg),
            StatusCode::UNAUTHORIZED => AppError::Unauthorized(msg),
            StatusCode::FORBIDDEN => AppError::PermissionDenied(msg),
            StatusCode::INTERNAL_SERVER_ERROR => AppError::InternalError(msg),
            _ => AppError::BadRequest(msg),
        }
    }
}
