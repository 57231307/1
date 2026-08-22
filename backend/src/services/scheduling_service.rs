//! 排程核心服务（scheduling_service）
//!
//! 仅保留 `SchedulingService` 本体与 `new()` 入口。
//! 排程相关 DTO 已迁移到 `crate::models::dto::scheduling_dto`。
//! 9 个 pub fn 按职责拆分到 3 个子模块：
//! - `scheduling_auto`   排程自动调度（auto_schedule / detect_conflicts / save_schedule_result）
//! - `scheduling_manual` 排程手动调整（adjust_schedule）
//! - `scheduling_query`  排程查询与甘特图（get_gantt_data / list_scheduled_orders / get_schedule_history / get_schedule_result / confirm_schedule_result）

// A.7.3：DTO 迁移到 models/dto/ 后，pub use 再导出保持外部引用兼容（tests/test_scheduling.rs 引用）
#[allow(unused_imports)]
pub use crate::models::dto::scheduling_dto::*;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

/// 排程服务
pub struct SchedulingService {
    pub(crate) db: Arc<DatabaseConnection>,
    /// V15 P2 缺陷 9.2：排程冲突自动告警通知服务
    pub(crate) notification_service:
        Option<crate::services::event_notification_service::EventNotificationService>,
}

impl SchedulingService {
    /// 创建排程服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            db: db.clone(),
            // V15 P2 缺陷 9.2：默认注入 EventNotificationService 用于冲突告警
            notification_service: Some(
                crate::services::event_notification_service::EventNotificationService::new(db),
            ),
        }
    }
}
