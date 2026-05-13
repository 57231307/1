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
        
        // 1. 应收账款 (未结清的AR发票)
        let ar_total = finance_invoice::Entity::find()
            .filter(finance_invoice::Column::IsDeleted.eq(false))
            .filter(finance_invoice::Column::InvoiceType.eq("AR"))
            .filter(finance_invoice::Column::Status.ne("CANCELLED"))
            .filter(finance_invoice::Column::Status.ne("COMPLETED"))
            .select_only()
            .column_as(Expr::col(finance_invoice::Column::TotalAmount).sum(), "unpaid")
            .into_tuple::<Option<Decimal>>()
            .one(self.db.as_ref())
            .await?
            .flatten()
            .unwrap_or(Decimal::ZERO);

        // 2. 应付账款 (未结清的AP发票)
        let ap_total = finance_invoice::Entity::find()
            .filter(finance_invoice::Column::IsDeleted.eq(false))
            .filter(finance_invoice::Column::InvoiceType.eq("AP"))
            .filter(finance_invoice::Column::Status.ne("CANCELLED"))
            .filter(finance_invoice::Column::Status.ne("COMPLETED"))
            .select_only()
            .column_as(Expr::col(finance_invoice::Column::TotalAmount).sum(), "unpaid")
            .into_tuple::<Option<Decimal>>()
            .one(self.db.as_ref())
            .await?
            .flatten()
            .unwrap_or(Decimal::ZERO);

        // 3. 现金/银行存款 (收款总额 - 付款总额)
        let total_received = finance_payment::Entity::find()
            .filter(finance_payment::Column::IsDeleted.eq(false))
            .filter(finance_payment::Column::PaymentType.eq("RECEIPT"))
            .filter(finance_payment::Column::Status.eq("COMPLETED"))
            .select_only()
            .column_as(Expr::col(finance_payment::Column::Amount).sum(), "total")
            .into_tuple::<Option<Decimal>>()
            .one(self.db.as_ref())
            .await?
            .flatten()
            .unwrap_or(Decimal::ZERO);

        let total_paid = finance_payment::Entity::find()
            .filter(finance_payment::Column::IsDeleted.eq(false))
            .filter(finance_payment::Column::PaymentType.eq("PAYMENT"))
            .filter(finance_payment::Column::Status.eq("COMPLETED"))
            .select_only()
            .column_as(Expr::col(finance_payment::Column::Amount).sum(), "total")
            .into_tuple::<Option<Decimal>>()
            .one(self.db.as_ref())
            .await?
            .flatten()
            .unwrap_or(Decimal::ZERO);

        let cash_total = total_received - total_paid;

        let total_assets = ar_total + cash_total;
        let total_liabilities = ap_total;
        let total_equity = total_assets - total_liabilities;

        Ok(BalanceSheet {
            assets: vec![
                ReportItem { name: "货币资金".to_string(), amount: cash_total },
                ReportItem { name: "应收账款".to_string(), amount: ar_total },
            ],
            total_assets,
            liabilities: vec![
                ReportItem { name: "应付账款".to_string(), amount: ap_total },
            ],
            total_liabilities,
            equity: vec![
                ReportItem { name: "未分配利润".to_string(), amount: total_equity },
            ],
            total_equity,
        })
    }

    /// 利润表 (简化版)
    pub async fn get_income_statement(&self, start_date: chrono::NaiveDate, end_date: chrono::NaiveDate) -> Result<IncomeStatement, sea_orm::DbErr> {
        // 1. 营业收入 (销售发票总额)
        let revenue = finance_invoice::Entity::find()
            .filter(finance_invoice::Column::IsDeleted.eq(false))
            .filter(finance_invoice::Column::InvoiceType.eq("AR"))
            .filter(finance_invoice::Column::Status.ne("CANCELLED"))
            .filter(finance_invoice::Column::InvoiceDate.gte(start_date))
            .filter(finance_invoice::Column::InvoiceDate.lte(end_date))
            .select_only()
            .column_as(Expr::col(finance_invoice::Column::TotalAmount).sum(), "total")
            .into_tuple::<Option<Decimal>>()
            .one(self.db.as_ref())
            .await?
            .flatten()
            .unwrap_or(Decimal::ZERO);

        // 2. 营业成本 (采购发票总额)
        let cogs = finance_invoice::Entity::find()
            .filter(finance_invoice::Column::IsDeleted.eq(false))
            .filter(finance_invoice::Column::InvoiceType.eq("AP"))
            .filter(finance_invoice::Column::Status.ne("CANCELLED"))
            .filter(finance_invoice::Column::InvoiceDate.gte(start_date))
            .filter(finance_invoice::Column::InvoiceDate.lte(end_date))
            .select_only()
            .column_as(Expr::col(finance_invoice::Column::TotalAmount).sum(), "total")
            .into_tuple::<Option<Decimal>>()
            .one(self.db.as_ref())
            .await?
            .flatten()
            .unwrap_or(Decimal::ZERO);

        let net_income = revenue - cogs;

        Ok(IncomeStatement {
            revenue: vec![
                ReportItem { name: "主营业务收入".to_string(), amount: revenue },
            ],
            total_revenue: revenue,
            expenses: vec![
                ReportItem { name: "主营业务成本".to_string(), amount: cogs },
            ],
            total_expenses: cogs,
            net_income,
        })
    }
}
