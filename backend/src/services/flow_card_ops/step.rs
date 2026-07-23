//! 工序流转记录 Service impl 子模块（flow_card_ops/step）
//!
//! D10 第 5 批拆分：从原 flow_card_service.rs 迁移 StepRecordService 的业务方法
//!（start_step / complete_step / get_by_id / list_by_flow_card / create_rework +
//! 3 个私有 helper：validate_card_for_step_start / fetch_route_or_default / build_step_active_model）。
//! new 构造函数保留在 facade。

use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};
use sea_orm::DatabaseConnection;

use crate::models::process_route::{self, Entity as RouteEntity};
use crate::models::process_step_record::{
    self, ActiveModel as StepActiveModel, Entity as StepEntity, Model as StepModel,
};
use crate::models::production_flow_card::{
    self, ActiveModel as CardActiveModel, Entity as CardEntity, Model as CardModel,
};
use crate::models::status::flow_card as card_status;
use crate::models::status::step_record as step_status;
use crate::services::flow_card_service::{CompleteStepRequest, StartStepRequest, StepRecordService};
use crate::utils::error::AppError;

impl StepRecordService {
    /// 扫码开始工序（自动创建 pending 记录并切换到 in_progress）
    pub async fn start_step(&self, req: StartStepRequest) -> Result<StepModel, AppError> {
        let card = CardEntity::find_by_id(req.flow_card_id)
            .filter(production_flow_card::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("流转卡 {} 不存在", req.flow_card_id)))?;
        Self::validate_card_for_step_start(&card)?;
        let (route_code, route_name, process_type, step_seq) =
            Self::fetch_route_or_default(self.db.as_ref(), req.process_route_id, &card).await?;
        let now = crate::utils::date_utils::utc_now_fixed();
        let active = Self::build_step_active_model(
            &req, &card, route_code, route_name, process_type, step_seq, now,
        );
        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("工序记录创建失败: {}", e)))?;
        let mut card_active: CardActiveModel = card.into();
        card_active.current_step_seq = Set(step_seq);
        card_active.updated_at = Set(now);
        card_active.update(&*self.db).await?;
        Ok(result)
    }

    /// 校验流转卡状态可开始工序
    fn validate_card_for_step_start(card: &CardModel) -> Result<(), AppError> {
        let valid_statuses = [
            card_status::SCHEDULED,
            card_status::PREPARING,
            card_status::DYEING,
            card_status::DYED,
            card_status::INSPECTING,
        ];
        if !valid_statuses.contains(&card.status.as_str()) {
            return Err(AppError::business(format!(
                "流转卡状态为 {}，仅在 {:?} 状态可开始工序",
                card.status, valid_statuses
            )));
        }
        Ok(())
    }

    /// 获取工序路线信息（无路线时使用占位默认值）
    async fn fetch_route_or_default(
        db: &DatabaseConnection,
        route_id: Option<i32>,
        card: &CardModel,
    ) -> Result<(String, String, String, i32), AppError> {
        if let Some(rid) = route_id {
            let route = RouteEntity::find_by_id(rid)
                .filter(process_route::Column::IsDeleted.eq(false))
                .one(db)
                .await?
                .ok_or_else(|| AppError::not_found(format!("工序路线 {} 不存在", rid)))?;
            Ok((route.route_code, route.route_name, route.process_type, route.seq))
        } else {
            Ok((
                "CUSTOM".to_string(),
                "自定义工序".to_string(),
                "other".to_string(),
                card.current_step_seq,
            ))
        }
    }

    /// 构建 StepActiveModel（含所有字段填充）
    fn build_step_active_model(
        req: &StartStepRequest,
        card: &CardModel,
        route_code: String,
        route_name: String,
        process_type: String,
        step_seq: i32,
        now: chrono::DateTime<chrono::FixedOffset>,
    ) -> StepActiveModel {
        StepActiveModel {
            id: Default::default(),
            flow_card_id: Set(req.flow_card_id),
            process_route_id: Set(req.process_route_id),
            step_seq: Set(step_seq),
            route_code: Set(route_code),
            route_name: Set(route_name),
            process_type: Set(process_type),
            worker_ids: Set(req.worker_ids.clone()),
            worker_names: Set(req.worker_names.clone()),
            equipment_id: Set(req.equipment_id),
            equipment_name: Set(req.equipment_name.clone()),
            start_at: Set(now),
            end_at: Set(None),
            duration_minutes: Set(None),
            planned_quantity: Set(card.planned_fabric_weight),
            actual_quantity: Set(None),
            qualified_quantity: Set(None),
            status: Set(step_status::IN_PROGRESS.to_string()),
            abnormal_description: Set(None),
            handling_opinion: Set(None),
            rework_source_id: Set(None),
            remarks: Set(None),
            is_deleted: Set(false),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        }
    }

    /// 扫码结束工序（in_progress → completed）
    pub async fn complete_step(
        &self,
        id: i32,
        req: CompleteStepRequest,
    ) -> Result<StepModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != step_status::IN_PROGRESS {
            return Err(AppError::business(format!(
                "工序记录状态为 {}，仅 in_progress 状态可完成",
                model.status
            )));
        }

        // 业务校验：合格产量不能超过实际产量
        if let (Some(actual), Some(qualified)) = (req.actual_quantity, req.qualified_quantity) {
            if qualified > actual {
                return Err(AppError::business("合格产量不能超过实际产量"));
            }
        }

        let now = crate::utils::date_utils::utc_now_fixed();
        // 计算工时（分钟）
        let duration_minutes = (now - model.start_at).num_minutes();

        // 有异常描述则标记为 abnormal，否则 completed
        let has_abnormal = req.abnormal_description.is_some();
        let new_status = if has_abnormal {
            step_status::ABNORMAL.to_string()
        } else {
            step_status::COMPLETED.to_string()
        };

        let mut active: StepActiveModel = model.into();
        active.end_at = Set(Some(now));
        active.duration_minutes = Set(Some(duration_minutes as i32));
        active.actual_quantity = Set(req.actual_quantity);
        active.qualified_quantity = Set(req.qualified_quantity);
        active.abnormal_description = Set(req.abnormal_description);
        active.handling_opinion = Set(req.handling_opinion);
        active.remarks = Set(req.remarks);
        active.status = Set(new_status);
        active.updated_at = Set(now);

        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<StepModel, AppError> {
        let model = StepEntity::find_by_id(id)
            .filter(process_step_record::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("工序记录 {} 不存在", id)))?;
        Ok(model)
    }

    /// 按流转卡查询所有工序记录（按序号排序）
    pub async fn list_by_flow_card(&self, flow_card_id: i32) -> Result<Vec<StepModel>, AppError> {
        let list = StepEntity::find()
            .filter(process_step_record::Column::FlowCardId.eq(flow_card_id))
            .filter(process_step_record::Column::IsDeleted.eq(false))
            .order_by_asc(process_step_record::Column::StepSeq)
            .all(&*self.db)
            .await?;
        Ok(list)
    }

    /// 创建回修工序（关联原工序记录）
    pub async fn create_rework(
        &self,
        source_step_id: i32,
        req: StartStepRequest,
    ) -> Result<StepModel, AppError> {
        let source = self.get_by_id(source_step_id).await?;

        let mut rework_req = req;
        let now = crate::utils::date_utils::utc_now_fixed();

        let active = StepActiveModel {
            id: Default::default(),
            flow_card_id: Set(rework_req.flow_card_id),
            process_route_id: Set(rework_req.process_route_id.or(source.process_route_id)),
            step_seq: Set(source.step_seq),
            route_code: Set(source.route_code.clone()),
            route_name: Set(format!("回修-{}", source.route_name)),
            process_type: Set(source.process_type.clone()),
            worker_ids: Set(rework_req.worker_ids.take()),
            worker_names: Set(rework_req.worker_names.take()),
            equipment_id: Set(rework_req.equipment_id.take()),
            equipment_name: Set(rework_req.equipment_name.take()),
            start_at: Set(now),
            end_at: Set(None),
            duration_minutes: Set(None),
            planned_quantity: Set(source.planned_quantity),
            actual_quantity: Set(None),
            qualified_quantity: Set(None),
            status: Set(step_status::REWORK.to_string()),
            abnormal_description: Set(None),
            handling_opinion: Set(None),
            rework_source_id: Set(Some(source_step_id)),
            remarks: Set(None),
            is_deleted: Set(false),
            created_by: Set(rework_req.created_by.take()),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("回修工序创建失败: {}", e)))?;
        Ok(result)
    }
}
