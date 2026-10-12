// 保留旧路径 re-export，避免外部引用断裂
pub use crate::models::dto::sales_analysis_dto::*;
use crate::models::sales_analysis;
// 硬编码 "active" 替换为 master_data 常量
use crate::models::status::master_data;
use crate::services::bi_analysis_service::{BiAnalysisService, TimeSeriesPoint};
use crate::utils::cache::AppCache;
use crate::utils::data_scope::DataScopeContext;
use crate::utils::error::AppError;
use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::Serialize;
use std::sync::Arc;
use tracing::{info, warn};

/// 趋势分桶粒度词表。与聚合实现点 `bi_analysis_ops/sales.rs::build_period_expr`
/// 的 match 分支同源，仅作入参合法性判定，不在此重写第二套分桶口径。
const TREND_GRANULARITIES: [&str; 5] = ["day", "week", "month", "quarter", "year"];

/// 趋势缺省粒度：键缺失与非法值回落共用同一取值（与同聚合实现点的兜底分支同词）
const TREND_DEFAULT_GRANULARITY: &str = "month";

/// 销售趋势查询入参（键名与 handler `TrendQuery` 一致；空串形态已由边界中间件剔除）
#[derive(Debug, Clone, Default)]
pub struct SalesTrendQueryParams {
    /// 分桶粒度：`day`/`week`/`month`/`quarter`/`year`；缺省 `month`，
    /// 非法值回落 `month` 并 `tracing::warn` 留痕（不 400、不静默）
    pub granularity: Option<String>,
    /// 起始日期 `YYYY-MM-DD`；与 `end_date` 成对，缺省两者时按粒度回看 12 桶
    pub start_date: Option<String>,
    /// 结束日期 `YYYY-MM-DD`；`end_date < start_date` → 400 `VALIDATION_ERROR`
    pub end_date: Option<String>,
    /// 桶键等值过滤（如月粒度 `2026-08`）；缺省不过滤（已提交的 Option 语义）
    pub period: Option<String>,
}

/// 销售趋势出参行 —— 按粒度分桶的时间序列（`data` 恒为数组，无销量窗口为 `[]`）。
///
/// 金额类键为固定两位小数的字符串（本仓 Decimal=字符串口径，对齐同域 targets DTO）：
/// `amount`/`profit` 的刻度与成交列 `sales_orders.total_amount` 的 DECIMAL(14,2) 一致；
/// `quantity` 亦按契约示例（"5.00"）的两位小数形态渲染。
/// Model 列 `total_amount` → 出参键 `amount` 的改名**只发生在**
/// `SalesAnalysisService::to_trend_point` 这一处，全仓禁止第二个映射点。
#[derive(Debug, Clone, Serialize)]
pub struct SalesTrendPoint {
    /// 桶键（day: `YYYY-MM-DD`、week: `IYYY-IW`、month: `YYYY-MM`、
    /// quarter: 与 BI 聚合实现点当前产出形态一致、year: `YYYY`）
    pub period: String,
    /// 该桶销售额（`sales_orders.total_amount` 求和）
    pub amount: String,
    /// 该桶订单数
    pub order_count: i64,
    /// 该桶销售数量（明细行数量求和）
    pub quantity: String,
    /// 该桶利润（销售额 − 成本，成本口径见聚合实现点业务规则注释）
    pub profit: String,
}

/// 金额类出参的字符串形态：固定两位小数。BI 聚合点 `TimeSeriesPoint` 已将 Decimal
/// 固化为 f64（`dec_to_f64` 走 to_string/parse，业务金额的有限十进制数无损），
/// 此处按成交列 DECIMAL(14,2) 的刻度还原为字符串，不输出 JSON number（财务口径禁浮点线形态）。
fn format_trend_decimal(v: f64) -> String {
    format!("{v:.2}")
}

/// 粒度入参归一：合法值原样透传；非法值回落缺省月桶并 `warn` 留痕。
/// 处置口径与同聚合实现点对未知粒度的兜底一致（回落而非 400），但绝不静默。
fn normalize_trend_granularity(raw: Option<&str>) -> String {
    match raw {
        Some(g) if TREND_GRANULARITIES.contains(&g) => g.to_string(),
        Some(g) => {
            warn!(
                granularity = %g,
                fallback = %TREND_DEFAULT_GRANULARITY,
                "销售趋势收到非法粒度，已回落缺省月桶：合法词表 {:?}，\
                 分桶表达式唯一实现见 bi_analysis_ops/sales.rs::build_period_expr",
                TREND_GRANULARITIES
            );
            TREND_DEFAULT_GRANULARITY.to_string()
        }
        None => TREND_DEFAULT_GRANULARITY.to_string(),
    }
}

