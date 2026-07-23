//! 业务模式规则 Service impl 子模块（business_mode_ops/rule）
//!
//! 批次 489 D10-2b 拆分：从原 `business_mode_service.rs` L1082-1222 迁移。
//! 包含 BusinessModeRuleService 的 5 个方法：
//! - create / update / delete（CRUD）
//! - get_by_id / list_by_mode（查询）
//!
//! 业务规则：
//! - 规则物理删除（无软删除字段）
//! - 同模式内规则代码唯一
//! - list_by_mode 仅返回 is_active=true 的规则，按 id 升序排序

use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};

use crate::models::business_mode_config::{self, Entity as ConfigEntity};
use crate::models::business_mode_rule::{
    self, ActiveModel as RuleActiveModel, Entity as RuleEntity, Model as RuleModel,
};
use crate::utils::error::AppError;

use crate::services::business_mode_service::{validate_rule_type, BusinessModeRuleService};
use crate::services::business_mode_ops::types::{
    CreateBusinessModeRuleRequest, UpdateBusinessModeRuleRequest,
};

impl BusinessModeRuleService {
    /// 创建业务模式规则
    pub async fn create(&self, req: CreateBusinessModeRuleRequest) -> Result<RuleModel, AppError> {
        validate_rule_type(&req.rule_type)?;

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

        // 校验同模式内规则代码唯一
        if let Some(_existing) = RuleEntity::find()
            .filter(business_mode_rule::Column::ModeId.eq(req.mode_id))
            .filter(business_mode_rule::Column::RuleCode.eq(&req.rule_code))
            .one(&*self.db)
            .await?
        {
            return Err(AppError::business(format!(
                "业务模式 {} 已存在规则代码 {}",
                req.mode_id, req.rule_code
            )));
        }

        let now = crate::utils::date_utils::utc_now_fixed();
        let active = RuleActiveModel {
            id: Default::default(),
            mode_id: Set(req.mode_id),
            rule_code: Set(req.rule_code),
            rule_name: Set(req.rule_name),
            rule_type: Set(req.rule_type),
            module_name: Set(req.module_name),
            validation_logic: Set(req.validation_logic),
            description: Set(req.description),
            is_active: Set(req.is_active.unwrap_or(true)),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("业务模式规则创建失败: {}", e)))?;
        Ok(result)
    }

    /// 更新业务模式规则
    pub async fn update(
        &self,
        id: i32,
        req: UpdateBusinessModeRuleRequest,
    ) -> Result<RuleModel, AppError> {
        let model = self.get_by_id(id).await?;
        let original_rule_code = model.rule_code.clone();
        let mode_id = model.mode_id;

        let mut active: RuleActiveModel = model.into();

        if let Some(v) = req.rule_code {
            if v != original_rule_code {
                // 校验同模式内规则代码唯一
                if let Some(_existing) = RuleEntity::find()
                    .filter(business_mode_rule::Column::ModeId.eq(mode_id))
                    .filter(business_mode_rule::Column::RuleCode.eq(&v))
                    .one(&*self.db)
                    .await?
                {
                    return Err(AppError::business(format!(
                        "业务模式 {} 已存在规则代码 {}",
                        mode_id, v
                    )));
                }
            }
            active.rule_code = Set(v);
        }
        if let Some(v) = req.rule_name {
            active.rule_name = Set(v);
        }
        if let Some(v) = req.rule_type {
            validate_rule_type(&v)?;
            active.rule_type = Set(v);
        }
        if let Some(v) = req.module_name {
            active.module_name = Set(v);
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

    /// 删除业务模式规则（物理删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        let _ = model;
        RuleEntity::delete_by_id(id)
            .exec(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("业务模式规则删除失败: {}", e)))?;
        Ok(())
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<RuleModel, AppError> {
        RuleEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("业务模式规则 {} 不存在", id)))
    }

    /// 按业务模式查询规则列表
    pub async fn list_by_mode(&self, mode_id: i32) -> Result<Vec<RuleModel>, AppError> {
        let items = RuleEntity::find()
            .filter(business_mode_rule::Column::ModeId.eq(mode_id))
            .filter(business_mode_rule::Column::IsActive.eq(true))
            .order_by_asc(business_mode_rule::Column::Id)
            .all(&*self.db)
            .await?;
        Ok(items)
    }
}
