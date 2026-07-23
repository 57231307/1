//! 大货处方与加料处方 Service（facade）
//!
//! v14 批次 424：大货处方与加料处方流程
//! 依据：面料行业真实业务调研文档 §11.2 大货处方（染色配料单）与加料处方（染色补料单）
//! 真实业务流程：
//!   大货处方单：扫描流转卡条码 → 依据备布数量 → 加载小样处方/历史大货处方 → 根据浴比/浴量
//!              → 填写物料明细 → 计算用量 → 开具大货处方单 → 审核后自动建立生产领用单据
//!   加料处方单：扫描流转卡 → 加载已审核大货处方 → 登记加料物料 → 生成加料处方单
//!   关键约束：同一工单号只能开一张大货处方单，追加物料须开加料处方单
//!
//! 核心能力：
//! - 大货处方 CRUD + 状态流转（draft → approved → closed → cancelled）
//! - 用量计算（浓度% × 布重 × 浴比 / 100）
//! - 一工单一处方约束校验
//! - 加料处方 CRUD + 状态流转（draft → approved → closed）
//!
//! 本文件作为 facade：保留 2 个 Service struct + new 构造函数 + 7 个 DTOs +
//! 纯函数（单号生成/浴比解析/用量计算/状态校验）+ 单元测试。
//! 业务 impl 块迁移至 production_recipe_ops 子模块
//!（recipe_crud / recipe_state / addition），通过 db 字段 pub(crate) 让 ops 访问，
//! 外部引用路径不变。

use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use std::sync::Arc;

use crate::models::production_recipe::RecipeMaterialItem;
use crate::models::status::production_recipe as recipe_status;
use crate::models::status::production_recipe_addition as addition_status;
use crate::utils::error::AppError;

// ============================================================================
// 大货处方 Service struct 定义（impl 块在 production_recipe_ops/recipe_crud、recipe_state 子模块）
// ============================================================================

/// 创建大货处方请求
///
/// 真实业务必填字段（依据 §11.2 大货处方）：
/// - fabric_weight: 备布重量（用量计算依据）
/// - liquor_ratio: 浴比（如 1:8）
/// - recipe_detail: 处方明细（染料+助剂）
#[derive(Debug, Clone, Deserialize)]
pub struct CreateProductionRecipeRequest {
    pub work_order_id: Option<i32>,
    pub dye_batch_id: Option<i32>,
    pub source_recipe_id: Option<i32>,
    pub lab_dip_resample_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub color_no: Option<String>,
    pub fabric_name: Option<String>,
    pub fabric_spec: Option<String>,
    pub fabric_width: Option<Decimal>,
    pub gram_weight: Option<Decimal>,
    /// 备布重量 kg（必填，用量计算依据）
    pub fabric_weight: Decimal,
    pub equipment_no: Option<String>,
    /// 浴比如 1:8（必填）
    pub liquor_ratio: String,
    pub bath_volume: Option<Decimal>,
    pub adjustment_factor: Option<Decimal>,
    pub recipe_detail: Option<Vec<RecipeMaterialItem>>,
    pub total_dye_cost: Option<Decimal>,
    pub total_auxiliary_cost: Option<Decimal>,
    pub remarks: Option<String>,
    pub issued_by: Option<i32>,
    pub created_by: Option<i32>,
}

/// 更新大货处方请求（仅 draft 状态可更新）
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateProductionRecipeRequest {
    pub work_order_id: Option<i32>,
    pub dye_batch_id: Option<i32>,
    pub source_recipe_id: Option<i32>,
    pub lab_dip_resample_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub color_no: Option<String>,
    pub fabric_name: Option<String>,
    pub fabric_spec: Option<String>,
    pub fabric_width: Option<Decimal>,
    pub gram_weight: Option<Decimal>,
    pub fabric_weight: Option<Decimal>,
    pub equipment_no: Option<String>,
    pub liquor_ratio: Option<String>,
    pub bath_volume: Option<Decimal>,
    pub adjustment_factor: Option<Decimal>,
    pub recipe_detail: Option<Vec<RecipeMaterialItem>>,
    pub total_dye_cost: Option<Decimal>,
    pub total_auxiliary_cost: Option<Decimal>,
    pub remarks: Option<String>,
}

/// 大货处方查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct ProductionRecipeQuery {
    pub work_order_id: Option<i32>,
    pub dye_batch_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub color_no: Option<String>,
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 审核请求
#[derive(Debug, Clone, Deserialize)]
pub struct ApproveRecipeRequest {
    pub approved_by: i32,
}

