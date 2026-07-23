//! 缸号回修记录 Service impl 子模块（dye_batch_state_machine_ops/rework）
//!
//! 批次 490 D10-4a 拆分：从原 `dye_batch_state_machine_service.rs` 迁移
//! DyeBatchReworkService 的 impl 块（9 个方法）。
//! 包含方法：create / update / delete / get_by_id / approve / start_rework
//! / complete_rework / cancel_rework / list。
//!
//! struct 定义 + new 构造函数保留在 facade（dye_batch_state_machine_service.rs），
//! 本模块通过 `impl DyeBatchReworkService` 追加业务方法。

use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    Set,
};

use crate::models::dye_batch_rework::{
    self, ActiveModel as ReworkActiveModel, Entity as ReworkEntity, Model as ReworkModel,
};
use crate::models::status::dye_batch_rework_status;
use crate::services::dye_batch_state_machine_service::{
    check_rework_eligibility, validate_lifecycle_status, validate_rework_type,
    CreateReworkRequest, DyeBatchReworkService, ReworkQuery, UpdateReworkRequest,
};
use crate::utils::error::AppError;

impl DyeBatchReworkService {
    /// 创建回修记录
    pub async fn create(&self, req: CreateReworkRequest) -> Result<ReworkModel, AppError> {
        validate_rework_type(&req.rework_type)?;
        validate_lifecycle_status(&req.original_status)?;
        // 校验回修资格（只有 inspecting/stored 状态可回修）
        check_rework_eligibility(&req.original_status)?;

        let now = crate::utils::date_utils::utc_now_fixed();
        let active = ReworkActiveModel {
            id: Default::default(),
            original_batch_id: Set(req.original_batch_id),
            original_batch_no: Set(req.original_batch_no),
            rework_batch_id: Set(req.rework_batch_id),
            rework_batch_no: Set(req.rework_batch_no),
            rework_type: Set(req.rework_type),
            rework_reason: Set(req.rework_reason),
            original_status: Set(req.original_status),
            approved_by: Set(None),
            approved_at: Set(None),
            status: Set(dye_batch_rework_status::DRAFT.to_string()),
            started_at: Set(None),
            completed_at: Set(None),
            remarks: Set(req.remarks),
            is_deleted: Set(false),
            // V15 Batch 479 P0-F21：返工走生产订单流程（创建时未关联生产订单，后续回填）
            production_order_id: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("缸号回修记录创建失败: {}", e)))?;
        Ok(result)
    }

    /// 更新回修记录（仅 draft 状态可编辑）
    pub async fn update(&self, id: i32, req: UpdateReworkRequest) -> Result<ReworkModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != dye_batch_rework_status::DRAFT {
            return Err(AppError::business(format!(
                "只有草稿状态的回修单可编辑，当前状态: {}",
                model.status
            )));
        }
        let mut active: ReworkActiveModel = model.into();

        if let Some(v) = req.rework_type {
            validate_rework_type(&v)?;
            active.rework_type = Set(v);
        }
        if let Some(v) = req.rework_reason {
            active.rework_reason = Set(v);
        }
        if let Some(v) = req.rework_batch_id {
            active.rework_batch_id = Set(Some(v));
        }
        if let Some(v) = req.rework_batch_no {
            active.rework_batch_no = Set(Some(v));
        }
        if let Some(v) = req.remarks {
            active.remarks = Set(Some(v));
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 软删除回修记录
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        if model.status == dye_batch_rework_status::IN_PROGRESS
            || model.status == dye_batch_rework_status::COMPLETED
        {
            return Err(AppError::business(format!(
                "回修中/已完成的回修单不可删除，当前状态: {}",
                model.status
            )));
        }
        let mut active: ReworkActiveModel = model.into();
        active.is_deleted = Set(true);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        active.update(&*self.db).await?;
        Ok(())
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<ReworkModel, AppError> {
        ReworkEntity::find_by_id(id)
            .filter(dye_batch_rework::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("缸号回修记录 {} 不存在", id)))
    }

    /// 审批回修单（draft → approved）
    pub async fn approve(&self, id: i32, approved_by: i32) -> Result<ReworkModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != dye_batch_rework_status::DRAFT {
            return Err(AppError::business(format!(
                "只有草稿状态的回修单可审批，当前状态: {}",
                model.status
            )));
        }
        let now = crate::utils::date_utils::utc_now_fixed();
        let mut active: ReworkActiveModel = model.into();
        active.status = Set(dye_batch_rework_status::APPROVED.to_string());
        active.approved_by = Set(Some(approved_by));
        active.approved_at = Set(Some(now));
        active.updated_at = Set(now);
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 开始回修（approved → in_progress）
    pub async fn start_rework(&self, id: i32) -> Result<ReworkModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != dye_batch_rework_status::APPROVED {
            return Err(AppError::business(format!(
                "只有已审批的回修单可开始回修，当前状态: {}",
                model.status
            )));
        }
        let now = crate::utils::date_utils::utc_now_fixed();
        let mut active: ReworkActiveModel = model.into();
        active.status = Set(dye_batch_rework_status::IN_PROGRESS.to_string());
        active.started_at = Set(Some(now));
        active.updated_at = Set(now);
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 完成回修（in_progress → completed）
    pub async fn complete_rework(&self, id: i32) -> Result<ReworkModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != dye_batch_rework_status::IN_PROGRESS {
            return Err(AppError::business(format!(
                "只有回修中的回修单可完成回修，当前状态: {}",
                model.status
            )));
        }
        let now = crate::utils::date_utils::utc_now_fixed();
        let mut active: ReworkActiveModel = model.into();
        active.status = Set(dye_batch_rework_status::COMPLETED.to_string());
        active.completed_at = Set(Some(now));
        active.updated_at = Set(now);
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 取消回修单（非 completed → cancelled）
    pub async fn cancel_rework(&self, id: i32) -> Result<ReworkModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status == dye_batch_rework_status::COMPLETED {
            return Err(AppError::business("已完成的回修单不可取消"));
        }
        if model.status == dye_batch_rework_status::CANCELLED {
            return Err(AppError::business("回修单已是取消状态"));
        }
        let now = crate::utils::date_utils::utc_now_fixed();
        let mut active: ReworkActiveModel = model.into();
        active.status = Set(dye_batch_rework_status::CANCELLED.to_string());
        active.updated_at = Set(now);
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 分页查询
    pub async fn list(&self, query: ReworkQuery) -> Result<(Vec<ReworkModel>, u64), AppError> {
        let mut q = ReworkEntity::find()
            .filter(dye_batch_rework::Column::IsDeleted.eq(false));
        if let Some(v) = query.original_batch_id {
            q = q.filter(dye_batch_rework::Column::OriginalBatchId.eq(v));
        }
        if let Some(v) = query.rework_batch_id {
            q = q.filter(dye_batch_rework::Column::ReworkBatchId.eq(v));
        }
        if let Some(v) = query.rework_type {
            q = q.filter(dye_batch_rework::Column::ReworkType.eq(v));
        }
        if let Some(v) = query.status {
            q = q.filter(dye_batch_rework::Column::Status.eq(v));
        }
        if let Some(kw) = query.keyword {
            q = q.filter(
                Condition::any()
                    .add(dye_batch_rework::Column::OriginalBatchNo.contains(&kw))
                    .add(dye_batch_rework::Column::ReworkBatchNo.contains(&kw))
                    .add(dye_batch_rework::Column::ReworkReason.contains(&kw)),
            );
        }

        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

        let total = q.clone().count(&*self.db).await?;
        let items = q
            .order_by_desc(dye_batch_rework::Column::Id)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;
        Ok((items, total))
    }
}
