//! 劳动合同管理 Service
//!
//! V15 P1 batch-08 缺陷 21：劳动合同电子化管理
//! 依据：《劳动法》《劳动合同法》第10/19/20条
//!
//! 真实业务：
//! - 劳动合同签订/续签/终止全流程管理
//! - 试用期合规校验（长度 + 工资比例）
//! - 合同到期预警（30/60/90 天三级）
//! - 状态机：active(有效) → expiring_soon(即将到期) → expired(过期) / terminated(已终止)

use crate::models::labor_contract::{
    self, ActiveModel as ContractActiveModel, Entity as ContractEntity, Model as ContractModel,
};
use crate::utils::error::AppError;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use serde::Deserialize;
use std::sync::Arc;

/// 创建劳动合同请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateLaborContractRequest {
    pub worker_id: i32,
    pub contract_no: String,
    /// 合同类型：fixed_term(固定期限) / permanent(无固定期限) / task_based(任务制)
    pub contract_type: String,
    pub start_date: NaiveDate,
    /// 合同结束日期（无固定期限为 None）
    pub end_date: Option<NaiveDate>,
    /// 试用期结束日期（None 表示无试用期）
    pub probation_end_date: Option<NaiveDate>,
    pub probation_salary: Decimal,
    pub regular_salary: Decimal,
    pub position: Option<String>,
    pub department: Option<String>,
    pub work_location: Option<String>,
    /// 工时制度：standard(标准) / comprehensive(综合) / flexible(不定)
    pub working_hours_system: String,
    pub sign_date: NaiveDate,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 更新劳动合同请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateLaborContractRequest {
    pub position: Option<String>,
    pub department: Option<String>,
    pub work_location: Option<String>,
    pub working_hours_system: Option<String>,
    pub remarks: Option<String>,
}

/// 劳动合同查询参数
#[derive(Debug, Clone, Default, Deserialize)]
pub struct LaborContractQuery {
    pub worker_id: Option<i32>,
    pub contract_type: Option<String>,
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 合同到期预警级别
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractExpiryLevel {
    Normal,
    Warning90Days,
    Warning60Days,
    Warning30Days,
    Expired,
}

impl ContractExpiryLevel {
    pub fn desc(&self) -> &'static str {
        match self {
            Self::Normal => "正常",
            Self::Warning90Days => "90天到期预警",
            Self::Warning60Days => "60天到期预警",
            Self::Warning30Days => "30天到期预警",
            Self::Expired => "已过期",
        }
    }

    pub fn needs_warning(&self) -> bool {
        !matches!(self, Self::Normal)
    }
}

/// 合同到期预警结果
#[derive(Debug, Clone)]
pub struct ContractExpiryWarning {
    pub contract: ContractModel,
    pub level: ContractExpiryLevel,
    pub days_until_expiry: i64,
}

/// 试用期合规校验结果
#[derive(Debug, Clone)]
pub struct ProbationValidation {
    pub is_valid: bool,
    pub errors: Vec<String>,
}

pub struct LaborContractService {
    db: Arc<DatabaseConnection>,
}

