//! 业务模式配置 Service impl 子模块（business_mode_ops/config）
//!
//! 批次 489 D10-2b 拆分：从原 `business_mode_service.rs` L465-841 迁移。
//! 包含 BusinessModeConfigService 的 11 个方法：
//! - create / update / delete（CRUD）
//! - get_by_id / get_by_code / get_default_mode（查询）
//! - set_default（默认模式管理）
//! - get_with_flow_steps / get_with_rules / get_full_detail（详情查询）
//! - list（分页查询）
//! - clear_other_defaults / apply_basic_fields / apply_validated_fields / apply_module_flags（私有 helper）
//!
//! 业务规则：
//! - 6 种业务模式一致性校验（§6 业务模式 6 种）
//! - 默认模式全局唯一（set_default 时清除其他默认）
//! - 被单据引用的模式不可删除

use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    Set,
};

use crate::models::business_mode_config::{
    self, ActiveModel as ConfigActiveModel, Entity as ConfigEntity, Model as ConfigModel,
};
use crate::models::business_mode_flow_step::{self, Entity as FlowStepEntity, Model as FlowStepModel};
use crate::models::business_mode_order_link::{self, Entity as OrderLinkEntity};
use crate::models::business_mode_rule::{self, Entity as RuleEntity, Model as RuleModel};
use crate::utils::error::AppError;

use crate::services::business_mode_service::{
    check_module_consistency, validate_cost_method, validate_inventory_type, validate_material_source,
    validate_mode_category, validate_mode_code, validate_settlement_method,
    BusinessModeConfigService,
};
use crate::services::business_mode_ops::types::{
    BusinessModeConfigQuery, CreateBusinessModeConfigRequest, UpdateBusinessModeConfigRequest,
};

/// 更新时用于一致性校验的最终值聚合
struct FinalConsistencyValues {
    material_source: String,
    settlement_method: String,
    require_purchase: bool,
    require_production: bool,
    require_outsourcing: bool,
    require_sales: bool,
}

impl BusinessModeConfigService {
    /// 创建业务模式配置
    pub async fn create(
        &self,
        req: CreateBusinessModeConfigRequest,
    ) -> Result<ConfigModel, AppError> {
        validate_mode_code(&req.mode_code)?;
        validate_material_source(&req.material_source)?;
        validate_settlement_method(&req.settlement_method)?;
        validate_inventory_type(&req.inventory_type)?;
        validate_cost_method(&req.cost_method)?;
        validate_mode_category(&req.mode_category)?;

        // 校验业务模式配置一致性
        check_module_consistency(
            &req.mode_code,
            req.require_purchase.unwrap_or(false),
            req.require_production.unwrap_or(false),
            req.require_outsourcing.unwrap_or(false),
            req.require_sales.unwrap_or(false),
            &req.material_source,
            &req.settlement_method,
        )?;

        // 校验模式代码唯一性
        if let Some(_existing) = ConfigEntity::find()
            .filter(business_mode_config::Column::ModeCode.eq(&req.mode_code))
            .filter(business_mode_config::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
        {
            return Err(AppError::business(format!(
                "业务模式代码 {} 已存在",
                req.mode_code
            )));
        }

        let now = crate::utils::date_utils::utc_now_fixed();
        let process_chain = req
            .process_chain
            .unwrap_or_else(|| serde_json::Value::Array(vec![]));

        let active = ConfigActiveModel {
            id: Default::default(),
            mode_code: Set(req.mode_code),
            mode_name: Set(req.mode_name),
            description: Set(req.description),
            is_active: Set(req.is_active.unwrap_or(true)),
            is_default: Set(req.is_default.unwrap_or(false)),
            process_chain: Set(process_chain),
            material_source: Set(req.material_source),
            settlement_method: Set(req.settlement_method),
            inventory_type: Set(req.inventory_type),
            cost_method: Set(req.cost_method),
            require_purchase: Set(req.require_purchase.unwrap_or(false)),
            require_production: Set(req.require_production.unwrap_or(false)),
            require_outsourcing: Set(req.require_outsourcing.unwrap_or(false)),
            require_sales: Set(req.require_sales.unwrap_or(false)),
            mode_category: Set(req.mode_category),
            remarks: Set(req.remarks),
            is_deleted: Set(false),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("业务模式配置创建失败: {}", e)))?;

        // 若设置为默认，先清除其他默认
        if result.is_default {
            self.clear_other_defaults(result.id).await?;
        }

        Ok(result)
    }

