//! 染化料批次 Service impl 子模块（chemical_ops/lot）
//!
//! 批次 490 D10-3a 拆分：从原 `chemical_service.rs` L968-1262 迁移。
//! 包含 ChemicalLotService 的 10 个方法：
//! - create / update / delete（CRUD）
//! - pass_inspection / fail_inspection（来料检验状态流转：pending → passed/failed）
//! - consume / scrap（批次状态流转：active → consumed/scrapped）
//! - get_by_id / get_by_no / list（查询）
//!
//! 业务规则：
//! - 创建时校验染化料主数据存在、批号唯一、数量非负
//! - 创建时自动计算总成本（compute_total_cost），初始检验状态 pending，初始批次状态 active
//! - 更新时若单位成本变化则重算总成本
//! - 检验流转：仅 pending/quarantine 状态可标记合格/不合格
//! - 消耗流转：仅 active 状态可标记耗尽（quantity_available 归零）
//! - 报废流转：仅 active 状态可报废
//! - 软删除（is_deleted = true）

use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};

use crate::models::chemical_lot::{
    self, ActiveModel as LotActiveModel, Entity as LotEntity, Model as LotModel,
};
use crate::models::chemical_master::{self, Entity as MasterEntity};
use crate::models::status::chemical_inspection_status;
use crate::models::status::chemical_lot_status;
use crate::utils::error::AppError;

use crate::services::chemical_ops::types::{
    ChemicalLotQuery, CreateChemicalLotRequest, UpdateChemicalLotRequest,
};
use crate::services::chemical_service::{compute_total_cost, ChemicalLotService};

impl ChemicalLotService {
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
