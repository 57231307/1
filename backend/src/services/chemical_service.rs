//! 染化料主数据 Service
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

use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    Set,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::models::chemical_category::{
    self, ActiveModel as CategoryActiveModel, Entity as CategoryEntity, Model as CategoryModel,
};
use crate::models::chemical_lot::{
    self, ActiveModel as LotActiveModel, Entity as LotEntity, Model as LotModel,
};
use crate::models::chemical_master::{
    self, ActiveModel as MasterActiveModel, Entity as MasterEntity, Model as MasterModel,
};
use crate::models::chemical_requisition::{
    self, ActiveModel as RequisitionActiveModel, Entity as RequisitionEntity,
    Model as RequisitionModel,
};
use crate::models::status::chemical_inspection_status;
use crate::models::status::chemical_lot_status;
use crate::models::status::chemical_requisition_status;
use crate::models::status::chemical_requisition_type;
use crate::models::status::chemical_status;
use crate::models::status::chemical_type;
use crate::utils::error::AppError;

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
// 染化料主数据 Service
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

/// 染化料主数据 Service
pub struct ChemicalMasterService {
    db: Arc<DatabaseConnection>,
}

impl ChemicalMasterService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建染化料主数据
    pub async fn create(&self, req: CreateChemicalMasterRequest) -> Result<MasterModel, AppError> {
        // 校验染化料类型
        validate_chemical_type(&req.chemical_type)?;

        // 校验染料类型时 dye_category 必须提供
        if req.chemical_type == chemical_type::DYE && req.dye_category.is_none() {
            return Err(AppError::business("染料类型必须提供 dye_category"));
        }
        // 校验助剂类型时 auxiliary_category 必须提供
        if req.chemical_type == chemical_type::AUXILIARY && req.auxiliary_category.is_none() {
            return Err(AppError::business("助剂类型必须提供 auxiliary_category"));
        }

        // 校验标准价、成本价非负
        let standard_price = req.standard_price.unwrap_or(Decimal::ZERO);
        if standard_price < Decimal::ZERO {
            return Err(AppError::business("标准价不能为负"));
        }
        let cost_price = req.cost_price.unwrap_or(Decimal::ZERO);
        if cost_price < Decimal::ZERO {
            return Err(AppError::business("成本价不能为负"));
        }

        // 校验安全库存相关字段非负
        let safety_stock = req.safety_stock.unwrap_or(Decimal::ZERO);
        if safety_stock < Decimal::ZERO {
            return Err(AppError::business("安全库存不能为负"));
        }
        let reorder_point = req.reorder_point.unwrap_or(Decimal::ZERO);
        if reorder_point < Decimal::ZERO {
            return Err(AppError::business("再订货点不能为负"));
        }
        let reorder_quantity = req.reorder_quantity.unwrap_or(Decimal::ZERO);
        if reorder_quantity < Decimal::ZERO {
            return Err(AppError::business("再订货量不能为负"));
        }

        // 校验编码唯一性
        if let Some(_existing) = MasterEntity::find()
            .filter(chemical_master::Column::ChemicalCode.eq(&req.chemical_code))
            .filter(chemical_master::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
        {
            return Err(AppError::business(format!(
                "染化料编码 {} 已存在",
                req.chemical_code
            )));
        }

        let now = crate::utils::date_utils::utc_now_fixed();
        let unit = req.unit.unwrap_or_else(|| "kg".to_string());
        // MSDS 更新时间：仅在提供 msds_url 时记录
        let msds_updated_at_value = if req.msds_url.is_some() {
            Some(now)
        } else {
            None
        };

        let active = MasterActiveModel {
            id: Default::default(),
            chemical_code: Set(req.chemical_code),
            chemical_name: Set(req.chemical_name),
            chemical_name_en: Set(req.chemical_name_en),
            chemical_type: Set(req.chemical_type),
            category_id: Set(req.category_id),
            dye_category: Set(req.dye_category),
            color_index: Set(req.color_index),
            auxiliary_category: Set(req.auxiliary_category),
            cas_number: Set(req.cas_number),
            molecular_formula: Set(req.molecular_formula),
            molecular_weight: Set(req.molecular_weight),
            specification: Set(req.specification),
            unit: Set(unit),
            standard_price: Set(standard_price),
            cost_price: Set(cost_price),
            ghs_classification: Set(req.ghs_classification),
            un_number: Set(req.un_number),
            hazard_class: Set(req.hazard_class),
            hazard_pictogram: Set(req.hazard_pictogram),
            signal_word: Set(req.signal_word),
            msds_url: Set(req.msds_url),
            msds_version: Set(req.msds_version),
            msds_updated_at: Set(msds_updated_at_value),
            shelf_life_days: Set(req.shelf_life_days),
            storage_condition: Set(req.storage_condition),
            storage_temperature: Set(req.storage_temperature),
            safety_stock: Set(safety_stock),
            reorder_point: Set(reorder_point),
            reorder_quantity: Set(reorder_quantity),
            package_unit: Set(req.package_unit),
            package_capacity: Set(req.package_capacity),
            packages_per_pallet: Set(req.packages_per_pallet),
            supplier_id: Set(req.supplier_id),
            supplier_product_code: Set(req.supplier_product_code),
            fastness_light: Set(req.fastness_light),
            fastness_washing: Set(req.fastness_washing),
            active_ingredient: Set(req.active_ingredient),
            concentration: Set(req.concentration),
            status: Set(chemical_status::ACTIVE.to_string()),
            remarks: Set(req.remarks),
            is_deleted: Set(false),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("染化料主数据创建失败: {}", e)))?;
        Ok(result)
    }

    /// 更新染化料主数据
    pub async fn update(
        &self,
        id: i32,
        req: UpdateChemicalMasterRequest,
    ) -> Result<MasterModel, AppError> {
        let model = self.get_by_id(id).await?;

        let mut active: MasterActiveModel = model.into();

        if let Some(v) = req.chemical_name {
            active.chemical_name = Set(v);
        }
        if let Some(v) = req.chemical_name_en {
            active.chemical_name_en = Set(Some(v));
        }
        if let Some(v) = req.category_id {
            active.category_id = Set(Some(v));
        }
        if let Some(v) = req.dye_category {
            active.dye_category = Set(Some(v));
        }
        if let Some(v) = req.color_index {
            active.color_index = Set(Some(v));
        }
        if let Some(v) = req.auxiliary_category {
            active.auxiliary_category = Set(Some(v));
        }
        if let Some(v) = req.cas_number {
            active.cas_number = Set(Some(v));
        }
        if let Some(v) = req.molecular_formula {
            active.molecular_formula = Set(Some(v));
        }
        if let Some(v) = req.molecular_weight {
            active.molecular_weight = Set(Some(v));
        }
        if let Some(v) = req.specification {
            active.specification = Set(Some(v));
        }
        if let Some(v) = req.unit {
            active.unit = Set(v);
        }
        if let Some(v) = req.standard_price {
            if v < Decimal::ZERO {
                return Err(AppError::business("标准价不能为负"));
            }
            active.standard_price = Set(v);
        }
        if let Some(v) = req.cost_price {
            if v < Decimal::ZERO {
                return Err(AppError::business("成本价不能为负"));
            }
            active.cost_price = Set(v);
        }
        if let Some(v) = req.ghs_classification {
            active.ghs_classification = Set(Some(v));
        }
        if let Some(v) = req.un_number {
            active.un_number = Set(Some(v));
        }
        if let Some(v) = req.hazard_class {
            active.hazard_class = Set(Some(v));
        }
        if let Some(v) = req.hazard_pictogram {
            active.hazard_pictogram = Set(Some(v));
        }
        if let Some(v) = req.signal_word {
            active.signal_word = Set(Some(v));
        }
        if let Some(v) = req.msds_url {
            active.msds_url = Set(Some(v));
            active.msds_updated_at = Set(Some(crate::utils::date_utils::utc_now_fixed()));
        }
        if let Some(v) = req.msds_version {
            active.msds_version = Set(Some(v));
        }
        if let Some(v) = req.shelf_life_days {
            active.shelf_life_days = Set(Some(v));
        }
        if let Some(v) = req.storage_condition {
            active.storage_condition = Set(Some(v));
        }
        if let Some(v) = req.storage_temperature {
            active.storage_temperature = Set(Some(v));
        }
        if let Some(v) = req.safety_stock {
            if v < Decimal::ZERO {
                return Err(AppError::business("安全库存不能为负"));
            }
            active.safety_stock = Set(v);
        }
        if let Some(v) = req.reorder_point {
            if v < Decimal::ZERO {
                return Err(AppError::business("再订货点不能为负"));
            }
            active.reorder_point = Set(v);
        }
        if let Some(v) = req.reorder_quantity {
            if v < Decimal::ZERO {
                return Err(AppError::business("再订货量不能为负"));
            }
            active.reorder_quantity = Set(v);
        }
        if let Some(v) = req.package_unit {
            active.package_unit = Set(Some(v));
        }
        if let Some(v) = req.package_capacity {
            active.package_capacity = Set(Some(v));
        }
        if let Some(v) = req.packages_per_pallet {
            active.packages_per_pallet = Set(Some(v));
        }
        if let Some(v) = req.supplier_id {
            active.supplier_id = Set(Some(v));
        }
        if let Some(v) = req.supplier_product_code {
            active.supplier_product_code = Set(Some(v));
        }
        if let Some(v) = req.fastness_light {
            active.fastness_light = Set(Some(v));
        }
        if let Some(v) = req.fastness_washing {
            active.fastness_washing = Set(Some(v));
        }
        if let Some(v) = req.active_ingredient {
            active.active_ingredient = Set(Some(v));
        }
        if let Some(v) = req.concentration {
            active.concentration = Set(Some(v));
        }
        if let Some(v) = req.status {
            if v != chemical_status::ACTIVE
                && v != chemical_status::INACTIVE
                && v != chemical_status::DISCONTINUED
            {
                return Err(AppError::business(format!(
                    "染化料状态必须是 active / inactive / discontinued，当前: {}",
                    v
                )));
            }
            active.status = Set(v);
        }
        if let Some(v) = req.remarks {
            active.remarks = Set(Some(v));
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 软删除染化料主数据
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        let mut active: MasterActiveModel = model.into();
        active.is_deleted = Set(true);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        active.update(&*self.db).await?;
        Ok(())
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<MasterModel, AppError> {
        MasterEntity::find_by_id(id)
            .filter(chemical_master::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("染化料 {} 不存在", id)))
    }

    /// 按编码查询
    pub async fn get_by_code(&self, chemical_code: &str) -> Result<MasterModel, AppError> {
        MasterEntity::find()
            .filter(chemical_master::Column::ChemicalCode.eq(chemical_code))
            .filter(chemical_master::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!("染化料编码 {} 不存在", chemical_code))
            })
    }

    /// 分页查询
    pub async fn list(
        &self,
        query: ChemicalMasterQuery,
    ) -> Result<(Vec<MasterModel>, u64), AppError> {
        let mut q = MasterEntity::find()
            .filter(chemical_master::Column::IsDeleted.eq(false));
        if let Some(v) = query.chemical_type {
            q = q.filter(chemical_master::Column::ChemicalType.eq(v));
        }
        if let Some(v) = query.category_id {
            q = q.filter(chemical_master::Column::CategoryId.eq(v));
        }
        if let Some(v) = query.dye_category {
            q = q.filter(chemical_master::Column::DyeCategory.eq(v));
        }
        if let Some(v) = query.auxiliary_category {
            q = q.filter(chemical_master::Column::AuxiliaryCategory.eq(v));
        }
        if let Some(v) = query.supplier_id {
            q = q.filter(chemical_master::Column::SupplierId.eq(v));
        }
        if let Some(v) = query.status {
            q = q.filter(chemical_master::Column::Status.eq(v));
        }
        if let Some(v) = query.cas_number {
            q = q.filter(chemical_master::Column::CasNumber.eq(v));
        }
        if let Some(v) = query.ghs_classification {
            q = q.filter(chemical_master::Column::GhsClassification.eq(v));
        }
        if let Some(kw) = query.keyword {
            q = q.filter(
                Condition::any()
                    .add(chemical_master::Column::ChemicalCode.contains(&kw))
                    .add(chemical_master::Column::ChemicalName.contains(&kw))
                    .add(chemical_master::Column::ChemicalNameEn.contains(&kw)),
            );
        }

        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

        let total = q.clone().count(&*self.db).await?;
        let items = q
            .order_by_desc(chemical_master::Column::Id)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;
        Ok((items, total))
    }
}