impl LaborContractService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建劳动合同
    ///
    /// 业务校验（《劳动合同法》）：
    /// - 第 19 条：试用期长度限制（1-3 年合同试用期 ≤ 3 个月，3 年以上/无固定期限 ≤ 6 个月）
    /// - 第 20 条：试用期工资 ≥ 转正工资 80%
    /// - 合同编号唯一
    /// - 合同结束日期 > 开始日期（固定期限合同）
    pub async fn create(&self, req: CreateLaborContractRequest) -> Result<ContractModel, AppError> {
        Self::validate_contract_type(&req.contract_type)?;
        Self::validate_working_hours_system(&req.working_hours_system)?;

        if req.probation_salary < Decimal::ZERO || req.regular_salary < Decimal::ZERO {
            return Err(AppError::bad_request("工资不能为负"));
        }

        // 固定期限合同必须填写结束日期
        if req.contract_type == "fixed_term" && req.end_date.is_none() {
            return Err(AppError::bad_request("固定期限合同必须填写结束日期"));
        }

        // 结束日期必须晚于开始日期
        if let Some(end_date) = req.end_date {
            if end_date <= req.start_date {
                return Err(AppError::bad_request("合同结束日期必须晚于开始日期"));
            }
        }

        // 试用期合规校验
        if let Some(probation_end) = req.probation_end_date {
            let validation = Self::validate_probation(
                req.start_date,
                req.end_date,
                probation_end,
                req.probation_salary,
                req.regular_salary,
            );
            if !validation.is_valid {
                return Err(AppError::business(validation.errors.join("; ")));
            }
        }

        // 校验合同编号唯一性
        if ContractEntity::find()
            .filter(labor_contract::Column::ContractNo.eq(&req.contract_no))
            .one(&*self.db)
            .await?
            .is_some()
        {
            return Err(AppError::business(format!(
                "劳动合同编号 {} 已存在",
                req.contract_no
            )));
        }

        let now = crate::utils::date_utils::utc_now_fixed();
        let active = ContractActiveModel {
            worker_id: Set(req.worker_id),
            contract_no: Set(req.contract_no),
            contract_type: Set(req.contract_type),
            start_date: Set(req.start_date),
            end_date: Set(req.end_date),
            probation_end_date: Set(req.probation_end_date),
            probation_salary: Set(req.probation_salary),
            regular_salary: Set(req.regular_salary),
            position: Set(req.position),
            department: Set(req.department),
            work_location: Set(req.work_location),
            working_hours_system: Set(req.working_hours_system),
            sign_date: Set(req.sign_date),
            status: Set("active".to_string()),
            termination_date: Set(None),
            termination_reason: Set(None),
            remarks: Set(req.remarks),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("劳动合同创建失败: {}", e)))?;
        Ok(result)
    }

    /// 获取劳动合同详情
    pub async fn get_by_id(&self, id: i32) -> Result<ContractModel, AppError> {
        ContractEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("劳动合同 {} 不存在", id)))
    }

    /// 按工人查询当前有效合同
    pub async fn get_active_by_worker(
        &self,
        worker_id: i32,
    ) -> Result<Option<ContractModel>, AppError> {
        let contract = ContractEntity::find()
            .filter(labor_contract::Column::WorkerId.eq(worker_id))
            .filter(labor_contract::Column::Status.eq("active"))
            .one(&*self.db)
            .await?;
        Ok(contract)
    }

    /// 更新劳动合同（仅 active 状态可更新，不可修改核心条款）
    pub async fn update(
        &self,
        id: i32,
        req: UpdateLaborContractRequest,
    ) -> Result<ContractModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != "active" {
            return Err(AppError::business(format!(
                "仅有效(active)状态合同可更新，当前状态: {}",
                model.status
            )));
        }

        let mut active: ContractActiveModel = model.into();
        if let Some(v) = req.position {
            active.position = Set(Some(v));
        }
        if let Some(v) = req.department {
            active.department = Set(Some(v));
        }
        if let Some(v) = req.work_location {
            active.work_location = Set(Some(v));
        }
        if let Some(v) = req.working_hours_system {
            Self::validate_working_hours_system(&v)?;
            active.working_hours_system = Set(v);
        }
        if let Some(v) = req.remarks {
            active.remarks = Set(Some(v));
        }
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 查询劳动合同列表
    pub async fn list(
        &self,
        params: LaborContractQuery,
    ) -> Result<(Vec<ContractModel>, u64), AppError> {
        let mut query = ContractEntity::find();

        if let Some(worker_id) = params.worker_id {
            query = query.filter(labor_contract::Column::WorkerId.eq(worker_id));
        }
        if let Some(contract_type) = &params.contract_type {
            query = query.filter(labor_contract::Column::ContractType.eq(contract_type));
        }
        if let Some(status) = &params.status {
            query = query.filter(labor_contract::Column::Status.eq(status));
        }

        let total = query.clone().count(&*self.db).await?;

        let page = params.page.unwrap_or(1).max(1);
        let page_size = params.page_size.unwrap_or(20).clamp(1, 200);

        let list = query
            .order_by_desc(labor_contract::Column::SignDate)
            .offset((page - 1) * page_size)
            .limit(page_size)
            .all(&*self.db)
            .await?;

        Ok((list, total))
    }

    /// 终止劳动合同
    ///
    /// 业务规则（《劳动合同法》第36/39/40条）：
    /// - 仅 active 状态可终止
    /// - 必须填写终止日期与终止原因
    pub async fn terminate(
        &self,
        id: i32,
        termination_date: NaiveDate,
        termination_reason: String,
    ) -> Result<ContractModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != "active" {
            return Err(AppError::business(format!(
                "仅有效(active)状态合同可终止，当前状态: {}",
                model.status
            )));
        }
        if termination_reason.trim().is_empty() {
            return Err(AppError::bad_request("终止原因不能为空"));
        }

        let mut active: ContractActiveModel = model.into();
        active.status = Set("terminated".to_string());
        active.termination_date = Set(Some(termination_date));
        active.termination_reason = Set(Some(termination_reason));
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 扫描合同到期并生成预警
    ///
    /// 业务规则（《劳动合同法》第10条：建立劳动关系应当订立书面劳动合同）：
    /// - 到期前 90/60/30 天三级预警
    /// - 已过期合同状态自动更新为 expired
    /// - 无固定期限合同不参与到期预警
    pub async fn scan_expiry_warnings(&self) -> Result<Vec<ContractExpiryWarning>, AppError> {
        let today = chrono::Local::now().date_naive();
        let active_contracts = ContractEntity::find()
            .filter(labor_contract::Column::Status.eq("active"))
            .filter(labor_contract::Column::EndDate.is_not_null())
            .all(&*self.db)
            .await?;

        let mut warnings = Vec::new();
        for contract in active_contracts {
            let end_date = match contract.end_date {
                Some(d) => d,
                None => continue,
            };
            let days_until_expiry = (end_date - today).num_days();
            let level = Self::classify_expiry_level(days_until_expiry);

            if level.needs_warning() {
                // 已过期的合同自动更新状态
                if level == ContractExpiryLevel::Expired {
                    let mut active: ContractActiveModel = contract.clone().into();
                    active.status = Set("expired".to_string());
                    active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
                    let _ = active.update(&*self.db).await;
                }

                warnings.push(ContractExpiryWarning {
                    contract,
                    level,
                    days_until_expiry,
                });
            }
        }

        // 按剩余天数升序排列（最紧急的在前）
        warnings.sort_by_key(|w| w.days_until_expiry);
        Ok(warnings)
    }

    /// 试用期合规校验（纯函数）
    ///
    /// 业务规则（《劳动合同法》）：
    /// - 第 19 条：合同期 < 1 年 → 试用期 ≤ 1 个月；1-3 年 → ≤ 2 个月；≥ 3 年或无固定期限 → ≤ 6 个月
    /// - 第 20 条：试用期工资 ≥ 转正工资 80%
    pub fn validate_probation(
        start_date: NaiveDate,
        end_date: Option<NaiveDate>,
        probation_end: NaiveDate,
        probation_salary: Decimal,
        regular_salary: Decimal,
    ) -> ProbationValidation {
        let mut errors: Vec<String> = Vec::new();

        // 试用期必须晚于合同开始日期
        if probation_end <= start_date {
            errors.push("试用期结束日期必须晚于合同开始日期".to_string());
        }

        // 试用期长度校验
        let probation_days = (probation_end - start_date).num_days();
        let max_probation_days = match end_date {
            Some(end) => {
                let contract_days = (end - start_date).num_days();
                if contract_days < 365 {
                    30 // < 1 年：1 个月
                } else if contract_days < 365 * 3 {
                    60 // 1-3 年：2 个月
                } else {
                    180 // ≥ 3 年：6 个月
                }
            }
            None => 180, // 无固定期限：6 个月
        };

        if probation_days > max_probation_days {
            errors.push(format!(
                "试用期长度 {} 天超过法定上限 {} 天（《劳动合同法》第19条）",
                probation_days, max_probation_days
            ));
        }

        // 试用期工资校验：≥ 转正工资 80%
        if regular_salary > Decimal::ZERO {
            let min_probation_salary = regular_salary * Decimal::new(8, 1); // 0.8
            if probation_salary < min_probation_salary {
                errors.push(format!(
                    "试用期工资 {} 低于转正工资的 80% {}（《劳动合同法》第20条）",
                    probation_salary, min_probation_salary
                ));
            }
        }

        ProbationValidation {
            is_valid: errors.is_empty(),
            errors,
        }
    }

    /// 根据剩余天数判定预警级别（纯函数）
    pub fn classify_expiry_level(days_until_expiry: i64) -> ContractExpiryLevel {
        if days_until_expiry < 0 {
            ContractExpiryLevel::Expired
        } else if days_until_expiry <= 30 {
            ContractExpiryLevel::Warning30Days
        } else if days_until_expiry <= 60 {
            ContractExpiryLevel::Warning60Days
        } else if days_until_expiry <= 90 {
            ContractExpiryLevel::Warning90Days
        } else {
            ContractExpiryLevel::Normal
        }
    }

    /// 校验合同类型
    fn validate_contract_type(contract_type: &str) -> Result<(), AppError> {
        match contract_type {
            "fixed_term" | "permanent" | "task_based" => Ok(()),
            _ => Err(AppError::bad_request(format!(
                "无效的合同类型: {}（应为 fixed_term/permanent/task_based）",
                contract_type
            ))),
        }
    }

    /// 校验工时制度
    fn validate_working_hours_system(system: &str) -> Result<(), AppError> {
        match system {
            "standard" | "comprehensive" | "flexible" => Ok(()),
            _ => Err(AppError::bad_request(format!(
                "无效的工时制度: {}（应为 standard/comprehensive/flexible）",
                system
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_expiry_level_normal() {
        assert_eq!(
            LaborContractService::classify_expiry_level(120),
            ContractExpiryLevel::Normal
        );
        assert_eq!(
            LaborContractService::classify_expiry_level(91),
            ContractExpiryLevel::Normal
        );
    }

    #[test]
    fn test_classify_expiry_level_30_days() {
        assert_eq!(
            LaborContractService::classify_expiry_level(30),
            ContractExpiryLevel::Warning30Days
        );
        assert_eq!(
            LaborContractService::classify_expiry_level(0),
            ContractExpiryLevel::Warning30Days
        );
    }

    #[test]
    fn test_classify_expiry_level_expired() {
        assert_eq!(
            LaborContractService::classify_expiry_level(-1),
            ContractExpiryLevel::Expired
        );
    }

    #[test]
    fn test_validate_probation_short_contract() {
        // 合同期 6 个月，试用期 1 个月（合法）
        let start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 7, 1).unwrap();
        let probation_end = NaiveDate::from_ymd_opt(2026, 2, 1).unwrap();
        let result = LaborContractService::validate_probation(
            start,
            Some(end),
            probation_end,
            Decimal::new(4000, 0),
            Decimal::new(5000, 0),
        );
        assert!(result.is_valid);
    }

    #[test]
    fn test_validate_probation_too_long() {
        // 合同期 1 年，试用期 6 个月（违法，应 ≤ 2 个月）
        let start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2027, 1, 1).unwrap();
        let probation_end = NaiveDate::from_ymd_opt(2026, 7, 1).unwrap();
        let result = LaborContractService::validate_probation(
            start,
            Some(end),
            probation_end,
            Decimal::new(4000, 0),
            Decimal::new(5000, 0),
        );
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("试用期长度")));
    }

    #[test]
    fn test_validate_probation_salary_too_low() {
        // 试用期工资 < 转正工资 80%
        let start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2027, 1, 1).unwrap();
        let probation_end = NaiveDate::from_ymd_opt(2026, 2, 1).unwrap();
        let result = LaborContractService::validate_probation(
            start,
            Some(end),
            probation_end,
            Decimal::new(3000, 0), // 3000 < 5000 * 0.8 = 4000
            Decimal::new(5000, 0),
        );
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("80%")));
    }

    #[test]
    fn test_validate_contract_type_valid() {
        assert!(LaborContractService::validate_contract_type("fixed_term").is_ok());
        assert!(LaborContractService::validate_contract_type("permanent").is_ok());
        assert!(LaborContractService::validate_contract_type("task_based").is_ok());
    }

    #[test]
    fn test_validate_contract_type_invalid() {
        assert!(LaborContractService::validate_contract_type("invalid").is_err());
    }

    #[test]
    fn test_validate_working_hours_system_valid() {
        assert!(LaborContractService::validate_working_hours_system("standard").is_ok());
        assert!(LaborContractService::validate_working_hours_system("comprehensive").is_ok());
        assert!(LaborContractService::validate_working_hours_system("flexible").is_ok());
    }

    #[test]
    fn test_validate_working_hours_system_invalid() {
        assert!(LaborContractService::validate_working_hours_system("invalid").is_err());
    }
}
