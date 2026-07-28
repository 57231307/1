//! 业务消息常量模块
//!
//! 批次 404 新增：集中管理 handler 层常用的 CRUD 业务消息，避免跨文件硬编码
//! 导致的文案不一致问题（如"删除成功" vs "客户删除成功" vs "评估已删除"）。
//!
//! 使用方式：
//! ```rust,ignore
//! use crate::utils::messages::biz_msg;
//! Ok(Json(ApiResponse::success_with_message((), biz_msg::DELETE_OK)))
//! ```

/// CRUD 通用业务消息常量
pub mod biz_msg {
    /// 创建成功
    pub const CREATE_OK: &str = "创建成功";
    /// 更新成功
    pub const UPDATE_OK: &str = "更新成功";
    /// 删除成功
    pub const DELETE_OK: &str = "删除成功";
    /// 审批通过
    pub const APPROVE_OK: &str = "审批通过";
    /// 执行成功
    pub const EXECUTE_OK: &str = "执行成功";
    /// 操作成功
    pub const OPERATE_OK: &str = "操作成功";
}

/// AppError 错误消息常量（V15 批次 07 P1-10：集中管理 error.rs 硬编码中文，use crate::utils::messages::err_msg 引用）
pub mod err_msg {
    // === Display 前缀（write! 拼接 msg） ===
    pub const DB_ERROR_PREFIX: &str = "数据库错误：";
    pub const VALIDATION_PREFIX: &str = "验证错误：";
    pub const NOT_FOUND_PREFIX: &str = "未找到：";
    pub const BUSINESS_PREFIX: &str = "业务错误：";
    pub const UNAUTHORIZED_PREFIX: &str = "未授权：";
    pub const INTERNAL_PREFIX: &str = "内部错误：";
    pub const BAD_REQUEST_PREFIX: &str = "请求错误：";
    pub const PERMISSION_PREFIX: &str = "权限不足：";
    pub const NOT_IMPLEMENTED_PREFIX: &str = "未实现：";

    // === public_message 脱敏文案（HTTP 响应） ===
    pub const DB_ERROR_PUBLIC: &str = "数据库错误";
    pub const VALIDATION_PUBLIC: &str = "请求参数验证失败";
    pub const NOT_FOUND_PUBLIC: &str = "资源未找到";
    pub const BUSINESS_PUBLIC: &str = "业务处理失败";
    pub const UNAUTHORIZED_PUBLIC: &str = "未授权";
    pub const INTERNAL_PUBLIC: &str = "服务器内部错误";
    pub const BAD_REQUEST_PUBLIC: &str = "请求参数错误";
    pub const PERMISSION_PUBLIC: &str = "无权限";
    pub const NOT_IMPLEMENTED_PUBLIC: &str = "功能未实现";
    pub const TOO_MANY_REQUESTS_PUBLIC: &str = "请求过于频繁，请稍后重试";

    // === log_meta 日志标签 ===
    pub const LOG_DB_ERROR: &str = "数据库错误";
    pub const LOG_VALIDATION: &str = "验证错误";
    pub const LOG_NOT_FOUND: &str = "资源未找到";
    pub const LOG_BUSINESS: &str = "业务错误";
    pub const LOG_UNAUTHORIZED: &str = "未授权访问";
    pub const LOG_INTERNAL: &str = "内部错误";
    pub const LOG_PERMISSION: &str = "权限不足";
    pub const LOG_BAD_REQUEST: &str = "请求错误";
    pub const LOG_NOT_IMPLEMENTED: &str = "功能未实现";
    pub const LOG_TOO_MANY_REQUESTS: &str = "请求过多";

    // === log_meta 修复建议 ===
    pub const HINT_DB: &str = "检查数据库连接状态和 SQL 查询";
    pub const HINT_VALIDATION: &str = "检查请求参数格式和必填项";
    pub const HINT_NOT_FOUND: &str = "检查资源 ID 是否正确或资源是否已被删除";
    pub const HINT_BUSINESS: &str = "检查业务规则和前置条件";
    pub const HINT_UNAUTHORIZED: &str = "检查 Token 是否有效或是否已过期";
    pub const HINT_INTERNAL: &str = "检查系统日志或联系管理员";
    pub const HINT_PERMISSION: &str = "检查用户角色和权限配置";
    pub const HINT_BAD_REQUEST: &str = "检查请求格式和参数";
    pub const HINT_NOT_IMPLEMENTED: &str = "该功能正在开发中";

    // === 数据库错误分类（classify_db_*） ===
    pub const DB_DUPLICATE: &str = "数据重复";
    pub const DB_RELATION: &str = "数据关联错误";
    pub const DB_EXEC: &str = "数据库执行错误";
    pub const DB_QUERY_SYNTAX: &str = "查询语法错误";
    pub const DB_QUERY: &str = "数据库查询错误";
    pub const DB_TIMEOUT: &str = "数据库操作超时";
    pub const DB_CUSTOM: &str = "数据库自定义错误";
    pub const DB_CONN_FAIL: &str = "数据库连接失败";
    pub const DB_TYPE_LABEL: &str = "数据库类型错误";
    pub const DB_JSON_ERR: &str = "数据库 JSON 处理错误";
    pub const DB_MIGRATION_ERR: &str = "数据库迁移错误";
    pub const DB_OP_FAIL: &str = "数据库操作失败";

    // === error_severity_and_action 的 action_required（detail JSON 用） ===
    pub const ACTION_DB: &str = "检查数据库连接和查询";
    pub const ACTION_VALIDATION: &str = "检查请求参数";
    pub const ACTION_NOT_FOUND: &str = "检查资源是否存在";
    pub const ACTION_BUSINESS: &str = "检查业务规则";
    pub const ACTION_UNAUTHORIZED: &str = "检查认证信息";
    pub const ACTION_INTERNAL: &str = "联系系统管理员";
    pub const ACTION_PERMISSION: &str = "检查用户权限";
    pub const ACTION_BAD_REQUEST: &str = "检查请求格式";
    pub const ACTION_NOT_IMPLEMENTED: &str = "联系开发团队实现该功能";
    pub const ACTION_TOO_MANY_REQUESTS: &str = "稍后重试";

    // === tracing 日志文案 ===
    pub const LOG_RECORD_NOT_FOUND: &str = "记录不存在";
    pub const LOG_DB_JSON: &str = "数据库 JSON 错误";
    pub const LOG_DETAIL: &str = "详情";
    pub const LOG_SUGGESTION: &str = "建议";

    // === 其他 ===
    pub const JSON_SERIALIZE_PREFIX: &str = "JSON 序列化错误：";
    pub const RETRY_HINT_PREFIX: &str = "等待 ";
    pub const RETRY_HINT_SUFFIX: &str = " 秒后重试";
}
