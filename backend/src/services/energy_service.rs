//! 能耗管理 Service
//!
//! v14 批次 428：能耗管理贯通
//! 依据：面料行业真实业务调研文档 §12.6 能耗管理
//! 真实业务流程：
//!   能耗采集 → 能源计量设备（水/电/汽表，IoT 对接）→ 时间段登记能耗记录
//!   分摊规则 → 定义分摊基准（按工时/产量/设备/车间）+ 标准单位能耗（用于超基准预警）
//!   月末分摊 → 按规则将总能耗分摊到缸号/工序/订单 → 生成 cost_collection 记录
//!   单位能耗分析 → 每米布能耗/每缸能耗/能耗产值分析
//!
//! 核心能力：
//! - 能源计量设备 CRUD + 状态机（active→inactive/maintenance）
//! - 能耗记录 CRUD + 状态机（draft→confirmed→cancelled）+ IoT 自动采集
//! - 能耗分摊规则 CRUD + 状态机（draft→active→disabled）
//! - 能耗分摊记录 CRUD + 状态机 + 月末分摊计算 + 关联 cost_collection
//!
//! 复用现有功能（§10.0.1）：
//! - process_step_record 表：作为工时/产量数据源（批次 425 已建）
//! - process_route 表：作为工序定义（批次 425 已建）
//! - cost_collection 表：作为成本归集目标（批次 422 已建）
//! - production_flow_card 表：作为缸号/流转卡关联（批次 425 已建）

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
use crate::models::energy_allocation_rule::{
    self, ActiveModel as RuleActiveModel, Entity as RuleEntity, Model as RuleModel,
};
use crate::models::energy_consumption_record::{
    self, ActiveModel as ConsumptionActiveModel, Entity as ConsumptionEntity,
    Model as ConsumptionModel,
};
use crate::models::energy_meter::{
    self, ActiveModel as MeterActiveModel, Entity as MeterEntity, Model as MeterModel,
};
use crate::models::process_route::Entity as RouteEntity;
use crate::models::process_step_record::{self, Entity as StepEntity};
use crate::models::status::energy_allocation_basis;
use crate::models::status::energy_meter_status;
use crate::models::status::energy_record_status;
use crate::models::status::energy_recording_method;
use crate::models::status::energy_rule_status;
use crate::models::status::energy_type;
use crate::utils::error::AppError;

// ============================================================================
// 能耗计算纯函数
// ============================================================================

/// 计算消耗量（当前读数 - 上次读数）
///
/// 业务规则：
/// - 若当前读数 < 上次读数，返回 0（可能是表计回零或异常）
/// - 否则返回差值
pub fn compute_consumption(
    previous_reading: Decimal,
    current_reading: Decimal,
) -> Decimal {
    if current_reading < previous_reading {
        return Decimal::ZERO;
    }
    current_reading - previous_reading
}

/// 计算总成本（消耗量 × 单价）
pub fn compute_total_cost(consumption: Decimal, unit_price: Decimal) -> Decimal {
    consumption * unit_price
}

/// 计算分摊比例（分摊依据量 / 总依据量）
///
/// 业务规则：
/// - 若总依据量为 0，返回 0（避免除零）
pub fn compute_allocation_ratio(
    basis_value: Decimal,
    total_basis_value: Decimal,
) -> Decimal {
    if total_basis_value <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    basis_value / total_basis_value
}

/// 计算分摊消耗量（总消耗量 × 分摊比例）
pub fn compute_allocated_consumption(
    total_consumption: Decimal,
    allocation_ratio: Decimal,
) -> Decimal {
    total_consumption * allocation_ratio
}

/// 计算分摊成本（总成本 × 分摊比例）
pub fn compute_allocated_cost(
    total_cost: Decimal,
    allocation_ratio: Decimal,
) -> Decimal {
    total_cost * allocation_ratio
}

/// 计算单位能耗（分摊消耗量 / 单位产量）
///
/// 业务规则：
/// - 若单位产量为 0 或 None，返回 None
/// - 用于单位能耗分析（每米布能耗、每缸能耗）
pub fn compute_unit_consumption(
    allocated_consumption: Decimal,
    output_quantity: Option<Decimal>,
) -> Option<Decimal> {
    let output = output_quantity?;
    if output <= Decimal::ZERO {
        return None;
    }
    Some(allocated_consumption / output)
}