/// 用量计算请求（按浓度+布重+浴比计算各物料用量）
#[derive(Debug, Clone, Deserialize)]
pub struct CalculateAmountsRequest {
    /// 备布重量 kg
    pub fabric_weight: Decimal,
    /// 浴比如 1:8
    pub liquor_ratio: String,
    /// 加成系数（默认 1.00）
    pub adjustment_factor: Option<Decimal>,
    /// 物料明细（需包含 concentration）
    pub items: Vec<RecipeMaterialItem>,
}

/// 大货处方 Service
///
/// `pub(crate) db`：production_recipe_ops 子模块（recipe_crud / recipe_state）需直接访问
/// db 字段执行 sea_orm 查询。
pub struct ProductionRecipeService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl ProductionRecipeService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 生成大货处方单号：PR-YYYYMMDDHHMMSS-NNN
    ///
    /// `pub(crate)`：production_recipe_ops::recipe_crud 的 create 方法调用。
    pub(crate) fn generate_recipe_no() -> String {
        let now = chrono::Utc::now();
        let timestamp = now.format("%Y%m%d%H%M%S");
        let random = crate::utils::random::random_6_digit() % 1000;
        format!("PR-{}-{:03}", timestamp, random)
    }

    /// 解析浴比字符串（如 "1:8"）为浴比数值（8.0）
    ///
    /// 真实业务：浴比格式为 "1:N"，N 通常为 5-20
    pub fn parse_liquor_ratio(ratio: &str) -> Result<Decimal, AppError> {
        let trimmed = ratio.trim();
        if trimmed.is_empty() {
            return Err(AppError::business("浴比不能为空"));
        }
        // 支持 "1:8" / "1：8"（全角冒号）/ "1/8" 三种格式
        // 一次遍历将全角冒号和斜杠统一为半角冒号，避免连续 str::replace 触发 clippy 警告
        let normalized: String = trimmed
            .chars()
            .map(|c| if c == '：' || c == '/' { ':' } else { c })
            .collect();
        let parts: Vec<&str> = normalized.split(':').collect();
        if parts.len() != 2 {
            return Err(AppError::business(format!(
                "浴比格式错误：{}（应为 1:N 格式，如 1:8）",
                ratio
            )));
        }
        let denominator = parts[1].trim().parse::<Decimal>().map_err(|_| {
            AppError::business(format!("浴比数值解析失败：{}", parts[1]))
        })?;
        if denominator <= Decimal::ZERO {
            return Err(AppError::business("浴比数值必须大于 0"));
        }
        Ok(denominator)
    }

    /// 计算用量（根据浓度+布重+浴比）
    ///
    /// 真实业务公式：用量 = 浓度% × 布重 × 浴比 / 100
    /// 其中浓度%为对布重百分比（owf%），浴比为 "1:N" 中的 N
    /// 加成系数用于修正小样→大货得色差异（默认 1.00）
    pub fn calculate_amounts(req: CalculateAmountsRequest) -> Result<Vec<RecipeMaterialItem>, AppError> {
        if req.fabric_weight <= Decimal::ZERO {
            return Err(AppError::business("备布重量必须大于 0"));
        }
        let ratio = Self::parse_liquor_ratio(&req.liquor_ratio)?;
        let factor = req.adjustment_factor.unwrap_or(Decimal::ONE);
        if factor <= Decimal::ZERO {
            return Err(AppError::business("加成系数必须大于 0"));
        }

        let hundred = Decimal::from(100);
        let mut result = Vec::with_capacity(req.items.len());
        for mut item in req.items {
            // 仅当浓度存在时才重新计算用量；助剂可能无浓度（直接给用量）
            if let Some(conc) = item.concentration {
                if conc < Decimal::ZERO {
                    return Err(AppError::business(format!(
                        "物料 {} 浓度不能为负",
                        item.material_code
                    )));
                }
                // 用量 = 浓度% × 布重 × 浴比 / 100 × 加成系数
                let amount = (conc * req.fabric_weight * ratio / hundred) * factor;
                item.amount = amount;
            }
            result.push(item);
        }
        Ok(result)
    }

    // ===== 状态流转校验 =====

    /// 校验状态流转合法性
    ///
    /// 状态机：draft → approved → closed
    ///         draft → cancelled
    ///         approved → closed
    pub fn validate_status_transition(current: &str, new: &str) -> Result<(), AppError> {
        let valid = match current {
            recipe_status::DRAFT => matches!(new, recipe_status::APPROVED | recipe_status::CANCELLED),
            recipe_status::APPROVED => matches!(new, recipe_status::CLOSED),
            recipe_status::CLOSED => false,    // 终态
            recipe_status::CANCELLED => false, // 终态
            _ => false,
        };
        if !valid {
            return Err(AppError::business(format!(
                "大货处方状态流转非法：{} → {}",
                current, new
            )));
        }
        Ok(())
    }

    /// 校验：仅 draft 状态可更新
    pub fn validate_can_update(status: &str) -> Result<(), AppError> {
        if status != recipe_status::DRAFT {
            return Err(AppError::business(format!(
                "当前状态 {} 不可更新（仅 draft 可更新）",
                status
            )));
        }
        Ok(())
    }

    /// 校验：仅 draft 状态可删除
    pub fn validate_can_delete(status: &str) -> Result<(), AppError> {
        if status != recipe_status::DRAFT {
            return Err(AppError::business(format!(
                "当前状态 {} 不可删除（仅 draft 可删除）",
                status
            )));
        }
        Ok(())
    }
}

