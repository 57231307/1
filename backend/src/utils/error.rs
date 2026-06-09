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
    BusinessError(String),
    Unauthorized(String),
    InternalError(String),
    BadRequest(String),
    PermissionDenied(String),
    NotImplemented(String),
    TooManyRequests {
        retry_after: Option<u64>,
        message: String,
    },
}

impl AppError {
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }
    pub fn business(msg: impl Into<String>) -> Self {
        Self::BusinessError(msg.into())
    }
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::ValidationError(msg.into())
    }
    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self::Unauthorized(msg.into())
    }
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::InternalError(msg.into())
    }
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self::BadRequest(msg.into())
    }
    pub fn permission_denied(msg: impl Into<String>) -> Self {
        Self::PermissionDenied(msg.into())
    }
    pub fn database(msg: impl Into<String>) -> Self {
        Self::DatabaseError(msg.into())
    }
    pub fn not_implemented(msg: impl Into<String>) -> Self {
        Self::NotImplemented(msg.into())
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AppError::DatabaseError(msg) => write!(f, "数据库错误：{}", msg),
            AppError::ValidationError(msg) => write!(f, "验证错误：{}", msg),
            AppError::NotFound(msg) => write!(f, "未找到：{}", msg),
            AppError::BusinessError(msg) => write!(f, "业务错误：{}", msg),
            AppError::Unauthorized(msg) => write!(f, "未授权：{}", msg),
            AppError::InternalError(msg) => write!(f, "内部错误：{}", msg),
            AppError::BadRequest(msg) => write!(f, "请求错误：{}", msg),
            AppError::PermissionDenied(msg) => write!(f, "权限不足：{}", msg),
            AppError::NotImplemented(msg) => write!(f, "未实现：{}", msg),
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
                (StatusCode::BAD_REQUEST, "BadRequest", msg.clone(), detail)
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
            AppError::NotImplemented(msg) => {
                let detail = serde_json::json!({
                    "error_type": "NotImplemented",
                    "message": msg,
                    "severity": "MEDIUM",
                    "action_required": "联系开发团队实现该功能"
                });
                tracing::warn!(
                    "【功能未实现】{} | 详情: {} | 建议: 该功能正在开发中",
                    msg,
                    detail
                );
                (
                    StatusCode::NOT_IMPLEMENTED,
                    "NotImplemented",
                    "功能未实现".to_string(),
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
                AppError::database("数据库连接失败")
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
                AppError::database(error_kind.to_string())
            }
            sea_orm::DbErr::Query(_) => {
                let error_kind = if err_str.contains("syntax error") {
                    "查询语法错误"
                } else {
                    "数据库查询错误"
                };
                tracing::error!("数据库查询错误 [{}]: {}", error_kind, err);
                AppError::database(error_kind.to_string())
            }
            sea_orm::DbErr::RecordNotFound(msg) => {
                tracing::warn!("记录不存在：{}", msg);
                AppError::not_found(msg.clone())
            }
            sea_orm::DbErr::Custom(_) => {
                let error_kind = if err_str.contains("timeout") {
                    "数据库操作超时"
                } else {
                    "数据库自定义错误"
                };
                tracing::error!("数据库自定义错误 [{}]: {}", error_kind, err);
                AppError::database(error_kind.to_string())
            }
            sea_orm::DbErr::Type(msg) => {
                tracing::error!("数据库类型错误：{:?}", msg);
                AppError::database(format!("数据库类型错误: {}", msg))
            }
            sea_orm::DbErr::Json(msg) => {
                tracing::error!("数据库 JSON 错误：{}", msg);
                AppError::database("数据库 JSON 处理错误")
            }
            sea_orm::DbErr::Migration(msg) => {
                tracing::error!("数据库迁移错误：{}", msg);
                AppError::database("数据库迁移错误")
            }
            _ => {
                tracing::error!("数据库操作失败：{}", err);
                AppError::database("数据库操作失败")
            }
        }
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::internal(format!("JSON 序列化错误：{}", err))
    }
}

