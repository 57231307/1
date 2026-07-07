//! 业务状态常量模块
//!
//! 批次 100 P3-A 修复（v5 复审）：抽取 4 个 service 文件中的硬编码状态字符串为常量，
//! 提高可维护性，避免字符串拼写错误导致状态匹配失败。
//!
//! 按业务域分组：
//! - 通用状态：DRAFT/PENDING/APPROVED/CANCELLED/COMPLETED/ACTIVE（多业务共用）
//! - 生产订单专属：SCHEDULED/IN_PROGRESS/PENDING_APPROVAL/REJECTED
//! - 付款专属：REGISTERED/CONFIRMED/PAID/PARTIAL_PAID
//! - 采购订单状态：批次 158 v11 真实接入 po/ 子模块
//! - 销售订单状态：批次 158 v11 真实接入 so/ 子模块
//! - 通用审批状态：批次 158 v11 真实接入 color_price / budget_adjustment / ar_invoice
//! - 库存预留状态：批次 158 v11 真实接入 so/delivery
//! - 销售发货状态：批次 158 v11 真实接入 so/delivery

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

/// 应收账款专属状态常量（批次 102 v6 P3-1 修复）
///
/// ar_service.rs 中三类模型的状态字段值：
/// - ar_collection.status：小写（pending/confirmed/cancelled）
/// - ar_reconciliation.reconciliation_status：大写（COMPLETED/CANCELLED）
/// - ar_reconciliation_item.match_status：大写（MATCHED）
///
/// ar_invoice.status 复用 common::STATUS_* 与 payment::PAYMENT_*（与 ar_invoice_service.rs 保持一致）
pub mod ar {
    /// 收款单待确认（ar_collection.status，小写值）
    pub const COLLECTION_PENDING: &str = "pending";

    /// 收款单已确认（ar_collection.status，小写值）
    pub const COLLECTION_CONFIRMED: &str = "confirmed";

    /// 收款单已取消（ar_collection.status，小写值）
    ///
    /// 批次 158 v11 真实接入：ar_service.rs cancel_collection 方法使用此常量将收款状态置为 cancelled
    pub const COLLECTION_CANCELLED: &str = "cancelled";

    /// 核销单已完成（ar_reconciliation.reconciliation_status，大写值）
    pub const RECONCILIATION_COMPLETED: &str = "COMPLETED";

    /// 核销单已取消（ar_reconciliation.reconciliation_status，大写值）
    pub const RECONCILIATION_CANCELLED: &str = "CANCELLED";

    /// 核销明细已匹配（ar_reconciliation_item.match_status，大写值）
    pub const MATCH_MATCHED: &str = "MATCHED";
}

/// 应付发票专属状态常量（批次 102 v6 P3-2 修复）
///
/// ap_invoice.invoice_status 字段值（大写）：
/// - DRAFT：草稿（与 common::STATUS_DRAFT 同值，但语义独立，单独定义便于维护）
/// - AUDITED：已审核（ap_invoice 专属，非通用审批状态）
/// - PAID：已付款（与 payment::PAYMENT_PAID 同值）
/// - PARTIAL_PAID：部分付款（与 payment::PAYMENT_PARTIAL_PAID 同值）
/// - CANCELLED：已取消（与 common::STATUS_CANCELLED 同值）
///
/// 复用策略：DRAFT/PAID/PARTIAL_PAID/CANCELLED 直接引用 common/payment 模块常量，
/// 仅 AUDITED 在本模块单独定义。
pub mod ap_invoice {
    /// 已审核（ap_invoice 专属状态，区别于通用 APPROVED）
    pub const INVOICE_AUDITED: &str = "AUDITED";
}

/// 应付付款申请专属审批状态常量（批次 102 v6 P3-3 修复）
///
/// ap_payment_request.approval_status 字段值（大写）：
/// - DRAFT：草稿（与 common::STATUS_DRAFT 同值，复用）
/// - APPROVING：审批中（ap_payment_request 专属）
/// - APPROVED：已审批（与 common::STATUS_APPROVED 同值，复用）
/// - REJECTED：已拒绝（与 approval::REJECTED 同值，但 approval 模块为 dead_code，
///   此处单独定义避免依赖 dead_code 模块）
pub mod ap_payment_request {
    /// 审批中（ap_payment_request 专属状态）
    pub const APPROVAL_APPROVING: &str = "APPROVING";

    /// 已拒绝（避免依赖 dead_code 的 approval::REJECTED，单独定义）
    pub const APPROVAL_REJECTED: &str = "REJECTED";
}

/// 凭证状态常量（批次 102 v6 P3-1 修复）
///
/// voucher.status 字段值（小写，凭证专属状态机）：
/// draft → submitted → reviewed → posted
///
/// 注意：与 ar_invoice / ap_invoice 的大写状态值不同，凭证全部用小写。
pub mod voucher {
    /// 草稿：凭证初始状态，可编辑
    pub const VOUCHER_DRAFT: &str = "draft";

    /// 已提交：等待审核
    pub const VOUCHER_SUBMITTED: &str = "submitted";

    /// 已审核：审核通过，等待过账
    pub const VOUCHER_REVIEWED: &str = "reviewed";

    /// 已过账：已记入账簿，不可再修改
    pub const VOUCHER_POSTED: &str = "posted";
}

// 采购订单状态
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
// 批次 158 v11 真实接入：移除 allow 标注，业务代码引用常量替代字符串字面量（规则 0）
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
    /// 已拒绝（so/contract.rs reject_order 接入，批次 158 v11 真实接入）
    pub const REJECTED: &str = "rejected";
}

// 通用审批状态（批次 158 v11 真实接入：color_price / budget_adjustment / ar_invoice 业务引用）
// 注：DRAFT 和 CANCELLED 在当前业务中无使用场景已删除；如未来审批流程扩展需要可重新添加
pub mod approval {
    /// 待审批
    pub const PENDING: &str = "PENDING";
    /// 已审批
    pub const APPROVED: &str = "APPROVED";
    /// 已拒绝
    pub const REJECTED: &str = "REJECTED";
}

// 库存预留状态（inventory_reservation.status，小写值）
// 批次 158 v11 真实接入：so/delivery.rs 中库存预留状态字符串字面量统一引用此模块（规则 0）
pub mod inventory_reservation {
    /// 待处理（已创建预留，等待发货扣减）
    pub const PENDING: &str = "pending";
    /// 已完成（发货已扣减库存）
    pub const FULFILLED: &str = "fulfilled";
    /// 已取消（订单取消或库存不足释放）
    pub const CANCELLED: &str = "cancelled";
}

// 销售发货状态（sales_delivery.status，小写值）
// 批次 158 v11 真实接入：so/delivery.rs 中发货状态字符串字面量统一引用此模块（规则 0）
pub mod sales_delivery {
    /// 待处理
    pub const PENDING: &str = "pending";
    /// 已发货
    pub const SHIPPED: &str = "shipped";
    /// 已取消
    pub const CANCELLED: &str = "cancelled";
}
