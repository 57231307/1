//! 社保公积金扣缴 Service
//!
//! V15 P1 batch-08 缺陷 23：社保公积金合规
//! 依据：《社会保险法》第58条 + 《住房公积金管理条例》第14条
//!
//! 真实业务：
//! - 按月计算五险一金扣缴金额（单位/个人部分）
//! - 校验缴费基数合规性（不低于当地最低基数、不高于当地最高基数）
//! - 状态机：pending(待缴) → paid(已缴) / cancelled(已撤销)
//! - 与工资服务联动：工资发放时自动生成社保扣缴记录

use crate::models::social_insurance_record::{
    self, ActiveModel as InsuranceActiveModel, Entity as InsuranceEntity, Model as InsuranceModel,
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

/// 创建社保缴纳记录请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateSocialInsuranceRequest {
    pub worker_id: i32,
    pub period_year: i32,
    pub period_month: i32,
    /// 缴费基数（应为上年度月平均工资）
    pub base_amount: Decimal,
    pub payment_date: Option<NaiveDate>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 社保缴纳记录查询参数
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SocialInsuranceQuery {
    pub worker_id: Option<i32>,
    pub period_year: Option<i32>,
    pub period_month: Option<i32>,
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 缴费基数校验结果
#[derive(Debug, Clone)]
pub struct BaseValidation {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub is_below_minimum: bool,
    pub is_above_maximum: bool,
}

/// 五险一金费率配置（默认值，可按地区调整）
#[derive(Debug, Clone)]
pub struct InsuranceRateConfig {
    /// 养老保险单位费率（默认 16%）
    pub pension_employer_rate: Decimal,
    /// 养老保险个人费率（默认 8%）
    pub pension_employee_rate: Decimal,
    /// 医疗保险单位费率（默认 8%）
    pub medical_employer_rate: Decimal,
    /// 医疗保险个人费率（默认 2%）
    pub medical_employee_rate: Decimal,
    /// 失业保险单位费率（默认 0.5%）
    pub unemployment_employer_rate: Decimal,
    /// 失业保险个人费率（默认 0.5%）
    pub unemployment_employee_rate: Decimal,
    /// 工伤保险单位费率（默认 0.4%，行业差别费率）
    pub work_injury_employer_rate: Decimal,
    /// 生育保险单位费率（默认 1%）
    pub maternity_employer_rate: Decimal,
    /// 公积金单位费率（默认 12%）
    pub housing_fund_employer_rate: Decimal,
    /// 公积金个人费率（默认 12%）
    pub housing_fund_employee_rate: Decimal,
    /// 当地最低缴费基数
    pub min_base_amount: Decimal,
    /// 当地最高缴费基数
    pub max_base_amount: Decimal,
}

impl Default for InsuranceRateConfig {
    fn default() -> Self {
        Self {
            pension_employer_rate: Decimal::new(16, 2),      // 0.16
            pension_employee_rate: Decimal::new(8, 2),       // 0.08
            medical_employer_rate: Decimal::new(8, 2),       // 0.08
            medical_employee_rate: Decimal::new(2, 2),       // 0.02
            unemployment_employer_rate: Decimal::new(5, 3),  // 0.005
            unemployment_employee_rate: Decimal::new(5, 3),  // 0.005
            work_injury_employer_rate: Decimal::new(4, 3),   // 0.004
            maternity_employer_rate: Decimal::new(1, 2),     // 0.01
            housing_fund_employer_rate: Decimal::new(12, 2), // 0.12
            housing_fund_employee_rate: Decimal::new(12, 2), // 0.12
            // 默认下限按全国社保最低基数示例（应按地区配置覆盖）
            min_base_amount: Decimal::new(4250, 0),
            max_base_amount: Decimal::new(31884, 0),
        }
    }
}

/// 五险一金计算结果
#[derive(Debug, Clone)]
pub struct InsuranceCalculationResult {
    pub pension_employer: Decimal,
    pub pension_employee: Decimal,
    pub medical_employer: Decimal,
    pub medical_employee: Decimal,
    pub unemployment_employer: Decimal,
    pub unemployment_employee: Decimal,
    pub work_injury_employer: Decimal,
    pub maternity_employer: Decimal,
    pub housing_fund_employer: Decimal,
    pub housing_fund_employee: Decimal,
    pub total_employer: Decimal,
    pub total_employee: Decimal,
}

pub struct SocialInsuranceService {
    db: Arc<DatabaseConnection>,
    /// 费率配置（生产环境应从数据库或配置中心加载）
    rate_config: InsuranceRateConfig,
}

impl SocialInsuranceService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            db,
            rate_config: InsuranceRateConfig::default(),
        }
    }

    /// 使用自定义费率配置构造
    pub fn with_rate_config(db: Arc<DatabaseConnection>, rate_config: InsuranceRateConfig) -> Self {
        Self { db, rate_config }
    }

    /// 创建社保缴纳记录（自动计算五险一金）
    ///
    /// 业务规则（《社会保险法》第58条 + 《住房公积金管理条例》第14条）：
    /// - 用工之日起 30 日内办理社保登记
    /// - 缴费基数应为上年度月平均工资，不得低于当地最低基数或高于最高基数
    /// - 五险一金单位/个人部分按费率自动计算
    pub async fn create(
        &self,
        req: CreateSocialInsuranceRequest,
    ) -> Result<InsuranceModel, AppError> {
        Self::validate_period(req.period_year, req.period_month)?;
        if req.base_amount <= Decimal::ZERO {
            return Err(AppError::bad_request("缴费基数必须大于 0"));
        }

        // 缴费基数合规校验
        let validation = self.validate_base_amount(req.base_amount);
        if !validation.is_valid {
            return Err(AppError::business(validation.errors.join("; ")));
        }

        // 校验同期间同工人不重复缴纳
        if InsuranceEntity::find()
            .filter(social_insurance_record::Column::WorkerId.eq(req.worker_id))
            .filter(social_insurance_record::Column::PeriodYear.eq(req.period_year))
            .filter(social_insurance_record::Column::PeriodMonth.eq(req.period_month))
            .filter(
                social_insurance_record::Column::Status
                    .is_in(["pending".to_string(), "paid".to_string()]),
            )
            .one(&*self.db)
            .await?
            .is_some()
        {
            return Err(AppError::business(format!(
                "工人 {} 在 {}-{} 已存在社保记录",
                req.worker_id, req.period_year, req.period_month
            )));
        }

        // 计算五险一金
        let calc = Self::calculate_insurance(req.base_amount, &self.rate_config);

        let now = crate::utils::date_utils::utc_now_fixed();
        let active = InsuranceActiveModel {
            worker_id: Set(req.worker_id),
            period_year: Set(req.period_year),
            period_month: Set(req.period_month),
            base_amount: Set(req.base_amount),
            pension_employer: Set(calc.pension_employer),
            pension_employee: Set(calc.pension_employee),
            medical_employer: Set(calc.medical_employer),
            medical_employee: Set(calc.medical_employee),
            unemployment_employer: Set(calc.unemployment_employer),
            unemployment_employee: Set(calc.unemployment_employee),
            work_injury_employer: Set(calc.work_injury_employer),
            maternity_employer: Set(calc.maternity_employer),
            housing_fund_employer: Set(calc.housing_fund_employer),
            housing_fund_employee: Set(calc.housing_fund_employee),
            total_employer: Set(calc.total_employer),
            total_employee: Set(calc.total_employee),
            status: Set(if req.payment_date.is_some() {
                "paid".to_string()
            } else {
                "pending".to_string()
            }),
            payment_date: Set(req.payment_date),
            remarks: Set(req.remarks),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("社保记录创建失败: {}", e)))?;
        Ok(result)
    }

    /// 获取社保记录详情
    pub async fn get_by_id(&self, id: i32) -> Result<InsuranceModel, AppError> {
        InsuranceEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("社保记录 {} 不存在", id)))
    }

    /// 查询社保记录列表
    pub async fn list(
        &self,
        params: SocialInsuranceQuery,
    ) -> Result<(Vec<InsuranceModel>, u64), AppError> {
        let mut query = InsuranceEntity::find();

        if let Some(worker_id) = params.worker_id {
            query = query.filter(social_insurance_record::Column::WorkerId.eq(worker_id));
        }
        if let Some(year) = params.period_year {
            query = query.filter(social_insurance_record::Column::PeriodYear.eq(year));
        }
        if let Some(month) = params.period_month {
            query = query.filter(social_insurance_record::Column::PeriodMonth.eq(month));
        }
        if let Some(status) = &params.status {
            query = query.filter(social_insurance_record::Column::Status.eq(status));
        }

        let total = query.clone().count(&*self.db).await?;

        let page = params.page.unwrap_or(1).max(1);
        let page_size = params.page_size.unwrap_or(20).clamp(1, 200);

        let list = query
            .order_by_desc(social_insurance_record::Column::PeriodYear)
            .order_by_desc(social_insurance_record::Column::PeriodMonth)
            .offset((page - 1) * page_size)
            .limit(page_size)
            .all(&*self.db)
            .await?;

        Ok((list, total))
    }

    /// 按工人查询期间社保记录
    pub async fn get_by_worker_period(
        &self,
        worker_id: i32,
        period_year: i32,
        period_month: i32,
    ) -> Result<Option<InsuranceModel>, AppError> {
        let record = InsuranceEntity::find()
            .filter(social_insurance_record::Column::WorkerId.eq(worker_id))
            .filter(social_insurance_record::Column::PeriodYear.eq(period_year))
            .filter(social_insurance_record::Column::PeriodMonth.eq(period_month))
            .one(&*self.db)
            .await?;
        Ok(record)
    }

    /// 确认缴纳（pending → paid）
    pub async fn mark_paid(
        &self,
        id: i32,
        payment_date: NaiveDate,
    ) -> Result<InsuranceModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != "pending" {
            return Err(AppError::business(format!(
                "仅待缴(pending)状态可确认缴纳，当前状态: {}",
                model.status
            )));
        }

        let mut active: InsuranceActiveModel = model.into();
        active.status = Set("paid".to_string());
        active.payment_date = Set(Some(payment_date));
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 撤销社保记录（仅 pending 状态可撤销）
    pub async fn cancel(&self, id: i32) -> Result<InsuranceModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != "pending" {
            return Err(AppError::business(format!(
                "仅待缴(pending)状态可撤销，当前状态: {}",
                model.status
            )));
        }

        let mut active: InsuranceActiveModel = model.into();
        active.status = Set("cancelled".to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 验证缴费基数（使用实例费率配置）
    pub fn validate_base_amount(&self, base_amount: Decimal) -> BaseValidation {
        Self::validate_base_amount_with_config(base_amount, &self.rate_config)
    }

    /// 静态验证缴费基数（使用默认费率配置，供无 DB 场景与测试使用）
    pub fn validate_base_amount_static(base_amount: Decimal) -> BaseValidation {
        let config = InsuranceRateConfig::default();
        Self::validate_base_amount_with_config(base_amount, &config)
    }

    /// 验证缴费基数内部实现：缴费基数需在 [min, max] 区间内
    fn validate_base_amount_with_config(
        base_amount: Decimal,
        config: &InsuranceRateConfig,
    ) -> BaseValidation {
        let mut errors: Vec<String> = Vec::new();
        let mut is_below_minimum = false;
        let mut is_above_maximum = false;

        if base_amount < config.min_base_amount {
            is_below_minimum = true;
            errors.push(format!(
                "缴费基数 {} 低于当地最低基数 {}（《社会保险法》第60条）",
                base_amount, config.min_base_amount
            ));
        }

        if base_amount > config.max_base_amount {
            is_above_maximum = true;
            errors.push(format!(
                "缴费基数 {} 高于当地最高基数 {}（超出部分不计入缴费基数）",
                base_amount, config.max_base_amount
            ));
        }

        BaseValidation {
            is_valid: errors.is_empty(),
            errors,
            is_below_minimum,
            is_above_maximum,
        }
    }

    /// 计算五险一金扣缴金额（纯函数）
    ///
    /// 业务规则：
    /// - 养老保险：单位 16% + 个人 8%
    /// - 医疗保险：单位 8% + 个人 2%
    /// - 失业保险：单位 0.5% + 个人 0.5%
    /// - 工伤保险：单位 0.4%（个人不缴）
    /// - 生育保险：单位 1%（个人不缴）
    /// - 公积金：单位 12% + 个人 12%
    pub fn calculate_insurance(
        base_amount: Decimal,
        config: &InsuranceRateConfig,
    ) -> InsuranceCalculationResult {
        let pension_employer = base_amount * config.pension_employer_rate;
        let pension_employee = base_amount * config.pension_employee_rate;
        let medical_employer = base_amount * config.medical_employer_rate;
        let medical_employee = base_amount * config.medical_employee_rate;
        let unemployment_employer = base_amount * config.unemployment_employer_rate;
        let unemployment_employee = base_amount * config.unemployment_employee_rate;
        let work_injury_employer = base_amount * config.work_injury_employer_rate;
        let maternity_employer = base_amount * config.maternity_employer_rate;
        let housing_fund_employer = base_amount * config.housing_fund_employer_rate;
        let housing_fund_employee = base_amount * config.housing_fund_employee_rate;

        let total_employer = pension_employer
            + medical_employer
            + unemployment_employer
            + work_injury_employer
            + maternity_employer
            + housing_fund_employer;

        let total_employee =
            pension_employee + medical_employee + unemployment_employee + housing_fund_employee;

        InsuranceCalculationResult {
            pension_employer,
            pension_employee,
            medical_employer,
            medical_employee,
            unemployment_employer,
            unemployment_employee,
            work_injury_employer,
            maternity_employer,
            housing_fund_employer,
            housing_fund_employee,
            total_employer,
            total_employee,
        }
    }

    /// 校验期间合法性
    fn validate_period(year: i32, month: i32) -> Result<(), AppError> {
        if !(2000..=2100).contains(&year) {
            return Err(AppError::bad_request(format!("无效的年度: {}", year)));
        }
        if !(1..=12).contains(&month) {
            return Err(AppError::bad_request(format!("无效的月份: {}", month)));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_insurance_default_rates() {
        let config = InsuranceRateConfig::default();
        let result = SocialInsuranceService::calculate_insurance(Decimal::new(10000, 0), &config);

        // 养老保险：单位 1600 + 个人 800
        assert_eq!(result.pension_employer, Decimal::new(1600, 0));
        assert_eq!(result.pension_employee, Decimal::new(800, 0));

        // 医疗保险：单位 800 + 个人 200
        assert_eq!(result.medical_employer, Decimal::new(800, 0));
        assert_eq!(result.medical_employee, Decimal::new(200, 0));

        // 公积金：单位 1200 + 个人 1200
        assert_eq!(result.housing_fund_employer, Decimal::new(1200, 0));
        assert_eq!(result.housing_fund_employee, Decimal::new(1200, 0));

        // 单位合计：1600+800+50+40+100+1200 = 3790
        assert_eq!(result.total_employer, Decimal::new(3790, 0));

        // 个人合计：800+200+50+1200 = 2250
        assert_eq!(result.total_employee, Decimal::new(2250, 0));
    }

    #[test]
    fn test_validate_base_amount_normal() {
        // validate_base_amount 是纯函数，不依赖数据库连接
        let validation =
            SocialInsuranceService::validate_base_amount_static(Decimal::new(10000, 0));
        assert!(validation.is_valid);
        assert!(!validation.is_below_minimum);
        assert!(!validation.is_above_maximum);
    }

    #[test]
    fn test_validate_period_valid() {
        assert!(SocialInsuranceService::validate_period(2026, 1).is_ok());
        assert!(SocialInsuranceService::validate_period(2026, 12).is_ok());
    }

    #[test]
    fn test_validate_period_invalid_year() {
        assert!(SocialInsuranceService::validate_period(1999, 1).is_err());
        assert!(SocialInsuranceService::validate_period(2101, 1).is_err());
    }

    #[test]
    fn test_validate_period_invalid_month() {
        assert!(SocialInsuranceService::validate_period(2026, 0).is_err());
        assert!(SocialInsuranceService::validate_period(2026, 13).is_err());
    }
}
