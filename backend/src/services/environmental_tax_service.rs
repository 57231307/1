//! 环保税核算 Service
//!
//! V15 P1 batch-08 缺陷 15：环保税核算
//! 依据：《环境保护税法》印染企业废水/废气/固废排放
//!
//! 真实业务：
//! - 按月记录污染物排放量
//! - 计算污染当量数（环保税计税依据）
//! - 计算应缴环保税额
//! - 生成环保税申报表

use crate::constants::environmental_tax::statutory_pollution_equivalent;
use crate::models::pollutant_discharge_record::{
    self, ActiveModel as DischargeActiveModel, Entity as DischargeEntity, Model as DischargeModel,
};
use crate::utils::error::AppError;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::Deserialize;
use std::sync::Arc;
use tracing::warn;

/// 创建污染物排放记录请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateDischargeRecordRequest {
    pub discharge_type: String,
    pub pollutant_name: String,
    pub discharge_amount: Decimal,
    pub discharge_unit: Option<String>,
    pub concentration: Option<Decimal>,
    pub concentration_unit: Option<String>,
    pub period_year: i32,
    pub period_month: i32,
    pub monitoring_point: Option<String>,
    pub remarks: Option<String>,
}

/// 环保税计算结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct EnvironmentalTaxResult {
    pub pollutant_name: String,
    pub tax_unit_equivalent: Decimal,
    pub tax_amount: Decimal,
}

pub struct EnvironmentalTaxService {
    db: Arc<DatabaseConnection>,
    /// 环保税适用税额（元/污染当量，地方在法定幅度 1.2–12 元内确定的**可变配置值**）。
    /// 来源为部署配置 `AppSettings::env_tax_rate_per_equivalent`（handler 侧经
    /// `config::settings::global_env_tax_rate_per_equivalent()` 注入），本服务内部
    /// **不保留任何默认税额**：`None` = 未配置，所有计税路径显式失败。
    env_tax_rate_per_equivalent: Option<Decimal>,
}

impl EnvironmentalTaxService {
    pub fn new(db: Arc<DatabaseConnection>, env_tax_rate_per_equivalent: Option<Decimal>) -> Self {
        Self {
            db,
            env_tax_rate_per_equivalent,
        }
    }

