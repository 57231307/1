#![allow(dead_code)]
use crate::models::financial_analysis;
use crate::models::financial_analysis_result;
use crate::utils::error::AppError;
use chrono::Utc;
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
            .offset((params.page * params.page_size) as u64)
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
            status: Set("active".to_string()),
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
    ) -> Result<Vec<financial_analysis_result::Model>, AppError> {
        info!(
            "查询财务指标 {} 的趋势数据，限制：{} 条",
            indicator_id, limit
        );

        let results = financial_analysis_result::Entity::find()
            .filter(financial_analysis_result::Column::IndicatorId.eq(indicator_id))
            .order_by(financial_analysis_result::Column::AnalysisDate, Order::Desc)
            .limit(limit as u64)
            .all(&*self.db)
            .await?;

        info!("查询到 {} 条趋势数据", results.len());
        Ok(results)
    }

    /// 计算财务指标（核心方法）
    ///
    /// 根据科目余额和应收应付数据，自动计算以下指标：
    /// - 流动比率 = 流动资产 / 流动负债
    /// - 速动比率 = (流动资产 - 存货) / 流动负债
    /// - 资产负债率 = 总负债 / 总资产
    /// - 应收账款周转率 = 销售收入 / 平均应收账款
    /// - 应付账款周转率 = 采购成本 / 平均应付账款
    pub async fn calculate_indicators(
        &self,
        period: &str,
        user_id: i32,
    ) -> Result<Vec<financial_analysis_result::Model>, AppError> {
        info!("开始计算财务指标，期间: {}", period);

        use crate::models::account_balance;
        use crate::models::account_subject;

        // 获取该期间所有科目余额
        let balances = account_balance::Entity::find()
            .filter(account_balance::Column::Period.eq(period))
            .all(&*self.db)
            .await?;

        // 获取所有科目信息
        let subjects = account_subject::Entity::find()
            .all(&*self.db)
            .await?;

        // 构建科目 ID -> 科目信息的映射
        let subject_map: std::collections::HashMap<i32, &account_subject::Model> =
            subjects.iter().map(|s| (s.id, s)).collect();

        // 按科目代码前缀分类汇总期末余额
        let mut current_assets = Decimal::ZERO;     // 流动资产 (1xxx，排除长期资产)
        let mut current_liabilities = Decimal::ZERO; // 流动负债 (2xxx)
        let mut total_assets = Decimal::ZERO;        // 总资产 (1xxx)
        let mut total_liabilities = Decimal::ZERO;   // 总负债 (2xxx)
        let mut inventory = Decimal::ZERO;           // 存货 (1403, 1405, 1406, 1407, 1408, 1409, 1411)
        let mut accounts_receivable = Decimal::ZERO;  // 应收账款 (1122)
        let mut accounts_payable = Decimal::ZERO;     // 应付账款 (2202)
        let mut sales_revenue = Decimal::ZERO;        // 销售收入 (6001 主营业务收入)
        let mut purchase_cost = Decimal::ZERO;        // 采购成本 (6001 对应的借方或 6401 主营业务成本)

        for balance in &balances {
            if let Some(subject) = subject_map.get(&balance.subject_id) {
                let code = &subject.code;
                // 计算净余额（借方减贷方）
                let net_balance = balance.ending_balance_debit - balance.ending_balance_credit;

                // 按科目代码前缀分类
                if code.starts_with('1') {
                    // 资产类
                    total_assets += net_balance.max(Decimal::ZERO);

                    // 流动资产（排除 16xx 固定资产、17xx 无形资产、18xx 长期待摊等）
                    if !code.starts_with("16")
                        && !code.starts_with("17")
                        && !code.starts_with("18")
                        && !code.starts_with("19")
                    {
                        current_assets += net_balance.max(Decimal::ZERO);
                    }

                    // 存货科目
                    if code.starts_with("1403")
                        || code.starts_with("1405")
                        || code.starts_with("1406")
                        || code.starts_with("1407")
                        || code.starts_with("1408")
                        || code.starts_with("1409")
                        || code.starts_with("1411")
                    {
                        inventory += net_balance.max(Decimal::ZERO);
                    }

                    // 应收账款
                    if code == "1122" {
                        accounts_receivable += net_balance.max(Decimal::ZERO);
                    }
                } else if code.starts_with('2') {
                    // 负债类（贷方余额为正）
                    let liability_balance = (-net_balance).max(Decimal::ZERO);
                    total_liabilities += liability_balance;

                    // 流动负债（排除 25xx 长期借款等）
                    if !code.starts_with("25") && !code.starts_with("26") {
                        current_liabilities += liability_balance;
                    }

                    // 应付账款
                    if code == "2202" {
                        accounts_payable += liability_balance;
                    }
                } else if code.starts_with('6') {
                    // 损益类
                    if code.starts_with("6001") {
                        // 主营业务收入（贷方余额为正）
                        sales_revenue += (-net_balance).max(Decimal::ZERO);
                    }
                    if code.starts_with("6401") {
                        // 主营业务成本（借方余额为正）
                        purchase_cost += net_balance.max(Decimal::ZERO);
                    }
                }
            }
        }

        // 如果科目余额中无销售收入/采购成本，尝试从应收/应付单据获取
        if sales_revenue.is_zero() {
            use crate::models::ar_invoice;
            let ar_total: Option<Decimal> = ar_invoice::Entity::find()
                .filter(ar_invoice::Column::Status.ne("CANCELLED"))
                .all(&*self.db)
                .await?
                .iter()
                .map(|inv| Some(inv.invoice_amount))
                .reduce(|a, b| Some(a.unwrap_or_default() + b.unwrap_or_default()))
                .unwrap_or(None);
            sales_revenue = ar_total.unwrap_or(Decimal::ZERO);
        }

        if purchase_cost.is_zero() {
            use crate::models::ap_invoice;
            let ap_total: Option<Decimal> = ap_invoice::Entity::find()
                .filter(ap_invoice::Column::InvoiceStatus.ne("CANCELLED"))
                .all(&*self.db)
                .await?
                .iter()
                .map(|inv| Some(inv.amount))
                .reduce(|a, b| Some(a.unwrap_or_default() + b.unwrap_or_default()))
                .unwrap_or(None);
            purchase_cost = ap_total.unwrap_or(Decimal::ZERO);
        }

        // 定义精度处理闭包
        let safe_div = |numerator: Decimal, denominator: Decimal| -> Option<Decimal> {
            if denominator.is_zero() {
                None
            } else {
                Some(
                    (numerator / denominator)
                        .round_dp_with_strategy(4, RoundingStrategy::MidpointAwayFromZero),
                )
            }
        };

        // 构建指标计算结果
        let mut results = Vec::new();

        // 获取或创建指标定义
        let indicator_defs = self.ensure_indicator_definitions(user_id).await?;

        // 1. 流动比率
        if let Some(indicator) = indicator_defs.iter().find(|i| i.indicator_code == "CURRENT_RATIO") {
            if let Some(value) = safe_div(current_assets, current_liabilities) {
                let result = self
                    .save_indicator_result(
                        "auto",
                        period,
                        indicator.id,
                        value,
                        Some(Decimal::from(2)), // 目标值 2
                        user_id,
                    )
                    .await?;
                results.push(result);
            }
        }

        // 2. 速动比率
        if let Some(indicator) = indicator_defs.iter().find(|i| i.indicator_code == "QUICK_RATIO") {
            if let Some(value) = safe_div(current_assets - inventory, current_liabilities) {
                let result = self
                    .save_indicator_result(
                        "auto",
                        period,
                        indicator.id,
                        value,
                        Some(Decimal::from(1)), // 目标值 1
                        user_id,
                    )
                    .await?;
                results.push(result);
            }
        }

        // 3. 资产负债率
        if let Some(indicator) = indicator_defs.iter().find(|i| i.indicator_code == "DEBT_ASSET_RATIO") {
            if let Some(value) = safe_div(total_liabilities, total_assets) {
                let result = self
                    .save_indicator_result(
                        "auto",
                        period,
                        indicator.id,
                        value,
                        Some(Decimal::try_new(60, 2).unwrap()), // 目标值 60%
                        user_id,
                    )
                    .await?;
                results.push(result);
            }
        }

        // 4. 应收账款周转率
        if let Some(indicator) = indicator_defs.iter().find(|i| i.indicator_code == "AR_TURNOVER_RATIO") {
            if let Some(value) = safe_div(sales_revenue, accounts_receivable) {
                let result = self
                    .save_indicator_result(
                        "auto",
                        period,
                        indicator.id,
                        value,
                        None,
                        user_id,
                    )
                    .await?;
                results.push(result);
            }
        }

        // 5. 应付账款周转率
        if let Some(indicator) = indicator_defs.iter().find(|i| i.indicator_code == "AP_TURNOVER_RATIO") {
            if let Some(value) = safe_div(purchase_cost, accounts_payable) {
                let result = self
                    .save_indicator_result(
                        "auto",
                        period,
                        indicator.id,
                        value,
                        None,
                        user_id,
                    )
                    .await?;
                results.push(result);
            }
        }

        info!(
            "财务指标计算完成，期间: {}，共计算 {} 个指标",
            period,
            results.len()
        );
        Ok(results)
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
                        status: Set("active".to_string()),
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
