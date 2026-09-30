//! 统一错误类型 `AppError` 与 HTTP 出参脱敏机制。
//!
//! # 出参文案规则（白名单式外显，默认脱敏）
//!
//! 所有错误经 [`IntoResponse`] 出参时，`message` 字段默认是**固定脱敏常量**
//! （如 `AppError::business` → "业务处理失败"），真实文案只进 tracing 日志。
//!
//! 唯一例外：构造点显式使用 [`AppError::business_displayable`]（新变体
//! `BusinessErrorDisplayable`）或 [`AppError::validation_displayable`]（变体
//! `ValidationErrorDisplayable`）时，出参 `message` 携带真实文案。
//! **不存在任何全局开关/环境变量**，是否外显由每个构造点逐条显式声明。
//!
//! ## 输入校验文案的外显规则（`ValidationError` 族）
//!
//! `validator` 派生校验的文案全部由本仓库自行撰写
//! （如「产品名称长度不能超过200个字符」「金额必须为正且不超过10亿」），描述的对象
//! **只是用户自己刚提交的字段**，不含他人数据与内部标识，属于必须告知用户的内容。
//! 因此 [`From<validator::ValidationErrors>`] 一律转成
//! `ValidationErrorDisplayable` 并只提取可读文案（`HTTP 400 / code=VALIDATION_ERROR`
//! 与脱敏形态完全一致，仅 message 从固定常量换成真实原因）。
//! 手工构造时同理：文案只涉及请求字段就用 [`AppError::validation_displayable`]，
//! 携带第三方库错误原文（表名/SQL/堆栈/内部路径）才保留脱敏的
//! [`AppError::validation`]。
//!
//! ## 何时允许 `business_displayable`（安全边界，硬性规则）
//!
//! 仅当文案**只涉及当前用户自己提交的输入或本应告知用户的业务规则**，
//! 且不包含以下任何内容时才允许外显：
//! - 权限判定依据（角色/权限比对结果、管理员身份校验逻辑）
//! - 其它用户/其它主体的数据（他人姓名、库存数量、余额、未付金额等）
//! - 内部标识（数据库表名、记录 ID、内部编码、文件路径）
//! - SQL / 堆栈 / 任何系统实现细节
//!
//! 不确定的一律用 [`AppError::business`]（脱敏）。禁止通过拼 format! 把
//! 查询到的实体数据塞进可外显文案。
//!
//! 允许的例子：
//! 1. "二级审批人不能与一级审批人相同"（双方均为操作者自己提交的审批身份约束）
//! 2. "device_id 不能为空"（用户请求体字段校验）
//! 3. "只有敏感角色变更需要审批"（公开业务规则，不泄露权限判定细节）
//!
//! 禁止的例子：
//! 1. "当前库存 12.5 米，不足出库"（含库存数量）
//! 2. "业务模式 42 不存在"（含内部记录 ID）
//! 3. "unique constraint failed: users.username"（含表名/SQL 细节）

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use std::fmt;

use crate::utils::messages::err_msg;