/// 判断能耗是否超过基准
///
/// 业务规则：
/// - 实际单位能耗 > 标准单位能耗 × (1 + tolerance) 时视为超基准
/// - tolerance 默认 0.1（10% 容差）
/// - 返回 (是否超基准, 实际单位能耗, 偏差百分比)
pub fn check_consumption_exceeds_standard(
    actual_unit_consumption: Decimal,
    standard_consumption_per_unit: Decimal,
    tolerance: Decimal,
) -> (bool, Decimal) {
    if standard_consumption_per_unit <= Decimal::ZERO {
        return (false, Decimal::ZERO);
    }
    let threshold = standard_consumption_per_unit * (Decimal::ONE + tolerance);
    let deviation = if actual_unit_consumption > standard_consumption_per_unit {
        (actual_unit_consumption - standard_consumption_per_unit)
            / standard_consumption_per_unit
            * Decimal::new(100, 0)
    } else {
        Decimal::ZERO
    };
    (actual_unit_consumption > threshold, deviation)
}

/// 校验能源类型是否合法
pub fn validate_meter_type(meter_type: &str) -> Result<(), AppError> {
    let valid_types = [
        energy_type::WATER,
        energy_type::ELECTRICITY,
        energy_type::STEAM,
        energy_type::GAS,
        energy_type::COMPRESSED_AIR,
    ];
    if !valid_types.contains(&meter_type) {
        return Err(AppError::business(format!(
            "能源类型必须是 water / electricity / steam / gas / compressed_air，当前: {}",
            meter_type
        )));
    }
    Ok(())
}

/// 校验分摊基准是否合法
pub fn validate_allocation_basis(basis: &str) -> Result<(), AppError> {
    let valid_basis = [
        energy_allocation_basis::BY_DURATION,
        energy_allocation_basis::BY_OUTPUT,
        energy_allocation_basis::BY_EQUIPMENT,
        energy_allocation_basis::BY_WORKSHOP,
    ];
    if !valid_basis.contains(&basis) {
        return Err(AppError::business(format!(
            "分摊基准必须是 by_duration / by_output / by_equipment / by_workshop，当前: {}",
            basis
        )));
    }
    Ok(())
}

// ============================================================================
// 能源计量设备 Service
// ============================================================================

/// 创建计量设备请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateMeterRequest {
    pub meter_name: String,
    pub meter_type: String,
    pub workshop: Option<String>,
    pub equipment_id: Option<i32>,
    pub equipment_name: Option<String>,
    pub location: Option<String>,
    pub iot_device_id: Option<String>,
    pub unit: Option<String>,
    pub current_reading: Option<Decimal>,
    pub previous_reading: Option<Decimal>,
    pub unit_price: Option<Decimal>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 更新计量设备请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateMeterRequest {
    pub meter_name: Option<String>,
    pub workshop: Option<String>,
    pub equipment_id: Option<i32>,
    pub equipment_name: Option<String>,
    pub location: Option<String>,
    pub iot_device_id: Option<String>,
    pub unit: Option<String>,
    pub unit_price: Option<Decimal>,
    pub status: Option<String>,
    pub remarks: Option<String>,
}

/// 计量设备查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct MeterQuery {
    pub meter_type: Option<String>,
    pub workshop: Option<String>,
    pub status: Option<String>,
    pub equipment_id: Option<i32>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 能源计量设备 Service
pub struct EnergyMeterService {
    db: Arc<DatabaseConnection>,
}

