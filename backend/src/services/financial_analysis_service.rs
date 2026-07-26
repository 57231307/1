use crate::models::financial_analysis;
use crate::models::financial_analysis_result;
// 批次 211 P2-5 修复（v12 复审）：硬编码 "active" 替换为 master_data 常量
use crate::models::status::master_data;
use crate::utils::error::AppError;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use rust_decimal::RoundingStrategy;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct IndicatorQueryParams {
    pub indicator_type: Option<String>,
    pub status: Option<String>,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateIndicatorRequest {
    pub indicator_name: String,
    pub indicator_code: String,
    pub indicator_type: String,
    pub formula: Option<String>,
    pub unit: Option<String>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FinancialAnalysisRequest {
    pub analysis_type: String,
    pub period: String,
    pub indicator_id: i32,
    pub indicator_value: Decimal,
    pub target_value: Option<Decimal>,
}

pub struct FinancialAnalysisService {
    db: Arc<DatabaseConnection>,
}

/// 余额分类汇总（流动/非流动资产、负债、存货、应收应付、收入成本）
#[derive(Default)]
struct BalanceSummary {
    current_assets: Decimal,
    current_liabilities: Decimal,
    total_assets: Decimal,
    total_liabilities: Decimal,
    inventory: Decimal,
    accounts_receivable: Decimal,
    accounts_payable: Decimal,
    sales_revenue: Decimal,
    purchase_cost: Decimal,
}

type SubjectMap = std::collections::HashMap<i32, crate::models::account_subject::Model>;

/// 按科目代码前缀汇总期末余额到 BalanceSummary
fn aggregate_balance_summary(
    balances: &[crate::models::account_balance::Model],
    subject_map: &SubjectMap,
) -> BalanceSummary {
    let mut summary = BalanceSummary::default();
    for balance in balances {
        if let Some(subject) = subject_map.get(&balance.subject_id) {
            let net_balance = balance.ending_balance_debit - balance.ending_balance_credit;
            classify_balance_entry(&subject.code, net_balance, &mut summary);
        }
    }
    summary
}

/// 按科目代码前缀将单条余额分类计入 summary
fn classify_balance_entry(code: &str, net_balance: Decimal, summary: &mut BalanceSummary) {
    if code.starts_with('1') {
        summary.total_assets += net_balance.max(Decimal::ZERO);
        if !code.starts_with("16")
            && !code.starts_with("17")
            && !code.starts_with("18")
            && !code.starts_with("19")
        {
            summary.current_assets += net_balance.max(Decimal::ZERO);
        }
        if code.starts_with("1403")
            || code.starts_with("1405")
            || code.starts_with("1406")
            || code.starts_with("1407")
            || code.starts_with("1408")
            || code.starts_with("1409")
            || code.starts_with("1411")
        {
            summary.inventory += net_balance.max(Decimal::ZERO);
        }
        if code == "1122" {
            summary.accounts_receivable += net_balance.max(Decimal::ZERO);
        }
    } else if code.starts_with('2') {
        let liability_balance = (-net_balance).max(Decimal::ZERO);
        summary.total_liabilities += liability_balance;
        if !code.starts_with("25") && !code.starts_with("26") {
            summary.current_liabilities += liability_balance;
        }
        if code == "2202" {
            summary.accounts_payable += liability_balance;
        }
    } else if code.starts_with('6') {
        if code.starts_with("6001") {
            summary.sales_revenue += (-net_balance).max(Decimal::ZERO);
        }
        if code.starts_with("6401") {
            summary.purchase_cost += net_balance.max(Decimal::ZERO);
        }
    }
}

impl FinancialAnalysisService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn get_indicators_list(
        &self,
        params: IndicatorQueryParams,
    ) -> Result<(Vec<financial_analysis::Model>, u64), AppError> {
        let mut query = financial_analysis::Entity::find();

        if let Some(indicator_type) = &params.indicator_type {
            query = query.filter(financial_analysis::Column::IndicatorType.eq(indicator_type));
        }

        if let Some(status) = &params.status {
            query = query.filter(financial_analysis::Column::Status.eq(status));
        }

        let total = query.clone().count(&*self.db).await?;

        let indicators = query
            .order_by(financial_analysis::Column::Id, Order::Desc)
            // 批次 98 P2-A 修复（v5 复审）：page clamp 防 DoS
            .offset((params.page.clamp(1, 1000).saturating_sub(1) * params.page_size) as u64)
            .limit(params.page_size as u64)
            .all(&*self.db)
            .await?;

        Ok((indicators, total))
    }

    pub async fn create_indicator(
        &self,
        req: CreateIndicatorRequest,
        _user_id: i32,
    ) -> Result<financial_analysis::Model, AppError> {
        info!("正在创建财务指标：{}", req.indicator_code);

        let active_indicator = financial_analysis::ActiveModel {
            indicator_name: Set(req.indicator_name),
            indicator_code: Set(req.indicator_code),
            indicator_type: Set(req.indicator_type),
            formula: Set(req.formula),
            unit: Set(req.unit),
            status: Set(master_data::ACTIVE.to_string()),
            remark: Set(req.remark),
            ..Default::default()
        };

        let indicator = active_indicator.insert(&*self.db).await?;
        info!("财务指标创建成功：{}", indicator.indicator_code);
        Ok(indicator)
    }

    pub async fn create_analysis_result(
        &self,
        req: FinancialAnalysisRequest,
        user_id: i32,
    ) -> Result<financial_analysis_result::Model, AppError> {
        info!(
            "用户 {} 正在创建财务分析结果：类型={}, 周期={}, 指标ID={}",
            user_id, req.analysis_type, req.period, req.indicator_id
        );

        // 计算差异
        let variance = req.target_value.map(|t| req.indicator_value - t);

        // 计算差异率
        let variance_rate = req.target_value.and_then(|t| {
            if t != Decimal::ZERO {
                Some((req.indicator_value - t) / t * Decimal::from(100))
            } else {
                None
            }
        });

        // 判断趋势方向
        let trend = variance.map(|v| {
            if v > Decimal::ZERO {
                "上升".to_string()
            } else if v < Decimal::ZERO {
                "下降".to_string()
            } else {
                "持平".to_string()
            }
        });

        // 创建分析结果记录
        let active_result = financial_analysis_result::ActiveModel {
            analysis_type: Set(req.analysis_type),
            period: Set(req.period),
            indicator_id: Set(req.indicator_id),
            indicator_value: Set(req.indicator_value),
            target_value: Set(req.target_value),
            variance: Set(variance),
            variance_rate: Set(variance_rate),
            trend: Set(trend),
            analysis_date: Set(Some(Utc::now().date_naive())),
            created_by: Set(Some(user_id)),
            created_at: Set(Utc::now()),
            ..Default::default()
        };

        let result = active_result.insert(&*self.db).await?;
        info!("财务分析结果创建成功，记录ID：{}", result.id);
        Ok(result)
    }

    pub async fn get_trends(
        &self,
        indicator_id: i32,
        limit: i64,
        start_date: Option<&str>,
        end_date: Option<&str>,
        period: Option<&str>,
    ) -> Result<Vec<financial_analysis_result::Model>, AppError> {
        info!(
            "查询财务指标 {} 的趋势数据，限制：{} 条，start={:?}, end={:?}, period={:?}",
            indicator_id, limit, start_date, end_date, period
        );

        let mut query = financial_analysis_result::Entity::find()
            .filter(financial_analysis_result::Column::IndicatorId.eq(indicator_id));

        // 精确匹配 period（YYYY-MM 格式）
        if let Some(p) = period {
            query = query.filter(financial_analysis_result::Column::Period.eq(p));
        }

        // 日期范围过滤（analysis_date >= start_date）
        if let Some(s) = start_date {
            if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                query = query.filter(financial_analysis_result::Column::AnalysisDate.gte(d));
            }
        }

        // 日期范围过滤（analysis_date <= end_date）
        if let Some(e) = end_date {
            if let Ok(d) = NaiveDate::parse_from_str(e, "%Y-%m-%d") {
                query = query.filter(financial_analysis_result::Column::AnalysisDate.lte(d));
            }
        }

        let results = query
            .order_by(financial_analysis_result::Column::AnalysisDate, Order::Desc)
            .limit(limit as u64)
            .all(&*self.db)
            .await?;

        info!("查询到 {} 条趋势数据", results.len());
        Ok(results)
    }

    /// 计算财务指标（流动/速动/资产负债/应收应付周转率）
    pub async fn calculate_indicators(
        &self,
        period: &str,
        user_id: i32,
    ) -> Result<Vec<financial_analysis_result::Model>, AppError> {
        info!("开始计算财务指标，期间: {}", period);
        let (balances, subject_map) = self.fetch_period_balances(period).await?;
        let mut summary = aggregate_balance_summary(&balances, &subject_map);
        if summary.sales_revenue.is_zero() {
            summary.sales_revenue = self.fallback_sales_revenue_from_ar().await?;
        }
        if summary.purchase_cost.is_zero() {
            summary.purchase_cost = self.fallback_purchase_cost_from_ap().await?;
        }
        let indicator_defs = self.ensure_indicator_definitions(user_id).await?;
        let results = self
            .compute_indicator_results(period, user_id, &summary, &indicator_defs)
            .await?;
        info!("财务指标计算完成，期间: {}，共计算 {} 个指标", period, results.len());
        Ok(results)
    }

    /// 拉取期间科目余额与科目字典映射
    async fn fetch_period_balances(
        &self,
        period: &str,
    ) -> Result<(Vec<crate::models::account_balance::Model>, SubjectMap), AppError> {
        use crate::models::{account_balance, account_subject};
        let balances = account_balance::Entity::find()
            .filter(account_balance::Column::Period.eq(period))
            .all(&*self.db)
            .await?;
        // P3 维度 6 修复（批次 87）：补 LIMIT 兜底防止全表加载
        let subjects = account_subject::Entity::find().limit(10_000).all(&*self.db).await?;
        let subject_map = subjects.into_iter().map(|s| (s.id, s)).collect();
        Ok((balances, subject_map))
    }

    /// 应收发票汇总作为销售收入兜底
    async fn fallback_sales_revenue_from_ar(&self) -> Result<Decimal, AppError> {
        use crate::models::ar_invoice;
        let ar_total: Option<Decimal> = ar_invoice::Entity::find()
            .filter(ar_invoice::Column::Status.ne("CANCELLED"))
            .all(&*self.db)
            .await?
            .iter()
            .map(|inv| Some(inv.invoice_amount))
            .reduce(|a, b| Some(a.unwrap_or_default() + b.unwrap_or_default()))
            .unwrap_or(None);
        Ok(ar_total.unwrap_or(Decimal::ZERO))
    }

    /// 应付发票汇总作为采购成本兜底
    async fn fallback_purchase_cost_from_ap(&self) -> Result<Decimal, AppError> {
        use crate::models::ap_invoice;
        let ap_total: Option<Decimal> = ap_invoice::Entity::find()
            .filter(ap_invoice::Column::InvoiceStatus.ne("CANCELLED"))
            .all(&*self.db)
            .await?
            .iter()
            .map(|inv| Some(inv.amount))
            .reduce(|a, b| Some(a.unwrap_or_default() + b.unwrap_or_default()))
            .unwrap_or(None);
        Ok(ap_total.unwrap_or(Decimal::ZERO))
    }

    /// 计算并保存 5 个财务指标结果
    async fn compute_indicator_results(
        &self,
        period: &str,
        user_id: i32,
        summary: &BalanceSummary,
        indicator_defs: &[financial_analysis::Model],
    ) -> Result<Vec<financial_analysis_result::Model>, AppError> {
        let ratios: [(&str, Option<Decimal>, Option<Decimal>); 5] = [
            ("CURRENT_RATIO", Self::safe_div(summary.current_assets, summary.current_liabilities), Some(Decimal::from(2))),
            ("QUICK_RATIO", Self::safe_div(summary.current_assets - summary.inventory, summary.current_liabilities), Some(Decimal::from(1))),
            ("DEBT_ASSET_RATIO", Self::safe_div(summary.total_liabilities, summary.total_assets), Some(Decimal::new(60, 2))),
            ("AR_TURNOVER_RATIO", Self::safe_div(summary.sales_revenue, summary.accounts_receivable), None),
            ("AP_TURNOVER_RATIO", Self::safe_div(summary.purchase_cost, summary.accounts_payable), None),
        ];
        let mut results = Vec::new();
        for (code, value, threshold) in ratios {
            self.try_save_indicator(indicator_defs, code, period, value, threshold, user_id, &mut results)
                .await?;
        }
        Ok(results)
    }

    /// 安全除法（分母为 0 返回 None，否则四舍五入保留 4 位）
    fn safe_div(numerator: Decimal, denominator: Decimal) -> Option<Decimal> {
        if denominator.is_zero() {
            None
        } else {
            Some((numerator / denominator).round_dp_with_strategy(4, RoundingStrategy::MidpointAwayFromZero))
        }
    }

    /// 按指标代码查找定义并在 value 为 Some 时保存结果
    async fn try_save_indicator(
        &self,
        indicator_defs: &[financial_analysis::Model],
        code: &str,
        period: &str,
        value: Option<Decimal>,
        target: Option<Decimal>,
        user_id: i32,
        results: &mut Vec<financial_analysis_result::Model>,
    ) -> Result<(), AppError> {
        if let Some(indicator) = indicator_defs.iter().find(|i| i.indicator_code == code) {
            if let Some(value) = value {
                let result = self
                    .save_indicator_result("auto", period, indicator.id, value, target, user_id)
                    .await?;
                results.push(result);
            }
        }
        Ok(())
    }

    /// 确保指标定义存在，不存在则自动创建
    async fn ensure_indicator_definitions(
        &self,
        _user_id: i32,
    ) -> Result<Vec<financial_analysis::Model>, AppError> {
        let definitions = vec![
            (
                "CURRENT_RATIO",
                "流动比率",
                "偿债能力",
                "流动资产 / 流动负债",
                "比率",
            ),
            (
                "QUICK_RATIO",
                "速动比率",
                "偿债能力",
                "(流动资产 - 存货) / 流动负债",
                "比率",
            ),
            (
                "DEBT_ASSET_RATIO",
                "资产负债率",
                "偿债能力",
                "总负债 / 总资产",
                "%",
            ),
            (
                "AR_TURNOVER_RATIO",
                "应收账款周转率",
                "营运能力",
                "销售收入 / 平均应收账款",
                "次",
            ),
            (
                "AP_TURNOVER_RATIO",
                "应付账款周转率",
                "营运能力",
                "采购成本 / 平均应付账款",
                "次",
            ),
        ];

        let mut indicators = Vec::new();

        for (code, name, type_, formula, unit) in definitions {
            let existing = financial_analysis::Entity::find()
                .filter(financial_analysis::Column::IndicatorCode.eq(code))
                .one(&*self.db)
                .await?;

            let indicator = match existing {
                Some(m) => m,
                None => {
                    let active = financial_analysis::ActiveModel {
                        indicator_name: Set(name.to_string()),
                        indicator_code: Set(code.to_string()),
                        indicator_type: Set(type_.to_string()),
                        formula: Set(Some(formula.to_string())),
                        unit: Set(Some(unit.to_string())),
                        status: Set(master_data::ACTIVE.to_string()),
                        remark: Set(None),
                        ..Default::default()
                    };
                    let inserted = active.insert(&*self.db).await?;
                    info!("自动创建财务指标定义: {} ({})", name, code);
                    inserted
                }
            };

            indicators.push(indicator);
        }

        Ok(indicators)
    }

    /// 保存指标计算结果
    async fn save_indicator_result(
        &self,
        analysis_type: &str,
        period: &str,
        indicator_id: i32,
        indicator_value: Decimal,
        target_value: Option<Decimal>,
        user_id: i32,
    ) -> Result<financial_analysis_result::Model, AppError> {
        let variance = target_value.map(|t| indicator_value - t);
        let variance_rate = target_value.and_then(|t| {
            if t != Decimal::ZERO {
                Some(
                    ((indicator_value - t) / t * Decimal::from(100))
                        .round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero),
                )
            } else {
                None
            }
        });
        let trend = variance.map(|v| {
            if v > Decimal::ZERO {
                "上升".to_string()
            } else if v < Decimal::ZERO {
                "下降".to_string()
            } else {
                "持平".to_string()
            }
        });

        let active = financial_analysis_result::ActiveModel {
            analysis_type: Set(analysis_type.to_string()),
            period: Set(period.to_string()),
            indicator_id: Set(indicator_id),
            indicator_value: Set(indicator_value),
            target_value: Set(target_value),
            variance: Set(variance),
            variance_rate: Set(variance_rate),
            trend: Set(trend),
            analysis_date: Set(Some(Utc::now().date_naive())),
            created_by: Set(Some(user_id)),
            created_at: Set(Utc::now()),
            ..Default::default()
        };

        let result = active.insert(&*self.db).await?;
        Ok(result)
    }
}