#[derive(Debug, Clone, Serialize)]
pub enum AppError {
    DatabaseError(String),
    ValidationError(String),
    /// 可外显的输入校验错误：仅允许由 [`AppError::validation_displayable`] 或
    /// [`From<validator::ValidationErrors>`] 构造。出参 `code`/`status` 与
    /// `ValidationError` 完全一致（`VALIDATION_ERROR` / 400），唯一区别是 HTTP
    /// 响应的 `message` 携带真实拒绝原因。使用条件见模块文档。
    ValidationErrorDisplayable(String),
    NotFound(String),
    BusinessError(String),
    /// 可外显的业务错误：仅允许由 [`AppError::business_displayable`] 构造。
    /// 出参 `code` 与 `BusinessError` 完全一致（`BUSINESS_ERROR`），
    /// 唯一区别是 HTTP 响应的 `message` 携带真实文案。
    /// 使用条件见模块文档的安全边界；默认请始终使用 [`AppError::business`]。
    BusinessErrorDisplayable(String),
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
    /// 构造**可外显**的业务错误：HTTP 出参 `message` 携带真实文案。
    ///
    /// 仅当文案满足模块文档的安全边界（只涉及用户自己提交的数据/公开业务规则，
    /// 不含库存数量、余额、他人数据、内部 ID、表名、SQL、权限判定依据）时才允许使用；
    /// 不满足就改用 [`AppError::business`]（默认脱敏）。
    pub fn business_displayable(msg: impl Into<String>) -> Self {
        Self::BusinessErrorDisplayable(msg.into())
    }
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::ValidationError(msg.into())
    }
    /// 构造**可外显**的输入校验错误：HTTP 出参 `message` 携带真实拒绝原因。
    ///
    /// 仅当文案只描述**用户自己提交的字段**违反的公开规则（长度/范围/必填/格式）
    /// 时使用；若文案携带第三方库错误原文（可能含表名、SQL、内部路径、堆栈），
    /// 保留脱敏的 [`AppError::validation`]。
    pub fn validation_displayable(msg: impl Into<String>) -> Self {
        Self::ValidationErrorDisplayable(msg.into())
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
    pub fn too_many_requests(msg: impl Into<String>) -> Self {
        Self::TooManyRequests {
            retry_after: None,
            message: msg.into(),
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AppError::DatabaseError(msg) => write!(f, "{}{}", err_msg::DB_ERROR_PREFIX, msg),
            AppError::ValidationError(msg) => write!(f, "{}{}", err_msg::VALIDATION_PREFIX, msg),
            AppError::ValidationErrorDisplayable(msg) => {
                write!(f, "{}{}", err_msg::VALIDATION_PREFIX, msg)
            }
            AppError::NotFound(msg) => write!(f, "{}{}", err_msg::NOT_FOUND_PREFIX, msg),
            AppError::BusinessError(msg) => write!(f, "{}{}", err_msg::BUSINESS_PREFIX, msg),
            AppError::BusinessErrorDisplayable(msg) => {
                write!(f, "{}{}", err_msg::BUSINESS_PREFIX, msg)
            }
            AppError::Unauthorized(msg) => write!(f, "{}{}", err_msg::UNAUTHORIZED_PREFIX, msg),
            AppError::InternalError(msg) => write!(f, "{}{}", err_msg::INTERNAL_PREFIX, msg),
            AppError::BadRequest(msg) => write!(f, "{}{}", err_msg::BAD_REQUEST_PREFIX, msg),
            AppError::PermissionDenied(msg) => write!(f, "{}{}", err_msg::PERMISSION_PREFIX, msg),
            AppError::NotImplemented(msg) => {
                write!(f, "{}{}", err_msg::NOT_IMPLEMENTED_PREFIX, msg)
            }
            AppError::TooManyRequests { message, .. } => write!(f, "{}", message),
        }
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // 漏洞 #4/#8/#12 修复：detail 仅用于 tracing 日志，HTTP 响应仅含脱敏 code/message
        let (status, error_type) = self.error_status_and_type();
        let (severity, action_required) = self.error_severity_and_action();
        let detail = self.build_detail(error_type, severity, action_required);
        self.log_error(&detail);
        let trace_id = uuid::Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().timestamp();
        let body = serde_json::json!({
            "code": self.error_code(),
            "message": self.public_message(),
            "trace_id": trace_id,
            "timestamp": timestamp,
        });

        // batch-17 P3: 为 TooManyRequests 添加 Retry-After HTTP 头
        let mut response = (status, Json(body)).into_response();
        if let AppError::TooManyRequests {
            retry_after: Some(seconds),
            ..
        } = &self
        {
            response
                .headers_mut()
                .insert("Retry-After", seconds.to_string().parse().unwrap());
        }

        response
    }
}

