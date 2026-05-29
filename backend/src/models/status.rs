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
#[allow(dead_code)]
pub mod sales_order {
    /// 草稿
    pub const DRAFT: &str = "DRAFT";
    /// 待审批
    pub const PENDING_APPROVAL: &str = "PENDING_APPROVAL";
    /// 已确认
    pub const CONFIRMED: &str = "CONFIRMED";
    /// 已取消
    pub const CANCELLED: &str = "CANCELLED";
    /// 已完成
    pub const COMPLETED: &str = "COMPLETED";
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
