//! 财务报表 Service
//!
//! 提供资产负债表、利润表、现金流量表等财务报表功能

use crate::models::{
    account_subject, assist_accounting_record, customer_credit, finance_invoice, finance_payment,
    fixed_asset, inventory_stock, voucher, voucher_item,
};
use crate::utils::error::AppError;
use rust_decimal::Decimal;
use sea_orm::{
    sea_query::Expr, ColumnTrait, DatabaseConnection, EntityTrait, JoinType,
    QueryFilter, QuerySelect, RelationTrait,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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

/// 试算平衡表条目
#[derive(Debug, Serialize, Deserialize)]
pub struct TrialBalanceEntry {
    pub subject_code: String,
    pub subject_name: String,
    pub level: i32,
    pub initial_debit: Decimal,
    pub initial_credit: Decimal,
    pub period_debit: Decimal,
    pub period_credit: Decimal,
    pub ending_debit: Decimal,
    pub ending_credit: Decimal,
}

/// 试算平衡表
#[derive(Debug, Serialize, Deserialize)]
pub struct TrialBalance {
    pub entries: Vec<TrialBalanceEntry>,
    pub total_initial_debit: Decimal,
    pub total_initial_credit: Decimal,
    pub total_period_debit: Decimal,
    pub total_period_credit: Decimal,
    pub total_ending_debit: Decimal,
    pub total_ending_credit: Decimal,
    pub period: String,
}

/// 总账条目
#[derive(Debug, Serialize, Deserialize)]
pub struct GeneralLedgerEntry {
    pub voucher_date: String,
    pub voucher_no: String,
    pub line_no: i32,
    pub summary: Option<String>,
    pub debit: Decimal,
    pub credit: Decimal,
    pub direction: String,
    pub balance: Decimal,
}

/// 总账
#[derive(Debug, Serialize, Deserialize)]
pub struct GeneralLedger {
    pub subject_code: String,
    pub subject_name: String,
    pub entries: Vec<GeneralLedgerEntry>,
    pub opening_balance: Decimal,
    pub closing_balance: Decimal,
    pub total_debit: Decimal,
    pub total_credit: Decimal,
    pub period_start: String,
    pub period_end: String,
}

/// 明细账条目
#[derive(Debug, Serialize, Deserialize)]
pub struct SubsidiaryLedgerEntry {
    pub business_date: String,
    pub business_no: String,
    pub business_type: String,
    pub subject_code: String,
    pub subject_name: String,
    pub summary: Option<String>,
    pub debit: Decimal,
    pub credit: Decimal,
    pub customer_id: Option<i32>,
    pub supplier_id: Option<i32>,
}

/// 明细账
#[derive(Debug, Serialize, Deserialize)]
pub struct SubsidiaryLedger {
    pub dimension_type: String,
    pub dimension_value: String,
    pub entries: Vec<SubsidiaryLedgerEntry>,
    pub total_debit: Decimal,
    pub total_credit: Decimal,
    pub period_start: String,
    pub period_end: String,
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
    pub async fn get_balance_sheet(&self) -> Result<BalanceSheet, AppError> {
        // 1. 流动资产
        // 应收账款
        let ar_total = finance_invoice::Entity::find()
            .filter(finance_invoice::Column::Status.ne("CANCELLED"))
            .filter(finance_invoice::Column::Status.ne("COMPLETED"))
            .select_only()
            .column_as(
                Expr::col(finance_invoice::Column::TotalAmount).sum(),
                "unpaid",
            )
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
            .column_as(
                Expr::col(inventory_stock::Column::QuantityAvailable).sum(),
                "total",
            )
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
            .column_as(
                Expr::col(customer_credit::Column::UsedCredit).sum(),
                "total",
            )
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
                ReportItem {
                    name: "货币资金".to_string(),
                    amount: cash_total,
                    description: Some("现金及银行存款".to_string()),
                },
                ReportItem {
                    name: "应收账款".to_string(),
                    amount: ar_total,
                    description: Some("未结清发票金额".to_string()),
                },
                ReportItem {
                    name: "存货".to_string(),
                    amount: inventory_total,
                    description: Some("库存商品价值".to_string()),
                },
                ReportItem {
                    name: "固定资产".to_string(),
                    amount: fixed_asset_total,
                    description: Some("固定资产净值".to_string()),
                },
            ],
            total_assets,
            liabilities: vec![ReportItem {
                name: "预收账款".to_string(),
                amount: advance_total,
                description: Some("客户预付款".to_string()),
            }],
            total_liabilities,
            equity: vec![ReportItem {
                name: "所有者权益".to_string(),
                amount: total_equity,
                description: Some("资产-负债".to_string()),
            }],
            total_equity,
            report_date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
        })
    }

    /// 利润表
    pub async fn get_income_statement(
        &self,
        start_date: chrono::NaiveDate,
        end_date: chrono::NaiveDate,
    ) -> Result<IncomeStatement, AppError> {
        // 营业收入（已完成的发票）
        let total_revenue = finance_invoice::Entity::find()
            .filter(finance_invoice::Column::Status.eq("COMPLETED"))
            .filter(finance_invoice::Column::InvoiceDate.gte(start_date))
            .filter(finance_invoice::Column::InvoiceDate.lte(end_date))
            .select_only()
            .column_as(
                Expr::col(finance_invoice::Column::TotalAmount).sum(),
                "total",
            )
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
            ReportItem {
                name: "管理费用".to_string(),
                amount: total_expenses * Decimal::new(15, 2),
                description: None,
            },
            ReportItem {
                name: "销售费用".to_string(),
                amount: total_expenses * Decimal::new(1, 1),
                description: None,
            },
            ReportItem {
                name: "财务费用".to_string(),
                amount: total_expenses * Decimal::new(5, 2),
                description: None,
            },
        ];
        let total_operating_expenses: Decimal = operating_expenses.iter().map(|i| i.amount).sum();

        let operating_income = gross_profit - total_operating_expenses;
        let other_income = Decimal::ZERO;
        let other_expenses = Decimal::ZERO;
        let net_income = operating_income + other_income - other_expenses;

        Ok(IncomeStatement {
            revenue: vec![ReportItem {
                name: "营业收入".to_string(),
                amount: total_revenue,
                description: Some("主营业务收入".to_string()),
            }],
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
    pub async fn get_cash_flow_statement(
        &self,
        start_date: chrono::NaiveDate,
        end_date: chrono::NaiveDate,
    ) -> Result<CashFlowStatement, AppError> {
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
        let investing_activities = vec![ReportItem {
            name: "购建固定资产".to_string(),
            amount: Decimal::ZERO,
            description: None,
        }];
        let net_cash_from_investing = Decimal::ZERO;

        // 筹资活动现金流（简化）
        let financing_activities = vec![ReportItem {
            name: "吸收投资".to_string(),
            amount: Decimal::ZERO,
            description: None,
        }];
        let net_cash_from_financing = Decimal::ZERO;

        let net_change_in_cash =
            net_cash_from_operations + net_cash_from_investing + net_cash_from_financing;
        let beginning_cash = Decimal::ZERO; // 需要从期初余额获取
        let ending_cash = beginning_cash + net_change_in_cash;

        Ok(CashFlowStatement {
            operating_activities: vec![
                ReportItem {
                    name: "销售商品收到的现金".to_string(),
                    amount: cash_receipts,
                    description: None,
                },
                ReportItem {
                    name: "购买商品支付的现金".to_string(),
                    amount: cash_payments,
                    description: None,
                },
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

    /// 试算平衡表
    ///
    /// 直接读取科目余额表(account_subjects)中各科目的期初/本期/期末余额。
    /// 期间格式: YYYY-MM。
    pub async fn get_trial_balance(
        &self,
        period: Option<String>,
    ) -> Result<TrialBalance, AppError> {
        let period_str = period.unwrap_or_else(|| chrono::Utc::now().format("%Y-%m").to_string());

        let subjects = account_subject::Entity::find()
            .filter(account_subject::Column::Status.eq("ACTIVE"))
            .all(self.db.as_ref())
            .await?;

        let mut entries: Vec<TrialBalanceEntry> = subjects
            .into_iter()
            .map(|s| TrialBalanceEntry {
                subject_code: s.code,
                subject_name: s.name,
                level: s.level,
                initial_debit: s.initial_balance_debit,
                initial_credit: s.initial_balance_credit,
                period_debit: s.current_period_debit,
                period_credit: s.current_period_credit,
                ending_debit: s.ending_balance_debit,
                ending_credit: s.ending_balance_credit,
            })
            .collect();

        // 按科目编码排序
        entries.sort_by(|a, b| a.subject_code.cmp(&b.subject_code));

        let total_initial_debit: Decimal = entries.iter().map(|e| e.initial_debit).sum();
        let total_initial_credit: Decimal = entries.iter().map(|e| e.initial_credit).sum();
        let total_period_debit: Decimal = entries.iter().map(|e| e.period_debit).sum();
        let total_period_credit: Decimal = entries.iter().map(|e| e.period_credit).sum();
        let total_ending_debit: Decimal = entries.iter().map(|e| e.ending_debit).sum();
        let total_ending_credit: Decimal = entries.iter().map(|e| e.ending_credit).sum();

        Ok(TrialBalance {
            entries,
            total_initial_debit,
            total_initial_credit,
            total_period_debit,
            total_period_credit,
            total_ending_debit,
            total_ending_credit,
            period: period_str,
        })
    }

    /// 总账（按科目代码）
    ///
    /// 返回指定科目在某个期间内的所有凭证分录，附带借贷方向与逐笔余额。
    pub async fn get_general_ledger(
        &self,
        subject_code: String,
        start_date: chrono::NaiveDate,
        end_date: chrono::NaiveDate,
    ) -> Result<GeneralLedger, AppError> {
        // 查询科目基本信息
        let subject = account_subject::Entity::find()
            .filter(account_subject::Column::Code.eq(&subject_code))
            .one(self.db.as_ref())
            .await?;

        let (subject_name, opening_balance) = match subject {
            Some(s) => {
                let open = s.initial_balance_debit - s.initial_balance_credit;
                (s.name, open)
            }
            None => (subject_code.clone(), Decimal::ZERO),
        };

        // 联表查询凭证分录
        let rows: Vec<(
            chrono::NaiveDate,
            String,
            i32,
            Option<String>,
            Decimal,
            Decimal,
        )> = voucher_item::Entity::find()
            .join(JoinType::InnerJoin, voucher_item::Relation::Voucher.def())
            .filter(voucher_item::Column::SubjectCode.eq(&subject_code))
            .filter(voucher::Column::VoucherDate.gte(start_date))
            .filter(voucher::Column::VoucherDate.lte(end_date))
            .select_only()
            .column_as(voucher::Column::VoucherDate, "voucher_date")
            .column_as(voucher::Column::VoucherNo, "voucher_no")
            .column_as(voucher_item::Column::LineNo, "line_no")
            .column_as(voucher_item::Column::Summary, "summary")
            .column_as(voucher_item::Column::Debit, "debit")
            .column_as(voucher_item::Column::Credit, "credit")
            .into_tuple()
            .all(self.db.as_ref())
            .await?;

        let mut entries: Vec<GeneralLedgerEntry> = Vec::with_capacity(rows.len());
        let mut running_balance = opening_balance;
        let mut total_debit = Decimal::ZERO;
        let mut total_credit = Decimal::ZERO;

        for (voucher_date, voucher_no, line_no, summary, debit, credit) in rows {
            total_debit += debit;
            total_credit += credit;
            let direction = if debit > Decimal::ZERO {
                "DEBIT".to_string()
            } else if credit > Decimal::ZERO {
                "CREDIT".to_string()
            } else {
                "".to_string()
            };
            running_balance += debit - credit;
            entries.push(GeneralLedgerEntry {
                voucher_date: voucher_date.format("%Y-%m-%d").to_string(),
                voucher_no,
                line_no,
                summary,
                debit,
                credit,
                direction,
                balance: running_balance,
            });
        }

        Ok(GeneralLedger {
            subject_code,
            subject_name,
            entries,
            opening_balance,
            closing_balance: running_balance,
            total_debit,
            total_credit,
            period_start: start_date.format("%Y-%m-%d").to_string(),
            period_end: end_date.format("%Y-%m-%d").to_string(),
        })
    }

    /// 明细账（按辅助核算维度）
    ///
    /// 通过 assist_accounting_record 关联查询，可按客户/供应商/部门/员工等过滤。
    pub async fn get_subsidiary_ledger(
        &self,
        dimension_type: String,
        dimension_value: String,
        start_date: chrono::NaiveDate,
        end_date: chrono::NaiveDate,
    ) -> Result<SubsidiaryLedger, AppError> {
        let mut query = assist_accounting_record::Entity::find();

        // 根据维度类型设置过滤条件
        match dimension_type.as_str() {
            "CUSTOMER" => {
                if let Ok(customer_id) = dimension_value.parse::<i32>() {
                    query =
                        query.filter(assist_accounting_record::Column::CustomerId.eq(customer_id));
                }
            }
            "SUPPLIER" => {
                if let Ok(supplier_id) = dimension_value.parse::<i32>() {
                    query =
                        query.filter(assist_accounting_record::Column::SupplierId.eq(supplier_id));
                }
            }
            "DEPARTMENT" => {
                if let Ok(department_id) = dimension_value.parse::<i32>() {
                    query = query
                        .filter(assist_accounting_record::Column::WorkshopId.eq(department_id));
                }
            }
            _ => {
                // 其他维度: 返回空结果，但保持结构完整
            }
        }

        let start = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
            start_date.and_hms_opt(0, 0, 0).unwrap(),
            chrono::Utc,
        );
        let end = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
            end_date.and_hms_opt(23, 59, 59).unwrap(),
            chrono::Utc,
        );
        let records = query
            .filter(assist_accounting_record::Column::CreatedAt.gte(start))
            .filter(assist_accounting_record::Column::CreatedAt.lte(end))
            .all(self.db.as_ref())
            .await?;

        // 联表查询科目名称
        let subject_ids: Vec<i32> = records.iter().map(|r| r.account_subject_id).collect();
        let subjects: Vec<account_subject::Model> = if subject_ids.is_empty() {
            vec![]
        } else {
            account_subject::Entity::find()
                .filter(account_subject::Column::Id.is_in(subject_ids))
                .all(self.db.as_ref())
                .await?
        };
        let subject_map: std::collections::HashMap<i32, account_subject::Model> =
            subjects.into_iter().map(|s| (s.id, s)).collect();

        let mut entries: Vec<SubsidiaryLedgerEntry> = Vec::with_capacity(records.len());
        let mut total_debit = Decimal::ZERO;
        let mut total_credit = Decimal::ZERO;

        for r in records {
            total_debit += r.debit_amount;
            total_credit += r.credit_amount;
            let subject_info = subject_map.get(&r.account_subject_id);
            entries.push(SubsidiaryLedgerEntry {
                business_date: r.created_at.format("%Y-%m-%d").to_string(),
                business_no: r.business_no,
                business_type: r.business_type,
                subject_code: subject_info.map(|s| s.code.clone()).unwrap_or_default(),
                subject_name: subject_info.map(|s| s.name.clone()).unwrap_or_default(),
                summary: r.remarks,
                debit: r.debit_amount,
                credit: r.credit_amount,
                customer_id: r.customer_id,
                supplier_id: r.supplier_id,
            });
        }

        Ok(SubsidiaryLedger {
            dimension_type,
            dimension_value,
            entries,
            total_debit,
            total_credit,
            period_start: start_date.format("%Y-%m-%d").to_string(),
            period_end: end_date.format("%Y-%m-%d").to_string(),
        })
    }
}
