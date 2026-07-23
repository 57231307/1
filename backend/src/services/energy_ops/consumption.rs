//! 能耗记录 Service
//!
//! 批次 488 D10-2a 拆分：从原 `energy_service.rs` 迁移 EnergyConsumptionService + 4 个 DTOs/structs。
//!
//! 注意：原文件中的 `DurationGroupKey` 仅被 `EnergyAllocationRecordService` 使用，
//! 因此随 `allocation_record` 子模块迁移，本模块不含该结构。

use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::models::energy_consumption_record::{
    self, ActiveModel as ConsumptionActiveModel, Entity as ConsumptionEntity,
    Model as ConsumptionModel,
};
use crate::models::energy_meter::{
    self, ActiveModel as MeterActiveModel, Entity as MeterEntity,
};
use crate::models::process_route::Entity as RouteEntity;
use crate::models::status::energy_record_status;
use crate::models::status::energy_recording_method;
use crate::utils::error::AppError;

// 复用 facade 的纯函数校验与计算（保持单一来源，避免逻辑重复）
use crate::services::energy_service::{compute_consumption, compute_total_cost, validate_meter_type};

/// 创建能耗记录请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateConsumptionRequest {
    pub meter_id: Option<i32>,
    pub meter_type: String,
    pub workshop: Option<String>,
    pub unit: Option<String>,
    pub previous_reading: Option<Decimal>,
    pub current_reading: Option<Decimal>,
    pub unit_price: Option<Decimal>,
    pub period_start: chrono::DateTime<chrono::FixedOffset>,
    pub period_end: chrono::DateTime<chrono::FixedOffset>,
    pub recording_method: Option<String>,
    pub dye_lot_no: Option<String>,
    pub process_route_id: Option<i32>,
    pub route_code: Option<String>,
    pub equipment_id: Option<i32>,
    pub equipment_name: Option<String>,
    pub operator_id: Option<i32>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 更新能耗记录请求（仅 draft 状态可更新）
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateConsumptionRequest {
    pub previous_reading: Option<Decimal>,
    pub current_reading: Option<Decimal>,
    pub unit_price: Option<Decimal>,
    pub period_start: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub period_end: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub dye_lot_no: Option<String>,
    pub process_route_id: Option<i32>,
    pub route_code: Option<String>,
    pub equipment_id: Option<i32>,
    pub equipment_name: Option<String>,
    pub remarks: Option<String>,
}

/// 能耗记录查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct ConsumptionQuery {
    pub meter_id: Option<i32>,
    pub meter_type: Option<String>,
    pub workshop: Option<String>,
    pub dye_lot_no: Option<String>,
    pub process_route_id: Option<i32>,
    pub equipment_id: Option<i32>,
    pub status: Option<String>,
    pub recording_method: Option<String>,
    pub period_start: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub period_end: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 车间能耗汇总记录（用于 summarize_by_workshop 返回值，避免复杂元组类型）
#[derive(Debug, Clone)]
pub struct WorkshopEnergySummary {
    /// 车间
    pub workshop: String,
    /// 能源类型
    pub meter_type: String,
    /// 总消耗量
    pub total_consumption: Decimal,
    /// 总成本
    pub total_cost: Decimal,
}

/// 能耗记录 Service
pub struct EnergyConsumptionService {
    db: Arc<DatabaseConnection>,
}

