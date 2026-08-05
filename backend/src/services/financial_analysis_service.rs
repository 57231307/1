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
    // V15 P1 17.5-D2/D3/D4 扩展字段
    /// 毛利 = 销售收入 - 主营业务成本
    gross_profit: Decimal,
    /// 营业利润 = 毛利 - 销售费用 - 管理费用 - 财务费用 + 投资收益
    operating_profit: Decimal,
    /// 净利润 = 营业利润 + 营业外收入 - 营业外支出 - 所得税
    net_profit: Decimal,
    /// 所有者权益合计（4xxx）
    total_equity: Decimal,
    /// 销售费用（6601）
    selling_expenses: Decimal,
    /// 管理费用（6602）
    administrative_expenses: Decimal,
    /// 财务费用（6603）
    financial_expenses: Decimal,
    /// 营业外收入（6301）
    non_operating_income: Decimal,
    /// 营业外支出（6711）
    non_operating_expenses: Decimal,
    /// 所得税费用（6801）
    income_tax_expense: Decimal,
    // V15 P2 17.5-D6：现金流字段
    /// 经营活动现金流入（6001 主营 + 6051 其他业务）
    operating_cash_inflow: Decimal,
    /// 经营活动现金流出（6401 主营成本 + 6402 其他业务成本 + 6601/6602/6603 费用）
    operating_cash_outflow: Decimal,
    /// 经营活动现金流量净额
    operating_cash_flow: Decimal,
    /// 投资活动现金流入（6111 投资收益）
    investing_cash_inflow: Decimal,
    /// 投资活动现金流出
    investing_cash_outflow: Decimal,
    /// 筹资活动现金流入
    financing_cash_inflow: Decimal,
    /// 筹资活动现金流出（6603 财务费用中的利息支出）
    financing_cash_outflow: Decimal,
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
    } else if code.starts_with('4') {
        // V15 P1 17.5-D2：所有者权益合计（4xxx）
        summary.total_equity += (-net_balance).max(Decimal::ZERO);
    } else if code.starts_with('6') {
        if code.starts_with("6001") {
            summary.sales_revenue += (-net_balance).max(Decimal::ZERO);
        }
        if code.starts_with("6401") {
            summary.purchase_cost += net_balance.max(Decimal::ZERO);
        }
        // V15 P1 17.5-D3：费用类科目
        if code.starts_with("6601") {
            summary.selling_expenses += net_balance.max(Decimal::ZERO);
        }
        if code.starts_with("6602") {
            summary.administrative_expenses += net_balance.max(Decimal::ZERO);
        }
        if code.starts_with("6603") {
            summary.financial_expenses += net_balance.max(Decimal::ZERO);
        }
        if code.starts_with("6301") {
            summary.non_operating_income += (-net_balance).max(Decimal::ZERO);
        }
        if code.starts_with("6711") {
            summary.non_operating_expenses += net_balance.max(Decimal::ZERO);
        }
        if code.starts_with("6801") {
            summary.income_tax_expense += net_balance.max(Decimal::ZERO);
        }
        // V15 P2 17.5-D6：现金流分类
        if code.starts_with("6001") || code.starts_with("6051") {
            summary.operating_cash_inflow += (-net_balance).max(Decimal::ZERO);
        }
        if code.starts_with("6401")
            || code.starts_with("6402")
            || code.starts_with("6601")
            || code.starts_with("6602")
            || code.starts_with("6603")
        {
            summary.operating_cash_outflow += net_balance.max(Decimal::ZERO);
        }
        if code.starts_with("6111") {
            summary.investing_cash_inflow += (-net_balance).max(Decimal::ZERO);
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

    /// V15 P2 17.5-D5：趋势分析增强 - 线性回归 + 移动平均
    /// 返回趋势统计信息：斜率、截距、R²、3期移动平均、5期移动平均、趋势方向
    pub async fn get_trend_analysis(
        &self,
        indicator_id: i32,
        start_date: Option<&str>,
        end_date: Option<&str>,
        period: Option<&str>,
    ) -> Result<serde_json::Value, AppError> {
        // 获取趋势数据（按时间正序）
        let mut query = financial_analysis_result::Entity::find()
            .filter(financial_analysis_result::Column::IndicatorId.eq(indicator_id));

        if let Some(p) = period {
            query = query.filter(financial_analysis_result::Column::Period.eq(p));
        }
        if let Some(s) = start_date {
            if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                query = query.filter(financial_analysis_result::Column::AnalysisDate.gte(d));
            }
        }
        if let Some(e) = end_date {
            if let Ok(d) = NaiveDate::parse_from_str(e, "%Y-%m-%d") {
                query = query.filter(financial_analysis_result::Column::AnalysisDate.lte(d));
            }
        }

        let results = query
            .order_by(financial_analysis_result::Column::AnalysisDate, Order::Asc)
            .all(&*self.db)
            .await?;

        if results.is_empty() {
            return Ok(serde_json::json!({
                "data_points": 0,
                "message": "无趋势数据",
            }));
        }

        let values: Vec<f64> = results
            .iter()
            .map(|r| r.indicator_value.to_string().parse::<f64>().unwrap_or(0.0))
            .collect();
        let n = values.len() as f64;

        // 线性回归：y = a + bx
        let x_mean = (n - 1.0) / 2.0;
        let y_mean = values.iter().sum::<f64>() / n;

        let mut ss_xy = 0.0;
        let mut ss_xx = 0.0;
        let mut ss_yy = 0.0;
        for (i, &v) in values.iter().enumerate() {
            let x = i as f64;
            ss_xy += (x - x_mean) * (v - y_mean);
            ss_xx += (x - x_mean) * (x - x_mean);
            ss_yy += (v - y_mean) * (v - y_mean);
        }

        let slope = if ss_xx > 0.0 { ss_xy / ss_xx } else { 0.0 };
        let intercept = y_mean - slope * x_mean;
        let r_squared = if ss_xx > 0.0 && ss_yy > 0.0 {
            (ss_xy * ss_xy) / (ss_xx * ss_yy)
        } else {
            0.0
        };

        // 移动平均
        let ma3 = Self::moving_average(&values, 3);
        let ma5 = Self::moving_average(&values, 5);

        // 趋势方向判断
        let trend_direction = if slope > 0.01 {
            "上升"
        } else if slope < -0.01 {
            "下降"
        } else {
            "平稳"
        };

        Ok(serde_json::json!({
            "data_points": results.len(),
            "linear_regression": {
                "slope": slope,
                "intercept": intercept,
                "r_squared": r_squared,
            },
            "moving_average": {
                "ma3": ma3,
                "ma5": ma5,
            },
            "trend_direction": trend_direction,
            "latest_value": values.last(),
            "period_range": {
                "start": results.first().map(|r| r.period.clone()),
                "end": results.last().map(|r| r.period.clone()),
            },
        }))
    }

    /// 计算移动平均
    fn moving_average(values: &[f64], window: usize) -> Vec<Option<f64>> {
        if values.len() < window {
            return vec![None; values.len()];
        }
        let mut result = vec![None; window - 1];
        for i in (window - 1)..values.len() {
            let sum: f64 = values[(i - window + 1)..=i].iter().sum();
            result.push(Some(sum / window as f64));
        }
        result
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
        info!(
            "财务指标计算完成，期间: {}，共计算 {} 个指标",
            period,
            results.len()
        );
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
        let subjects = account_subject::Entity::find()
            .limit(10_000)
            .all(&*self.db)
            .await?;
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
            (
                "CURRENT_RATIO",
                Self::safe_div(summary.current_assets, summary.current_liabilities),
                Some(Decimal::from(2)),
            ),
            (
                "QUICK_RATIO",
                Self::safe_div(
                    summary.current_assets - summary.inventory,
                    summary.current_liabilities,
                ),
                Some(Decimal::from(1)),
            ),
            (
                "DEBT_ASSET_RATIO",
                Self::safe_div(summary.total_liabilities, summary.total_assets),
                Some(Decimal::new(60, 2)),
            ),
            (
                "AR_TURNOVER_RATIO",
                Self::safe_div(summary.sales_revenue, summary.accounts_receivable),
                None,
            ),
            (
                "AP_TURNOVER_RATIO",
                Self::safe_div(summary.purchase_cost, summary.accounts_payable),
                None,
            ),
        ];
        let mut results = Vec::new();
        for (code, value, threshold) in ratios {
            self.try_save_indicator(
                indicator_defs,
                code,
                period,
                value,
                threshold,
                user_id,
                &mut results,
            )
            .await?;
        }
        Ok(results)
    }

    /// V15 P2 17.5-D6：计算现金流比率
    /// 经营活动现金流量比率 = 经营活动现金流量净额 / 流动负债
    /// 销售现金比率 = 经营活动现金流量净额 / 销售收入
    /// 现金流量充足率 = 经营活动现金流量净额 / (资本支出 + 存货增加 + 现金股利)
    pub async fn calculate_cash_flow_ratios(
        &self,
        period: &str,
        user_id: i32,
    ) -> Result<Vec<financial_analysis_result::Model>, AppError> {
        info!("开始计算现金流比率，期间: {}", period);
        let (balances, subject_map) = self.fetch_period_balances(period).await?;
        let mut summary = aggregate_balance_summary(&balances, &subject_map);

        // 计算经营活动现金流量净额
        summary.operating_cash_flow =
            summary.operating_cash_inflow - summary.operating_cash_outflow;

        if summary.sales_revenue.is_zero() {
            summary.sales_revenue = self.fallback_sales_revenue_from_ar().await?;
        }

        let ratios: [(&str, Option<Decimal>, Option<Decimal>); 3] = [
            (
                "OPERATING_CF_RATIO",
                Self::safe_div(summary.operating_cash_flow, summary.current_liabilities),
                Some(Decimal::new(40, 2)),
            ),
            (
                "SALES_CF_RATIO",
                Self::safe_div(summary.operating_cash_flow, summary.sales_revenue),
                Some(Decimal::new(20, 2)),
            ),
            (
                "CF_ADEQUACY_RATIO",
                Self::safe_div(
                    summary.operating_cash_flow,
                    summary.inventory + summary.selling_expenses + summary.administrative_expenses,
                ),
                Some(Decimal::from(1)),
            ),
        ];

        // 确保指标定义存在，然后获取所有指标定义
        self.ensure_cash_flow_indicator_definitions(user_id)
            .await?;
        let indicator_defs = financial_analysis::Entity::find()
            .filter(financial_analysis::Column::Status.eq(master_data::ACTIVE))
            .all(&*self.db)
            .await?;
        let mut results = Vec::new();
        for (code, value, target) in ratios {
            self.try_save_indicator(
                indicator_defs,
                code,
                period,
                value,
                target,
                user_id,
                &mut results,
            )
            .await?;
        }
        info!("现金流比率计算完成，期间: {}，共 {} 个指标", period, results.len());
        Ok(results)
    }

    /// 确保现金流指标定义存在
    async fn ensure_cash_flow_indicator_definitions(
        &self,
        _user_id: i32,
    ) -> Result<(), AppError> {
        let definitions = vec![
            (
                "OPERATING_CF_RATIO",
                "经营活动现金流量比率",
                "现金流",
                "经营活动现金流量净额 / 流动负债",
                "%",
            ),
            (
                "SALES_CF_RATIO",
                "销售现金比率",
                "现金流",
                "经营活动现金流量净额 / 销售收入",
                "%",
            ),
            (
                "CF_ADEQUACY_RATIO",
                "现金流量充足率",
                "现金流",
                "经营活动现金流量净额 / (资本支出 + 存货增加 + 现金股利)",
                "比率",
            ),
        ];

        for (code, name, type_, formula, unit) in definitions {
            let existing = financial_analysis::Entity::find()
                .filter(financial_analysis::Column::IndicatorCode.eq(code))
                .one(&*self.db)
                .await?;

            if existing.is_none() {
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
                active.insert(&*self.db).await?;
                info!("自动创建现金流指标定义: {} ({})", name, code);
            }
        }

        Ok(())
    }

    /// 安全除法（分母为 0 返回 None，否则四舍五入保留 4 位）
    fn safe_div(numerator: Decimal, denominator: Decimal) -> Option<Decimal> {
        if denominator.is_zero() {
            None
        } else {
            Some(
                (numerator / denominator)
                    .round_dp_with_strategy(4, RoundingStrategy::MidpointAwayFromZero),
            )
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

    /// V15 P1 17.5-D2：杜邦分析
    /// ROE = 净利率 × 总资产周转率 × 权益乘数；净利率 = 净利润 / 销售收入；总资产周转率 = 销售收入 / 总资产；权益乘数 = 总资产 / 所有者权益；返回杜邦分析结果，包含各分解指标与最终 ROE
    pub async fn dupont_analysis(
        &self,
        period: &str,
        user_id: i32,
    ) -> Result<DuPontAnalysisResult, AppError> {
        info!("开始杜邦分析，期间: {}", period);
        let (balances, subject_map) = self.fetch_period_balances(period).await?;
        let mut summary = aggregate_balance_summary(&balances, &subject_map);
        if summary.sales_revenue.is_zero() {
            summary.sales_revenue = self.fallback_sales_revenue_from_ar().await?;
        }
        // 计算派生指标
        summary.gross_profit = summary.sales_revenue - summary.purchase_cost;
        summary.operating_profit = summary.gross_profit
            - summary.selling_expenses
            - summary.administrative_expenses
            - summary.financial_expenses;
        summary.net_profit = summary.operating_profit + summary.non_operating_income
            - summary.non_operating_expenses
            - summary.income_tax_expense;

        let net_margin = Self::safe_div(summary.net_profit, summary.sales_revenue);
        let asset_turnover = Self::safe_div(summary.sales_revenue, summary.total_assets);
        let equity_multiplier = Self::safe_div(summary.total_assets, summary.total_equity);
        let roe = match (net_margin, asset_turnover, equity_multiplier) {
            (Some(nm), Some(at), Some(em)) => Some(nm * at * em),
            _ => None,
        };

        let result = DuPontAnalysisResult {
            period: period.to_string(),
            net_margin,
            asset_turnover,
            equity_multiplier,
            roe,
            net_profit: summary.net_profit,
            sales_revenue: summary.sales_revenue,
            total_assets: summary.total_assets,
            total_equity: summary.total_equity,
        };

        // 持久化 ROE 指标结果
        if let Some(roe_value) = roe {
            self.ensure_dupont_indicator_definitions(user_id).await?;
            let roe_indicator = financial_analysis::Entity::find()
                .filter(financial_analysis::Column::IndicatorCode.eq("ROE"))
                .one(&*self.db)
                .await?;
            if let Some(indicator) = roe_indicator {
                let _ = self
                    .save_indicator_result("dupont", period, indicator.id, roe_value, None, user_id)
                    .await?;
            }
        }
        Ok(result)
    }

    /// V15 P1 17.5-D3：盈利能力比率计算
    /// 计算毛利率、净利率、营业利润率三项盈利能力指标：毛利率 = (销售收入 - 主营业务成本) / 销售收入；营业利润率 = 营业利润 / 销售收入；净利率 = 净利润 / 销售收入
    pub async fn calculate_profitability_ratios(
        &self,
        period: &str,
        user_id: i32,
    ) -> Result<Vec<financial_analysis_result::Model>, AppError> {
        info!("开始盈利能力比率计算，期间: {}", period);
        let (balances, subject_map) = self.fetch_period_balances(period).await?;
        let mut summary = aggregate_balance_summary(&balances, &subject_map);
        if summary.sales_revenue.is_zero() {
            summary.sales_revenue = self.fallback_sales_revenue_from_ar().await?;
        }
        summary.gross_profit = summary.sales_revenue - summary.purchase_cost;
        summary.operating_profit = summary.gross_profit
            - summary.selling_expenses
            - summary.administrative_expenses
            - summary.financial_expenses;
        summary.net_profit = summary.operating_profit + summary.non_operating_income
            - summary.non_operating_expenses
            - summary.income_tax_expense;

        let ratios: [(&str, Option<Decimal>, Option<Decimal>); 3] = [
            (
                "GROSS_MARGIN",
                Self::safe_div(summary.gross_profit, summary.sales_revenue),
                Some(Decimal::new(30, 2)),
            ),
            (
                "OPERATING_MARGIN",
                Self::safe_div(summary.operating_profit, summary.sales_revenue),
                Some(Decimal::new(15, 2)),
            ),
            (
                "NET_MARGIN",
                Self::safe_div(summary.net_profit, summary.sales_revenue),
                Some(Decimal::new(10, 2)),
            ),
        ];

        // 确保指标定义存在
        self.ensure_profitability_indicator_definitions(user_id)
            .await?;
        let indicator_defs = self.ensure_indicator_definitions(user_id).await?;
        let mut results = Vec::new();
        for (code, value, target) in ratios {
            self.try_save_indicator(
                &indicator_defs,
                code,
                period,
                value,
                target,
                user_id,
                &mut results,
            )
            .await?;
        }
        Ok(results)
    }

    /// V15 P1 17.5-D4：发展能力比率计算
    /// 计算收入增长率、利润增长率、资产增长率：收入增长率 = (本期收入 - 上期收入) / 上期收入 × 100%；利润增长率 = (本期净利润 - 上期净利润) / 上期净利润 × 100%；资产增长率 = (本期总资产 - 上期总资产) / 上期总资产 × 100%
    pub async fn calculate_development_ratios(
        &self,
        period: &str,
        user_id: i32,
    ) -> Result<Vec<financial_analysis_result::Model>, AppError> {
        info!("开始发展能力比率计算，期间: {}", period);
        let (current_balances, subject_map) = self.fetch_period_balances(period).await?;
        let mut current = aggregate_balance_summary(&current_balances, &subject_map);
        if current.sales_revenue.is_zero() {
            current.sales_revenue = self.fallback_sales_revenue_from_ar().await?;
        }
        current.gross_profit = current.sales_revenue - current.purchase_cost;
        current.operating_profit = current.gross_profit
            - current.selling_expenses
            - current.administrative_expenses
            - current.financial_expenses;
        current.net_profit = current.operating_profit + current.non_operating_income
            - current.non_operating_expenses
            - current.income_tax_expense;

        // 获取上期期间
        let prior_period = Self::prior_period(period)?;
        let (prior_balances, _) = self.fetch_period_balances(&prior_period).await?;
        let prior = aggregate_balance_summary(&prior_balances, &subject_map);

        let revenue_growth = Self::calc_growth_rate(current.sales_revenue, prior.sales_revenue);
        let profit_growth = Self::calc_growth_rate(current.net_profit, prior.net_profit);
        let asset_growth = Self::calc_growth_rate(current.total_assets, prior.total_assets);

        let ratios: [(&str, Option<Decimal>, Option<Decimal>); 3] = [
            (
                "REVENUE_GROWTH_RATE",
                revenue_growth,
                Some(Decimal::new(10, 2)),
            ),
            (
                "PROFIT_GROWTH_RATE",
                profit_growth,
                Some(Decimal::new(10, 2)),
            ),
            ("ASSET_GROWTH_RATE", asset_growth, Some(Decimal::new(10, 2))),
        ];

        self.ensure_development_indicator_definitions(user_id)
            .await?;
        let indicator_defs = self.ensure_indicator_definitions(user_id).await?;
        let mut results = Vec::new();
        for (code, value, target) in ratios {
            self.try_save_indicator(
                &indicator_defs,
                code,
                period,
                value,
                target,
                user_id,
                &mut results,
            )
            .await?;
        }
        Ok(results)
    }

    /// 计算增长率 = (本期 - 上期) / 上期 × 100%
    fn calc_growth_rate(current: Decimal, prior: Decimal) -> Option<Decimal> {
        if prior.is_zero() {
            return None;
        }
        Some(
            ((current - prior) / prior * Decimal::from(100))
                .round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero),
        )
    }

    /// 计算上期期间（YYYY-MM → 上一个月）
    fn prior_period(period: &str) -> Result<String, AppError> {
        let parts: Vec<&str> = period.split('-').collect();
        if parts.len() != 2 {
            return Err(AppError::validation(
                "期间格式错误，应为 YYYY-MM".to_string(),
            ));
        }
        let year: i32 = parts[0]
            .parse()
            .map_err(|_| AppError::validation("年份解析错误"))?;
        let month: u32 = parts[1]
            .parse()
            .map_err(|_| AppError::validation("月份解析错误"))?;
        if month == 1 {
            Ok(format!("{:04}-12", year - 1))
        } else {
            Ok(format!("{:04}-{:02}", year, month - 1))
        }
    }

    /// V15 P1 17.5-D2：确保 ROE 指标定义存在
    async fn ensure_dupont_indicator_definitions(&self, _user_id: i32) -> Result<(), AppError> {
        let definitions = vec![(
            "ROE",
            "净资产收益率",
            "盈利能力",
            "净利率 × 总资产周转率 × 权益乘数",
            "%",
        )];
        for (code, name, type_, formula, unit) in definitions {
            let existing = financial_analysis::Entity::find()
                .filter(financial_analysis::Column::IndicatorCode.eq(code))
                .one(&*self.db)
                .await?;
            if existing.is_none() {
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
                active.insert(&*self.db).await?;
            }
        }
        Ok(())
    }

    /// V15 P1 17.5-D3：确保盈利能力指标定义存在
    async fn ensure_profitability_indicator_definitions(
        &self,
        _user_id: i32,
    ) -> Result<(), AppError> {
        let definitions = vec![
            (
                "GROSS_MARGIN",
                "毛利率",
                "盈利能力",
                "(销售收入 - 主营业务成本) / 销售收入",
                "%",
            ),
            (
                "OPERATING_MARGIN",
                "营业利润率",
                "盈利能力",
                "营业利润 / 销售收入",
                "%",
            ),
            ("NET_MARGIN", "净利率", "盈利能力", "净利润 / 销售收入", "%"),
        ];
        for (code, name, type_, formula, unit) in definitions {
            let existing = financial_analysis::Entity::find()
                .filter(financial_analysis::Column::IndicatorCode.eq(code))
                .one(&*self.db)
                .await?;
            if existing.is_none() {
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
                active.insert(&*self.db).await?;
            }
        }
        Ok(())
    }

    /// V15 P1 17.5-D4：确保发展能力指标定义存在
    async fn ensure_development_indicator_definitions(
        &self,
        _user_id: i32,
    ) -> Result<(), AppError> {
        let definitions = vec![
            (
                "REVENUE_GROWTH_RATE",
                "收入增长率",
                "发展能力",
                "(本期收入 - 上期收入) / 上期收入 × 100%",
                "%",
            ),
            (
                "PROFIT_GROWTH_RATE",
                "利润增长率",
                "发展能力",
                "(本期净利润 - 上期净利润) / 上期净利润 × 100%",
                "%",
            ),
            (
                "ASSET_GROWTH_RATE",
                "资产增长率",
                "发展能力",
                "(本期总资产 - 上期总资产) / 上期总资产 × 100%",
                "%",
            ),
        ];
        for (code, name, type_, formula, unit) in definitions {
            let existing = financial_analysis::Entity::find()
                .filter(financial_analysis::Column::IndicatorCode.eq(code))
                .one(&*self.db)
                .await?;
            if existing.is_none() {
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
                active.insert(&*self.db).await?;
            }
        }
        Ok(())
    }
}

/// V15 P1 17.5-D2：杜邦分析结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct DuPontAnalysisResult {
    /// 期间
    pub period: String,
    /// 净利率 = 净利润 / 销售收入
    pub net_margin: Option<Decimal>,
    /// 总资产周转率 = 销售收入 / 总资产
    pub asset_turnover: Option<Decimal>,
    /// 权益乘数 = 总资产 / 所有者权益
    pub equity_multiplier: Option<Decimal>,
    /// 净资产收益率 ROE = 净利率 × 总资产周转率 × 权益乘数
    pub roe: Option<Decimal>,
    /// 净利润
    pub net_profit: Decimal,
    /// 销售收入
    pub sales_revenue: Decimal,
    /// 总资产
    pub total_assets: Decimal,
    /// 所有者权益
    pub total_equity: Decimal,
}
