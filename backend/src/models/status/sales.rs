//! 销售状态常量分组
//!
//! 批次 490 D10-3b 拆分：从 models/status.rs 抽取的销售/报价/定制订单状态常量子模块组。
//! 包含：sales_order/sales_delivery/sales_return/quotation/custom_order/quotation_ext/price_approval/custom_order_ext/sales_fabric_order

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

// 销售发货状态（sales_delivery.status，小写值）
// 批次 158 v11 真实接入：so/delivery.rs 中发货状态字符串字面量统一引用此模块（规则 0）
pub mod sales_delivery {
    /// 待处理
    pub const PENDING: &str = "pending";
    /// 已发货
    pub const SHIPPED: &str = "shipped";
    /// 已取消（批次 216 真实接入：cancel_delivery 方法使用）
    pub const CANCELLED: &str = "cancelled";
}

/// 销售退货状态常量（大写值）
///
/// 批次 232 v13 P1-1 修复：sales_return.status 字段状态常量化
/// 状态机：DRAFT → SUBMITTED → APPROVED → COMPLETED / REJECTED
pub mod sales_return {
    /// 草稿：退货单初始状态，可编辑
    pub const DRAFT: &str = "DRAFT";

    /// 已提交：等待审批
    pub const SUBMITTED: &str = "SUBMITTED";

    /// 已审批：审批通过，可执行退货
    pub const APPROVED: &str = "APPROVED";

    /// 已拒绝：审批未通过
    pub const REJECTED: &str = "REJECTED";

    /// 已完成：退货流程完结
    pub const COMPLETED: &str = "COMPLETED";
}

/// 报价单状态（quotation.status，小写值）
/// 批次 234 v13 真实接入：quotation_service.rs 中报价状态字符串字面量统一引用此模块（规则 0）
pub mod quotation {
    /// 草稿：报价单初始状态，可编辑
    pub const DRAFT: &str = "draft";

    /// 已审批：审批通过
    pub const APPROVED: &str = "approved";

    /// 已拒绝：审批未通过
    pub const REJECTED: &str = "rejected";

    /// 已取消：报价单作废
    pub const CANCELLED: &str = "cancelled";
}

/// 定制订单状态（custom_order.status，小写值）
/// 批次 234 v13 真实接入：custom_order_crud_service.rs 中订单状态字符串字面量统一引用此模块（规则 0）
pub mod custom_order {
    /// 草稿：订单初始状态，可编辑
    pub const DRAFT: &str = "draft";

    /// 待处理：等待排产
    pub const PENDING: &str = "pending";

    /// 已完成：订单流程完结
    pub const COMPLETED: &str = "completed";

    /// 已取消：订单作废
    pub const CANCELLED: &str = "cancelled";
}

/// 报价单状态（sales_quotation.status，小写值，补充批次 234 的 quotation 模块）
/// 批次 236 v13 真实接入：quotation_approval_service.rs、quotation_convert_service.rs 等
/// 注意：quotation 模块（批次 234）已定义 DRAFT/APPROVED/REJECTED/CANCELLED，
/// 本模块补充审批/转换流程专属状态
pub mod quotation_ext {
    /// 待审批：报价单已提交审批
    pub const PENDING_APPROVAL: &str = "pending_approval";

    /// 已过期：报价单已过期
    pub const EXPIRED: &str = "expired";

    /// 已转化：报价单已转化为销售订单
    pub const CONVERTED: &str = "converted";
}

/// 价格审批状态（sales_price.status / purchase_price.status，小写值）
/// 批次 236 v13 真实接入：sales_price_service.rs、purchase_price_service.rs
pub mod price_approval {
    /// 待审批：价格待审批
    pub const PENDING: &str = "pending";

    /// 已审批：价格已审批
    pub const APPROVED: &str = "approved";
}

/// 定制订单售后/质量/工序状态（小写值）
/// 批次 236 v13 真实接入：custom_order_quality_service.rs、custom_order_aftersales_service.rs、custom_order_process_service.rs
pub mod custom_order_ext {
    /// 质量问题-开启
    pub const QUALITY_OPEN: &str = "open";

    /// 质量问题-关闭
    pub const QUALITY_CLOSED: &str = "closed";

    /// 质量问题-已解决
    pub const QUALITY_RESOLVED: &str = "resolved";

    /// 售后-已开启
    pub const AFTERSALES_OPENED: &str = "opened";

    /// 售后-已拒绝
    pub const AFTERSALES_REJECTED: &str = "rejected";

    /// 工序-待处理
    pub const PROCESS_PENDING: &str = "pending";
}

/// 销售面料订单状态（sales_fabric_order.status，小写值）
/// 批次 236 v13 真实接入：sales_fabric_order_handler.rs
pub mod sales_fabric_order {
    /// 待处理
    pub const PENDING: &str = "pending";

    /// 已审批
    pub const APPROVED: &str = "approved";
}
