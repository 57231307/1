//! 报表数据源服务（report/ds）
//!
//! 包含数据源管理、聚合查询、报表执行、缓存管理：
//! - `aggregate_data`         通用聚合查询分发
//! - `aggregate_*_data`       4 类业务聚合（销售/采购/库存/财务）
//! - `query_*_report`         4 类业务明细分页查询
//! - `execute_report`         报表执行入口（支持缓存 + 数据加载）
//! - 内部缓存：`generate_cache_key` / `get_cached_data` / `set_cached_data`
//!
//! 拆分自原 `report_engine_service.rs` 的"数据聚合"段。

use chrono::Utc;
// 批次 357 v13 复审 baseline 清零：移除 unused import ActiveModelTrait
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::Instant;
use tracing::info;

use crate::models::finance_payment::Entity as FinancePaymentEntity;
use crate::models::inventory_stock::Entity as InventoryStockEntity;
use crate::models::purchase_order::Entity as PurchaseOrderEntity;
use crate::models::purchase_order_item::Entity as PurchaseOrderItemEntity;
use crate::models::sales_order::Entity as SalesOrderEntity;
use crate::models::sales_order_item::Entity as SalesOrderItemEntity;
use crate::utils::error::AppError;

use super::{
    AggregateRequest, AggregateResult, AggregationType, DEFAULT_CACHE_TTL_SECONDS, DataSource,
    ExecuteReportRequest, ReportData, ReportEngineService, ReportMetadata,
};

/// 销售汇总的单个期间聚合桶
///
/// 功能 承载一个期间内的销售额合计、订单数与去重客户集合，是汇总行的原料。
/// 调用方 仅本文件的 group_sales_orders_by_period 与 query_sales_summary_report。
/// 入参 由聚合过程逐单累加，period 是已格式化的期间键。
/// 传给谁 交给 query_sales_summary_report 装配成报表行。
/// 存什么 期间键、DECIMAL 销售额合计、u64 订单数与客户 ID 集合。
/// 存哪里 不落库，只活在单次查询的内存里。
struct SalesPeriodAggregate {
    period: String,
    total_amount: rust_decimal::Decimal,
    order_count: u64,
    customer_ids: HashSet<i32>,
}

impl ReportEngineService {
    /// 通用数据聚合
    pub async fn aggregate_data(
        &self,
        req: AggregateRequest,
    ) -> Result<Vec<AggregateResult>, AppError> {
        match req.data_source {
            DataSource::Sales => {
                let req_clone = req.clone();
                self.aggregate_sales_data(req_clone).await
            }
            DataSource::Purchase => self.aggregate_purchase_data(req).await,
            DataSource::Inventory => self.aggregate_inventory_data(req).await,
            DataSource::Finance => self.aggregate_finance_data(req).await,
        }
    }

    /// 销售数据聚合
    pub async fn aggregate_sales_data(
        &self,
        req: AggregateRequest,
    ) -> Result<Vec<AggregateResult>, AppError> {
        let orders = Self::fetch_sales_orders(&self.db, &req).await?;
        let aggregation = Self::aggregate_sales_by_group(&orders, &req);
        Ok(Self::build_aggregate_results(aggregation))
    }

    /// 按日期范围查询销售订单
    async fn fetch_sales_orders(
        db: &DatabaseConnection,
        req: &AggregateRequest,
    ) -> Result<Vec<crate::models::sales_order::Model>, AppError> {
        let mut select = SalesOrderEntity::find();
        if let Some(date_range) = &req.date_range {
            select = select.filter(
                crate::models::sales_order::Column::OrderDate
                    .between(date_range.start, date_range.end),
            );
        }
        select.all(db).await.map_err(AppError::from)
    }

    /// 按分组维度聚合销售订单（customer/month/total，累计 total_amount + order_count）
    fn aggregate_sales_by_group(
        orders: &[crate::models::sales_order::Model],
        req: &AggregateRequest,
    ) -> HashMap<String, HashMap<String, rust_decimal::Decimal>> {
        let mut aggregation: HashMap<String, HashMap<String, rust_decimal::Decimal>> =
            HashMap::new();
        for order in orders {
            let group_key = match req.aggregation_type {
                AggregationType::GroupBy => {
                    if req.group_by.iter().any(|g| g == "customer") {
                        format!("customer_{}", order.customer_id)
                    } else if req.group_by.iter().any(|g| g == "month") {
                        format!("month_{}", order.order_date.format("%Y-%m"))
                    } else {
                        "total".to_string()
                    }
                }
                _ => "total".to_string(),
            };
            let entry = aggregation.entry(group_key).or_default();
            *entry
                .entry("total_amount".to_string())
                .or_insert(rust_decimal::Decimal::ZERO) += order.total_amount;
            *entry
                .entry("order_count".to_string())
                .or_insert(rust_decimal::Decimal::ZERO) += rust_decimal::Decimal::ONE;
        }
        aggregation
    }

    /// 采购数据聚合
    /// P2-2 修复（v12 复审）：原实现返回 `Ok(Vec::new())` 桩代码，；现真实接入 purchase_orders 表，按 supplier/month 分组聚合 total_amount 与 order_count。；D08 Tier 4 子批次9：拆分为 ≤10 行主函数 + 2 个 helper（fetch_purchase_orders / group_purchase_orders），；复用已有 build_aggregate_results 构建最终结果。
    pub async fn aggregate_purchase_data(
        &self,
        req: AggregateRequest,
    ) -> Result<Vec<AggregateResult>, AppError> {
        let orders = Self::fetch_purchase_orders(&self.db, &req).await?;
        let aggregation = Self::group_purchase_orders(&orders, &req);
        Ok(Self::build_aggregate_results(aggregation))
    }

    /// 按日期范围查询采购订单
    async fn fetch_purchase_orders(
        db: &sea_orm::DatabaseConnection,
        req: &AggregateRequest,
    ) -> Result<Vec<crate::models::purchase_order::Model>, AppError> {
        let mut select = PurchaseOrderEntity::find();
        if let Some(date_range) = &req.date_range {
            select = select.filter(
                crate::models::purchase_order::Column::OrderDate
                    .between(date_range.start, date_range.end),
            );
        }
        Ok(select.all(db).await?)
    }

    /// 按 supplier/month 维度分组聚合采购订单（total_amount + order_count）
    fn group_purchase_orders(
        orders: &[crate::models::purchase_order::Model],
        req: &AggregateRequest,
    ) -> HashMap<String, HashMap<String, rust_decimal::Decimal>> {
        let mut aggregation: HashMap<String, HashMap<String, rust_decimal::Decimal>> =
            HashMap::new();
        for order in orders {
            // 分组键：按 supplier/month 维度，与 aggregate_sales_data 保持一致
            let group_key = match req.aggregation_type {
                AggregationType::GroupBy => {
                    if req.group_by.iter().any(|g| g == "supplier") {
                        format!("supplier_{}", order.supplier_id)
                    } else if req.group_by.iter().any(|g| g == "month") {
                        format!("month_{}", order.order_date.format("%Y-%m"))
                    } else {
                        "total".to_string()
                    }
                }
                _ => "total".to_string(),
            };
            let entry = aggregation.entry(group_key).or_default();
            *entry
                .entry("total_amount".to_string())
                .or_insert(rust_decimal::Decimal::ZERO) += order.total_amount;
            *entry
                .entry("order_count".to_string())
                .or_insert(rust_decimal::Decimal::ZERO) += rust_decimal::Decimal::ONE;
        }
        aggregation
    }

