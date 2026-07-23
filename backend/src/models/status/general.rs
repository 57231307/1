//! 通用状态常量分组
//!
//! 批次 490 D10-3b 拆分：从 models/status.rs 抽取的通用/系统级业务状态常量子模块组。
//! 包含：common/payment/master_data/import_task/login_log/email_log/active_status/audit_message/health_check/reconcile_result/failover

/// 通用状态常量（多业务共用）
pub mod common {
    /// 草稿：单据初始状态，可编辑/删除
    pub const STATUS_DRAFT: &str = "DRAFT";

    /// 待处理：通用待办或审批中状态
    pub const STATUS_PENDING: &str = "PENDING";

    /// 已审批：审批流程通过
    pub const STATUS_APPROVED: &str = "APPROVED";

    /// 已取消：单据作废，不可再变更
    pub const STATUS_CANCELLED: &str = "CANCELLED";

    /// 已完成：业务流程完结
    pub const STATUS_COMPLETED: &str = "COMPLETED";

    /// 激活：主数据或库存可用状态
    pub const STATUS_ACTIVE: &str = "ACTIVE";
}

/// 付款状态常量
pub mod payment {
    /// 已登记：付款单已创建但未确认执行
    pub const PAYMENT_REGISTERED: &str = "REGISTERED";

    /// 已确认：付款已执行，等待银行对账
    pub const PAYMENT_CONFIRMED: &str = "CONFIRMED";

    /// 已付款：全部结清（应收/应付通用复用）
    pub const PAYMENT_PAID: &str = "PAID";

    /// 部分付款：尚未结清（应收/应付通用复用）
    pub const PAYMENT_PARTIAL_PAID: &str = "PARTIAL_PAID";
}

/// 主数据启用/停用状态（小写值）
///
/// 批次 208 P2-5 修复（v12 复审）：
/// supplier/customer/fixed_asset 等主数据的 status 字段使用小写 "active"/"inactive"，
/// 与 common::STATUS_ACTIVE（大写 "ACTIVE"）不同，单独定义避免大小写混淆。
pub mod master_data {
    /// 启用：主数据可用状态
    pub const ACTIVE: &str = "active";

    /// 停用：主数据不可用状态
    pub const INACTIVE: &str = "inactive";
}

/// 导入任务状态（import_task.status，小写值）
/// 批次 235 v13 真实接入：import_export_service.rs 中导入任务状态字符串字面量统一引用此模块（规则 0）
pub mod import_task {
    /// 运行中：导入任务正在执行
    pub const RUNNING: &str = "running";

    /// 成功：导入任务全部成功
    pub const SUCCESS: &str = "success";

    /// 失败：导入任务全部失败
    pub const FAILED: &str = "failed";

    /// 部分成功：导入任务部分成功部分失败
    pub const PARTIAL: &str = "partial";
}

/// 登录日志状态（log_login.status，大写值）
/// 批次 236 v13 真实接入：login_security_handler.rs、auth_handler.rs 中登录日志状态字符串字面量统一引用此模块（规则 0）
pub mod login_log {
    /// 成功：登录成功
    pub const SUCCESS: &str = "SUCCESS";

    /// 失败：登录失败
    pub const FAILED: &str = "FAILED";
}

/// 邮件日志状态（email_log.status，大写值）
/// 批次 236 v13 真实接入：email_log_service.rs 中邮件日志状态字符串字面量统一引用此模块（规则 0）
pub mod email_log {
    /// 待发送：邮件待发送
    pub const PENDING: &str = "PENDING";

    /// 已发送：邮件已发送
    pub const SENT: &str = "SENT";

    /// 发送失败：邮件发送失败
    pub const FAILED: &str = "FAILED";
}

/// 启用/停用状态（通用，大写值 ACTIVE/INACTIVE）
/// 批次 236 v13 真实接入：email_template/report_subscription/report_template/bom_process_definition 等启用停用状态
/// 注意：与 master_data 模块（小写 active/inactive）区分，本模块用于大写 ACTIVE/INACTIVE 场景
pub mod active_status {
    /// 启用
    pub const ACTIVE: &str = "ACTIVE";

    /// 停用
    pub const INACTIVE: &str = "INACTIVE";
}

/// 审计消息状态（omni_audit_message.status，大写值）
/// 批次 236 v13 真实接入：omni_audit_service.rs
pub mod audit_message {
    /// 失败：审计消息失败
    pub const FAILED: &str = "FAILED";

    /// 拒绝：审计消息被拒绝
    pub const DENIED: &str = "DENIED";
}

/// 健康检查状态（health_check.status，小写值）
/// 批次 236 v13 真实接入：health_handler.rs
pub mod health_check {
    /// 健康
    pub const HEALTHY: &str = "healthy";

    /// 不健康
    pub const UNHEALTHY: &str = "unhealthy";

    /// 降级
    pub const DEGRADED: &str = "degraded";
}

/// 对账结果状态（reconcile_result.status，大写值）
/// 批次 236 v13 真实接入：ap_reconciliation_handler.rs
pub mod reconcile_result {
    /// 失败
    pub const FAILED: &str = "FAILED";
}

/// 故障切换状态（failover current_state，小写值）
/// 批次 236 v13 真实接入：failover_service.rs
pub mod failover {
    /// 主节点
    pub const PRIMARY: &str = "primary";

    /// 备节点
    pub const BACKUP: &str = "backup";
}