    /// 更新业务模式配置
    pub async fn update(
        &self,
        id: i32,
        req: UpdateBusinessModeConfigRequest,
    ) -> Result<ConfigModel, AppError> {
        let model = self.get_by_id(id).await?;
        let original_mode_code = model.mode_code.clone();
        let mut finals = FinalConsistencyValues {
            material_source: model.material_source.clone(),
            settlement_method: model.settlement_method.clone(),
            require_purchase: model.require_purchase,
            require_production: model.require_production,
            require_outsourcing: model.require_outsourcing,
            require_sales: model.require_sales,
        };
        let mut active: ConfigActiveModel = model.into();
        Self::apply_basic_fields(&mut active, &req);
        Self::apply_validated_fields(&mut active, &req, &mut finals)?;
        Self::apply_module_flags(&mut active, &req, &mut finals);
        check_module_consistency(
            &original_mode_code,
            finals.require_purchase,
            finals.require_production,
            finals.require_outsourcing,
            finals.require_sales,
            &finals.material_source,
            &finals.settlement_method,
        )?;
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        if updated.is_default {
            self.clear_other_defaults(updated.id).await?;
        }
        Ok(updated)
    }

    /// 应用基础字段（无校验）
    fn apply_basic_fields(
        active: &mut ConfigActiveModel,
        req: &UpdateBusinessModeConfigRequest,
    ) {
        if let Some(v) = &req.mode_name {
            active.mode_name = Set(v.clone());
        }
        if let Some(v) = &req.description {
            active.description = Set(Some(v.clone()));
        }
        if let Some(v) = req.is_active {
            active.is_active = Set(v);
        }
        if let Some(v) = req.is_default {
            active.is_default = Set(v);
        }
        if let Some(v) = &req.process_chain {
            active.process_chain = Set(v.clone());
        }
        if let Some(v) = &req.remarks {
            active.remarks = Set(Some(v.clone()));
        }
    }

    /// 应用需校验的字段（material_source/settlement_method/inventory_type/cost_method/mode_category）
    fn apply_validated_fields(
        active: &mut ConfigActiveModel,
        req: &UpdateBusinessModeConfigRequest,
        finals: &mut FinalConsistencyValues,
    ) -> Result<(), AppError> {
        if let Some(v) = &req.material_source {
            validate_material_source(v)?;
            finals.material_source = v.clone();
            active.material_source = Set(v.clone());
        }
        if let Some(v) = &req.settlement_method {
            validate_settlement_method(v)?;
            finals.settlement_method = v.clone();
            active.settlement_method = Set(v.clone());
        }
        if let Some(v) = &req.inventory_type {
            validate_inventory_type(v)?;
            active.inventory_type = Set(v.clone());
        }
        if let Some(v) = &req.cost_method {
            validate_cost_method(v)?;
            active.cost_method = Set(v.clone());
        }
        if let Some(v) = &req.mode_category {
            validate_mode_category(v)?;
            active.mode_category = Set(v.clone());
        }
        Ok(())
    }

    /// 应用模块标志字段（require_*）
    fn apply_module_flags(
        active: &mut ConfigActiveModel,
        req: &UpdateBusinessModeConfigRequest,
        finals: &mut FinalConsistencyValues,
    ) {
        if let Some(v) = req.require_purchase {
            finals.require_purchase = v;
            active.require_purchase = Set(v);
        }
        if let Some(v) = req.require_production {
            finals.require_production = v;
            active.require_production = Set(v);
        }
        if let Some(v) = req.require_outsourcing {
            finals.require_outsourcing = v;
            active.require_outsourcing = Set(v);
        }
        if let Some(v) = req.require_sales {
            finals.require_sales = v;
            active.require_sales = Set(v);
        }
    }

    /// 软删除业务模式配置
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;

        // 校验是否被单据引用（被引用的模式不可删除）
        let linked_count = OrderLinkEntity::find()
            .filter(business_mode_order_link::Column::ModeId.eq(id))
            .count(&*self.db)
            .await?;
        if linked_count > 0 {
            return Err(AppError::business(format!(
                "业务模式 {} 已被 {} 个单据引用，不可删除",
                model.mode_code, linked_count
            )));
        }

