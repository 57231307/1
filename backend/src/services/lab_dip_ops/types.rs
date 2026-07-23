//! 化验室打样 DTO 子模块（lab_dip_ops/types）
//!
//! 批次 D10 拆分：从原 `lab_dip_service.rs` 迁移 9 个 DTO struct。
//! 包含打样通知单 / 打样小样 / 复样记录 的 Create / Update / Query / Record 请求体。

use rust_decimal::Decimal;
use serde::Deserialize;

use crate::models::lab_dip_sample::FormulaDetailItem;

// ============================================================================
// 打样通知单 DTO
// ============================================================================

/// 创建打样通知单请求
///
/// 真实业务必填字段（依据 §11.1 打样通知单）：
/// - light_source: 对色光源（D65/TL84/U3000/CWF/A 等）
/// - sample_versions: 打样版数（默认 4，即 ABCD 四版）
/// - required_date: 客户要求交期
#[derive(Debug, Clone, Deserialize)]
pub struct CreateLabDipRequestRequest {
    pub customer_id: Option<i32>,
    pub customer_color_no: Option<String>,
    pub customer_color_name: Option<String>,
    pub sample_type: Option<String>,
    pub fabric_spec: Option<String>,
    pub fabric_component: Option<String>,
    pub sample_size: Option<String>,
    /// 主对色光源（必填）：D65/TL84/U3000/CWF/A 等
    pub light_source: String,
    pub secondary_light_source: Option<String>,
    pub color_fastness_req: Option<String>,
    pub eco_requirement: Option<String>,
    /// 打样版数（默认 4，即 ABCD 四版）
    pub sample_versions: Option<i32>,
    pub dye_category: Option<String>,
    /// 客户要求交期（必填）
    pub required_date: chrono::NaiveDate,
    pub expected_days: Option<i32>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 更新打样通知单请求（仅 pending/sampling 状态可更新）
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateLabDipRequestRequest {
    pub customer_id: Option<i32>,
    pub customer_color_no: Option<String>,
    pub customer_color_name: Option<String>,
    pub sample_type: Option<String>,
    pub fabric_spec: Option<String>,
    pub fabric_component: Option<String>,
    pub sample_size: Option<String>,
    pub light_source: Option<String>,
    pub secondary_light_source: Option<String>,
    pub color_fastness_req: Option<String>,
    pub eco_requirement: Option<String>,
    pub sample_versions: Option<i32>,
    pub dye_category: Option<String>,
    pub required_date: Option<chrono::NaiveDate>,
    pub expected_days: Option<i32>,
    pub remarks: Option<String>,
}

/// 打样通知单查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct LabDipRequestQuery {
    pub request_no: Option<String>,
    pub customer_id: Option<i32>,
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

// ============================================================================
// 打样小样 DTO
// ============================================================================

/// 创建打样小样请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateLabDipSampleRequest {
    pub request_id: i32,
    /// 版本标识：A/B/C/D/E...（如不传则自动生成）
    pub version_label: Option<String>,
    pub recipe_no: Option<String>,
    pub dye_recipe_id: Option<i32>,
    pub formula: Option<String>,
    pub formula_detail: Option<Vec<FormulaDetailItem>>,
    pub temperature: Option<Decimal>,
    pub time_minutes: Option<i32>,
    pub liquor_ratio: Option<String>,
    pub ph_value: Option<Decimal>,
    pub dyeing_method: Option<String>,
    pub dye_cost: Option<Decimal>,
    pub auxiliary_cost: Option<Decimal>,
    pub total_cost: Option<Decimal>,
    pub color_difference_grade: Option<i32>,
    pub color_difference_value: Option<Decimal>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 更新打样小样请求（仅 pending 对色状态可更新）
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateLabDipSampleRequest {
    pub recipe_no: Option<String>,
    pub dye_recipe_id: Option<i32>,
    pub formula: Option<String>,
    pub formula_detail: Option<Vec<FormulaDetailItem>>,
    pub temperature: Option<Decimal>,
    pub time_minutes: Option<i32>,
    pub liquor_ratio: Option<String>,
    pub ph_value: Option<Decimal>,
    pub dyeing_method: Option<String>,
    pub dye_cost: Option<Decimal>,
    pub auxiliary_cost: Option<Decimal>,
    pub total_cost: Option<Decimal>,
    pub color_difference_grade: Option<i32>,
    pub color_difference_value: Option<Decimal>,
    pub remarks: Option<String>,
}

/// 记录对色结果请求
#[derive(Debug, Clone, Deserialize)]
pub struct RecordMatchingResultRequest {
    /// 色差等级（4-5 级为 OK，<4 级为重打）
    pub color_difference_grade: i32,
    pub color_difference_value: Option<Decimal>,
    /// 审核人
    pub approved_by: Option<i32>,
    pub approval_comment: Option<String>,
}

// ============================================================================
// 复样记录 DTO
// ============================================================================

/// 创建复样记录请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateResampleRequest {
    pub request_id: i32,
    pub source_sample_id: i32,
    pub workshop_fabric_batch: Option<String>,
    pub dye_batch_no: Option<String>,
    pub auxiliary_batch_no: Option<String>,
    pub production_plan_id: Option<i32>,
    pub adjusted_formula: Option<String>,
    pub adjustment_factor: Option<Decimal>,
    pub adjusted_temperature: Option<Decimal>,
    pub adjusted_time_minutes: Option<i32>,
    pub adjusted_liquor_ratio: Option<String>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 记录复样结果请求
#[derive(Debug, Clone, Deserialize)]
pub struct RecordResampleResultRequest {
    pub color_difference_grade: i32,
    pub color_difference_value: Option<Decimal>,
    pub reviewed_by: Option<i32>,
    pub review_comment: Option<String>,
}

/// 染色技术卡开具请求
#[derive(Debug, Clone, Deserialize)]
pub struct IssueTechCardRequest {
    /// 开卡人（研发组长）
    pub issued_by: i32,
}
