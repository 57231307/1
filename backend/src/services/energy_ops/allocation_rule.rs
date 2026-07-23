//! 能耗分摊规则 Service
//!
//! 批次 488 D10-2a 拆分：从原 `energy_service.rs` 迁移 EnergyAllocationRuleService + 3 个 DTOs。

use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::models::energy_allocation_rule::{
    self, ActiveModel as RuleActiveModel, Entity as RuleEntity, Model as RuleModel,
};
use crate::models::process_route::Entity as RouteEntity;
use crate::models::status::energy_rule_status;
use crate::utils::error::AppError;

// 复用 facade 的纯函数校验（保持单一来源，避免逻辑重复）
use crate::services::energy_service::{validate_allocation_basis, validate_meter_type};

/// 创建分摊规则请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateRuleRequest {
    pub rule_name: String,
    pub meter_type: String,
    pub allocation_basis: String,
    pub workshop: Option<String>,
    pub process_route_id: Option<i32>,
    pub route_code: Option<String>,
    pub effective_date: chrono::NaiveDate,
    pub expiry_date: Option<chrono::NaiveDate>,
    pub standard_consumption_per_unit: Option<Decimal>,
    pub standard_unit: Option<String>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 更新分摊规则请求（仅 draft 状态可更新）
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateRuleRequest {
    pub rule_name: Option<String>,
    pub allocation_basis: Option<String>,
    pub workshop: Option<String>,
    pub process_route_id: Option<i32>,
    pub route_code: Option<String>,
    pub effective_date: Option<chrono::NaiveDate>,
    pub expiry_date: Option<chrono::NaiveDate>,
    pub standard_consumption_per_unit: Option<Decimal>,
    pub standard_unit: Option<String>,
    pub remarks: Option<String>,
}

/// 分摊规则查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct RuleQuery {
    pub meter_type: Option<String>,
    pub workshop: Option<String>,
    pub process_route_id: Option<i32>,
    pub allocation_basis: Option<String>,
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 能耗分摊规则 Service
pub struct EnergyAllocationRuleService {
    db: Arc<DatabaseConnection>,
}

