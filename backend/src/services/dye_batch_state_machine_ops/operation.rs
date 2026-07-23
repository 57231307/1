//! 缸号操作记录 Service impl 子模块（dye_batch_state_machine_ops/operation）
//!
//! 批次 490 D10-4a 拆分：从原 `dye_batch_state_machine_service.rs` 迁移
//! DyeBatchOperationService 的 impl 块（5 个方法）。
//! 包含方法：create / get_by_id / list_by_type / list_by_batch / list。
//!
//! struct 定义 + new 构造函数保留在 facade（dye_batch_state_machine_service.rs），
//! 本模块通过 `impl DyeBatchOperationService` 追加业务方法。

use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    Set,
};

use crate::models::dye_batch_operation::{
    self, ActiveModel as OperationActiveModel, Entity as OperationEntity, Model as OperationModel,
};
use crate::services::dye_batch_state_machine_service::{
    validate_operation_type, CreateOperationRequest, DyeBatchOperationService, OperationQuery,
};
use crate::utils::error::AppError;

impl DyeBatchOperationService {
    /// 创建操作记录
    pub async fn create(&self, req: CreateOperationRequest) -> Result<OperationModel, AppError> {
        validate_operation_type(&req.operation_type)?;

        let now = crate::utils::date_utils::utc_now_fixed();
        let active = OperationActiveModel {
            id: Default::default(),
            operation_type: Set(req.operation_type),
            operation_name: Set(req.operation_name),
            target_batch_id: Set(req.target_batch_id),
            target_batch_no: Set(req.target_batch_no),
            source_batch_ids: Set(req.source_batch_ids),
            source_batch_nos: Set(req.source_batch_nos),
            operation_data: Set(req.operation_data),
            operator_id: Set(req.operator_id),
            operator_name: Set(req.operator_name),
            operation_at: Set(now),
            remarks: Set(req.remarks),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("缸号操作记录创建失败: {}", e)))?;
        Ok(result)
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<OperationModel, AppError> {
        OperationEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("缸号操作记录 {} 不存在", id)))
    }

    /// 按操作类型查询
    pub async fn list_by_type(
        &self,
        operation_type: &str,
    ) -> Result<Vec<OperationModel>, AppError> {
        validate_operation_type(operation_type)?;
        let items = OperationEntity::find()
            .filter(dye_batch_operation::Column::OperationType.eq(operation_type))
            .order_by_desc(dye_batch_operation::Column::OperationAt)
            .all(&*self.db)
            .await?;
        Ok(items)
    }

    /// 按目标缸号查询
    pub async fn list_by_batch(&self, target_batch_id: i32) -> Result<Vec<OperationModel>, AppError> {
        let items = OperationEntity::find()
            .filter(dye_batch_operation::Column::TargetBatchId.eq(target_batch_id))
            .order_by_desc(dye_batch_operation::Column::OperationAt)
            .all(&*self.db)
            .await?;
        Ok(items)
    }

    /// 分页查询
    pub async fn list(
        &self,
        query: OperationQuery,
    ) -> Result<(Vec<OperationModel>, u64), AppError> {
        let mut q = OperationEntity::find();
        if let Some(v) = query.operation_type {
            q = q.filter(dye_batch_operation::Column::OperationType.eq(v));
        }
        if let Some(v) = query.target_batch_id {
            q = q.filter(dye_batch_operation::Column::TargetBatchId.eq(v));
        }
        if let Some(kw) = query.keyword {
            q = q.filter(
                Condition::any()
                    .add(dye_batch_operation::Column::TargetBatchNo.contains(&kw))
                    .add(dye_batch_operation::Column::OperationName.contains(&kw))
                    .add(dye_batch_operation::Column::OperatorName.contains(&kw)),
            );
        }

        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

        let total = q.clone().count(&*self.db).await?;
        let items = q
            .order_by_desc(dye_batch_operation::Column::Id)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;
        Ok((items, total))
    }
}
