//! 财务报表 Service
//!
//! 提供资产负债表、利润表、现金流量表等财务报表功能

// 批次 100 P3-A 修复（v5 复审）：状态字符串常量化，引用 crate::models::status

use crate::models::{
    account_subject, assist_accounting_record, finance_payment, voucher, voucher_item,
};
use crate::utils::error::AppError;
use rust_decimal::Decimal;
use sea_orm::{
    sea_query::Expr, ColumnTrait, DatabaseConnection, EntityTrait, JoinType,
    QueryFilter, QueryOrder, QuerySelect, RelationTrait,
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
        // F-P1-2 修复（批次 363 v13 复审）：资产/负债项从凭证体系取时点余额
        // 原实现：存货取 QuantityAvailable（数量非金额，会计口径错误）；_ap_total 应付账款
        // 计算后未使用（死代码）；预收账款从客户信用额度取数（业务口径与会计口径混淆）。
        // 修复：按中国企业会计准则科目编码从已过账凭证分录累计借贷方差额取余额。
        // 报表时点 = 当前日期。
        let report_date = chrono::Utc::now().date_naive();

        // 1. 流动资产
        // 应收账款：1122 应收账款（资产类，借方-贷方）
        let ar_total = self
            .get_subject_balance_by_prefix("1122", true, report_date)
            .await?;

        // 货币资金：1001 库存现金 + 1002 银行存款（资产类，借方-贷方）
        let cash_on_hand = self
            .get_subject_balance_by_prefix("1001", true, report_date)
            .await?;
        let cash_in_bank = self
            .get_subject_balance_by_prefix("1002", true, report_date)
            .await?;
        let cash_total = cash_on_hand + cash_in_bank;

        // 存货：14xx 库存商品/原材料（资产类，借方-贷方，修复原取数量非金额）
        let inventory_total = self
            .get_subject_balance_by_prefix("14", true, report_date)
            .await?;

        // 固定资产：16xx 固定资产（资产类，借方-贷方）
        let fixed_asset_total = self
            .get_subject_balance_by_prefix("16", true, report_date)
            .await?;

        let total_assets = ar_total + cash_total + inventory_total + fixed_asset_total;

        // 2. 负债
        // 应付账款：2202 应付账款（负债类，贷方-借方，修复 _ap_total 未使用死代码）
        let ap_total = self
            .get_subject_balance_by_prefix("2202", false, report_date)
            .await?;
        // 预收账款：2203 预收账款（负债类，贷方-借方）
        let advance_total = self
            .get_subject_balance_by_prefix("2203", false, report_date)
            .await?;

        let total_liabilities = ap_total + advance_total;

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
            liabilities: vec![
                ReportItem {
                    name: "应付账款".to_string(),
                    amount: ap_total,
                    description: Some("未结清供应商款项".to_string()),
                },
                ReportItem {
                    name: "预收账款".to_string(),
                    amount: advance_total,
                    description: Some("客户预付款".to_string()),
                },
            ],
            total_liabilities,
            equity: vec![ReportItem {
                name: "所有者权益".to_string(),
                amount: total_equity,
                description: Some("资产-负债".to_string()),
            }],
            total_equity,
            report_date: report_date.format("%Y-%m-%d").to_string(),
        })
    }

    /// 利润表
    pub async fn get_income_statement(
        &self,
        start_date: chrono::NaiveDate,
        end_date: chrono::NaiveDate,
    ) -> Result<IncomeStatement, AppError> {
        // F-P1-2 修复（批次 362 v13 复审）：从凭证体系取数，移除硬编码比例
        // 原实现从 finance_invoice/finance_payment 业务表取数 + 硬编码 70%/15%/10%/5% 比例，
        // 违反禁止硬编码规则且与会计实务脱节。
        // 修复：从已过账凭证分录按科目编码前缀聚合，参考中国企业会计准则科目编码：
        // 60xx = 收入类，64xx = 成本类，6601 = 销售费用，6602 = 管理费用，6603 = 财务费用。

        let total_revenue = self
            .sum_voucher_amount_by_subject_prefix(
                "60",
                true,
                start_date,
                end_date,
            )
            .await?;

        let cost_of_goods_sold = self
            .sum_voucher_amount_by_subject_prefix(
                "64",
                false,
                start_date,
                end_date,
            )
            .await?;

        let gross_profit = total_revenue - cost_of_goods_sold;

        let sales_expense = self
            .sum_voucher_amount_by_subject_prefix(
                "6601",
                false,
                start_date,
                end_date,
            )
            .await?;

        let management_expense = self
            .sum_voucher_amount_by_subject_prefix(
                "6602",
                false,
                start_date,
                end_date,
            )
            .await?;

        let financial_expense = self
            .sum_voucher_amount_by_subject_prefix(
                "6603",
                false,
                start_date,
                end_date,
            )
            .await?;

        let operating_expenses = vec![
            ReportItem {
                name: "管理费用".to_string(),
                amount: management_expense,
                description: None,
            },
            ReportItem {
                name: "销售费用".to_string(),
                amount: sales_expense,
                description: None,
            },
            ReportItem {
                name: "财务费用".to_string(),
                amount: financial_expense,
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
                description: Some("主营业务收入及其他业务收入".to_string()),
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

    /// F-P1-2 修复（批次 362 v13 复审）：按科目编码前缀聚合已过账凭证金额
    ///
    /// 从已过账凭证分录联表查询，按科目编码前缀过滤，返回借方或贷方总额。
    /// 用于利润表等财务报表从凭证体系取数，替代原硬编码比例估算。
    ///
    /// 参数说明：
    /// - `prefix`：科目编码前缀（如 "60" 表示收入类，"64" 表示成本类）
    /// - `is_credit`：true 返回贷方总额（收入类），false 返回借方总额（成本/费用类）
    async fn sum_voucher_amount_by_subject_prefix(
        &self,
        prefix: &str,
        is_credit: bool,
        start_date: chrono::NaiveDate,
        end_date: chrono::NaiveDate,
    ) -> Result<Decimal, AppError> {
        let target_column = if is_credit {
            voucher_item::Column::Credit
        } else {
            voucher_item::Column::Debit
        };

        let amount: Option<Decimal> = voucher_item::Entity::find()
            .join(JoinType::InnerJoin, voucher_item::Relation::Voucher.def())
            .filter(voucher::Column::Status.eq(crate::models::status::voucher::VOUCHER_POSTED))
            .filter(voucher::Column::VoucherDate.gte(start_date))
            .filter(voucher::Column::VoucherDate.lte(end_date))
            .filter(voucher_item::Column::SubjectCode.starts_with(prefix))
            .select_only()
            .column_as(Expr::col(target_column).sum(), "total")
            .into_tuple()
            .one(self.db.as_ref())
            .await?
            .flatten();

        Ok(amount.unwrap_or(Decimal::ZERO))
    }

    /// F-P1-2 修复（批次 363 v13 复审）：按科目编码前缀取科目余额（时点数）
    ///
    /// 从已过账凭证分录联表查询，按科目编码前缀过滤，返回借方累计与贷方累计的差额。
    /// 用于资产负债表等时点报表从凭证体系取数，替代原从业务表取数量或硬编码零值。
    ///
    /// 参数说明：
    /// - `prefix`：科目编码前缀（如 "14" 表示存货类，"16" 表示固定资产类）
    /// - `is_asset`：true 返回借方-贷方（资产类余额方向），false 返回贷方-借方（负债/权益类余额方向）
    /// - `up_to_date`：截至该日期（含）的凭证参与累计
    async fn get_subject_balance_by_prefix(
        &self,
        prefix: &str,
        is_asset: bool,
        up_to_date: chrono::NaiveDate,
    ) -> Result<Decimal, AppError> {
        let (debit_col, credit_col) = (
            Expr::col(voucher_item::Column::Debit).sum(),
            Expr::col(voucher_item::Column::Credit).sum(),
        );
        let result: Option<(Option<Decimal>, Option<Decimal>)> = voucher_item::Entity::find()
            .join(JoinType::InnerJoin, voucher_item::Relation::Voucher.def())
            .filter(voucher::Column::Status.eq(crate::models::status::voucher::VOUCHER_POSTED))
            .filter(voucher::Column::VoucherDate.lte(up_to_date))
            .filter(voucher_item::Column::SubjectCode.starts_with(prefix))
            .select_only()
            .column_as(debit_col, "total_debit")
            .column_as(credit_col, "total_credit")
            .into_tuple()
            .one(self.db.as_ref())
            .await?;
        let (debit_opt, credit_opt) = result.unwrap_or((None, None));
        let debit = debit_opt.unwrap_or(Decimal::ZERO);
        let credit = credit_opt.unwrap_or(Decimal::ZERO);
        let balance = if is_asset {
            debit - credit
        } else {
            credit - debit
        };
        Ok(balance)
    }

    /// 现金流量表
    pub async fn get_cash_flow_statement(
        &self,
        start_date: chrono::NaiveDate,
        end_date: chrono::NaiveDate,
    ) -> Result<CashFlowStatement, AppError> {
        // F-P1-2 修复（批次 363 v13 复审）：投资/筹资活动从凭证体系取数，期初现金从科目余额取数
        // 原实现：投资活动/筹资活动/期初现金均硬编码 Decimal::ZERO，违反禁止硬编码规则且报表失真。
        // 修复：按中国企业会计准则科目编码从已过账凭证分录取期间发生额与期初余额。

        // 经营活动现金流（保留从 finance_payment 取数，支付单/收款单即经营性现金流转）
        let cash_receipts = finance_payment::Entity::find()
            .filter(finance_payment::Column::Status.eq(crate::models::status::common::STATUS_COMPLETED))
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
            .filter(finance_payment::Column::Status.eq(crate::models::status::common::STATUS_PENDING))
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

        // 投资活动现金流（1601 固定资产：贷方=处置收回，借方=购建支付）
        let investing_inflow = self
            .sum_voucher_amount_by_subject_prefix("1601", true, start_date, end_date)
            .await?;
        let investing_outflow = self
            .sum_voucher_amount_by_subject_prefix("1601", false, start_date, end_date)
            .await?;
        let net_cash_from_investing = investing_inflow - investing_outflow;
        let investing_activities = vec![
            ReportItem {
                name: "处置固定资产收回的现金".to_string(),
                amount: investing_inflow,
                description: None,
            },
            ReportItem {
                name: "购建固定资产支付的现金".to_string(),
                amount: investing_outflow,
                description: None,
            },
        ];

        // 筹资活动现金流（25xx 借款：贷方=借入，借方=偿还）
        let financing_inflow = self
            .sum_voucher_amount_by_subject_prefix("25", true, start_date, end_date)
            .await?;
        let financing_outflow = self
            .sum_voucher_amount_by_subject_prefix("25", false, start_date, end_date)
            .await?;
        let net_cash_from_financing = financing_inflow - financing_outflow;
        let financing_activities = vec![
            ReportItem {
                name: "借入款项收到的现金".to_string(),
                amount: financing_inflow,
                description: None,
            },
            ReportItem {
                name: "偿还借款支付的现金".to_string(),
                amount: financing_outflow,
                description: None,
            },
        ];

        let net_change_in_cash =
            net_cash_from_operations + net_cash_from_investing + net_cash_from_financing;
        // 期初现金 = 截至期初前一日的 1001 库存现金 + 1002 银行存款 余额
        let day_before_start = start_date.pred_opt().unwrap_or(start_date);
        let beginning_cash_on_hand = self
            .get_subject_balance_by_prefix("1001", true, day_before_start)
            .await?;
        let beginning_cash_in_bank = self
            .get_subject_balance_by_prefix("1002", true, day_before_start)
            .await?;
        let beginning_cash = beginning_cash_on_hand + beginning_cash_in_bank;
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
            .filter(account_subject::Column::Status.eq(crate::models::status::common::STATUS_ACTIVE))
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

        // 批次 95 P3-2 修复：and_hms_opt 返回 Option，原 .unwrap() 在非法参数时 panic。
        // 虽然此处 0,0,0 / 23,59,59 是合法值不会触发 panic，但保留 unwrap 不符合错误处理规范。
        // 改为 ok_or_else + ? 将失败显式传播为 AppError（函数返回 Result）。
        let start = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
            start_date
                .and_hms_opt(0, 0, 0)
                .ok_or_else(|| AppError::internal("批次 95 P3-2: 起始日期时间构造失败".to_string()))?,
            chrono::Utc,
        );
        let end = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
            end_date
                .and_hms_opt(23, 59, 59)
                .ok_or_else(|| AppError::internal("批次 95 P3-2: 结束日期时间构造失败".to_string()))?,
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

    /// 按科目编码前缀穿透到凭证分录（F-P2-2 修复，批次 387 v13 复审）
    ///
    /// 复用 get_general_ledger 的联表模式，扩展返回业务单据追溯字段。
    /// 用于资产负债表/利润表/现金流量表穿透。
    pub async fn drill_down_by_subject_prefix(
        &self,
        subject_prefix: String,
        start_date: chrono::NaiveDate,
        end_date: chrono::NaiveDate,
    ) -> Result<Vec<VoucherItemDetail>, AppError> {
        let rows: Vec<(
            i32,
            String,
            chrono::NaiveDate,
            i32,
            String,
            Option<String>,
            Option<String>,
            Decimal,
            Decimal,
            Option<String>,
            Option<String>,
            Option<i32>,
            Option<String>,
        )> = voucher_item::Entity::find()
            .join(JoinType::InnerJoin, voucher_item::Relation::Voucher.def())
            .filter(voucher_item::Column::SubjectCode.starts_with(&subject_prefix))
            .filter(voucher::Column::VoucherDate.gte(start_date))
            .filter(voucher::Column::VoucherDate.lte(end_date))
            .filter(
                voucher::Column::Status.eq(crate::models::status::voucher::VOUCHER_POSTED),
            )
            .select_only()
            .column_as(voucher::Column::Id, "voucher_id")
            .column_as(voucher::Column::VoucherNo, "voucher_no")
            .column_as(voucher::Column::VoucherDate, "voucher_date")
            .column_as(voucher_item::Column::LineNo, "line_no")
            .column_as(voucher_item::Column::SubjectCode, "subject_code")
            .column_as(voucher_item::Column::SubjectName, "subject_name")
            .column_as(voucher_item::Column::Summary, "summary")
            .column_as(voucher_item::Column::Debit, "debit")
            .column_as(voucher_item::Column::Credit, "credit")
            .column_as(voucher::Column::SourceType, "source_type")
            .column_as(voucher::Column::SourceModule, "source_module")
            .column_as(voucher::Column::SourceBillId, "source_bill_id")
            .column_as(voucher::Column::SourceBillNo, "source_bill_no")
            .order_by_asc(voucher::Column::VoucherDate)
            .order_by_asc(voucher::Column::VoucherNo)
            .order_by_asc(voucher_item::Column::LineNo)
            .into_tuple()
            .all(self.db.as_ref())
            .await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    voucher_id,
                    voucher_no,
                    voucher_date,
                    line_no,
                    subject_code,
                    subject_name,
                    summary,
                    debit,
                    credit,
                    source_type,
                    source_module,
                    source_bill_id,
                    source_bill_no,
                )| VoucherItemDetail {
                    voucher_id,
                    voucher_no,
                    voucher_date,
                    line_no,
                    subject_code,
                    subject_name,
                    summary,
                    debit,
                    credit,
                    source_type,
                    source_module,
                    source_bill_id,
                    source_bill_no,
                },
            )
            .collect())
    }

    /// 按期间 + 科目编码穿透到凭证分录（F-P2-2 修复，批次 387 v13 复审）
    ///
    /// 用于试算平衡表穿透。period 格式 YYYY-MM，转换为月初到月末日期范围。
    pub async fn drill_down_by_period_and_subject(
        &self,
        period: String,
        subject_code: String,
    ) -> Result<Vec<VoucherItemDetail>, AppError> {
        // 解析 period (YYYY-MM) 为日期范围
        let parts: Vec<&str> = period.split('-').collect();
        if parts.len() != 2 {
            return Err(AppError::validation("period 格式必须为 YYYY-MM"));
        }
        let year: i32 = parts[0].parse().map_err(|_| AppError::validation("period 年份无效"))?;
        let month: u32 = parts[1].parse().map_err(|_| AppError::validation("period 月份无效"))?;
        if !(1..=12).contains(&month) {
            return Err(AppError::validation("period 月份必须在 1-12 之间"));
        }
        let start_date = chrono::NaiveDate::from_ymd_opt(year, month, 1)
            .ok_or_else(|| AppError::validation("period 起始日期无效"))?;
        let end_date = chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
            .or_else(|| chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1))
            .ok_or_else(|| AppError::validation("period 结束日期无效"))?
            - chrono::Duration::days(1);

        // 复用前缀穿透方法（subject_code 作为前缀，支持一级科目下所有明细）
        self.drill_down_by_subject_prefix(subject_code, start_date, end_date)
            .await
    }
}

/// 凭证分录穿透明细（F-P2-2 修复，批次 387 v13 复审）
///
/// 用于报表项目穿透到凭证分录级，包含业务单据追溯字段（source_type/source_bill_id），
/// 前端可据此继续调用业务单据 API 完成全链路追溯。
#[derive(Debug, Serialize, Deserialize)]
pub struct VoucherItemDetail {
    pub voucher_id: i32,
    pub voucher_no: String,
    pub voucher_date: chrono::NaiveDate,
    pub line_no: i32,
    pub subject_code: String,
    pub subject_name: Option<String>,
    pub summary: Option<String>,
    pub debit: Decimal,
    pub credit: Decimal,
    pub source_type: Option<String>,
    pub source_module: Option<String>,
    pub source_bill_id: Option<i32>,
    pub source_bill_no: Option<String>,
}