/// 缺省窗口起点：按粒度回看 12 个桶，并对齐到首桶起点（保证最老一桶键完整）。
/// 仅服务缺省路径；显式窗口不做对齐，尊重调用方提交值。
fn trend_default_start(end: NaiveDate, granularity: &str) -> NaiveDate {
    let first_of_month = end.with_day(1).expect("day=1 对任意有效日期恒合法");
    match granularity {
        "day" => end - chrono::Duration::days(11),
        // 12 个 ISO 周桶：窗口长 12*7-1 天
        "week" => end - chrono::Duration::days(83),
        "month" => first_of_month
            .checked_sub_months(chrono::Months::new(11))
            .expect("当前日期回拨 11 个月必在 NaiveDate 有效范围内"),
        "quarter" => {
            // 对齐到当季首月（月序号 (m-1)/3*3+1），再回拨 11 个季度 = 33 个月
            let quarter_first_month =
                NaiveDate::from_ymd_opt(end.year(), (end.month() - 1) / 3 * 3 + 1, 1)
                    .expect("合法年内的季度首月恒存在");
            quarter_first_month
                .checked_sub_months(chrono::Months::new(33))
                .expect("当前日期回拨 33 个月必在 NaiveDate 有效范围内")
        }
        "year" => NaiveDate::from_ymd_opt(end.year() - 11, 1, 1)
            .expect("当前年份回拨 11 年必在 NaiveDate 有效范围内"),
        _ => first_of_month
            .checked_sub_months(chrono::Months::new(11))
            .expect("当前日期回拨 11 个月必在 NaiveDate 有效范围内"),
    }
}

/// 解析窗口参数为 (start, end)。
/// - 两者都缺 ⇒ 按粒度回看 12 桶；只给一半 ⇒ 400 `VALIDATION_ERROR`（成对契约）；
/// - 形态非 `YYYY-MM-DD` ⇒ 400 `VALIDATION_ERROR`（文案只描述用户自己提交的字段）；
/// - `end < start` 不在此处判——唯一判点在复用点 `sales_by_time`（400 + 同机器码），
///   避免两处门各自演化。
fn resolve_trend_window(
    start: Option<&str>,
    end: Option<&str>,
    granularity: &str,
) -> Result<(NaiveDate, NaiveDate), AppError> {
    match (start, end) {
        (None, None) => {
            let end_date = chrono::Local::now().date_naive();
            Ok((trend_default_start(end_date, granularity), end_date))
        }
        (Some(_), None) | (None, Some(_)) => Err(AppError::validation_displayable(
            "start_date 与 end_date 必须成对提供（格式 YYYY-MM-DD）",
        )),
        (Some(s), Some(e)) => Ok((
            parse_trend_date("start_date", s)?,
            parse_trend_date("end_date", e)?,
        )),
    }
}