impl AppError {
    /// 返回 (status, error_type) 用于 HTTP 响应状态码与日志分类
    fn error_status_and_type(&self) -> (StatusCode, &'static str) {
        match self {
            AppError::DatabaseError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "DatabaseError"),
            AppError::ValidationError(_) => (StatusCode::BAD_REQUEST, "ValidationError"),
            AppError::ValidationErrorDisplayable(_) => (StatusCode::BAD_REQUEST, "ValidationError"),
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, "NotFound"),
            AppError::BusinessError(_) => (StatusCode::BAD_REQUEST, "BusinessError"),
            AppError::BusinessErrorDisplayable(_) => (StatusCode::BAD_REQUEST, "BusinessError"),
            AppError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "Unauthorized"),
            AppError::InternalError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "InternalError"),
            AppError::PermissionDenied(_) => (StatusCode::FORBIDDEN, "PermissionDenied"),
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, "BadRequest"),
            AppError::NotImplemented(_) => (StatusCode::NOT_IMPLEMENTED, "NotImplemented"),
            AppError::TooManyRequests { .. } => (StatusCode::TOO_MANY_REQUESTS, "TooManyRequests"),
        }
    }

    /// 返回 (severity, action_required) 用于日志辅助信息
    fn error_severity_and_action(&self) -> (&'static str, &'static str) {
        match self {
            AppError::DatabaseError(_) => ("HIGH", err_msg::ACTION_DB),
            AppError::ValidationError(_) => ("LOW", err_msg::ACTION_VALIDATION),
            AppError::ValidationErrorDisplayable(_) => ("LOW", err_msg::ACTION_VALIDATION),
            AppError::NotFound(_) => ("MEDIUM", err_msg::ACTION_NOT_FOUND),
            AppError::BusinessError(_) => ("MEDIUM", err_msg::ACTION_BUSINESS),
            AppError::BusinessErrorDisplayable(_) => ("MEDIUM", err_msg::ACTION_BUSINESS),
            AppError::Unauthorized(_) => ("HIGH", err_msg::ACTION_UNAUTHORIZED),
            AppError::InternalError(_) => ("CRITICAL", err_msg::ACTION_INTERNAL),
            AppError::PermissionDenied(_) => ("HIGH", err_msg::ACTION_PERMISSION),
            AppError::BadRequest(_) => ("LOW", err_msg::ACTION_BAD_REQUEST),
            AppError::NotImplemented(_) => ("MEDIUM", err_msg::ACTION_NOT_IMPLEMENTED),
            AppError::TooManyRequests { .. } => ("MEDIUM", err_msg::ACTION_TOO_MANY_REQUESTS),
        }
    }

    /// 返回错误消息字符串引用（TooManyRequests 用 message 字段）
    fn message_str(&self) -> &str {
        match self {
            AppError::DatabaseError(m)
            | AppError::ValidationError(m)
            | AppError::ValidationErrorDisplayable(m)
            | AppError::NotFound(m)
            | AppError::BusinessError(m)
            | AppError::BusinessErrorDisplayable(m)
            | AppError::Unauthorized(m)
            | AppError::InternalError(m)
            | AppError::BadRequest(m)
            | AppError::PermissionDenied(m)
            | AppError::NotImplemented(m) => m,
            AppError::TooManyRequests { message, .. } => message,
        }
    }

    /// 构建 detail JSON（TooManyRequests 含 retry_after；仅用于 tracing 日志，不进 HTTP 响应）
    fn build_detail(
        &self,
        error_type: &'static str,
        severity: &'static str,
        action_required: &'static str,
    ) -> serde_json::Value {
        let msg = self.message_str();
        match self {
            AppError::TooManyRequests { retry_after, .. } => serde_json::json!({
                "error_type": error_type,
                "message": msg,
                "retry_after": retry_after,
                "severity": severity,
                "action_required": action_required
            }),
            _ => serde_json::json!({
                "error_type": error_type,
                "message": msg,
                "severity": severity,
                "action_required": action_required
            }),
        }
    }

    /// 返回 (log_label, log_suggestion) 用于 tracing 日志定制文案
    fn log_meta(&self) -> (&'static str, String) {
        match self {
            AppError::DatabaseError(_) => (err_msg::LOG_DB_ERROR, err_msg::HINT_DB.to_string()),
            AppError::ValidationError(_) => (
                err_msg::LOG_VALIDATION,
                err_msg::HINT_VALIDATION.to_string(),
            ),
            AppError::ValidationErrorDisplayable(_) => (
                err_msg::LOG_VALIDATION,
                err_msg::HINT_VALIDATION.to_string(),
            ),
            AppError::NotFound(_) => (err_msg::LOG_NOT_FOUND, err_msg::HINT_NOT_FOUND.to_string()),
            AppError::BusinessError(_) => {
                (err_msg::LOG_BUSINESS, err_msg::HINT_BUSINESS.to_string())
            }
            AppError::BusinessErrorDisplayable(_) => {
                (err_msg::LOG_BUSINESS, err_msg::HINT_BUSINESS.to_string())
            }
            AppError::Unauthorized(_) => (
                err_msg::LOG_UNAUTHORIZED,
                err_msg::HINT_UNAUTHORIZED.to_string(),
            ),
            AppError::InternalError(_) => {
                (err_msg::LOG_INTERNAL, err_msg::HINT_INTERNAL.to_string())
            }
            AppError::PermissionDenied(_) => (
                err_msg::LOG_PERMISSION,
                err_msg::HINT_PERMISSION.to_string(),
            ),
            AppError::BadRequest(_) => (
                err_msg::LOG_BAD_REQUEST,
                err_msg::HINT_BAD_REQUEST.to_string(),
            ),
            AppError::NotImplemented(_) => (
                err_msg::LOG_NOT_IMPLEMENTED,
                err_msg::HINT_NOT_IMPLEMENTED.to_string(),
            ),
            AppError::TooManyRequests { retry_after, .. } => (
                err_msg::LOG_TOO_MANY_REQUESTS,
                format!(
                    "{}{:?}{}",
                    err_msg::RETRY_HINT_PREFIX,
                    retry_after,
                    err_msg::RETRY_HINT_SUFFIX
                ),
            ),
        }
    }

    /// 记录结构化错误日志（DatabaseError/InternalError 用 ERROR 级别，其余用 WARN）
    fn log_error(&self, detail: &serde_json::Value) {
        let (label, suggestion) = self.log_meta();
        let msg = self.message_str();
        let is_error = matches!(
            self,
            AppError::DatabaseError(_) | AppError::InternalError(_)
        );
        if is_error {
            tracing::error!(
                "【{label}】{msg} | {detail_word}: {detail} | {suggestion_word}: {suggestion}",
                label = label,
                msg = msg,
                detail_word = err_msg::LOG_DETAIL,
                detail = detail,
                suggestion_word = err_msg::LOG_SUGGESTION,
                suggestion = suggestion
            );
        } else {
            tracing::warn!(
                "【{label}】{msg} | {detail_word}: {detail} | {suggestion_word}: {suggestion}",
                label = label,
                msg = msg,
                detail_word = err_msg::LOG_DETAIL,
                detail = detail,
                suggestion_word = err_msg::LOG_SUGGESTION,
                suggestion = suggestion
            );
        }
    }
}

