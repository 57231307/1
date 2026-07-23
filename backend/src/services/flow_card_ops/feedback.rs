//! 工序质量反馈单 Service impl 子模块（flow_card_ops/feedback）
//!
//! D10 第 5 批拆分：从原 flow_card_service.rs 迁移 QualityFeedbackService 的 5 个业务方法
//!（create / handle / close / get_by_id / list_by_flow_card）。
//! 单号生成纯函数 generate_feedback_no 保留在 facade，本模块通过 Self:: 调用。

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};

use crate::models::process_quality_feedback::{
    self, ActiveModel as FeedbackActiveModel, Entity as FeedbackEntity, Model as FeedbackModel,
};
use crate::models::production_flow_card::{self, Entity as CardEntity};
use crate::models::status::quality_feedback as feedback_status;
use crate::services::flow_card_service::{
    CreateFeedbackRequest, HandleFeedbackRequest, QualityFeedbackService,
};
use crate::utils::error::AppError;

impl QualityFeedbackService {
    /// 创建质量反馈单
    pub async fn create(&self, req: CreateFeedbackRequest) -> Result<FeedbackModel, AppError> {
        // 业务校验：反馈类型合法
        let valid_types = ["abnormal", "rework", "defect", "other"];
        if !valid_types.contains(&req.feedback_type.as_str()) {
            return Err(AppError::business(format!(
                "反馈类型必须是 {:?} 之一",
                valid_types
            )));
        }

        // 业务校验：严重等级合法
        let severity = req.severity.unwrap_or_else(|| "medium".to_string());
        let valid_severities = ["low", "medium", "high", "critical"];
        if !valid_severities.contains(&severity.as_str()) {
            return Err(AppError::business(format!(
                "严重等级必须是 {:?} 之一",
                valid_severities
            )));
        }

        // 校验流转卡存在
        let card_exists = CardEntity::find_by_id(req.flow_card_id)
            .filter(production_flow_card::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .is_some();
        if !card_exists {
            return Err(AppError::not_found(format!(
                "流转卡 {} 不存在",
                req.flow_card_id
            )));
        }

        let feedback_no = Self::generate_feedback_no();
        let now = crate::utils::date_utils::utc_now_fixed();

        let active = FeedbackActiveModel {
            id: Default::default(),
            feedback_no: Set(feedback_no),
            flow_card_id: Set(req.flow_card_id),
            step_record_id: Set(req.step_record_id),
            feedback_type: Set(req.feedback_type),
            description: Set(req.description),
            severity: Set(severity),
            found_by: Set(req.found_by),
            found_at: Set(now),
            handling_opinion: Set(None),
            handled_by: Set(None),
            handled_at: Set(None),
            handling_result: Set(None),
            status: Set(feedback_status::PENDING.to_string()),
            remarks: Set(req.remarks),
            is_deleted: Set(false),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("质量反馈单创建失败: {}", e)))?;
        Ok(result)
    }

    /// 处理反馈单（pending → processing → resolved）
    pub async fn handle(
        &self,
        id: i32,
        req: HandleFeedbackRequest,
    ) -> Result<FeedbackModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status == feedback_status::CLOSED {
            return Err(AppError::business("已关闭的反馈单不可再处理"));
        }

        let now = crate::utils::date_utils::utc_now_fixed();
        let mut active: FeedbackActiveModel = model.into();

        if let Some(v) = req.handling_opinion {
            active.handling_opinion = Set(Some(v));
        }
        if let Some(v) = req.handled_by {
            active.handled_by = Set(Some(v));
            active.handled_at = Set(Some(now));
        }

        // 状态流转：pending → processing（有处理意见但无结果）→ resolved（有处理结果）
        let new_status = if req.handling_result.is_some() {
            active.handling_result = Set(req.handling_result);
            feedback_status::RESOLVED.to_string()
        } else {
            feedback_status::PROCESSING.to_string()
        };
        active.status = Set(new_status);
        active.updated_at = Set(now);

        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 关闭反馈单（resolved → closed）
    pub async fn close(&self, id: i32) -> Result<FeedbackModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != feedback_status::RESOLVED {
            return Err(AppError::business(format!(
                "反馈单状态为 {}，仅 resolved 状态可关闭",
                model.status
            )));
        }

        let mut active: FeedbackActiveModel = model.into();
        active.status = Set(feedback_status::CLOSED.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<FeedbackModel, AppError> {
        let model = FeedbackEntity::find_by_id(id)
            .filter(process_quality_feedback::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("质量反馈单 {} 不存在", id)))?;
        Ok(model)
    }

    /// 按流转卡查询反馈单
    pub async fn list_by_flow_card(
        &self,
        flow_card_id: i32,
    ) -> Result<Vec<FeedbackModel>, AppError> {
        let list = FeedbackEntity::find()
            .filter(process_quality_feedback::Column::FlowCardId.eq(flow_card_id))
            .filter(process_quality_feedback::Column::IsDeleted.eq(false))
            .order_by_desc(process_quality_feedback::Column::FoundAt)
            .all(&*self.db)
            .await?;
        Ok(list)
    }
}
