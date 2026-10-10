//! 流转卡与工序流转 Service（facade）
//!
//! v14 批次 425：流转卡条码与车间工序流转。本文件作为 facade，保留 4 个 Service struct
//! + new 构造函数 + 5 个纯函数（单号生成/状态校验）+ 单元测试。
//! 业务 impl 块迁移至 flow_card_ops 子模块（route / card_crud / card_state / step / feedback），
//! 通过 db 字段 pub(crate) 让 ops 访问，外部引用路径不变。
//!
//! 9 个 DTO 已迁移至 `models/dto/flow_card_dto`，通过 `pub use` 再导出，
//! 保持外部引用路径 `crate::services::flow_card_service::*` 不变。

use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::models::status::flow_card as card_status;
use crate::utils::error::AppError;

// DTO 从 models/dto/flow_card_dto 引入，并 pub use 再导出，保持外部引用路径不变
pub use crate::models::dto::flow_card_dto::*;

// ============================================================================
// 工序路线模板 Service struct 定义（impl 块在 flow_card_ops/route 子模块）
// ============================================================================

/// 工序路线 Service
pub struct ProcessRouteService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl ProcessRouteService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

// ============================================================================
// 流转卡 Service struct 定义（impl 块在 flow_card_ops/card_crud、card_state 子模块）
// ============================================================================

/// 流转卡号（production_flow_card.card_no）自动编码前缀：沿用原手写格式
/// "FC-{时间戳}-{随机}" 的业务前缀 FC，新格式统一为 {FC}{YYYYMMDD}{3位流水}
///（前缀集中定义于此，ops 子模块只引用常量，不散落字面量）。
pub const FLOW_CARD_NO_PREFIX: &str = "FC";

/// 流转卡 Service
pub struct FlowCardService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl FlowCardService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 状态流转校验（缸号全生命周期状态机）
    pub fn validate_status_transition(from: &str, to: &str) -> Result<(), AppError> {
        let allowed = match from {
            card_status::PENDING => vec![card_status::SCHEDULED, card_status::TERMINATED],
            card_status::SCHEDULED => vec![
                card_status::PREPARING,
                card_status::PENDING,
                card_status::TERMINATED,
            ],
            card_status::PREPARING => vec![card_status::DYEING, card_status::TERMINATED],
            card_status::DYEING => vec![card_status::DYED, card_status::TERMINATED],
            card_status::DYED => vec![card_status::INSPECTING],
            card_status::INSPECTING => vec![card_status::COMPLETED, card_status::DYEING],
            card_status::COMPLETED => vec![card_status::SHIPPED],
            card_status::SHIPPED => vec![],
            card_status::TERMINATED => vec![card_status::PENDING],
            _ => return Err(AppError::business(format!("未知流转卡状态: {}", from))),
        };

        if !allowed.contains(&to) {
            return Err(AppError::business(format!(
                "流转卡状态不允许从 {} 流转到 {}（允许: {:?}）",
                from, to, allowed
            )));
        }
        Ok(())
    }

    /// 仅 pending/scheduled 状态可更新
    pub fn validate_can_update(status: &str) -> Result<(), AppError> {
        if status != card_status::PENDING && status != card_status::SCHEDULED {
            return Err(AppError::business(format!(
                "流转卡状态为 {}，仅 pending/scheduled 状态可更新",
                status
            )));
        }
        Ok(())
    }
}

// ============================================================================
// 工序流转记录 Service struct 定义（impl 块在 flow_card_ops/step 子模块）
// ============================================================================

/// 工序流转记录 Service
pub struct StepRecordService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl StepRecordService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

// ============================================================================
// 工序质量反馈单 Service struct 定义（impl 块在 flow_card_ops/feedback 子模块）
// ============================================================================

/// 质量反馈单号（process_quality_feedback.feedback_no）自动编码前缀：
/// 沿用原手写格式 "QF-{时间戳}-{随机}" 的业务前缀 QF，
/// 新格式统一为 {QF}{YYYYMMDD}{3位流水}。
pub const QUALITY_FEEDBACK_NO_PREFIX: &str = "QF";

/// 质量反馈单 Service
pub struct QualityFeedbackService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl QualityFeedbackService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

// ============================================================================
// 单元测试
// ============================================================================
