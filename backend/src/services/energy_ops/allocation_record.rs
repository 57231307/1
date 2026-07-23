//! 能耗分摊记录 Service
//!
//! 批次 488 D10-2a 拆分：从原 `energy_service.rs` 迁移 EnergyAllocationRecordService + 4 个 DTOs。
//!
//! 本模块包含私有结构 `DurationGroupKey`（原文件中定义于 EnergyConsumptionService 之前，
//! 但仅被本 Service 的 `monthly_allocation_by_duration` / `group_step_duration_by_key` /
//! `build_allocation_record` 使用，故随本子模块迁移）。
//!
//! 跨 service 协作：`monthly_allocation_by_duration` 通过参数注入
//! `EnergyConsumptionService` 与 `EnergyAllocationRuleService`，分别用于汇总总能耗
//! 与查询生效规则。

use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::models::energy_allocation_record::{
    self, ActiveModel as AllocationRecordActiveModel, Entity as AllocationRecordEntity,
    Model as AllocationRecordModel,
};
use crate::models::energy_allocation_rule::Model as RuleModel;
use crate::models::process_step_record::{self, Entity as StepEntity};
use crate::models::status::energy_allocation_basis;
use crate::models::status::energy_record_status;
use crate::utils::error::AppError;

// 复用 facade 的纯函数校验与计算（保持单一来源，避免逻辑重复）
use crate::services::energy_service::{
    compute_allocated_consumption, compute_allocated_cost, compute_allocation_ratio,
    compute_unit_consumption, validate_allocation_basis, validate_meter_type,
};

// 跨 service 协作：月末分摊方法参数引用兄弟 service 类型
use crate::services::energy_ops::{
    EnergyAllocationRuleService, EnergyConsumptionService,
};

/// 创建分摊记录请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateAllocationRecordRequest {
    pub period_start: chrono::DateTime<chrono::FixedOffset>,
    pub period_end: chrono::DateTime<chrono::FixedOffset>,
    pub meter_type: String,
    pub workshop: Option<String>,
    pub allocation_rule_id: Option<i32>,
    pub allocation_basis: String,
    pub total_consumption: Decimal,
    pub total_cost: Decimal,
    pub dye_lot_no: Option<String>,
    pub production_order_id: Option<i32>,
    pub production_order_no: Option<String>,
    pub process_route_id: Option<i32>,
    pub route_code: Option<String>,
    pub flow_card_id: Option<i32>,
    pub allocation_basis_value: Decimal,
    pub allocation_ratio: Option<Decimal>,
    pub allocated_consumption: Option<Decimal>,
    pub allocated_cost: Option<Decimal>,
    pub output_quantity: Option<Decimal>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 更新分摊记录请求（仅 draft 状态可更新）
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateAllocationRecordRequest {
    pub total_consumption: Option<Decimal>,
    pub total_cost: Option<Decimal>,
    pub allocation_basis_value: Option<Decimal>,
    pub allocation_ratio: Option<Decimal>,
    pub allocated_consumption: Option<Decimal>,
    pub allocated_cost: Option<Decimal>,
    pub output_quantity: Option<Decimal>,
    pub remarks: Option<String>,
}

/// 分摊记录查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct AllocationRecordQuery {
    pub meter_type: Option<String>,
    pub workshop: Option<String>,
    pub dye_lot_no: Option<String>,
    pub production_order_id: Option<i32>,
    pub process_route_id: Option<i32>,
    pub allocation_rule_id: Option<i32>,
    pub status: Option<String>,
    pub period_start: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub period_end: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 月末分摊请求
#[derive(Debug, Clone, Deserialize)]
pub struct MonthlyAllocationRequest {
    pub period_start: chrono::DateTime<chrono::FixedOffset>,
    pub period_end: chrono::DateTime<chrono::FixedOffset>,
    pub workshop: Option<String>,
    pub meter_type: Option<String>,
    pub created_by: Option<i32>,
}

/// 工时分组键（用于 monthly_allocation_by_duration 内部分组，避免复杂元组类型）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct DurationGroupKey {
    /// 缸号（简化：暂用 equipment_name）
    dye_lot_no: Option<String>,
    /// 工序路线 ID
    route_id: Option<i32>,
    /// 工序编码
    route_code: Option<String>,
}

/// 能耗分摊记录 Service
pub struct EnergyAllocationRecordService {
    db: Arc<DatabaseConnection>,
}