impl EnergyMeterService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 生成计量设备编号：EM-YYYYMMDDHHMMSS-NNN
    fn generate_meter_no() -> String {
        let now = chrono::Utc::now();
        let timestamp = now.format("%Y%m%d%H%M%S");
        let random = crate::utils::random::random_6_digit() % 1000;
        format!("EM-{}-{:03}", timestamp, random)
    }

    /// 创建计量设备
    pub async fn create(&self, req: CreateMeterRequest) -> Result<MeterModel, AppError> {
        // 校验能源类型
        validate_meter_type(&req.meter_type)?;

        // 校验单价非负
        let unit_price = req.unit_price.unwrap_or(Decimal::ZERO);
        if unit_price < Decimal::ZERO {
            return Err(AppError::business("单价不能为负"));
        }

        let meter_no = Self::generate_meter_no();
        let now = crate::utils::date_utils::utc_now_fixed();
        let unit = req.unit.unwrap_or_else(|| "度".to_string());

        let active = MeterActiveModel {
            id: Default::default(),
            meter_no: Set(meter_no),
            meter_name: Set(req.meter_name),
            meter_type: Set(req.meter_type),
            workshop: Set(req.workshop),
            equipment_id: Set(req.equipment_id),
            equipment_name: Set(req.equipment_name),
            location: Set(req.location),
            iot_device_id: Set(req.iot_device_id),
            unit: Set(unit),
            current_reading: Set(req.current_reading.unwrap_or(Decimal::ZERO)),
            previous_reading: Set(req.previous_reading.unwrap_or(Decimal::ZERO)),
            last_reading_at: Set(Some(now)),
            unit_price: Set(unit_price),
            status: Set(energy_meter_status::ACTIVE.to_string()),
            remarks: Set(req.remarks),
            is_deleted: Set(false),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("计量设备创建失败: {}", e)))?;
        Ok(result)
    }

    /// 更新计量设备
    pub async fn update(
        &self,
        id: i32,
        req: UpdateMeterRequest,
    ) -> Result<MeterModel, AppError> {
        let model = self.get_by_id(id).await?;

        let mut active: MeterActiveModel = model.into();

        if let Some(v) = req.meter_name {
            active.meter_name = Set(v);
        }
        if let Some(v) = req.workshop {
            active.workshop = Set(Some(v));
        }
        if let Some(v) = req.equipment_id {
            active.equipment_id = Set(Some(v));
        }
        if let Some(v) = req.equipment_name {
            active.equipment_name = Set(Some(v));
        }
        if let Some(v) = req.location {
            active.location = Set(Some(v));
        }
        if let Some(v) = req.iot_device_id {
            active.iot_device_id = Set(Some(v));
        }
        if let Some(v) = req.unit {
            active.unit = Set(v);
        }
        if let Some(v) = req.unit_price {
            if v < Decimal::ZERO {
                return Err(AppError::business("单价不能为负"));
            }
            active.unit_price = Set(v);
        }
        if let Some(v) = req.status {
            if v != energy_meter_status::ACTIVE
                && v != energy_meter_status::INACTIVE
                && v != energy_meter_status::MAINTENANCE
            {
                return Err(AppError::business(format!(
                    "计量设备状态必须是 active / inactive / maintenance，当前: {}",
                    v
                )));
            }
            active.status = Set(v);
        }
        if let Some(v) = req.remarks {
            active.remarks = Set(Some(v));
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 软删除计量设备
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        let mut active: MeterActiveModel = model.into();
        active.is_deleted = Set(true);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        active.update(&*self.db).await?;
        Ok(())
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<MeterModel, AppError> {
        MeterEntity::find_by_id(id)
            .filter(energy_meter::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("计量设备 {} 不存在", id)))
    }

    /// 按编号查询
    pub async fn get_by_no(&self, meter_no: &str) -> Result<MeterModel, AppError> {
        MeterEntity::find()
            .filter(energy_meter::Column::MeterNo.eq(meter_no))
            .filter(energy_meter::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("计量设备编号 {} 不存在", meter_no)))
    }

    /// 分页查询
    pub async fn list(
        &self,
        query: MeterQuery,
    ) -> Result<(Vec<MeterModel>, u64), AppError> {
        let mut q = MeterEntity::find().filter(energy_meter::Column::IsDeleted.eq(false));
        if let Some(v) = query.meter_type {
            q = q.filter(energy_meter::Column::MeterType.eq(v));
        }
        if let Some(v) = query.workshop {
            q = q.filter(energy_meter::Column::Workshop.eq(v));
        }
        if let Some(v) = query.status {
            q = q.filter(energy_meter::Column::Status.eq(v));
        }
        if let Some(v) = query.equipment_id {
            q = q.filter(energy_meter::Column::EquipmentId.eq(v));
        }

        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

        let total = q.clone().count(&*self.db).await?;
        let items = q
            .order_by_desc(energy_meter::Column::Id)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;
        Ok((items, total))
    }
}