impl EnergyAllocationRuleService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 生成规则编号：EAR-YYYYMMDDHHMMSS-NNN
    fn generate_rule_no() -> String {
        let now = chrono::Utc::now();
        let timestamp = now.format("%Y%m%d%H%M%S");
        let random = crate::utils::random::random_6_digit() % 1000;
        format!("EAR-{}-{:03}", timestamp, random)
    }

    /// 创建分摊规则
    pub async fn create(&self, req: CreateRuleRequest) -> Result<RuleModel, AppError> {
        validate_meter_type(&req.meter_type)?;
        validate_allocation_basis(&req.allocation_basis)?;

        // 校验工序路线存在（若提供）
        if let Some(route_id) = req.process_route_id {
            let _route = RouteEntity::find_by_id(route_id)
                .one(&*self.db)
                .await?
                .ok_or_else(|| {
                    AppError::business(format!("工序路线 {} 不存在", route_id))
                })?;
        }

        // 校验失效日期
        if let Some(expiry) = req.expiry_date {
            if expiry <= req.effective_date {
                return Err(AppError::business("失效日期必须晚于生效日期"));
            }
        }

        // 校验标准消耗非负
        let standard_consumption = req.standard_consumption_per_unit.unwrap_or(Decimal::ZERO);
        if standard_consumption < Decimal::ZERO {
            return Err(AppError::business("标准单位能耗不能为负"));
        }

        let rule_no = Self::generate_rule_no();
        let now = crate::utils::date_utils::utc_now_fixed();

        let active = RuleActiveModel {
            id: Default::default(),
            rule_no: Set(rule_no),
            rule_name: Set(req.rule_name),
            meter_type: Set(req.meter_type),
            allocation_basis: Set(req.allocation_basis),
            workshop: Set(req.workshop),
            process_route_id: Set(req.process_route_id),
            route_code: Set(req.route_code),
            effective_date: Set(req.effective_date),
            expiry_date: Set(req.expiry_date),
            standard_consumption_per_unit: Set(standard_consumption),
            standard_unit: Set(req.standard_unit),
            status: Set(energy_rule_status::DRAFT.to_string()),
            remarks: Set(req.remarks),
            is_deleted: Set(false),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("分摊规则创建失败: {}", e)))?;
        Ok(result)
    }

    /// 更新分摊规则（仅 draft 状态可更新）
    pub async fn update(
        &self,
        id: i32,
        req: UpdateRuleRequest,
    ) -> Result<RuleModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != energy_rule_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可更新，当前状态: {}",
                model.status
            )));
        }

        let original_effective_date = model.effective_date;
        let mut active: RuleActiveModel = model.into();

        if let Some(v) = req.rule_name {
            active.rule_name = Set(v);
        }
        if let Some(v) = req.allocation_basis {
            validate_allocation_basis(&v)?;
            active.allocation_basis = Set(v);
        }
        if let Some(v) = req.workshop {
            active.workshop = Set(Some(v));
        }
        if let Some(v) = req.process_route_id {
            active.process_route_id = Set(Some(v));
        }
        if let Some(v) = req.route_code {
            active.route_code = Set(Some(v));
        }
        if let Some(v) = req.effective_date {
            active.effective_date = Set(v);
        }
        if let Some(v) = req.expiry_date {
            let effective = req.effective_date.unwrap_or(original_effective_date);
            if v <= effective {
                return Err(AppError::business("失效日期必须晚于生效日期"));
            }
            active.expiry_date = Set(Some(v));
        }
        if let Some(v) = req.standard_consumption_per_unit {
            if v < Decimal::ZERO {
                return Err(AppError::business("标准单位能耗不能为负"));
            }
            active.standard_consumption_per_unit = Set(v);
        }
        if let Some(v) = req.standard_unit {
            active.standard_unit = Set(Some(v));
        }
        if let Some(v) = req.remarks {
            active.remarks = Set(Some(v));
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 软删除分摊规则（仅 draft 状态可删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != energy_rule_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可删除，当前状态: {}",
                model.status
            )));
        }
        let mut active: RuleActiveModel = model.into();
        active.is_deleted = Set(true);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        active.update(&*self.db).await?;
        Ok(())
    }

    /// 启用规则（draft → active）
    pub async fn activate(&self, id: i32) -> Result<RuleModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != energy_rule_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可启用，当前状态: {}",
                model.status
            )));
        }
        let mut active: RuleActiveModel = model.into();
        active.status = Set(energy_rule_status::ACTIVE.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 停用规则（active → disabled）
    pub async fn disable(&self, id: i32) -> Result<RuleModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != energy_rule_status::ACTIVE {
            return Err(AppError::business(format!(
                "仅启用(active)状态可停用，当前状态: {}",
                model.status
            )));
        }
        let mut active: RuleActiveModel = model.into();
        active.status = Set(energy_rule_status::DISABLED.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<RuleModel, AppError> {
        RuleEntity::find_by_id(id)
            .filter(energy_allocation_rule::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("分摊规则 {} 不存在", id)))
    }

    /// 按编号查询
    pub async fn get_by_no(&self, rule_no: &str) -> Result<RuleModel, AppError> {
        RuleEntity::find()
            .filter(energy_allocation_rule::Column::RuleNo.eq(rule_no))
            .filter(energy_allocation_rule::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("分摊规则编号 {} 不存在", rule_no)))
    }

    /// 分页查询
    pub async fn list(&self, query: RuleQuery) -> Result<(Vec<RuleModel>, u64), AppError> {
        let mut q = RuleEntity::find()
            .filter(energy_allocation_rule::Column::IsDeleted.eq(false));
        if let Some(v) = query.meter_type {
            q = q.filter(energy_allocation_rule::Column::MeterType.eq(v));
        }
        if let Some(v) = query.workshop {
            q = q.filter(energy_allocation_rule::Column::Workshop.eq(v));
        }
        if let Some(v) = query.process_route_id {
            q = q.filter(energy_allocation_rule::Column::ProcessRouteId.eq(v));
        }
        if let Some(v) = query.allocation_basis {
            q = q.filter(energy_allocation_rule::Column::AllocationBasis.eq(v));
        }
        if let Some(v) = query.status {
            q = q.filter(energy_allocation_rule::Column::Status.eq(v));
        }

        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

        let total = q.clone().count(&*self.db).await?;
        let items = q
            .order_by_desc(energy_allocation_rule::Column::Id)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;
        Ok((items, total))
    }

    /// 按车间+能源类型+工序查询生效规则
    pub async fn get_effective_rule(
        &self,
        workshop: &str,
        meter_type: &str,
        process_route_id: Option<i32>,
        date: chrono::NaiveDate,
    ) -> Result<Option<RuleModel>, AppError> {
        let mut q = RuleEntity::find()
            .filter(energy_allocation_rule::Column::IsDeleted.eq(false))
            .filter(energy_allocation_rule::Column::Status.eq(energy_rule_status::ACTIVE))
            .filter(energy_allocation_rule::Column::Workshop.eq(workshop))
            .filter(energy_allocation_rule::Column::MeterType.eq(meter_type))
            .filter(energy_allocation_rule::Column::EffectiveDate.lte(date));
        if let Some(rid) = process_route_id {
            q = q.filter(energy_allocation_rule::Column::ProcessRouteId.eq(rid));
        }
        let rule = q
            .order_by_desc(energy_allocation_rule::Column::EffectiveDate)
            .one(&*self.db)
            .await?;
        // 校验失效日期
        if let Some(r) = &rule {
            if let Some(expiry) = r.expiry_date {
                if date > expiry {
                    return Ok(None);
                }
            }
        }
        Ok(rule)
    }
}
