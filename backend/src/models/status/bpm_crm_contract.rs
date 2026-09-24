#![allow(dead_code)]
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

/// 预算管理状态常量（小写值，批次 209 P2-5 修复，状态机 draft → rejected / approved → active）
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

/// 合同状态常量（小写值，批次 210 P2-5，状态机 draft→active→cancelled）
pub mod contract {
    /// 草稿：合同初始状态，可编辑
    pub const DRAFT: &str = "draft";

    /// 活跃：合同已激活，可执行
    pub const ACTIVE: &str = "active";

    /// 已取消：合同作废
    pub const CANCELLED: &str = "cancelled";
}

/// 运单状态常量（大写值，状态机 IN_TRANSIT→DELIVERED→SIGNED，SIGNED 触发 AR 应收确认）
pub mod logistics_waybill {
    /// 运输中：运单已创建，货物在途
    pub const IN_TRANSIT: &str = "IN_TRANSIT";

    /// 已送达：货物已送达目的地
    pub const DELIVERED: &str = "DELIVERED";

    /// V15 P0-B13：已签收：客户已签收，触发 AR 应收确认
    pub const SIGNED: &str = "SIGNED";

    /// 全部合法运单状态，入参校验的唯一取值来源
    pub const ALL: &[&str] = &[IN_TRANSIT, DELIVERED, SIGNED];
}

/// 物流轨迹事件类型（logistics_tracking_events.event_type）
///
/// 与上面的 `logistics_waybill` 是两套词表：事件是轨迹上的一个点（小写码），
/// 状态是运单当前所处阶段（大写码），二者不可互相赋值。
/// 该列此前是自由文本且写入无校验，任意写法都能进轨迹，界面只能显示码原文。
pub mod logistics_event_type {
    /// 提货：承运方已取货
    pub const PICKUP: &str = "pickup";

    /// 运输中：货物在干线/支线运输途中
    pub const IN_TRANSIT: &str = "in_transit";

    /// 到达：货物已到达目的网点
    pub const ARRIVED: &str = "arrived";

    /// 签收：收货人已签收
    pub const DELIVERED: &str = "delivered";

    /// 全部合法事件类型，入参校验的唯一取值来源
    pub const ALL: &[&str] = &[PICKUP, IN_TRANSIT, ARRIVED, DELIVERED];
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
///
/// 漏斗：new → contacted → qualified → converted / lost / pool；
/// assigned 为分配/认领流转的归属态（crm/assign.rs 自动分配与认领写入）。
pub mod crm_lead {
    /// 新线索：刚创建的线索
    pub const NEW: &str = "new";

    /// 已联系：已与线索建立初步沟通
    pub const CONTACTED: &str = "contacted";

    /// 已确认资格：线索需求/资质已确认，具备转化价值
    pub const QUALIFIED: &str = "qualified";

    /// 已分配：线索经自动分配/认领已归属到销售（分配流转中间态）
    pub const ASSIGNED: &str = "assigned";

    /// 已转化：线索已转化为商机
    pub const CONVERTED: &str = "converted";

    /// 客户池：线索已进入客户池
    pub const POOL: &str = "pool";

    /// 已流失：线索已丢失
    pub const LOST: &str = "lost";

    /// 全部合法线索状态，入参校验与 DB CHECK（chk_crm_lead_lead_status）的唯一取值来源
    pub const ALL: &[&str] = &[NEW, CONTACTED, QUALIFIED, ASSIGNED, CONVERTED, POOL, LOST];
}

/// CRM 商机状态与阶段常量（opportunity_status / opportunity_stage，大写值）
/// 批次 236 v13 真实接入：crm/opp.rs 中商机状态、阶段字符串字面量统一引用此模块（规则 0）
///
/// 两套词表共用本模块：
/// - opportunity_status：OPEN / CLOSED_WON / CLOSED_LOST（无 pool 公海态、无 draft/cancelled，
///   公海机制仅存在于 crm_lead.lead_status）
/// - opportunity_stage：QUALIFICATION → NEEDS_ANALYSIS → PROPOSAL → NEGOTIATION，
///   终态与 status 共用 CLOSED_WON / CLOSED_LOST
pub mod crm_opportunity {
    /// 进行中：商机打开，尚未关闭（opportunity_status）
    pub const OPEN: &str = "OPEN";

    /// 赢单：商机已赢（opportunity_status 与 opportunity_stage 终态共用）
    pub const CLOSED_WON: &str = "CLOSED_WON";

    /// 输单：商机已输（opportunity_status 与 opportunity_stage 终态共用）
    pub const CLOSED_LOST: &str = "CLOSED_LOST";

    /// 阶段-初步接洽：资质确认
    pub const QUALIFICATION: &str = "QUALIFICATION";

    /// 阶段-需求分析
    pub const NEEDS_ANALYSIS: &str = "NEEDS_ANALYSIS";

    /// 阶段-方案报价
    pub const PROPOSAL: &str = "PROPOSAL";

    /// 阶段-谈判议价
    pub const NEGOTIATION: &str = "NEGOTIATION";

    /// 全部合法商机阶段，DB CHECK（chk_crm_opportunity_stage）取值来源
    pub const ALL_STAGES: &[&str] = &[
        QUALIFICATION,
        NEEDS_ANALYSIS,
        PROPOSAL,
        NEGOTIATION,
        CLOSED_WON,
        CLOSED_LOST,
    ];

    /// 全部合法商机状态，DB CHECK（chk_crm_opportunity_status）取值来源
    pub const ALL_STATUSES: &[&str] = &[OPEN, CLOSED_WON, CLOSED_LOST];
}

/// 合同状态（sales_contract.status / purchase_contract.status，小写值）
/// 批次 236 v13 真实接入：sales_contract_service.rs、purchase_contract_service.rs 等
pub mod contract_status {
    /// 草稿：合同初始状态
    pub const DRAFT: &str = "draft";

    /// 已取消：合同已取消
    pub const CANCELLED: &str = "cancelled";
}
