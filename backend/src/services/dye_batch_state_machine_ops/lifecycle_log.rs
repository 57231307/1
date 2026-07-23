//! 缸号生命周期日志 Service impl 子模块（dye_batch_state_machine_ops/lifecycle_log）
//!
//! 批次 490 D10-4a 拆分：从原 `dye_batch_state_machine_service.rs` 迁移
//! DyeBatchLifecycleLogService 的 impl 块（6 个方法）。
//! 包含方法：record_transition / get_by_id / list_by_batch / list_by_batch_no
//! / get_latest_status / list。
//!
//! struct 定义 + new 构造函数保留在 facade（dye_batch_state_machine_service.rs），
//! 本模块通过 `impl DyeBatchLifecycleLogService` 追加业务方法。

use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    Set,
};

use crate::models::dye_batch_lifecycle_log::{
    self, ActiveModel as LifecycleLogActiveModel, Entity as LifecycleLogEntity,
    Model as LifecycleLogModel,
};
use crate::services::dye_batch_state_machine_service::{
    validate_lifecycle_status, validate_transition_code, validate_transition_with_rule,
    CreateTransitionRequest, DyeBatchLifecycleLogService, LifecycleLogQuery,
};
use crate::utils::error::AppError;

impl DyeBatchLifecycleLogService {
    /// 记录状态流转（含校验）
    pub async fn record_transition(
        &self,
        req: CreateTransitionRequest,
    ) -> Result<LifecycleLogModel, AppError> {
        // 校验 to_status 与 transition_code 合法
        validate_lifecycle_status(&req.to_status)?;
        validate_transition_code(&req.transition_code)?;
        if let Some(fs) = &req.from_status {
            validate_lifecycle_status(fs)?;
        }
        // 校验状态流转合法性
        validate_transition_with_rule(
            req.from_status.as_deref(),
            &req.to_status,
            &req.transition_code,
        )?;

        let now = crate::utils::date_utils::utc_now_fixed();
        let active = LifecycleLogActiveModel {
            id: Default::default(),
            batch_id: Set(req.batch_id),
            batch_no: Set(req.batch_no),
            from_status: Set(req.from_status),
            to_status: Set(req.to_status),
            transition_code: Set(req.transition_code),
            transition_name: Set(req.transition_name),
            operator_id: Set(req.operator_id),
            operator_name: Set(req.operator_name),
            equipment_id: Set(req.equipment_id),
            equipment_name: Set(req.equipment_name),
            work_shift: Set(req.work_shift),
            captured_params: Set(req.captured_params),
            remarks: Set(req.remarks),
            transition_at: Set(now),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("缸号生命周期日志创建失败: {}", e)))?;
        Ok(result)
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<LifecycleLogModel, AppError> {
        LifecycleLogEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("缸号生命周期日志 {} 不存在", id)))
    }

    /// 按缸号 ID 查询生命周期日志（按 transition_at 升序）
    pub async fn list_by_batch(&self, batch_id: i32) -> Result<Vec<LifecycleLogModel>, AppError> {
        let items = LifecycleLogEntity::find()
            .filter(dye_batch_lifecycle_log::Column::BatchId.eq(batch_id))
            .order_by_asc(dye_batch_lifecycle_log::Column::TransitionAt)
            .all(&*self.db)
            .await?;
        Ok(items)
    }

    /// 按缸号查询生命周期日志
    pub async fn list_by_batch_no(
        &self,
        batch_no: &str,
    ) -> Result<Vec<LifecycleLogModel>, AppError> {
        let items = LifecycleLogEntity::find()
            .filter(dye_batch_lifecycle_log::Column::BatchNo.eq(batch_no))
            .order_by_asc(dye_batch_lifecycle_log::Column::TransitionAt)
            .all(&*self.db)
            .await?;
        Ok(items)
    }

    /// 获取缸号最新状态（按 transition_at 倒序取第一条）
    pub async fn get_latest_status(
        &self,
        batch_id: i32,
    ) -> Result<Option<String>, AppError> {
        let model = LifecycleLogEntity::find()
            .filter(dye_batch_lifecycle_log::Column::BatchId.eq(batch_id))
            .order_by_desc(dye_batch_lifecycle_log::Column::TransitionAt)
            .one(&*self.db)
            .await?;
        Ok(model.map(|m| m.to_status))
    }

    /// 分页查询
    pub async fn list(
        &self,
        query: LifecycleLogQuery,
    ) -> Result<(Vec<LifecycleLogModel>, u64), AppError> {
        let mut q = LifecycleLogEntity::find();
        if let Some(v) = query.batch_id {
            q = q.filter(dye_batch_lifecycle_log::Column::BatchId.eq(v));
        }
        if let Some(v) = query.batch_no {
            q = q.filter(dye_batch_lifecycle_log::Column::BatchNo.contains(&v));
        }
        if let Some(v) = query.transition_code {
            q = q.filter(dye_batch_lifecycle_log::Column::TransitionCode.eq(v));
        }
        if let Some(kw) = query.keyword {
            q = q.filter(
                Condition::any()
                    .add(dye_batch_lifecycle_log::Column::BatchNo.contains(&kw))
                    .add(dye_batch_lifecycle_log::Column::OperatorName.contains(&kw))
                    .add(dye_batch_lifecycle_log::Column::EquipmentName.contains(&kw)),
            );
        }

        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

        let total = q.clone().count(&*self.db).await?;
        let items = q
            .order_by_desc(dye_batch_lifecycle_log::Column::Id)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;
        Ok((items, total))
    }
}
