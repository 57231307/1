//! 能源计量设备 Service
//!
//! 批次 488 D10-2a 拆分：从原 `energy_service.rs` 迁移 EnergyMeterService + 3 个 DTOs。

use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::models::energy_meter::{
    self, ActiveModel as MeterActiveModel, Entity as MeterEntity, Model as MeterModel,
};
use crate::models::status::energy_meter_status;
use crate::utils::error::AppError;

// 复用 facade 的纯函数校验（保持单一来源，避免逻辑重复）
use crate::services::energy_service::validate_meter_type;

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
