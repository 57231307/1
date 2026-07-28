//! 环境监测与固废处置 Service
//!
//! V15 P1 batch-08 缺陷 19：废水/废气/固废排放监测与合规校验
//! 依据：《水污染防治法》《大气污染防治法》《固废污染防治法》《环境噪声污染防治法》
//!
//! 真实业务：
//! - 污染物监测记录登记（废水 COD/氨氮/色度，废气 VOCs，厂界噪声）
//! - 排放浓度合规校验（自动判定是否超标）
//! - 超标预警（超标立即推送告警）
//! - 固废处置联单管理（危废转移联单制度）

use crate::models::pollutant_monitoring_record::{
    self, ActiveModel as MonitoringActiveModel, Entity as MonitoringEntity,
    Model as MonitoringModel,
};
use crate::models::solid_waste_disposal_record::{
    self, ActiveModel as WasteActiveModel, Entity as WasteEntity, Model as WasteModel,
};
use crate::utils::error::AppError;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use serde::Deserialize;
use std::sync::Arc;

/// 创建污染物监测记录请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateMonitoringRecordRequest {
    /// 监测类型：wastewater(废水) / exhaust(废气) / noise(噪声) / solid_waste(固废)
    pub monitoring_type: String,
    pub monitoring_point: String,
    pub pollutant_name: String,
    pub measured_value: Decimal,
    pub unit: String,
    /// 排放限值（用于自动判定是否超标）
    pub limit_value: Decimal,
    pub monitoring_time: chrono::DateTime<chrono::FixedOffset>,
    pub monitoring_method: Option<String>,
    pub equipment_id: Option<i32>,
    pub operator_id: Option<i32>,
    pub remarks: Option<String>,
}

/// 创建固废处置联单请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateSolidWasteDisposalRequest {
    pub manifest_no: String,
    /// 废物类型：sludge(污泥) / waste_fabric(废布) / chemical_waste(废化学剂)
    pub waste_type: String,
    /// 废物类别：hazardous(危废) / general(一般固废)
    pub waste_category: String,
    pub waste_amount: Decimal,
    pub waste_unit: Option<String>,
    pub generation_date: NaiveDate,
    pub disposal_date: Option<NaiveDate>,
    /// 处置方式：landfill(填埋) / incineration(焚烧) / reuse(综合利用) / storage(暂存)
    pub disposal_method: String,
    pub disposal_vendor_id: Option<i32>,
    pub disposal_vendor_name: Option<String>,
    pub transport_license_no: Option<String>,
    pub disposal_license_no: Option<String>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 监测记录查询参数
