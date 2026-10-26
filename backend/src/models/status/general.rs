#![allow(dead_code)]
//! 通用状态常量分组
//!
//! 批次 490 D10-3b 拆分：从 models/status.rs 抽取的通用/系统级业务状态常量子模块组。
//! 包含：common/payment/master_data/import_task/login_log/email_log/active_status/audit_message/health_check/reconcile_result/failover
//!
//! 大小写历史包袱说明：本模块下 status 字面量大小写混杂，源于历史数据库迁移遗留——
//! 主数据/导入任务/健康检查/故障切换等早期表用小写（active/inactive/pending），
//! 单据/日志/审计等后建表用大写（DRAFT/PENDING/APPROVED）。
//! 各子模块内字面量值保持与数据库现状一致，common/active_status 用大写、master_data 用小写，
//! 新增状态需先确认对应表的既有值大小写再定义常量，避免大小写不匹配导致查询失效。

/// 通用状态常量（多业务共用）
pub mod common {
    /// 草稿：单据初始状态，可编辑/删除
    pub const STATUS_DRAFT: &str = "DRAFT";

    /// 待处理：通用待办或审批中状态
    pub const STATUS_PENDING: &str = "PENDING";

    /// 已审批：审批流程通过
    pub const STATUS_APPROVED: &str = "APPROVED";

    /// 已取消：单据作废，不可再变更
    pub const STATUS_CANCELLED: &str = "CANCELLED";

    /// 已完成：业务流程完结
    pub const STATUS_COMPLETED: &str = "COMPLETED";

    /// 激活：主数据或库存可用状态
    pub const STATUS_ACTIVE: &str = "ACTIVE";
}

/// 付款状态常量
pub mod payment {
    /// 已登记：付款单已创建但未确认执行
    pub const PAYMENT_REGISTERED: &str = "REGISTERED";

    /// 已确认：付款已执行，等待银行对账
    pub const PAYMENT_CONFIRMED: &str = "CONFIRMED";

    /// 已付款：全部结清（应收/应付通用复用）
    pub const PAYMENT_PAID: &str = "PAID";

    /// 部分付款：尚未结清（应收/应付通用复用）
    pub const PAYMENT_PARTIAL_PAID: &str = "PARTIAL_PAID";
}

/// 产品类型词表（面料域权威，中文值，与 products.product_type 列现状一致）。
/// 单一定义源，供 CSV 导入枚举校验、产品新建默认、仪表盘面料总数计数共用，禁止各处再硬编中文副本。
pub mod product_type {
    /// 坯布：未经染色整理的布
    pub const GREIGE: &str = "坯布";

    /// 成品布：已染色整理的布
    pub const FINISHED: &str = "成品布";

    /// 辅料：非面料的辅助物料，不计入面料总数
    pub const AUXILIARY: &str = "辅料";

    /// 全量产品类型词表，供导入校验等枚举合法性判定
    pub const ALL_TYPES: &[&str] = &[GREIGE, FINISHED, AUXILIARY];

    /// 面料集合，等于坯布加成品布，仪表盘面料总数与产品面料口径的唯一判定源
    pub const FABRIC_TYPES: &[&str] = &[GREIGE, FINISHED];
}

/// 产品色牢度等级（GB 18401 品色牢度分档，含半级）。落 product_colors.color_fastness_grade。
/// 单一定义源，供取值域校验、CSV 导入枚举校验、DB CHECK 约束与前端下拉词表共用，禁止另抄一份。
/// 半级形态（1-2、2-3、3-4、4-5）是标准定案的一部分，不是脏数据，收口时必须一并放行。
/// 空值语义为未考核或未录入。耐唾液项目只对 A 类考核，本色漂白与扎染蜡染等传统手工着色
/// 产品不考核色牢度，这两条是条件性判据，落在应用层与检验判定侧，不在本词表内表达。
pub mod color_fastness {
    pub const GRADE_1: &str = "1";
    pub const GRADE_1_2: &str = "1-2";
    pub const GRADE_2: &str = "2";
    pub const GRADE_2_3: &str = "2-3";
    pub const GRADE_3: &str = "3";
    pub const GRADE_3_4: &str = "3-4";
    pub const GRADE_4: &str = "4";
    pub const GRADE_4_5: &str = "4-5";
    pub const GRADE_5: &str = "5";

    /// 全量色牢度词表，供导入与入参枚举合法性判定与 DB CHECK 单源比对
    pub const ALL_GRADES: &[&str] = &[
        GRADE_1, GRADE_1_2, GRADE_2, GRADE_2_3, GRADE_3, GRADE_3_4, GRADE_4, GRADE_4_5, GRADE_5,
    ];
}

/// 主数据启用/停用状态（小写值，批次 208 P2-5：supplier/customer/fixed_asset 等用小写 active/inactive，与 common::STATUS_ACTIVE 大写区分）
pub mod master_data {
    /// 启用：主数据可用状态
    pub const ACTIVE: &str = "active";

    /// 停用：主数据不可用状态
    pub const INACTIVE: &str = "inactive";

    /// 待审批：主数据待审批状态（价格审批等，A.19 补充）
    pub const PENDING: &str = "pending";

