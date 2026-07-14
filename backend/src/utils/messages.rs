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