#[derive(Debug, Clone, Default, Deserialize)]
pub struct MonitoringRecordQuery {
    pub monitoring_type: Option<String>,
    pub pollutant_name: Option<String>,
    /// 仅查询超标记录
    pub only_exceeding: Option<bool>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 超标预警结果
#[derive(Debug, Clone)]
pub struct ExceedanceAlert {
    pub record: MonitoringModel,
    /// 超标倍数（实测值 / 限值 - 1）
    pub exceeding_ratio: Decimal,
}

/// 污染物排放限值参考表（印染行业国家标准）
///
/// 依据：
/// - 《水污染防治法》GB 4287-2012 纺织染整工业水污染物排放标准
/// - 《大气污染物综合排放标准》GB 16297
/// - 《工业企业厂界环境噪声排放标准》GB 12348
pub struct PollutionLimitReference;

impl PollutionLimitReference {
    /// 获取污染物排放限值（mg/L, mg/m³, dB）
    ///
    /// 返回 None 表示该污染物未在标准中预置，需用户手动传入 limit_value
    pub fn get_limit(monitoring_type: &str, pollutant_name: &str) -> Option<Decimal> {
        match (monitoring_type, pollutant_name) {
            // 废水排放限值（GB 4287-2012 纺织染整工业水污染物排放标准）
            ("wastewater", "COD") | ("wastewater", "cod") => Some(Decimal::new(80, 0)), // 80 mg/L
            ("wastewater", "氨氮") | ("wastewater", "NH3-N") => Some(Decimal::new(10, 0)), // 10 mg/L
            ("wastewater", "色度") | ("wastewater", "chromaticity") => Some(Decimal::new(50, 0)), // 50 倍
            // 废气排放限值
            ("exhaust", "VOCs") | ("exhaust", "vocs") => Some(Decimal::new(60, 0)), // 60 mg/m³
            // 厂界噪声（GB 12348）
            ("noise", "厂界昼间") | ("noise", "daytime") => Some(Decimal::new(65, 0)), // 65 dB
            ("noise", "厂界夜间") | ("noise", "nighttime") => Some(Decimal::new(55, 0)), // 55 dB
            _ => None,
        }
    }
}

pub struct PollutionMonitoringService {
    db: Arc<DatabaseConnection>,
}

impl PollutionMonitoringService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建污染物监测记录（自动判定是否超标）
    ///
    /// 业务规则：
    /// - 监测类型 wastewater/exhaust/noise/solid_waste
    /// - 实测值 > 限值 → is_exceeding=true，自动计算超标倍数
    /// - 超标倍数 = 实测值 / 限值 - 1
    pub async fn create_monitoring_record(
        &self,
        req: CreateMonitoringRecordRequest,
    ) -> Result<MonitoringModel, AppError> {
        Self::validate_monitoring_type(&req.monitoring_type)?;
        if req.limit_value <= Decimal::ZERO {
            return Err(AppError::bad_request("排放限值必须大于 0"));
        }

        // 自动判定是否超标
        let (is_exceeding, exceeding_ratio) =
            Self::check_exceedance(req.measured_value, req.limit_value);

        let now = crate::utils::date_utils::utc_now_fixed();
        let active = MonitoringActiveModel {
            monitoring_type: Set(req.monitoring_type),
            monitoring_point: Set(req.monitoring_point),
            pollutant_name: Set(req.pollutant_name),
            measured_value: Set(req.measured_value),
            unit: Set(req.unit),
            limit_value: Set(req.limit_value),
            is_exceeding: Set(is_exceeding),
            exceeding_ratio: Set(exceeding_ratio),
            monitoring_time: Set(req.monitoring_time),
            monitoring_method: Set(req.monitoring_method),
            equipment_id: Set(req.equipment_id),
            operator_id: Set(req.operator_id),
            remarks: Set(req.remarks),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("污染物监测记录创建失败: {}", e)))?;

        // 超标时立即生成预警（业务层应订阅此事件触发推送）
        if is_exceeding {
            tracing::warn!(
                record_id = result.id,
                pollutant = %result.pollutant_name,
                measured = %result.measured_value,
                limit = %result.limit_value,
                "污染物排放超标预警"
            );
        }

