//! 流转卡与工序流转 Service（facade）
//!
//! v14 批次 425：流转卡条码与车间工序流转。本文件作为 facade，保留 4 个 Service struct
//! + new 构造函数 + 9 个 DTOs + 5 个纯函数（单号生成/状态校验）+ 单元测试。
//! 业务 impl 块迁移至 flow_card_ops 子模块（route / card_crud / card_state / step / feedback），
//! 通过 db 字段 pub(crate) 让 ops 访问，外部引用路径不变。

use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use std::sync::Arc;

use crate::models::status::flow_card as card_status;
use crate::utils::error::AppError;

// ============================================================================
// 工序路线模板 Service struct 定义（impl 块在 flow_card_ops/route 子模块）
// ============================================================================

/// 创建工序路线请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateProcessRouteRequest {
    pub route_code: String,
    pub route_name: String,
    pub seq: i32,
    pub process_type: String,
    pub default_duration_minutes: Option<i32>,
    pub require_scan: Option<bool>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 更新工序路线请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateProcessRouteRequest {
    pub route_name: Option<String>,
    pub seq: Option<i32>,
    pub process_type: Option<String>,
    pub default_duration_minutes: Option<i32>,
    pub require_scan: Option<bool>,
    pub is_active: Option<bool>,
    pub remarks: Option<String>,
}

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

/// 创建流转卡请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateFlowCardRequest {
    pub production_order_id: i32,
    pub dye_batch_id: Option<i32>,
    pub dye_lot_no: Option<String>,
    pub process_route_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub customer_name: Option<String>,
    pub order_no: Option<String>,
    pub product_id: Option<i32>,
    pub product_name: Option<String>,
    pub color_no: Option<String>,
    pub dyeing_requirements: Option<String>,
    pub planned_fabric_weight: Option<Decimal>,
    pub priority: Option<i32>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 更新流转卡请求（仅 pending 状态可更新）
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateFlowCardRequest {
    pub dye_batch_id: Option<i32>,
    pub dye_lot_no: Option<String>,
    pub process_route_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub customer_name: Option<String>,
    pub order_no: Option<String>,
    pub product_id: Option<i32>,
    pub product_name: Option<String>,
    pub color_no: Option<String>,
    pub dyeing_requirements: Option<String>,
    pub planned_fabric_weight: Option<Decimal>,
    pub priority: Option<i32>,
    pub remarks: Option<String>,
}

/// 流转卡查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct FlowCardQuery {
    pub card_no: Option<String>,
    pub barcode: Option<String>,
    pub dye_lot_no: Option<String>,
    pub production_order_id: Option<i32>,
    pub status: Option<String>,
    pub customer_id: Option<i32>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 流转卡 Service
