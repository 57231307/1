use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::fmt;

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
    TooManyRequests {
        retry_after: Option<u64>,
        message: String,
    },
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
        let (status, error_type, error_message, log_detail) = match &self {
            AppError::DatabaseError(msg) => {
                let detail = serde_json::json!({
                    "error_type": "DatabaseError",
                    "message": msg,
                    "severity": "HIGH",
                    "action_required": "检查数据库连接和查询"
                });
                tracing::error!(
                    "【数据库错误】{} | 详情: {} | 建议: 检查数据库连接状态和 SQL 查询",
                    msg,
                    detail
                );
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DatabaseError",
                    msg.clone(),
                    detail,
                )
            }
            AppError::ValidationError(msg) => {
                let detail = serde_json::json!({
                    "error_type": "ValidationError",
                    "message": msg,
                    "severity": "LOW",
                    "action_required": "检查请求参数"
                });
                tracing::warn!(
                    "【验证错误】{} | 详情: {} | 建议: 检查请求参数格式和必填项",
                    msg,
                    detail
                );
                (
                    StatusCode::BAD_REQUEST,
                    "ValidationError",
                    "请求参数验证失败".to_string(),
                    detail,
                )
            }
            AppError::NotFound(msg) => {
                let detail = serde_json::json!({
                    "error_type": "NotFound",
                    "message": msg,
                    "severity": "MEDIUM",
                    "action_required": "检查资源是否存在"
                });
                tracing::warn!(
                    "【资源未找到】{} | 详情: {} | 建议: 检查资源 ID 是否正确或资源是否已被删除",
                    msg,
                    detail
                );
                (
                    StatusCode::NOT_FOUND,
                    "NotFound",
                    "未找到".to_string(),
                    detail,
                )
            }
            AppError::ResourceNotFound(msg) => {
                let detail = serde_json::json!({
                    "error_type": "ResourceNotFound",
                    "message": msg,
                    "severity": "MEDIUM",
                    "action_required": "检查资源是否存在"
                });
                tracing::warn!(
                    "【资源不存在】{} | 详情: {} | 建议: 检查资源 ID 是否正确或资源是否已被删除",
                    msg,
                    detail
                );
                (
                    StatusCode::NOT_FOUND,
                    "ResourceNotFound",
                    "资源不存在".to_string(),
                    detail,
                )
            }
            AppError::BusinessError(msg) => {
                let detail = serde_json::json!({
                    "error_type": "BusinessError",
                    "message": msg,
                    "severity": "MEDIUM",
                    "action_required": "检查业务规则"
                });
                tracing::warn!(
                    "【业务错误】{} | 详情: {} | 建议: 检查业务规则和前置条件",
                    msg,
                    detail
                );
                (
                    StatusCode::BAD_REQUEST,
                    "BusinessError",
                    "业务错误".to_string(),
                    detail,
                )
            }
            AppError::Unauthorized(msg) => {
                let detail = serde_json::json!({
                    "error_type": "Unauthorized",
                    "message": msg,
                    "severity": "HIGH",
                    "action_required": "检查认证信息"
                });
                tracing::warn!(
                    "【未授权访问】{} | 详情: {} | 建议: 检查 Token 是否有效或是否已过期",
                    msg,
                    detail
                );
                (
                    StatusCode::UNAUTHORIZED,
                    "Unauthorized",
                    "未授权访问".to_string(),
                    detail,
                )
            }
            AppError::InternalError(msg) => {
                let detail = serde_json::json!({
                    "error_type": "InternalError",
                    "message": msg,
                    "severity": "CRITICAL",
                    "action_required": "联系系统管理员"
                });
                tracing::error!(
                    "【内部错误】{} | 详情: {} | 建议: 检查系统日志或联系管理员",
                    msg,
                    detail
                );
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "InternalError",
                    "系统内部错误".to_string(),
                    detail,
                )
            }
            AppError::PermissionDenied(msg) => {
                let detail = serde_json::json!({
                    "error_type": "PermissionDenied",
                    "message": msg,
                    "severity": "HIGH",
                    "action_required": "检查用户权限"
                });
                tracing::warn!(
                    "【权限不足】{} | 详情: {} | 建议: 检查用户角色和权限配置",
                    msg,
                    detail
                );
                (
                    StatusCode::FORBIDDEN,
                    "PermissionDenied",
                    "权限不足".to_string(),
                    detail,
                )
            }
            AppError::BadRequest(msg) => {
                let detail = serde_json::json!({
                    "error_type": "BadRequest",
                    "message": msg,
                    "severity": "LOW",
                    "action_required": "检查请求格式"
                });
                tracing::warn!(
                    "【请求错误】{} | 详情: {} | 建议: 检查请求格式和参数",
                    msg,
                    detail
                );
                (
                    StatusCode::BAD_REQUEST,
                    "BadRequest",
                    "请求错误".to_string(),
                    detail,
                )
            }
            AppError::TooManyRequests {
                retry_after,
                message,
            } => {
                let detail = serde_json::json!({
                    "error_type": "TooManyRequests",
                    "message": message,
                    "retry_after": retry_after,
                    "severity": "MEDIUM",
                    "action_required": "稍后重试"
                });
                tracing::warn!(
                    "【请求过多】{} | 详情: {} | 建议: 等待 {:?} 秒后重试",
                    message,
                    detail,
                    retry_after
                );
                let retry_msg = if let Some(seconds) = retry_after {
                    format!("{}，请{}秒后再试", message, seconds)
                } else {
                    message.clone()
                };
                (
                    StatusCode::TOO_MANY_REQUESTS,
                    "TooManyRequests",
                    retry_msg,
                    detail,
                )
            }
        };

        // 返回统一的 ApiResponse 格式 {code, data, message}
        let body = serde_json::json!({
            "code": status.as_u16(),
            "data": null,
            "message": error_message,
            "error_type": error_type,
            "detail": log_detail
        });

        (status, Json(body)).into_response()
    }
}