impl From<sea_orm::DbErr> for AppError {
    fn from(err: sea_orm::DbErr) -> Self {
        let err_str = err.to_string();
        match &err {
            sea_orm::DbErr::Conn(_) => {
                tracing::error!("{}：{}", err_msg::DB_CONN_FAIL, err);
                AppError::database(err_msg::DB_CONN_FAIL)
            }
            sea_orm::DbErr::Exec(_) => {
                let error_kind = classify_db_exec_error(&err_str);
                tracing::error!("{} [{}]: {}", err_msg::DB_EXEC, error_kind, err);
                AppError::database(error_kind.to_string())
            }
            sea_orm::DbErr::Query(_) => {
                let error_kind = classify_db_query_error(&err_str);
                tracing::error!("{} [{}]: {}", err_msg::DB_QUERY, error_kind, err);
                AppError::database(error_kind.to_string())
            }
            sea_orm::DbErr::RecordNotFound(msg) => {
                tracing::warn!("{}：{}", err_msg::LOG_RECORD_NOT_FOUND, msg);
                AppError::not_found(msg.clone())
            }
            sea_orm::DbErr::Custom(_) => {
                let error_kind = classify_db_custom_error(&err_str);
                tracing::error!("{} [{}]: {}", err_msg::DB_CUSTOM, error_kind, err);
                AppError::database(error_kind.to_string())
            }
            sea_orm::DbErr::Type(msg) => {
                tracing::error!("{}：{:?}", err_msg::DB_TYPE_LABEL, msg);
                AppError::database(format!("{}: {}", err_msg::DB_TYPE_LABEL, msg))
            }
            sea_orm::DbErr::Json(msg) => {
                tracing::error!("{}：{}", err_msg::LOG_DB_JSON, msg);
                AppError::database(err_msg::DB_JSON_ERR)
            }
            sea_orm::DbErr::Migration(msg) => {
                tracing::error!("{}：{}", err_msg::DB_MIGRATION_ERR, msg);
                AppError::database(err_msg::DB_MIGRATION_ERR)
            }
            _ => {
                tracing::error!("{}：{}", err_msg::DB_OP_FAIL, err);
                AppError::database(err_msg::DB_OP_FAIL)
            }
        }
    }
}