/// 单个日期字段的形态解析：非法即 400 `VALIDATION_ERROR`，文案携带用户提交的原值。
fn parse_trend_date(field: &str, raw: &str) -> Result<NaiveDate, AppError> {
    NaiveDate::parse_from_str(raw, "%Y-%m-%d").map_err(|_| {
        AppError::validation_displayable(format!("{field} 必须为 YYYY-MM-DD 格式，实际收到：{raw}"))
    })
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

    /// 趋势查询 —— 现算 `sales_orders`、按粒度分桶的时间序列。
    ///
    /// 数据源换到 `sales_orders` 的决定性事实：`sales_statistics` 对本表实际销售**零写入方**
    /// （全仓 `sales_analysis::ActiveModel` 仅 `create_target` / `update_target` 两处，均写
    /// `statistic_type="target"`），继续读该表无论怎么过滤都结构性恒空；真实成交只由订单模块
    /// 落在 `sales_orders`。聚合本体**复用** `BiAnalysisService::sales_by_time`
    /// （自带数据范围注入、状态排除门与 5min TTL 缓存），本方法不重写第二套聚合 SQL。
    ///
    /// 处置口径：
    /// - `granularity` 非法 → 回落 `month` 并 `tracing::warn` 留痕（不 400、不静默）；
    /// - `start_date`/`end_date` 必须成对且为 `YYYY-MM-DD`，否则 400 `VALIDATION_ERROR`；
    ///   两者缺省时按粒度回看 12 桶；`end_date < start_date` 由复用点判 400 `VALIDATION_ERROR`；
    /// - `period` = 桶键等值过滤（缺省不过滤，沿用已提交的 `Option` 语义，不退回必填）；
    /// - 排序：复用点已 `ORDER BY period ASC`（桶键定宽前缀，字符串升序＝时间升序）；
    /// - 桶策略 sparse：无成交的桶不补零，窗口内真无销售 ⇒ 空数组（200，不是错误）。
    pub async fn get_trends(
        &self,
        params: SalesTrendQueryParams,
        scope: DataScopeContext,
        cache: Arc<AppCache>,
    ) -> Result<Vec<SalesTrendPoint>, AppError> {
        let granularity = normalize_trend_granularity(params.granularity.as_deref());
        let (start, end) = resolve_trend_window(
            params.start_date.as_deref(),
            params.end_date.as_deref(),
            &granularity,
        )?;
        info!(
            "查询销售趋势（现算 sales_orders 分桶），粒度：{granularity}，窗口：{start}..={end}，桶键过滤：{:?}",
            params.period
        );

        let bi = BiAnalysisService::new_with_cache(self.db.clone(), scope, cache);
        let points = bi.sales_by_time(start, end, &granularity).await?;

        let mut trends: Vec<SalesTrendPoint> =
            points.into_iter().map(Self::to_trend_point).collect();
        if let Some(p) = &params.period {
            trends.retain(|t| &t.period == p);
        }
        Ok(trends)
    }

    /// 唯一映射点：BI 聚合行 `TimeSeriesPoint` → 销售分析出参行 `SalesTrendPoint`。
    /// `total_amount→amount`、`profit_amount→profit` 的键名改名**只允许存在于本函数**，
    /// 全仓出现第二处改名（无论后端 DTO 还是前端适配层）即视为映射点失守。
    fn to_trend_point(p: TimeSeriesPoint) -> SalesTrendPoint {
        SalesTrendPoint {
            period: p.period,
            amount: format_trend_decimal(p.total_amount),
            order_count: p.order_count,
            quantity: format_trend_decimal(p.quantity),
            profit: format_trend_decimal(p.profit_amount),
        }
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
        let active_customers: i64 = customer_ids.len() as i64;

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

    /// 获取产品销售排名（v11 批次 152 P2-A：接入 dimension_type 字段；默认 "product"：按产品维度排名；自定义值（如 "product_category"）：按指定维度排名）
    pub async fn product_ranking(
        &self,
        params: ProductRankingParams,
    ) -> Result<Vec<ProductRankingItem>, AppError> {
        info!("获取产品销售排名，参数：{:?}", params);

        let limit = params.limit.unwrap_or(10);
        // v11 批次 152 P2-A：接入 dimension_type，默认 "product"
        let dimension_type = params
            .dimension_type
            .unwrap_or_else(|| "product".to_string());

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

    /// 获取客户销售排名（v11 批次 152 P2-A：接入 dimension_type 字段；默认 "customer"：按客户维度排名；自定义值（如 "customer_industry"）：按指定维度排名）
    pub async fn customer_ranking(
        &self,
        params: CustomerRankingParams,
    ) -> Result<Vec<CustomerRankingItem>, AppError> {
        info!("获取客户销售排名，参数：{:?}", params);

        let limit = params.limit.unwrap_or(10);
        // v11 批次 152 P2-A：接入 dimension_type，默认 "customer"
        let dimension_type = params
            .dimension_type
            .unwrap_or_else(|| "customer".to_string());

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
        let status = req
            .status
            .unwrap_or_else(|| master_data::ACTIVE.to_string());

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
    /// v11 批次 151 P2-A：接入 ExportParams.format 字段，直接返回 xlsx 字节流；None 或 "xlsx"：返回 xlsx 字节流（规则 3 合规）；"csv"：拒绝（规则 3 禁止 CSV 作为最终交付格式）；其他值：validation 错误
    pub async fn export_report(&self, params: ExportParams) -> Result<Vec<u8>, AppError> {
        const EXPORT_LIMIT: u64 = 10000;
        info!("导出销售分析报告，参数：{:?}", params);

        // v11 批次 151 P2-A：接入 format 字段校验
        let format = params.format.as_deref().unwrap_or("xlsx").to_lowercase();
        match format.as_str() {
            "xlsx" => {}
            "csv" => {
                return Err(AppError::validation_displayable(
                    "CSV 格式已禁用，请使用 xlsx 格式导出（规则 3 合规）",
                ));
            }
            other => {
                return Err(AppError::validation_displayable(format!(
                    "不支持的导出格式：{}，当前仅支持 xlsx",
                    other
                )));
            }
        }

        let mut query = sales_analysis::Entity::find();
        if let Some(p) = &params.period {
            query = query.filter(sales_analysis::Column::Period.eq(p));
        }
        let records = query.limit(EXPORT_LIMIT).all(&*self.db).await?;

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
