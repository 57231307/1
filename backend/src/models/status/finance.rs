//! 财务状态常量分组
//!
//! 批次 490 D10-3b 拆分：从 models/status.rs 抽取的财务/应收应付/凭证/会计期间状态常量子模块组。
//! 包含：ar/ap_invoice/ap_payment_request/voucher/accounting_period/finance_invoice/finance_payment/ap_reconciliation/ap_verification/fixed_asset/cost_collection/accounting_period_closing

/// 应收账款专属状态常量（批次 102 v6 P3-1 修复，批次 231 v13 P1-1 统一小写）
///
/// 三类模型的状态字段值（批次 231 统一为小写，修复 ar_service.rs 大写与 ar/recon.rs 小写不一致的 P0 数据问题）：
/// - ar_collection.status：小写（pending/confirmed/cancelled）
/// - ar_reconciliation.reconciliation_status：小写（draft/sent/confirmed/disputed/closed/cancelled）
/// - ar_reconciliation_item.match_status：大写（MATCHED/UNMATCHED）
///
/// ar_invoice.status 复用 common::STATUS_* 与 payment::PAYMENT_*（与 ar_invoice_service.rs 保持一致）
pub mod ar {
    /// 收款单待确认（ar_collection.status，小写值）
    pub const COLLECTION_PENDING: &str = "pending";

    /// 收款单已确认（ar_collection.status，小写值）
    pub const COLLECTION_CONFIRMED: &str = "confirmed";

    /// 收款单已取消（ar_collection.status，小写值）
    pub const COLLECTION_CANCELLED: &str = "cancelled";

    /// 对账单草稿（ar_reconciliation.reconciliation_status，小写值）
    pub const RECONCILIATION_DRAFT: &str = "draft";

    /// 对账单已发送（ar_reconciliation.reconciliation_status，小写值）
    pub const RECONCILIATION_SENT: &str = "sent";

    /// 对账单已确认（ar_reconciliation.reconciliation_status，小写值）
    pub const RECONCILIATION_CONFIRMED: &str = "confirmed";

    /// 对账单有争议（ar_reconciliation.reconciliation_status，小写值）
    pub const RECONCILIATION_DISPUTED: &str = "disputed";

    /// 对账单已关闭（ar_reconciliation.reconciliation_status，小写值）
    pub const RECONCILIATION_CLOSED: &str = "closed";

    /// 对账单已取消（ar_reconciliation.reconciliation_status，小写值）
    pub const RECONCILIATION_CANCELLED: &str = "cancelled";

    /// 核销明细已匹配（ar_reconciliation_item.match_status，大写值）
    pub const MATCH_MATCHED: &str = "MATCHED";

    /// 核销明细未匹配（ar_reconciliation_item.match_status，大写值）
    pub const MATCH_UNMATCHED: &str = "UNMATCHED";
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

/// 会计期间状态常量（大写值）
///
/// 批次 232 v13 P1-1 修复：accounting_period.status 字段状态常量化
/// 状态机：OPEN（开放）→ CLOSED（已关账）
pub mod accounting_period {
    /// 开放：期间可进行凭证录入
    pub const OPEN: &str = "OPEN";

    /// 已关账：期间已结账，不可再录入凭证
    pub const CLOSED: &str = "CLOSED";
}

/// 财务发票状态（finance_invoice.status，小写值）
/// 批次 234 v13 真实接入：finance_invoice_service.rs 中发票状态字符串字面量统一引用此模块（规则 0）
pub mod finance_invoice {
    /// 待处理：发票初始状态，可审批
    pub const PENDING: &str = "pending";

    /// 已审批：审批通过
    pub const APPROVED: &str = "approved";
}

/// 财务付款状态（finance_payment.status，小写值）
/// 批次 234 v13 真实接入：finance_payment_service.rs 中付款状态字符串字面量统一引用此模块（规则 0）
pub mod finance_payment {
    /// 待处理：付款单初始状态
    pub const PENDING: &str = "pending";
}

/// 应付对账状态（ap_reconciliation.reconciliation_status，大写值）
/// 批次 234 v13 真实接入：ap_reconciliation_service.rs 中对账状态字符串字面量统一引用此模块（规则 0）
pub mod ap_reconciliation {
    /// 待处理：对账单初始状态，可执行对账
    pub const PENDING: &str = "PENDING";

    /// 已确认：对账完成，已锁定
    pub const CONFIRMED: &str = "CONFIRMED";

    /// 有争议：对账结果存在异议，需复核
    pub const DISPUTED: &str = "DISPUTED";
}

/// 应付核销状态（ap_verification.verification_status，大写值）
/// 批次 236 v13 真实接入：ap_verification_service.rs
pub mod ap_verification {
    /// 已完成：核销完成
    pub const COMPLETED: &str = "COMPLETED";

    /// 已取消：核销已取消
    pub const CANCELLED: &str = "CANCELLED";
}

/// 固定资产状态（fixed_asset.status / fixed_asset_depreciation.status）
/// 批次 236 v13 真实接入：fixed_asset_service.rs
pub mod fixed_asset {
    /// 已处置：固定资产已处置（小写）
    pub const DISPOSED: &str = "disposed";

    /// 已完成：折旧已完成（大写）
    pub const DEPRECIATION_COMPLETED: &str = "COMPLETED";
}

/// 成本归集状态（cost_collection.status，小写值）
/// 批次 236 v13 真实接入：cost_collection_service.rs
pub mod cost_collection {
    /// 草稿：成本归集初始状态
    pub const DRAFT: &str = "draft";
}

/// 会计期间补充状态（accounting_period.status，大写值，补充批次 232 的 OPEN/CLOSED）
/// 批次 236 v13 真实接入：missing_handlers.rs
pub mod accounting_period_closing {
    /// 关账中：会计期间正在关账
    pub const CLOSING: &str = "CLOSING";
}