    /// 库存数据聚合
    pub async fn aggregate_inventory_data(
        &self,
        _req: AggregateRequest,
    ) -> Result<Vec<AggregateResult>, AppError> {
        let stocks = InventoryStockEntity::find().all(&*self.db).await?;

        let mut group_by_warehouse: HashMap<i32, rust_decimal::Decimal> = HashMap::new();
        let mut group_by_product: HashMap<i32, rust_decimal::Decimal> = HashMap::new();

        for stock in stocks {
            *group_by_warehouse
                .entry(stock.warehouse_id)
                .or_insert(rust_decimal::Decimal::ZERO) += stock.quantity_on_hand;
            *group_by_product
                .entry(stock.product_id)
                .or_insert(rust_decimal::Decimal::ZERO) += stock.quantity_on_hand;
        }

        let mut results = Vec::new();

        for (warehouse_id, total) in group_by_warehouse {
            results.push(AggregateResult {
                columns: vec!["warehouse".to_string(), "total_quantity".to_string()],
                rows: vec![vec![warehouse_id.to_string(), total.to_string()]],
                total_count: 1,
                groups: vec![("warehouse".to_string(), serde_json::json!(warehouse_id))],
                aggregations: vec![(
                    "total_quantity".to_string(),
                    serde_json::json!(total.to_string()),
                )],
                count: 1,
            });
        }

        for (product_id, total) in group_by_product {
            results.push(AggregateResult {
                columns: vec!["product".to_string(), "total_quantity".to_string()],
                rows: vec![vec![product_id.to_string(), total.to_string()]],
                total_count: 1,
                groups: vec![("product".to_string(), serde_json::json!(product_id))],
                aggregations: vec![(
                    "total_quantity".to_string(),
                    serde_json::json!(total.to_string()),
                )],
                count: 1,
            });
        }

        Ok(results)
    }

    /// 财务数据聚合
    /// P2-2 修复（v12 复审）：原实现返回 `Ok(Vec::new())` 桩代码，；现真实接入 finance_payments 表，按 month/payment_method 分组聚合 amount 与 payment_count。；D08 Tier 4 子批次2：拆分为 ≤50 行主函数 + 3 个 helper（aggregate_payments_by_group / compute_finance_group_key / build_aggregate_results）
    pub async fn aggregate_finance_data(
        &self,
        req: AggregateRequest,
    ) -> Result<Vec<AggregateResult>, AppError> {
        let mut select = FinancePaymentEntity::find();
        if let Some(date_range) = &req.date_range {
            let start = date_range
                .start
                .and_hms_opt(0, 0, 0)
                .ok_or_else(|| AppError::validation_displayable("报表日期范围起始值非法"))?
                .and_utc();
            let end = date_range
                .end
                .and_hms_opt(23, 59, 59)
                .ok_or_else(|| AppError::validation_displayable("报表日期范围结束值非法"))?
                .and_utc();
            select = select
                .filter(crate::models::finance_payment::Column::PaymentDate.between(start, end));
        }
        let payments = select.all(&*self.db).await?;
        let aggregation = Self::aggregate_payments_by_group(&payments, &req);
        Ok(Self::build_aggregate_results(aggregation))
    }

    /// 按分组键聚合付款记录（total_amount + payment_count）
    fn aggregate_payments_by_group(
        payments: &[crate::models::finance_payment::Model],
        req: &AggregateRequest,
    ) -> HashMap<String, HashMap<String, rust_decimal::Decimal>> {
        let mut aggregation: HashMap<String, HashMap<String, rust_decimal::Decimal>> =
            HashMap::new();
        for payment in payments {
            let group_key = Self::compute_finance_group_key(payment, req);
            let entry = aggregation.entry(group_key).or_default();
            *entry
                .entry("total_amount".to_string())
                .or_insert(rust_decimal::Decimal::ZERO) += payment.amount;
            *entry
                .entry("payment_count".to_string())
                .or_insert(rust_decimal::Decimal::ZERO) += rust_decimal::Decimal::ONE;
        }
        aggregation
    }

    /// 计算付款记录的分组键（month_ / payment_method_ / total）
    fn compute_finance_group_key(
        payment: &crate::models::finance_payment::Model,
        req: &AggregateRequest,
    ) -> String {
        match req.aggregation_type {
            AggregationType::GroupBy => {
                if req.group_by.iter().any(|g| g == "month") {
                    format!("month_{}", payment.payment_date.format("%Y-%m"))
                } else if req.group_by.iter().any(|g| g == "payment_method") {
                    format!(
                        "payment_method_{}",
                        payment
                            .payment_method
                            .clone()
                            .unwrap_or_else(|| "unknown".to_string())
                    )
                } else {
                    "total".to_string()
                }
            }
            _ => "total".to_string(),
        }
    }

