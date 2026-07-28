#![allow(dead_code)]
//! 财务状态常量分组
//!
//! 批次 490 D10-3b 拆分：从 models/status.rs 抽取的财务/应收应付/凭证/会计期间状态常量子模块组。
//! 包含：ar/ap_invoice/ap_payment_request/voucher/accounting_period/finance_invoice/finance_payment/ap_reconciliation/ap_verification/fixed_asset/cost_collection/accounting_period_closing

/// 应收账款专属状态常量（批次 231 统一小写：ar_collection pending/confirmed/cancelled、ar_reconciliation draft/sent/confirmed/disputed/closed/cancelled、ar_reconciliation_item MATCHED/UNMATCHED；ar_invoice 复用 common 与 payment 常量）
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

/// 应付发票专属状态常量（批次 102 v6 P3-2：ap_invoice.invoice_status 大写，仅 AUDITED 单独定义，DRAFT/PAID/PARTIAL_PAID/CANCELLED 复用 common/payment）
pub mod ap_invoice {
    /// 已审核（ap_invoice 专属状态，区别于通用 APPROVED）
    pub const INVOICE_AUDITED: &str = "AUDITED";
}

/// 应付付款申请专属审批状态常量（批次 102 v6 P3-3：ap_payment_request.approval_status 大写，APPROVING/REJECTED 单独定义，DRAFT/APPROVED 复用 common）
pub mod ap_payment_request {
    /// 审批中（ap_payment_request 专属状态）
    pub const APPROVAL_APPROVING: &str = "APPROVING";

    /// 已拒绝（避免依赖 dead_code 的 approval::REJECTED，单独定义）
    pub const APPROVAL_REJECTED: &str = "REJECTED";
}

/// 凭证状态常量（批次 102 v6 P3-1：voucher.status 小写，状态机 draft→submitted→reviewed→posted，与 ar/ap_invoice 大写不同）
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

/// 会计期间状态常量（大写值，批次 232 v13 P1-1，状态机 OPEN→CLOSED）
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
