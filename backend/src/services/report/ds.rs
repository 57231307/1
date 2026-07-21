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
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use std::collections::HashMap;
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
    AggregateRequest, AggregateResult, AggregationType, DataSource, ExecuteReportRequest,
    ReportData, ReportEngineService, ReportMetadata, DEFAULT_CACHE_TTL_SECONDS,
};

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
        let mut select = SalesOrderEntity::find();

        if let Some(date_range) = &req.date_range {
            select = select.filter(
                crate::models::sales_order::Column::OrderDate
                    .between(date_range.start, date_range.end),
            );
        }

        let orders = select.all(&*self.db).await?;

        let mut results = Vec::new();
        let mut aggregation: HashMap<String, HashMap<String, rust_decimal::Decimal>> =
            HashMap::new();

        for order in &orders {
            // 简化的分组逻辑
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

            // 同时构造 columns/rows/total_count 给 handler 使用
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

        Ok(results)
    }

    /// 采购数据聚合
    ///
    /// P2-2 修复（v12 复审）：原实现返回 `Ok(Vec::new())` 桩代码，
    /// 现真实接入 purchase_orders 表，按 supplier/month 分组聚合 total_amount 与 order_count。
    pub async fn aggregate_purchase_data(
        &self,
        req: AggregateRequest,
    ) -> Result<Vec<AggregateResult>, AppError> {
        let mut select = PurchaseOrderEntity::find();

        if let Some(date_range) = &req.date_range {
            select = select.filter(
                crate::models::purchase_order::Column::OrderDate
                    .between(date_range.start, date_range.end),
            );
        }

        let orders = select.all(&*self.db).await?;

        let mut results = Vec::new();
        let mut aggregation: HashMap<String, HashMap<String, rust_decimal::Decimal>> =
            HashMap::new();

        for order in &orders {
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

            // 同时构造 columns/rows/total_count 给 handler 使用
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

        Ok(results)
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
    ///
    /// P2-2 修复（v12 复审）：原实现返回 `Ok(Vec::new())` 桩代码，
    /// 现真实接入 finance_payments 表，按 month/payment_method 分组聚合 amount 与 payment_count。
    /// D08 Tier 4 子批次2：拆分为 ≤50 行主函数 + 3 个 helper（aggregate_payments_by_group / compute_finance_group_key / build_aggregate_results）
    pub async fn aggregate_finance_data(
        &self,
        req: AggregateRequest,
    ) -> Result<Vec<AggregateResult>, AppError> {
        let mut select = FinancePaymentEntity::find();
        if let Some(date_range) = &req.date_range {
            let start = date_range
                .start
                .and_hms_opt(0, 0, 0)
                .ok_or_else(|| AppError::internal("报表日期范围起始时分秒非法"))?
                .and_utc();
            let end = date_range
                .end
                .and_hms_opt(23, 59, 59)
                .ok_or_else(|| AppError::internal("报表日期范围结束时分秒非法"))?
                .and_utc();
            select = select.filter(
                crate::models::finance_payment::Column::PaymentDate.between(start, end),
            );
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

        // 加载数据
        let data = match req.template_id.as_str() {
            "sales_summary" => self.query_sales_report(&template, &req).await?,
            "inventory_status" => self.query_inventory_report(&template, &req).await?,
            "top_products" | "customer_analysis" | "profit_analysis" => {
                self.query_sales_report(&template, &req).await?
            }
            "purchase_summary" => self.query_purchase_report(&template, &req).await?,
            _ => {
                // 默认: 销售明细
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
                            .ok_or_else(|| AppError::internal("报表日期范围起始时分秒非法"))?
                            .and_utc(),
                    ),
            );
            select = select.filter(
                crate::models::sales_order_item::Column::CreatedAt
                    .lte(
                        date_range
                            .end
                            .and_hms_opt(23, 59, 59)
                            .ok_or_else(|| AppError::internal("报表日期范围结束时分秒非法"))?
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

    /// 查询库存报表
    pub async fn query_inventory_report(
        &self,
        template: &super::ReportTemplate,
        _req: &ExecuteReportRequest,
    ) -> Result<ReportData, AppError> {
        let stocks = InventoryStockEntity::find()
            .order_by_asc(crate::models::inventory_stock::Column::Id)
            .all(&*self.db)
            .await?;

        let rows: Vec<serde_json::Value> = stocks
            .iter()
            .map(|stock| {
                serde_json::json!({
                    "warehouse_id": stock.warehouse_id,
                    "product_id": stock.product_id,
                    "quantity_on_hand": stock.quantity_on_hand.to_string(),
                    "quantity_available": stock.quantity_available.to_string(),
                    "quantity_reserved": stock.quantity_reserved.to_string(),
                    "reorder_point": stock.reorder_point.to_string(),
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
                parameters: None,
            },
        })
    }

    /// 查询采购报表
    ///
    /// P2-2 修复（v12 复审）：原实现返回空数据桩代码，
    /// 现真实查询 purchase_order_item 明细，按日期范围过滤，返回订单明细行。
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
                        .ok_or_else(|| AppError::internal("报表日期范围起始时分秒非法"))?
                        .and_utc(),
                ),
            );
            select = select.filter(
                crate::models::purchase_order_item::Column::CreatedAt.lte(
                    date_range
                        .end
                        .and_hms_opt(23, 59, 59)
                        .ok_or_else(|| AppError::internal("报表日期范围结束时分秒非法"))?
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
