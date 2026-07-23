//! 质量/染色状态常量分组
//!
//! 批次 490 D10-3b 拆分：从 models/status.rs 抽取的质量/染色/验布/缸号生命周期状态常量子模块组。
//! 包含：quality_standard/quality_handling/dye_recipe/lab_dip_request/lab_dip_sample/lab_dip_resample/production_recipe/production_recipe_addition/quality_feedback/fabric_inspection/fabric_scoring/fabric_grade/dye_batch_lifecycle_status/dye_batch_transition_code/dye_batch_rework_type/dye_batch_rework_status/dye_batch_operation_type

/// 质量标准状态（quality_standard.status，小写值）
/// 批次 236 v13 真实接入：quality_standard_service.rs
pub mod quality_standard {
    /// 草稿：质量标准初始状态
    pub const DRAFT: &str = "draft";

    /// 已审批：质量标准已审批
    pub const APPROVED: &str = "approved";

    /// 已拒绝：质量标准被拒绝
    pub const REJECTED: &str = "rejected";
}

/// 质量检验处理状态（quality_inspection_unqualified.handling_status，小写值）
/// 批次 236 v13 真实接入：quality_inspection_service.rs
pub mod quality_handling {
    /// 待处理：不合格品待处理
    pub const PENDING: &str = "pending";
}

/// 染色配方状态（dye_recipe.status，中文值）
/// v14 批次 423A 常量化：原 handler 硬编码中文字符串，现统一常量
/// 依据：面料行业真实业务调研文档 §11.1 化验室打样流程
pub mod dye_recipe {
    /// 草稿：配方初始状态
    pub const DRAFT: &str = "草稿";

    /// 已审核：配方已审核通过
    pub const APPROVED: &str = "已审核";

    /// 已停用：配方已停用
    pub const DISABLED: &str = "已停用";
}

/// 化验室打样通知单状态（lab_dip_request.status，小写值）
/// v14 批次 423B 真实业务常量化
/// 依据：面料行业真实业务调研文档 §11.1 化验室打样 5 步闭环
/// 状态机：pending → sampling → submitted → approved/rejected → completed
pub mod lab_dip_request {
    /// 待打样：通知单已下达，待技术科安排打样
    pub const PENDING: &str = "pending";

    /// 打样中：技术科正在打 ABCD 多版小样
    pub const SAMPLING: &str = "sampling";

    /// 已送客户：小样已送客户等待确认
    pub const SUBMITTED: &str = "submitted";

    /// OK 样确认：客户已选中 OK 样
    pub const APPROVED: &str = "approved";

    /// 重打：客户要求重打
    pub const REJECTED: &str = "rejected";

    /// 已建库：复样通过，处方已升级为大货模板入库
    pub const COMPLETED: &str = "completed";
}

/// 打样小样对色结果（lab_dip_sample.matching_result，小写值）
/// v14 批次 423B 真实业务常量化
/// 依据：面料行业真实业务调研文档 §11.1 对色规范（0/45 度观察，色差 4-5 级为 OK）
pub mod lab_dip_sample {
    /// 待对色：小样刚打出，尚未对色
    pub const PENDING: &str = "pending";

    /// 对色 OK：色差达 4-5 级
    pub const MATCHED: &str = "matched";

    /// 不匹配：色差低于 4 级，需重打
    pub const NOT_MATCHED: &str = "not_matched";

    /// 客户选中 OK 样：客户从多版中选中此版作为 OK 样
    pub const SELECTED: &str = "selected";
}

/// 复样结果状态（lab_dip_resample.result，小写值）
/// v14 批次 423B 真实业务常量化
/// 依据：面料行业真实业务调研文档 §11.1 复样（大货前验证，色差 4-5 级方可投产）
pub mod lab_dip_resample {
    /// 待复样：复样任务已创建，待执行
    pub const PENDING: &str = "pending";

    /// 复样通过：色差达 4-5 级，可投产
    pub const PASSED: &str = "passed";

    /// 复样失败：色差低于 4 级，不可投产
    pub const FAILED: &str = "failed";

    /// 需调整重试：处方需加成/冲减调整后重新复样
    pub const ADJUSTED: &str = "adjusted";
}

/// 大货处方状态（production_recipe.status，小写值）
/// v14 批次 424 真实业务常量化
/// 依据：面料行业真实业务调研文档 §11.2 大货处方（染色配料单）
/// 状态机：draft → approved → closed → cancelled
pub mod production_recipe {
    /// 草稿：大货处方初始状态，可编辑/删除
    pub const DRAFT: &str = "draft";

    /// 已审核：处方已审核，自动建立生产领用单据，不可再编辑
    pub const APPROVED: &str = "approved";