// ============================================================================
// 能耗记录 Service
// ============================================================================

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

// ============================================================================
// 能耗分摊规则 Service
// ============================================================================

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

// ============================================================================
// 能耗分摊记录 Service
// ============================================================================

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
    /// 1. 查询时段内所有已确认的能耗记录，按车间+能源类型汇总总消耗和总成本
    /// 2. 查询时段内所有 completed 工序记录，按缸号+工序分组统计工时
    /// 3. 按工时比例分摊总能耗到每个缸号+工序
    /// 4. 生成分摊记录
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
            // 2. 查询该车间在该时段内的 completed 工序记录，按缸号+工序分组统计工时
            let step_records = StepEntity::find()
                .filter(process_step_record::Column::Status.eq("completed"))
                .filter(process_step_record::Column::IsDeleted.eq(false))
                .filter(process_step_record::Column::StartAt.gte(req.period_start))
                .filter(process_step_record::Column::StartAt.lte(req.period_end))
                .all(&*self.db)
                .await?;

            // 按缸号+工序分组汇总工时
            // 注意：工序记录没有直接的 workshop 字段，这里通过 equipment_name 或 route_code 简化处理
            // 真实业务中应关联设备所属车间
            let mut grouped_duration: std::collections::HashMap<DurationGroupKey, i32> =
                std::collections::HashMap::new();

            for step in step_records {
                // 缸号通过 flow_card 关联查询（简化：暂用 equipment_name 作为车间归属）
                let dye_lot_no = step.equipment_name.clone(); // 简化：实际应通过 flow_card 查询 dye_lot_no
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

            let total_duration: i32 = grouped_duration.values().sum();
            if total_duration == 0 {
                continue;
            }

            let total_duration_decimal = Decimal::from(total_duration);

            // 3. 按工时比例分摊
            for (key, duration) in grouped_duration {
                let dye_lot_no = key.dye_lot_no;
                let route_id = key.route_id;
                let route_code = key.route_code;
                let basis_value = Decimal::from(duration);
                let ratio = compute_allocation_ratio(basis_value, total_duration_decimal);
                let allocated_consumption =
                    compute_allocated_consumption(total_consumption, ratio);
                let allocated_cost = compute_allocated_cost(total_cost, ratio);

                // 查询生效规则
                let rule = if let Some(rid) = route_id {
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

                let allocation_no = Self::generate_allocation_no();
                let now = crate::utils::date_utils::utc_now_fixed();

                let active = AllocationRecordActiveModel {
                    id: Default::default(),
                    allocation_no: Set(allocation_no),
                    period_start: Set(req.period_start),
                    period_end: Set(req.period_end),
                    meter_type: Set(meter_type.clone()),
                    workshop: Set(Some(workshop.clone())),
                    allocation_rule_id: Set(rule.as_ref().map(|r| r.id)),
                    allocation_basis: Set(energy_allocation_basis::BY_DURATION.to_string()),
                    total_consumption: Set(total_consumption),
                    total_cost: Set(total_cost),
                    dye_lot_no: Set(dye_lot_no.clone()),
                    production_order_id: Set(None),
                    production_order_no: Set(None),
                    process_route_id: Set(route_id),
                    route_code: Set(route_code),
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
                };

                let result = active
                    .insert(&*self.db)
                    .await
                    .map_err(|e| AppError::database(format!("分摊记录创建失败: {}", e)))?;
                results.push(result);
            }
        }

        Ok(results)
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    #[test]
    fn 测试计算消耗量_正常() {
        let result = compute_consumption(Decimal::new(100, 0), Decimal::new(150, 0));
        assert_eq!(result, Decimal::new(50, 0));
    }

    #[test]
    fn 测试计算消耗量_当前小于上次返回零() {
        let result = compute_consumption(Decimal::new(150, 0), Decimal::new(100, 0));
        assert_eq!(result, Decimal::ZERO);
    }

    #[test]
    fn 测试计算消耗量_相等返回零() {
        let result = compute_consumption(Decimal::new(100, 0), Decimal::new(100, 0));
        assert_eq!(result, Decimal::ZERO);
    }

    #[test]
    fn 测试计算总成本() {
        let result = compute_total_cost(Decimal::new(50, 0), Decimal::new(12, 1));
        assert_eq!(result, Decimal::new(600, 0));
    }

    #[test]
    fn 测试计算分摊比例_正常() {
        let result = compute_allocation_ratio(Decimal::new(30, 0), Decimal::new(100, 0));
        assert_eq!(result, Decimal::new(30, 2)); // 0.30
    }

    #[test]
    fn 测试计算分摊比例_总依据为零返回零() {
        let result = compute_allocation_ratio(Decimal::new(30, 0), Decimal::ZERO);
        assert_eq!(result, Decimal::ZERO);
    }

    #[test]
    fn 测试计算分摊消耗量() {
        let result = compute_allocated_consumption(
            Decimal::new(1000, 0),
            Decimal::new(30, 2), // 0.30
        );
        assert_eq!(result, Decimal::new(300, 0));
    }

    #[test]
    fn 测试计算分摊成本() {
        let result = compute_allocated_cost(
            Decimal::new(5000, 0),
            Decimal::new(30, 2), // 0.30
        );
        assert_eq!(result, Decimal::new(1500, 0));
    }

    #[test]
    fn 测试计算单位能耗_正常() {
        let result = compute_unit_consumption(
            Decimal::new(300, 0),
            Some(Decimal::new(100, 0)),
        );
        assert_eq!(result, Some(Decimal::new(3, 0)));
    }

    #[test]
    fn 测试计算单位能耗_产量为零返回None() {
        let result = compute_unit_consumption(Decimal::new(300, 0), Some(Decimal::ZERO));
        assert_eq!(result, None);
    }

    #[test]
    fn 测试计算单位能耗_产量为None返回None() {
        let result = compute_unit_consumption(Decimal::new(300, 0), None);
        assert_eq!(result, None);
    }

    #[test]
    fn 测试超基准判断_正常未超() {
        let (exceeds, deviation) = check_consumption_exceeds_standard(
            Decimal::new(95, 0),
            Decimal::new(100, 0),
            Decimal::new(10, 2), // 0.10
        );
        assert!(!exceeds);
        assert_eq!(deviation, Decimal::ZERO);
    }

    #[test]
    fn 测试超基准判断_超出阈值() {
        // 标准 100，容差 10%，阈值 110，实际 120 → 超基准
        let (exceeds, deviation) = check_consumption_exceeds_standard(
            Decimal::new(120, 0),
            Decimal::new(100, 0),
            Decimal::new(10, 2),
        );
        assert!(exceeds);
        // 偏差 = (120 - 100) / 100 × 100 = 20
        assert_eq!(deviation, Decimal::new(20, 0));
    }

    #[test]
    fn 测试超基准判断_标准为零返回未超() {
        let (exceeds, deviation) = check_consumption_exceeds_standard(
            Decimal::new(120, 0),
            Decimal::ZERO,
            Decimal::new(10, 2),
        );
        assert!(!exceeds);
        assert_eq!(deviation, Decimal::ZERO);
    }

    #[test]
    fn 测试校验能源类型_合法() {
        assert!(validate_meter_type("water").is_ok());
        assert!(validate_meter_type("electricity").is_ok());
        assert!(validate_meter_type("steam").is_ok());
        assert!(validate_meter_type("gas").is_ok());
        assert!(validate_meter_type("compressed_air").is_ok());
    }

    #[test]
    fn 测试校验能源类型_非法() {
        assert!(validate_meter_type("invalid").is_err());
    }

    #[test]
    fn 测试校验分摊基准_合法() {
        assert!(validate_allocation_basis("by_duration").is_ok());
        assert!(validate_allocation_basis("by_output").is_ok());
        assert!(validate_allocation_basis("by_equipment").is_ok());
        assert!(validate_allocation_basis("by_workshop").is_ok());
    }

    #[test]
    fn 测试校验分摊基准_非法() {
        assert!(validate_allocation_basis("invalid").is_err());
    }
}
