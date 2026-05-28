//! 财务报表 Service
//!
//! 提供资产负债表、利润表、现金流量表等财务报表功能

use rust_decimal::Decimal;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, QuerySelect, sea_query::Expr, ColumnTrait};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::models::{finance_invoice, finance_payment, inventory_stock, fixed_asset, customer_credit};

/// 资产负债表
#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceSheet {
    pub assets: Vec<ReportItem>,
    pub total_assets: Decimal,
    pub liabilities: Vec<ReportItem>,
    pub total_liabilities: Decimal,
    pub equity: Vec<ReportItem>,
    pub total_equity: Decimal,
    pub report_date: String,
}

/// 利润表
#[derive(Debug, Serialize, Deserialize)]
pub struct IncomeStatement {
    pub revenue: Vec<ReportItem>,
    pub total_revenue: Decimal,
    pub cost_of_goods_sold: Decimal,
    pub gross_profit: Decimal,
    pub operating_expenses: Vec<ReportItem>,
    pub total_operating_expenses: Decimal,
    pub operating_income: Decimal,
    pub other_income: Decimal,
    pub other_expenses: Decimal,
    pub net_income: Decimal,
    pub period_start: String,
    pub period_end: String,
}

/// 现金流量表
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct CashFlowStatement {
    pub operating_activities: Vec<ReportItem>,
    pub net_cash_from_operations: Decimal,
    pub investing_activities: Vec<ReportItem>,
    pub net_cash_from_investing: Decimal,
    pub financing_activities: Vec<ReportItem>,
    pub net_cash_from_financing: Decimal,
    pub net_change_in_cash: Decimal,
    pub beginning_cash: Decimal,
    pub ending_cash: Decimal,
    pub period_start: String,
    pub period_end: String,
}

/// 报表项目
#[derive(Debug, Serialize, Deserialize)]
pub struct ReportItem {
    pub name: String,
    pub amount: Decimal,
    pub description: Option<String>,
}

/// 财务报表 Service
pub struct FinanceReportService {
    db: Arc<DatabaseConnection>,
}