    /// 已关闭：生产完成，处方归档
    pub const CLOSED: &str = "closed";

    /// 已取消：草稿状态作废
    pub const CANCELLED: &str = "cancelled";
}

/// 加料处方状态（production_recipe_addition.status，小写值）
/// v14 批次 424 真实业务常量化
/// 依据：面料行业真实业务调研文档 §11.2 加料处方（染色补料单）
/// 状态机：draft → approved → closed
pub mod production_recipe_addition {
    /// 草稿：加料处方初始状态
    pub const DRAFT: &str = "draft";

    /// 已审核：加料处方已审核
    pub const APPROVED: &str = "approved";

    /// 已关闭：加料完成，处方归档
    pub const CLOSED: &str = "closed";
}

/// 工序质量反馈单状态（process_quality_feedback.status，小写值）
/// v14 批次 425 真实业务常量化
/// 依据：面料行业真实业务调研文档 §12.3 工序质量反馈单
/// 状态机：pending → processing → resolved → closed
pub mod quality_feedback {
    /// 待处理：反馈单已登记，待处理
    pub const PENDING: &str = "pending";

    /// 处理中：正在处理
    pub const PROCESSING: &str = "processing";

    /// 已解决：处理完成
    pub const RESOLVED: &str = "resolved";

    /// 已关闭：已关闭归档
    pub const CLOSED: &str = "closed";
}

/// 验布记录状态（fabric_inspection_record.status，小写值）
/// v14 批次 426 真实业务常量化
/// 依据：面料行业真实业务调研文档 §12.4 验布打卷与成品入库
/// 状态机：pending → inspecting → graded → rolled → closed
pub mod fabric_inspection {
    /// 待验布：验布记录已创建，等待开始验布
    pub const PENDING: &str = "pending";

    /// 验布中：验布机正在检验，采集疵点中
    pub const INSPECTING: &str = "inspecting";

    /// 已评级：验布完成，已计算总扣分/每百平方码分数/等级
    pub const GRADED: &str = "graded";

    /// 已打卷：已生成成品布卷（inventory_piece），待入库
    pub const ROLLED: &str = "rolled";

    /// 已关闭：已入库归档
    pub const CLOSED: &str = "closed";
}

/// 验布评分制式（fabric_inspection_record.scoring_system，小写值）
/// v14 批次 426 真实业务常量化
/// 依据：AATCC 检验标准 / ASTM D5430 / 面料验货基础知识
pub mod fabric_scoring {
    /// 四分制：AATCC/ASTM D5430，针织+梭织通用
    /// 疵点长度 ≤3寸=1分, 3-6寸=2分, 6-9寸=3分, >9寸=4分，破洞/连续=4分
    /// 等级：每百平方码分数 ≤40 = 首级(first)，>40 = 次级(second)
    pub const FOUR_POINT: &str = "four_point";

    /// 十分制：梭织布专用
    /// 经向：1寸下=1/1-5寸=3/5-10寸=5/10-36寸=10
    /// 纬向：1寸下=1/1-5寸=3/5寸-半门幅=5/半门幅上=10，破洞=10
    /// 等级：总扣分 < 总码数 = 首级(first)，≥ 总码数 = 次级(second)
    pub const TEN_POINT: &str = "ten_point";
}

/// 验布等级（fabric_inspection_record.grade，小写值）
/// v14 批次 426 真实业务常量化
/// 依据：面料检验"四分制"与"十分制"的异同点
pub mod fabric_grade {
    /// 首级：合格品，可正常入库销售
    pub const FIRST: &str = "first";

    /// 次级：不合格品，需降级销售或返工
    pub const SECOND: &str = "second";
}