impl From<sea_orm::DbErr> for AppError {
    fn from(err: sea_orm::DbErr) -> Self {
        let err_str = err.to_string();
        match &err {
            sea_orm::DbErr::Conn(_) => {
                tracing::error!("数据库连接失败：{}", err);
                AppError::DatabaseError("数据库连接失败".to_string())
            }
            sea_orm::DbErr::Exec(_) => {
                let error_kind =
                    if err_str.contains("unique constraint") || err_str.contains("duplicate") {
                        "数据重复"
                    } else if err_str.contains("foreign key constraint")
                        || err_str.contains("references")
                    {
                        "数据关联错误"
                    } else {
                        "数据库执行错误"
                    };
                tracing::error!("数据库执行错误 [{}]: {}", error_kind, err);
                AppError::DatabaseError(error_kind.to_string())
            }
            sea_orm::DbErr::Query(_) => {
                let error_kind = if err_str.contains("syntax error") {
                    "查询语法错误"
                } else {
                    "数据库查询错误"
                };
                tracing::error!("数据库查询错误 [{}]: {}", error_kind, err);
                AppError::DatabaseError(error_kind.to_string())
            }
            sea_orm::DbErr::RecordNotFound(msg) => {
                tracing::warn!("记录不存在：{}", msg);
                AppError::ResourceNotFound(msg.clone())
            }
            sea_orm::DbErr::Custom(_) => {
                let error_kind = if err_str.contains("timeout") {
                    "数据库操作超时"
                } else {
                    "数据库自定义错误"
                };
                tracing::error!("数据库自定义错误 [{}]: {}", error_kind, err);
                AppError::DatabaseError(error_kind.to_string())
            }
            sea_orm::DbErr::Type(msg) => {
                tracing::error!("数据库类型错误：{:?}", msg);
                AppError::DatabaseError(format!("数据库类型错误: {}", msg))
            }
            sea_orm::DbErr::Json(msg) => {
                tracing::error!("数据库 JSON 错误：{}", msg);
                AppError::DatabaseError("数据库 JSON 处理错误".to_string())
            }
            sea_orm::DbErr::Migration(msg) => {
                tracing::error!("数据库迁移错误：{}", msg);
                AppError::DatabaseError("数据库迁移错误".to_string())
            }
            _ => {
                tracing::error!("数据库操作失败：{}", err);
                AppError::DatabaseError("数据库操作失败".to_string())
            }
        }
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