fn classify_db_exec_error(err_str: &str) -> &'static str {
    if err_str.contains("unique constraint") || err_str.contains("duplicate") {
        err_msg::DB_DUPLICATE
    } else if err_str.contains("foreign key constraint") || err_str.contains("references") {
        err_msg::DB_RELATION
    } else {
        err_msg::DB_EXEC
    }
}

fn classify_db_query_error(err_str: &str) -> &'static str {
    if err_str.contains("syntax error") {
        err_msg::DB_QUERY_SYNTAX
    } else {
        err_msg::DB_QUERY
    }
}

fn classify_db_custom_error(err_str: &str) -> &'static str {
    if err_str.contains("timeout") {
        err_msg::DB_TIMEOUT
    } else {
        err_msg::DB_CUSTOM
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::internal(format!("{}{}", err_msg::JSON_SERIALIZE_PREFIX, err))
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
    fn from(errors: validator::ValidationErrors) -> Self {
        AppError::validation_displayable(readable_validation_errors(&errors))
    }
}

/// 把 `validator` 错误集压成可直接外显的可读文案。
///
/// 字段级错误只取本仓库自行撰写的 `message`（描述的是用户自己刚提交的字段，
/// 满足模块文档的安全边界）；无 `message` 时退回规则 `code` 并冠上字段名，
/// 避免只剩无上下文的规则标识。多条按出现顺序去重后以「；」拼接，让一次请求
/// 就能改对所有越界字段。一条都提不出来时退回整体序列化——不吞掉拒绝原因。
fn readable_validation_errors(errors: &validator::ValidationErrors) -> String {
    fn collect(map: &validator::ValidationErrors, out: &mut Vec<String>) {
        for (field, kind) in map.errors() {
            match kind {
                validator::ValidationErrorsKind::Field(errs) => {
                    for e in errs {
                        out.push(match e.message.clone() {
                            Some(msg) => msg.into_owned(),
                            None => format!("{field}: {}", e.code),
                        });
                    }
                }
                validator::ValidationErrorsKind::Struct(inner) => collect(inner, out),
                validator::ValidationErrorsKind::List(items) => {
                    for inner in items.values() {
                        collect(inner, out);
                    }
                }
            }
        }
    }

    let mut parts = Vec::new();
    collect(errors, &mut parts);
    let mut seen = std::collections::HashSet::new();
    parts.retain(|p| seen.insert(p.clone()));
    if parts.is_empty() {
        errors.to_string()
    } else {
        parts.join("；")
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
#[derive(Debug, Clone, Serialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
    pub trace_id: String,
    pub timestamp: i64,
}

/// `Unauthorized` 变体的字符串错误码（与 [`AppError::error_code`] 同源）。
/// 认证中间件不经 `AppError` 出参（需原样外显固定文案）时必须复用这两个常量，
/// 保证失败信封的 `code` 取值全站唯一。
pub const CODE_UNAUTHORIZED: &str = "UNAUTHORIZED";
/// `PermissionDenied` 变体的字符串错误码（与 [`AppError::error_code`] 同源）
pub const CODE_FORBIDDEN: &str = "FORBIDDEN";

/// 为已有 `AppError` 追加响应序列化能力（不修改任何现有方法）
impl AppError {
    /// 转换为对外统一的 [`ErrorResponse`]
    pub fn to_response(&self) -> ErrorResponse {
        let trace_id = Uuid::new_v4().to_string();
        let timestamp = Utc::now().timestamp();

        // 漏洞 #4 / #8 修复：to_response 与 IntoResponse 保持一致，
        // 永远返回脱敏的 public_message，不再根据环境暴露 Display 完整内容
        // （避免开发环境/测试环境对外暴露时泄露 SQL / 文件路径 / 堆栈）。
        // 详细信息通过 trace_id 在服务端日志（tracing）中查询。
        let code = self.error_code();
        let message = self.public_message();

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
            AppError::Unauthorized(_) => CODE_UNAUTHORIZED,
            AppError::PermissionDenied(_) => CODE_FORBIDDEN,
            AppError::ValidationError(_) => "VALIDATION_ERROR",
            AppError::ValidationErrorDisplayable(_) => "VALIDATION_ERROR",
            AppError::BusinessError(_) => "BUSINESS_ERROR",
            AppError::BusinessErrorDisplayable(_) => "BUSINESS_ERROR",
            AppError::DatabaseError(_) => "DATABASE_ERROR",
            AppError::InternalError(_) => "INTERNAL_ERROR",
            AppError::NotImplemented(_) => "NOT_IMPLEMENTED",
            AppError::TooManyRequests { .. } => "TOO_MANY_REQUESTS",
        }
        .to_string()
    }

    /// 生产环境对外暴露的脱敏文案。
    ///
    /// 例外：`BusinessErrorDisplayable` 由构造点（`AppError::business_displayable`）
    /// 显式声明文案安全，直接返回真实文案；其余一律返回固定脱敏常量。
    /// 出参文案：默认脱敏，仅 `BusinessErrorDisplayable` 变体返回真实文案。
    pub(crate) fn public_message(&self) -> String {
        match self {
            AppError::DatabaseError(_) => err_msg::DB_ERROR_PUBLIC.to_string(),
            AppError::ValidationError(_) => err_msg::VALIDATION_PUBLIC.to_string(),
            AppError::ValidationErrorDisplayable(msg) => msg.clone(),
            AppError::NotFound(_) => err_msg::NOT_FOUND_PUBLIC.to_string(),
            AppError::BusinessError(_) => err_msg::BUSINESS_PUBLIC.to_string(),
            AppError::BusinessErrorDisplayable(msg) => msg.clone(),
            AppError::Unauthorized(_) => err_msg::UNAUTHORIZED_PUBLIC.to_string(),
            AppError::InternalError(_) => err_msg::INTERNAL_PUBLIC.to_string(),
            AppError::BadRequest(_) => err_msg::BAD_REQUEST_PUBLIC.to_string(),
            AppError::PermissionDenied(_) => err_msg::PERMISSION_PUBLIC.to_string(),
            AppError::NotImplemented(_) => err_msg::NOT_IMPLEMENTED_PUBLIC.to_string(),
            AppError::TooManyRequests { .. } => err_msg::TOO_MANY_REQUESTS_PUBLIC.to_string(),
        }
    }
}

