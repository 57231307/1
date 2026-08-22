//! 缸号全生命周期状态机 Service（facade）
//!
//! v14 批次 432：缸号全生命周期状态机
//! 依据：面料行业真实业务调研文档 §12.7 缸号状态机 + §3.2 缸号全生命周期追踪
//!
//! 核心能力：缸号生命周期日志 CRUD + 按 batch_id 查询 + 按时间范围查询 + 记录状态流转 + 获取最新状态；
//! 缸号状态规则 CRUD + 校验流转合法性 + 查询允许的流转；
//! 缸号回修记录 CRUD + 审批 + 开始回修 + 完成回修 + 取消回修；
//! 缸号操作记录 CRUD + 按类型查询 + 按缸号查询。
//!
//! 16 种状态：pending_schedule 待排缸 / scheduled 已排缸 / preparing 备布中 / dyeing 进缸染色 / washing 皂洗 / fixing 固色 / dehydrating 脱水 / drying 烘干 / inspecting 验布 / stored 入库 / shipped 发货（终态）/ cancelled 取消（终态）/ terminated 终止（终态）/ rework 回修中（可回到 dyeing）/ on_hold 暂停（异常态，可恢复）/ failed 失败（终态）。
//!
//! 批次 490 D10-4a 拆分：本文件作为 facade，保留 4 个 Service struct + new 构造函数。
//! 10 个 DTO 已迁移至 `models/dto/dye_batch_dto`，通过 `pub use` 再导出，
//! 保持外部引用路径 `crate::services::dye_batch_state_machine_service::*` 不变。
//! 4 个 Service 的业务方法 impl 块迁移至 `dye_batch_state_machine_ops` 子模块
//! （lifecycle_log / state_rule / rework / operation），通过跨模块 `impl XxxService` 追加方法。
//! V15 P2 B07-P2-1 拆分：11 个纯验证函数 + 单元测试迁移至 `dye_batch_state_machine_validation`，
//! 通过 `pub use` 再导出，保持外部引用路径 `crate::services::dye_batch_state_machine_service::*` 不变。

use sea_orm::DatabaseConnection;
use std::sync::Arc;

// V15 P2 B07-P2-1：纯验证函数从 dye_batch_state_machine_validation 再导出，保持外部引用路径不变
pub use crate::services::dye_batch_state_machine_validation::*;

// DTO 从 models/dto/dye_batch_dto 引入，并 pub use 再导出，保持外部引用路径不变
pub use crate::models::dto::dye_batch_dto::*;

// ============================================================================
// 缸号生命周期日志 Service
// ============================================================================

/// 缸号生命周期日志 Service
/// 业务方法（record_transition / get_by_id / list_by_batch / list_by_batch_no；/ get_latest_status / list）定义在 dye_batch_state_machine_ops::lifecycle_log。
pub struct DyeBatchLifecycleLogService {
    /// 数据库连接（pub(crate) 供 dye_batch_state_machine_ops 子模块访问）
    pub(crate) db: Arc<DatabaseConnection>,
}

impl DyeBatchLifecycleLogService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

// ============================================================================
// 缸号状态流转规则 Service
// ============================================================================

/// 缸号状态流转规则 Service
/// 业务方法（create / update / delete / get_by_id / check_transition；/ list_allowed_transitions / list）定义在 dye_batch_state_machine_ops::state_rule。
pub struct DyeBatchStateRuleService {
    /// 数据库连接（pub(crate) 供 dye_batch_state_machine_ops 子模块访问）
    pub(crate) db: Arc<DatabaseConnection>,
}

impl DyeBatchStateRuleService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

// ============================================================================
// 缸号回修记录 Service
// ============================================================================

/// 缸号回修记录 Service
/// 业务方法（create / update / delete / get_by_id / approve / start_rework；/ complete_rework / cancel_rework / list）定义在 dye_batch_state_machine_ops::rework。
pub struct DyeBatchReworkService {
    /// 数据库连接（pub(crate) 供 dye_batch_state_machine_ops 子模块访问）
    pub(crate) db: Arc<DatabaseConnection>,
}

impl DyeBatchReworkService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

// ============================================================================
// 缸号操作记录 Service
// ============================================================================

/// 缸号操作记录 Service
/// 业务方法（create / get_by_id / list_by_type / list_by_batch / list）；定义在 dye_batch_state_machine_ops::operation。
pub struct DyeBatchOperationService {
    /// 数据库连接（pub(crate) 供 dye_batch_state_machine_ops 子模块访问）
    pub(crate) db: Arc<DatabaseConnection>,
}

impl DyeBatchOperationService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}
