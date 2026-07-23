//! 缸号状态流转规则 Service impl 子模块（dye_batch_state_machine_ops/state_rule）
//!
//! 批次 490 D10-4a 拆分：从原 `dye_batch_state_machine_service.rs` 迁移
//! DyeBatchStateRuleService 的 impl 块（7 个方法）。
//! 包含方法：create / update / delete / get_by_id / check_transition
//! / list_allowed_transitions / list。
//!
//! struct 定义 + new 构造函数保留在 facade（dye_batch_state_machine_service.rs），
//! 本模块通过 `impl DyeBatchStateRuleService` 追加业务方法。

use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};

use crate::models::dye_batch_state_rule::{
    self, ActiveModel as StateRuleActiveModel, Entity as StateRuleEntity, Model as StateRuleModel,
};
use crate::services::dye_batch_state_machine_service::{
    validate_lifecycle_status, validate_transition_code, CreateStateRuleRequest,
    DyeBatchStateRuleService, StateRuleQuery, UpdateStateRuleRequest,
};
use crate::utils::error::AppError;

impl DyeBatchStateRuleService {
    /// 创建状态流转规则
    pub async fn create(&self, req: CreateStateRuleRequest) -> Result<StateRuleModel, AppError> {
        validate_lifecycle_status(&req.from_status)?;
        validate_lifecycle_status(&req.to_status)?;
        validate_transition_code(&req.transition_code)?;

        // 校验唯一性
        if let Some(_existing) = StateRuleEntity::find()
            .filter(dye_batch_state_rule::Column::FromStatus.eq(&req.from_status))
            .filter(dye_batch_state_rule::Column::ToStatus.eq(&req.to_status))
            .filter(dye_batch_state_rule::Column::TransitionCode.eq(&req.transition_code))
            .one(&*self.db)
            .await?
        {
            return Err(AppError::business(format!(
                "状态流转规则 {} → {}（{}）已存在",
                req.from_status, req.to_status, req.transition_code
            )));
        }

        let now = crate::utils::date_utils::utc_now_fixed();
        let active = StateRuleActiveModel {
            id: Default::default(),
            from_status: Set(req.from_status),
            to_status: Set(req.to_status),
            transition_code: Set(req.transition_code),
            transition_name: Set(req.transition_name),
            is_allowed: Set(req.is_allowed.unwrap_or(true)),
            require_operator: Set(req.require_operator.unwrap_or(true)),
            require_equipment: Set(req.require_equipment.unwrap_or(false)),
            require_remarks: Set(req.require_remarks.unwrap_or(false)),
            validation_logic: Set(req.validation_logic),
            description: Set(req.description),
            is_active: Set(req.is_active.unwrap_or(true)),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("缸号状态流转规则创建失败: {}", e)))?;
        Ok(result)
    }

    /// 更新状态流转规则
    pub async fn update(
        &self,
        id: i32,
        req: UpdateStateRuleRequest,
    ) -> Result<StateRuleModel, AppError> {
        let model = self.get_by_id(id).await?;
        let mut active: StateRuleActiveModel = model.into();

        if let Some(v) = req.transition_name {
            active.transition_name = Set(v);
        }
        if let Some(v) = req.is_allowed {
            active.is_allowed = Set(v);
        }
        if let Some(v) = req.require_operator {
            active.require_operator = Set(v);
        }
        if let Some(v) = req.require_equipment {
            active.require_equipment = Set(v);
        }
        if let Some(v) = req.require_remarks {
            active.require_remarks = Set(v);
        }
        if let Some(v) = req.validation_logic {
            active.validation_logic = Set(Some(v));
        }
        if let Some(v) = req.description {
            active.description = Set(Some(v));
        }
        if let Some(v) = req.is_active {
            active.is_active = Set(v);
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 删除状态流转规则（物理删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        let _ = model;
        StateRuleEntity::delete_by_id(id)
            .exec(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("缸号状态流转规则删除失败: {}", e)))?;
        Ok(())
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<StateRuleModel, AppError> {
        StateRuleEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("缸号状态流转规则 {} 不存在", id)))
    }

    /// 查 DB 校验状态流转合法性
    pub async fn check_transition(
        &self,
        from_status: Option<&str>,
        to_status: &str,
        transition_code: &str,
    ) -> Result<bool, AppError> {
        let mut q = StateRuleEntity::find()
            .filter(dye_batch_state_rule::Column::ToStatus.eq(to_status))
            .filter(dye_batch_state_rule::Column::TransitionCode.eq(transition_code))
            .filter(dye_batch_state_rule::Column::IsAllowed.eq(true))
            .filter(dye_batch_state_rule::Column::IsActive.eq(true));
        if let Some(fs) = from_status {
            q = q.filter(dye_batch_state_rule::Column::FromStatus.eq(fs));
        }
        let count = q.count(&*self.db).await?;
        Ok(count > 0)
    }

    /// 查询允许的流转列表
    pub async fn list_allowed_transitions(
        &self,
        from_status: Option<&str>,
    ) -> Result<Vec<StateRuleModel>, AppError> {
        let mut q = StateRuleEntity::find()
            .filter(dye_batch_state_rule::Column::IsAllowed.eq(true))
            .filter(dye_batch_state_rule::Column::IsActive.eq(true));
        if let Some(fs) = from_status {
            q = q.filter(dye_batch_state_rule::Column::FromStatus.eq(fs));
        }
        let items = q
            .order_by_asc(dye_batch_state_rule::Column::Id)
            .all(&*self.db)
            .await?;
        Ok(items)
    }

    /// 分页查询
    pub async fn list(
        &self,
        query: StateRuleQuery,
    ) -> Result<(Vec<StateRuleModel>, u64), AppError> {
        let mut q = StateRuleEntity::find();
        if let Some(v) = query.from_status {
            q = q.filter(dye_batch_state_rule::Column::FromStatus.eq(v));
        }
        if let Some(v) = query.to_status {
            q = q.filter(dye_batch_state_rule::Column::ToStatus.eq(v));
        }
        if let Some(v) = query.transition_code {
            q = q.filter(dye_batch_state_rule::Column::TransitionCode.eq(v));
        }
        if let Some(v) = query.is_active {
            q = q.filter(dye_batch_state_rule::Column::IsActive.eq(v));
        }

        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

        let total = q.clone().count(&*self.db).await?;
        let items = q
            .order_by_desc(dye_batch_state_rule::Column::Id)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;
        Ok((items, total))
    }
}