#[cfg(test)]
mod displayable_message_tests {
    use super::*;

    /// ① 默认 business 构造：出参 message 仍为固定脱敏常量，真实文案不外显
    #[test]
    fn business_default_still_sanitized() {
        let err = AppError::business("二级审批人不能与一级审批人相同");
        let resp = err.to_response();
        assert_eq!(resp.message, err_msg::BUSINESS_PUBLIC);
        assert_eq!(resp.message, "业务处理失败");
    }

    /// ② displayable 构造：出参 message 携带真实文案
    #[test]
    fn business_displayable_exposes_real_message() {
        let err = AppError::business_displayable("审批人不能是申请人");
        let resp = err.to_response();
        assert_eq!(resp.message, "审批人不能是申请人");
    }

    /// ③ 两种构造的 code 完全一致（出参契约不变）
    #[test]
    fn business_and_displayable_share_code() {
        let a = AppError::business("x");
        let b = AppError::business_displayable("x");
        assert_eq!(a.error_code(), b.error_code());
        assert_eq!(b.error_code(), "BUSINESS_ERROR");
    }

    /// IntoResponse 出参链路与 to_response 共用同一 public_message，
    /// 这里验证两条链路（默认脱敏 / 显式外显）行为一致
    #[test]
    fn into_response_and_to_response_consistent() {
        let err_default = AppError::business("库存不足 12.5");
        assert_eq!(err_default.error_code(), "BUSINESS_ERROR");
        assert_eq!(err_default.to_response().message, err_msg::BUSINESS_PUBLIC);
        assert!(!err_default.to_response().message.contains("12.5"));

        let err_display = AppError::business_displayable("只有敏感角色变更需要审批");
        assert_eq!(err_display.error_code(), "BUSINESS_ERROR");
        assert_eq!(
            err_display.to_response().message,
            "只有敏感角色变更需要审批"
        );
    }