impl EnergyConsumptionService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 生成记录编号：EC-YYYYMMDDHHMMSS-NNN
    fn generate_record_no() -> String {
        let now = chrono::Utc::now();
        let timestamp = now.format("%Y%m%d%H%M%S");
        let random = crate::utils::random::random_6_digit() % 1000;
        format!("EC-{}-{:03}", timestamp, random)
    }

    /// 创建能耗记录
    pub async fn create(&self, req: CreateConsumptionRequest) -> Result<ConsumptionModel, AppError> {
        // 校验能源类型
        validate_meter_type(&req.meter_type)?;

        // 校验时段
        if req.period_end <= req.period_start {
            return Err(AppError::business("结束时间必须晚于开始时间"));
        }

        // 校验计量设备存在（若提供）
        if let Some(meter_id) = req.meter_id {
            let _meter = MeterEntity::find_by_id(meter_id)
                .filter(energy_meter::Column::IsDeleted.eq(false))
                .one(&*self.db)
                .await?
                .ok_or_else(|| {
                    AppError::business(format!("计量设备 {} 不存在", meter_id))
                })?;
        }

        // 校验工序路线存在（若提供）
        if let Some(route_id) = req.process_route_id {
            let _route = RouteEntity::find_by_id(route_id)
                .one(&*self.db)
                .await?
                .ok_or_else(|| {
                    AppError::business(format!("工序路线 {} 不存在", route_id))
                })?;
        }

        // 计算消耗量和总成本
        let previous_reading = req.previous_reading.unwrap_or(Decimal::ZERO);
        let current_reading = req.current_reading.unwrap_or(Decimal::ZERO);
        let consumption = compute_consumption(previous_reading, current_reading);
        let unit_price = req.unit_price.unwrap_or(Decimal::ZERO);
        let total_cost = compute_total_cost(consumption, unit_price);

        let recording_method = req
            .recording_method
            .unwrap_or_else(|| energy_recording_method::MANUAL.to_string());
        if recording_method != energy_recording_method::MANUAL
            && recording_method != energy_recording_method::IOT
            && recording_method != energy_recording_method::AUTO_CALC
        {
            return Err(AppError::business(format!(
                "录入方式必须是 manual / iot / auto_calc，当前: {}",
                recording_method
            )));
        }

        let record_no = Self::generate_record_no();
        let now = crate::utils::date_utils::utc_now_fixed();
        let unit = req.unit.unwrap_or_else(|| "度".to_string());

        let active = ConsumptionActiveModel {
            id: Default::default(),
            record_no: Set(record_no),
            meter_id: Set(req.meter_id),
            meter_type: Set(req.meter_type),
            workshop: Set(req.workshop),
            unit: Set(unit),
            previous_reading: Set(previous_reading),
            current_reading: Set(current_reading),
            consumption: Set(consumption),
            unit_price: Set(unit_price),
            total_cost: Set(total_cost),
            period_start: Set(req.period_start),
            period_end: Set(req.period_end),
            recording_method: Set(recording_method),
            dye_lot_no: Set(req.dye_lot_no),
            process_route_id: Set(req.process_route_id),
            route_code: Set(req.route_code),
            equipment_id: Set(req.equipment_id),
            equipment_name: Set(req.equipment_name),
            operator_id: Set(req.operator_id),
            recorded_at: Set(now),
            status: Set(energy_record_status::DRAFT.to_string()),
            remarks: Set(req.remarks),
            is_deleted: Set(false),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("能耗记录创建失败: {}", e)))?;

        // 若关联计量设备，同步更新设备的当前读数和上次读数
        if let Some(meter_id) = req.meter_id {
            if let Some(meter) = MeterEntity::find_by_id(meter_id)
                .filter(energy_meter::Column::IsDeleted.eq(false))
                .one(&*self.db)
                .await?
            {
                let mut meter_active: MeterActiveModel = meter.into();
                meter_active.previous_reading = Set(previous_reading);
                meter_active.current_reading = Set(current_reading);
                meter_active.last_reading_at = Set(Some(now));
                meter_active.updated_at = Set(now);
                meter_active.update(&*self.db).await?;
            }
        }

        Ok(result)
    }

    /// 更新能耗记录（仅 draft 状态可更新）
    pub async fn update(
        &self,
        id: i32,
        req: UpdateConsumptionRequest,
    ) -> Result<ConsumptionModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != energy_record_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可更新，当前状态: {}",
                model.status
            )));
        }

        // 在 model.into() 之前记录原值，避免 ActiveValue 取值复杂
        let original_previous_reading = model.previous_reading;
        let original_current_reading = model.current_reading;
        let original_unit_price = model.unit_price;

        let mut active: ConsumptionActiveModel = model.into();

        // 重新计算消耗量和总成本（若读数变更）
        let previous_reading = req
            .previous_reading
            .unwrap_or(original_previous_reading);
        let current_reading = req
            .current_reading
            .unwrap_or(original_current_reading);
        let unit_price = req
            .unit_price
            .unwrap_or(original_unit_price);

        if req.previous_reading.is_some() || req.current_reading.is_some() {
            let consumption = compute_consumption(previous_reading, current_reading);
            let total_cost = compute_total_cost(consumption, unit_price);
            active.consumption = Set(consumption);
            active.total_cost = Set(total_cost);
        }

        if let Some(v) = req.previous_reading {
            active.previous_reading = Set(v);
        }
        if let Some(v) = req.current_reading {
            active.current_reading = Set(v);
        }
        if let Some(v) = req.unit_price {
            active.unit_price = Set(v);
        }
        if let Some(v) = req.period_start {
            active.period_start = Set(v);
        }
        if let Some(v) = req.period_end {
            active.period_end = Set(v);
        }
        if let Some(v) = req.dye_lot_no {
            active.dye_lot_no = Set(Some(v));
        }
        if let Some(v) = req.process_route_id {
            active.process_route_id = Set(Some(v));
        }
        if let Some(v) = req.route_code {
            active.route_code = Set(Some(v));
        }
        if let Some(v) = req.equipment_id {
            active.equipment_id = Set(Some(v));
        }
        if let Some(v) = req.equipment_name {
            active.equipment_name = Set(Some(v));
        }
        if let Some(v) = req.remarks {
            active.remarks = Set(Some(v));
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 软删除能耗记录（仅 draft 状态可删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != energy_record_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可删除，当前状态: {}",
                model.status
            )));
        }
        let mut active: ConsumptionActiveModel = model.into();
        active.is_deleted = Set(true);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        active.update(&*self.db).await?;
        Ok(())
    }

    /// 确认能耗记录（draft → confirmed）
    pub async fn confirm(&self, id: i32, _confirmed_by: Option<i32>) -> Result<ConsumptionModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != energy_record_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可确认，当前状态: {}",
                model.status
            )));
        }
        let mut active: ConsumptionActiveModel = model.into();
        active.status = Set(energy_record_status::CONFIRMED.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 取消能耗记录（draft/confirmed → cancelled）
    pub async fn cancel(&self, id: i32) -> Result<ConsumptionModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status == energy_record_status::CANCELLED {
            return Err(AppError::business("记录已取消，不能重复取消"));
        }
        let mut active: ConsumptionActiveModel = model.into();
        active.status = Set(energy_record_status::CANCELLED.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<ConsumptionModel, AppError> {
        ConsumptionEntity::find_by_id(id)
            .filter(energy_consumption_record::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("能耗记录 {} 不存在", id)))
    }

    /// 按编号查询
    pub async fn get_by_no(&self, record_no: &str) -> Result<ConsumptionModel, AppError> {
        ConsumptionEntity::find()
            .filter(energy_consumption_record::Column::RecordNo.eq(record_no))
            .filter(energy_consumption_record::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("能耗记录编号 {} 不存在", record_no)))
    }

    /// 分页查询
    pub async fn list(
        &self,
        query: ConsumptionQuery,
    ) -> Result<(Vec<ConsumptionModel>, u64), AppError> {
        let mut q = ConsumptionEntity::find()
            .filter(energy_consumption_record::Column::IsDeleted.eq(false));
        if let Some(v) = query.meter_id {
            q = q.filter(energy_consumption_record::Column::MeterId.eq(v));
        }
        if let Some(v) = query.meter_type {
            q = q.filter(energy_consumption_record::Column::MeterType.eq(v));
        }
        if let Some(v) = query.workshop {
            q = q.filter(energy_consumption_record::Column::Workshop.eq(v));
        }
        if let Some(v) = query.dye_lot_no {
            q = q.filter(energy_consumption_record::Column::DyeLotNo.eq(v));
        }
        if let Some(v) = query.process_route_id {
            q = q.filter(energy_consumption_record::Column::ProcessRouteId.eq(v));
        }
        if let Some(v) = query.equipment_id {
            q = q.filter(energy_consumption_record::Column::EquipmentId.eq(v));
        }
        if let Some(v) = query.status {
            q = q.filter(energy_consumption_record::Column::Status.eq(v));
        }
        if let Some(v) = query.recording_method {
            q = q.filter(energy_consumption_record::Column::RecordingMethod.eq(v));
        }
        if let Some(v) = query.period_start {
            q = q.filter(energy_consumption_record::Column::PeriodStart.gte(v));
        }
        if let Some(v) = query.period_end {
            q = q.filter(energy_consumption_record::Column::PeriodEnd.lte(v));
        }

        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

        let total = q.clone().count(&*self.db).await?;
        let items = q
            .order_by_desc(energy_consumption_record::Column::Id)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;
        Ok((items, total))
    }

    /// 按时段汇总车间总能耗（用于月末分摊）
    pub async fn summarize_by_workshop(
        &self,
        period_start: chrono::DateTime<chrono::FixedOffset>,
        period_end: chrono::DateTime<chrono::FixedOffset>,
        workshop: Option<String>,
        meter_type: Option<String>,
    ) -> Result<Vec<WorkshopEnergySummary>, AppError> {
        let mut q = ConsumptionEntity::find()
            .filter(energy_consumption_record::Column::IsDeleted.eq(false))
            .filter(energy_consumption_record::Column::Status.eq(energy_record_status::CONFIRMED))
            .filter(energy_consumption_record::Column::PeriodStart.gte(period_start))
            .filter(energy_consumption_record::Column::PeriodEnd.lte(period_end));
        if let Some(v) = workshop.clone() {
            q = q.filter(energy_consumption_record::Column::Workshop.eq(v));
        }
        if let Some(v) = meter_type.clone() {
            q = q.filter(energy_consumption_record::Column::MeterType.eq(v));
        }

        let records = q.all(&*self.db).await?;

        // 按车间 + 能源类型汇总（用元组 key + 累加器结构，避免复杂嵌套元组类型）
        let mut summary: std::collections::HashMap<(String, String), (Decimal, Decimal)> =
            std::collections::HashMap::new();
        for r in records {
            let ws = r.workshop.unwrap_or_else(|| "未分配".to_string());
            let key = (ws, r.meter_type);
            let entry = summary.entry(key).or_insert((Decimal::ZERO, Decimal::ZERO));
            entry.0 += r.consumption;
            entry.1 += r.total_cost;
        }

        Ok(summary
            .into_iter()
            .map(|((ws, mt), (cons, cost))| WorkshopEnergySummary {
                workshop: ws,
                meter_type: mt,
                total_consumption: cons,
                total_cost: cost,
            })
            .collect())
    }
}
