use rust_decimal::Decimal;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, QuerySelect, sea_query::Expr, ColumnTrait};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::models::{finance_invoice, finance_payment};

#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceSheet {
    pub assets: Vec<ReportItem>,
    pub total_assets: Decimal,
    pub liabilities: Vec<ReportItem>,
    pub total_liabilities: Decimal,
    pub equity: Vec<ReportItem>,
    pub total_equity: Decimal,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IncomeStatement {
    pub revenue: Vec<ReportItem>,
    pub total_revenue: Decimal,
    pub expenses: Vec<ReportItem>,
    pub total_expenses: Decimal,
    pub net_income: Decimal,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReportItem {
    pub name: String,
    pub amount: Decimal,
}

pub struct FinanceReportService {
    db: Arc<DatabaseConnection>,
}

impl FinanceReportService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 资产负债表 (简化版)
    pub async fn get_balance_sheet(&self) -> Result<BalanceSheet, sea_orm::DbErr> {
        // 简化实现：资产 = 应收账款, 负债 = 应付账款, 所有者权益 = 资产 - 负债
        
        // 1. 应收账款 (未结清的发票)
        let ar_total = finance_invoice::Entity::find()
            .filter(finance_invoice::Column::Status.ne("CANCELLED"))
            .filter(finance_invoice::Column::Status.ne("COMPLETED"))
            .select_only()
            .column_as(Expr::col(finance_invoice::Column::TotalAmount).sum(), "unpaid")
            .into_tuple::<Option<Decimal>>()
            .one(self.db.as_ref())
            .await?
            .flatten()
            .unwrap_or(Decimal::ZERO);

        // 2. 现金/银行存款 (收款总额)
        let total_received = finance_payment::Entity::find()
            .filter(finance_payment::Column::Status.eq("COMPLETED"))
            .select_only()
            .column_as(Expr::col(finance_payment::Column::Amount).sum(), "total")
            .into_tuple::<Option<Decimal>>()
            .one(self.db.as_ref())
            .await?
            .flatten()
            .unwrap_or(Decimal::ZERO);

        let cash_total = total_received;
        let total_assets = ar_total + cash_total;
        let total_liabilities = Decimal::ZERO;
        let total_equity = total_assets - total_liabilities;

        Ok(BalanceSheet {
            assets: vec![
                ReportItem { name: "应收账款".to_string(), amount: ar_total },
                ReportItem { name: "现金及银行存款".to_string(), amount: cash_total },
            ],
            total_assets,
            liabilities: vec![],
            total_liabilities,
            equity: vec![
                ReportItem { name: "所有者权益".to_string(), amount: total_equity },
            ],
            total_equity,
        })
    }

    /// 利润表 (简化版)
    pub async fn get_income_statement(&self, _start_date: chrono::NaiveDate, _end_date: chrono::NaiveDate) -> Result<IncomeStatement, sea_orm::DbErr> {
        // 简化实现
        let total_revenue = finance_invoice::Entity::find()
            .filter(finance_invoice::Column::Status.eq("COMPLETED"))
            .select_only()
            .column_as(Expr::col(finance_invoice::Column::TotalAmount).sum(), "total")
            .into_tuple::<Option<Decimal>>()
            .one(self.db.as_ref())
            .await?
            .flatten()
            .unwrap_or(Decimal::ZERO);

        let total_expenses = finance_payment::Entity::find()
            .filter(finance_payment::Column::Status.eq("COMPLETED"))
            .select_only()
            .column_as(Expr::col(finance_payment::Column::Amount).sum(), "total")
            .into_tuple::<Option<Decimal>>()
            .one(self.db.as_ref())
            .await?
            .flatten()
            .unwrap_or(Decimal::ZERO);

        let net_income = total_revenue - total_expenses;

        Ok(IncomeStatement {
            revenue: vec![
                ReportItem { name: "营业收入".to_string(), amount: total_revenue },
            ],
            total_revenue,
            expenses: vec![
                ReportItem { name: "营业支出".to_string(), amount: total_expenses },
            ],
            total_expenses,
            net_income,
        })
    }
}