    /// 真实文案仍进 Display（tracing 日志侧不受外显开关影响）
    #[test]
    fn display_keeps_real_message_for_both() {
        assert!(
            AppError::business("内部原因ABC")
                .to_string()
                .contains("内部原因ABC")
        );
        assert!(
            AppError::business_displayable("审批人不能是申请人")
                .to_string()
                .contains("审批人不能是申请人")
        );
    }

    // ------------------------------------------------------------------
    // 校验族（ValidationError / ValidationErrorDisplayable）对称断言
    // ------------------------------------------------------------------

    /// ① 手工 `validation` 构造仍默认脱敏（携带第三方库错误原文的构造点必须走这条）
    #[test]
    fn validation_default_still_sanitized() {
        let err = AppError::validation("Timestamp parse failed: premature end of input");
        assert_eq!(err.error_code(), "VALIDATION_ERROR");
        assert_eq!(err.to_response().message, err_msg::VALIDATION_PUBLIC);
        assert!(
            !err.to_response().message.contains("premature"),
            "第三方库错误原文不得进 HTTP 出参"
        );
    }

    /// ② displayable 构造：code/HTTP 与脱敏形态完全一致，仅 message 换真实原因
    #[test]
    fn validation_displayable_exposes_real_reason() {
        use axum::response::IntoResponse as _;
        let err = AppError::validation_displayable("批次号长度必须在1-50个字符之间");
        assert_eq!(err.error_code(), "VALIDATION_ERROR");
        assert_eq!(err.to_response().message, "批次号长度必须在1-50个字符之间");
        assert_eq!(
            AppError::validation("x").into_response().status(),
            err.clone().into_response().status(),
            "两变体 HTTP 状态必须一致（不得因外显而改变出参契约）"
        );
    }

    /// ③ `From<validator::ValidationErrors>`：提取字段级可读文案、多字段去重拼接，
    /// 且产出的是可外显变体——这是"提交只报参数错误、看不到原因"的总修复点。
    #[test]
    fn validation_errors_from_dto_are_readable_and_displayable() {
        use validator::Validate;

        #[derive(Validate)]
        struct Probe {
            #[validate(length(min = 1, message = "批次号不能为空"))]
            batch_no: String,
            #[validate(range(min = 0, message = "数量不能为负"))]
            quantity: i32,
        }

        let errors = Probe {
            batch_no: String::new(),
            quantity: -1,
        }
        .validate()
        .expect_err("空批次号与负数量必须被拒");

        let err = AppError::from(errors);
        assert!(
            matches!(err, AppError::ValidationErrorDisplayable(_)),
            "DTO 校验拒绝必须走可读外显变体，实际={err:?}"
        );
        let msg = err.to_response().message;
        assert_ne!(msg, err_msg::VALIDATION_PUBLIC, "出参不得再是脱敏常量");
        assert!(msg.contains("批次号不能为空"), "实际外显: {msg}");
        assert!(
            msg.contains("数量不能为负"),
            "两个字段的原因都必须带到: {msg}"
        );
        assert!(
            !msg.contains("ValidationErrors") && !msg.contains("batch_no"),
            "不得把结构体序列化文本/内部字段名塞给出参: {msg}"
        );
    }

    /// ④ 无 message 只有规则 code 时退回「字段: code」，整体为空时退回序列化——不吞原因
    #[test]
    fn validation_errors_without_message_still_surface_something() {
        let mut errors = validator::ValidationErrors::new();
        errors.add("consent_type", validator::ValidationError::new("length"));
        let msg = AppError::from(errors).to_response().message;
        assert_eq!(msg, "consent_type: length", "实际: {msg}");
    }
}
