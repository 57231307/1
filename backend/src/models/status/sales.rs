#![allow(dead_code)]
//! 销售状态常量分组
//!
//! A.18.3 状态大小写规范：销售状态统一小写（draft/pending/approved 等），
//! 与 DB CHECK 约束一致。新增代码必须引用本模块常量，禁止字面量（A.19 逐步替换存量）。
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
    /// 全部合法取值（更新订单头时用于取值域校验，禁止把任意字符串写进状态列）
    pub const ALL: &[&str] = &[
        DRAFT,
        PENDING,
        APPROVED,
        PARTIAL_SHIPPED,
        SHIPPED,
        COMPLETED,
        CANCELLED,
        REJECTED,
    ];
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

/// 销售退货状态常量（大写值，批次 232 v13 P1-1，状态机 DRAFT→SUBMITTED→APPROVED→COMPLETED/REJECTED）
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

/// 定制订单状态（custom_orders.status，小写值）——本域唯一权威词表
///
/// 取值域 = 状态机 `utils/process_state_machine.rs::CustomOrderStatus::as_str()`
/// 的 10 个工艺态 + 大额变更挂起态 `change_pending`
/// （`custom_order_crud_service.rs::submit_change_request` 金额变化超阈值写入，
/// `approve_change` 按该值比较），与迁移 m0064 的 `chk_custom_order_status`
/// 取值集合逐项相等（契约锁：`tests/contract_wave5_custom_order_status_unity_test.rs`）。
///
/// 原模块仅 4 token 且 `pending` 为悬空 token（不在 DB CHECK 集内，
/// 实际仅被 process_nodes 借用）：已移除；process_nodes.status 的取值
/// 权威改 `models/status/production.rs::process_node`（含 pending/in_progress/
/// completed/blocked，与其 chk_node_status 一致）。
/// 新增代码必须引用本模块常量，禁止字面量（规则 0）。
pub mod custom_order {
    /// 草稿：订单初始状态，可编辑
    pub const DRAFT: &str = "draft";

    /// 打样中：关联 lab_dip_request，客户确认 OK 样后推进（V15 P0-B11）
    pub const LAB_DIP: &str = "lab_dip";

    /// 报价中：关联 sales_quotation，报价审批通过后推进（V15 P0-B11）
    pub const QUOTATION: &str = "quotation";

    /// 纱线采购中
    pub const YARN_PURCHASING: &str = "yarn_purchasing";

    /// 染整中
    pub const DYEING: &str = "dyeing";

    /// 后整理中
    pub const FINISHING: &str = "finishing";

    /// 交付中
    pub const DELIVERY: &str = "delivery";

    /// 售后中
    pub const AFTER_SALES: &str = "after_sales";

    /// 变更挂起：金额变化超阈值进入二级审批（submit_change_request 写入，
    /// approve_change 状态门按该值比较）
    pub const CHANGE_PENDING: &str = "change_pending";

    /// 已完成：订单流程完结
    pub const COMPLETED: &str = "completed";

    /// 已取消：订单作废
    pub const CANCELLED: &str = "cancelled";

    /// 全部合法取值（与 m0064 CHECK 取值集合逐项相等，双向锁定）
    pub const ALL: &[&str] = &[
        DRAFT,
        LAB_DIP,
        QUOTATION,
        YARN_PURCHASING,
        DYEING,
        FINISHING,
        DELIVERY,
        AFTER_SALES,
        CHANGE_PENDING,
        COMPLETED,
        CANCELLED,
    ];
}

/// 报价单状态（sales_quotation.status 小写，批次 236 v13 补充 quotation 模块审批/转换流程专属状态）
pub mod quotation_ext {
    /// 待审批：报价单已提交审批
    pub const PENDING_APPROVAL: &str = "pending_approval";

    /// 已过期：报价单已过期
    pub const EXPIRED: &str = "expired";

    /// 已转化：报价单已转化为销售订单
    pub const CONVERTED: &str = "converted";
}

/// 价格域权威词表（sales_prices.status / purchase_prices.status，小写值）
///
/// 与迁移 CHECK `chk_sales_price_status` / `chk_purchase_price_status` 取值集逐项相等
/// （契约锁：`tests/contract_wave8_price_status_parity_test.rs`）：
/// - 写入方全集产 pending→approved（销售侧仅这两态，见 `sales_price_service.rs`）；
/// - 采购侧停用态 inactive 由 `purchase_price_service.rs::update_price` 透传写入，
///   `chk_sales_price_status` 是本集合去掉 inactive 的子集。
/// 不设 active/expired 存储态：所谓"当前生效"只在查询侧按 status + 有效期区间计算。
pub mod price_approval {
    /// 待审批：价格待审批
    pub const PENDING: &str = "pending";

    /// 已审批：价格已审批
    pub const APPROVED: &str = "approved";

    /// 已停用：采购侧记录级停用（销售侧无此写入方）
    pub const INACTIVE: &str = "inactive";

    /// 全部合法取值（价格状态入参校验取值域，禁止把任意字符串写进状态列）
    pub const ALL: &[&str] = &[PENDING, APPROVED, INACTIVE];
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

    /// 售后-已受理（V15 P1 batch-19 缺陷 23.3.2：opened → accepted，写 accepted_at）
    pub const AFTERSALES_ACCEPTED: &str = "accepted";

    /// 售后-处理中（accepted → processing → resolved）
    pub const AFTERSALES_PROCESSING: &str = "processing";

    /// 售后-已解决（processing → resolved，写 closed_at 以外的结论态）
    pub const AFTERSALES_RESOLVED: &str = "resolved";

    /// 售后-已评价（V15 P1 batch-19 缺陷 23.3.2：resolved → evaluated，写评价三列）
    pub const AFTERSALES_EVALUATED: &str = "evaluated";

    /// 售后-已关闭（终态，任一非终态均可关闭；写 closed_at）
    pub const AFTERSALES_CLOSED: &str = "closed";

    /// 售后工单状态全部合法取值（写入方 `services/custom_order_aftersales_service.rs`
    /// 状态机 `AFTERSALES_TRANSITIONS` 的节点集，与其逐 token 相等）。
    ///
    /// ⚠️ DB 侧 CHECK `chk_aftersales_status`（migration
    /// `m0044_integrate_unreferenced_migrations.rs:251`）当前只覆盖
    /// opened/processing/resolved/closed/rejected，缺 `accepted`/`evaluated`
    /// ⇒ 写这两态必撞 CHECK 返回裸 500（CI #4669 用例 65-01）。
    /// 迁移须把本列表补齐进 CHECK（三端同源：写入方词表 = 本列表 = CHECK 取值集）。
    pub const AFTERSALES_ALL: &[&str] = &[
        AFTERSALES_OPENED,
        AFTERSALES_ACCEPTED,
        AFTERSALES_PROCESSING,
        AFTERSALES_RESOLVED,
        AFTERSALES_EVALUATED,
        AFTERSALES_CLOSED,
        AFTERSALES_REJECTED,
    ];

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
