//! 职业健康合规 Service
//!
//! V15 P1 batch-08 缺陷 24：职业健康合规
//! 依据：《职业病防治法》第 26/35 条 + 《危险化学品安全管理条例》
//!
//! 真实业务：
//! - 职业危害因素检测（苯/甲醛/噪声/粉尘/高温），超标自动预警
//! - 职业健康体检档案（上岗前/在岗期间/离岗时），在岗期间体检到期提醒
//! - PPE 个人防护用品发放记录，到期/回收状态管理

use crate::models::occupational_health_exam::{
    self, ActiveModel as ExamActiveModel, Entity as ExamEntity, Model as ExamModel,
};
use crate::models::occupational_hazard_monitoring::{
    self, ActiveModel as HazardActiveModel, Entity as HazardEntity, Model as HazardModel,
};
use crate::models::ppe_distribution_record::{
    self, ActiveModel as PpeActiveModel, Entity as PpeEntity, Model as PpeModel,
};
use crate::utils::error::AppError;
use chrono::{Local, NaiveDate};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use serde::Deserialize;
use std::sync::Arc;

/// 创建职业危害因素检测记录请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateHazardMonitoringRequest {
    /// 危害类型：chemical(化学) / physical(物理) / dust(粉尘) / biological(生物)
    pub hazard_type: String,
    /// 危害名称：苯/甲醛/噪声/粉尘/高温
    pub hazard_name: String,
    pub monitoring_point: String,
    pub measured_value: Decimal,
    pub unit: String,
    /// 限值（用于自动判定是否超标）
    pub limit_value: Decimal,
    pub monitoring_date: NaiveDate,
    pub monitoring_organization: Option<String>,
    pub monitoring_method: Option<String>,
    pub report_url: Option<String>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 创建职业健康体检档案请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateHealthExamRequest {
    pub worker_id: i32,
    /// 体检类型：pre_employment(上岗前) / in_service(在岗期间) / resignation(离岗时)
    pub exam_type: String,
    pub exam_date: NaiveDate,
    /// 下次体检日期（在岗期间体检必填，自动到期提醒）
    pub next_exam_date: Option<NaiveDate>,
    pub exam_organization: Option<String>,
    /// 体检结果：normal(正常) / abnormal(异常) / contraindication(禁忌)
    pub exam_result: String,
    /// 危害暴露史（JSON）
    pub hazard_exposure: Option<serde_json::Value>,
    pub contraindications: Option<String>,
    pub report_url: Option<String>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 创建 PPE 发放记录请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreatePpeDistributionRequest {
    pub worker_id: i32,
    pub ppe_name: String,
    /// 防护用品类型：mask(口罩) / gloves(手套) / goggles(护目镜) / earplug(耳塞) / respirator(防毒面具) / suit(防护服)
    pub ppe_type: String,
    pub specification: Option<String>,
    pub quantity: i32,
    pub distribution_date: NaiveDate,
    /// 到期日期（用于过期检查）
    pub expiry_date: Option<NaiveDate>,
    pub hazard_type: Option<String>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 危害因素检测记录查询参数
#[derive(Debug, Clone, Default, Deserialize)]
pub struct HazardMonitoringQuery {
    pub hazard_type: Option<String>,
    pub hazard_name: Option<String>,
    /// 仅查询超标记录
    pub only_exceeding: Option<bool>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 体检档案查询参数
#[derive(Debug, Clone, Default, Deserialize)]
pub struct HealthExamQuery {
    pub worker_id: Option<i32>,
    pub exam_type: Option<String>,
    pub exam_result: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// PPE 发放记录查询参数
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PpeDistributionQuery {
    pub worker_id: Option<i32>,
    pub ppe_type: Option<String>,
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 职业危害因素超标预警
#[derive(Debug, Clone)]
pub struct HazardExceedanceAlert {
    pub record: HazardModel,
    /// 超标倍数（实测值 / 限值 - 1）
    pub exceeding_ratio: Decimal,
}

/// 体检到期预警级别
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExamExpiryWarningLevel {
    /// 已过期
    Expired,
    /// 30 天内到期
    Critical,
    /// 60 天内到期
    Warning,
    /// 90 天内到期
    Notice,
    /// 暂无需预警
    None,
}

impl ExamExpiryWarningLevel {
    pub fn needs_warning(&self) -> bool {
        matches!(self, Self::Expired | Self::Critical | Self::Warning | Self::Notice)
    }
}

/// 体检到期预警结果
#[derive(Debug, Clone)]
pub struct ExamExpiryWarning {
    pub exam: ExamModel,
    pub level: ExamExpiryWarningLevel,
    pub days_until_expiry: i64,
}

/// 职业危害因素限值参考表
///
/// 依据：
/// - 《工作场所有害因素职业接触限值 第1部分：化学有害因素》GBZ 2.1
/// - 《工作场所有害因素职业接触限值 第2部分：物理因素》GBZ 2.2
pub struct OccupationalHazardLimitReference;

impl OccupationalHazardLimitReference {
    /// 获取职业危害因素限值（PC-TWA 时间加权平均容许浓度，mg/m³ 或 dB）
    ///
    /// 返回 None 表示该危害因素未在标准中预置，需用户手动传入 limit_value
    pub fn get_limit(hazard_type: &str, hazard_name: &str) -> Option<Decimal> {
        match (hazard_type, hazard_name) {
            // 化学有害因素（GBZ 2.1）
            ("chemical", "苯") | ("chemical", "benzene") => Some(Decimal::new(6, 0)),    // 6 mg/m³
            ("chemical", "甲醛") | ("chemical", "formaldehyde") => Some(Decimal::new(5, 1)), // 0.5 mg/m³
            ("chemical", "甲苯") | ("chemical", "toluene") => Some(Decimal::new(50, 0)), // 50 mg/m³
            ("chemical", "二甲苯") | ("chemical", "xylene") => Some(Decimal::new(50, 0)), // 50 mg/m³
            // 物理因素（GBZ 2.2）
            ("physical", "噪声") | ("physical", "noise") => Some(Decimal::new(85, 0)),   // 85 dB
            ("physical", "高温") | ("physical", "heat") => Some(Decimal::new(35, 0)),     // 35℃（综合温度）
            // 粉尘（总尘）
            ("dust", "棉尘") | ("dust", "cotton_dust") => Some(Decimal::new(1, 0)),       // 1 mg/m³
            ("dust", "矽尘") | ("dust", "silica") => Some(Decimal::new(1, 0)),            // 1 mg/m³（含10%以上游离二氧化硅）
            _ => None,
        }
    }
}

pub struct OccupationalHealthService {
    db: Arc<DatabaseConnection>,
}

impl OccupationalHealthService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建职业危害因素检测记录（自动判定是否超标）
    ///
    /// 业务规则（《职业病防治法》第 26 条）：
    /// - 实测值 > 限值 → is_exceeding=true，自动计算超标倍数
    /// - 超标倍数 = 实测值 / 限值 - 1
    /// - 超标时立即生成预警
    pub async fn create_hazard_monitoring(
        &self,
        req: CreateHazardMonitoringRequest,
    ) -> Result<HazardModel, AppError> {
        Self::validate_hazard_type(&req.hazard_type)?;
        if req.limit_value <= Decimal::ZERO {
            return Err(AppError::bad_request("职业危害因素限值必须大于 0"));
        }

        // 自动判定是否超标
        let (is_exceeding, exceeding_ratio) = Self::check_exceedance(req.measured_value, req.limit_value);

        let now = crate::utils::date_utils::utc_now_fixed();
        let active = HazardActiveModel {
            hazard_type: Set(req.hazard_type),
            hazard_name: Set(req.hazard_name.clone()),
            monitoring_point: Set(req.monitoring_point),
            measured_value: Set(req.measured_value),
            unit: Set(req.unit),
            limit_value: Set(req.limit_value),
            is_exceeding: Set(is_exceeding),
            exceeding_ratio: Set(exceeding_ratio),
            monitoring_date: Set(req.monitoring_date),
            monitoring_organization: Set(req.monitoring_organization),
            monitoring_method: Set(req.monitoring_method),
            report_url: Set(req.report_url),
            remarks: Set(req.remarks),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("职业危害因素检测记录创建失败: {}", e)))?;

        // 超标时立即生成预警（业务层应订阅此事件触发推送）
        if is_exceeding {
            tracing::warn!(
                record_id = result.id,
                hazard_name = %req.hazard_name,
                measured = %result.measured_value,
                limit = %result.limit_value,
                "职业危害因素超标预警（《职业病防治法》第26条）"
            );
        }

        Ok(result)
    }

    /// 查询职业危害因素检测记录列表
    pub async fn list_hazard_monitorings(
        &self,
        params: HazardMonitoringQuery,
    ) -> Result<(Vec<HazardModel>, u64), AppError> {
        let mut query = HazardEntity::find();

        if let Some(hazard_type) = &params.hazard_type {
            query = query.filter(occupational_hazard_monitoring::Column::HazardType.eq(hazard_type));
        }
        if let Some(hazard_name) = &params.hazard_name {
            query = query.filter(occupational_hazard_monitoring::Column::HazardName.eq(hazard_name));
        }
        if params.only_exceeding.unwrap_or(false) {
            query = query.filter(occupational_hazard_monitoring::Column::IsExceeding.eq(true));
        }

        let total = query.clone().count(&*self.db).await?;

        let page = params.page.unwrap_or(1).max(1);
        let page_size = params.page_size.unwrap_or(20).clamp(1, 200);

        let list = query
            .order_by_desc(occupational_hazard_monitoring::Column::MonitoringDate)
            .offset((page - 1) * page_size)
            .limit(page_size)
            .all(&*self.db)
            .await?;

        Ok((list, total))
    }

    /// 创建职业健康体检档案
    ///
    /// 业务规则（《职业病防治法》第 35 条）：
    /// - 上岗前必须体检（pre_employment）
    /// - 在岗期间每年一次体检（in_service，next_exam_date 必填）
    /// - 离岗时必须体检（resignation）
    /// - 体检结果为 contraindication 时禁止从事相关作业
    pub async fn create_health_exam(
        &self,
        req: CreateHealthExamRequest,
    ) -> Result<ExamModel, AppError> {
        Self::validate_exam_type(&req.exam_type)?;
        Self::validate_exam_result(&req.exam_result)?;

        // 在岗期间体检必须有下次体检日期
        if req.exam_type == "in_service" && req.next_exam_date.is_none() {
            return Err(AppError::business(
                "在岗期间体检必须填写下次体检日期（《职业病防治法》第35条）",
            ));
        }

        // next_exam_date 必须晚于体检日期
        if let Some(next) = req.next_exam_date {
            if next <= req.exam_date {
                return Err(AppError::bad_request("下次体检日期必须晚于本次体检日期"));
            }
        }

        let now = crate::utils::date_utils::utc_now_fixed();
        let active = ExamActiveModel {
            worker_id: Set(req.worker_id),
            exam_type: Set(req.exam_type),
            exam_date: Set(req.exam_date),
            next_exam_date: Set(req.next_exam_date),
            exam_organization: Set(req.exam_organization),
            exam_result: Set(req.exam_result.clone()),
            hazard_exposure: Set(req.hazard_exposure),
            contraindications: Set(req.contraindications),
            report_url: Set(req.report_url),
            remarks: Set(req.remarks),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("职业健康体检档案创建失败: {}", e)))?;

        // 体检结果为禁忌症时立即预警
        if req.exam_result == "contraindication" {
            tracing::warn!(
                exam_id = result.id,
                worker_id = req.worker_id,
                "职业健康禁忌症预警：禁止从事相关危害作业（《职业病防治法》第35条）"
            );
        }

        Ok(result)
    }

    /// 查询体检档案列表
    pub async fn list_health_exams(
        &self,
        params: HealthExamQuery,
    ) -> Result<(Vec<ExamModel>, u64), AppError> {
        let mut query = ExamEntity::find();

        if let Some(worker_id) = params.worker_id {
            query = query.filter(occupational_health_exam::Column::WorkerId.eq(worker_id));
        }
        if let Some(exam_type) = &params.exam_type {
            query = query.filter(occupational_health_exam::Column::ExamType.eq(exam_type));
        }
        if let Some(exam_result) = &params.exam_result {
            query = query.filter(occupational_health_exam::Column::ExamResult.eq(exam_result));
        }

        let total = query.clone().count(&*self.db).await?;

        let page = params.page.unwrap_or(1).max(1);
        let page_size = params.page_size.unwrap_or(20).clamp(1, 200);

        let list = query
            .order_by_desc(occupational_health_exam::Column::ExamDate)
            .offset((page - 1) * page_size)
            .limit(page_size)
            .all(&*self.db)
            .await?;

        Ok((list, total))
    }

    /// 扫描在岗期间体检到期预警
    ///
    /// 业务规则（《职业病防治法》第 35 条）：
    /// - 到期前 90/60/30 天三级预警
    /// - 已过期的高优先级预警（应立即组织体检）
    pub async fn scan_exam_expiry_warnings(&self) -> Result<Vec<ExamExpiryWarning>, AppError> {
        let today = Local::now().date_naive();
        // 仅扫描在岗期间体检，且有下次体检日期的记录
        let exams = ExamEntity::find()
            .filter(occupational_health_exam::Column::ExamType.eq("in_service"))
            .filter(occupational_health_exam::Column::NextExamDate.is_not_null())
            .all(&*self.db)
            .await?;

        let mut warnings = Vec::new();
        for exam in exams {
            if let Some(next_date) = exam.next_exam_date {
                let days_until_expiry = (next_date - today).num_days();
                let level = Self::classify_exam_expiry_level(days_until_expiry);

                if level.needs_warning() {
                    warnings.push(ExamExpiryWarning {
                        exam,
                        level,
                        days_until_expiry,
                    });
                }
            }
        }

        // 按到期日期升序排列（最紧急的在前）
        warnings.sort_by_key(|w| w.days_until_expiry);
        Ok(warnings)
    }

    /// 创建 PPE 发放记录
    ///
    /// 业务规则（《职业病防治法》第 22 条）：
    /// - 必须为接触危害因素的工人配备 PPE
    /// - PPE 必须在有效期内使用
    /// - 到期前应提醒更换
    pub async fn create_ppe_distribution(
        &self,
        req: CreatePpeDistributionRequest,
    ) -> Result<PpeModel, AppError> {
        Self::validate_ppe_type(&req.ppe_type)?;
        if req.quantity <= 0 {
            return Err(AppError::bad_request("PPE 发放数量必须大于 0"));
        }

        // 到期日期必须晚于发放日期
        if let Some(expiry) = req.expiry_date {
            if expiry <= req.distribution_date {
                return Err(AppError::bad_request("PPE 到期日期必须晚于发放日期"));
            }
        }

        let now = crate::utils::date_utils::utc_now_fixed();
        let active = PpeActiveModel {
            worker_id: Set(req.worker_id),
            ppe_name: Set(req.ppe_name),
            ppe_type: Set(req.ppe_type),
            specification: Set(req.specification),
            quantity: Set(req.quantity),
            distribution_date: Set(req.distribution_date),
            expiry_date: Set(req.expiry_date),
            hazard_type: Set(req.hazard_type),
            status: Set("distributed".to_string()),
            remarks: Set(req.remarks),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("PPE 发放记录创建失败: {}", e)))?;

        Ok(result)
    }

    /// 查询 PPE 发放记录列表
    pub async fn list_ppe_distributions(
        &self,
        params: PpeDistributionQuery,
    ) -> Result<(Vec<PpeModel>, u64), AppError> {
        let mut query = PpeEntity::find();

        if let Some(worker_id) = params.worker_id {
            query = query.filter(ppe_distribution_record::Column::WorkerId.eq(worker_id));
        }
        if let Some(ppe_type) = &params.ppe_type {
            query = query.filter(ppe_distribution_record::Column::PpeType.eq(ppe_type));
        }
        if let Some(status) = &params.status {
            query = query.filter(ppe_distribution_record::Column::Status.eq(status));
        }

        let total = query.clone().count(&*self.db).await?;

        let page = params.page.unwrap_or(1).max(1);
        let page_size = params.page_size.unwrap_or(20).clamp(1, 200);

        let list = query
            .order_by_desc(ppe_distribution_record::Column::DistributionDate)
            .offset((page - 1) * page_size)
            .limit(page_size)
            .all(&*self.db)
            .await?;

        Ok((list, total))
    }

    /// 回收 PPE（distributed → returned）
    pub async fn return_ppe(&self, id: i32) -> Result<PpeModel, AppError> {
        let model = PpeEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("PPE 发放记录 {} 不存在", id)))?;

        if model.status != "distributed" {
            return Err(AppError::business(format!(
                "仅已发放(distributed)状态可回收，当前状态: {}",
                model.status
            )));
        }

        let mut active: PpeActiveModel = model.into();
        active.status = Set("returned".to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 扫描已过期的 PPE，自动更新状态为 expired
    ///
    /// 业务规则：PPE 到期后禁止使用，必须立即更换
    pub async fn scan_expired_ppe(&self) -> Result<Vec<PpeModel>, AppError> {
        let today = Local::now().date_naive();
        let expired_candidates = PpeEntity::find()
            .filter(ppe_distribution_record::Column::Status.eq("distributed"))
            .filter(ppe_distribution_record::Column::ExpiryDate.is_not_null())
            .all(&*self.db)
            .await?;

        let mut expired_list = Vec::new();
        for model in expired_candidates {
            if let Some(expiry) = model.expiry_date {
                if expiry < today {
                    let mut active: PpeActiveModel = model.clone().into();
                    active.status = Set("expired".to_string());
                    active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
                    if let Ok(updated) = active.update(&*self.db).await {
                        expired_list.push(updated);
                    }
                }
            }
        }

        Ok(expired_list)
    }

    /// 判定是否超标（纯函数）
    ///
    /// 业务规则：
    /// - 实测值 > 限值 → is_exceeding=true
    /// - 超标倍数 = 实测值 / 限值 - 1
    fn check_exceedance(measured_value: Decimal, limit_value: Decimal) -> (bool, Option<Decimal>) {
        if measured_value > limit_value && limit_value > Decimal::ZERO {
            let ratio = measured_value / limit_value - Decimal::ONE;
            (true, Some(ratio))
        } else {
            (false, None)
        }
    }

    /// 体检到期预警级别分类（纯函数）
    fn classify_exam_expiry_level(days_until_expiry: i64) -> ExamExpiryWarningLevel {
        if days_until_expiry < 0 {
            ExamExpiryWarningLevel::Expired
        } else if days_until_expiry <= 30 {
            ExamExpiryWarningLevel::Critical
        } else if days_until_expiry <= 60 {
            ExamExpiryWarningLevel::Warning
        } else if days_until_expiry <= 90 {
            ExamExpiryWarningLevel::Notice
        } else {
            ExamExpiryWarningLevel::None
        }
    }

    /// 校验危害类型
    fn validate_hazard_type(hazard_type: &str) -> Result<(), AppError> {
        match hazard_type {
            "chemical" | "physical" | "dust" | "biological" => Ok(()),
            _ => Err(AppError::bad_request(format!(
                "无效的危害类型: {}（应为 chemical/physical/dust/biological）",
                hazard_type
            ))),
        }
    }

    /// 校验体检类型
    fn validate_exam_type(exam_type: &str) -> Result<(), AppError> {
        match exam_type {
            "pre_employment" | "in_service" | "resignation" => Ok(()),
            _ => Err(AppError::bad_request(format!(
                "无效的体检类型: {}（应为 pre_employment/in_service/resignation）",
                exam_type
            ))),
        }
    }

    /// 校验体检结果
    fn validate_exam_result(exam_result: &str) -> Result<(), AppError> {
        match exam_result {
            "normal" | "abnormal" | "contraindication" => Ok(()),
            _ => Err(AppError::bad_request(format!(
                "无效的体检结果: {}（应为 normal/abnormal/contraindication）",
                exam_result
            ))),
        }
    }

    /// 校验 PPE 类型
    fn validate_ppe_type(ppe_type: &str) -> Result<(), AppError> {
        match ppe_type {
            "mask" | "gloves" | "goggles" | "earplug" | "respirator" | "suit" => Ok(()),
            _ => Err(AppError::bad_request(format!(
                "无效的 PPE 类型: {}（应为 mask/gloves/goggles/earplug/respirator/suit）",
                ppe_type
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_exceedance_normal() {
        let (is_exceeding, ratio) =
            OccupationalHealthService::check_exceedance(Decimal::new(5, 0), Decimal::new(10, 0));
        assert!(!is_exceeding);
        assert!(ratio.is_none());
    }

    #[test]
    fn test_check_exceedance_exceeding() {
        let (is_exceeding, ratio) =
            OccupationalHealthService::check_exceedance(Decimal::new(15, 0), Decimal::new(10, 0));
        assert!(is_exceeding);
        // 超标倍数 = 15/10 - 1 = 0.5
        assert_eq!(ratio, Some(Decimal::new(5, 1)));
    }

    #[test]
    fn test_check_exceedance_equal() {
        let (is_exceeding, ratio) =
            OccupationalHealthService::check_exceedance(Decimal::new(10, 0), Decimal::new(10, 0));
        assert!(!is_exceeding);
        assert!(ratio.is_none());
    }

    #[test]
    fn test_classify_exam_expiry_level_expired() {
        assert_eq!(
            OccupationalHealthService::classify_exam_expiry_level(-1),
            ExamExpiryWarningLevel::Expired
        );
        assert_eq!(
            OccupationalHealthService::classify_exam_expiry_level(-30),
            ExamExpiryWarningLevel::Expired
        );
    }

    #[test]
    fn test_classify_exam_expiry_level_critical() {
        assert_eq!(
            OccupationalHealthService::classify_exam_expiry_level(0),
            ExamExpiryWarningLevel::Critical
        );
        assert_eq!(
            OccupationalHealthService::classify_exam_expiry_level(30),
            ExamExpiryWarningLevel::Critical
        );
    }

    #[test]
    fn test_classify_exam_expiry_level_warning() {
        assert_eq!(
            OccupationalHealthService::classify_exam_expiry_level(31),
            ExamExpiryWarningLevel::Warning
        );
        assert_eq!(
            OccupationalHealthService::classify_exam_expiry_level(60),
            ExamExpiryWarningLevel::Warning
        );
    }

    #[test]
    fn test_classify_exam_expiry_level_notice() {
        assert_eq!(
            OccupationalHealthService::classify_exam_expiry_level(61),
            ExamExpiryWarningLevel::Notice
        );
        assert_eq!(
            OccupationalHealthService::classify_exam_expiry_level(90),
            ExamExpiryWarningLevel::Notice
        );
    }

    #[test]
    fn test_classify_exam_expiry_level_none() {
        assert_eq!(
            OccupationalHealthService::classify_exam_expiry_level(91),
            ExamExpiryWarningLevel::None
        );
        assert_eq!(
            OccupationalHealthService::classify_exam_expiry_level(365),
            ExamExpiryWarningLevel::None
        );
    }

    #[test]
    fn test_validate_hazard_type_valid() {
        assert!(OccupationalHealthService::validate_hazard_type("chemical").is_ok());
        assert!(OccupationalHealthService::validate_hazard_type("physical").is_ok());
        assert!(OccupationalHealthService::validate_hazard_type("dust").is_ok());
        assert!(OccupationalHealthService::validate_hazard_type("biological").is_ok());
    }

    #[test]
    fn test_validate_hazard_type_invalid() {
        assert!(OccupationalHealthService::validate_hazard_type("invalid").is_err());
    }

    #[test]
    fn test_validate_exam_type_valid() {
        assert!(OccupationalHealthService::validate_exam_type("pre_employment").is_ok());
        assert!(OccupationalHealthService::validate_exam_type("in_service").is_ok());
        assert!(OccupationalHealthService::validate_exam_type("resignation").is_ok());
    }

    #[test]
    fn test_validate_exam_type_invalid() {
        assert!(OccupationalHealthService::validate_exam_type("invalid").is_err());
    }

    #[test]
    fn test_validate_exam_result_valid() {
        assert!(OccupationalHealthService::validate_exam_result("normal").is_ok());
        assert!(OccupationalHealthService::validate_exam_result("abnormal").is_ok());
        assert!(OccupationalHealthService::validate_exam_result("contraindication").is_ok());
    }

    #[test]
    fn test_validate_exam_result_invalid() {
        assert!(OccupationalHealthService::validate_exam_result("invalid").is_err());
    }

    #[test]
    fn test_validate_ppe_type_valid() {
        assert!(OccupationalHealthService::validate_ppe_type("mask").is_ok());
        assert!(OccupationalHealthService::validate_ppe_type("gloves").is_ok());
        assert!(OccupationalHealthService::validate_ppe_type("goggles").is_ok());
        assert!(OccupationalHealthService::validate_ppe_type("earplug").is_ok());
        assert!(OccupationalHealthService::validate_ppe_type("respirator").is_ok());
        assert!(OccupationalHealthService::validate_ppe_type("suit").is_ok());
    }

    #[test]
    fn test_validate_ppe_type_invalid() {
        assert!(OccupationalHealthService::validate_ppe_type("invalid").is_err());
    }

    #[test]
    fn test_get_limit_benzene() {
        let limit = OccupationalHazardLimitReference::get_limit("chemical", "苯");
        assert_eq!(limit, Some(Decimal::new(6, 0)));
    }

    #[test]
    fn test_get_limit_noise() {
        let limit = OccupationalHazardLimitReference::get_limit("physical", "噪声");
        assert_eq!(limit, Some(Decimal::new(85, 0)));
    }

    #[test]
    fn test_get_limit_unknown() {
        let limit = OccupationalHazardLimitReference::get_limit("biological", "未知");
        assert_eq!(limit, None);
    }

    #[test]
    fn test_exam_expiry_warning_level_needs_warning() {
        assert!(ExamExpiryWarningLevel::Expired.needs_warning());
        assert!(ExamExpiryWarningLevel::Critical.needs_warning());
        assert!(ExamExpiryWarningLevel::Warning.needs_warning());
        assert!(ExamExpiryWarningLevel::Notice.needs_warning());
        assert!(!ExamExpiryWarningLevel::None.needs_warning());
    }
}