impl From<(StatusCode, String)> for AppError {
    fn from((status, msg): (StatusCode, String)) -> Self {
        match status {
            StatusCode::NOT_FOUND => AppError::not_found(msg),
            StatusCode::BAD_REQUEST => AppError::bad_request(msg),
            StatusCode::UNAUTHORIZED => AppError::unauthorized(msg),
            StatusCode::FORBIDDEN => AppError::permission_denied(msg),
            StatusCode::INTERNAL_SERVER_ERROR => AppError::internal(msg),
            _ => AppError::bad_request(msg),
        }
    }
}

impl From<validator::ValidationErrors> for AppError {
    fn from(err: validator::ValidationErrors) -> Self {
        AppError::validation(err.to_string())
    }
}

// ============================================================================
// 后端安全增强：错误响应统一化 & 生产环境脱敏
// ----------------------------------------------------------------------------
// 本段仅在文件末尾追加，不修改现有 AppError / Display / IntoResponse / From
// 实现，确保对外 API 完全向后兼容。
// ============================================================================

use chrono::Utc;
use uuid::Uuid;

/// 对外暴露的统一错误响应体
///
/// 字段说明：
/// - `code`      业务错误码（字符串枚举，便于多端/多语言统一处理）
/// - `message`   错误消息：开发环境保留 `Display` 详细描述；生产环境脱敏为通用文案
/// - `trace_id`  本次请求的链路追踪 ID，可用于服务端日志关联
/// - `timestamp` 错误发生时的 Unix 时间戳（秒）
#[derive(Debug, Clone, Serialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
    pub trace_id: String,
    pub timestamp: i64,
}

/// 为已有 `AppError` 追加响应序列化能力（不修改任何现有方法）
impl AppError {
    /// 转换为对外统一的 [`ErrorResponse`]
    ///
    /// 行为：
    /// - `cfg!(debug_assertions)` 为 true（即 `cargo run` / `cargo test`）→ 返回 `Display` 详细描述
    /// - release 构建 → 返回脱敏的通用文案，敏感信息（SQL 片段、内部堆栈等）不再外泄
    pub fn to_response(&self) -> ErrorResponse {
        let trace_id = Uuid::new_v4().to_string();
        let timestamp = Utc::now().timestamp();

        let code = self.error_code();
        let message = if cfg!(debug_assertions) {
            // 开发环境：暴露 Display 的完整内容，便于排查
            self.to_string()
        } else {
            // 生产环境：脱敏为通用文案
            self.public_message()
        };

        ErrorResponse {
            code,
            message,
            trace_id,
            timestamp,
        }
    }

    /// 业务错误码（稳定的字符串枚举）
    pub fn error_code(&self) -> String {
        match self {
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::BadRequest(_) => "BAD_REQUEST",
            AppError::Unauthorized(_) => "UNAUTHORIZED",
            AppError::PermissionDenied(_) => "FORBIDDEN",
            AppError::ValidationError(_) => "VALIDATION_ERROR",
            AppError::BusinessError(_) => "BUSINESS_ERROR",
            AppError::DatabaseError(_) => "DATABASE_ERROR",
            AppError::InternalError(_) => "INTERNAL_ERROR",
            AppError::NotImplemented(_) => "NOT_IMPLEMENTED",
            AppError::TooManyRequests { .. } => "TOO_MANY_REQUESTS",
        }
        .to_string()
    }

    /// 生产环境对外暴露的脱敏文案
    fn public_message(&self) -> String {
        match self {
            AppError::DatabaseError(_) => "数据库错误".to_string(),
            AppError::ValidationError(_) => "请求参数验证失败".to_string(),
            AppError::NotFound(_) => "资源未找到".to_string(),
            AppError::BusinessError(_) => "业务处理失败".to_string(),
            AppError::Unauthorized(_) => "未授权".to_string(),
            AppError::InternalError(_) => "服务器内部错误".to_string(),
            AppError::BadRequest(_) => "请求参数错误".to_string(),
            AppError::PermissionDenied(_) => "无权限".to_string(),
            AppError::NotImplemented(_) => "功能未实现".to_string(),
            AppError::TooManyRequests { .. } => "请求过于频繁，请稍后重试".to_string(),
        }
    }
}