// ============================================================================
// 染化料分类 Service
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

/// 染化料分类 Service
pub struct ChemicalCategoryService {
    db: Arc<DatabaseConnection>,
}

impl ChemicalCategoryService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建染化料分类
    pub async fn create(&self, req: CreateChemicalCategoryRequest) -> Result<CategoryModel, AppError> {
        validate_chemical_type(&req.category_type)?;

        // 校验父分类存在（若提供）
        if let Some(parent_id) = req.parent_id {
            if CategoryEntity::find_by_id(parent_id)
                .filter(chemical_category::Column::IsDeleted.eq(false))
                .one(&*self.db)
                .await?
                .is_none()
            {
                return Err(AppError::business(format!("父分类 {} 不存在", parent_id)));
            }
        }

        // 校验编码唯一性
        if let Some(_existing) = CategoryEntity::find()
            .filter(chemical_category::Column::CategoryCode.eq(&req.category_code))
            .filter(chemical_category::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
        {
            return Err(AppError::business(format!(
                "分类编码 {} 已存在",
                req.category_code
            )));
        }

        let now = crate::utils::date_utils::utc_now_fixed();

        let active = CategoryActiveModel {
            id: Default::default(),
            category_code: Set(req.category_code),
            category_name: Set(req.category_name),
            parent_id: Set(req.parent_id),
            category_type: Set(req.category_type),
            description: Set(req.description),
            sort_order: Set(req.sort_order.unwrap_or(0)),
            is_active: Set(true),
            is_deleted: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("染化料分类创建失败: {}", e)))?;
        Ok(result)
    }

    /// 更新染化料分类
    pub async fn update(
        &self,
        id: i32,
        req: UpdateChemicalCategoryRequest,
    ) -> Result<CategoryModel, AppError> {
        let model = self.get_by_id(id).await?;
        let mut active: CategoryActiveModel = model.into();

        if let Some(v) = req.category_name {
            active.category_name = Set(v);
        }
        if let Some(v) = req.parent_id {
            // 禁止将自身设为父分类
            if v == id {
                return Err(AppError::business("不能将自身设为父分类"));
            }
            active.parent_id = Set(Some(v));
        }
        if let Some(v) = req.category_type {
            validate_chemical_type(&v)?;
            active.category_type = Set(v);
        }
        if let Some(v) = req.description {
            active.description = Set(Some(v));
        }
        if let Some(v) = req.sort_order {
            active.sort_order = Set(v);
        }
        if let Some(v) = req.is_active {
            active.is_active = Set(v);
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 软删除染化料分类
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        // 校验是否有子分类
        let children_count = CategoryEntity::find()
            .filter(chemical_category::Column::ParentId.eq(id))
            .filter(chemical_category::Column::IsDeleted.eq(false))
            .count(&*self.db)
            .await?;
        if children_count > 0 {
            return Err(AppError::business("存在子分类，无法删除"));
        }

        let model = self.get_by_id(id).await?;
        let mut active: CategoryActiveModel = model.into();
        active.is_deleted = Set(true);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        active.update(&*self.db).await?;
        Ok(())
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<CategoryModel, AppError> {
        CategoryEntity::find_by_id(id)
            .filter(chemical_category::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("染化料分类 {} 不存在", id)))
    }

    /// 分页查询
    pub async fn list(
        &self,
        query: ChemicalCategoryQuery,
    ) -> Result<(Vec<CategoryModel>, u64), AppError> {
        let mut q = CategoryEntity::find()
            .filter(chemical_category::Column::IsDeleted.eq(false));
        if let Some(v) = query.parent_id {
            q = q.filter(chemical_category::Column::ParentId.eq(v));
        }
        if let Some(v) = query.category_type {
            q = q.filter(chemical_category::Column::CategoryType.eq(v));
        }
        if let Some(v) = query.is_active {
            q = q.filter(chemical_category::Column::IsActive.eq(v));
        }

        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

        let total = q.clone().count(&*self.db).await?;
        let items = q
            .order_by_asc(chemical_category::Column::SortOrder)
            .order_by_desc(chemical_category::Column::Id)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;
        Ok((items, total))
    }

    /// 查询分类树（按 parent_id 查询子分类）
    pub async fn get_tree(&self, parent_id: Option<i32>) -> Result<Vec<CategoryModel>, AppError> {
        let mut q = CategoryEntity::find()
            .filter(chemical_category::Column::IsDeleted.eq(false))
            .filter(chemical_category::Column::IsActive.eq(true));
        if let Some(pid) = parent_id {
            q = q.filter(chemical_category::Column::ParentId.eq(pid));
        } else {
            q = q.filter(chemical_category::Column::ParentId.is_null());
        }
        let items = q
            .order_by_asc(chemical_category::Column::SortOrder)
            .order_by_desc(chemical_category::Column::Id)
            .all(&*self.db)
            .await?;
        Ok(items)
    }
}