        let mut active: ConfigActiveModel = model.into();
        active.is_deleted = Set(true);
        active.is_active = Set(false);
        active.is_default = Set(false);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        active.update(&*self.db).await?;
        Ok(())
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<ConfigModel, AppError> {
        ConfigEntity::find_by_id(id)
            .filter(business_mode_config::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("业务模式配置 {} 不存在", id)))
    }

    /// 按模式代码查询
    pub async fn get_by_code(&self, mode_code: &str) -> Result<ConfigModel, AppError> {
        ConfigEntity::find()
            .filter(business_mode_config::Column::ModeCode.eq(mode_code))
            .filter(business_mode_config::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("业务模式代码 {} 不存在", mode_code)))
    }

    /// 查询默认业务模式
    pub async fn get_default_mode(&self) -> Result<Option<ConfigModel>, AppError> {
        let model = ConfigEntity::find()
            .filter(business_mode_config::Column::IsDefault.eq(true))
            .filter(business_mode_config::Column::IsDeleted.eq(false))
            .filter(business_mode_config::Column::IsActive.eq(true))
            .one(&*self.db)
            .await?;
        Ok(model)
    }

    /// 设置默认业务模式（先清除其他默认，再设置当前）
    pub async fn set_default(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        if !model.is_active {
            return Err(AppError::business("未启用的业务模式不可设置为默认"));
        }

        self.clear_other_defaults(id).await?;

        let mut active: ConfigActiveModel = model.into();
        active.is_default = Set(true);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        active.update(&*self.db).await?;
        Ok(())
    }

    /// 获取业务模式详情（含流程节点）
    pub async fn get_with_flow_steps(
        &self,
        id: i32,
    ) -> Result<(ConfigModel, Vec<FlowStepModel>), AppError> {
        let model = self.get_by_id(id).await?;
        let steps = FlowStepEntity::find()
            .filter(business_mode_flow_step::Column::ModeId.eq(id))
            .order_by_asc(business_mode_flow_step::Column::StepNo)
            .all(&*self.db)
            .await?;
        Ok((model, steps))
    }

    /// 获取业务模式详情（含规则）
    pub async fn get_with_rules(
        &self,
        id: i32,
    ) -> Result<(ConfigModel, Vec<RuleModel>), AppError> {
        let model = self.get_by_id(id).await?;
        let rules = RuleEntity::find()
            .filter(business_mode_rule::Column::ModeId.eq(id))
            .filter(business_mode_rule::Column::IsActive.eq(true))
            .order_by_asc(business_mode_rule::Column::Id)
            .all(&*self.db)
            .await?;
        Ok((model, rules))
    }

    /// 获取业务模式完整详情（含流程节点+规则）
    pub async fn get_full_detail(
        &self,
        id: i32,
    ) -> Result<(ConfigModel, Vec<FlowStepModel>, Vec<RuleModel>), AppError> {
        let (model, steps) = self.get_with_flow_steps(id).await?;
        let rules = RuleEntity::find()
            .filter(business_mode_rule::Column::ModeId.eq(id))
            .filter(business_mode_rule::Column::IsActive.eq(true))
            .order_by_asc(business_mode_rule::Column::Id)
            .all(&*self.db)
            .await?;
        Ok((model, steps, rules))
    }

    /// 分页查询
    pub async fn list(
        &self,
        query: BusinessModeConfigQuery,
    ) -> Result<(Vec<ConfigModel>, u64), AppError> {
        let mut q = ConfigEntity::find()
            .filter(business_mode_config::Column::IsDeleted.eq(false));
        if let Some(v) = query.mode_code {
            q = q.filter(business_mode_config::Column::ModeCode.eq(v));
        }
        if let Some(v) = query.mode_category {
            q = q.filter(business_mode_config::Column::ModeCategory.eq(v));
        }
        if let Some(v) = query.is_active {
            q = q.filter(business_mode_config::Column::IsActive.eq(v));
        }
        if let Some(v) = query.material_source {
            q = q.filter(business_mode_config::Column::MaterialSource.eq(v));
        }
        if let Some(v) = query.settlement_method {
            q = q.filter(business_mode_config::Column::SettlementMethod.eq(v));
        }
        if let Some(kw) = query.keyword {
            q = q.filter(
                Condition::any()
                    .add(business_mode_config::Column::ModeCode.contains(&kw))
                    .add(business_mode_config::Column::ModeName.contains(&kw))
                    .add(business_mode_config::Column::Description.contains(&kw)),
            );
        }

        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

        let total = q.clone().count(&*self.db).await?;
        let items = q
            .order_by_desc(business_mode_config::Column::Id)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;
        Ok((items, total))
    }

    /// 清除其他默认模式（内部方法）
    async fn clear_other_defaults(&self, exclude_id: i32) -> Result<(), AppError> {
        let others = ConfigEntity::find()
            .filter(business_mode_config::Column::IsDefault.eq(true))
            .filter(business_mode_config::Column::IsDeleted.eq(false))
            .filter(business_mode_config::Column::Id.ne(exclude_id))
            .all(&*self.db)
            .await?;

        let now = crate::utils::date_utils::utc_now_fixed();
        for m in others {
            let mut active: ConfigActiveModel = m.into();
            active.is_default = Set(false);
            active.updated_at = Set(now);
            active.update(&*self.db).await?;
        }
        Ok(())
    }
}
