//! 染化料主数据 Service impl 子模块（chemical_ops/master）
//!
//! 批次 490 D10-3a 拆分：从原 `chemical_service.rs` L286-694 迁移。
//! 包含 ChemicalMasterService 的 6 个公开方法 + 10 个私有 helper：
//! - create / update / delete（CRUD）
//! - get_by_id / get_by_code / list（查询）
//! - apply_basic_info / apply_chemical_properties / apply_pricing / apply_ghs_msds
//!   / apply_storage_params / apply_inventory_params / apply_packaging
//!   / apply_supplier_info / apply_dye_fastness / apply_status_and_remarks（私有 helper）
//!
//! 业务规则：
//! - 创建时校验类型合法、染料必填 dye_category、助剂必填 auxiliary_category
//! - 价格/库存字段非负校验
//! - 状态流转：active → inactive / discontinued（更新时校验）
//! - 软删除（is_deleted = true）

use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    Set,
};

use crate::models::chemical_master::{
    self, ActiveModel as MasterActiveModel, Entity as MasterEntity, Model as MasterModel,
};
use crate::models::status::chemical_status;
use crate::models::status::chemical_type;
use crate::utils::error::AppError;

use crate::services::chemical_ops::types::{
    ChemicalMasterQuery, CreateChemicalMasterRequest, UpdateChemicalMasterRequest,
};
use crate::services::chemical_service::{validate_chemical_type, ChemicalMasterService};

/// create 校验通过后的非负字段（价格 + 安全库存）
struct CreateValidatedFields {
    standard_price: Decimal,
    cost_price: Decimal,
    safety_stock: Decimal,
    reorder_point: Decimal,
    reorder_quantity: Decimal,
}