    /// 已审批：主数据已审批状态（价格审批等，A.19 补充）
    pub const APPROVED: &str = "approved";

    // 以下为 AI 模型版本状态（model_version.status，小写值，A.19.8 补充）
    // 与 ai_model_management_service.rs validate_model_status 的字面量一一对应

    /// 草稿：模型版本初始状态，可编辑/审批
    pub const DRAFT: &str = "draft";

    /// 退役：模型版本被新激活版本替代后的降级状态
    pub const RETIRED: &str = "retired";

    /// 归档：模型版本长期下线，不可再激活
    pub const ARCHIVED: &str = "archived";

    // 以下为 AI 模型版本审批状态（model_version.approval_status，小写值，A.19.8 补充）
    // 与 ai_model_management_service.rs validate_approval_status 的字面量一一对应

    /// 已拒绝：模型版本审批未通过
    pub const REJECTED: &str = "rejected";
}

/// 导入任务状态（import_task.status，小写值）
/// 批次 235 v13 真实接入：import_export_service.rs 中导入任务状态字符串字面量统一引用此模块（规则 0）
pub mod import_task {
    /// 运行中：导入任务正在执行
    pub const RUNNING: &str = "running";

    /// 成功：导入任务全部成功
    pub const SUCCESS: &str = "success";

    /// 失败：导入任务全部失败
    pub const FAILED: &str = "failed";

    /// 部分成功：导入任务部分成功部分失败
    pub const PARTIAL: &str = "partial";

    /// 已取消：导入任务被用户手动取消（cancel 端点使用）
    pub const CANCELLED: &str = "cancelled";
}

/// 登录日志状态（log_login.status，大写值）
/// 批次 236 v13 真实接入：login_security_handler.rs、auth_handler.rs 中登录日志状态字符串字面量统一引用此模块（规则 0）
pub mod login_log {
    /// 成功：登录成功
    pub const SUCCESS: &str = "SUCCESS";

    /// 失败：登录失败
    pub const FAILED: &str = "FAILED";
}

/// 邮件日志状态（email_log.status，大写值）
/// 批次 236 v13 真实接入：email_log_service.rs 中邮件日志状态字符串字面量统一引用此模块（规则 0）
pub mod email_log {
    /// 待发送：邮件待发送
    pub const PENDING: &str = "PENDING";

    /// 已发送：邮件已发送
    pub const SENT: &str = "SENT";

    /// 发送失败：邮件发送失败
    pub const FAILED: &str = "FAILED";
}

/// 启用/停用状态（通用，大写值 ACTIVE/INACTIVE，批次 236 v13 真实接入，与 master_data 小写模块区分）
pub mod active_status {
    /// 启用
    pub const ACTIVE: &str = "ACTIVE";

    /// 停用
    pub const INACTIVE: &str = "INACTIVE";
}

/// 审计消息状态（omni_audit_message.status，大写值）
/// 批次 236 v13 真实接入：omni_audit_service.rs
pub mod audit_message {
    /// 失败：审计消息失败
    pub const FAILED: &str = "FAILED";

    /// 拒绝：审计消息被拒绝
    pub const DENIED: &str = "DENIED";
}

/// 健康检查状态（health_check.status，小写值）
/// 批次 236 v13 真实接入：health_handler.rs
pub mod health_check {
    /// 健康
    pub const HEALTHY: &str = "healthy";

    /// 不健康
    pub const UNHEALTHY: &str = "unhealthy";

    /// 降级
    pub const DEGRADED: &str = "degraded";
}

/// 对账结果状态（reconcile_result.status，大写值）
/// 批次 236 v13 真实接入：ap_reconciliation_handler.rs
pub mod reconcile_result {
    /// 失败
    pub const FAILED: &str = "FAILED";
}

/// 故障切换状态（failover current_state，小写值）
/// 批次 236 v13 真实接入：failover_service.rs
pub mod failover {
    /// 主节点
    pub const PRIMARY: &str = "primary";

    /// 备节点
    pub const BACKUP: &str = "backup";
}

/// V15 P1-2: 批次追溯日志操作类型（batch_trace_log.operation_type，大写值）
/// 覆盖面料行业全链路操作：创建/染色/验布/分级/发货/回修/转移/调整/合缸/分缸
pub mod batch_trace_operation_type {
    /// 创建：批次初始创建
    pub const CREATE: &str = "CREATE";
    /// 染色：进缸染色操作
    pub const DYE: &str = "DYE";
    /// 验布：验布打卷操作
    pub const INSPECT: &str = "INSPECT";
    /// 分级：A/B/C 级分级判定
    pub const GRADE: &str = "GRADE";
    /// 发货：成品发货出库
    pub const SHIP: &str = "SHIP";
    /// 回修：不合格品返工回修
    pub const REWORK: &str = "REWORK";
    /// 转移：库存转移
    pub const TRANSFER: &str = "TRANSFER";
    /// 调整：库存调整
    pub const ADJUST: &str = "ADJUST";
    /// 合缸：多缸号合并
    pub const MERGE: &str = "MERGE";
    /// 分缸：单缸号拆分
    pub const SPLIT: &str = "SPLIT";
}
