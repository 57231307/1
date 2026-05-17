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

#[derive(Debug, Clone, Serialize)] pub enum AppError {
    DatabaseError(String),
    ValidationError(String),
    NotFound(String),
    ResourceNotFound(String),
    BusinessError(String),
    Unauthorized(String),
    InternalError(String),
    BadRequest(String),
    PermissionDenied(String),
    TooManyRequests { retry_after: Option<u64>, message: String },
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
            AppError::TooManyRequests { message, .. } => write!(f, "{}", message),
        }
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_type, error_message) = match &self {
            AppError::DatabaseError(msg) => {
                tracing::error!("数据库错误: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DatabaseError",
                    msg.clone(),
                )
            }
            AppError::ValidationError(msg) => {
                tracing::warn!("验证错误: {}", msg);
                (
                    StatusCode::BAD_REQUEST,
                    "ValidationError",
                    "请求参数验证失败".to_string(),
                )
            }
            AppError::NotFound(msg) => {
                tracing::warn!("资源未找到: {}", msg);
                (StatusCode::NOT_FOUND, "NotFound", "未找到".to_string())
            }
            AppError::ResourceNotFound(msg) => {
                tracing::warn!("资源不存在: {}", msg);
                (StatusCode::NOT_FOUND, "ResourceNotFound", "资源不存在".to_string())
            }
            AppError::BusinessError(msg) => {
                tracing::warn!("业务错误: {}", msg);
                (StatusCode::BAD_REQUEST, "BusinessError", "业务错误".to_string())
            }
            AppError::Unauthorized(msg) => {
                tracing::warn!("未授权访问: {}", msg);
                (StatusCode::UNAUTHORIZED, "Unauthorized", "未授权访问".to_string())
            }
            AppError::InternalError(msg) => {
                tracing::error!("内部错误: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "InternalError",
                    "系统内部错误".to_string(),
                )
            }
            AppError::PermissionDenied(msg) => {
                tracing::warn!("权限不足: {}", msg);
                (
                    StatusCode::FORBIDDEN,
                    "PermissionDenied",
                    "权限不足".to_string(),
                )
            }
            AppError::BadRequest(msg) => {
                tracing::warn!("请求错误: {}", msg);
                (
                    StatusCode::BAD_REQUEST,
                    "BadRequest",
                    "请求错误".to_string(),
                )
            }
            AppError::TooManyRequests { retry_after, message } => {
                tracing::warn!("请求过多: {}", message);
                let retry_msg = if let Some(seconds) = retry_after {
                    format!("{}，请{}秒后再试", message, seconds)
                } else {
                    message.clone()
                };
                (
                    StatusCode::TOO_MANY_REQUESTS,
                    "TooManyRequests",
                    retry_msg,
                )
            }
        };

        // 返回统一的 ApiResponse 格式 {code, data, message}
        let body = serde_json::json!({
            "code": status.as_u16(),
            "data": null,
            "message": error_message
        });

        (status, Json(body)).into_response()
    }
}

impl From<sea_orm::DbErr> for AppError {
    fn from(err: sea_orm::DbErr) -> Self {
        let err_str = err.to_string();
        let error_kind = match &err {
            sea_orm::DbErr::Conn(_) => "数据库连接失败",
            sea_orm::DbErr::Exec(_) => {
                if err_str.contains("unique constraint") || err_str.contains("duplicate") {
                    "数据重复"
                } else if err_str.contains("foreign key constraint") || err_str.contains("references") {
                    "数据关联错误"
                } else {
                    "数据库执行错误"
                }
            }
            sea_orm::DbErr::Query(_) => {
                if err_str.contains("syntax error") {
                    "查询语法错误"
                } else {
                    "数据库查询错误"
                }
            }
            sea_orm::DbErr::RecordNotFound(_) => "记录不存在",
            sea_orm::DbErr::Custom(_) => {
                if err_str.contains("timeout") {
                    "数据库操作超时"
                } else {
                    "数据库自定义错误"
                }
            }
            sea_orm::DbErr::Type(msg) => {
                tracing::error!("数据库类型错误：{}", msg);
                "数据库类型错误"
            }
            sea_orm::DbErr::Json(msg) => {
                tracing::error!("数据库 JSON 错误：{}", msg);
                "数据库 JSON 处理错误"
            }
            sea_orm::DbErr::Migration(msg) => {
                tracing::error!("数据库迁移错误：{}", msg);
                "数据库迁移错误"
            }
            _ => "数据库操作失败",
        };

        tracing::error!("数据库错误 [{}]: {}", error_kind, err);
        AppError::DatabaseError(error_kind.to_string())
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

impl From<validator::ValidationErrors> for AppError {
    fn from(err: validator::ValidationErrors) -> Self {
        AppError::ValidationError(err.to_string())
    }
}
