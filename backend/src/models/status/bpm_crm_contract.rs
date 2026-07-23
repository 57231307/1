//! BPM/CRM/合同状态常量分组
//!
//! 批次 490 D10-3b 拆分：从 models/status.rs 抽取的 BPM/CRM/合同/预算/物流状态常量子模块组。
//! 包含：approval/budget/contract/logistics_waybill/bpm_instance/bpm_task/crm_lead/crm_opportunity/contract_status

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

/// 预算管理状态常量（小写值）
///
/// 批次 209 P2-5 修复（v12 复审）：
/// budget_plan.status 与 budget_management.status（预算项目）使用小写状态值，
/// 状态机：draft → rejected / approved → active
pub mod budget {
    /// 草稿：预算方案初始状态，可编辑
    pub const DRAFT: &str = "draft";

    /// 已拒绝：审批未通过
    pub const REJECTED: &str = "rejected";

    /// 已审批：审批通过，等待执行
    pub const APPROVED: &str = "approved";

    /// 执行中：预算方案已激活，预算项目处于活跃状态
    pub const ACTIVE: &str = "active";
}

/// 合同状态常量（小写值）
///
/// 批次 210 P2-5 修复（v12 复审）：
/// sales_contract.status 与 purchase_contract.status 使用小写状态值，
/// 状态机：draft → active → cancelled
pub mod contract {
    /// 草稿：合同初始状态，可编辑
    pub const DRAFT: &str = "draft";

    /// 活跃：合同已激活，可执行
    pub const ACTIVE: &str = "active";

    /// 已取消：合同作废
    pub const CANCELLED: &str = "cancelled";
}

/// 运单状态常量（大写值）
///
/// 批次 232 v13 P1-1 修复：logistics_waybill.status 字段状态常量化
/// 状态机：IN_TRANSIT（运输中）→ DELIVERED（已送达）→ SIGNED（已签收）
///
/// V15 P0-B13（Batch 483）：新增 SIGNED 已签收状态
/// - 签收时调用 sign_waybill handler，状态推进到 SIGNED
/// - SIGNED 状态触发 AR 应收确认（财务做收款计划）
pub mod logistics_waybill {
    /// 运输中：运单已创建，货物在途
    pub const IN_TRANSIT: &str = "IN_TRANSIT";

    /// 已送达：货物已送达目的地
    pub const DELIVERED: &str = "DELIVERED";

    /// V15 P0-B13：已签收：客户已签收，触发 AR 应收确认
    pub const SIGNED: &str = "SIGNED";
}

/// BPM 流程实例状态（bpm_process_instance.status，大写值）
/// 批次 235 v13 真实接入：bpm_service.rs 中流程实例状态字符串字面量统一引用此模块（规则 0）
pub mod bpm_instance {
    /// 处理中：流程实例运行中
    pub const PROCESSING: &str = "PROCESSING";

    /// 已完成：流程实例正常结束
    pub const COMPLETED: &str = "COMPLETED";

    /// 已终止：流程实例被异常终止
    pub const TERMINATED: &str = "TERMINATED";

    /// 已取消：流程实例被取消
    pub const CANCELLED: &str = "CANCELLED";
}

/// BPM 任务状态（bpm_task.status，小写值）
/// 批次 235 v13 真实接入：bpm_service.rs 中任务状态字符串字面量统一引用此模块（规则 0）
pub mod bpm_task {
    /// 待处理：任务待办理
    pub const PENDING: &str = "pending";

    /// 已完成：任务已办理完成
    pub const COMPLETED: &str = "completed";

    /// 已拒绝：任务被拒绝
    pub const REJECTED: &str = "rejected";

    /// 已取消：任务被取消
    pub const CANCELLED: &str = "cancelled";
}

/// CRM 线索状态（crm_lead.lead_status，小写值）
/// 批次 236 v13 真实接入：crm/assign.rs、crm/lead.rs、crm/pool.rs 等线索状态字符串字面量统一引用此模块（规则 0）
pub mod crm_lead {
    /// 新线索：刚创建的线索
    pub const NEW: &str = "new";

    /// 已转化：线索已转化为商机
    pub const CONVERTED: &str = "converted";

    /// 客户池：线索已进入客户池
    pub const POOL: &str = "pool";

    /// 已流失：线索已丢失
    pub const LOST: &str = "lost";
}

/// CRM 商机状态（opportunity.status，大写值）
/// 批次 236 v13 真实接入：crm/opp.rs 中商机状态字符串字面量统一引用此模块（规则 0）
pub mod crm_opportunity {
    /// 赢单：商机已赢
    pub const CLOSED_WON: &str = "CLOSED_WON";

    /// 输单：商机已输
    pub const CLOSED_LOST: &str = "CLOSED_LOST";
}

/// 合同状态（sales_contract.status / purchase_contract.status，小写值）
/// 批次 236 v13 真实接入：sales_contract_service.rs、purchase_contract_service.rs 等
pub mod contract_status {
    /// 草稿：合同初始状态
    pub const DRAFT: &str = "draft";

    /// 已取消：合同已取消
    pub const CANCELLED: &str = "cancelled";
}
