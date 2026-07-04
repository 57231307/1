//! 业务状态常量模块
//!
//! 批次 100 P3-A 修复（v5 复审）：抽取 4 个 service 文件中的硬编码状态字符串为常量，
//! 提高可维护性，避免字符串拼写错误导致状态匹配失败。
//!
//! 按业务域分组：
//! - 通用状态：DRAFT/PENDING/APPROVED/CANCELLED/COMPLETED/ACTIVE（多业务共用）
//! - 生产订单专属：SCHEDULED/IN_PROGRESS/PENDING_APPROVAL/REJECTED
//! - 付款专属：REGISTERED/CONFIRMED/PAID/PARTIAL_PAID
//! - 采购订单状态：保留历史模块（dead_code，待业务接入）
//! - 销售订单状态：保留历史模块（dead_code，待业务接入）
//! - 通用审批状态：保留历史模块（dead_code，待业务接入）

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

/// 生产订单状态常量
pub mod production {
    /// 已排产：草稿审批通过后排入生产计划
    pub const PRODUCTION_SCHEDULED: &str = "SCHEDULED";

    /// 生产中：实际开始投料生产
    pub const PRODUCTION_IN_PROGRESS: &str = "IN_PROGRESS";

    /// 待审批：提交审批流程，等待审批结果
    pub const PRODUCTION_PENDING_APPROVAL: &str = "PENDING_APPROVAL";

    /// 已拒绝：审批未通过，可退回草稿重新编辑
    pub const PRODUCTION_REJECTED: &str = "REJECTED";
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

// 采购订单状态（历史模块，待业务接入后逐项移除 allow）
#[allow(dead_code)] // TODO(tech-debt): 业务代码当前用字符串字面量，未来应统一引用此常量
pub mod purchase_order {
    /// 草稿
    pub const DRAFT: &str = "DRAFT";
    /// 待审批
    pub const PENDING_APPROVAL: &str = "PENDING_APPROVAL";
    /// 已提交
    pub const SUBMITTED: &str = "SUBMITTED";
    /// 已审批
    pub const APPROVED: &str = "APPROVED";
    /// 已拒绝
    pub const REJECTED: &str = "REJECTED";
    /// 已收货
    pub const RECEIVED: &str = "RECEIVED";
    /// 已关闭
    pub const CLOSED: &str = "CLOSED";
    /// 已取消
    pub const CANCELLED: &str = "CANCELLED";
    /// 已完成
    pub const COMPLETED: &str = "COMPLETED";
    /// 部分收货
    pub const PARTIAL_RECEIVED: &str = "PARTIAL_RECEIVED";
}

// 销售订单状态
// 批次 14（2026-06-28）：修正常量值为小写，与业务代码（order_workflow.rs/order_crud.rs/delivery.rs）一致；
// 补全 partial_shipped 和 shipped 状态；删除业务中不存在的 PENDING_APPROVAL 和 CONFIRMED。
// 原常量值大写（"DRAFT"）与业务小写（"draft"）矛盾，若被引用会查不到数据（隐性 P0 风险）。
#[allow(dead_code)] // TODO(tech-debt): 业务代码当前用字符串字面量，未来应统一引用此常量
pub mod sales_order {
    /// 草稿
    pub const DRAFT: &str = "draft";
    /// 待审核
    pub const PENDING: &str = "pending";
    /// 已审核
    pub const APPROVED: &str = "approved";
    /// 部分发货
    pub const PARTIAL_SHIPPED: &str = "partial_shipped";
    /// 已发货
    pub const SHIPPED: &str = "shipped";
    /// 已完成
    pub const COMPLETED: &str = "completed";
    /// 已取消
    pub const CANCELLED: &str = "cancelled";
}

// 通用审批状态（历史模块，待业务接入后逐项移除 allow）
#[allow(dead_code)] // TODO(tech-debt): 业务代码当前用字符串字面量，未来应统一引用此常量
pub mod approval {
    /// 草稿
    pub const DRAFT: &str = "DRAFT";
    /// 待审批
    pub const PENDING: &str = "PENDING";
    /// 已审批
    pub const APPROVED: &str = "APPROVED";
    /// 已拒绝
    pub const REJECTED: &str = "REJECTED";
    /// 已取消
    pub const CANCELLED: &str = "CANCELLED";
}
