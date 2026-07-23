//! 染化料主数据 Service（facade）
//!
//! v14 批次 429：染化料主数据完善
//! 依据：面料行业真实业务调研文档 §4.3 染化料管理 + §11.4 染化料主数据管理
//! 真实业务流程：
//!   染化料分类树（dye/auxiliary/chemical）→ 染化料主数据（GHS 危化品 + MSDS + 保质期 + 安全库存）
//!   → 来料批次（lot_no + 效期 + 来料检验状态）→ 领用单（关联染色缸号）
//!
//! 核心能力：
//! - 染化料主数据 CRUD + 状态流转（active → inactive / discontinued）+ 安全库存检查
//! - 染化料分类 CRUD + 树形结构
//! - 染化料批次 CRUD + 效期管理 + 来料检验状态流转（pending → passed/failed/quarantine）
//! - 染化料领用单 CRUD + 状态机（draft → approved → issued → partial_returned → closed）+ 取消
//!
//! 复用现有功能（§10.0.1）：
//! - chemical_master 表：本批次新建（含 GHS 危化品标注 + MSDS 安全数据表）
//! - suppliers 表：染化料供应商关联（批次 1 已建）
//! - warehouses 表：批次存储仓库关联（批次 1 已建）
//! - dye_batch 表：领用单关联染色缸号（批次 28 已建）
//! - production_orders 表：领用单关联生产订单（批次 1 已建）
//!
//! 批次 490 D10-3a 拆分：本文件作为 facade，保留纯函数 + Service struct + new 构造函数 + 测试。
//! 4 个 Service 的 impl 块迁移至 `chemical_ops` 子模块（master / category / lot / requisition）。
//! DTO struct 迁移至 `chemical_ops::types`，本 facade 通过 `pub use` 二次 re-export 保持外部引用路径不变。

use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::models::status::chemical_inspection_status;
use crate::models::status::chemical_lot_status;
use crate::models::status::chemical_requisition_status;
use crate::models::status::chemical_requisition_type;
use crate::models::status::chemical_type;
use crate::utils::error::AppError;

// re-export DTOs 与 ops 子模块，保持外部 `use crate::services::chemical_service::{...}` 路径不变
pub use crate::services::chemical_ops::{
    ChemicalCategoryQuery, ChemicalLotQuery, ChemicalMasterQuery, ChemicalRequisitionQuery,
    CreateChemicalCategoryRequest, CreateChemicalLotRequest, CreateChemicalMasterRequest,
    CreateChemicalRequisitionRequest, UpdateChemicalCategoryRequest, UpdateChemicalLotRequest,
    UpdateChemicalMasterRequest, UpdateChemicalRequisitionRequest,
};

// ============================================================================
// 染化料计算纯函数
// ============================================================================

/// 计算批次剩余保质期天数
///
/// 业务规则：
/// - 若失效日期为 None，返回 None（无保质期限制）
/// - 否则返回 (失效日期 - 当前日期) 的天数
/// - 若已过期，返回负数
pub fn compute_remaining_shelf_life(
    expiry_date: Option<chrono::NaiveDate>,
    today: chrono::NaiveDate,
) -> Option<i64> {
    expiry_date.map(|d| (d - today).num_days())
}

/// 计算批次总成本（接收数量 × 单位成本）
pub fn compute_total_cost(quantity_received: Decimal, unit_cost: Decimal) -> Decimal {
    quantity_received * unit_cost
}

/// 校验染化料类型是否合法
pub fn validate_chemical_type(chemical_type: &str) -> Result<(), AppError> {
    let valid_types = [
        chemical_type::DYE,
        chemical_type::AUXILIARY,
        chemical_type::CHEMICAL,
    ];
    if !valid_types.contains(&chemical_type) {
        return Err(AppError::business(format!(
            "染化料类型必须是 dye / auxiliary / chemical，当前: {}",
            chemical_type
        )));
    }
    Ok(())
}

/// 校验来料检验状态是否合法
pub fn validate_inspection_status(status: &str) -> Result<(), AppError> {
    let valid = [
        chemical_inspection_status::PENDING,
        chemical_inspection_status::PASSED,
        chemical_inspection_status::FAILED,
        chemical_inspection_status::QUARANTINE,
    ];
    if !valid.contains(&status) {
        return Err(AppError::business(format!(
            "来料检验状态必须是 pending / passed / failed / quarantine，当前: {}",
            status
        )));
    }
    Ok(())
}

/// 校验批次状态是否合法
pub fn validate_lot_status(status: &str) -> Result<(), AppError> {
    let valid = [
        chemical_lot_status::ACTIVE,
        chemical_lot_status::CONSUMED,
        chemical_lot_status::EXPIRED,
        chemical_lot_status::SCRAPPED,
    ];
    if !valid.contains(&status) {
        return Err(AppError::business(format!(
            "批次状态必须是 active / consumed / expired / scrapped，当前: {}",
            status
        )));
    }
    Ok(())
}

/// 校验领用单类型是否合法
pub fn validate_requisition_type(requisition_type: &str) -> Result<(), AppError> {
    let valid = [
        chemical_requisition_type::PRODUCTION,
        chemical_requisition_type::LAB,
        chemical_requisition_type::RD,
    ];
    if !valid.contains(&requisition_type) {
        return Err(AppError::business(format!(
            "领用单类型必须是 production / lab / rd，当前: {}",
            requisition_type
        )));
    }
    Ok(())
}

