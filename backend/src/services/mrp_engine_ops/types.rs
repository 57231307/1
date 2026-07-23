//! MRP 引擎数据结构
//!
//! 批次 490 D10-3b 拆分：从 mrp_engine_service.rs 抽取的全部 pub struct 定义。
//! 包含请求/响应/参数对象，供 ops 子模块和 facade 共享。

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// MRP计算请求
#[derive(Debug, Clone, Deserialize)]
pub struct MrpCalculationRequest {
    pub items: Vec<MrpCalculationItem>,
    pub source_type: String,
    pub source_id: Option<i32>,
    pub consider_safety_stock: bool,
    pub consider_in_transit: bool,
}

/// MRP计算项
#[derive(Debug, Clone, Deserialize)]
pub struct MrpCalculationItem {
    pub product_id: i32,
    pub required_quantity: Decimal,
    pub required_date: NaiveDate,
}

/// 物料需求计算结果
#[derive(Debug, Clone, Serialize)]
pub struct MaterialRequirement {
    pub product_id: i32,
    pub required_quantity: Decimal,
    pub required_date: NaiveDate,
    pub on_hand_quantity: Decimal,
    pub in_transit_quantity: Decimal,
    pub safety_stock: Decimal,
    pub available_quantity: Decimal,
    pub shortage_quantity: Decimal,
    pub source_type: String,
    pub source_id: Option<i32>,
    pub bom_level: i32,
}

/// MRP计算结果摘要
#[derive(Debug, Clone, Serialize)]
pub struct MrpCalculationSummary {
    pub calculation_no: String,
    pub total_items: i32,
    pub items_with_shortage: i32,
    pub results: Vec<crate::models::mrp_result::Model>,
    pub requirements: Vec<MaterialRequirement>,
}

/// 库存信息
///
/// 批次 490 D10-3b 拆分：原 private struct 提升为 `pub(crate)`，供 ops 子模块和测试模块共享。
/// 仅 crate 内可见，不对外暴露（facade 不重导出此类型，保持原 API 表面不变）。
#[derive(Debug, Clone)]
pub(crate) struct StockInfo {
    pub(crate) on_hand: Decimal,
    pub(crate) in_transit: Decimal,
    pub(crate) safety_stock: Decimal,
    pub(crate) available: Decimal,
}

/// 物料需求计算参数对象
///
/// 批次 336 v10 复审 P3 修复：引入参数对象消除 calculate_requirement 的 too_many_arguments 警告。
/// 聚合物料需求计算所需的全部参数，避免函数签名携带 8 个参数。
#[derive(Debug, Clone)]
pub struct RequirementCalcParams {
    /// 产品 ID
    pub product_id: i32,
    /// 需求数量
    pub required_quantity: Decimal,
    /// 需求日期
    pub required_date: NaiveDate,
    /// 来源类型（如订单/生产计划）
    pub source_type: String,
    /// 来源 ID（可选）
    pub source_id: Option<i32>,
    /// 是否考虑安全库存
    pub consider_safety_stock: bool,
    /// 是否考虑在途库存
    pub consider_in_transit: bool,
    /// BOM 层级（顶层为 0）
    pub bom_level: i32,
}

/// BOM 递归展开参数对象
///
/// 批次 339 v10 复审 P3 修复：引入参数对象消除 explode_bom_recursive 的 too_many_arguments 警告。
/// 聚合递归展开 BOM 所需的标量参数，&mut 借用参数（results / stock_cache）保留为独立参数。
/// 使用生命周期 `&'a str` 借用 source_type，避免递归调用中的不必要的 to_string()。
#[derive(Debug, Clone)]
pub struct ExplodeBomArgs<'a> {
    /// 产品 ID
    pub product_id: i32,
    /// 父级需求数量
    pub parent_quantity: Decimal,
    /// 需求日期
    pub required_date: NaiveDate,
    /// 来源类型
    pub source_type: &'a str,
    /// 来源 ID
    pub source_id: Option<i32>,
    /// 当前 BOM 层级
    pub current_level: i32,
    /// 最大 BOM 层级
    pub max_level: i32,
    /// 是否考虑安全库存
    pub consider_safety_stock: bool,
    /// 是否考虑在途库存
    pub consider_in_transit: bool,
}

/// BOM 展开查询参数（公开接口层，owned 版本）
///
/// 批次 413 技术债务清理：引入参数对象消除 explode_bom 的 too_many_arguments 警告。
/// 与内部 `ExplodeBomArgs<'a>` 区分：此结构体为 owned 版本（source_type 为 String），
/// 不含 current_level/max_level（由 explode_bom 内部固定为 1/10）。
#[derive(Debug, Clone)]
pub struct MrpExplodeQuery {
    /// 产品 ID
    pub product_id: i32,
    /// 父级需求数量
    pub parent_quantity: Decimal,
    /// 需求日期
    pub required_date: NaiveDate,
    /// 来源类型（如订单/生产计划）
    pub source_type: String,
    /// 来源 ID（可选）
    pub source_id: Option<i32>,
    /// 是否考虑安全库存
    pub consider_safety_stock: bool,
    /// 是否考虑在途库存
    pub consider_in_transit: bool,
}

/// MRP 计算查询参数（公开接口层）
///
/// 批次 413 技术债务清理：引入参数对象消除 run_mrp_calculation 的 too_many_arguments 警告。
/// 与 `RequirementCalcParams` 区分：此结构体为公开接口参数，不含 bom_level（由内部固定为 0）。
#[derive(Debug, Clone)]
pub struct MrpCalculationQuery {
    /// 产品 ID
    pub product_id: i32,
    /// 需求数量
    pub required_quantity: Decimal,
    /// 需求日期
    pub required_date: NaiveDate,
    /// 来源类型（如订单/生产计划）
    pub source_type: String,
    /// 来源 ID（可选）
    pub source_id: Option<i32>,
    /// 是否考虑安全库存
    pub consider_safety_stock: bool,
    /// 是否考虑在途库存
    pub consider_in_transit: bool,
}
