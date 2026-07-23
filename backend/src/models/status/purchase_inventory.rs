//! 采购库存状态常量分组
//!
//! 批次 490 D10-3b 拆分：从 models/status.rs 抽取的采购/库存状态常量子模块组。
//! 包含：purchase_order/purchase_receipt/inventory_reservation/inventory_transfer/inventory_count/purchase_return/purchase_inspection/inventory_adjustment/inventory_piece/purchase_receipt_inspection

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
    /// 已关闭
    pub const CLOSED: &str = "CLOSED";
    /// 已取消（批次 215 真实接入：cancel_order 方法使用）
    pub const CANCELLED: &str = "CANCELLED";
    /// 已完成
    pub const COMPLETED: &str = "COMPLETED";
    /// 部分收货
    pub const PARTIAL_RECEIVED: &str = "PARTIAL_RECEIVED";
}

/// 采购收货单状态常量（purchase_receipt.receipt_status，大写值）
///
/// 批次 214 P2-1 修复（v12 复审）：抽取 purchase_receipt_service.rs 和 po/receipt.rs 中的硬编码状态字符串
/// 状态机：DRAFT → CONFIRMED → COMPLETED
pub mod purchase_receipt {
    /// 草稿：收货单初始状态，可编辑
    pub const DRAFT: &str = "DRAFT";
    /// 已确认：收货已确认，等待入库
    pub const CONFIRMED: &str = "CONFIRMED";
    /// 已完成：收货入库流程完成（幂等键）
    pub const COMPLETED: &str = "COMPLETED";
}

// 库存预留状态（inventory_reservation.status，小写值）
// 批次 158 v11 真实接入：so/delivery.rs 中库存预留状态字符串字面量统一引用此模块（规则 0）
// 批次 341 v11 复审 P2 修复：LOCKED/RELEASED 常量已被 inventory_reservation_service.rs 和测试代码广泛使用，
// 移除过时的 #[allow(dead_code)] 抑制（lock_reservation/release_reservation 方法已真实实现）。
pub mod inventory_reservation {
    /// 待处理（已创建预留，等待发货扣减）
    pub const PENDING: &str = "pending";
    /// 已锁定（库存已锁定，等待发货扣减）
    pub const LOCKED: &str = "locked";
    /// 已消耗（发货已扣减库存，原 FULFILLED 值修正为 consumed 与业务代码一致）
    pub const CONSUMED: &str = "consumed";
    /// 已释放（订单取消或库存不足释放）
    pub const RELEASED: &str = "released";
    /// 已取消（订单取消或库存不足释放）
    pub const CANCELLED: &str = "cancelled";
}

/// 库存调拨状态（inventory_transfer.status，小写值）
/// 批次 234 v13 真实接入：inv/inventory_move.rs 中调拨状态字符串字面量统一引用此模块（规则 0）
pub mod inventory_transfer {
    /// 待处理：调拨单初始状态，可审批
    pub const PENDING: &str = "pending";

    /// 已审批：审批通过，可发货
    pub const APPROVED: &str = "approved";

    /// 已拒绝：审批未通过
    pub const REJECTED: &str = "rejected";

    /// 已发货：调拨已发出，待接收
    pub const SHIPPED: &str = "shipped";

    /// 已完成：调拨流程完结
    pub const COMPLETED: &str = "completed";
}

/// 库存盘点状态（inventory_count.status，小写值）
/// 批次 234 v13 真实接入：inventory_count_service.rs 中盘点状态字符串字面量统一引用此模块（规则 0）
pub mod inventory_count {
    /// 待处理：盘点单初始状态，可执行盘点
    pub const PENDING: &str = "pending";

    /// 已完成：盘点流程完结
    pub const COMPLETED: &str = "completed";
}

/// 采购退货状态（purchase_return.return_status，小写值）
/// 批次 234 v13 真实接入：purchase_return_service.rs 中退货状态字符串字面量统一引用此模块（规则 0）
pub mod purchase_return {
    /// 草稿：退货单初始状态，可编辑
    pub const DRAFT: &str = "draft";

    /// 已提交：等待审批
    pub const SUBMITTED: &str = "submitted";

    /// 已审批：审批通过，可执行退货
    pub const APPROVED: &str = "approved";

    /// 已拒绝：审批未通过
    pub const REJECTED: &str = "rejected";
}

/// 采购检验状态（purchase_inspection.inspection_status，小写值）
/// 批次 234 v13 真实接入：purchase_inspection_service.rs 中检验状态字符串字面量统一引用此模块（规则 0）
pub mod purchase_inspection {
    /// 待处理：检验单初始状态，可执行检验
    pub const PENDING: &str = "pending";

    /// 已完成：检验流程完结
    pub const COMPLETED: &str = "completed";
}

/// 库存调整状态（inventory_adjustment.status，小写值）
/// 批次 234 v13 真实接入：inventory_adjustment_service.rs 中调整状态字符串字面量统一引用此模块（规则 0）
pub mod inventory_adjustment {
    /// 待处理：调整单初始状态，可审批
    pub const PENDING: &str = "pending";

    /// 已审批：审批通过，已应用调整
    pub const APPROVED: &str = "approved";

    /// 已拒绝：审批未通过
    pub const REJECTED: &str = "rejected";
}

/// 库存裁片状态（inventory_piece.status，大写值）
/// 批次 236 v13 真实接入：barcode_scanner_handler.rs、piece_split_handler.rs 中裁片状态字符串字面量统一引用此模块（规则 0）
pub mod inventory_piece {
    /// 已发货：裁片已发货
    pub const SHIPPED: &str = "SHIPPED";

    /// 缺陷：裁片有缺陷
    pub const DEFECT: &str = "DEFECT";

    /// 可用：裁片可用
    pub const AVAILABLE: &str = "AVAILABLE";

    /// 不可用：裁片不可用
    pub const UNAVAILABLE: &str = "UNAVAILABLE";

    /// 已预留：裁片已为订单预留
    pub const RESERVED: &str = "RESERVED";
}

/// 采购收货检验状态（purchase_receipt.inspection_status，大写值）
/// 批次 236 v13 真实接入：purchase_receipt_service.rs
pub mod purchase_receipt_inspection {
    /// 待检验
    pub const PENDING: &str = "PENDING";
}
