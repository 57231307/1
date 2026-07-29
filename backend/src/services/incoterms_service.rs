//! Incoterms 贸易术语服务
//!
//! V15 P1 batch-19 缺陷 23.5.2/23.5.4：
//! - 缺陷 23.5.2：术语与价格构成集成（按 Incoterm 自动计算运费/保费/关税）
//! - 缺陷 23.5.4：术语使用月报（按术语统计出口量/金额）

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{DatabaseConnection, EntityTrait, FromQueryResult, Statement};
use serde::Serialize;
use std::sync::Arc;

use crate::container::AppState;
use crate::models::sales_quotation::Entity as QuotationEntity;
use crate::utils::error::AppError;
use crate::utils::incoterms::Incoterms2020;

/// V15 P1 batch-19 缺陷 23.5.2：价格构成 DTO
#[derive(Debug, Serialize)]
pub struct PriceComposition {
    pub incoterm: String,
    pub product_cost: Decimal,
    pub freight_cost: Option<Decimal>,
    pub insurance_cost: Option<Decimal>,
    pub duty_cost: Option<Decimal>,
    pub total_amount: Decimal,
}

/// V15 P1 batch-19 缺陷 23.5.4：术语使用月报 DTO
#[derive(Debug, Serialize)]
pub struct IncotermsMonthlyReport {
    pub year: i32,
    pub month: u32,
    pub items: Vec<IncotermUsageItem>,
}

/// V15 P1 batch-19 缺陷 23.5.4：术语使用统计项
#[derive(Debug, Serialize, FromQueryResult)]
pub struct IncotermUsageItem {
    pub incoterm: String,
    pub count: i64,
    pub total_amount: Decimal,
    pub freight_cost: Decimal,
    pub insurance_cost: Decimal,
    pub duty_cost: Decimal,
}

/// Incoterms 服务
pub struct IncotermsService {
    db: Arc<DatabaseConnection>,
}

impl IncotermsService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub fn from_state(state: &AppState) -> Self {
        Self {
            db: state.db.clone(),
        }
    }

    /// V15 P1 batch-19 缺陷 23.5.2：获取报价单价格构成（按 Incoterm 解析）
    pub async fn get_price_composition(
        &self,
        quotation_id: i64,
    ) -> Result<PriceComposition, AppError> {
        let quotation = QuotationEntity::find_by_id(quotation_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("报价单不存在"))?;

        let incoterm =
            Incoterms2020::from_code(&quotation.price_terms).map_err(AppError::business)?;

        Ok(PriceComposition {
            incoterm: incoterm.code().to_string(),
            product_cost: quotation.subtotal - quotation.tax_amount,
            freight_cost: quotation.freight_cost,
            insurance_cost: quotation.insurance_cost,
            duty_cost: quotation.duty_cost,
            total_amount: quotation.total_amount,
        })
    }

    /// V15 P1 batch-19 缺陷 23.5.2：按 Incoterm 计算价格构成各成本项
    /// 返回 (product_cost, freight, insurance, duty) 完整价格构成；product_cost 为基础成本始终返回。
    pub fn calculate_costs_by_incoterm(
        incoterm: Incoterms2020,
        product_cost: Decimal,
        freight_cost: Option<Decimal>,
        insurance_cost: Option<Decimal>,
        duty_cost: Option<Decimal>,
    ) -> (Decimal, Option<Decimal>, Option<Decimal>, Option<Decimal>) {
        // EXW/FCA/FAS 不含运费
        let freight = if incoterm.includes_freight() {
            freight_cost
        } else {
            None
        };
        // CIF/CIP/DDP 含保险
        let insurance = if incoterm.includes_insurance() {
            insurance_cost
        } else {
            None
        };
        // 仅 DDP 含关税
        let duty = if incoterm.requires_duty_paid() {
            duty_cost
        } else {
            None
        };
        (product_cost, freight, insurance, duty)
    }

    /// V15 P1 batch-19 缺陷 23.5.4：生成术语使用月报
    pub async fn monthly_usage_report(
        &self,
        year: i32,
        month: u32,
    ) -> Result<IncotermsMonthlyReport, AppError> {
        let start_date = NaiveDate::from_ymd_opt(year, month, 1)
            .ok_or_else(|| AppError::validation("无效的年月".to_string()))?;
        let next_month = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1)
        }
        .ok_or_else(|| AppError::validation("无效的年月".to_string()))?;

        let sql = r#"
            SELECT
                price_terms as incoterm,
                COUNT(*) as count,
                COALESCE(SUM(total_amount), 0) as total_amount,
                COALESCE(SUM(freight_cost), 0) as freight_cost,
                COALESCE(SUM(insurance_cost), 0) as insurance_cost,
                COALESCE(SUM(duty_cost), 0) as duty_cost
            FROM sales_quotations
            WHERE quotation_date >= $1 AND quotation_date < $2
            AND status = 'approved'
            GROUP BY price_terms
            ORDER BY count DESC
        "#;

        let items = IncotermUsageItem::find_by_statement(Statement::from_sql_and_values(
            sea_orm::DbBackend::Postgres,
            sql,
            vec![start_date.into(), next_month.into()],
        ))
        .all(&*self.db)
        .await?;

        Ok(IncotermsMonthlyReport { year, month, items })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_costs_exw_no_freight() {
        // EXW 不含运费/保费/关税，仅含产品成本
        let (p, f, i, d) = IncotermsService::calculate_costs_by_incoterm(
            Incoterms2020::Exw,
            Decimal::from(1000),
            Some(Decimal::from(100)),
            Some(Decimal::from(50)),
            Some(Decimal::from(200)),
        );
        assert_eq!(p, Decimal::from(1000));
        assert_eq!(f, None);
        assert_eq!(i, None);
        assert_eq!(d, None);
    }

    #[test]
    fn test_calculate_costs_cif_includes_freight_insurance() {
        // CIF 含运费和保险，不含关税
        let (p, f, i, d) = IncotermsService::calculate_costs_by_incoterm(
            Incoterms2020::Cif,
            Decimal::from(1000),
            Some(Decimal::from(100)),
            Some(Decimal::from(50)),
            Some(Decimal::from(200)),
        );
        assert_eq!(p, Decimal::from(1000));
        assert_eq!(f, Some(Decimal::from(100)));
        assert_eq!(i, Some(Decimal::from(50)));
        assert_eq!(d, None);
    }

    #[test]
    fn test_calculate_costs_ddp_includes_all() {
        // DDP 含运费/保费/关税
        let (p, f, i, d) = IncotermsService::calculate_costs_by_incoterm(
            Incoterms2020::Ddp,
            Decimal::from(1000),
            Some(Decimal::from(100)),
            Some(Decimal::from(50)),
            Some(Decimal::from(200)),
        );
        assert_eq!(p, Decimal::from(1000));
        assert_eq!(f, Some(Decimal::from(100)));
        assert_eq!(i, Some(Decimal::from(50)));
        assert_eq!(d, Some(Decimal::from(200)));
    }

    #[test]
    fn test_calculate_costs_fob_freight_only() {
        // FOB 含运费，不含保险/关税
        let (p, f, i, d) = IncotermsService::calculate_costs_by_incoterm(
            Incoterms2020::Fob,
            Decimal::from(1000),
            Some(Decimal::from(100)),
            Some(Decimal::from(50)),
            Some(Decimal::from(200)),
        );
        assert_eq!(p, Decimal::from(1000));
        assert_eq!(f, Some(Decimal::from(100)));
        assert_eq!(i, None);
        assert_eq!(d, None);
    }
}