// ============================================================================
// 加料处方 Service struct 定义（impl 块在 production_recipe_ops/addition 子模块）
// ============================================================================

/// 创建加料处方请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateProductionRecipeAdditionRequest {
    /// 关联大货处方（必填）
    pub production_recipe_id: i32,
    pub work_order_id: Option<i32>,
    pub dye_batch_id: Option<i32>,
    /// 加料原因：色差/助剂不足/工艺调整
    pub addition_reason: Option<String>,
    pub addition_detail: Option<Vec<crate::models::production_recipe_addition::AdditionMaterialItem>>,
    pub total_cost: Option<Decimal>,
    pub remarks: Option<String>,
    pub issued_by: Option<i32>,
    pub created_by: Option<i32>,
}

/// 加料处方查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct ProductionRecipeAdditionQuery {
    pub production_recipe_id: Option<i32>,
    pub work_order_id: Option<i32>,
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 加料处方 Service
///
/// `pub(crate) db`：production_recipe_ops::addition 子模块需直接访问 db 字段执行
/// sea_orm 查询。
pub struct ProductionRecipeAdditionService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl ProductionRecipeAdditionService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 生成加料处方单号：PA-YYYYMMDDHHMMSS-NNN
    ///
    /// `pub(crate)`：production_recipe_ops::addition 的 create 方法调用。
    pub(crate) fn generate_addition_no() -> String {
        let now = chrono::Utc::now();
        let timestamp = now.format("%Y%m%d%H%M%S");
        let random = crate::utils::random::random_6_digit() % 1000;
        format!("PA-{}-{:03}", timestamp, random)
    }

    // ===== 状态流转校验 =====

    /// 校验状态流转合法性
    ///
    /// 状态机：draft → approved → closed
    pub fn validate_status_transition(current: &str, new: &str) -> Result<(), AppError> {
        let valid = match current {
            addition_status::DRAFT => matches!(new, addition_status::APPROVED),
            addition_status::APPROVED => matches!(new, addition_status::CLOSED),
            addition_status::CLOSED => false, // 终态
            _ => false,
        };
        if !valid {
            return Err(AppError::business(format!(
                "加料处方状态流转非法：{} → {}",
                current, new
            )));
        }
        Ok(())
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use rust_decimal::prelude::FromPrimitive;

    /// 测试大货处方单号生成格式：PR-YYYYMMDDHHMMSS-NNN
    #[test]
    fn test_generate_recipe_no() {
        let no = ProductionRecipeService::generate_recipe_no();
        assert!(no.starts_with("PR-"));
        let parts: Vec<&str> = no.split('-').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[1].len(), 14); // YYYYMMDDHHMMSS
        assert_eq!(parts[2].len(), 3); // 3 位随机
    }

    /// 测试加料处方单号生成格式：PA-YYYYMMDDHHMMSS-NNN
    #[test]
    fn test_generate_addition_no() {
        let no = ProductionRecipeAdditionService::generate_addition_no();
        assert!(no.starts_with("PA-"));
        let parts: Vec<&str> = no.split('-').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[1].len(), 14);
        assert_eq!(parts[2].len(), 3);
    }

    /// 测试浴比解析
    #[test]
    fn test_parse_liquor_ratio() {
        // 标准 "1:8" 格式
        assert_eq!(
            ProductionRecipeService::parse_liquor_ratio("1:8").unwrap(),
            Decimal::from(8)
        );
        // 全角冒号
        assert_eq!(
            ProductionRecipeService::parse_liquor_ratio("1：10").unwrap(),
            Decimal::from(10)
        );
        // 斜杠格式
        assert_eq!(
            ProductionRecipeService::parse_liquor_ratio("1/12").unwrap(),
            Decimal::from(12)
        );
        // 带空格
        assert_eq!(
            ProductionRecipeService::parse_liquor_ratio(" 1:8 ").unwrap(),
            Decimal::from(8)
        );
        // 非法格式
        assert!(ProductionRecipeService::parse_liquor_ratio("").is_err());
        assert!(ProductionRecipeService::parse_liquor_ratio("abc").is_err());
        assert!(ProductionRecipeService::parse_liquor_ratio("1:").is_err());
        assert!(ProductionRecipeService::parse_liquor_ratio("1:0").is_err());
        assert!(ProductionRecipeService::parse_liquor_ratio("1:-5").is_err());
    }

    /// 测试用量计算
    ///
    /// 真实业务公式：用量 = 浓度% × 布重 × 浴比 / 100 × 加成系数
    #[test]
    fn test_calculate_amounts() {
        let fabric_weight = Decimal::from(100); // 100 kg
        let liquor_ratio = "1:8".to_string(); // 浴比 8
        let items = vec![RecipeMaterialItem {
            material_code: "D001".to_string(),
            material_name: "活性红".to_string(),
            concentration: Some(Decimal::from(2)), // 2% owf
            unit: "kg".to_string(),
            amount: Decimal::ZERO, // 待计算
            category: "dye".to_string(),
        }];

        let req = CalculateAmountsRequest {
            fabric_weight,
            liquor_ratio,
            adjustment_factor: None,
            items,
        };
        let result = ProductionRecipeService::calculate_amounts(req).unwrap();
        // 用量 = 2 × 100 × 8 / 100 × 1 = 16 kg
        assert_eq!(result[0].amount, Decimal::from(16));
    }

    /// 测试用量计算（带加成系数）
    #[test]
    fn test_calculate_amounts_with_factor() {
        let fabric_weight = Decimal::from(200); // 200 kg
        let liquor_ratio = "1:10".to_string(); // 浴比 10
        let items = vec![RecipeMaterialItem {
            material_code: "D002".to_string(),
            material_name: "分散蓝".to_string(),
            concentration: Some(Decimal::from(3)), // 3% owf
            unit: "kg".to_string(),
            amount: Decimal::ZERO,
            category: "dye".to_string(),
        }];

        let req = CalculateAmountsRequest {
            fabric_weight,
            liquor_ratio,
            adjustment_factor: Some(Decimal::from(150) / Decimal::from(100)), // 1.50 加成
            items,
        };
        let result = ProductionRecipeService::calculate_amounts(req).unwrap();
        // 用量 = 3 × 200 × 10 / 100 × 1.5 = 90 kg
        assert_eq!(result[0].amount, Decimal::from(90));
    }

    /// 测试用量计算（助剂无浓度，保留原用量）
    #[test]
    fn test_calculate_amounts_auxiliary_no_concentration() {
        let fabric_weight = Decimal::from(100);
        let liquor_ratio = "1:8".to_string();
        let original_amount = Decimal::from(5);
        let items = vec![RecipeMaterialItem {
            material_code: "A001".to_string(),
            material_name: "匀染剂".to_string(),
            concentration: None, // 助剂无浓度
            unit: "kg".to_string(),
            amount: original_amount,
            category: "auxiliary".to_string(),
        }];

        let req = CalculateAmountsRequest {
            fabric_weight,
            liquor_ratio,
            adjustment_factor: None,
            items,
        };
        let result = ProductionRecipeService::calculate_amounts(req).unwrap();
        // 无浓度不重算，保留原用量
        assert_eq!(result[0].amount, original_amount);
    }

    /// 测试用量计算非法输入
    #[test]
    fn test_calculate_amounts_invalid() {
        // 备布重量 <= 0
        let req = CalculateAmountsRequest {
            fabric_weight: Decimal::ZERO,
            liquor_ratio: "1:8".to_string(),
            adjustment_factor: None,
            items: vec![],
        };
        assert!(ProductionRecipeService::calculate_amounts(req).is_err());

        // 浴比格式错误
        let req = CalculateAmountsRequest {
            fabric_weight: Decimal::from(100),
            liquor_ratio: "abc".to_string(),
            adjustment_factor: None,
            items: vec![],
        };
        assert!(ProductionRecipeService::calculate_amounts(req).is_err());

        // 加成系数 <= 0
        let req = CalculateAmountsRequest {
            fabric_weight: Decimal::from(100),
            liquor_ratio: "1:8".to_string(),
            adjustment_factor: Some(Decimal::ZERO),
            items: vec![],
        };
        assert!(ProductionRecipeService::calculate_amounts(req).is_err());

        // 浓度为负
        let req = CalculateAmountsRequest {
            fabric_weight: Decimal::from(100),
            liquor_ratio: "1:8".to_string(),
            adjustment_factor: None,
            items: vec![RecipeMaterialItem {
                material_code: "D001".to_string(),
                material_name: "活性红".to_string(),
                concentration: Some(Decimal::from(-1)),
                unit: "kg".to_string(),
                amount: Decimal::ZERO,
                category: "dye".to_string(),
            }],
        };
        assert!(ProductionRecipeService::calculate_amounts(req).is_err());
    }

    /// 测试大货处方状态流转合法性
    #[test]
    fn test_recipe_status_transition_valid() {
        // 合法流转
        assert!(ProductionRecipeService::validate_status_transition(
            recipe_status::DRAFT,
            recipe_status::APPROVED
        )
        .is_ok());
        assert!(ProductionRecipeService::validate_status_transition(
            recipe_status::DRAFT,
            recipe_status::CANCELLED
        )
        .is_ok());
        assert!(ProductionRecipeService::validate_status_transition(
            recipe_status::APPROVED,
            recipe_status::CLOSED
        )
        .is_ok());
    }

    /// 测试大货处方状态流转非法
    #[test]
    fn test_recipe_status_transition_invalid() {
        // 非法流转
        assert!(ProductionRecipeService::validate_status_transition(
            recipe_status::DRAFT,
            recipe_status::CLOSED
        )
        .is_err());
        assert!(ProductionRecipeService::validate_status_transition(
            recipe_status::APPROVED,
            recipe_status::DRAFT
        )
        .is_err());
        assert!(ProductionRecipeService::validate_status_transition(
            recipe_status::APPROVED,
            recipe_status::CANCELLED
        )
        .is_err());
        // 终态不可流转
        assert!(ProductionRecipeService::validate_status_transition(
            recipe_status::CLOSED,
            recipe_status::APPROVED
        )
        .is_err());
        assert!(ProductionRecipeService::validate_status_transition(
            recipe_status::CANCELLED,
            recipe_status::DRAFT
        )
        .is_err());
    }

    /// 测试大货处方更新/删除状态校验
    #[test]
    fn test_recipe_validate_can_update_and_delete() {
        // 仅 draft 可更新
        assert!(ProductionRecipeService::validate_can_update(recipe_status::DRAFT).is_ok());
        assert!(ProductionRecipeService::validate_can_update(recipe_status::APPROVED).is_err());
        assert!(ProductionRecipeService::validate_can_update(recipe_status::CLOSED).is_err());
        assert!(ProductionRecipeService::validate_can_update(recipe_status::CANCELLED).is_err());

        // 仅 draft 可删除
        assert!(ProductionRecipeService::validate_can_delete(recipe_status::DRAFT).is_ok());
        assert!(ProductionRecipeService::validate_can_delete(recipe_status::APPROVED).is_err());
        assert!(ProductionRecipeService::validate_can_delete(recipe_status::CLOSED).is_err());
    }

    /// 测试加料处方状态流转
    #[test]
    fn test_addition_status_transition() {
        // 合法流转
        assert!(ProductionRecipeAdditionService::validate_status_transition(
            addition_status::DRAFT,
            addition_status::APPROVED
        )
        .is_ok());
        assert!(ProductionRecipeAdditionService::validate_status_transition(
            addition_status::APPROVED,
            addition_status::CLOSED
        )
        .is_ok());

        // 非法流转
        assert!(ProductionRecipeAdditionService::validate_status_transition(
            addition_status::DRAFT,
            addition_status::CLOSED
        )
        .is_err());
        assert!(ProductionRecipeAdditionService::validate_status_transition(
            addition_status::APPROVED,
            addition_status::DRAFT
        )
        .is_err());
        // 终态
        assert!(ProductionRecipeAdditionService::validate_status_transition(
            addition_status::CLOSED,
            addition_status::APPROVED
        )
        .is_err());
    }

    /// 测试 FromPrimitive trait 可用（确保 rust_decimal::prelude::FromPrimitive 引入正确）
    #[test]
    fn test_decimal_from_f64() {
        let d = Decimal::from_f64(1.5).unwrap();
        assert_eq!(d, Decimal::from(15) / Decimal::from(10));
    }
}