impl EnergyAllocationRecordService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 生成分摊编号：EAR-YYYYMMDDHHMMSS-NNN
    fn generate_allocation_no() -> String {
        let now = chrono::Utc::now();
        let timestamp = now.format("%Y%m%d%H%M%S");
        let random = crate::utils::random::random_6_digit() % 1000;
        format!("EAR-{}-{:03}", timestamp, random)
    }

    /// 创建分摊记录
    pub async fn create(
        &self,
        req: CreateAllocationRecordRequest,
    ) -> Result<AllocationRecordModel, AppError> {
        validate_meter_type(&req.meter_type)?;
        validate_allocation_basis(&req.allocation_basis)?;

        if req.period_end <= req.period_start {
            return Err(AppError::business("结束时间必须晚于开始时间"));
        }

        // 计算分摊比例、消耗量、成本（若未提供）
        let allocation_ratio = req.allocation_ratio.unwrap_or(Decimal::ONE);
        let allocated_consumption = req
            .allocated_consumption
            .unwrap_or_else(|| compute_allocated_consumption(req.total_consumption, allocation_ratio));
        let allocated_cost = req
            .allocated_cost
            .unwrap_or_else(|| compute_allocated_cost(req.total_cost, allocation_ratio));
        let unit_consumption =
            compute_unit_consumption(allocated_consumption, req.output_quantity);

        let allocation_no = Self::generate_allocation_no();
        let now = crate::utils::date_utils::utc_now_fixed();

        let active = AllocationRecordActiveModel {
            id: Default::default(),
            allocation_no: Set(allocation_no),
            period_start: Set(req.period_start),
            period_end: Set(req.period_end),
            meter_type: Set(req.meter_type),
            workshop: Set(req.workshop),
            allocation_rule_id: Set(req.allocation_rule_id),
            allocation_basis: Set(req.allocation_basis),
            total_consumption: Set(req.total_consumption),
            total_cost: Set(req.total_cost),
            dye_lot_no: Set(req.dye_lot_no),
            production_order_id: Set(req.production_order_id),
            production_order_no: Set(req.production_order_no),
            process_route_id: Set(req.process_route_id),
            route_code: Set(req.route_code),
            flow_card_id: Set(req.flow_card_id),
            allocation_basis_value: Set(req.allocation_basis_value),
            allocation_ratio: Set(allocation_ratio),
            allocated_consumption: Set(allocated_consumption),
            allocated_cost: Set(allocated_cost),
            output_quantity: Set(req.output_quantity),
            unit_consumption: Set(unit_consumption),
            cost_collection_id: Set(None),
            status: Set(energy_record_status::DRAFT.to_string()),
            confirmed_by: Set(None),
            confirmed_at: Set(None),
            remarks: Set(req.remarks),
            is_deleted: Set(false),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("分摊记录创建失败: {}", e)))?;
        Ok(result)
    }

    /// 更新分摊记录（仅 draft 状态可更新）
    pub async fn update(
        &self,
        id: i32,
        req: UpdateAllocationRecordRequest,
    ) -> Result<AllocationRecordModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != energy_record_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可更新，当前状态: {}",
                model.status
            )));
        }

        // 记录原值（ActiveValue 不支持 unwrap_or_default，需在 model.into() 前保存）
        let original_allocated_consumption = model.allocated_consumption;

        let mut active: AllocationRecordActiveModel = model.into();

        if let Some(v) = req.total_consumption {
            active.total_consumption = Set(v);
        }
        if let Some(v) = req.total_cost {
            active.total_cost = Set(v);
        }
        if let Some(v) = req.allocation_basis_value {
            active.allocation_basis_value = Set(v);
        }
        if let Some(v) = req.allocation_ratio {
            active.allocation_ratio = Set(v);
        }
        if let Some(v) = req.allocated_consumption {
            active.allocated_consumption = Set(v);
        }
        if let Some(v) = req.allocated_cost {
            active.allocated_cost = Set(v);
        }
        if let Some(v) = req.output_quantity {
            active.output_quantity = Set(Some(v));
            // 重新计算单位能耗（用 req 或原值，避免 ActiveValue unwrap）
            let allocated = req
                .allocated_consumption
                .unwrap_or(original_allocated_consumption);
            let unit = compute_unit_consumption(allocated, Some(v));
            active.unit_consumption = Set(unit);
        }
        if let Some(v) = req.remarks {
            active.remarks = Set(Some(v));
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 软删除分摊记录（仅 draft 状态可删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != energy_record_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可删除，当前状态: {}",
                model.status
            )));
        }
        let mut active: AllocationRecordActiveModel = model.into();
        active.is_deleted = Set(true);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        active.update(&*self.db).await?;
        Ok(())
    }

    /// 确认分摊记录（draft → confirmed）
    pub async fn confirm(
        &self,
        id: i32,
        confirmed_by: Option<i32>,
    ) -> Result<AllocationRecordModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != energy_record_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可确认，当前状态: {}",
                model.status
            )));
        }
        let now = crate::utils::date_utils::utc_now_fixed();
        let mut active: AllocationRecordActiveModel = model.into();
        active.status = Set(energy_record_status::CONFIRMED.to_string());
        active.confirmed_by = Set(confirmed_by);
        active.confirmed_at = Set(Some(now));
        active.updated_at = Set(now);
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 取消分摊记录（draft/confirmed → cancelled）
    pub async fn cancel(&self, id: i32) -> Result<AllocationRecordModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status == energy_record_status::CANCELLED {
            return Err(AppError::business("记录已取消，不能重复取消"));
        }
        let mut active: AllocationRecordActiveModel = model.into();
        active.status = Set(energy_record_status::CANCELLED.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<AllocationRecordModel, AppError> {
        AllocationRecordEntity::find_by_id(id)
            .filter(energy_allocation_record::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("分摊记录 {} 不存在", id)))
    }

    /// 按编号查询
    pub async fn get_by_no(
        &self,
        allocation_no: &str,
    ) -> Result<AllocationRecordModel, AppError> {
        AllocationRecordEntity::find()
            .filter(energy_allocation_record::Column::AllocationNo.eq(allocation_no))
            .filter(energy_allocation_record::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("分摊记录编号 {} 不存在", allocation_no)))
    }

    /// 分页查询
    pub async fn list(
        &self,
        query: AllocationRecordQuery,
    ) -> Result<(Vec<AllocationRecordModel>, u64), AppError> {
        let mut q = AllocationRecordEntity::find()
            .filter(energy_allocation_record::Column::IsDeleted.eq(false));
        if let Some(v) = query.meter_type {
            q = q.filter(energy_allocation_record::Column::MeterType.eq(v));
        }
        if let Some(v) = query.workshop {
            q = q.filter(energy_allocation_record::Column::Workshop.eq(v));
        }
        if let Some(v) = query.dye_lot_no {
            q = q.filter(energy_allocation_record::Column::DyeLotNo.eq(v));
        }
        if let Some(v) = query.production_order_id {
            q = q.filter(energy_allocation_record::Column::ProductionOrderId.eq(v));
        }
        if let Some(v) = query.process_route_id {
            q = q.filter(energy_allocation_record::Column::ProcessRouteId.eq(v));
        }
        if let Some(v) = query.allocation_rule_id {
            q = q.filter(energy_allocation_record::Column::AllocationRuleId.eq(v));
        }
        if let Some(v) = query.status {
            q = q.filter(energy_allocation_record::Column::Status.eq(v));
        }
        if let Some(v) = query.period_start {
            q = q.filter(energy_allocation_record::Column::PeriodStart.gte(v));
        }
        if let Some(v) = query.period_end {
            q = q.filter(energy_allocation_record::Column::PeriodEnd.lte(v));
        }

        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

        let total = q.clone().count(&*self.db).await?;
        let items = q
            .order_by_desc(energy_allocation_record::Column::Id)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;
        Ok((items, total))
    }

    /// 月末按工时自动分摊
    ///
    /// 业务流程：
    /// 1. 查询时段内已确认的能耗记录，按车间+能源类型汇总
    /// 2. 查询工序记录按缸号+工序分组统计工时
    /// 3. 按工时比例分摊总能耗，生成分摊记录
    pub async fn monthly_allocation_by_duration(
        &self,
        req: MonthlyAllocationRequest,
        consumption_service: &EnergyConsumptionService,
        rule_service: &EnergyAllocationRuleService,
    ) -> Result<Vec<AllocationRecordModel>, AppError> {
        // 1. 汇总各车间+能源类型的总能耗
        let summaries = consumption_service
            .summarize_by_workshop(
                req.period_start,
                req.period_end,
                req.workshop.clone(),
                req.meter_type.clone(),
            )
            .await?;

        if summaries.is_empty() {
            return Ok(Vec::new());
        }

        let mut results = Vec::new();

        for summary in summaries {
            let workshop = summary.workshop;
            let meter_type = summary.meter_type;
            let total_consumption = summary.total_consumption;
            let total_cost = summary.total_cost;

            // 2. 按缸号+工序分组统计工时
            let grouped_duration = Self::group_step_duration_by_key(
                &self.db, req.period_start, req.period_end,
            )
            .await?;

            let total_duration: i32 = grouped_duration.values().sum();
            if total_duration == 0 {
                continue;
            }

            let total_duration_decimal = Decimal::from(total_duration);

            // 3. 按工时比例分摊
            for (key, duration) in grouped_duration {
                // 查询生效规则
                let rule = if let Some(rid) = key.route_id {
                    rule_service
                        .get_effective_rule(
                            &workshop,
                            &meter_type,
                            Some(rid),
                            req.period_start.date_naive(),
                        )
                        .await?
                } else {
                    None
                };

                let active = Self::build_allocation_record(
                    &req, &workshop, &meter_type,
                    total_consumption, total_cost,
                    &key, duration, total_duration_decimal, &rule,
                );

                let result = active
                    .insert(&*self.db)
                    .await
                    .map_err(|e| AppError::database(format!("分摊记录创建失败: {}", e)))?;
                results.push(result);
            }
        }

        Ok(results)
    }

    /// 按缸号+工序分组统计工时
    async fn group_step_duration_by_key(
        db: &DatabaseConnection,
        period_start: chrono::DateTime<chrono::FixedOffset>,
        period_end: chrono::DateTime<chrono::FixedOffset>,
    ) -> Result<std::collections::HashMap<DurationGroupKey, i32>, AppError> {
        let step_records = StepEntity::find()
            .filter(process_step_record::Column::Status.eq("completed"))
            .filter(process_step_record::Column::IsDeleted.eq(false))
            .filter(process_step_record::Column::StartAt.gte(period_start))
            .filter(process_step_record::Column::StartAt.lte(period_end))
            .all(db)
            .await?;

        let mut grouped_duration: std::collections::HashMap<DurationGroupKey, i32> =
            std::collections::HashMap::new();

        for step in step_records {
            // 缸号通过 flow_card 关联查询（简化：暂用 equipment_name 作为车间归属）
            let dye_lot_no = step.equipment_name.clone();
            let route_id = step.process_route_id;
            let route_code = Some(step.route_code.clone());
            let key = DurationGroupKey {
                dye_lot_no,
                route_id,
                route_code,
            };
            let duration = step.duration_minutes.unwrap_or(0);
            *grouped_duration.entry(key).or_insert(0) += duration;
        }

        Ok(grouped_duration)
    }

    /// 构建分摊记录 ActiveModel
    fn build_allocation_record(
        req: &MonthlyAllocationRequest,
        workshop: &str,
        meter_type: &str,
        total_consumption: Decimal,
        total_cost: Decimal,
        key: &DurationGroupKey,
        duration: i32,
        total_duration_decimal: Decimal,
        rule: &Option<RuleModel>,
    ) -> AllocationRecordActiveModel {
        let basis_value = Decimal::from(duration);
        let ratio = compute_allocation_ratio(basis_value, total_duration_decimal);
        let allocated_consumption = compute_allocated_consumption(total_consumption, ratio);
        let allocated_cost = compute_allocated_cost(total_cost, ratio);

        let allocation_no = Self::generate_allocation_no();
        let now = crate::utils::date_utils::utc_now_fixed();

        AllocationRecordActiveModel {
            id: Default::default(),
            allocation_no: Set(allocation_no),
            period_start: Set(req.period_start),
            period_end: Set(req.period_end),
            meter_type: Set(meter_type.to_string()),
            workshop: Set(Some(workshop.to_string())),
            allocation_rule_id: Set(rule.as_ref().map(|r| r.id)),
            allocation_basis: Set(energy_allocation_basis::BY_DURATION.to_string()),
            total_consumption: Set(total_consumption),
            total_cost: Set(total_cost),
            dye_lot_no: Set(key.dye_lot_no.clone()),
            production_order_id: Set(None),
            production_order_no: Set(None),
            process_route_id: Set(key.route_id),
            route_code: Set(key.route_code.clone()),
            flow_card_id: Set(None),
            allocation_basis_value: Set(basis_value),
            allocation_ratio: Set(ratio),
            allocated_consumption: Set(allocated_consumption),
            allocated_cost: Set(allocated_cost),
            output_quantity: Set(None),
            unit_consumption: Set(None),
            cost_collection_id: Set(None),
            status: Set(energy_record_status::DRAFT.to_string()),
            confirmed_by: Set(None),
            confirmed_at: Set(None),
            remarks: Set(Some("月末按工时自动分摊".to_string())),
            is_deleted: Set(false),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        }
    }
}