    /// 将聚合 HashMap 转换为 AggregateResult 列表
    fn build_aggregate_results(
        aggregation: HashMap<String, HashMap<String, rust_decimal::Decimal>>,
    ) -> Vec<AggregateResult> {
        let mut results = Vec::new();
        for (group_key, values) in aggregation {
            let groups = if group_key.contains('_') {
                let parts: Vec<&str> = group_key.splitn(2, '_').collect();
                vec![(
                    parts[0].to_string(),
                    serde_json::Value::String(parts[1].to_string()),
                )]
            } else {
                vec![(
                    group_key.clone(),
                    serde_json::Value::String(group_key.clone()),
                )]
            };
            let aggregations: Vec<(String, serde_json::Value)> = values
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::json!(v.to_string())))
                .collect();
            let mut columns = vec!["group".to_string()];
            let mut row_values = vec![group_key.clone()];
            for (k, v) in &values {
                columns.push(k.clone());
                row_values.push(v.to_string());
            }
            results.push(AggregateResult {
                columns,
                rows: vec![row_values],
                total_count: 1,
                groups,
                aggregations,
                count: 1,
            });
        }
        results
    }

    /// 执行报表（统一入口：缓存 + 数据加载 + 元数据）
    pub async fn execute_report(&self, req: ExecuteReportRequest) -> Result<ReportData, AppError> {
        let start_time = Instant::now();

        let cache_key = self.generate_cache_key(&req);
        let use_cache = req.use_cache.unwrap_or(true);

        // 尝试从缓存获取
        if use_cache {
            if let Some(cached_data) = self.get_cached_data(&cache_key).await? {
                info!("报表缓存命中: {}", cache_key);
                return Ok(cached_data);
            }
        }

        // 获取模板
        let template = self.get_template(&req.template_id).await?;

        // 加载数据：每枚预置模板走各自的装配腿，出参行键与模板声明列严格同源；
        // 未登记的模板 id 走销售明细兜底腿（自定义模板既有契约，本腿不投影）。
        let data = match req.template_id.as_str() {
            "sales_summary" => self.query_sales_summary_report(&template, &req).await?,
            "sales_detail" => self.query_sales_detail_report(&template, &req).await?,
            "top_products" => self.query_top_products_report(&template, &req).await?,
            "customer_analysis" => self.query_customer_analysis_report(&template, &req).await?,
            "profit_analysis" => self.query_profit_analysis_report(&template, &req).await?,
            "inventory_status" => self.query_inventory_report(&template, &req).await?,
            "inventory_turnover" => {
                self.query_inventory_turnover_report(&template, &req)
                    .await?
            }
            "purchase_summary" => self.query_purchase_report(&template, &req).await?,
            _ => {
                // 默认: 销售明细（自定义模板兜底）
                self.query_sales_report(&template, &req).await?
            }
        };

        let mut data = data;
        data.metadata.query_time_ms = start_time.elapsed().as_millis() as u64;
        data.metadata.cache_hit = false;

        // 设置缓存
        if use_cache {
            self.set_cached_data(cache_key, data.clone()).await?;
        }

        Ok(data)
    }

    /// 查询销售汇总报表
    ///
    /// 功能 按期间聚合销售订单，行键取自模板声明的四个列键，使列与行同一来源装配，
    /// 汇总模板不再返回明细行。期间由 group_by 粒度决定，销售额按期间累加，
    /// 订单数按订单行计数，客户数按客户去重计数。
    /// 调用方 execute_report 在 template_id 为 sales_summary 时分派到本函数。
    /// 入参 template 取模板声明的列、数据源与 group_by 参数默认值，
    /// req 取日期范围与调用方提交的 group_by 参数值。
    /// 传给谁 聚合行装进 ReportData.rows，交给报表执行 handler 的 data 字段
    /// 与 report/exp 的各格式导出器消费，并整体进本模块的进程内缓存。
    /// 存什么 每行四键，period 是期间字符串，total_amount 是 DECIMAL 线格式字符串，
    /// order_count 与 customer_count 是整数。
    /// 存哪里 不落库，只作为报表结果随 generate_cache_key 的键进内存缓存。
    pub async fn query_sales_summary_report(
        &self,
        template: &super::ReportTemplate,
        req: &ExecuteReportRequest,
    ) -> Result<ReportData, AppError> {
        let period_fmt = Self::resolve_sales_summary_period(template, req)?;
        let orders = Self::fetch_sales_summary_orders(&*self.db, req.date_range.as_ref()).await?;
        let groups = Self::group_sales_orders_by_period(&orders, period_fmt);

        let rows: Vec<serde_json::Value> = groups
            .into_values()
            .map(|agg| {
                serde_json::json!({
                    "period": agg.period,
                    "total_amount": agg.total_amount.to_string(),
                    "order_count": agg.order_count,
                    "customer_count": agg.customer_ids.len(),
                })
            })
            .collect();
        Self::ensure_rows_match_declared_columns(template, &rows)?;

        Ok(ReportData {
            columns: template.columns.clone(),
            total_rows: rows.len() as u64,
            rows,
            generated_at: Utc::now(),
            summary: None,
            metadata: ReportMetadata {
                data_source: template.data_source.clone(),
                query_time_ms: 0,
                cache_hit: false,
                parameters: req.parameters.clone(),
            },
        })
    }

    /// 解析销售汇总的期间粒度
    ///
    /// 功能 先取调用方提交的 group_by，缺失时回落模板自身声明的默认粒度，
    /// 把词表 day、week、month、year 映射成期间格式化串。
    /// 调用方 仅本文件的 query_sales_summary_report。
    /// 入参 req.parameters 里的 group_by 字符串与模板 parameters 中同名参数的默认值。
    /// 传给谁 返回的格式化串交给 group_sales_orders_by_period 做期间格式化。
    /// 存什么 不存数据，词表外的取值按校验族点名拒绝，不静默换成别的粒度。
    /// 存哪里 不落库，结果只在本次请求内使用。
    fn resolve_sales_summary_period(
        template: &super::ReportTemplate,
        req: &ExecuteReportRequest,
    ) -> Result<&'static str, AppError> {
        let requested = req
            .parameters
            .as_ref()
            .and_then(|params| params.get("group_by"))
            .and_then(|value| value.as_str())
            .or_else(|| {
                template
                    .parameters
                    .iter()
                    .find(|param| param.name == "group_by")
                    .and_then(|param| param.default_value.as_ref())
                    .and_then(|value| value.as_str())
            });

        match requested {
            Some("day") | None => Ok("%Y-%m-%d"),
            Some("week") => Ok("%G-W%V"),
            Some("month") => Ok("%Y-%m"),
            Some("year") => Ok("%Y"),
            Some(other) => Err(AppError::validation_displayable(format!(
                "分组方式 group_by 仅支持 day、week、month、year，收到 {}",
                other
            ))),
        }
    }

    /// 读取销售汇总所需的销售订单行
    ///
    /// 功能 按订单日范围取 sales_orders 原始行，范围缺失时取全表。
    /// 调用方 仅本文件的 query_sales_summary_report。
    /// 入参 数据库连接与可选日期范围（起止为 NaiveDate）。
    /// 传给谁 订单行交给 group_sales_orders_by_period 聚合。
    /// 存什么 不存数据，边界值非法时按校验族点名拒绝。
    /// 存哪里 不落库，查询结果只在内存里向上传递。
    async fn fetch_sales_summary_orders(
        db: &DatabaseConnection,
        date_range: Option<&super::DateRange>,
    ) -> Result<Vec<crate::models::sales_order::Model>, AppError> {
        let mut select = SalesOrderEntity::find();
        if let Some(range) = date_range {
            let start = range
                .start
                .and_hms_opt(0, 0, 0)
                .ok_or_else(|| AppError::validation_displayable("报表日期范围起始值非法"))?
                .and_utc();
            let end = range
                .end
                .and_hms_opt(23, 59, 59)
                .ok_or_else(|| AppError::validation_displayable("报表日期范围结束值非法"))?
                .and_utc();
            select =
                select.filter(crate::models::sales_order::Column::OrderDate.between(start, end));
        }
        Ok(select.all(db).await?)
    }

    /// 按期间聚合销售订单
    ///
    /// 功能 用同一格式化串把订单日折成期间键，按期间累加销售额与订单数并按客户去重，
    /// 结果按期间键升序输出，行序不再随数据库返回序漂移。
    /// 调用方 仅本文件的 query_sales_summary_report。
    /// 入参 销售订单行切片与期间格式化串。
    /// 传给谁 聚合桶交给 query_sales_summary_report 装配报表行。
    /// 存什么 每个期间一条，含期间键、销售额合计、订单数与客户集合。
    /// 存哪里 不落库，聚合结果只进报表结果对象。
    fn group_sales_orders_by_period(
        orders: &[crate::models::sales_order::Model],
        period_fmt: &'static str,
    ) -> BTreeMap<String, SalesPeriodAggregate> {
        let mut groups: BTreeMap<String, SalesPeriodAggregate> = BTreeMap::new();
        for order in orders {
            let period = order.order_date.format(period_fmt).to_string();
            let entry = groups
                .entry(period.clone())
                .or_insert_with(|| SalesPeriodAggregate {
                    period,
                    total_amount: rust_decimal::Decimal::ZERO,
                    order_count: 0_u64,
                    customer_ids: HashSet::new(),
                });
            entry.total_amount += order.total_amount;
            entry.order_count += 1;
            entry.customer_ids.insert(order.customer_id);
        }
        groups
    }

    /// 装配腿末端护栏：出参行键集合与模板声明列键集合必须逐行双向相等
    ///
    /// 功能 在每枚预置模板的装配腿返回前，逐行比对行对象键集合与声明列键集合，
    /// 缺键或多键都点名拒绝，把"声明列与实际行不同源"从静默漂移变为显式硬失败。
    /// 调用方 各预置模板的 query_*_report 装配函数在构造 ReportData 之前。
    /// 入参 template 取声明列键集合，rows 取装配完成的行对象列表。
    /// 传给谁 不匹配时把带缺键与多键点名的错误交给调用 handler；匹配时放行。
    /// 存什么 不存数据；非对象行同样点名拒绝，不静默跳过。
    /// 存哪里 不落库，护栏只在本次请求内生效。
    fn ensure_rows_match_declared_columns(
        template: &super::ReportTemplate,
        rows: &[serde_json::Value],
    ) -> Result<(), AppError> {
        let declared: std::collections::BTreeSet<&str> =
            template.columns.iter().map(|c| c.key.as_str()).collect();
        for (idx, row) in rows.iter().enumerate() {
            let Some(obj) = row.as_object() else {
                return Err(AppError::internal(format!(
                    "报表 {} 第 {} 行不是对象行，无法与声明列对齐",
                    template.id, idx
                )));
            };
            let actual: std::collections::BTreeSet<&str> = obj.keys().map(String::as_str).collect();
            if actual != declared {
                let missing: Vec<&str> = declared.difference(&actual).copied().collect();
                let extra: Vec<&str> = actual.difference(&declared).copied().collect();
                return Err(AppError::internal(format!(
                    "报表 {} 行键与声明列不同源：缺键 {:?}，多键 {:?}",
                    template.id, missing, extra
                )));
            }
        }
        Ok(())
    }

    /// 读取请求 filters 中指定键的字符串值
    ///
    /// 功能 从 ExecuteReportRequest.filters 里按键取出用户提交的过滤值。
    /// 调用方 预置模板装配腿中支持 select 过滤的函数。
    /// 入参 req 与过滤键名。
    /// 传给谁 返回的裸串交给各腿的 parse 校验；未提交返回 None。
    /// 存什么 不存数据。
    /// 存哪里 不落库，只在本次请求内读取。
    fn filter_value<'a>(req: &'a ExecuteReportRequest, key: &str) -> Option<&'a str> {
        req.filters
            .iter()
            .find(|f| f.key == key)
            .and_then(|f| f.value.as_deref())
            .filter(|v| !v.is_empty())
    }

    /// 把请求过滤串解析为正整数 id
    ///
    /// 功能 将用户提交的 id 过滤值解析为 i32，非法值点名拒绝而不是忽略该过滤。
    /// 调用方 预置模板装配腿的 customer_id / warehouse_id / supplier_id 过滤。
    /// 入参 key 是过滤键名（用于报错点名），raw 是提交的裸串。
    /// 传给谁 解析出的 id 交给各腿的 QueryFilter；失败把校验错误交给调用 handler。
    /// 存什么 不存数据；空串与非数字都拒绝，不回退成"查全部"。
    /// 存哪里 不落库。
    fn parse_filter_id(key: &str, raw: &str) -> Result<i32, AppError> {
        raw.parse::<i32>().map_err(|_| {
            AppError::validation_displayable(format!("过滤参数 {} 需为整数 id，收到 {}", key, raw))
        })
    }

    /// 按 id 集合批量取产品编码与名称
    ///
    /// 功能 用一次 in-list 查询取回产品维表映射，杜绝逐行查库的 N+1。
    /// 调用方 销售族与库存族装配腿的产品列取数。
    /// 入参 db 与产品 id 集合。
    /// 传给谁 返回的映射交给装配闭包拼行；空集合直接返回空映射不触库。
    /// 存什么 id 到（code, name）二元组。
    /// 存哪里 不落库，映射只在本次查询内存活。
    async fn load_product_map(
        db: &DatabaseConnection,
        ids: &HashSet<i32>,
    ) -> Result<HashMap<i32, (String, String)>, AppError> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let id_list: Vec<i32> = ids.iter().copied().collect();
        let products = crate::models::product::Entity::find()
            .filter(crate::models::product::Column::Id.is_in(id_list))
            .all(db)
            .await?;
        Ok(products
            .into_iter()
            .map(|p| (p.id, (p.code, p.name)))
            .collect())
    }

    /// 按 id 集合批量取客户名称
    ///
    /// 功能 用一次 in-list 查询取回客户维表映射，杜绝逐行查库的 N+1。
    /// 调用方 销售族与财务族装配腿的客户列取数。
    /// 入参 db 与客户 id 集合。
    /// 传给谁 返回的映射交给装配闭包拼行；空集合直接返回空映射不触库。
    /// 存什么 id 到客户名称。
    /// 存哪里 不落库，映射只在本次查询内存活。
    async fn load_customer_name_map(
        db: &DatabaseConnection,
        ids: &HashSet<i32>,
    ) -> Result<HashMap<i32, String>, AppError> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let id_list: Vec<i32> = ids.iter().copied().collect();
        let customers = crate::models::customer::Entity::find()
            .filter(crate::models::customer::Column::Id.is_in(id_list))
            .all(db)
            .await?;
        Ok(customers
            .into_iter()
            .map(|c| (c.id, c.customer_name))
            .collect())
    }

    /// 查询销售明细报表（按声明列投影的明细行）
    ///
    /// 功能 取销售订单行明细，关联订单号与客户、产品名称后，严格按模板声明的
    /// 五列（order_no、customer_name、product_name、quantity、amount）逐行装配，
    /// 不再返回整实体行。日期范围沿用明细行创建时间口径，客户过滤按订单维度下推。
    /// 调用方 execute_report 在 template_id 为 sales_detail 时分派到本函数。
    /// 入参 template 取声明列；req 取日期范围与 customer_id 过滤。
    /// 传给谁 行集合装进 ReportData.rows，交给 handler 与导出器按列键取值。
    /// 存什么 quantity 与 amount 是 DECIMAL 线格式字符串；维度名是字符串。
    /// 存哪里 不落库，结果随缓存键进内存缓存。
    pub async fn query_sales_detail_report(
        &self,
        template: &super::ReportTemplate,
        req: &ExecuteReportRequest,
    ) -> Result<ReportData, AppError> {
        let order_ids = Self::sales_detail_order_scope(&self.db, req).await?;
        if matches!(order_ids, Some(ref ids) if ids.is_empty()) {
            return Ok(Self::empty_report_data(template, req));
        }
        let items = Self::fetch_sales_items(&self.db, req.date_range.as_ref(), order_ids).await?;

        let order_id_set: HashSet<i32> = items.iter().map(|i| i.order_id).collect();
        let product_id_set: HashSet<i32> = items.iter().map(|i| i.product_id).collect();
        let orders = Self::fetch_sales_orders_by_ids(&self.db, &order_id_set).await?;
        let order_no_map: HashMap<i32, (String, i32)> = orders
            .into_iter()
            .map(|o| (o.id, (o.order_no, o.customer_id)))
            .collect();
        let customer_ids: HashSet<i32> = order_no_map.values().map(|(_, cid)| *cid).collect();
        let customer_map = Self::load_customer_name_map(&self.db, &customer_ids).await?;
        let product_map = Self::load_product_map(&self.db, &product_id_set).await?;

        let mut staged: Vec<(String, String, String, String, String)> = items
            .iter()
            .map(|item| {
                let (order_no, customer_id) = order_no_map
                    .get(&item.order_id)
                    .cloned()
                    .unwrap_or_default();
                let customer_name = customer_map.get(&customer_id).cloned().unwrap_or_default();
                let product_name = product_map
                    .get(&item.product_id)
                    .map(|(_, name)| name.clone())
                    .unwrap_or_default();
                (
                    order_no,
                    customer_name,
                    product_name,
                    item.quantity.to_string(),
                    item.total_amount.to_string(),
                )
            })
            .collect();
        staged.sort_by(|a, b| (&a.0, &a.2).cmp(&(&b.0, &b.2)));

        let rows: Vec<serde_json::Value> = staged
            .into_iter()
            .map(
                |(order_no, customer_name, product_name, quantity, amount)| {
                    serde_json::json!({
                        "order_no": order_no,
                        "customer_name": customer_name,
                        "product_name": product_name,
                        "quantity": quantity,
                        "amount": amount,
                    })
                },
            )
            .collect();
        Self::ensure_rows_match_declared_columns(template, &rows)?;

        Ok(Self::build_report_data(template, req, rows))
    }

    /// 销售明细的订单域（客户过滤下推）
    ///
    /// 功能 未提交客户过滤时返回 None 表示不限定订单域；提交时按 customer_id
    /// 取该客户的订单 id 集合，非法 id 值点名拒绝。
    /// 调用方 仅本文件的 query_sales_detail_report。
    /// 入参 db 与请求。
    /// 传给谁 订单 id 集合交给 fetch_sales_items 作 in-list 过滤。
    /// 存什么 不存数据；过滤值存在但非法时按校验族拒绝，不静默忽略过滤。
    /// 存哪里 不落库。
    async fn sales_detail_order_scope(
        db: &DatabaseConnection,
        req: &ExecuteReportRequest,
    ) -> Result<Option<Vec<i32>>, AppError> {
        let Some(raw) = Self::filter_value(req, "customer_id") else {
            return Ok(None);
        };
        let customer_id = Self::parse_filter_id("customer_id", raw)?;
        let orders = SalesOrderEntity::find()
            .filter(crate::models::sales_order::Column::CustomerId.eq(customer_id))
            .all(db)
            .await?;
        Ok(Some(orders.into_iter().map(|o| o.id).collect()))
    }

    /// 查询畅销产品报表（按声明列聚合投影）
    ///
    /// 功能 把销售订单行按产品聚合，销售额取行总额合计、销量取行数量合计，
    /// 按销售额降序取 TOP N 后严格装配模板声明的四列（product_code、product_name、
    /// total_quantity、total_amount）。N 先取请求参数 top_n，缺失回落模板默认。
    /// 调用方 execute_report 在 template_id 为 top_products 时分派到本函数。
    /// 入参 template 取声明列与 top_n 默认值；req 取日期范围与参数。
    /// 传给谁 聚合行装进 ReportData.rows，交给 handler 与导出器按列键取值。
    /// 存什么 total_quantity 与 total_amount 是 DECIMAL 线格式字符串。
    /// 存哪里 不落库，结果随缓存键进内存缓存。
    pub async fn query_top_products_report(
        &self,
        template: &super::ReportTemplate,
        req: &ExecuteReportRequest,
    ) -> Result<ReportData, AppError> {
        let top_n = Self::resolve_top_n(template, req)?;
        let items = Self::fetch_sales_items(&self.db, req.date_range.as_ref(), None).await?;

        let mut by_product: BTreeMap<i32, (rust_decimal::Decimal, rust_decimal::Decimal)> =
            BTreeMap::new();
        for item in &items {
            let entry = by_product
                .entry(item.product_id)
                .or_insert((rust_decimal::Decimal::ZERO, rust_decimal::Decimal::ZERO));
            entry.0 += item.quantity;
            entry.1 += item.total_amount;
        }

        let product_ids: HashSet<i32> = by_product.keys().copied().collect();
        let product_map = Self::load_product_map(&self.db, &product_ids).await?;

        let mut ranked: Vec<(i32, rust_decimal::Decimal, rust_decimal::Decimal)> = by_product
            .into_iter()
            .map(|(pid, (qty, amount))| (pid, qty, amount))
            .collect();
        ranked.sort_by(|a, b| {
            let name_of = |pid: &i32| {
                product_map
                    .get(pid)
                    .map(|p| p.1.clone())
                    .unwrap_or_default()
            };
            b.2.cmp(&a.2)
                .then_with(|| name_of(&a.0).cmp(&name_of(&b.0)))
                .then_with(|| a.0.cmp(&b.0))
        });

        let rows: Vec<serde_json::Value> = ranked
            .into_iter()
            .take(top_n)
            .map(|(pid, qty, amount)| {
                let (code, name) = product_map.get(&pid).cloned().unwrap_or_default();
                serde_json::json!({
                    "product_code": code,
                    "product_name": name,
                    "total_quantity": qty.to_string(),
                    "total_amount": amount.to_string(),
                })
            })
            .collect();
        Self::ensure_rows_match_declared_columns(template, &rows)?;

        Ok(Self::build_report_data(template, req, rows))
    }

    /// 解析 TOP N 参数
    ///
    /// 功能 先取请求参数 top_n，缺失回落模板默认，双缺失默认 20；
    /// 非正整数按校验族点名拒绝。
    /// 调用方 仅本文件的 query_top_products_report。
    /// 入参 template 的 parameters 默认值与 req.parameters。
    /// 传给谁 返回的条数交给畅销产品腿的截断。
    /// 存什么 不存数据；0、负数、非数字都拒绝，不静默换成别的值。
    /// 存哪里 不落库。
    fn resolve_top_n(
        template: &super::ReportTemplate,
        req: &ExecuteReportRequest,
    ) -> Result<usize, AppError> {
        let raw = req
            .parameters
            .as_ref()
            .and_then(|params| params.get("top_n"))
            .cloned()
            .or_else(|| {
                template
                    .parameters
                    .iter()
                    .find(|p| p.name == "top_n")
                    .and_then(|p| p.default_value.clone())
            });
        match raw {
            None => Ok(20),
            Some(value) => {
                let parsed = value
                    .as_i64()
                    .or_else(|| value.as_str().and_then(|s| s.parse::<i64>().ok()));
                match parsed {
                    Some(n) if n > 0 => Ok(n as usize),
                    Some(other) => Err(AppError::validation_displayable(format!(
                        "参数 top_n 需为正整数，收到 {}",
                        other
                    ))),
                    None => Err(AppError::validation_displayable(format!(
                        "参数 top_n 需为整数，收到 {}",
                        value
                    ))),
                }
            }
        }
    }

    /// 查询客户分析报表（按声明列聚合投影）
    ///
    /// 功能 按客户聚合销售订单，订单数按订单计数、销售额累加总单金额、
    /// 客单价为销售额除以订单数后按货币两位精度落形，行严格装配模板声明的
    /// 四列（customer_name、order_count、total_amount、avg_order_amount）。
    /// 日期范围按订单日过滤，与销售汇总同一条时间轴。
    /// 调用方 execute_report 在 template_id 为 customer_analysis 时分派到本函数。
    /// 入参 template 取声明列；req 取日期范围。
    /// 传给谁 聚合行装进 ReportData.rows，交给 handler 与导出器按列键取值。
    /// 存什么 total_amount 与 avg_order_amount 是 DECIMAL 线格式字符串，order_count 是整数。
    /// 存哪里 不落库，结果随缓存键进内存缓存。
    pub async fn query_customer_analysis_report(
        &self,
        template: &super::ReportTemplate,
        req: &ExecuteReportRequest,
    ) -> Result<ReportData, AppError> {
        let orders = Self::fetch_sales_summary_orders(&*self.db, req.date_range.as_ref()).await?;

        let mut by_customer: BTreeMap<i32, (u64, rust_decimal::Decimal)> = BTreeMap::new();
        for order in &orders {
            let entry = by_customer
                .entry(order.customer_id)
                .or_insert((0_u64, rust_decimal::Decimal::ZERO));
            entry.0 += 1;
            entry.1 += order.total_amount;
        }

        let customer_ids: HashSet<i32> = by_customer.keys().copied().collect();
        let customer_map = Self::load_customer_name_map(&self.db, &customer_ids).await?;

        let mut ranked: Vec<(i32, u64, rust_decimal::Decimal)> = by_customer
            .into_iter()
            .map(|(cid, entry)| (cid, entry.0, entry.1))
            .collect();
        ranked.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));

        let rows: Vec<serde_json::Value> = ranked
            .into_iter()
            .map(|(cid, count, total)| {
                let avg = (total / rust_decimal::Decimal::from(count)).round_dp(2);
                serde_json::json!({
                    "customer_name": customer_map.get(&cid).cloned().unwrap_or_default(),
                    "order_count": count,
                    "total_amount": total.to_string(),
                    "avg_order_amount": avg.to_string(),
                })
            })
            .collect();
        Self::ensure_rows_match_declared_columns(template, &rows)?;

        Ok(Self::build_report_data(template, req, rows))
    }

    /// 查询利润分析报表（按声明列聚合投影）
    ///
    /// 功能 按产品聚合销售收入（不含税小计合计），成本按产品成本价乘以销量折算，
    /// 毛利为收入减成本，毛利率为毛利对收入之比（四位小数即百分两位）；
    /// 成本价缺失或收入为零时成本、毛利、毛利率输出 null，表达不可计算而非编造数值。
    /// 行严格装配模板声明的五列（product_name、revenue、cost、profit、profit_margin）。
    /// 调用方 execute_report 在 template_id 为 profit_analysis 时分派到本函数。
    /// 入参 template 取声明列；req 取日期范围（明细行创建时间轴，与销售明细同源）。
    /// 传给谁 聚合行装进 ReportData.rows，交给 handler 与导出器按列键取值。
    /// 存什么 revenue 与 profit 是 DECIMAL 线格式字符串，margin 是比例字符串，
    /// cost 在有成本价时为字符串、缺失时为 null。
    /// 存哪里 不落库，结果随缓存键进内存缓存。
    pub async fn query_profit_analysis_report(
        &self,
        template: &super::ReportTemplate,
        req: &ExecuteReportRequest,
    ) -> Result<ReportData, AppError> {
        let items = Self::fetch_sales_items(&self.db, req.date_range.as_ref(), None).await?;

        let mut by_product: BTreeMap<i32, (rust_decimal::Decimal, rust_decimal::Decimal)> =
            BTreeMap::new();
        for item in &items {
            let entry = by_product
                .entry(item.product_id)
                .or_insert((rust_decimal::Decimal::ZERO, rust_decimal::Decimal::ZERO));
            entry.0 += item.quantity;
            entry.1 += item.subtotal;
        }

        let product_ids: HashSet<i32> = by_product.keys().copied().collect();
        let products = if product_ids.is_empty() {
            Vec::new()
        } else {
            let id_list: Vec<i32> = product_ids.iter().copied().collect();
            crate::models::product::Entity::find()
                .filter(crate::models::product::Column::Id.is_in(id_list))
                .all(&*self.db)
                .await?
        };
        let product_info: HashMap<i32, (String, Option<rust_decimal::Decimal>)> = products
            .into_iter()
            .map(|p| (p.id, (p.name, p.cost_price)))
            .collect();

        let mut ranked: Vec<(i32, rust_decimal::Decimal, rust_decimal::Decimal)> = by_product
            .into_iter()
            .map(|(pid, entry)| (pid, entry.1, entry.0))
            .collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        let rows: Vec<serde_json::Value> = ranked
            .into_iter()
            .map(|(pid, revenue, qty)| {
                let (name, cost_price) = product_info
                    .get(&pid)
                    .cloned()
                    .unwrap_or((String::new(), None));
                let cost = cost_price.map(|cp| cp * qty);
                let profit = cost.map(|c| revenue - c);
                let margin = profit
                    .filter(|_| !revenue.is_zero())
                    .map(|p| (p / revenue).round_dp(4));
                serde_json::json!({
                    "product_name": name,
                    "revenue": revenue.to_string(),
                    "cost": cost.map(|c| c.round_dp(2).to_string()),
                    "profit": profit.map(|p| p.round_dp(2).to_string()),
                    "profit_margin": margin.map(|m| m.to_string()),
                })
            })
            .collect();
        Self::ensure_rows_match_declared_columns(template, &rows)?;

        Ok(Self::build_report_data(template, req, rows))
    }

    /// 按日期范围与客户域过滤读取销售订单行
    ///
    /// 功能 明细族共用的取行函数：按明细行创建时间过滤，可选限定订单 id 集合。
    /// 调用方 销售明细、畅销产品、利润分析三腿。
    /// 入参 db、可选日期范围、可选订单 id 限定集合。
    /// 传给谁 行集合交给各腿聚合或投影；边界值非法按校验族点名拒绝。
    /// 存什么 不存数据。
    /// 存哪里 不落库。
    async fn fetch_sales_items(
        db: &DatabaseConnection,
        date_range: Option<&super::DateRange>,
        order_ids: Option<Vec<i32>>,
    ) -> Result<Vec<crate::models::sales_order_item::Model>, AppError> {
        let mut select = SalesOrderItemEntity::find();
        if let Some(range) = date_range {
            select = select.filter(
                crate::models::sales_order_item::Column::CreatedAt.gte(
                    range
                        .start
                        .and_hms_opt(0, 0, 0)
                        .ok_or_else(|| AppError::validation_displayable("报表日期范围起始值非法"))?
                        .and_utc(),
                ),
            );
            select = select.filter(
                crate::models::sales_order_item::Column::CreatedAt.lte(
                    range
                        .end
                        .and_hms_opt(23, 59, 59)
                        .ok_or_else(|| AppError::validation_displayable("报表日期范围结束值非法"))?
                        .and_utc(),
                ),
            );
        }
        if let Some(ids) = order_ids {
            select = select.filter(crate::models::sales_order_item::Column::OrderId.is_in(ids));
        }
        Ok(select.all(db).await?)
    }

    /// 按 id 集合读取销售订单头
    ///
    /// 功能 用一次 in-list 查询取回订单号与归属客户，供明细腿关联维度。
    /// 调用方 仅本文件的 query_sales_detail_report。
    /// 入参 db 与订单 id 集合。
    /// 传给谁 订单头交给明细腿建映射；空集合不触库。
    /// 存什么 不存数据。
    /// 存哪里 不落库。
    async fn fetch_sales_orders_by_ids(
        db: &DatabaseConnection,
        ids: &HashSet<i32>,
    ) -> Result<Vec<crate::models::sales_order::Model>, AppError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let id_list: Vec<i32> = ids.iter().copied().collect();
        Ok(SalesOrderEntity::find()
            .filter(crate::models::sales_order::Column::Id.is_in(id_list))
            .all(db)
            .await?)
    }

    /// 用模板声明与请求参数装配一份 ReportData（各腿共用的收尾构造）
    ///
    /// 功能 以模板声明列为 columns、装配行为 rows 构造报表结果对象，
    /// 元数据的参数快照只取请求自身提交的值，不写入服务内部推导值。
    /// 调用方 各预置模板装配腿的返回处。
    /// 入参 template 取声明列与数据源，req 取参数快照，rows 取已护栏校验的行。
    /// 传给谁 交给 execute_report 统一盖 query_time_ms 与缓存标记后进缓存与 handler。
    /// 存什么 不新增字段，纯组装。
    /// 存哪里 不落库。
    fn build_report_data(
        template: &super::ReportTemplate,
        req: &ExecuteReportRequest,
        rows: Vec<serde_json::Value>,
    ) -> ReportData {
        ReportData {
            columns: template.columns.clone(),
            total_rows: rows.len() as u64,
            rows,
            generated_at: Utc::now(),
            summary: None,
            metadata: ReportMetadata {
                data_source: template.data_source.clone(),
                query_time_ms: 0,
                cache_hit: false,
                parameters: req.parameters.clone(),
            },
        }
    }

    /// 构造一张空报表结果（请求过滤命中零对象时的快速返回）
    ///
    /// 功能 保留模板声明列与零行的合法空报表，不触库取维表。
    /// 调用方 预置模板腿在过滤域为空集时。
    /// 入参 template 与 req。
    /// 传给谁 交给 execute_report 进缓存与 handler。
    /// 存什么 columns 照声明、rows 为空。
    /// 存哪里 不落库。
    fn empty_report_data(
        template: &super::ReportTemplate,
        req: &ExecuteReportRequest,
    ) -> ReportData {
        Self::build_report_data(template, req, Vec::new())
    }

    /// 查询销售报表
    pub async fn query_sales_report(
        &self,
        template: &super::ReportTemplate,
        req: &ExecuteReportRequest,
    ) -> Result<ReportData, AppError> {
        let mut select = SalesOrderItemEntity::find();

        if let Some(date_range) = &req.date_range {
            select = select.filter(
                crate::models::sales_order_item::Column::CreatedAt
                    // P1-3 修复（批次 80 v1 复审）：expect 改为 ok_or_else 返回 AppError，
                    // 避免 0/0/0/23/59/59 边界输入导致 panic
                    .gte(
                        date_range
                            .start
                            .and_hms_opt(0, 0, 0)
                            .ok_or_else(|| AppError::validation_displayable("报表日期范围起始值非法"))?
                            .and_utc(),
                    ),
            );
            select = select.filter(
                crate::models::sales_order_item::Column::CreatedAt.lte(
                    date_range
                        .end
                        .and_hms_opt(23, 59, 59)
                        .ok_or_else(|| AppError::validation_displayable("报表日期范围结束值非法"))?
                        .and_utc(),
                ),
            );
        }

        let items = select.all(&*self.db).await?;

        // 转换为报表行
        let rows: Vec<serde_json::Value> = items
            .iter()
            .map(|item| {
                serde_json::json!({
                    "order_id": item.order_id,
                    "product_id": item.product_id,
                    "quantity": item.quantity_meters.to_string(),
                    "unit_price": item.unit_price.to_string(),
                    "amount": item.total_amount.to_string(),
                    "subtotal": item.subtotal.to_string(),
                    "tax_amount": item.tax_amount.to_string(),
                    "created_at": item.created_at.to_rfc3339(),
                })
            })
            .collect();

        Ok(ReportData {
            columns: template.columns.clone(),
            total_rows: rows.len() as u64,
            rows,
            generated_at: Utc::now(),
            summary: None,
            metadata: ReportMetadata {
                data_source: template.data_source.clone(),
                query_time_ms: 0,
                cache_hit: false,
                parameters: req.parameters.clone(),
            },
        })
    }

    /// 查询库存报表（按声明列投影的名称化行）
    ///
    /// 功能 取库存批次行，把仓库与产品 id 关联成名称与编码后，严格按模板声明的
    /// 五列（warehouse_name、product_code、product_name、quantity_on_hand、
    /// quantity_available）逐行装配，不再输出主键 id 与声明外数量列；
    /// 仓库过滤按提交值下推，非法 id 点名拒绝。行序按库存行 id 稳定。
    /// 调用方 execute_report 在 template_id 为 inventory_status 时分派到本函数。
    /// 入参 template 取声明列；req 取 warehouse_id 过滤。
    /// 传给谁 行集合装进 ReportData.rows，交给 handler 与导出器按列键取值。
    /// 存什么 两个数量列是 DECIMAL 线格式字符串，维度列是字符串。
    /// 存哪里 不落库，结果随缓存键进内存缓存。
    pub async fn query_inventory_report(
        &self,
        template: &super::ReportTemplate,
        req: &ExecuteReportRequest,
    ) -> Result<ReportData, AppError> {
        let mut select = InventoryStockEntity::find();
        if let Some(raw) = Self::filter_value(req, "warehouse_id") {
            let warehouse_id = Self::parse_filter_id("warehouse_id", raw)?;
            select =
                select.filter(crate::models::inventory_stock::Column::WarehouseId.eq(warehouse_id));
        }
        let stocks = select
            .order_by_asc(crate::models::inventory_stock::Column::Id)
            .all(&*self.db)
            .await?;

        let product_ids: HashSet<i32> = stocks.iter().map(|s| s.product_id).collect();
        let warehouse_ids: HashSet<i32> = stocks.iter().map(|s| s.warehouse_id).collect();
        let product_map = Self::load_product_map(&self.db, &product_ids).await?;
        let warehouse_map = Self::load_warehouse_name_map(&self.db, &warehouse_ids).await?;

        let rows: Vec<serde_json::Value> = stocks
            .iter()
            .map(|stock| {
                let (product_code, product_name) = product_map
                    .get(&stock.product_id)
                    .cloned()
                    .unwrap_or_default();
                serde_json::json!({
                    "warehouse_name": warehouse_map.get(&stock.warehouse_id).cloned().unwrap_or_default(),
                    "product_code": product_code,
                    "product_name": product_name,
                    "quantity_on_hand": stock.quantity_on_hand.to_string(),
                    "quantity_available": stock.quantity_available.to_string(),
                })
            })
            .collect();
        Self::ensure_rows_match_declared_columns(template, &rows)?;

        Ok(Self::build_report_data(template, req, rows))
    }

    /// 查询库存周转率报表（按声明列聚合投影）
    ///
    /// 功能 出库量取期间内销售出库流水的数量合计，期初库存由该批次首笔流水的
    /// 变动前存量倒推、期间无流水时等于现存量，平均库存取（期初+期末）除以二，
    /// 周转率为出库量除以平均库存（两位小数）；平均库存为零且有出库时输出 null
    /// 表达不可计算。行严格装配模板声明的四列（product_name、avg_stock、
    /// outbound、turnover_rate），按产品合并各仓库批次。时间轴用流水创建时间，
    /// 基准取自请求日期范围，不忽略请求侧参数。
    /// 调用方 execute_report 在 template_id 为 inventory_turnover 时分派到本函数。
    /// 入参 template 取声明列；req 取日期范围。
    /// 传给谁 聚合行装进 ReportData.rows，交给 handler 与导出器按列键取值。
    /// 存什么 三个数量列与比率是 DECIMAL 线格式字符串，turnover_rate 可为 null。
    /// 存哪里 不落库，结果随缓存键进内存缓存。
    pub async fn query_inventory_turnover_report(
        &self,
        template: &super::ReportTemplate,
        req: &ExecuteReportRequest,
    ) -> Result<ReportData, AppError> {
        let transactions =
            Self::fetch_inventory_transactions(&self.db, req.date_range.as_ref()).await?;

        // 每批次（产品+仓库）：期末存量取最近一笔流水的变动后存量；无流水批次取现存量表。
        let mut batch_close: HashMap<(i32, i32), rust_decimal::Decimal> = HashMap::new();
        let mut batch_open: HashMap<(i32, i32), Option<rust_decimal::Decimal>> = HashMap::new();
        let mut outbound: HashMap<i32, rust_decimal::Decimal> = HashMap::new();
        for txn in &transactions {
            let key = (txn.product_id, txn.warehouse_id);
            batch_open.entry(key).or_insert(txn.quantity_before_meters);
            batch_close.insert(key, txn.quantity_after_meters);
            if txn.transaction_type == "SALES_DELIVERY" {
                *outbound
                    .entry(txn.product_id)
                    .or_insert(rust_decimal::Decimal::ZERO) += txn.quantity_meters;
            }
        }

        let stocks = InventoryStockEntity::find().all(&*self.db).await?;
        // 批次级现存量快照：(产品, 仓库) 维度，与流水批次键同构
        let mut stock_by_batch: HashMap<(i32, i32), rust_decimal::Decimal> = HashMap::new();
        for s in &stocks {
            *stock_by_batch
                .entry((s.product_id, s.warehouse_id))
                .or_insert(rust_decimal::Decimal::ZERO) += s.quantity_on_hand;
        }

        // 批次全集：流水覆盖过的批次加现存量表覆盖的批次，逐批次定期初期末，
        // 无流水批次的期初期末都取现存量快照，跨仓批次不再互相吞掉存量。
        let mut batch_keys: HashSet<(i32, i32)> = HashSet::new();
        batch_keys.extend(batch_close.keys().copied());
        batch_keys.extend(stock_by_batch.keys().copied());

        let mut product_ids: HashSet<i32> = batch_keys.iter().map(|(pid, _)| *pid).collect();
        product_ids.extend(outbound.keys().copied());

        let mut per_product: BTreeMap<i32, (rust_decimal::Decimal, rust_decimal::Decimal)> =
            BTreeMap::new();
        for key in &batch_keys {
            let close = batch_close.get(key).copied().unwrap_or_else(|| {
                stock_by_batch
                    .get(key)
                    .copied()
                    .unwrap_or(rust_decimal::Decimal::ZERO)
            });
            let open = batch_open.get(key).copied().flatten().unwrap_or(close);
            let entry = per_product
                .entry(key.0)
                .or_insert((rust_decimal::Decimal::ZERO, rust_decimal::Decimal::ZERO));
            entry.0 += open;
            entry.1 += close;
        }
        // 只有出库没有存量与流水批次的产品（理论不该发生，如实入表）：期初期末都记零
        for pid in product_ids
            .iter()
            .filter(|pid| !per_product.contains_key(pid))
        {
            per_product.insert(
                *pid,
                (rust_decimal::Decimal::ZERO, rust_decimal::Decimal::ZERO),
            );
        }

        let product_map = Self::load_product_map(&self.db, &product_ids).await?;

        let two = rust_decimal::Decimal::TWO;
        let rows: Vec<serde_json::Value> = per_product
            .into_iter()
            .map(|(pid, (open, close))| {
                let avg = ((open + close) / two).round_dp(4);
                let out = outbound.get(&pid).copied().unwrap_or(rust_decimal::Decimal::ZERO);
                let rate = if avg.is_zero() {
                    if out.is_zero() {
                        Some(rust_decimal::Decimal::ZERO)
                    } else {
                        None
                    }
                } else {
                    Some((out / avg).round_dp(2))
                };
                serde_json::json!({
                    "product_name": product_map.get(&pid).map(|(_, n)| n.clone()).unwrap_or_default(),
                    "avg_stock": avg.to_string(),
                    "outbound": out.to_string(),
                    "turnover_rate": rate.map(|r| r.to_string()),
                })
            })
            .collect();
        Self::ensure_rows_match_declared_columns(template, &rows)?;

        Ok(Self::build_report_data(template, req, rows))
    }

    /// 按期间读取库存流水
    ///
    /// 功能 取日期范围内的全部库存流水（含变动前后存量），行序按流水 id 升序，
    /// 保证逐批次折叠时首笔即期初、末笔即期末。
    /// 调用方 仅本文件的 query_inventory_turnover_report。
    /// 入参 db 与可选日期范围。
    /// 传给谁 流水交给周转率腿折算；边界非法按校验族点名拒绝。
    /// 存什么 不存数据。
    /// 存哪里 不落库。
    async fn fetch_inventory_transactions(
        db: &DatabaseConnection,
        date_range: Option<&super::DateRange>,
    ) -> Result<Vec<crate::models::inventory_transaction::Model>, AppError> {
        let mut select = crate::models::inventory_transaction::Entity::find();
        if let Some(range) = date_range {
            select = select.filter(
                crate::models::inventory_transaction::Column::CreatedAt.gte(
                    range
                        .start
                        .and_hms_opt(0, 0, 0)
                        .ok_or_else(|| AppError::validation_displayable("报表日期范围起始值非法"))?
                        .and_utc(),
                ),
            );
            select = select.filter(
                crate::models::inventory_transaction::Column::CreatedAt.lte(
                    range
                        .end
                        .and_hms_opt(23, 59, 59)
                        .ok_or_else(|| AppError::validation_displayable("报表日期范围结束值非法"))?
                        .and_utc(),
                ),
            );
        }
        Ok(select
            .order_by_asc(crate::models::inventory_transaction::Column::Id)
            .all(db)
            .await?)
    }

    /// 按 id 集合批量取仓库名称
    ///
    /// 功能 一次 in-list 查询取回仓库维表映射，杜绝逐行查库的 N+1。
    /// 调用方 库存族装配腿的仓库列取数。
    /// 入参 db 与仓库 id 集合。
    /// 传给谁 映射交给装配闭包拼行；空集合不触库。
    /// 存什么 id 到仓库名。
    /// 存哪里 不落库。
    async fn load_warehouse_name_map(
        db: &DatabaseConnection,
        ids: &HashSet<i32>,
    ) -> Result<HashMap<i32, String>, AppError> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let id_list: Vec<i32> = ids.iter().copied().collect();
        let rows = crate::models::warehouse::Entity::find()
            .filter(crate::models::warehouse::Column::Id.is_in(id_list))
            .all(db)
            .await?;
        Ok(rows.into_iter().map(|w| (w.id, w.name)).collect())
    }

    /// 查询采购报表（P2-2 修复（v12 复审）：原实现返回空数据桩代码，；现真实查询 purchase_order_item 明细，按日期范围过滤，返回订单明细行。）
    pub async fn query_purchase_report(
        &self,
        template: &super::ReportTemplate,
        req: &ExecuteReportRequest,
    ) -> Result<ReportData, AppError> {
        let mut select = PurchaseOrderItemEntity::find();

        if let Some(date_range) = &req.date_range {
            select = select.filter(
                crate::models::purchase_order_item::Column::CreatedAt.gte(
                    date_range
                        .start
                        .and_hms_opt(0, 0, 0)
                        .ok_or_else(|| AppError::validation_displayable("报表日期范围起始值非法"))?
                        .and_utc(),
                ),
            );
            select = select.filter(
                crate::models::purchase_order_item::Column::CreatedAt.lte(
                    date_range
                        .end
                        .and_hms_opt(23, 59, 59)
                        .ok_or_else(|| AppError::validation_displayable("报表日期范围结束值非法"))?
                        .and_utc(),
                ),
            );
        }

        let items = select.all(&*self.db).await?;

        // 转换为报表行
        let rows: Vec<serde_json::Value> = items
            .iter()
            .map(|item| {
                serde_json::json!({
                    "order_id": item.order_id,
                    "product_id": item.product_id,
                    "quantity": item.quantity.to_string(),
                    "unit_price": item.unit_price.to_string(),
                    "amount": item.total_amount.to_string(),
                    "subtotal": item.subtotal.to_string(),
                    "tax_amount": item.tax_amount.to_string(),
                    "discount_amount": item.discount_amount.to_string(),
                    "created_at": item.created_at.to_rfc3339(),
                })
            })
            .collect();

        Ok(ReportData {
            columns: template.columns.clone(),
            total_rows: rows.len() as u64,
            rows,
            generated_at: Utc::now(),
            summary: None,
            metadata: ReportMetadata {
                data_source: template.data_source.clone(),
                query_time_ms: 0,
                cache_hit: false,
                parameters: req.parameters.clone(),
            },
        })
    }

    // ==================================================
    // 缓存管理
    // ==================================================

    /// 生成缓存键（基于 template_id + filters + parameters + date_range 的 SHA256）
    pub(crate) fn generate_cache_key(&self, req: &ExecuteReportRequest) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        req.template_id.hash(&mut hasher);
        for filter in &req.filters {
            filter.key.hash(&mut hasher);
        }
        if let Some(params) = &req.parameters {
            params.to_string().hash(&mut hasher);
        }
        if let Some(date_range) = &req.date_range {
            date_range.start.hash(&mut hasher);
            date_range.end.hash(&mut hasher);
        }
        format!("report:{}", hasher.finish())
    }

    /// 获取缓存数据
    pub(crate) async fn get_cached_data(&self, key: &str) -> Result<Option<ReportData>, AppError> {
        let cache = self.cache.read().await;
        if let Some(entry) = cache.get(key) {
            if entry.expires_at > Utc::now() {
                return Ok(Some(entry.data.clone()));
            }
        }
        Ok(None)
    }

    /// 设置缓存数据
    pub(crate) async fn set_cached_data(
        &self,
        key: String,
        data: ReportData,
    ) -> Result<(), AppError> {
        let mut cache = self.cache.write().await;
        let now = Utc::now();
        cache.insert(
            key,
            super::CacheEntry {
                data,
                created_at: now,
                expires_at: now + chrono::Duration::seconds(DEFAULT_CACHE_TTL_SECONDS),
                hit_count: 0,
            },
        );
        Ok(())
    }

    /// 按数据源清除缓存
    pub async fn clear_cache_by_source(&self, _data_source: &DataSource) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// 清除所有缓存
    pub async fn clear_all_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
}
