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

use crate::models::pollutant_discharge_record::{
    self, ActiveModel as DischargeActiveModel, Entity as DischargeEntity, Model as DischargeModel,
};
use crate::utils::error::AppError;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    Set,
};
use serde::Deserialize;
use std::sync::Arc;

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
    pub created_by: Option<i32>,
}

/// 环保税计算结果
#[derive(Debug, Clone)]
pub struct EnvironmentalTaxResult {
    pub pollutant_name: String,
    pub tax_unit_equivalent: Decimal,
    pub tax_amount: Decimal,
}

pub struct EnvironmentalTaxService {
    db: Arc<DatabaseConnection>,
}

impl EnvironmentalTaxService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建污染物排放记录（自动计算环保税）
    ///
    /// 业务规则（《环境保护税法》）：
    /// - 污染当量数 = 排放浓度 × 排放量 / 污染当量值
    /// - 应缴税额 = 污染当量数 × 适用税额（每污染当量 1.2-12 元）
    pub async fn create_discharge_record(
        &self,
        req: CreateDischargeRecordRequest,
    ) -> Result<DischargeModel, AppError> {
        Self::validate_discharge_type(&req.discharge_type)?;
        if req.discharge_amount < Decimal::ZERO {
            return Err(AppError::bad_request("排放量不能为负"));
        }

        // 计算污染当量数与税额
        let (tax_unit_equivalent, tax_amount) =
            Self::calculate_tax(&req.discharge_type, &req.pollutant_name, req.discharge_amount, req.concentration);

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
            created_by: Set(req.created_by),
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

    /// 计算环保税（纯函数）
    ///
    /// 业务规则（《环境保护税法》附表）：
    /// - 污染当量值：COD=1kg、氨氮=0.5kg、VOCs=0.5kg、污泥=1吨
    /// - 适用税额：每污染当量 1.2 元（最低）- 12 元（最高），取中值 2.4 元
    /// - 污染当量数 = 排放量 / 污染当量值
    /// - 应缴税额 = 污染当量数 × 适用税额
    fn calculate_tax(
        discharge_type: &str,
        pollutant_name: &str,
        discharge_amount: Decimal,
        _concentration: Option<Decimal>,
    ) -> (Decimal, Decimal) {
        // 污染当量值（kg/吨）
        let pollution_equivalent_value = match pollutant_name {
            "COD" | "cod" => Decimal::new(1, 0),     // 1kg
            "氨氮" | "NH3-N" => Decimal::new(5, 1),   // 0.5kg
            "VOCs" | "vocs" => Decimal::new(5, 1),   // 0.5kg
            "污泥" | "sludge" => Decimal::new(1000, 0), // 1吨=1000kg
            _ => Decimal::new(1, 0),                 // 默认 1kg
        };

        // 适用税额（元/污染当量）：取中值 2.4 元
        let tax_rate = Decimal::new(24, 1); // 2.4

        // 污染当量数 = 排放量 / 污染当量值
        let tax_unit_equivalent = if pollution_equivalent_value > Decimal::ZERO {
            discharge_amount / pollution_equivalent_value
        } else {
            Decimal::ZERO
        };

        // 应缴税额 = 污染当量数 × 适用税额
        let tax_amount = tax_unit_equivalent * tax_rate;

        let _ = discharge_type; // 排放类型仅用于分类，不影响计算
        (tax_unit_equivalent, tax_amount)
    }

    /// 校验排放类型
    fn validate_discharge_type(discharge_type: &str) -> Result<(), AppError> {
        match discharge_type {
            "wastewater" | "exhaust" | "solid_waste" => Ok(()),
            _ => Err(AppError::bad_request(format!(
                "无效的排放类型: {}（应为 wastewater/exhaust/solid_waste）",
                discharge_type
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_tax_cod() {
        let (equivalent, tax) = EnvironmentalTaxService::calculate_tax(
            "wastewater",
            "COD",
            Decimal::new(100, 0), // 100kg
            None,
        );
        // 污染当量数 = 100 / 1 = 100
        assert_eq!(equivalent, Decimal::new(100, 0));
        // 应缴税额 = 100 × 2.4 = 240
        assert_eq!(tax, Decimal::new(240, 0));
    }

    #[test]
    fn test_calculate_tax_vocs() {
        let (equivalent, tax) = EnvironmentalTaxService::calculate_tax(
            "exhaust",
            "VOCs",
            Decimal::new(50, 0), // 50kg
            None,
        );
        // 污染当量数 = 50 / 0.5 = 100
        assert_eq!(equivalent, Decimal::new(100, 0));
        // 应缴税额 = 100 × 2.4 = 240
        assert_eq!(tax, Decimal::new(240, 0));
    }

    #[test]
    fn test_validate_discharge_type_valid() {
        assert!(EnvironmentalTaxService::validate_discharge_type("wastewater").is_ok());
        assert!(EnvironmentalTaxService::validate_discharge_type("exhaust").is_ok());
        assert!(EnvironmentalTaxService::validate_discharge_type("solid_waste").is_ok());
    }

    #[test]
    fn test_validate_discharge_type_invalid() {
        assert!(EnvironmentalTaxService::validate_discharge_type("invalid").is_err());
    }
}