/// v14 批次 432：缸号全生命周期状态机
///
/// 依据：面料行业真实业务调研文档 §12.7 缸号状态机 + §3.2 缸号全生命周期追踪
/// 14 种状态：待排缸→已排缸→备布中→进缸染色→皂洗→固色→脱水→烘干→验布→入库→发货 + 取消/终止/回修
/// 终态：shipped 发货 / cancelled 取消 / terminated 终止
/// 回修：rework 可回到 dyeing 重新进缸
pub mod dye_batch_lifecycle_status {
    /// 待排缸：缸号已创建，等待排缸
    pub const PENDING_SCHEDULE: &str = "pending_schedule";
    /// 已排缸：已分配缸位，可变更/合缸/终止
    pub const SCHEDULED: &str = "scheduled";
    /// 备布中：从坯布仓库领坯备布
    pub const PREPARING: &str = "preparing";
    /// 进缸染色：投缸染色，采集生产进度
    pub const DYEING: &str = "dyeing";
    /// 皂洗：染色后皂洗工序
    pub const WASHING: &str = "washing";
    /// 固色：皂洗后固色工序
    pub const FIXING: &str = "fixing";
    /// 脱水：固色后脱水工序
    pub const DEHYDRATING: &str = "dehydrating";
    /// 烘干：脱水后烘干工序
    pub const DRYING: &str = "drying";
    /// 验布：烘干后验布打卷
    pub const INSPECTING: &str = "inspecting";
    /// 入库：验布完成入库
    pub const STORED: &str = "stored";
    /// 发货：成品已发货（终态）
    pub const SHIPPED: &str = "shipped";
    /// 取消：缸号作废（终态）
    pub const CANCELLED: &str = "cancelled";
    /// 终止：异常终止（终态）
    pub const TERMINATED: &str = "terminated";
    /// 回修中：回修订单重新进缸，可回到 dyeing
    pub const REWORK: &str = "rework";
}

/// v14 批次 432：缸号全生命周期状态机
///
/// 缸号流转操作代码（dye_batch_lifecycle_log.transition_code / dye_batch_state_rule.transition_code）
pub mod dye_batch_transition_code {
    /// 排缸：pending_schedule → scheduled
    pub const SCHEDULE: &str = "schedule";
    /// 备布：scheduled → preparing
    pub const PREPARE: &str = "prepare";
    /// 进缸染色：preparing → dyeing 或 rework → dyeing
    pub const START_DYEING: &str = "start_dyeing";
    /// 皂洗：dyeing → washing
    pub const WASH: &str = "wash";
    /// 固色：washing → fixing
    pub const FIX: &str = "fix";
    /// 脱水：fixing → dehydrating
    pub const DEHYDRATE: &str = "dehydrate";
    /// 烘干：dehydrating → drying
    pub const DRY: &str = "dry";
    /// 验布：drying → inspecting
    pub const INSPECT: &str = "inspect";
    /// 入库：inspecting → stored
    pub const STORE: &str = "store";
    /// 发货：stored → shipped（终态）
    pub const SHIP: &str = "ship";
    /// 取消：任意非终态 → cancelled（终态）
    pub const CANCEL: &str = "cancel";
    /// 回修：inspecting/stored → rework
    pub const REWORK: &str = "rework";
    /// 终止：scheduled/preparing/dyeing/rework → terminated（终态）
    pub const TERMINATE: &str = "terminate";
}

/// v14 批次 432：缸号全生命周期状态机
///
/// 缸号回修类型（dye_batch_rework.rework_type）
pub mod dye_batch_rework_type {
    /// 色差：染色色差超允许范围，需回修调色
    pub const COLOR_DIFFERENCE: &str = "color_difference";
    /// 疵点：布面疵点超允许范围，需回修修补
    pub const DEFECT: &str = "defect";
    /// 规格不符：门幅/克重/纱支等规格不符，需回修调整
    pub const SPECIFICATION_UNQUALIFIED: &str = "specification_unqualified";
    /// 其他：其他原因回修
    pub const OTHER: &str = "other";
}

/// v14 批次 432：缸号全生命周期状态机
///
/// 缸号回修单状态（dye_batch_rework.status）
/// 状态机：draft 草稿 → approved 已审批 → in_progress 回修中 → completed 已完成 / cancelled 已取消
pub mod dye_batch_rework_status {
    /// 草稿：回修单初始状态，可编辑
    pub const DRAFT: &str = "draft";
    /// 已审批：审批通过，可开始回修
    pub const APPROVED: &str = "approved";
    /// 回修中：回修进行中
    pub const IN_PROGRESS: &str = "in_progress";
    /// 已完成：回修完成
    pub const COMPLETED: &str = "completed";
    /// 已取消：回修单作废
    pub const CANCELLED: &str = "cancelled";
}

/// v14 批次 432：缸号全生命周期状态机
///
/// 缸号操作类型（dye_batch_operation.operation_type）
pub mod dye_batch_operation_type {
    /// 合缸：多个缸号合并为一个缸号
    pub const MERGE: &str = "merge";
    /// 分缸：一个缸号拆分为多个缸号
    pub const SPLIT: &str = "split";
    /// 优先级调整：调整缸号优先级
    pub const PRIORITY_ADJUST: &str = "priority_adjust";
    /// 缸变更：变更缸号所属缸位
    pub const BATCH_CHANGE: &str = "batch_change";
    /// 计划变更：变更生产计划
    pub const SCHEDULE_CHANGE: &str = "schedule_change";
    /// 终止：终止缸号生产
    pub const TERMINATE: &str = "terminate";
}