// ============================================================================
// 染化料批次 Service
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

/// 染化料批次 Service
pub struct ChemicalLotService {
    db: Arc<DatabaseConnection>,
}

impl ChemicalLotService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建染化料批次
    pub async fn create(&self, req: CreateChemicalLotRequest) -> Result<LotModel, AppError> {
        // 校验染化料主数据存在
        if MasterEntity::find_by_id(req.chemical_id)
            .filter(chemical_master::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .is_none()
        {
            return Err(AppError::business(format!(
                "染化料 {} 不存在",
                req.chemical_id
            )));
        }

        // 校验批号唯一性
        if let Some(_existing) = LotEntity::find()
            .filter(chemical_lot::Column::LotNo.eq(&req.lot_no))
            .filter(chemical_lot::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
        {
            return Err(AppError::business(format!("批号 {} 已存在", req.lot_no)));
        }

        // 校验数量非负
        let quantity_received = req.quantity_received.unwrap_or(Decimal::ZERO);
        if quantity_received < Decimal::ZERO {
            return Err(AppError::business("接收数量不能为负"));
        }
        let unit_cost = req.unit_cost.unwrap_or(Decimal::ZERO);
        if unit_cost < Decimal::ZERO {
            return Err(AppError::business("单位成本不能为负"));
        }

        // 计算总成本
        let total_cost = compute_total_cost(quantity_received, unit_cost);

        let now = crate::utils::date_utils::utc_now_fixed();

        let active = LotActiveModel {
            id: Default::default(),
            lot_no: Set(req.lot_no),
            chemical_id: Set(req.chemical_id),
            supplier_id: Set(req.supplier_id),
            supplier_lot_no: Set(req.supplier_lot_no),
            production_date: Set(req.production_date),
            expiry_date: Set(req.expiry_date),
            received_date: Set(req.received_date),
            quantity_received: Set(quantity_received),
            quantity_available: Set(quantity_received),
            quantity_reserved: Set(Decimal::ZERO),
            inspection_status: Set(chemical_inspection_status::PENDING.to_string()),
            inspection_report_url: Set(req.inspection_report_url),
            unit_cost: Set(unit_cost),
            total_cost: Set(total_cost),
            warehouse_id: Set(req.warehouse_id),
            storage_zone: Set(req.storage_zone),
            status: Set(chemical_lot_status::ACTIVE.to_string()),
            remarks: Set(req.remarks),
            is_deleted: Set(false),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("染化料批次创建失败: {}", e)))?;
        Ok(result)
    }

    /// 更新染化料批次
    pub async fn update(
        &self,
        id: i32,
        req: UpdateChemicalLotRequest,
    ) -> Result<LotModel, AppError> {
        let model = self.get_by_id(id).await?;

        // 在 model.into() 之前记录原值，避免 ActiveValue 取值复杂
        let original_quantity_received = model.quantity_received;
        let original_unit_cost = model.unit_cost;

        let mut active: LotActiveModel = model.into();

        let mut need_recompute_cost = false;
        let new_quantity = original_quantity_received;
        let mut new_unit_cost = original_unit_cost;

        if let Some(v) = req.supplier_id {
            active.supplier_id = Set(Some(v));
        }
        if let Some(v) = req.supplier_lot_no {
            active.supplier_lot_no = Set(Some(v));
        }
        if let Some(v) = req.production_date {
            active.production_date = Set(Some(v));
        }
        if let Some(v) = req.expiry_date {
            active.expiry_date = Set(Some(v));
        }
        if let Some(v) = req.received_date {
            active.received_date = Set(Some(v));
        }
        if let Some(v) = req.warehouse_id {
            active.warehouse_id = Set(Some(v));
        }
        if let Some(v) = req.storage_zone {
            active.storage_zone = Set(Some(v));
        }
        if let Some(v) = req.unit_cost {
            if v < Decimal::ZERO {
                return Err(AppError::business("单位成本不能为负"));
            }
            new_unit_cost = v;
            need_recompute_cost = true;
        }
        if let Some(v) = req.inspection_report_url {
            active.inspection_report_url = Set(Some(v));
        }
        if let Some(v) = req.remarks {
            active.remarks = Set(Some(v));
        }

        if need_recompute_cost {
            let total_cost = compute_total_cost(new_quantity, new_unit_cost);
            active.unit_cost = Set(new_unit_cost);
            active.total_cost = Set(total_cost);
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 软删除染化料批次
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        let mut active: LotActiveModel = model.into();
        active.is_deleted = Set(true);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        active.update(&*self.db).await?;
        Ok(())
    }

    /// 来料检验合格（pending → passed）
    pub async fn pass_inspection(
        &self,
        id: i32,
        inspection_report_url: Option<String>,
    ) -> Result<LotModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.inspection_status != chemical_inspection_status::PENDING
            && model.inspection_status != chemical_inspection_status::QUARANTINE
        {
            return Err(AppError::business(format!(
                "仅待检(pending)或隔离(quarantine)状态可标记合格，当前状态: {}",
                model.inspection_status
            )));
        }
        let mut active: LotActiveModel = model.into();
        active.inspection_status = Set(chemical_inspection_status::PASSED.to_string());
        if let Some(url) = inspection_report_url {
            active.inspection_report_url = Set(Some(url));
        }
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 来料检验不合格（pending → failed）
    pub async fn fail_inspection(
        &self,
        id: i32,
        inspection_report_url: Option<String>,
    ) -> Result<LotModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.inspection_status != chemical_inspection_status::PENDING
            && model.inspection_status != chemical_inspection_status::QUARANTINE
        {
            return Err(AppError::business(format!(
                "仅待检(pending)或隔离(quarantine)状态可标记不合格，当前状态: {}",
                model.inspection_status
            )));
        }
        let mut active: LotActiveModel = model.into();
        active.inspection_status = Set(chemical_inspection_status::FAILED.to_string());
        if let Some(url) = inspection_report_url {
            active.inspection_report_url = Set(Some(url));
        }
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 消耗批次（active → consumed，当可用库存归零时）
    pub async fn consume(&self, id: i32) -> Result<LotModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != chemical_lot_status::ACTIVE {
            return Err(AppError::business(format!(
                "仅可用(active)状态可标记耗尽，当前状态: {}",
                model.status
            )));
        }
        let mut active: LotActiveModel = model.into();
        active.quantity_available = Set(Decimal::ZERO);
        active.status = Set(chemical_lot_status::CONSUMED.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 报废批次（active → scrapped）
    pub async fn scrap(&self, id: i32) -> Result<LotModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != chemical_lot_status::ACTIVE {
            return Err(AppError::business(format!(
                "仅可用(active)状态可报废，当前状态: {}",
                model.status
            )));
        }
        let mut active: LotActiveModel = model.into();
        active.status = Set(chemical_lot_status::SCRAPPED.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<LotModel, AppError> {
        LotEntity::find_by_id(id)
            .filter(chemical_lot::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("染化料批次 {} 不存在", id)))
    }

    /// 按批号查询
    pub async fn get_by_no(&self, lot_no: &str) -> Result<LotModel, AppError> {
        LotEntity::find()
            .filter(chemical_lot::Column::LotNo.eq(lot_no))
            .filter(chemical_lot::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("批号 {} 不存在", lot_no)))
    }

    /// 分页查询
    pub async fn list(
        &self,
        query: ChemicalLotQuery,
    ) -> Result<(Vec<LotModel>, u64), AppError> {
        let mut q = LotEntity::find()
            .filter(chemical_lot::Column::IsDeleted.eq(false));
        if let Some(v) = query.chemical_id {
            q = q.filter(chemical_lot::Column::ChemicalId.eq(v));
        }
        if let Some(v) = query.supplier_id {
            q = q.filter(chemical_lot::Column::SupplierId.eq(v));
        }
        if let Some(v) = query.warehouse_id {
            q = q.filter(chemical_lot::Column::WarehouseId.eq(v));
        }
        if let Some(v) = query.inspection_status {
            q = q.filter(chemical_lot::Column::InspectionStatus.eq(v));
        }
        if let Some(v) = query.storage_zone {
            q = q.filter(chemical_lot::Column::StorageZone.eq(v));
        }
        if let Some(v) = query.status {
            q = q.filter(chemical_lot::Column::Status.eq(v));
        }
        if let Some(v) = query.expiry_before {
            q = q.filter(chemical_lot::Column::ExpiryDate.lte(v));
        }

        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

        let total = q.clone().count(&*self.db).await?;
        let items = q
            .order_by_desc(chemical_lot::Column::Id)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;
        Ok((items, total))
    }
}

// ============================================================================
// 染化料领用单 Service
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

/// 染化料领用单 Service
pub struct ChemicalRequisitionService {
    db: Arc<DatabaseConnection>,
}

impl ChemicalRequisitionService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 生成领用单号：CR-YYYYMMDDHHMMSS-NNN
    fn generate_requisition_no() -> String {
        let now = chrono::Utc::now();
        let timestamp = now.format("%Y%m%d%H%M%S");
        let random = crate::utils::random::random_6_digit() % 1000;
        format!("CR-{}-{:03}", timestamp, random)
    }

    /// 创建染化料领用单
    pub async fn create(
        &self,
        req: CreateChemicalRequisitionRequest,
    ) -> Result<RequisitionModel, AppError> {
        validate_requisition_type(&req.requisition_type)?;

        // 校验生产领用必须关联染色缸号
        if req.requisition_type == chemical_requisition_type::PRODUCTION
            && req.dye_batch_id.is_none()
        {
            return Err(AppError::business("生产领用必须关联染色缸号"));
        }

        // 校验染色缸号存在（若提供）
        if let Some(dye_batch_id) = req.dye_batch_id {
            if crate::models::dye_batch::Entity::find_by_id(dye_batch_id)
                .one(&*self.db)
                .await?
                .is_none()
            {
                return Err(AppError::business(format!(
                    "染色缸号 {} 不存在",
                    dye_batch_id
                )));
            }
        }

        // 校验生产订单存在（若提供）
        if let Some(order_id) = req.production_order_id {
            if crate::models::production_order::Entity::find_by_id(order_id)
                .one(&*self.db)
                .await?
                .is_none()
            {
                return Err(AppError::business(format!("生产订单 {} 不存在", order_id)));
            }
        }

        let total_amount = req.total_amount.unwrap_or(Decimal::ZERO);
        if total_amount < Decimal::ZERO {
            return Err(AppError::business("总金额不能为负"));
        }

        let requisition_no = Self::generate_requisition_no();
        let now = crate::utils::date_utils::utc_now_fixed();

        let active = RequisitionActiveModel {
            id: Default::default(),
            requisition_no: Set(requisition_no),
            requisition_type: Set(req.requisition_type),
            department_id: Set(req.department_id),
            requisition_date: Set(req.requisition_date),
            required_date: Set(req.required_date),
            dye_batch_id: Set(req.dye_batch_id),
            production_order_id: Set(req.production_order_id),
            status: Set(chemical_requisition_status::DRAFT.to_string()),
            total_amount: Set(total_amount),
            remarks: Set(req.remarks),
            is_deleted: Set(false),
            created_by: Set(req.created_by),
            approved_by: Set(None),
            issued_by: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("染化料领用单创建失败: {}", e)))?;
        Ok(result)
    }

    /// 更新染化料领用单（仅 draft 状态可更新）
    pub async fn update(
        &self,
        id: i32,
        req: UpdateChemicalRequisitionRequest,
    ) -> Result<RequisitionModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != chemical_requisition_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可更新，当前状态: {}",
                model.status
            )));
        }

        let mut active: RequisitionActiveModel = model.into();

        if let Some(v) = req.department_id {
            active.department_id = Set(Some(v));
        }
        if let Some(v) = req.required_date {
            active.required_date = Set(Some(v));
        }
        if let Some(v) = req.dye_batch_id {
            active.dye_batch_id = Set(Some(v));
        }
        if let Some(v) = req.production_order_id {
            active.production_order_id = Set(Some(v));
        }
        if let Some(v) = req.total_amount {
            if v < Decimal::ZERO {
                return Err(AppError::business("总金额不能为负"));
            }
            active.total_amount = Set(v);
        }
        if let Some(v) = req.remarks {
            active.remarks = Set(Some(v));
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 软删除染化料领用单（仅 draft 状态可删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != chemical_requisition_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可删除，当前状态: {}",
                model.status
            )));
        }
        let mut active: RequisitionActiveModel = model.into();
        active.is_deleted = Set(true);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        active.update(&*self.db).await?;
        Ok(())
    }

    /// 审批领用单（draft → approved）
    pub async fn approve(
        &self,
        id: i32,
        approved_by: Option<i32>,
    ) -> Result<RequisitionModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != chemical_requisition_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可审批，当前状态: {}",
                model.status
            )));
        }
        let mut active: RequisitionActiveModel = model.into();
        active.status = Set(chemical_requisition_status::APPROVED.to_string());
        active.approved_by = Set(approved_by);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 发料（approved → issued）
    pub async fn issue(
        &self,
        id: i32,
        issued_by: Option<i32>,
    ) -> Result<RequisitionModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != chemical_requisition_status::APPROVED {
            return Err(AppError::business(format!(
                "仅已审批(approved)状态可发料，当前状态: {}",
                model.status
            )));
        }
        let mut active: RequisitionActiveModel = model.into();
        active.status = Set(chemical_requisition_status::ISSUED.to_string());
        active.issued_by = Set(issued_by);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 关闭领用单（issued/partial_returned → closed）
    pub async fn close(&self, id: i32) -> Result<RequisitionModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != chemical_requisition_status::ISSUED
            && model.status != chemical_requisition_status::PARTIAL_RETURNED
        {
            return Err(AppError::business(format!(
                "仅已发料(issued)或部分退回(partial_returned)状态可关闭，当前状态: {}",
                model.status
            )));
        }
        let mut active: RequisitionActiveModel = model.into();
        active.status = Set(chemical_requisition_status::CLOSED.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 取消领用单（任意非 closed/cancelled 状态 → cancelled）
    pub async fn cancel(&self, id: i32) -> Result<RequisitionModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status == chemical_requisition_status::CLOSED {
            return Err(AppError::business("已关闭状态不可取消"));
        }
        if model.status == chemical_requisition_status::CANCELLED {
            return Err(AppError::business("已取消状态不可重复取消"));
        }
        let mut active: RequisitionActiveModel = model.into();
        active.status = Set(chemical_requisition_status::CANCELLED.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<RequisitionModel, AppError> {
        RequisitionEntity::find_by_id(id)
            .filter(chemical_requisition::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("染化料领用单 {} 不存在", id)))
    }

    /// 按单号查询
    pub async fn get_by_no(&self, requisition_no: &str) -> Result<RequisitionModel, AppError> {
        RequisitionEntity::find()
            .filter(chemical_requisition::Column::RequisitionNo.eq(requisition_no))
            .filter(chemical_requisition::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!("领用单号 {} 不存在", requisition_no))
            })
    }

    /// 分页查询
    pub async fn list(
        &self,
        query: ChemicalRequisitionQuery,
    ) -> Result<(Vec<RequisitionModel>, u64), AppError> {
        let mut q = RequisitionEntity::find()
            .filter(chemical_requisition::Column::IsDeleted.eq(false));
        if let Some(v) = query.requisition_type {
            q = q.filter(chemical_requisition::Column::RequisitionType.eq(v));
        }
        if let Some(v) = query.department_id {
            q = q.filter(chemical_requisition::Column::DepartmentId.eq(v));
        }
        if let Some(v) = query.dye_batch_id {
            q = q.filter(chemical_requisition::Column::DyeBatchId.eq(v));
        }
        if let Some(v) = query.production_order_id {
            q = q.filter(chemical_requisition::Column::ProductionOrderId.eq(v));
        }
        if let Some(v) = query.status {
            q = q.filter(chemical_requisition::Column::Status.eq(v));
        }
        if let Some(v) = query.requisition_date_start {
            q = q.filter(chemical_requisition::Column::RequisitionDate.gte(v));
        }
        if let Some(v) = query.requisition_date_end {
            q = q.filter(chemical_requisition::Column::RequisitionDate.lte(v));
        }

        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

        let total = q.clone().count(&*self.db).await?;
        let items = q
            .order_by_desc(chemical_requisition::Column::Id)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;
        Ok((items, total))
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