impl FinanceReportService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 资产负债表
    pub async fn get_balance_sheet(&self) -> Result<BalanceSheet, sea_orm::DbErr> {
        // 1. 流动资产
        // 应收账款
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

        // 现金/银行存款（收款总额）
        let total_received = finance_payment::Entity::find()
            .filter(finance_payment::Column::Status.eq("COMPLETED"))
            .select_only()
            .column_as(Expr::col(finance_payment::Column::Amount).sum(), "total")
            .into_tuple::<Option<Decimal>>()
            .one(self.db.as_ref())
            .await?
            .flatten()
            .unwrap_or(Decimal::ZERO);

        // 应付账款
        let _ap_total = finance_payment::Entity::find()
            .filter(finance_payment::Column::Status.eq("PENDING"))
            .select_only()
            .column_as(Expr::col(finance_payment::Column::Amount).sum(), "total")
            .into_tuple::<Option<Decimal>>()
            .one(self.db.as_ref())
            .await?
            .flatten()
            .unwrap_or(Decimal::ZERO);

        let cash_total = total_received;

        // 库存资产（按成本估算）
        let inventory_total = inventory_stock::Entity::find()
            .filter(inventory_stock::Column::StockStatus.eq("ACTIVE"))
            .select_only()
            .column_as(Expr::col(inventory_stock::Column::QuantityAvailable).sum(), "total")
            .into_tuple::<Option<Decimal>>()
            .one(self.db.as_ref())
            .await?
            .flatten()
            .unwrap_or(Decimal::ZERO);

        // 固定资产净值
        let fixed_asset_total = fixed_asset::Entity::find()
            .filter(fixed_asset::Column::Status.eq("ACTIVE"))
            .select_only()
            .column_as(Expr::col(fixed_asset::Column::NetValue).sum(), "total")
            .into_tuple::<Option<Decimal>>()
            .one(self.db.as_ref())
            .await?
            .flatten()
            .unwrap_or(Decimal::ZERO);

        let total_assets = ar_total + cash_total + inventory_total + fixed_asset_total;

        // 2. 负债
        // 预收账款（客户信用额度已使用部分）
        let advance_total = customer_credit::Entity::find()
            .select_only()
            .column_as(Expr::col(customer_credit::Column::UsedCredit).sum(), "total")
            .into_tuple::<Option<Decimal>>()
            .one(self.db.as_ref())
            .await?
            .flatten()
            .unwrap_or(Decimal::ZERO);

        let total_liabilities = advance_total;

        // 3. 所有者权益
        let total_equity = total_assets - total_liabilities;

        Ok(BalanceSheet {
            assets: vec![
                ReportItem { name: "货币资金".to_string(), amount: cash_total, description: Some("现金及银行存款".to_string()) },
                ReportItem { name: "应收账款".to_string(), amount: ar_total, description: Some("未结清发票金额".to_string()) },
                ReportItem { name: "存货".to_string(), amount: inventory_total, description: Some("库存商品价值".to_string()) },
                ReportItem { name: "固定资产".to_string(), amount: fixed_asset_total, description: Some("固定资产净值".to_string()) },
            ],
            total_assets,
            liabilities: vec![
                ReportItem { name: "预收账款".to_string(), amount: advance_total, description: Some("客户预付款".to_string()) },
            ],
            total_liabilities,
            equity: vec![
                ReportItem { name: "所有者权益".to_string(), amount: total_equity, description: Some("资产-负债".to_string()) },
            ],
            total_equity,
            report_date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
        })
    }

    /// 利润表
    pub async fn get_income_statement(
        &self,
        start_date: chrono::NaiveDate,
        end_date: chrono::NaiveDate,
    ) -> Result<IncomeStatement, sea_orm::DbErr> {
        // 营业收入（已完成的发票）
        let total_revenue = finance_invoice::Entity::find()
            .filter(finance_invoice::Column::Status.eq("COMPLETED"))
            .filter(finance_invoice::Column::InvoiceDate.gte(start_date))
            .filter(finance_invoice::Column::InvoiceDate.lte(end_date))
            .select_only()
            .column_as(Expr::col(finance_invoice::Column::TotalAmount).sum(), "total")
            .into_tuple::<Option<Decimal>>()
            .one(self.db.as_ref())
            .await?
            .flatten()
            .unwrap_or(Decimal::ZERO);

        // 营业成本（已完成的付款）
        let total_expenses = finance_payment::Entity::find()
            .filter(finance_payment::Column::Status.eq("COMPLETED"))
            .filter(finance_payment::Column::PaymentDate.gte(start_date))
            .filter(finance_payment::Column::PaymentDate.lte(end_date))
            .select_only()
            .column_as(Expr::col(finance_payment::Column::Amount).sum(), "total")
            .into_tuple::<Option<Decimal>>()
            .one(self.db.as_ref())
            .await?
            .flatten()
            .unwrap_or(Decimal::ZERO);

        // 计算毛利润
        let cost_of_goods_sold = total_expenses * Decimal::new(7, 1); // 假设70%为成本
        let gross_profit = total_revenue - cost_of_goods_sold;

        // 运营费用（简化处理）
        let operating_expenses = vec![
            ReportItem { name: "管理费用".to_string(), amount: total_expenses * Decimal::new(15, 2), description: None },
            ReportItem { name: "销售费用".to_string(), amount: total_expenses * Decimal::new(1, 1), description: None },
            ReportItem { name: "财务费用".to_string(), amount: total_expenses * Decimal::new(5, 2), description: None },
        ];
        let total_operating_expenses: Decimal = operating_expenses.iter().map(|i| i.amount).sum();

        let operating_income = gross_profit - total_operating_expenses;
        let other_income = Decimal::ZERO;
        let other_expenses = Decimal::ZERO;
        let net_income = operating_income + other_income - other_expenses;

        Ok(IncomeStatement {
            revenue: vec![
                ReportItem { name: "营业收入".to_string(), amount: total_revenue, description: Some("主营业务收入".to_string()) },
            ],
            total_revenue,
            cost_of_goods_sold,
            gross_profit,
            operating_expenses,
            total_operating_expenses,
            operating_income,
            other_income,
            other_expenses,
            net_income,
            period_start: start_date.format("%Y-%m-%d").to_string(),
            period_end: end_date.format("%Y-%m-%d").to_string(),
        })
    }

    /// 现金流量表
    #[allow(dead_code)]
    pub async fn get_cash_flow_statement(
        &self,
        start_date: chrono::NaiveDate,
        end_date: chrono::NaiveDate,
    ) -> Result<CashFlowStatement, sea_orm::DbErr> {
        // 经营活动现金流
        let cash_receipts = finance_payment::Entity::find()
            .filter(finance_payment::Column::Status.eq("COMPLETED"))
            .filter(finance_payment::Column::PaymentDate.gte(start_date))
            .filter(finance_payment::Column::PaymentDate.lte(end_date))
            .select_only()
            .column_as(Expr::col(finance_payment::Column::Amount).sum(), "total")
            .into_tuple::<Option<Decimal>>()
            .one(self.db.as_ref())
            .await?
            .flatten()
            .unwrap_or(Decimal::ZERO);

        let cash_payments = finance_payment::Entity::find()
            .filter(finance_payment::Column::Status.eq("PENDING"))
            .filter(finance_payment::Column::PaymentDate.gte(start_date))
            .filter(finance_payment::Column::PaymentDate.lte(end_date))
            .select_only()
            .column_as(Expr::col(finance_payment::Column::Amount).sum(), "total")
            .into_tuple::<Option<Decimal>>()
            .one(self.db.as_ref())
            .await?
            .flatten()
            .unwrap_or(Decimal::ZERO);

        let net_cash_from_operations = cash_receipts - cash_payments;

        // 投资活动现金流（简化）
        let investing_activities = vec![
            ReportItem { name: "购建固定资产".to_string(), amount: Decimal::ZERO, description: None },
        ];
        let net_cash_from_investing = Decimal::ZERO;

        // 筹资活动现金流（简化）
        let financing_activities = vec![
            ReportItem { name: "吸收投资".to_string(), amount: Decimal::ZERO, description: None },
        ];
        let net_cash_from_financing = Decimal::ZERO;

        let net_change_in_cash = net_cash_from_operations + net_cash_from_investing + net_cash_from_financing;
        let beginning_cash = Decimal::ZERO; // 需要从期初余额获取
        let ending_cash = beginning_cash + net_change_in_cash;

        Ok(CashFlowStatement {
            operating_activities: vec![
                ReportItem { name: "销售商品收到的现金".to_string(), amount: cash_receipts, description: None },
                ReportItem { name: "购买商品支付的现金".to_string(), amount: cash_payments, description: None },
            ],
            net_cash_from_operations,
            investing_activities,
            net_cash_from_investing,
            financing_activities,
            net_cash_from_financing,
            net_change_in_cash,
            beginning_cash,
            ending_cash,
            period_start: start_date.format("%Y-%m-%d").to_string(),
            period_end: end_date.format("%Y-%m-%d").to_string(),
        })
    }
}