        Ok(result)
    }

    /// 查询监测记录列表
    pub async fn list_monitoring_records(
        &self,
        params: MonitoringRecordQuery,
    ) -> Result<(Vec<MonitoringModel>, u64), AppError> {
        let mut query = MonitoringEntity::find();

        if let Some(monitoring_type) = &params.monitoring_type {
            query = query
                .filter(pollutant_monitoring_record::Column::MonitoringType.eq(monitoring_type));
        }
        if let Some(pollutant_name) = &params.pollutant_name {
            query =
                query.filter(pollutant_monitoring_record::Column::PollutantName.eq(pollutant_name));
        }
        if params.only_exceeding.unwrap_or(false) {
            query = query.filter(pollutant_monitoring_record::Column::IsExceeding.eq(true));
        }

        let total = query.clone().count(&*self.db).await?;

        let page = params.page.unwrap_or(1).max(1);
        let page_size = params.page_size.unwrap_or(20).clamp(1, 200);

        let list = query
            .order_by_desc(pollutant_monitoring_record::Column::MonitoringTime)
            .offset((page - 1) * page_size)
            .limit(page_size)
            .all(&*self.db)
            .await?;

        Ok((list, total))
    }

    /// 创建固废处置联单
    pub async fn create_solid_waste_disposal(
        &self,
        req: CreateSolidWasteDisposalRequest,
    ) -> Result<WasteModel, AppError> {
        Self::validate_waste_type(&req.waste_type)?;
        Self::validate_waste_category(&req.waste_category)?;
        Self::validate_disposal_method(&req.disposal_method)?;

        // 危废必须填写运输许可证号与处置许可证号（《固废法》要求）
        if req.waste_category == "hazardous" {
            if req.transport_license_no.is_none() || req.disposal_license_no.is_none() {
                return Err(AppError::business(
                    "危废处置必须填写运输许可证号与处置许可证号（《固废法》第82条）",
                ));
            }
        }

        // 校验联单号唯一性
        if WasteEntity::find()
            .filter(solid_waste_disposal_record::Column::ManifestNo.eq(&req.manifest_no))
            .one(&*self.db)
            .await?
            .is_some()
        {
            return Err(AppError::business(format!(
                "固废处置联单号 {} 已存在",
                req.manifest_no
            )));
        }

        let now = crate::utils::date_utils::utc_now_fixed();
        let active = WasteActiveModel {
            manifest_no: Set(req.manifest_no),
            waste_type: Set(req.waste_type),
            waste_category: Set(req.waste_category),
            waste_amount: Set(req.waste_amount),
            waste_unit: Set(req.waste_unit.unwrap_or_else(|| "ton".to_string())),
            generation_date: Set(req.generation_date),
            disposal_date: Set(req.disposal_date),
            disposal_method: Set(req.disposal_method),
            disposal_vendor_id: Set(req.disposal_vendor_id),
            disposal_vendor_name: Set(req.disposal_vendor_name),
            transport_license_no: Set(req.transport_license_no),
            disposal_license_no: Set(req.disposal_license_no),
            status: Set("pending".to_string()),
            remarks: Set(req.remarks),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("固废处置联单创建失败: {}", e)))?;
        Ok(result)
    }

    /// 更新固废处置状态（运输中 / 已处置）
    pub async fn update_waste_status(
        &self,
        id: i32,
        status: &str,
        disposal_date: Option<NaiveDate>,
    ) -> Result<WasteModel, AppError> {
        Self::validate_waste_status(status)?;

        let waste = WasteEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("固废处置联单 {} 不存在", id)))?;

        // 状态机校验
        let valid_transition = match (waste.status.as_str(), status) {
            ("pending", "transporting") => true,
            ("pending", "cancelled") => true,
            ("transporting", "disposed") => true,
            ("transporting", "cancelled") => true,
            _ => false,
        };
        if !valid_transition {
            return Err(AppError::business(format!(
                "非法状态转换: {} → {}（合法: pending→transporting→disposed/cancelled）",
                waste.status, status
            )));
        }

        let mut active: WasteActiveModel = waste.into();
        active.status = Set(status.to_string());
        if status == "disposed" {
            active.disposal_date =
                Set(disposal_date.or_else(|| Some(chrono::Local::now().date_naive())));
        }
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 查询超标记录并生成预警
    pub async fn scan_exceedance_alerts(&self) -> Result<Vec<ExceedanceAlert>, AppError> {
        let exceeding_records = MonitoringEntity::find()
            .filter(pollutant_monitoring_record::Column::IsExceeding.eq(true))
            .order_by_desc(pollutant_monitoring_record::Column::MonitoringTime)
            .all(&*self.db)
            .await?;

        let alerts = exceeding_records
            .into_iter()
            .map(|record| {
                let exceeding_ratio = record.exceeding_ratio.unwrap_or(Decimal::ZERO);
                ExceedanceAlert {
                    record,
                    exceeding_ratio,
                }
            })
            .collect();
        Ok(alerts)
    }

    /// 校验是否超标（纯函数）
    ///
    /// 业务规则：
    /// - 实测值 > 限值 → 超标
    /// - 超标倍数 = 实测值 / 限值 - 1（保留 4 位小数）
    pub fn check_exceedance(measured: Decimal, limit: Decimal) -> (bool, Option<Decimal>) {
        if measured > limit && limit > Decimal::ZERO {
            let ratio = measured / limit - Decimal::ONE;
            (true, Some(ratio))
        } else {
            (false, None)
        }
    }

    /// 校验监测类型
    fn validate_monitoring_type(monitoring_type: &str) -> Result<(), AppError> {
        match monitoring_type {
            "wastewater" | "exhaust" | "noise" | "solid_waste" => Ok(()),
            _ => Err(AppError::bad_request(format!(
                "无效的监测类型: {}（应为 wastewater/exhaust/noise/solid_waste）",
                monitoring_type
            ))),
        }
    }

    /// 校验废物类型
    fn validate_waste_type(waste_type: &str) -> Result<(), AppError> {
        match waste_type {
            "sludge" | "waste_fabric" | "chemical_waste" => Ok(()),
            _ => Err(AppError::bad_request(format!(
                "无效的废物类型: {}（应为 sludge/waste_fabric/chemical_waste）",
                waste_type
            ))),
        }
    }

    /// 校验废物类别
    fn validate_waste_category(waste_category: &str) -> Result<(), AppError> {
        match waste_category {
            "hazardous" | "general" => Ok(()),
            _ => Err(AppError::bad_request(format!(
                "无效的废物类别: {}（应为 hazardous/general）",
                waste_category
            ))),
        }
    }

    /// 校验处置方式
    fn validate_disposal_method(method: &str) -> Result<(), AppError> {
        match method {
            "landfill" | "incineration" | "reuse" | "storage" => Ok(()),
            _ => Err(AppError::bad_request(format!(
                "无效的处置方式: {}（应为 landfill/incineration/reuse/storage）",
                method
            ))),
        }
    }

    /// 校验废物状态
    fn validate_waste_status(status: &str) -> Result<(), AppError> {
        match status {
            "pending" | "transporting" | "disposed" | "cancelled" => Ok(()),
            _ => Err(AppError::bad_request(format!(
                "无效的废物状态: {}（应为 pending/transporting/disposed/cancelled）",
                status
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    #[test]
    fn test_check_exceedance_normal() {
        let (is_exceeding, ratio) =
            PollutionMonitoringService::check_exceedance(Decimal::new(50, 0), Decimal::new(80, 0));
        assert!(!is_exceeding);
        assert_eq!(ratio, None);
    }

    #[test]
    fn test_check_exceedance_at_limit() {
        // 实测值等于限值不算超标
        let (is_exceeding, ratio) =
            PollutionMonitoringService::check_exceedance(Decimal::new(80, 0), Decimal::new(80, 0));
        assert!(!is_exceeding);
        assert_eq!(ratio, None);
    }

    #[test]
    fn test_check_exceedance_exceeded() {
        // 实测 120，限值 80 → 超标 0.5 倍
        let (is_exceeding, ratio) =
            PollutionMonitoringService::check_exceedance(Decimal::new(120, 0), Decimal::new(80, 0));
        assert!(is_exceeding);
        assert_eq!(ratio, Some(Decimal::new(5, 1))); // 0.5
    }

    #[test]
    fn test_validate_monitoring_type() {
        assert!(PollutionMonitoringService::validate_monitoring_type("wastewater").is_ok());
        assert!(PollutionMonitoringService::validate_monitoring_type("exhaust").is_ok());
        assert!(PollutionMonitoringService::validate_monitoring_type("noise").is_ok());
        assert!(PollutionMonitoringService::validate_monitoring_type("solid_waste").is_ok());
        assert!(PollutionMonitoringService::validate_monitoring_type("invalid").is_err());
    }

    #[test]
    fn test_validate_waste_type() {
        assert!(PollutionMonitoringService::validate_waste_type("sludge").is_ok());
        assert!(PollutionMonitoringService::validate_waste_type("waste_fabric").is_ok());
        assert!(PollutionMonitoringService::validate_waste_type("chemical_waste").is_ok());
        assert!(PollutionMonitoringService::validate_waste_type("invalid").is_err());
    }

    #[test]
    fn test_validate_disposal_method() {
        assert!(PollutionMonitoringService::validate_disposal_method("landfill").is_ok());
        assert!(PollutionMonitoringService::validate_disposal_method("incineration").is_ok());
        assert!(PollutionMonitoringService::validate_disposal_method("reuse").is_ok());
        assert!(PollutionMonitoringService::validate_disposal_method("storage").is_ok());
        assert!(PollutionMonitoringService::validate_disposal_method("invalid").is_err());
    }

    #[test]
    fn test_pollution_limit_reference_cod() {
        let limit = PollutionLimitReference::get_limit("wastewater", "COD");
        assert_eq!(limit, Some(Decimal::new(80, 0)));
    }

    #[test]
    fn test_pollution_limit_reference_vocs() {
        let limit = PollutionLimitReference::get_limit("exhaust", "VOCs");
        assert_eq!(limit, Some(Decimal::new(60, 0)));
    }

    #[test]
    fn test_pollution_limit_reference_unknown() {
        let limit = PollutionLimitReference::get_limit("wastewater", "Unknown");
        assert_eq!(limit, None);
    }
}
