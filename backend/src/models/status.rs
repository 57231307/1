/// 通用状态常量定义
/// 用于避免状态字符串硬编码
// 采购订单状态
#[allow(dead_code)]
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

// 通用审批状态
#[allow(dead_code)]
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
