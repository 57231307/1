//! 生产状态常量分组
//!
//! 批次 490 D10-3b 拆分：从 models/status.rs 抽取的生产/排程/MRP/流转卡状态常量子模块组。
//! 包含：production/scheduling/process_node/mrp/work_center/flow_card/step_record

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

/// 排程结果状态（scheduling_result.status，大写值）
/// 批次 234 v13 真实接入：scheduling_query.rs 中排程结果状态字符串字面量统一引用此模块（规则 0）
pub mod scheduling {
    /// 草稿：排程结果初始状态，可确认
    pub const DRAFT: &str = "DRAFT";

    /// 已确认：排程结果已确认，已应用到生产订单
    pub const CONFIRMED: &str = "CONFIRMED";
}

/// 定制订单流程节点状态（process_node.status，小写值）
/// 批次 234 v13 真实接入：custom_order_state_service.rs 中节点状态字符串字面量统一引用此模块（规则 0）
pub mod process_node {
    /// 进行中：节点正在执行
    pub const IN_PROGRESS: &str = "in_progress";

    /// 已完成：节点执行完毕
    pub const COMPLETED: &str = "completed";
}

/// MRP 结果状态（mrp_result.status，大写值）
/// 批次 235 v13 真实接入：mrp_engine_service.rs 中 MRP 结果状态字符串字面量统一引用此模块（规则 0）
pub mod mrp {
    /// 已计划：MRP 计划生成，待发布
    pub const PLANNED: &str = "PLANNED";

    /// 已发布：MRP 计划已发布为生产订单
    pub const RELEASED: &str = "RELEASED";

    /// 已确认：MRP 计划已确认（采购订单类型）
    pub const CONFIRMED: &str = "CONFIRMED";

    /// 已取消：MRP 计划已取消
    pub const CANCELLED: &str = "CANCELLED";
}

/// 工作中心状态（work_center.status，大写值，复用 active_status）
/// 产能负载项状态（capacity_load_item.status，大写值）
/// 批次 236 v13 真实接入：capacity_service.rs、scheduling_auto.rs 等
pub mod work_center {
    /// 空闲：产能负载项空闲
    pub const LOAD_IDLE: &str = "IDLE";

    /// 过载：产能负载项过载
    pub const LOAD_OVERLOADED: &str = "OVERLOADED";
}

/// 流转卡状态（production_flow_card.status，小写值）
/// v14 批次 425 真实业务常量化
/// 依据：面料行业真实业务调研文档 §12.1 流转卡 + §12.7 缸号状态机
/// 状态机：pending → scheduled → preparing → dyeing → dyed → inspecting → completed → shipped / terminated
pub mod flow_card {
    /// 待排缸：流转卡已生成，等待排缸
    pub const PENDING: &str = "pending";

    /// 已排缸：已分配缸位，可变更/合缸/终止
    pub const SCHEDULED: &str = "scheduled";

    /// 备布中：从坯布仓库领坯备布，坯布自动出库
    pub const PREPARING: &str = "preparing";

    /// 染色中：进缸染色，采集生产进度
    pub const DYEING: &str = "dyeing";

    /// 已出缸：染色完成出缸，待验布
    pub const DYED: &str = "dyed";

    /// 验布中：验布打卷，质检分级
    pub const INSPECTING: &str = "inspecting";

    /// 已完成：验布入库完成
    pub const COMPLETED: &str = "completed";

    /// 已发货：成品已发货
    pub const SHIPPED: &str = "shipped";

    /// 已终止：异常终止（合缸/缸变更/取消）
    pub const TERMINATED: &str = "terminated";
}

/// 工序流转记录状态（process_step_record.status，小写值）
/// v14 批次 425 真实业务常量化
/// 依据：面料行业真实业务调研文档 §12.3 车间工序流转
/// 状态机：pending → in_progress → completed / abnormal / rework
pub mod step_record {
    /// 待开始：工序待扫码开始
    pub const PENDING: &str = "pending";

    /// 进行中：工序已扫码开始，未扫码结束
    pub const IN_PROGRESS: &str = "in_progress";

    /// 已完成：工序已扫码结束，产量已确认
    pub const COMPLETED: &str = "completed";

    /// 异常：工序出现异常，需开工序质量反馈单
    pub const ABNORMAL: &str = "abnormal";

    /// 回修：回修工序，关联原工序记录
    pub const REWORK: &str = "rework";
}