    /// 创建污染物排放记录（自动计算环保税）
    ///
    /// 业务规则（《环境保护税法》附表口径）：污染当量数 = 排放量 ÷ 污染当量值；
    /// 应缴税额 = 污染当量数 × 适用税额（水污染物每污染当量 1.2-12 元，幅度内由地方确定）。
    /// 排放浓度**不进入当量数公式**（浓度只参与两处：① 排放量无法直接量测时按
    /// 水量×浓度折算排放量，折算发生在本服务的入参之前，`discharge_amount` 已是排放量；
    /// ② 浓度低于排放标准 30%/50% 的减征 25%/50% 判定，属申报环节，本服务当前仅将
    /// `concentration` 随单落库登记，未据此减免）。计算实现见 [`Self::calculate_tax`]。
    pub async fn create_discharge_record(
        &self,
        req: CreateDischargeRecordRequest,
        user_id: i32,
    ) -> Result<DischargeModel, AppError> {
        Self::validate_discharge_type(&req.discharge_type)?;
        if req.discharge_amount < Decimal::ZERO {
            return Err(AppError::bad_request("排放量不能为负"));
        }

        // 计算污染当量数与税额（未登记污染物/未配置适用税额时显式失败，落库前拒绝）
        let (tax_unit_equivalent, tax_amount) = Self::calculate_tax(
            &req.discharge_type,
            &req.pollutant_name,
            req.discharge_amount,
            req.concentration,
            self.env_tax_rate_per_equivalent,
        )?;

        let now = crate::utils::date_utils::utc_now_fixed();
        let active = DischargeActiveModel {
            discharge_type: Set(req.discharge_type),
            pollutant_name: Set(req.pollutant_name),
            discharge_amount: Set(req.discharge_amount),
            discharge_unit: Set(req.discharge_unit.unwrap_or_else(|| "kg".to_string())),
            concentration: Set(req.concentration),
            concentration_unit: Set(req.concentration_unit),
            tax_unit_equivalent: Set(Some(tax_unit_equivalent)),
            tax_amount: Set(tax_amount),
            period_year: Set(req.period_year),
            period_month: Set(req.period_month),
            monitoring_point: Set(req.monitoring_point),
            remarks: Set(req.remarks),
            // 建单人取服务端会话（handler 传入），请求体不承载身份
            created_by: Set(Some(user_id)),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("污染物排放记录创建失败: {}", e)))?;
        Ok(result)
    }

    /// 按期间查询污染物排放记录
    pub async fn list_by_period(
        &self,
        period_year: i32,
        period_month: i32,
    ) -> Result<Vec<DischargeModel>, AppError> {
        let list = DischargeEntity::find()
            .filter(pollutant_discharge_record::Column::PeriodYear.eq(period_year))
            .filter(pollutant_discharge_record::Column::PeriodMonth.eq(period_month))
            .all(&*self.db)
            .await?;
        Ok(list)
    }

    /// 生成环保税申报表（按期间汇总）
    pub async fn generate_tax_declaration(
        &self,
        period_year: i32,
        period_month: i32,
    ) -> Result<Vec<EnvironmentalTaxResult>, AppError> {
        let records = self.list_by_period(period_year, period_month).await?;

        // 按污染物名称汇总
        use std::collections::HashMap;
        let mut summary: HashMap<String, (Decimal, Decimal)> = HashMap::new();
        for record in records {
            let entry = summary
                .entry(record.pollutant_name.clone())
                .or_insert((Decimal::ZERO, Decimal::ZERO));
            entry.0 += record.tax_unit_equivalent.unwrap_or(Decimal::ZERO);
            entry.1 += record.tax_amount;
        }

        let result: Vec<EnvironmentalTaxResult> = summary
            .into_iter()
            .map(|(name, (equivalent, tax))| EnvironmentalTaxResult {
                pollutant_name: name,
                tax_unit_equivalent: equivalent,
                tax_amount: tax,
            })
            .collect();

        Ok(result)
    }

    /// 计算环保税（纯函数；适用税额由调用方从部署配置显式注入，内部不读任何默认值）
    ///
    /// 业务规则（《环境保护税法》附表，决策定案 分源口径）
    /// - **污染当量值＝法定不可调值**，唯一来源 [`crate::constants::environmental_tax`]
    ///   （COD=1kg、氨氮=0.5kg、VOCs=0.5kg、污泥=1吨；排放量以 kg 口径进入公式）。
    ///   未在该表登记的污染物一律返回校验错误显式拒绝计税，**禁止按 1kg 或任何估算值兜底**
    ///   继续算出看似正常的税额。
    /// - **适用税额＝地方在法定幅度内确定的可变值**（每污染当量 1.2–12 元、由省级确定），
    ///   来自配置 `env_tax_rate_per_equivalent` / 环境变量 `ENV_TAX_RATE_PER_EQUIVALENT`；
    ///   未配置（`None`）时记 warn 并显式返回业务错误，**不存在硬编码默认税额**。
    /// - 污染当量数 = 排放量 ÷ 污染当量值；应缴税额 = 污染当量数 × 适用税额。
    ///
    /// `_concentration`（排放浓度）按法定口径不参与当量数计算：入参 `discharge_amount` 即已是排放量
    /// （无法直接量测时由上游按水量×浓度折算后传入），浓度本身仅随单落库登记
    /// （`pollutant_discharge_record.concentration`，供达标/减征判定取数）。
    pub fn calculate_tax(
        discharge_type: &str,
        pollutant_name: &str,
        discharge_amount: Decimal,
        _concentration: Option<Decimal>,
        tax_rate_per_equivalent: Option<Decimal>,
    ) -> Result<(Decimal, Decimal), AppError> {
        let _ = discharge_type; // 排放类型仅用于分类，不影响计算

        // 污染当量值（法定不可调）：未登记 → 显式拒绝，不估、不按默认值继续算
        let pollution_equivalent_value = statutory_pollution_equivalent(pollutant_name)
            .ok_or_else(|| {
                AppError::validation_displayable("该污染物暂未配置法定污染当量值，请先登记后再计税")
            })?;
        // 法定当量值必须为正数（登记表数值缺陷属内部错误，显式失败而非吞掉继续算）
        if pollution_equivalent_value <= Decimal::ZERO {
            return Err(AppError::internal(format!(
                "污染物 {pollutant_name} 的法定污染当量值登记异常（须为正数），已拒绝计税"
            )));
        }

        // 适用税额（地方可变值，来自配置）：未配置 → 显式失败 + warn，禁止取默认值继续算
        let Some(tax_rate) = tax_rate_per_equivalent else {
            warn!(
                pollutant = %pollutant_name,
                "环保税适用税额未配置（env_tax_rate_per_equivalent / ENV_TAX_RATE_PER_EQUIVALENT），拒绝计税"
            );
            return Err(AppError::business_displayable(
                "环保税适用税额未配置，请完成部署配置后重新计税",
            ));
        };

        // 污染当量数 = 排放量 / 污染当量值
        let tax_unit_equivalent = discharge_amount / pollution_equivalent_value;

        // 应缴税额 = 污染当量数 × 适用税额
        let tax_amount = tax_unit_equivalent * tax_rate;

        Ok((tax_unit_equivalent, tax_amount))
    }

    /// 校验排放类型
    pub fn validate_discharge_type(discharge_type: &str) -> Result<(), AppError> {
        match discharge_type {
            "wastewater" | "exhaust" | "solid_waste" => Ok(()),
            _ => Err(AppError::bad_request(format!(
                "无效的排放类型: {}（应为 wastewater/exhaust/solid_waste）",
                discharge_type
            ))),
        }
    }
}