pub struct FlowCardService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl FlowCardService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 生成流转卡号：FC-YYYYMMDDHHMMSS-NNN
    pub(crate) fn generate_card_no() -> String {
        let now = chrono::Utc::now();
        let timestamp = now.format("%Y%m%d%H%M%S");
        let random = crate::utils::random::random_6_digit() % 1000;
        format!("FC-{}-{:03}", timestamp, random)
    }

    /// 生成条码：FC + 14位时间戳 + 6位随机数
    pub(crate) fn generate_barcode() -> String {
        let now = chrono::Utc::now();
        let timestamp = now.format("%Y%m%d%H%M%S");
        let random = crate::utils::random::random_6_digit();
        format!("FC{}{:06}", timestamp, random)
    }

    /// 状态流转校验（缸号全生命周期状态机）
    pub(crate) fn validate_status_transition(from: &str, to: &str) -> Result<(), AppError> {
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
    pub(crate) fn validate_can_update(status: &str) -> Result<(), AppError> {
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

/// 开始工序请求（扫码开始）
#[derive(Debug, Clone, Deserialize)]
pub struct StartStepRequest {
    pub flow_card_id: i32,
    pub process_route_id: Option<i32>,
    pub worker_ids: Option<String>,
    pub worker_names: Option<String>,
    pub equipment_id: Option<i32>,
    pub equipment_name: Option<String>,
    pub created_by: Option<i32>,
}

/// 结束工序请求（扫码结束）
#[derive(Debug, Clone, Deserialize)]
pub struct CompleteStepRequest {
    pub actual_quantity: Option<Decimal>,
    pub qualified_quantity: Option<Decimal>,
    pub abnormal_description: Option<String>,
    pub handling_opinion: Option<String>,
    pub remarks: Option<String>,
}

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

/// 创建质量反馈单请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateFeedbackRequest {
    pub flow_card_id: i32,
    pub step_record_id: Option<i32>,
    pub feedback_type: String,
    pub description: String,
    pub severity: Option<String>,
    pub found_by: Option<i32>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 处理反馈单请求
#[derive(Debug, Clone, Deserialize)]
pub struct HandleFeedbackRequest {
    pub handling_opinion: Option<String>,
    pub handling_result: Option<String>,
    pub handled_by: Option<i32>,
}

/// 质量反馈单 Service
pub struct QualityFeedbackService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl QualityFeedbackService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 生成反馈单号：QF-YYYYMMDDHHMMSS-NNN
    pub(crate) fn generate_feedback_no() -> String {
        let now = chrono::Utc::now();
        let timestamp = now.format("%Y%m%d%H%M%S");
        let random = crate::utils::random::random_6_digit() % 1000;
        format!("QF-{}-{:03}", timestamp, random)
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试流转卡号生成格式
    #[test]
    fn test_generate_card_no_format() {
        let card_no = FlowCardService::generate_card_no();
        assert!(card_no.starts_with("FC-"));
        // 格式：FC-YYYYMMDDHHMMSS-NNN
        let parts: Vec<&str> = card_no.split('-').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[1].len(), 14); // YYYYMMDDHHMMSS
        assert_eq!(parts[2].len(), 3); // NNN
    }

    /// 测试条码生成格式
    #[test]
    fn test_generate_barcode_format() {
        let barcode = FlowCardService::generate_barcode();
        assert!(barcode.starts_with("FC"));
        // 格式：FC + 14位时间戳 + 6位随机数 = 22 字符
        assert_eq!(barcode.len(), 22);
    }

    /// 测试反馈单号生成格式
    #[test]
    fn test_generate_feedback_no_format() {
        let no = QualityFeedbackService::generate_feedback_no();
        assert!(no.starts_with("QF-"));
        let parts: Vec<&str> = no.split('-').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[1].len(), 14);
        assert_eq!(parts[2].len(), 3);
    }

    /// 测试流转卡状态流转校验
    #[test]
    fn test_validate_status_transition_normal() {
        // 正常流转路径
        assert!(FlowCardService::validate_status_transition(
            card_status::PENDING,
            card_status::SCHEDULED
        )
        .is_ok());
        assert!(FlowCardService::validate_status_transition(
            card_status::SCHEDULED,
            card_status::PREPARING
        )
        .is_ok());
        assert!(FlowCardService::validate_status_transition(
            card_status::PREPARING,
            card_status::DYEING
        )
        .is_ok());
        assert!(FlowCardService::validate_status_transition(
            card_status::DYEING,
            card_status::DYED
        )
        .is_ok());
        assert!(FlowCardService::validate_status_transition(
            card_status::DYED,
            card_status::INSPECTING
        )
        .is_ok());
        assert!(FlowCardService::validate_status_transition(
            card_status::INSPECTING,
            card_status::COMPLETED
        )
        .is_ok());
        assert!(FlowCardService::validate_status_transition(
            card_status::COMPLETED,
            card_status::SHIPPED
        )
        .is_ok());
    }

    /// 测试流转卡状态流转校验：非法路径
    #[test]
    fn test_validate_status_transition_illegal() {
        // pending 不能直接到 dyeing
        assert!(FlowCardService::validate_status_transition(
            card_status::PENDING,
            card_status::DYEING
        )
        .is_err());
        // shipped 是终态，不可再流转
        assert!(FlowCardService::validate_status_transition(
            card_status::SHIPPED,
            card_status::PENDING
        )
        .is_err());
        // terminated 只能回到 pending
        assert!(FlowCardService::validate_status_transition(
            card_status::TERMINATED,
            card_status::SCHEDULED
        )
        .is_err());
        assert!(FlowCardService::validate_status_transition(
            card_status::TERMINATED,
            card_status::PENDING
        )
        .is_ok());
    }

    /// 测试可更新状态校验
    #[test]
    fn test_validate_can_update() {
        assert!(FlowCardService::validate_can_update(card_status::PENDING).is_ok());
        assert!(FlowCardService::validate_can_update(card_status::SCHEDULED).is_ok());
        assert!(FlowCardService::validate_can_update(card_status::DYEING).is_err());
        assert!(FlowCardService::validate_can_update(card_status::COMPLETED).is_err());
    }

    /// 测试回修场景：INSPECTING 可回到 DYEING（回修订单重新进缸）
    #[test]
    fn test_validate_status_transition_rework() {
        // 验布发现质量问题需要回修染色
        assert!(FlowCardService::validate_status_transition(
            card_status::INSPECTING,
            card_status::DYEING
        )
        .is_ok());
    }
}
