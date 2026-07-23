//! 染化料主数据 DTO 子模块（chemical_ops/types）
//!
//! 批次 490 D10-3a 拆分：从原 `chemical_service.rs` 迁移 12 个 DTO struct。
//! 包含染化料主数据 / 分类 / 批次 / 领用单 的 Create / Update / Query 请求体。

use rust_decimal::Decimal;
use serde::Deserialize;

// ============================================================================
// 染化料主数据 DTO
// ============================================================================

/// 创建染化料主数据请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateChemicalMasterRequest {
    pub chemical_code: String,
    pub chemical_name: String,
    pub chemical_name_en: Option<String>,
    pub chemical_type: String,
    pub category_id: Option<i32>,
    pub dye_category: Option<String>,
    pub color_index: Option<String>,
    pub auxiliary_category: Option<String>,
    pub cas_number: Option<String>,
    pub molecular_formula: Option<String>,
    pub molecular_weight: Option<Decimal>,
    pub specification: Option<String>,
    pub unit: Option<String>,
    pub standard_price: Option<Decimal>,
    pub cost_price: Option<Decimal>,
    pub ghs_classification: Option<String>,
    pub un_number: Option<String>,
    pub hazard_class: Option<String>,
    pub hazard_pictogram: Option<String>,
    pub signal_word: Option<String>,
    pub msds_url: Option<String>,
    pub msds_version: Option<String>,
    pub shelf_life_days: Option<i32>,
    pub storage_condition: Option<String>,
    pub storage_temperature: Option<String>,
    pub safety_stock: Option<Decimal>,
    pub reorder_point: Option<Decimal>,
    pub reorder_quantity: Option<Decimal>,
    pub package_unit: Option<String>,
    pub package_capacity: Option<Decimal>,
    pub packages_per_pallet: Option<i32>,
    pub supplier_id: Option<i32>,
    pub supplier_product_code: Option<String>,
    pub fastness_light: Option<String>,
    pub fastness_washing: Option<String>,
    pub active_ingredient: Option<String>,
    pub concentration: Option<Decimal>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 更新染化料主数据请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateChemicalMasterRequest {
    pub chemical_name: Option<String>,
    pub chemical_name_en: Option<String>,
    pub category_id: Option<i32>,
    pub dye_category: Option<String>,
    pub color_index: Option<String>,
    pub auxiliary_category: Option<String>,
    pub cas_number: Option<String>,
    pub molecular_formula: Option<String>,
    pub molecular_weight: Option<Decimal>,
    pub specification: Option<String>,
    pub unit: Option<String>,
    pub standard_price: Option<Decimal>,
    pub cost_price: Option<Decimal>,
    pub ghs_classification: Option<String>,
    pub un_number: Option<String>,
    pub hazard_class: Option<String>,
    pub hazard_pictogram: Option<String>,
    pub signal_word: Option<String>,
    pub msds_url: Option<String>,
    pub msds_version: Option<String>,
    pub shelf_life_days: Option<i32>,
    pub storage_condition: Option<String>,
    pub storage_temperature: Option<String>,
    pub safety_stock: Option<Decimal>,
    pub reorder_point: Option<Decimal>,
    pub reorder_quantity: Option<Decimal>,
    pub package_unit: Option<String>,
    pub package_capacity: Option<Decimal>,
    pub packages_per_pallet: Option<i32>,
    pub supplier_id: Option<i32>,
    pub supplier_product_code: Option<String>,
    pub fastness_light: Option<String>,
    pub fastness_washing: Option<String>,
    pub active_ingredient: Option<String>,
    pub concentration: Option<Decimal>,
    pub status: Option<String>,
    pub remarks: Option<String>,
}

/// 染化料主数据查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct ChemicalMasterQuery {
    pub chemical_type: Option<String>,
    pub category_id: Option<i32>,
    pub dye_category: Option<String>,
    pub auxiliary_category: Option<String>,
    pub supplier_id: Option<i32>,
    pub status: Option<String>,
    pub cas_number: Option<String>,
    pub ghs_classification: Option<String>,
    pub keyword: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

// ============================================================================
// 染化料分类 DTO
// ============================================================================

/// 创建染化料分类请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateChemicalCategoryRequest {
    pub category_code: String,
    pub category_name: String,
    pub parent_id: Option<i32>,
    pub category_type: String,
    pub description: Option<String>,
    pub sort_order: Option<i32>,
    pub created_by: Option<i32>,
}

/// 更新染化料分类请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateChemicalCategoryRequest {
    pub category_name: Option<String>,
    pub parent_id: Option<i32>,
    pub category_type: Option<String>,
    pub description: Option<String>,
    pub sort_order: Option<i32>,
    pub is_active: Option<bool>,
}

/// 染化料分类查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct ChemicalCategoryQuery {
    pub parent_id: Option<i32>,
    pub category_type: Option<String>,
    pub is_active: Option<bool>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

// ============================================================================
// 染化料批次 DTO
// ============================================================================

/// 创建染化料批次请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateChemicalLotRequest {
    pub lot_no: String,
    pub chemical_id: i32,
    pub supplier_id: Option<i32>,
    pub supplier_lot_no: Option<String>,
    pub production_date: Option<chrono::NaiveDate>,
    pub expiry_date: Option<chrono::NaiveDate>,
    pub received_date: Option<chrono::NaiveDate>,
    pub quantity_received: Option<Decimal>,
    pub unit_cost: Option<Decimal>,
    pub warehouse_id: Option<i32>,
    pub storage_zone: Option<String>,
    pub inspection_report_url: Option<String>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 更新染化料批次请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateChemicalLotRequest {
    pub supplier_id: Option<i32>,
    pub supplier_lot_no: Option<String>,
    pub production_date: Option<chrono::NaiveDate>,
    pub expiry_date: Option<chrono::NaiveDate>,
    pub received_date: Option<chrono::NaiveDate>,
    pub warehouse_id: Option<i32>,
    pub storage_zone: Option<String>,
    pub unit_cost: Option<Decimal>,
    pub inspection_report_url: Option<String>,
    pub remarks: Option<String>,
}

/// 染化料批次查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct ChemicalLotQuery {
    pub chemical_id: Option<i32>,
    pub supplier_id: Option<i32>,
    pub warehouse_id: Option<i32>,
    pub inspection_status: Option<String>,
    pub storage_zone: Option<String>,
    pub status: Option<String>,
    pub expiry_before: Option<chrono::NaiveDate>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

// ============================================================================
// 染化料领用单 DTO
// ============================================================================

/// 创建染化料领用单请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateChemicalRequisitionRequest {
    pub requisition_type: String,
    pub department_id: Option<i32>,
    pub requisition_date: chrono::NaiveDate,
    pub required_date: Option<chrono::NaiveDate>,
    pub dye_batch_id: Option<i32>,
    pub production_order_id: Option<i32>,
    pub total_amount: Option<Decimal>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 更新染化料领用单请求（仅 draft 状态可更新）
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateChemicalRequisitionRequest {
    pub department_id: Option<i32>,
    pub required_date: Option<chrono::NaiveDate>,
    pub dye_batch_id: Option<i32>,
    pub production_order_id: Option<i32>,
    pub total_amount: Option<Decimal>,
    pub remarks: Option<String>,
}

/// 染化料领用单查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct ChemicalRequisitionQuery {
    pub requisition_type: Option<String>,
    pub department_id: Option<i32>,
    pub dye_batch_id: Option<i32>,
    pub production_order_id: Option<i32>,
    pub status: Option<String>,
    pub requisition_date_start: Option<chrono::NaiveDate>,
    pub requisition_date_end: Option<chrono::NaiveDate>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}