impl ChemicalMasterService {
    /// 创建染化料主数据
    pub async fn create(&self, req: CreateChemicalMasterRequest) -> Result<MasterModel, AppError> {
        Self::validate_type_and_categories(&req)?;
        let fields = Self::validate_non_negative_fields(&req)?;
        self.check_chemical_code_unique(&req.chemical_code).await?;

        let now = crate::utils::date_utils::utc_now_fixed();
        let mut active = MasterActiveModel {
            id: Default::default(),
            ..Default::default()
        };
        Self::apply_create_main_fields(&mut active, &req, &fields, now);
        Self::apply_create_extra_fields(&mut active, &req, now);
        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("染化料主数据创建失败: {}", e)))?;
        Ok(result)
    }

    /// 校验类型合法 + 染料必填 dye_category / 助剂必填 auxiliary_category
    fn validate_type_and_categories(req: &CreateChemicalMasterRequest) -> Result<(), AppError> {
        validate_chemical_type(&req.chemical_type)?;
        if req.chemical_type == chemical_type::DYE && req.dye_category.is_none() {
            return Err(AppError::business("染料类型必须提供 dye_category"));
        }
        if req.chemical_type == chemical_type::AUXILIARY && req.auxiliary_category.is_none() {
            return Err(AppError::business("助剂类型必须提供 auxiliary_category"));
        }
        Ok(())
    }

    /// 校验标准价/成本价/安全库存相关字段非负并返回规整后的字段
    fn validate_non_negative_fields(
        req: &CreateChemicalMasterRequest,
    ) -> Result<CreateValidatedFields, AppError> {
        let standard_price = req.standard_price.unwrap_or(Decimal::ZERO);
        if standard_price < Decimal::ZERO {
            return Err(AppError::business("标准价不能为负"));
        }
        let cost_price = req.cost_price.unwrap_or(Decimal::ZERO);
        if cost_price < Decimal::ZERO {
            return Err(AppError::business("成本价不能为负"));
        }
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
        Ok(CreateValidatedFields {
            standard_price,
            cost_price,
            safety_stock,
            reorder_point,
            reorder_quantity,
        })
    }

    /// 校验染化料编码唯一性（未软删除范围内）
    async fn check_chemical_code_unique(&self, chemical_code: &str) -> Result<(), AppError> {
        if let Some(_existing) = MasterEntity::find()
            .filter(chemical_master::Column::ChemicalCode.eq(chemical_code))
            .filter(chemical_master::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
        {
            return Err(AppError::business(format!(
                "染化料编码 {} 已存在",
                chemical_code
            )));
        }
        Ok(())
    }

    /// 写入主信息 + 价格 + 库存字段（含 unit / msds_updated_at 派生）
    fn apply_create_main_fields(
        active: &mut MasterActiveModel,
        req: &CreateChemicalMasterRequest,
        fields: &CreateValidatedFields,
        now: chrono::DateTime<chrono::FixedOffset>,
    ) {
        let unit = req.unit.clone().unwrap_or_else(|| "kg".to_string());
        // MSDS 更新时间：仅在提供 msds_url 时记录
        let msds_updated_at = if req.msds_url.is_some() {
            Some(now)
        } else {
            None
        };
        active.chemical_code = Set(req.chemical_code.clone());
        active.chemical_name = Set(req.chemical_name.clone());
        active.chemical_name_en = Set(req.chemical_name_en.clone());
        active.chemical_type = Set(req.chemical_type.clone());
        active.category_id = Set(req.category_id);
        active.dye_category = Set(req.dye_category.clone());
        active.color_index = Set(req.color_index.clone());
        active.auxiliary_category = Set(req.auxiliary_category.clone());
        active.cas_number = Set(req.cas_number.clone());
        active.molecular_formula = Set(req.molecular_formula.clone());
        active.molecular_weight = Set(req.molecular_weight);
        active.specification = Set(req.specification.clone());
        active.unit = Set(unit);
        active.standard_price = Set(fields.standard_price);
        active.cost_price = Set(fields.cost_price);
        active.ghs_classification = Set(req.ghs_classification.clone());
        active.un_number = Set(req.un_number.clone());
        active.hazard_class = Set(req.hazard_class.clone());
        active.hazard_pictogram = Set(req.hazard_pictogram.clone());
        active.signal_word = Set(req.signal_word.clone());
        active.msds_url = Set(req.msds_url.clone());
        active.msds_version = Set(req.msds_version.clone());
        active.msds_updated_at = Set(msds_updated_at);
        active.safety_stock = Set(fields.safety_stock);
        active.reorder_point = Set(fields.reorder_point);
        active.reorder_quantity = Set(fields.reorder_quantity);
    }

    /// 写入存储 / 包装 / 供应商 / 染料牢度 / 状态 / 时间戳字段
    fn apply_create_extra_fields(
        active: &mut MasterActiveModel,
        req: &CreateChemicalMasterRequest,
        now: chrono::DateTime<chrono::FixedOffset>,
    ) {
        active.shelf_life_days = Set(req.shelf_life_days);
        active.storage_condition = Set(req.storage_condition.clone());
        active.storage_temperature = Set(req.storage_temperature.clone());
        active.package_unit = Set(req.package_unit.clone());
        active.package_capacity = Set(req.package_capacity);
        active.packages_per_pallet = Set(req.packages_per_pallet);
        active.supplier_id = Set(req.supplier_id);
        active.supplier_product_code = Set(req.supplier_product_code.clone());
        active.fastness_light = Set(req.fastness_light.clone());
        active.fastness_washing = Set(req.fastness_washing.clone());
        active.active_ingredient = Set(req.active_ingredient.clone());
        active.concentration = Set(req.concentration);
        active.status = Set(chemical_status::ACTIVE.to_string());
        active.remarks = Set(req.remarks.clone());
        active.is_deleted = Set(false);
        active.created_by = Set(req.created_by);
        active.created_at = Set(now);
        active.updated_at = Set(now);
    }

    /// 更新染化料主数据
    pub async fn update(
        &self,
        id: i32,
        req: UpdateChemicalMasterRequest,
    ) -> Result<MasterModel, AppError> {
        let model = self.get_by_id(id).await?;
        let mut active: MasterActiveModel = model.into();

        Self::apply_basic_info(&mut active, &req);
        Self::apply_chemical_properties(&mut active, &req);
        Self::apply_pricing(&mut active, &req)?;
        Self::apply_ghs_msds(&mut active, &req);
        Self::apply_storage_params(&mut active, &req);
        Self::apply_inventory_params(&mut active, &req)?;
        Self::apply_packaging(&mut active, &req);
        Self::apply_supplier_info(&mut active, &req);
        Self::apply_dye_fastness(&mut active, &req);
        Self::apply_status_and_remarks(&mut active, &req)?;

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    fn apply_basic_info(active: &mut MasterActiveModel, req: &UpdateChemicalMasterRequest) {
        if let Some(v) = &req.chemical_name {
            active.chemical_name = Set(v.clone());
        }
        if let Some(v) = &req.chemical_name_en {
            active.chemical_name_en = Set(Some(v.clone()));
        }
        if let Some(v) = req.category_id {
            active.category_id = Set(Some(v));
        }
        if let Some(v) = &req.dye_category {
            active.dye_category = Set(Some(v.clone()));
        }
        if let Some(v) = &req.color_index {
            active.color_index = Set(Some(v.clone()));
        }
        if let Some(v) = &req.auxiliary_category {
            active.auxiliary_category = Set(Some(v.clone()));
        }
    }

    fn apply_chemical_properties(
        active: &mut MasterActiveModel,
        req: &UpdateChemicalMasterRequest,
    ) {
        if let Some(v) = &req.cas_number {
            active.cas_number = Set(Some(v.clone()));
        }
        if let Some(v) = &req.molecular_formula {
            active.molecular_formula = Set(Some(v.clone()));
        }
        if let Some(v) = req.molecular_weight {
            active.molecular_weight = Set(Some(v));
        }
        if let Some(v) = &req.specification {
            active.specification = Set(Some(v.clone()));
        }
        if let Some(v) = &req.unit {
            active.unit = Set(v.clone());
        }
    }

    fn apply_pricing(
        active: &mut MasterActiveModel,
        req: &UpdateChemicalMasterRequest,
    ) -> Result<(), AppError> {
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
        Ok(())
    }

    fn apply_ghs_msds(active: &mut MasterActiveModel, req: &UpdateChemicalMasterRequest) {
        if let Some(v) = &req.ghs_classification {
            active.ghs_classification = Set(Some(v.clone()));
        }
        if let Some(v) = &req.un_number {
            active.un_number = Set(Some(v.clone()));
        }
        if let Some(v) = &req.hazard_class {
            active.hazard_class = Set(Some(v.clone()));
        }
        if let Some(v) = &req.hazard_pictogram {
            active.hazard_pictogram = Set(Some(v.clone()));
        }
        if let Some(v) = &req.signal_word {
            active.signal_word = Set(Some(v.clone()));
        }
        if let Some(v) = &req.msds_url {
            active.msds_url = Set(Some(v.clone()));
            active.msds_updated_at = Set(Some(crate::utils::date_utils::utc_now_fixed()));
        }
        if let Some(v) = &req.msds_version {
            active.msds_version = Set(Some(v.clone()));
        }
    }

    fn apply_storage_params(active: &mut MasterActiveModel, req: &UpdateChemicalMasterRequest) {
        if let Some(v) = req.shelf_life_days {
            active.shelf_life_days = Set(Some(v));
        }
        if let Some(v) = &req.storage_condition {
            active.storage_condition = Set(Some(v.clone()));
        }
        if let Some(v) = &req.storage_temperature {
            active.storage_temperature = Set(Some(v.clone()));
        }
    }

    fn apply_inventory_params(
        active: &mut MasterActiveModel,
        req: &UpdateChemicalMasterRequest,
    ) -> Result<(), AppError> {
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
        Ok(())
    }

    fn apply_packaging(active: &mut MasterActiveModel, req: &UpdateChemicalMasterRequest) {
        if let Some(v) = &req.package_unit {
            active.package_unit = Set(Some(v.clone()));
        }
        if let Some(v) = req.package_capacity {
            active.package_capacity = Set(Some(v));
        }
        if let Some(v) = req.packages_per_pallet {
            active.packages_per_pallet = Set(Some(v));
        }
    }

    fn apply_supplier_info(active: &mut MasterActiveModel, req: &UpdateChemicalMasterRequest) {
        if let Some(v) = req.supplier_id {
            active.supplier_id = Set(Some(v));
        }
        if let Some(v) = &req.supplier_product_code {
            active.supplier_product_code = Set(Some(v.clone()));
        }
    }

    fn apply_dye_fastness(active: &mut MasterActiveModel, req: &UpdateChemicalMasterRequest) {
        if let Some(v) = &req.fastness_light {
            active.fastness_light = Set(Some(v.clone()));
        }
        if let Some(v) = &req.fastness_washing {
            active.fastness_washing = Set(Some(v.clone()));
        }
        if let Some(v) = &req.active_ingredient {
            active.active_ingredient = Set(Some(v.clone()));
        }
        if let Some(v) = req.concentration {
            active.concentration = Set(Some(v));
        }
    }

    fn apply_status_and_remarks(
        active: &mut MasterActiveModel,
        req: &UpdateChemicalMasterRequest,
    ) -> Result<(), AppError> {
        if let Some(v) = &req.status {
            if v != chemical_status::ACTIVE
                && v != chemical_status::INACTIVE
                && v != chemical_status::DISCONTINUED
            {
                return Err(AppError::business(format!(
                    "染化料状态必须是 active / inactive / discontinued，当前: {}",
                    v
                )));
            }
            active.status = Set(v.clone());
        }
        if let Some(v) = &req.remarks {
            active.remarks = Set(Some(v.clone()));
        }
        Ok(())
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
