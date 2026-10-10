//! 委外加工发料明细 Service impl 子模块（outsourcing_ops/order_item）
//!
//! 批次 489 D10-2b 拆分：从原 `outsourcing_service.rs` L1002-1158 迁移。
//! 包含 OutsourcingOrderItemService 的 5 个方法：
//! - create / update / delete（CRUD）
//! - get_by_id / list_by_order（查询）
//!
//! 业务规则：
//! - 发料明细无软删除字段，delete 为物理删除
//! - 创建/更新时校验数量与单位成本非负
//! - 创建时校验委外订单存在 + 物料存在

use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};

use crate::models::outsourcing_order::{self, Entity as OrderEntity};
use crate::models::outsourcing_order_item::{
    self, ActiveModel as ItemActiveModel, Entity as ItemEntity, Model as ItemModel,
};
use crate::utils::error::AppError;

use crate::services::outsourcing_ops::types::{
    CreateOutsourcingOrderItemRequest, UpdateOutsourcingOrderItemRequest,
};
use crate::services::outsourcing_service::OutsourcingOrderItemService;

impl OutsourcingOrderItemService {
    /// 创建委外发料明细
    pub async fn create(
        &self,
        req: CreateOutsourcingOrderItemRequest,
    ) -> Result<ItemModel, AppError> {
        // 校验数量非负
        if req.quantity < Decimal::ZERO {
            return Err(AppError::business("发出数量不能为负"));
        }
        if req.unit_cost < Decimal::ZERO {
            return Err(AppError::business("单位成本不能为负"));
        }

        // 校验委外订单存在
        if OrderEntity::find_by_id(req.outsourcing_order_id)
            .filter(outsourcing_order::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .is_none()
        {
            return Err(AppError::business(format!(
                "委外订单 {} 不存在",
                req.outsourcing_order_id
            )));
        }

        // 校验物料存在
        if crate::models::product::Entity::find_by_id(req.product_id)
            .one(&*self.db)
            .await?
            .is_none()
        {
            return Err(AppError::business(format!(
                "物料 {} 不存在",
                req.product_id
            )));
        }

        let total_cost = req.quantity * req.unit_cost;
        let now = crate::utils::date_utils::utc_now_fixed();
        let unit = req.unit.unwrap_or_else(|| "kg".to_string());
        let processing_fee = req.processing_fee.unwrap_or(Decimal::ZERO);
        let freight_fee = req.freight_fee.unwrap_or(Decimal::ZERO);

        let active = ItemActiveModel {
            id: Default::default(),
            outsourcing_order_id: Set(req.outsourcing_order_id),
            product_id: Set(req.product_id),
            color_no: Set(req.color_no),
            dye_lot_no: Set(req.dye_lot_no),
            piece_no: Set(req.piece_no),
            batch_no: Set(req.batch_no),
            warehouse_id: Set(req.warehouse_id),
            quantity: Set(req.quantity),
            unit: Set(unit),
            unit_cost: Set(req.unit_cost),
            total_cost: Set(total_cost),
            processing_fee: Set(processing_fee),
            freight_fee: Set(freight_fee),
            inventory_transaction_id: Set(None),
            greige_fabric_id: Set(req.greige_fabric_id),
            remarks: Set(req.remarks),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("委外发料明细创建失败: {}", e)))?;
        Ok(result)
    }

    /// 更新委外发料明细
    ///
    /// 三态写入（RFC 7386，对齐 department_service::update）：
    /// None=不 Set、Some(None)=Set(None) 置 NULL（仅 DB 可空列）、Some(Some(v))=Set(v) 覆盖；
    /// NOT NULL 列（quantity/unit/unit_cost 建表即 NOT NULL，v15:679-681；
    /// processing_fee/freight_fee ALTER NOT NULL DEFAULT 0，v15:2195-2196）
    /// 的显式 null 在任何 DB 访问前拒绝（外显不脱敏）。
    pub async fn update(
        &self,
        id: i32,
        req: UpdateOutsourcingOrderItemRequest,
    ) -> Result<ItemModel, AppError> {
        if matches!(req.quantity, Some(None)) {
            return Err(AppError::business_displayable(
                "发出数量不能清空：该字段为必填项",
            ));
        }
        if matches!(req.unit, Some(None)) {
            return Err(AppError::business_displayable(
                "单位不能清空：该字段为必填项",
            ));
        }
        if matches!(req.unit_cost, Some(None)) {
            return Err(AppError::business_displayable(
                "单位成本不能清空：该字段为必填项",
            ));
        }
        if matches!(req.processing_fee, Some(None)) {
            return Err(AppError::business_displayable(
                "加工费不能清空：该字段为必填项",
            ));
        }
        if matches!(req.freight_fee, Some(None)) {
            return Err(AppError::business_displayable(
                "运费不能清空：该字段为必填项",
            ));
        }

        let model = self.get_by_id(id).await?;

        // 在 model.into() 之前记录原值，避免 ActiveValue 取值复杂
        let original_quantity = model.quantity;
        let original_unit_cost = model.unit_cost;
        let mut new_quantity = original_quantity;
        let mut new_unit_cost = original_unit_cost;
        let mut need_recompute_cost = false;

        let mut active: ItemActiveModel = model.into();

        // DB 可空列：Some(None)=Set(None) 清空、Some(Some(v))=Set(Some(v)) 覆盖
        if let Some(v) = req.color_no {
            active.color_no = Set(v);
        }
        if let Some(v) = req.dye_lot_no {
            active.dye_lot_no = Set(v);
        }
        if let Some(v) = req.batch_no {
            active.batch_no = Set(v);
        }
        if let Some(v) = req.warehouse_id {
            active.warehouse_id = Set(v);
        }
        if let Some(v) = req.remarks {
            active.remarks = Set(v);
        }
        // NOT NULL 列（Some(None) 已在入口拒绝）：仅覆盖/保持
        if let Some(v) = req.quantity.flatten() {
            if v < Decimal::ZERO {
                return Err(AppError::business("发出数量不能为负"));
            }
            new_quantity = v;
            need_recompute_cost = true;
        }
        if let Some(v) = req.unit.flatten() {
            active.unit = Set(v);
        }
        if let Some(v) = req.unit_cost.flatten() {
            if v < Decimal::ZERO {
                return Err(AppError::business("单位成本不能为负"));
            }
            new_unit_cost = v;
            need_recompute_cost = true;
        }
        if let Some(v) = req.processing_fee.flatten() {
            active.processing_fee = Set(v);
        }
        if let Some(v) = req.freight_fee.flatten() {
            active.freight_fee = Set(v);
        }

        if need_recompute_cost {
            active.total_cost = Set(new_quantity * new_unit_cost);
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 删除委外发料明细（物理删除，明细无软删除字段）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        let _ = model;
        ItemEntity::delete_by_id(id)
            .exec(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("委外发料明细删除失败: {}", e)))?;
        Ok(())
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<ItemModel, AppError> {
        ItemEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("委外发料明细 {} 不存在", id)))
    }

    /// 按委外订单查询明细列表
    pub async fn list_by_order(&self, order_id: i32) -> Result<Vec<ItemModel>, AppError> {
        let items = ItemEntity::find()
            .filter(outsourcing_order_item::Column::OutsourcingOrderId.eq(order_id))
            .order_by_desc(outsourcing_order_item::Column::Id)
            .all(&*self.db)
            .await?;
        Ok(items)
    }
}
