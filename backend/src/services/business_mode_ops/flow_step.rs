//! 业务模式流程节点 Service impl 子模块（business_mode_ops/flow_step）
//!
//! 批次 489 D10-2b 拆分：从原 `business_mode_service.rs` L875-1046 迁移。
//! 包含 BusinessModeFlowStepService 的 5 个方法：
//! - create / update / delete（CRUD）
//! - get_by_id / list_by_mode（查询）
//!
//! 业务规则：
//! - 流程节点物理删除（无软删除字段）
//! - 同模式内步骤序号唯一、步骤代码唯一
//! - list_by_mode 按 step_no 升序排序

use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};

use crate::models::business_mode_config::{self, Entity as ConfigEntity};
use crate::models::business_mode_flow_step::{
    self, ActiveModel as FlowStepActiveModel, Entity as FlowStepEntity, Model as FlowStepModel,
};
use crate::utils::error::AppError;

use crate::services::business_mode_service::BusinessModeFlowStepService;
use crate::services::business_mode_ops::types::{
    CreateBusinessModeFlowStepRequest, UpdateBusinessModeFlowStepRequest,
};

impl BusinessModeFlowStepService {
    /// 创建业务模式流程节点
    pub async fn create(
        &self,
        req: CreateBusinessModeFlowStepRequest,
    ) -> Result<FlowStepModel, AppError> {
        // 校验步骤序号非负
        if req.step_no < 1 {
            return Err(AppError::business("步骤序号必须从 1 开始"));
        }

        // 校验业务模式存在
        if ConfigEntity::find_by_id(req.mode_id)
            .filter(business_mode_config::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .is_none()
        {
            return Err(AppError::business(format!(
                "业务模式 {} 不存在",
                req.mode_id
            )));
        }

        // 校验同模式内步骤序号唯一
        if let Some(_existing) = FlowStepEntity::find()
            .filter(business_mode_flow_step::Column::ModeId.eq(req.mode_id))
            .filter(business_mode_flow_step::Column::StepNo.eq(req.step_no))
            .one(&*self.db)
            .await?
        {
            return Err(AppError::business(format!(
                "业务模式 {} 已存在步骤序号 {}",
                req.mode_id, req.step_no
            )));
        }

        // 校验同模式内步骤代码唯一
        if let Some(_existing) = FlowStepEntity::find()
            .filter(business_mode_flow_step::Column::ModeId.eq(req.mode_id))
            .filter(business_mode_flow_step::Column::StepCode.eq(&req.step_code))
            .one(&*self.db)
            .await?
        {
            return Err(AppError::business(format!(
                "业务模式 {} 已存在步骤代码 {}",
                req.mode_id, req.step_code
            )));
        }

        let now = crate::utils::date_utils::utc_now_fixed();
        let active = FlowStepActiveModel {
            id: Default::default(),
            mode_id: Set(req.mode_id),
            step_no: Set(req.step_no),
            step_code: Set(req.step_code),
            step_name: Set(req.step_name),
            module_name: Set(req.module_name),
            is_required: Set(req.is_required.unwrap_or(true)),
            description: Set(req.description),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("业务模式流程节点创建失败: {}", e)))?;
        Ok(result)
    }

    /// 更新业务模式流程节点
    pub async fn update(
        &self,
        id: i32,
        req: UpdateBusinessModeFlowStepRequest,
    ) -> Result<FlowStepModel, AppError> {
        let model = self.get_by_id(id).await?;
        let original_step_no = model.step_no;
        let original_step_code = model.step_code.clone();
        let mode_id = model.mode_id;

        let mut active: FlowStepActiveModel = model.into();

        if let Some(v) = req.step_no {
            if v < 1 {
                return Err(AppError::business("步骤序号必须从 1 开始"));
            }
            if v != original_step_no {
                // 校验同模式内步骤序号唯一
                if let Some(_existing) = FlowStepEntity::find()
                    .filter(business_mode_flow_step::Column::ModeId.eq(mode_id))
                    .filter(business_mode_flow_step::Column::StepNo.eq(v))
                    .one(&*self.db)
                    .await?
                {
                    return Err(AppError::business(format!(
                        "业务模式 {} 已存在步骤序号 {}",
                        mode_id, v
                    )));
                }
            }
            active.step_no = Set(v);
        }
        if let Some(v) = req.step_code {
            if v != original_step_code {
                // 校验同模式内步骤代码唯一
                if let Some(_existing) = FlowStepEntity::find()
                    .filter(business_mode_flow_step::Column::ModeId.eq(mode_id))
                    .filter(business_mode_flow_step::Column::StepCode.eq(&v))
                    .one(&*self.db)
                    .await?
                {
                    return Err(AppError::business(format!(
                        "业务模式 {} 已存在步骤代码 {}",
                        mode_id, v
                    )));
                }
            }
            active.step_code = Set(v);
        }
        if let Some(v) = req.step_name {
            active.step_name = Set(v);
        }
        if let Some(v) = req.module_name {
            active.module_name = Set(v);
        }
        if let Some(v) = req.is_required {
            active.is_required = Set(v);
        }
        if let Some(v) = req.description {
            active.description = Set(Some(v));
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 删除业务模式流程节点（物理删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        let _ = model;
        FlowStepEntity::delete_by_id(id)
            .exec(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("业务模式流程节点删除失败: {}", e)))?;
        Ok(())
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<FlowStepModel, AppError> {
        FlowStepEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("业务模式流程节点 {} 不存在", id)))
    }

    /// 按业务模式查询流程节点列表（按 step_no 排序）
    pub async fn list_by_mode(&self, mode_id: i32) -> Result<Vec<FlowStepModel>, AppError> {
        let items = FlowStepEntity::find()
            .filter(business_mode_flow_step::Column::ModeId.eq(mode_id))
            .order_by_asc(business_mode_flow_step::Column::StepNo)
            .all(&*self.db)
            .await?;
        Ok(items)
    }
}
