//! 化验室打样 DTO 子模块（lab_dip_ops/types）
//!
//! 批次 D10 拆分：从原 `lab_dip_service.rs` 迁移 9 个 DTO struct。
//! 包含打样通知单 / 打样小样 / 复样记录 的 Create / Update / Query / Record 请求体。

use rust_decimal::Decimal;
use serde::Deserialize;

use crate::models::lab_dip_sample::FormulaDetailItem;

/// JSON 三态反序列化适配器（RFC 7386 JSON Merge Patch 的"键缺席 ≠ 显式 null"语义所需）。
///
/// serde_json 对 `Option<Option<T>>` 的默认反序列化在遇到 JSON null 时直接调 visit_none()，
/// 把"显式 null"塌成外层 `None`，与"键缺席"不可区分。本适配器先按内层 `Option<T>` 反序列化
/// 再包一层：键缺席（配合 `#[serde(default)]`）= `None`、显式 null = `Some(None)`、有值 = `Some(Some(v))`。
/// 形态与 handlers/department_handler.rs 中同名私有适配器一致（跨域合并到共享 utils 超本波授权范围）。
fn double_option<'de, T, D>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Deserialize::deserialize(de).map(Some)
}

// ============================================================================
// 打样通知单 DTO
// ============================================================================

/// 创建打样通知单请求
/// 真实业务必填字段（依据 §11.1 打样通知单）：light_source: 对色光源（D65/TL84/U3000/CWF/A 等）；sample_versions: 打样版数（默认 4，即 ABCD 四版）；required_date: 客户要求交期
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
///
/// 三态语义（RFC 7386，对齐 handlers/department_handler.rs 范式）：
/// 键缺席=保持原值、显式 null=清空为 NULL（仅 DB 可空列）、有值=覆盖。
/// NOT NULL 列（light_source/sample_versions/required_date，v15 lab_dip_request DDL）
/// 显式 null 由 service 入口在任何 DB 访问前拒绝。
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateLabDipRequestRequest {
    /// 客户 ID：DB 可空 INTEGER（v15:2987）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub customer_id: Option<Option<i32>>,
    /// 客户色号：DB 可空 VARCHAR（v15:2988）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub customer_color_no: Option<Option<String>>,
    /// 客户色名：DB 可空 VARCHAR（v15:2989）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub customer_color_name: Option<Option<String>>,
    /// 来样类型：DB 可空 VARCHAR（v15:2990）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub sample_type: Option<Option<String>>,
    /// 坯布规格：DB 可空 VARCHAR（v15:2991）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub fabric_spec: Option<Option<String>>,
    /// 纤维成分：DB 可空 VARCHAR（v15:2992）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub fabric_component: Option<Option<String>>,
    /// 打样坯布大小：DB 可空 VARCHAR（v15:2993）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub sample_size: Option<Option<String>>,
    /// 主对色光源：NOT NULL VARCHAR（v15:2994）——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub light_source: Option<Option<String>>,
    /// 副光源：DB 可空 VARCHAR（v15:2995）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub secondary_light_source: Option<Option<String>>,
    /// 色牢度要求：DB 可空 VARCHAR（v15:2996）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub color_fastness_req: Option<Option<String>>,
    /// 环保标准：DB 可空 VARCHAR（v15:2997）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub eco_requirement: Option<Option<String>>,
    /// 打样版数：NOT NULL INTEGER（v15:2998）——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub sample_versions: Option<Option<i32>>,
    /// 染料类别：DB 可空 VARCHAR（v15:2999）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub dye_category: Option<Option<String>>,
    /// 客户要求交期：NOT NULL DATE（v15:3000）——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub required_date: Option<Option<chrono::NaiveDate>>,
    /// 预期打样天数：DB 可空 INTEGER（v15:3001）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub expected_days: Option<Option<i32>>,
    /// 备注：DB 可空 VARCHAR（v15:3007）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub remarks: Option<Option<String>>,
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
///
/// 三态语义（RFC 7386）：键缺席=保持原值、显式 null=清空为 NULL、有值=覆盖。
/// 本请求全部字段在 v15 lab_dip_sample DDL 中为可空列（recipe_no v15:3052、
/// dye_recipe_id 3053、formula 3054、formula_detail 3055、temperature 3056、
/// time_minutes 3057、liquor_ratio 3058、ph_value 3059、dyeing_method 3060、
/// dye_cost 3061、auxiliary_cost 3062、total_cost 3063、color_difference_grade 3064、
/// color_difference_value 3065、remarks 3072），均支持显式 null 清空。
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateLabDipSampleRequest {
    #[serde(default, deserialize_with = "double_option")]
    pub recipe_no: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub dye_recipe_id: Option<Option<i32>>,
    #[serde(default, deserialize_with = "double_option")]
    pub formula: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub formula_detail: Option<Option<Vec<FormulaDetailItem>>>,
    #[serde(default, deserialize_with = "double_option")]
    pub temperature: Option<Option<Decimal>>,
    #[serde(default, deserialize_with = "double_option")]
    pub time_minutes: Option<Option<i32>>,
    #[serde(default, deserialize_with = "double_option")]
    pub liquor_ratio: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub ph_value: Option<Option<Decimal>>,
    #[serde(default, deserialize_with = "double_option")]
    pub dyeing_method: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub dye_cost: Option<Option<Decimal>>,
    #[serde(default, deserialize_with = "double_option")]
    pub auxiliary_cost: Option<Option<Decimal>>,
    #[serde(default, deserialize_with = "double_option")]
    pub total_cost: Option<Option<Decimal>>,
    #[serde(default, deserialize_with = "double_option")]
    pub color_difference_grade: Option<Option<i32>>,
    #[serde(default, deserialize_with = "double_option")]
    pub color_difference_value: Option<Option<Decimal>>,
    #[serde(default, deserialize_with = "double_option")]
    pub remarks: Option<Option<String>>,
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
