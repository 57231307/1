//! 流转卡 CRUD 与查询 impl 子模块（flow_card_ops/card_crud）
//!
//! D10 第 5 批拆分：从原 flow_card_service.rs 迁移 FlowCardService 的 7 个 CRUD/查询方法
//!（create / update / delete / get_by_id / get_by_barcode / get_by_dye_lot / list）。
//! 单号生成与状态校验纯函数保留在 facade，本模块通过 Self:: 调用。

use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};

use crate::models::production_flow_card::{
    self, ActiveModel as CardActiveModel, Entity as CardEntity, Model as CardModel,
};
use crate::models::status::flow_card as card_status;
use crate::services::flow_card_service::{
    CreateFlowCardRequest, FlowCardQuery, FlowCardService, UpdateFlowCardRequest,
};
use crate::utils::error::AppError;

impl FlowCardService {
    /// 创建流转卡
    pub async fn create(&self, req: CreateFlowCardRequest) -> Result<CardModel, AppError> {
        // 业务校验：计划配布数量必须为正
        if let Some(weight) = req.planned_fabric_weight {
            if weight <= Decimal::ZERO {
                return Err(AppError::business("计划配布数量必须 > 0"));
            }
        }

        // 业务校验：优先级范围
        if let Some(p) = req.priority {
            if !(-100..=100).contains(&p) {
                return Err(AppError::business("优先级范围 -100 到 100"));
            }
        }

        let card_no = Self::generate_card_no();
        let barcode = Self::generate_barcode();
        let now = crate::utils::date_utils::utc_now_fixed();

        let active = CardActiveModel {
            id: Default::default(),
            card_no: Set(card_no),
            barcode: Set(barcode),
            production_order_id: Set(req.production_order_id),
            dye_batch_id: Set(req.dye_batch_id),
            dye_lot_no: Set(req.dye_lot_no),
            process_route_id: Set(req.process_route_id),
            customer_id: Set(req.customer_id),
            customer_name: Set(req.customer_name),
            order_no: Set(req.order_no),
            product_id: Set(req.product_id),
            product_name: Set(req.product_name),
            color_no: Set(req.color_no),
            dyeing_requirements: Set(req.dyeing_requirements),
            planned_fabric_weight: Set(req.planned_fabric_weight),
            actual_fabric_weight: Set(None),
            current_step_seq: Set(1),
            status: Set(card_status::PENDING.to_string()),
            scheduled_at: Set(None),
            prepared_at: Set(None),
            dye_start_at: Set(None),
            dye_end_at: Set(None),
            inspected_at: Set(None),
            completed_at: Set(None),
            shipped_at: Set(None),
            priority: Set(req.priority.unwrap_or(0)),
            remarks: Set(req.remarks),
            is_deleted: Set(false),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("流转卡创建失败: {}", e)))?;
        Ok(result)
    }

    /// 更新流转卡（仅 pending 状态可更新）
    pub async fn update(
        &self,
        id: i32,
        req: UpdateFlowCardRequest,
    ) -> Result<CardModel, AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_can_update(&model.status)?;

        let mut active: CardActiveModel = model.into();

        if let Some(v) = req.dye_batch_id {
            active.dye_batch_id = Set(Some(v));
        }
        if let Some(v) = req.dye_lot_no {
            active.dye_lot_no = Set(Some(v));
        }
        if let Some(v) = req.process_route_id {
            active.process_route_id = Set(Some(v));
        }
        if let Some(v) = req.customer_id {
            active.customer_id = Set(Some(v));
        }
        if let Some(v) = req.customer_name {
            active.customer_name = Set(Some(v));
        }
        if let Some(v) = req.order_no {
            active.order_no = Set(Some(v));
        }
        if let Some(v) = req.product_id {
            active.product_id = Set(Some(v));
        }
        if let Some(v) = req.product_name {
            active.product_name = Set(Some(v));
        }
        if let Some(v) = req.color_no {
            active.color_no = Set(Some(v));
        }
        if let Some(v) = req.dyeing_requirements {
            active.dyeing_requirements = Set(Some(v));
        }
        if let Some(v) = req.planned_fabric_weight {
            if v <= Decimal::ZERO {
                return Err(AppError::business("计划配布数量必须 > 0"));
            }
            active.planned_fabric_weight = Set(Some(v));
        }
        if let Some(v) = req.priority {
            if !(-100..=100).contains(&v) {
                return Err(AppError::business("优先级范围 -100 到 100"));
            }
            active.priority = Set(v);
        }
        if let Some(v) = req.remarks {
            active.remarks = Set(Some(v));
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 软删除流转卡（仅 pending/terminated 状态可删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != card_status::PENDING && model.status != card_status::TERMINATED {
            return Err(AppError::business(format!(
                "流转卡状态为 {}，仅 pending/terminated 状态可删除",
                model.status
            )));
        }

        let mut active: CardActiveModel = model.into();
        active.is_deleted = Set(true);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        active.update(&*self.db).await?;
        Ok(())
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<CardModel, AppError> {
        let model = CardEntity::find_by_id(id)
            .filter(production_flow_card::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("流转卡 {} 不存在", id)))?;
        Ok(model)
    }

    /// 按条码查询（扫码场景）
    pub async fn get_by_barcode(&self, barcode: &str) -> Result<CardModel, AppError> {
        let model = CardEntity::find()
            .filter(production_flow_card::Column::Barcode.eq(barcode))
            .filter(production_flow_card::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("条码 {} 对应的流转卡不存在", barcode)))?;
        Ok(model)
    }

    /// 按缸号查询
    pub async fn get_by_dye_lot(&self, dye_lot_no: &str) -> Result<CardModel, AppError> {
        let model = CardEntity::find()
            .filter(production_flow_card::Column::DyeLotNo.eq(dye_lot_no))
            .filter(production_flow_card::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("缸号 {} 对应的流转卡不存在", dye_lot_no)))?;
        Ok(model)
    }

    /// 分页查询
    pub async fn list(&self, query: FlowCardQuery) -> Result<(Vec<CardModel>, u64), AppError> {
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

        let mut q = CardEntity::find().filter(production_flow_card::Column::IsDeleted.eq(false));

        if let Some(no) = &query.card_no {
            q = q.filter(production_flow_card::Column::CardNo.contains(no));
        }
        if let Some(bc) = &query.barcode {
            q = q.filter(production_flow_card::Column::Barcode.eq(bc));
        }
        if let Some(dl) = &query.dye_lot_no {
            q = q.filter(production_flow_card::Column::DyeLotNo.eq(dl));
        }
        if let Some(oid) = query.production_order_id {
            q = q.filter(production_flow_card::Column::ProductionOrderId.eq(oid));
        }
        if let Some(s) = &query.status {
            q = q.filter(production_flow_card::Column::Status.eq(s));
        }
        if let Some(cid) = query.customer_id {
            q = q.filter(production_flow_card::Column::CustomerId.eq(cid));
        }

        q = q.order_by_desc(production_flow_card::Column::CreatedAt);

        let paginator = q.paginate(&*self.db, page_size);
        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page.saturating_sub(1)).await?;
        Ok((items, total))
    }
}
