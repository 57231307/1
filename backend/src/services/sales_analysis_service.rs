use crate::models::sales_analysis;
use crate::utils::error::AppError;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Clone, Default, Serialize)]
pub struct SalesOverviewStats {
    pub month_orders: i64,
    pub month_amount: Decimal,
    pub gross_profit_rate: Decimal,
    pub active_customers: i64,
    pub order_trend: f64,
    pub amount_trend: f64,
    pub profit_trend: f64,
    pub customer_trend: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProductRankingItem {
    pub product_name: String,
    pub amount: Decimal,
    pub quantity: Decimal,
    pub percentage: Decimal,
}

#[derive(Debug, Clone, Serialize)]
pub struct CustomerRankingItem {
    pub customer_name: String,
    pub amount: Decimal,
    pub order_count: i32,
    pub percentage: Decimal,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProductRankingParams {
    #[serde(rename = "type")]
    // v11 批次 152 P2-A：接入 dimension_type 字段，指定产品排名的维度
    // - None 或 "product"：按产品维度排名（默认）
    // - 其他值（如 "product_category"）：按指定维度排名，需数据库有对应 dimension_type 记录
    pub dimension_type: Option<String>,
    pub period: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CustomerRankingParams {
    #[serde(rename = "type")]
    // v11 批次 152 P2-A：接入 dimension_type 字段，指定客户排名的维度
    // - None 或 "customer"：按客户维度排名（默认）
    // - 其他值（如 "customer_industry"）：按指定维度排名，需数据库有对应 dimension_type 记录
    pub dimension_type: Option<String>,
    pub period: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateSalesTargetRequest {
    pub target_amount: Option<Decimal>,
    pub status: Option<String>,
    pub remarks: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct SalesTargetDto {
    pub id: i32,
    pub period: String,
    pub target_amount: Decimal,
    pub actual_amount: Decimal,
    pub completion_rate: Decimal,
    pub variance: Decimal,
    pub status: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ExportParams {
    pub period: Option<String>,
    // v11 批次 151 P2-A：接入 format 字段，指定导出格式
    // - None 或 "xlsx"：xlsx 格式（默认，规则 3 合规）
    // - "csv"：拒绝（规则 3 禁止 CSV 作为最终交付格式）
    // - 其他值：validation 错误
    pub format: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SalesStatisticQueryParams {
    pub statistic_type: Option<String>,
    pub period: Option<String>,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateSalesTargetInput {
    pub target_type: String,
    pub target_id: i32,
    pub period: String,
    pub target_amount: Decimal,
    pub start_date: String,
    pub end_date: String,
}

pub struct SalesAnalysisService {
    db: Arc<DatabaseConnection>,
}

impl SalesAnalysisService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn get_statistics_list(
        &self,
        params: SalesStatisticQueryParams,
    ) -> Result<(Vec<sales_analysis::Model>, u64), AppError> {
        let mut query = sales_analysis::Entity::find();

        if let Some(statistic_type) = &params.statistic_type {
            query = query.filter(sales_analysis::Column::StatisticType.eq(statistic_type));
        }

        if let Some(period) = &params.period {
            query = query.filter(sales_analysis::Column::Period.eq(period));
        }

        let total = query.clone().count(&*self.db).await?;

        let statistics = query
            .order_by(sales_analysis::Column::Id, Order::Desc)
            // 批次 98 P2-A 修复（v5 复审）：page clamp 防 DoS
            .offset((params.page.clamp(1, 1000).saturating_sub(1) * params.page_size) as u64)
            .limit(params.page_size as u64)
            .all(&*self.db)
            .await?;

        Ok((statistics, total))
    }

    pub async fn get_trends(&self, period: &str) -> Result<Vec<sales_analysis::Model>, AppError> {
        info!("查询销售趋势，周期：{}", period);

        let trends = sales_analysis::Entity::find()
            .filter(sales_analysis::Column::Period.eq(period))
            .order_by(sales_analysis::Column::Id, Order::Desc)
            .all(&*self.db)
            .await?;

        Ok(trends)
    }

    pub async fn get_rankings(
        &self,
        period: Option<&str>,
        limit: i64,
    ) -> Result<Vec<sales_analysis::Model>, AppError> {
        info!("查询销售排名，周期：{:?}", period);

        let mut query = sales_analysis::Entity::find();

        if let Some(p) = period {
            query = query.filter(sales_analysis::Column::Period.eq(p));
        }

        let rankings = query
            .order_by(sales_analysis::Column::Id, Order::Desc)
            .limit(limit as u64)
            .all(&*self.db)
            .await?;

        Ok(rankings)
    }

    pub async fn get_targets(
        &self,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<sales_analysis::Model>, u64), AppError> {
        info!("查询销售目标列表");

        let query = sales_analysis::Entity::find()
            .filter(sales_analysis::Column::StatisticType.eq("target".to_string()));

        let total = query.clone().count(&*self.db).await?;

        let targets = query
            .order_by(sales_analysis::Column::Id, Order::Desc)
            .offset((page.saturating_sub(1) * page_size) as u64)
            .limit(page_size as u64)
            .all(&*self.db)
            .await?;

        Ok((targets, total))
    }

    pub async fn create_target(
        &self,
        req: CreateSalesTargetInput,
        _user_id: i32,
    ) -> Result<sales_analysis::Model, AppError> {
        info!("正在创建销售目标");

        let active_target = sales_analysis::ActiveModel {
            statistic_type: Set("target".to_string()),
            period: Set(req.period),
            dimension_type: Set(req.target_type),
            dimension_id: Set(Some(req.target_id)),
            dimension_name: Set(Some(format!(
                "开始日期: {}, 结束日期: {}",
                req.start_date, req.end_date
            ))),
            total_amount: Set(req.target_amount),
            ..Default::default()
        };

        let target = active_target.insert(&*self.db).await?;
        info!("销售目标创建成功，ID: {}", target.id);
        Ok(target)
    }

    /// 获取销售概览统计
    #[allow(clippy::needless_late_init)]
    pub async fn get_overview_stats(&self) -> Result<SalesOverviewStats, AppError> {
        info!("获取销售概览统计");

        // 汇总所有销售统计数据
        // P3-7 修复（批次 84 v1 复审）：加 LIMIT 兜底防止全表加载内存爆炸
        // 长期应改为数据库聚合（SUM/COUNT DISTINCT），当前 LIMIT 10000 覆盖业务场景
        let stats = sales_analysis::Entity::find()
            .limit(10_000)
            .all(&*self.db)
            .await?;

        let mut month_orders: i64 = 0;
        let mut month_amount = Decimal::ZERO;
        let mut total_profit = Decimal::ZERO;
        let mut total_amount = Decimal::ZERO;
        let mut gross_profit_rate = Decimal::ZERO;
        let active_customers: i64;

        for s in &stats {
            if s.statistic_type == "order" {
                month_orders += s.order_count as i64;
                month_amount += s.total_amount;
            }
            total_profit += s.gross_profit;
            total_amount += s.total_amount;
        }

        if total_amount > Decimal::ZERO {
            gross_profit_rate = (total_profit / total_amount)
                .round_dp_with_strategy(4, rust_decimal::RoundingStrategy::MidpointAwayFromZero);
        }

        // 统计不同维度ID作为活跃客户近似值
        let mut customer_ids: std::collections::HashSet<i32> = std::collections::HashSet::new();
        for s in &stats {
            if s.dimension_type == "customer" {
                if let Some(id) = s.dimension_id {
                    customer_ids.insert(id);
                }
            }
        }
        active_customers = customer_ids.len() as i64;

        Ok(SalesOverviewStats {
            month_orders,
            month_amount,
            gross_profit_rate,
            active_customers,
            order_trend: 0.0,
            amount_trend: 0.0,
            profit_trend: 0.0,
            customer_trend: 0.0,
        })
    }

    /// 获取产品销售排名
    ///
    /// v11 批次 152 P2-A：接入 dimension_type 字段
    /// - 默认 "product"：按产品维度排名
    /// - 自定义值（如 "product_category"）：按指定维度排名
    pub async fn product_ranking(
        &self,
        params: ProductRankingParams,
    ) -> Result<Vec<ProductRankingItem>, AppError> {
        info!("获取产品销售排名，参数：{:?}", params);

        let limit = params.limit.unwrap_or(10);
        // v11 批次 152 P2-A：接入 dimension_type，默认 "product"
        let dimension_type = params.dimension_type.unwrap_or_else(|| "product".to_string());

        let mut query = sales_analysis::Entity::find()
            .filter(sales_analysis::Column::DimensionType.eq(dimension_type));

        if let Some(p) = &params.period {
            query = query.filter(sales_analysis::Column::Period.eq(p));
        }

        let records = query
            .order_by_desc(sales_analysis::Column::TotalAmount)
            .limit(limit as u64)
            .all(&*self.db)
            .await?;

        let total: Decimal = records.iter().map(|r| r.total_amount).sum();

        let items: Vec<ProductRankingItem> = records
            .into_iter()
            .map(|r| {
                let percentage = if total > Decimal::ZERO {
                    (r.total_amount / total * Decimal::from(100)).round_dp_with_strategy(
                        2,
                        rust_decimal::RoundingStrategy::MidpointAwayFromZero,
                    )
                } else {
                    Decimal::ZERO
                };
                ProductRankingItem {
                    product_name: r.dimension_name.unwrap_or_else(|| "未知产品".to_string()),
                    amount: r.total_amount,
                    quantity: r.total_qty,
                    percentage,
                }
            })
            .collect();

        Ok(items)
    }

    /// 获取客户销售排名
    ///
    /// v11 批次 152 P2-A：接入 dimension_type 字段
    /// - 默认 "customer"：按客户维度排名
    /// - 自定义值（如 "customer_industry"）：按指定维度排名
    pub async fn customer_ranking(
        &self,
        params: CustomerRankingParams,
    ) -> Result<Vec<CustomerRankingItem>, AppError> {
        info!("获取客户销售排名，参数：{:?}", params);

        let limit = params.limit.unwrap_or(10);
        // v11 批次 152 P2-A：接入 dimension_type，默认 "customer"
        let dimension_type = params.dimension_type.unwrap_or_else(|| "customer".to_string());

        let mut query = sales_analysis::Entity::find()
            .filter(sales_analysis::Column::DimensionType.eq(dimension_type));

        if let Some(p) = &params.period {
            query = query.filter(sales_analysis::Column::Period.eq(p));
        }

        let records = query
            .order_by_desc(sales_analysis::Column::TotalAmount)
            .limit(limit as u64)
            .all(&*self.db)
            .await?;

        let total: Decimal = records.iter().map(|r| r.total_amount).sum();

        let items: Vec<CustomerRankingItem> = records
            .into_iter()
            .map(|r| {
                let percentage = if total > Decimal::ZERO {
                    (r.total_amount / total * Decimal::from(100)).round_dp_with_strategy(
                        2,
                        rust_decimal::RoundingStrategy::MidpointAwayFromZero,
                    )
                } else {
                    Decimal::ZERO
                };
                CustomerRankingItem {
                    customer_name: r.dimension_name.unwrap_or_else(|| "未知客户".to_string()),
                    amount: r.total_amount,
                    order_count: r.order_count,
                    percentage,
                }
            })
            .collect();

        Ok(items)
    }

    /// 更新销售目标
    pub async fn update_target(
        &self,
        period: &str,
        req: UpdateSalesTargetRequest,
    ) -> Result<SalesTargetDto, AppError> {
        info!("更新销售目标，周期：{}", period);

        let existing = sales_analysis::Entity::find()
            .filter(sales_analysis::Column::Period.eq(period))
            .filter(sales_analysis::Column::StatisticType.eq("target"))
            .one(&*self.db)
            .await?;

        let target_amount = req.target_amount.unwrap_or(Decimal::ZERO);
        let status = req.status.unwrap_or_else(|| "active".to_string());

        let updated = if let Some(existing_model) = existing {
            let mut active: sales_analysis::ActiveModel = existing_model.clone().into();
            active.total_amount = Set(target_amount);
            active.dimension_name = Set(req.remarks.clone().or(existing_model.dimension_name));
            active.update(&*self.db).await?
        } else {
            let active = sales_analysis::ActiveModel {
                statistic_type: Set("target".to_string()),
                period: Set(period.to_string()),
                dimension_type: Set("overall".to_string()),
                dimension_id: Set(None),
                dimension_name: Set(req.remarks),
                total_amount: Set(target_amount),
                ..Default::default()
            };
            active.insert(&*self.db).await?
        };

        let actual_amount = updated.total_amount;
        let completion_rate = if updated.total_amount > Decimal::ZERO {
            (actual_amount / updated.total_amount * Decimal::from(100))
                .round_dp_with_strategy(2, rust_decimal::RoundingStrategy::MidpointAwayFromZero)
        } else {
            Decimal::ZERO
        };
        let variance = actual_amount - updated.total_amount;

        Ok(SalesTargetDto {
            id: updated.id,
            period: updated.period,
            target_amount: updated.total_amount,
            actual_amount,
            completion_rate,
            variance,
            status,
        })
    }

    /// 导出销售分析报告
    ///
    /// v11 批次 151 P2-A：接入 ExportParams.format 字段，直接返回 xlsx 字节流
    /// - None 或 "xlsx"：返回 xlsx 字节流（规则 3 合规）
    /// - "csv"：拒绝（规则 3 禁止 CSV 作为最终交付格式）
    /// - 其他值：validation 错误
    pub async fn export_report(&self, params: ExportParams) -> Result<Vec<u8>, AppError> {
        info!("导出销售分析报告，参数：{:?}", params);

        // v11 批次 151 P2-A：接入 format 字段校验
        let format = params.format.as_deref().unwrap_or("xlsx").to_lowercase();
        match format.as_str() {
            "xlsx" => {}
            "csv" => {
                return Err(AppError::validation(
                    "CSV 格式已禁用，请使用 xlsx 格式导出（规则 3 合规）",
                ));
            }
            other => {
                return Err(AppError::validation(format!(
                    "不支持的导出格式：{}，当前仅支持 xlsx",
                    other
                )));
            }
        }

        let mut query = sales_analysis::Entity::find();
        if let Some(p) = &params.period {
            query = query.filter(sales_analysis::Column::Period.eq(p));
        }
        let records = query.all(&*self.db).await?;

        // v11 批次 151 P2-A：直接构建 xlsx 字节流，消除原 CSV 中间步骤
        // 表头与原 CSV 保持一致，确保导出字段不丢失
        let headers: Vec<String> = vec![
            "ID".to_string(),
            "统计类型".to_string(),
            "周期".to_string(),
            "维度类型".to_string(),
            "维度ID".to_string(),
            "维度名称".to_string(),
            "订单数".to_string(),
            "总金额".to_string(),
            "总数量".to_string(),
            "毛利率".to_string(),
        ];
        let rows: Vec<Vec<String>> = records
            .iter()
            .map(|r| {
                vec![
                    r.id.to_string(),
                    r.statistic_type.clone(),
                    r.period.clone(),
                    r.dimension_type.clone(),
                    r.dimension_id.map(|i| i.to_string()).unwrap_or_default(),
                    r.dimension_name.clone().unwrap_or_default(),
                    r.order_count.to_string(),
                    r.total_amount.to_string(),
                    r.total_qty.to_string(),
                    r.gross_profit_rate.to_string(),
                ]
            })
            .collect();

        let table = crate::utils::xlsx_export::XlsxTable {
            sheet_name: "销售分析报告".to_string(),
            headers,
            rows,
        };
        crate::utils::xlsx_export::build_xlsx(&table)
    }
}