/// 校验领用单状态是否合法
pub fn validate_requisition_status(status: &str) -> Result<(), AppError> {
    let valid = [
        chemical_requisition_status::DRAFT,
        chemical_requisition_status::APPROVED,
        chemical_requisition_status::ISSUED,
        chemical_requisition_status::PARTIAL_RETURNED,
        chemical_requisition_status::CLOSED,
        chemical_requisition_status::CANCELLED,
    ];
    if !valid.contains(&status) {
        return Err(AppError::business(format!(
            "领用单状态必须是 draft / approved / issued / partial_returned / closed / cancelled，当前: {}",
            status
        )));
    }
    Ok(())
}

/// 检查染化料是否低库存
///
/// 业务规则：
/// - 返回 (低于安全库存, 低于再订货点)
/// - 低于安全库存触发紧急预警
/// - 低于再订货点触发采购建议
pub fn check_low_stock(
    available: Decimal,
    safety_stock: Decimal,
    reorder_point: Decimal,
) -> (bool, bool) {
    let below_safety = available < safety_stock;
    let below_reorder = available < reorder_point;
    (below_safety, below_reorder)
}

// ============================================================================
// 染化料 Service struct 定义（impl 块在 chemical_ops 子模块）
// ============================================================================

/// 染化料主数据 Service
pub struct ChemicalMasterService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl ChemicalMasterService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

/// 染化料分类 Service
pub struct ChemicalCategoryService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl ChemicalCategoryService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

/// 染化料批次 Service
pub struct ChemicalLotService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl ChemicalLotService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

/// 染化料领用单 Service
pub struct ChemicalRequisitionService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl ChemicalRequisitionService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rust_decimal::Decimal;

    #[test]
    fn 测试计算剩余保质期_未过期() {
        let expiry = NaiveDate::from_ymd_opt(2025, 12, 31);
        let today = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let result = compute_remaining_shelf_life(expiry, today);
        assert_eq!(result, Some(364));
    }

    #[test]
    fn 测试计算剩余保质期_已过期返回负数() {
        let expiry = NaiveDate::from_ymd_opt(2025, 1, 1);
        let today = NaiveDate::from_ymd_opt(2025, 12, 31).unwrap();
        let result = compute_remaining_shelf_life(expiry, today);
        assert_eq!(result, Some(-364));
    }

    #[test]
    fn 测试计算剩余保质期_无失效日期返回None() {
        let today = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let result = compute_remaining_shelf_life(None, today);
        assert_eq!(result, None);
    }

    #[test]
    fn 测试计算总成本() {
        let result = compute_total_cost(Decimal::new(100, 0), Decimal::new(12, 1));
        assert_eq!(result, Decimal::new(1200, 0));
    }

    #[test]
    fn 测试计算总成本_零数量() {
        let result = compute_total_cost(Decimal::ZERO, Decimal::new(12, 1));
        assert_eq!(result, Decimal::ZERO);
    }

    #[test]
    fn 测试校验染化料类型_合法() {
        assert!(validate_chemical_type("dye").is_ok());
        assert!(validate_chemical_type("auxiliary").is_ok());
        assert!(validate_chemical_type("chemical").is_ok());
    }

    #[test]
    fn 测试校验染化料类型_非法() {
        assert!(validate_chemical_type("invalid").is_err());
    }

    #[test]
    fn 测试校验来料检验状态_合法() {
        assert!(validate_inspection_status("pending").is_ok());
        assert!(validate_inspection_status("passed").is_ok());
        assert!(validate_inspection_status("failed").is_ok());
        assert!(validate_inspection_status("quarantine").is_ok());
    }

    #[test]
    fn 测试校验来料检验状态_非法() {
        assert!(validate_inspection_status("invalid").is_err());
    }

    #[test]
    fn 测试校验批次状态_合法() {
        assert!(validate_lot_status("active").is_ok());
        assert!(validate_lot_status("consumed").is_ok());
        assert!(validate_lot_status("expired").is_ok());
        assert!(validate_lot_status("scrapped").is_ok());
    }

    #[test]
    fn 测试校验批次状态_非法() {
        assert!(validate_lot_status("invalid").is_err());
    }

    #[test]
    fn 测试校验领用单类型_合法() {
        assert!(validate_requisition_type("production").is_ok());
        assert!(validate_requisition_type("lab").is_ok());
        assert!(validate_requisition_type("rd").is_ok());
    }

    #[test]
    fn 测试校验领用单类型_非法() {
        assert!(validate_requisition_type("invalid").is_err());
    }

    #[test]
    fn 测试校验领用单状态_合法() {
        assert!(validate_requisition_status("draft").is_ok());
        assert!(validate_requisition_status("approved").is_ok());
        assert!(validate_requisition_status("issued").is_ok());
        assert!(validate_requisition_status("partial_returned").is_ok());
        assert!(validate_requisition_status("closed").is_ok());
        assert!(validate_requisition_status("cancelled").is_ok());
    }

    #[test]
    fn 测试校验领用单状态_非法() {
        assert!(validate_requisition_status("invalid").is_err());
    }

    #[test]
    fn 测试低库存检查_低于安全库存() {
        let (below_safety, below_reorder) =
            check_low_stock(Decimal::new(5, 0), Decimal::new(10, 0), Decimal::new(20, 0));
        assert!(below_safety);
        assert!(below_reorder);
    }

    #[test]
    fn 测试低库存检查_低于再订货点但高于安全库存() {
        let (below_safety, below_reorder) =
            check_low_stock(Decimal::new(15, 0), Decimal::new(10, 0), Decimal::new(20, 0));
        assert!(!below_safety);
        assert!(below_reorder);
    }

    #[test]
    fn 测试低库存检查_正常库存() {
        let (below_safety, below_reorder) =
            check_low_stock(Decimal::new(50, 0), Decimal::new(10, 0), Decimal::new(20, 0));
        assert!(!below_safety);
        assert!(!below_reorder);
    }
}
