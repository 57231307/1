//! 报表引擎 Service
//!
//! 提供报表模板管理、数据导出、动态模板创建、PDF/Excel 导出、报表订阅与定时发送功能

#![allow(dead_code)]

use chrono::{NaiveDateTime, Utc};
use sea_orm::DatabaseConnection;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::models::finance_payment::Entity as FinancePaymentEntity;
use crate::models::inventory_stock::Entity as InventoryStockEntity;
use crate::models::purchase_order::Entity as PurchaseOrderEntity;
use crate::models::sales_order::Entity as SalesOrderEntity;
use crate::utils::error::AppError;

/// 报表类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReportType {
    Sales,
    Purchase,
    Inventory,
    Financial,
    Custom,
}

/// 数据源枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash)]
pub enum DataSource {
    Sales,
    Purchase,
    Inventory,
    Finance,
}

/// 聚合类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AggregationType {
    Sum,
    Count,
    Average,
    Min,
    Max,
    GroupBy,
}

/// 数据聚合请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateRequest {
    pub data_source: DataSource,
    pub filters: Vec<ReportFilter>,
    pub group_by: Option<Vec<String>>,
    pub aggregation_type: AggregationType,
    pub aggregation_field: Option<String>,
}

/// 数据聚合结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub total_count: u64,
}

/// 缓存条目
#[derive(Debug, Clone)]
struct CacheEntry {
    data: ReportData,
    created_at: NaiveDateTime,
    expires_at: NaiveDateTime,
}

/// 报表模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportTemplate {
    pub id: String,
    pub name: String,
    pub report_type: ReportType,
    pub columns: Vec<ReportColumn>,
    pub filters: Vec<ReportFilter>,
    pub sort_by: Option<String>,
    pub sort_order: String,
}

/// 报表列定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportColumn {
    pub field: String,
    pub title: String,
    pub data_type: String,
    pub width: Option<i32>,
    pub format: Option<String>,
}

/// 报表筛选条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportFilter {
    pub field: String,
    pub operator: String,
    pub value: String,
}

/// 报表数据
#[derive(Debug, Clone)]
pub struct ReportData {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub total_count: u64,
}

/// 导出格式
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone)]
pub enum ExportFormat {
    CSV,
    Excel,
    PDF,
    JSON,
}

/// 动态报表模板创建请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTemplateRequest {
    pub name: String,
    pub report_type: String,
    pub columns: Vec<CreateColumnRequest>,
    pub filters: Vec<CreateFilterRequest>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

/// 动态列定义请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateColumnRequest {
    pub field: String,
    pub title: String,
    pub data_type: String,
    pub width: Option<i32>,
    pub format: Option<String>,
}

/// 动态筛选条件请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFilterRequest {
    pub field: String,
    pub operator: String,
    pub value: String,
}

/// 报表订阅
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSubscription {
    pub id: String,
    pub user_id: i32,
    pub template_id: String,
    pub name: String,
    pub frequency: String,
    pub recipients: Vec<String>,
    pub format: String,
    pub enabled: bool,
    pub created_at: NaiveDateTime,
    pub next_run_at: Option<NaiveDateTime>,
}

/// 创建订阅请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSubscriptionRequest {
    pub template_id: String,
    pub name: String,
    pub frequency: String,
    pub recipients: Vec<String>,
    pub format: Option<String>,
}

/// PDF 导出结果
#[derive(Debug)]
pub struct PdfExportResult {
    pub data: Vec<u8>,
    pub filename: String,
}

/// Excel 导出结果
#[derive(Debug)]
pub struct ExcelExportResult {
    pub data: Vec<u8>,
    pub filename: String,
}

/// 报表引擎 Service
pub struct ReportEngineService {
    db: Arc<DatabaseConnection>,
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
}

impl ReportEngineService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            db,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 生成缓存键
    fn generate_cache_key(prefix: &str, params: &[&str]) -> String {
        let mut hasher = DefaultHasher::new();
        for param in params {
            param.hash(&mut hasher);
        }
        format!("{}_{:x}", prefix, hasher.finish())
    }

    /// 获取缓存数据
    async fn get_cached_data(&self, cache_key: &str) -> Option<ReportData> {
        let cache = self.cache.read().await;
        if let Some(entry) = cache.get(cache_key) {
            if Utc::now().naive_utc() < entry.expires_at {
                return Some(entry.data.clone());
            }
        }
        None
    }

    /// 设置缓存数据
    async fn set_cached_data(&self, cache_key: &str, data: ReportData, ttl_minutes: i64) {
        let mut cache = self.cache.write().await;
        let now = Utc::now().naive_utc();
        let entry = CacheEntry {
            data,
            created_at: now,
            expires_at: now + chrono::Duration::minutes(ttl_minutes),
        };
        cache.insert(cache_key.to_string(), entry);
    }

    /// 清除过期缓存
    async fn cleanup_expired_cache(&self) {
        let mut cache = self.cache.write().await;
        let now = Utc::now().naive_utc();
        cache.retain(|_, entry| now < entry.expires_at);
    }

    /// 清除指定数据源的缓存
    pub async fn clear_cache_by_source(&self, data_source: &DataSource) {
        let mut cache = self.cache.write().await;
        let prefix = match data_source {
            DataSource::Sales => "sales_",
            DataSource::Purchase => "purchase_",
            DataSource::Inventory => "inventory_",
            DataSource::Finance => "finance_",
        };
        cache.retain(|key, _| !key.starts_with(prefix));
    }

    /// 清除所有缓存
    pub async fn clear_all_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// 数据聚合方法
    pub async fn aggregate_data(
        &self,
        request: AggregateRequest,
        page: u64,
        page_size: u64,
    ) -> Result<AggregateResult, AppError> {
        // 生成缓存键
        let cache_key = Self::generate_cache_key(
            &format!("{:?}_", request.data_source).to_lowercase(),
            &[
                &format!("{:?}", request.filters),
                &format!("{:?}", request.group_by),
                &format!("{:?}", request.aggregation_type),
                request.aggregation_field.as_deref().unwrap_or(""),
                &page.to_string(),
                &page_size.to_string(),
            ],
        );

        // 尝试从缓存获取数据
        if let Some(cached) = self.get_cached_data(&cache_key).await {
            return Ok(AggregateResult {
                columns: cached.columns,
                rows: cached.rows,
                total_count: cached.total_count,
            });
        }

        // 清理过期缓存
        self.cleanup_expired_cache().await;

        // 根据数据源执行查询
        let result = match request.data_source {
            DataSource::Sales => self.aggregate_sales_data(&request, page, page_size).await?,
            DataSource::Purchase => {
                self.aggregate_purchase_data(&request, page, page_size)
                    .await?
            }
            DataSource::Inventory => {
                self.aggregate_inventory_data(&request, page, page_size)
                    .await?
            }
            DataSource::Finance => {
                self.aggregate_finance_data(&request, page, page_size)
                    .await?
            }
        };

        // 缓存结果（5分钟）
        let report_data = ReportData {
            columns: result.columns.clone(),
            rows: result.rows.clone(),
            total_count: result.total_count,
        };
        self.set_cached_data(&cache_key, report_data, 5).await;

        Ok(result)
    }

    /// 销售数据聚合
    async fn aggregate_sales_data(
        &self,
        request: &AggregateRequest,
        page: u64,
        page_size: u64,
    ) -> Result<AggregateResult, AppError> {
        let mut query = SalesOrderEntity::find();

        // 应用筛选条件
        for filter in &request.filters {
            match filter.field.as_str() {
                "status" => {
                    query = query.filter(
                        crate::models::sales_order::Column::Status.eq(filter.value.clone()),
                    );
                }
                "order_date" if filter.operator == ">=" => {
                    // 处理日期筛选
                    if let Ok(date) = chrono::NaiveDate::parse_from_str(&filter.value, "%Y-%m-%d") {
                        let datetime = date.and_hms_opt(0, 0, 0).expect("valid time");
                        query = query
                            .filter(crate::models::sales_order::Column::OrderDate.gte(datetime));
                    }
                }
                _ => {}
            }
        }

        // 执行查询获取数据
        let orders = query
            .order_by_desc(crate::models::sales_order::Column::CreatedAt)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;

        let total = SalesOrderEntity::find().count(&*self.db).await?;

        // 根据聚合类型处理数据
        let (columns, rows) =
            match request.aggregation_type {
                AggregationType::GroupBy => {
                    // 分组聚合
                    let group_fields = request.group_by.as_deref().unwrap_or(&[]);
                    let columns;
                    let mut rows = Vec::new();

                    if group_fields.contains(&"status".to_string()) {
                        // 按状态分组
                        let mut status_map: HashMap<String, (i32, rust_decimal::Decimal)> =
                            HashMap::new();
                        for order in &orders {
                            let entry = status_map
                                .entry(order.status.clone())
                                .or_insert((0, rust_decimal::Decimal::ZERO));
                            entry.0 += 1;
                            entry.1 += order.total_amount;
                        }

                        columns = vec![
                            "状态".to_string(),
                            "订单数".to_string(),
                            "总金额".to_string(),
                        ];
                        for (status, (count, amount)) in status_map {
                            rows.push(vec![status, count.to_string(), amount.to_string()]);
                        }
                    } else if group_fields.contains(&"customer_id".to_string()) {
                        // 按客户分组
                        let mut customer_map: HashMap<i32, (i32, rust_decimal::Decimal)> =
                            HashMap::new();
                        for order in &orders {
                            let entry = customer_map
                                .entry(order.customer_id)
                                .or_insert((0, rust_decimal::Decimal::ZERO));
                            entry.0 += 1;
                            entry.1 += order.total_amount;
                        }

                        columns = vec![
                            "客户ID".to_string(),
                            "订单数".to_string(),
                            "总金额".to_string(),
                        ];
                        for (customer_id, (count, amount)) in customer_map {
                            rows.push(vec![
                                customer_id.to_string(),
                                count.to_string(),
                                amount.to_string(),
                            ]);
                        }
                    } else {
                        // 默认返回原始数据
                        columns = vec![
                            "订单编号".to_string(),
                            "客户ID".to_string(),
                            "订单金额".to_string(),
                            "状态".to_string(),
                            "创建时间".to_string(),
                        ];
                        for order in &orders {
                            rows.push(vec![
                                order.order_no.clone(),
                                order.customer_id.to_string(),
                                order.total_amount.to_string(),
                                order.status.clone(),
                                order.created_at.format("%Y-%m-%d %H:%M").to_string(),
                            ]);
                        }
                    }

                    (columns, rows)
                }
                AggregationType::Sum => {
                    // 求和聚合
                    let field = request
                        .aggregation_field
                        .as_deref()
                        .unwrap_or("total_amount");
                    let sum = orders
                        .iter()
                        .fold(rust_decimal::Decimal::ZERO, |acc, order| match field {
                            "total_amount" => acc + order.total_amount,
                            "paid_amount" => acc + order.paid_amount,
                            "balance_amount" => acc + order.balance_amount,
                            _ => acc,
                        });

                    let columns = vec![format!("{} (求和)", field)];
                    let rows = vec![vec![sum.to_string()]];
                    (columns, rows)
                }
                AggregationType::Count => {
                    // 计数聚合
                    let columns = vec!["订单数量".to_string()];
                    let rows = vec![vec![orders.len().to_string()]];
                    (columns, rows)
                }
                AggregationType::Average => {
                    // 平均值聚合
                    let field = request
                        .aggregation_field
                        .as_deref()
                        .unwrap_or("total_amount");
                    let sum = orders
                        .iter()
                        .fold(rust_decimal::Decimal::ZERO, |acc, order| match field {
                            "total_amount" => acc + order.total_amount,
                            "paid_amount" => acc + order.paid_amount,
                            "balance_amount" => acc + order.balance_amount,
                            _ => acc,
                        });
                    let count = orders.len();
                    let avg = if count > 0 {
                        sum / rust_decimal::Decimal::from(count)
                    } else {
                        rust_decimal::Decimal::ZERO
                    };

                    let columns = vec![format!("{} (平均值)", field)];
                    let rows = vec![vec![avg.to_string()]];
                    (columns, rows)
                }
                AggregationType::Min | AggregationType::Max => {
                    // 最小值/最大值聚合
                    let field = request
                        .aggregation_field
                        .as_deref()
                        .unwrap_or("total_amount");
                    let values: Vec<rust_decimal::Decimal> = orders
                        .iter()
                        .map(|order| match field {
                            "total_amount" => order.total_amount,
                            "paid_amount" => order.paid_amount,
                            "balance_amount" => order.balance_amount,
                            _ => rust_decimal::Decimal::ZERO,
                        })
                        .collect();

                    let result = if request.aggregation_type == AggregationType::Min {
                        values
                            .iter()
                            .min()
                            .cloned()
                            .unwrap_or(rust_decimal::Decimal::ZERO)
                    } else {
                        values
                            .iter()
                            .max()
                            .cloned()
                            .unwrap_or(rust_decimal::Decimal::ZERO)
                    };

                    let label = if request.aggregation_type == AggregationType::Min {
                        "最小值"
                    } else {
                        "最大值"
                    };
                    let columns = vec![format!("{} ({})", field, label)];
                    let rows = vec![vec![result.to_string()]];
                    (columns, rows)
                }
            };

        Ok(AggregateResult {
            columns,
            rows,
            total_count: total,
        })
    }

    /// 采购数据聚合
    async fn aggregate_purchase_data(
        &self,
        request: &AggregateRequest,
        page: u64,
        page_size: u64,
    ) -> Result<AggregateResult, AppError> {
        let mut query = PurchaseOrderEntity::find();

        // 应用筛选条件
        for filter in &request.filters {
            match filter.field.as_str() {
                "order_status" => {
                    query = query.filter(
                        crate::models::purchase_order::Column::OrderStatus.eq(filter.value.clone()),
                    );
                }
                "supplier_id" => {
                    if let Ok(supplier_id) = filter.value.parse::<i32>() {
                        query = query.filter(
                            crate::models::purchase_order::Column::SupplierId.eq(supplier_id),
                        );
                    }
                }
                _ => {}
            }
        }

        let orders = query
            .order_by_desc(crate::models::purchase_order::Column::CreatedAt)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;

        let total = PurchaseOrderEntity::find().count(&*self.db).await?;

        let (columns, rows) =
            match request.aggregation_type {
                AggregationType::GroupBy => {
                    let group_fields = request.group_by.as_deref().unwrap_or(&[]);
                    let columns;
                    let mut rows = Vec::new();

                    if group_fields.contains(&"order_status".to_string()) {
                        let mut status_map: HashMap<String, (i32, rust_decimal::Decimal)> =
                            HashMap::new();
                        for order in &orders {
                            let entry = status_map
                                .entry(order.order_status.clone())
                                .or_insert((0, rust_decimal::Decimal::ZERO));
                            entry.0 += 1;
                            entry.1 += order.total_amount;
                        }

                        columns = vec![
                            "状态".to_string(),
                            "订单数".to_string(),
                            "总金额".to_string(),
                        ];
                        for (status, (count, amount)) in status_map {
                            rows.push(vec![status, count.to_string(), amount.to_string()]);
                        }
                    } else if group_fields.contains(&"supplier_id".to_string()) {
                        let mut supplier_map: HashMap<i32, (i32, rust_decimal::Decimal)> =
                            HashMap::new();
                        for order in &orders {
                            let entry = supplier_map
                                .entry(order.supplier_id)
                                .or_insert((0, rust_decimal::Decimal::ZERO));
                            entry.0 += 1;
                            entry.1 += order.total_amount;
                        }

                        columns = vec![
                            "供应商ID".to_string(),
                            "订单数".to_string(),
                            "总金额".to_string(),
                        ];
                        for (supplier_id, (count, amount)) in supplier_map {
                            rows.push(vec![
                                supplier_id.to_string(),
                                count.to_string(),
                                amount.to_string(),
                            ]);
                        }
                    } else {
                        columns = vec![
                            "采购单号".to_string(),
                            "供应商ID".to_string(),
                            "采购金额".to_string(),
                            "状态".to_string(),
                            "创建时间".to_string(),
                        ];
                        for order in &orders {
                            rows.push(vec![
                                order.order_no.clone(),
                                order.supplier_id.to_string(),
                                order.total_amount.to_string(),
                                order.order_status.clone(),
                                order.created_at.format("%Y-%m-%d %H:%M").to_string(),
                            ]);
                        }
                    }

                    (columns, rows)
                }
                AggregationType::Sum => {
                    let field = request
                        .aggregation_field
                        .as_deref()
                        .unwrap_or("total_amount");
                    let sum = orders
                        .iter()
                        .fold(rust_decimal::Decimal::ZERO, |acc, order| match field {
                            "total_amount" => acc + order.total_amount,
                            "total_amount_foreign" => acc + order.total_amount_foreign,
                            _ => acc,
                        });

                    let columns = vec![format!("{} (求和)", field)];
                    let rows = vec![vec![sum.to_string()]];
                    (columns, rows)
                }
                AggregationType::Count => {
                    let columns = vec!["订单数量".to_string()];
                    let rows = vec![vec![orders.len().to_string()]];
                    (columns, rows)
                }
                AggregationType::Average => {
                    let field = request
                        .aggregation_field
                        .as_deref()
                        .unwrap_or("total_amount");
                    let sum = orders
                        .iter()
                        .fold(rust_decimal::Decimal::ZERO, |acc, order| match field {
                            "total_amount" => acc + order.total_amount,
                            "total_amount_foreign" => acc + order.total_amount_foreign,
                            _ => acc,
                        });
                    let count = orders.len();
                    let avg = if count > 0 {
                        sum / rust_decimal::Decimal::from(count)
                    } else {
                        rust_decimal::Decimal::ZERO
                    };

                    let columns = vec![format!("{} (平均值)", field)];
                    let rows = vec![vec![avg.to_string()]];
                    (columns, rows)
                }
                AggregationType::Min | AggregationType::Max => {
                    let field = request
                        .aggregation_field
                        .as_deref()
                        .unwrap_or("total_amount");
                    let values: Vec<rust_decimal::Decimal> = orders
                        .iter()
                        .map(|order| match field {
                            "total_amount" => order.total_amount,
                            "total_amount_foreign" => order.total_amount_foreign,
                            _ => rust_decimal::Decimal::ZERO,
                        })
                        .collect();

                    let result = if request.aggregation_type == AggregationType::Min {
                        values
                            .iter()
                            .min()
                            .cloned()
                            .unwrap_or(rust_decimal::Decimal::ZERO)
                    } else {
                        values
                            .iter()
                            .max()
                            .cloned()
                            .unwrap_or(rust_decimal::Decimal::ZERO)
                    };

                    let label = if request.aggregation_type == AggregationType::Min {
                        "最小值"
                    } else {
                        "最大值"
                    };
                    let columns = vec![format!("{} ({})", field, label)];
                    let rows = vec![vec![result.to_string()]];
                    (columns, rows)
                }
            };

        Ok(AggregateResult {
            columns,
            rows,
            total_count: total,
        })
    }

    /// 库存数据聚合
    async fn aggregate_inventory_data(
        &self,
        request: &AggregateRequest,
        page: u64,
        page_size: u64,
    ) -> Result<AggregateResult, AppError> {
        let mut query = InventoryStockEntity::find();

        // 应用筛选条件
        for filter in &request.filters {
            match filter.field.as_str() {
                "warehouse_id" => {
                    if let Ok(warehouse_id) = filter.value.parse::<i32>() {
                        query = query.filter(
                            crate::models::inventory_stock::Column::WarehouseId.eq(warehouse_id),
                        );
                    }
                }
                "product_id" => {
                    if let Ok(product_id) = filter.value.parse::<i32>() {
                        query = query.filter(
                            crate::models::inventory_stock::Column::ProductId.eq(product_id),
                        );
                    }
                }
                "grade" => {
                    query = query.filter(
                        crate::models::inventory_stock::Column::Grade.eq(filter.value.clone()),
                    );
                }
                _ => {}
            }
        }

        let stocks = query
            .order_by_desc(crate::models::inventory_stock::Column::QuantityAvailable)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;

        let total = InventoryStockEntity::find().count(&*self.db).await?;

        let (columns, rows) =
            match request.aggregation_type {
                AggregationType::GroupBy => {
                    let group_fields = request.group_by.as_deref().unwrap_or(&[]);
                    let columns;
                    let mut rows = Vec::new();

                    if group_fields.contains(&"warehouse_id".to_string()) {
                        let mut warehouse_map: HashMap<
                            i32,
                            (i32, rust_decimal::Decimal, rust_decimal::Decimal),
                        > = HashMap::new();
                        for stock in &stocks {
                            let entry = warehouse_map.entry(stock.warehouse_id).or_insert((
                                0,
                                rust_decimal::Decimal::ZERO,
                                rust_decimal::Decimal::ZERO,
                            ));
                            entry.0 += 1;
                            entry.1 += stock.quantity_available;
                            entry.2 += stock.quantity_reserved;
                        }

                        columns = vec![
                            "仓库ID".to_string(),
                            "产品数".to_string(),
                            "可用库存".to_string(),
                            "预留库存".to_string(),
                        ];
                        for (warehouse_id, (count, available, reserved)) in warehouse_map {
                            rows.push(vec![
                                warehouse_id.to_string(),
                                count.to_string(),
                                available.to_string(),
                                reserved.to_string(),
                            ]);
                        }
                    } else if group_fields.contains(&"grade".to_string()) {
                        let mut grade_map: HashMap<String, (i32, rust_decimal::Decimal)> =
                            HashMap::new();
                        for stock in &stocks {
                            let entry = grade_map
                                .entry(stock.grade.clone())
                                .or_insert((0, rust_decimal::Decimal::ZERO));
                            entry.0 += 1;
                            entry.1 += stock.quantity_available;
                        }

                        columns = vec![
                            "等级".to_string(),
                            "产品数".to_string(),
                            "可用库存".to_string(),
                        ];
                        for (grade, (count, available)) in grade_map {
                            rows.push(vec![grade, count.to_string(), available.to_string()]);
                        }
                    } else {
                        columns = vec![
                            "产品ID".to_string(),
                            "可用库存".to_string(),
                            "预留库存".to_string(),
                            "在途库存".to_string(),
                            "仓库ID".to_string(),
                        ];
                        for stock in &stocks {
                            rows.push(vec![
                                stock.product_id.to_string(),
                                stock.quantity_available.to_string(),
                                stock.quantity_reserved.to_string(),
                                stock.quantity_incoming.to_string(),
                                stock.warehouse_id.to_string(),
                            ]);
                        }
                    }

                    (columns, rows)
                }
                AggregationType::Sum => {
                    let field = request
                        .aggregation_field
                        .as_deref()
                        .unwrap_or("quantity_available");
                    let sum = stocks
                        .iter()
                        .fold(rust_decimal::Decimal::ZERO, |acc, stock| match field {
                            "quantity_available" => acc + stock.quantity_available,
                            "quantity_reserved" => acc + stock.quantity_reserved,
                            "quantity_incoming" => acc + stock.quantity_incoming,
                            "quantity_on_hand" => acc + stock.quantity_on_hand,
                            _ => acc,
                        });

                    let columns = vec![format!("{} (求和)", field)];
                    let rows = vec![vec![sum.to_string()]];
                    (columns, rows)
                }
                AggregationType::Count => {
                    let columns = vec!["产品数量".to_string()];
                    let rows = vec![vec![stocks.len().to_string()]];
                    (columns, rows)
                }
                AggregationType::Average => {
                    let field = request
                        .aggregation_field
                        .as_deref()
                        .unwrap_or("quantity_available");
                    let sum = stocks
                        .iter()
                        .fold(rust_decimal::Decimal::ZERO, |acc, stock| match field {
                            "quantity_available" => acc + stock.quantity_available,
                            "quantity_reserved" => acc + stock.quantity_reserved,
                            "quantity_incoming" => acc + stock.quantity_incoming,
                            "quantity_on_hand" => acc + stock.quantity_on_hand,
                            _ => acc,
                        });
                    let count = stocks.len();
                    let avg = if count > 0 {
                        sum / rust_decimal::Decimal::from(count)
                    } else {
                        rust_decimal::Decimal::ZERO
                    };

                    let columns = vec![format!("{} (平均值)", field)];
                    let rows = vec![vec![avg.to_string()]];
                    (columns, rows)
                }
                AggregationType::Min | AggregationType::Max => {
                    let field = request
                        .aggregation_field
                        .as_deref()
                        .unwrap_or("quantity_available");
                    let values: Vec<rust_decimal::Decimal> = stocks
                        .iter()
                        .map(|stock| match field {
                            "quantity_available" => stock.quantity_available,
                            "quantity_reserved" => stock.quantity_reserved,
                            "quantity_incoming" => stock.quantity_incoming,
                            "quantity_on_hand" => stock.quantity_on_hand,
                            _ => rust_decimal::Decimal::ZERO,
                        })
                        .collect();

                    let result = if request.aggregation_type == AggregationType::Min {
                        values
                            .iter()
                            .min()
                            .cloned()
                            .unwrap_or(rust_decimal::Decimal::ZERO)
                    } else {
                        values
                            .iter()
                            .max()
                            .cloned()
                            .unwrap_or(rust_decimal::Decimal::ZERO)
                    };

                    let label = if request.aggregation_type == AggregationType::Min {
                        "最小值"
                    } else {
                        "最大值"
                    };
                    let columns = vec![format!("{} ({})", field, label)];
                    let rows = vec![vec![result.to_string()]];
                    (columns, rows)
                }
            };

        Ok(AggregateResult {
            columns,
            rows,
            total_count: total,
        })
    }

    /// 财务数据聚合
    async fn aggregate_finance_data(
        &self,
        request: &AggregateRequest,
        page: u64,
        page_size: u64,
    ) -> Result<AggregateResult, AppError> {
        let mut query = FinancePaymentEntity::find();

        // 应用筛选条件
        for filter in &request.filters {
            match filter.field.as_str() {
                "status" => {
                    query = query.filter(
                        crate::models::finance_payment::Column::Status.eq(filter.value.clone()),
                    );
                }
                "payment_method" => {
                    query = query.filter(
                        crate::models::finance_payment::Column::PaymentMethod
                            .eq(filter.value.clone()),
                    );
                }
                _ => {}
            }
        }

        let payments = query
            .order_by_desc(crate::models::finance_payment::Column::CreatedAt)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;

        let total = FinancePaymentEntity::find().count(&*self.db).await?;

        let (columns, rows) = match request.aggregation_type {
            AggregationType::GroupBy => {
                let group_fields = request.group_by.as_deref().unwrap_or(&[]);
                let columns;
                let mut rows = Vec::new();

                if group_fields.contains(&"status".to_string()) {
                    let mut status_map: HashMap<String, (i32, rust_decimal::Decimal)> =
                        HashMap::new();
                    for payment in &payments {
                        let entry = status_map
                            .entry(payment.status.clone())
                            .or_insert((0, rust_decimal::Decimal::ZERO));
                        entry.0 += 1;
                        entry.1 += payment.amount;
                    }

                    columns = vec![
                        "状态".to_string(),
                        "付款数".to_string(),
                        "总金额".to_string(),
                    ];
                    for (status, (count, amount)) in status_map {
                        rows.push(vec![status, count.to_string(), amount.to_string()]);
                    }
                } else if group_fields.contains(&"payment_method".to_string()) {
                    let mut method_map: HashMap<String, (i32, rust_decimal::Decimal)> =
                        HashMap::new();
                    for payment in &payments {
                        let method = payment
                            .payment_method
                            .clone()
                            .unwrap_or_else(|| "未知".to_string());
                        let entry = method_map
                            .entry(method)
                            .or_insert((0, rust_decimal::Decimal::ZERO));
                        entry.0 += 1;
                        entry.1 += payment.amount;
                    }

                    columns = vec![
                        "付款方式".to_string(),
                        "付款数".to_string(),
                        "总金额".to_string(),
                    ];
                    for (method, (count, amount)) in method_map {
                        rows.push(vec![method, count.to_string(), amount.to_string()]);
                    }
                } else {
                    columns = vec![
                        "付款单号".to_string(),
                        "金额".to_string(),
                        "付款方式".to_string(),
                        "状态".to_string(),
                        "创建时间".to_string(),
                    ];
                    for payment in &payments {
                        rows.push(vec![
                            payment.payment_no.clone(),
                            payment.amount.to_string(),
                            payment
                                .payment_method
                                .clone()
                                .unwrap_or_else(|| "".to_string()),
                            payment.status.clone(),
                            payment.created_at.format("%Y-%m-%d %H:%M").to_string(),
                        ]);
                    }
                }

                (columns, rows)
            }
            AggregationType::Sum => {
                let sum = payments
                    .iter()
                    .fold(rust_decimal::Decimal::ZERO, |acc, payment| {
                        acc + payment.amount
                    });

                let columns = vec!["金额 (求和)".to_string()];
                let rows = vec![vec![sum.to_string()]];
                (columns, rows)
            }
            AggregationType::Count => {
                let columns = vec!["付款数量".to_string()];
                let rows = vec![vec![payments.len().to_string()]];
                (columns, rows)
            }
            AggregationType::Average => {
                let sum = payments
                    .iter()
                    .fold(rust_decimal::Decimal::ZERO, |acc, payment| {
                        acc + payment.amount
                    });
                let count = payments.len();
                let avg = if count > 0 {
                    sum / rust_decimal::Decimal::from(count)
                } else {
                    rust_decimal::Decimal::ZERO
                };

                let columns = vec!["金额 (平均值)".to_string()];
                let rows = vec![vec![avg.to_string()]];
                (columns, rows)
            }
            AggregationType::Min | AggregationType::Max => {
                let values: Vec<rust_decimal::Decimal> =
                    payments.iter().map(|payment| payment.amount).collect();

                let result = if request.aggregation_type == AggregationType::Min {
                    values
                        .iter()
                        .min()
                        .cloned()
                        .unwrap_or(rust_decimal::Decimal::ZERO)
                } else {
                    values
                        .iter()
                        .max()
                        .cloned()
                        .unwrap_or(rust_decimal::Decimal::ZERO)
                };

                let label = if request.aggregation_type == AggregationType::Min {
                    "最小值"
                } else {
                    "最大值"
                };
                let columns = vec![format!("金额 ({})", label)];
                let rows = vec![vec![result.to_string()]];
                (columns, rows)
            }
        };

        Ok(AggregateResult {
            columns,
            rows,
            total_count: total,
        })
    }

    /// 获取预定义报表模板
    pub fn get_predefined_templates() -> Vec<ReportTemplate> {
        vec![
            ReportTemplate {
                id: "sales_daily".to_string(),
                name: "销售日报表".to_string(),
                report_type: ReportType::Sales,
                columns: vec![
                    ReportColumn {
                        field: "order_no".to_string(),
                        title: "订单编号".to_string(),
                        data_type: "string".to_string(),
                        width: Some(150),
                        format: None,
                    },
                    ReportColumn {
                        field: "customer_name".to_string(),
                        title: "客户名称".to_string(),
                        data_type: "string".to_string(),
                        width: Some(200),
                        format: None,
                    },
                    ReportColumn {
                        field: "order_date".to_string(),
                        title: "订单日期".to_string(),
                        data_type: "date".to_string(),
                        width: Some(120),
                        format: Some("YYYY-MM-DD".to_string()),
                    },
                    ReportColumn {
                        field: "total_amount".to_string(),
                        title: "订单金额".to_string(),
                        data_type: "decimal".to_string(),
                        width: Some(120),
                        format: Some("#.00".to_string()),
                    },
                    ReportColumn {
                        field: "status".to_string(),
                        title: "状态".to_string(),
                        data_type: "string".to_string(),
                        width: Some(100),
                        format: None,
                    },
                ],
                filters: vec![ReportFilter {
                    field: "order_date".to_string(),
                    operator: ">=".to_string(),
                    value: "today-30".to_string(),
                }],
                sort_by: Some("order_date".to_string()),
                sort_order: "desc".to_string(),
            },
            ReportTemplate {
                id: "inventory_status".to_string(),
                name: "库存状态报表".to_string(),
                report_type: ReportType::Inventory,
                columns: vec![
                    ReportColumn {
                        field: "product_code".to_string(),
                        title: "产品编码".to_string(),
                        data_type: "string".to_string(),
                        width: Some(150),
                        format: None,
                    },
                    ReportColumn {
                        field: "product_name".to_string(),
                        title: "产品名称".to_string(),
                        data_type: "string".to_string(),
                        width: Some(200),
                        format: None,
                    },
                    ReportColumn {
                        field: "quantity_available".to_string(),
                        title: "可用库存".to_string(),
                        data_type: "decimal".to_string(),
                        width: Some(120),
                        format: Some("#.00".to_string()),
                    },
                    ReportColumn {
                        field: "quantity_reserved".to_string(),
                        title: "预留库存".to_string(),
                        data_type: "decimal".to_string(),
                        width: Some(120),
                        format: Some("#.00".to_string()),
                    },
                    ReportColumn {
                        field: "warehouse".to_string(),
                        title: "仓库".to_string(),
                        data_type: "string".to_string(),
                        width: Some(150),
                        format: None,
                    },
                ],
                filters: vec![],
                sort_by: Some("quantity_available".to_string()),
                sort_order: "desc".to_string(),
            },
            ReportTemplate {
                id: "purchase_summary".to_string(),
                name: "采购汇总报表".to_string(),
                report_type: ReportType::Purchase,
                columns: vec![
                    ReportColumn {
                        field: "order_no".to_string(),
                        title: "采购单号".to_string(),
                        data_type: "string".to_string(),
                        width: Some(150),
                        format: None,
                    },
                    ReportColumn {
                        field: "supplier_name".to_string(),
                        title: "供应商".to_string(),
                        data_type: "string".to_string(),
                        width: Some(200),
                        format: None,
                    },
                    ReportColumn {
                        field: "order_date".to_string(),
                        title: "下单日期".to_string(),
                        data_type: "date".to_string(),
                        width: Some(120),
                        format: Some("YYYY-MM-DD".to_string()),
                    },
                    ReportColumn {
                        field: "total_amount".to_string(),
                        title: "采购金额".to_string(),
                        data_type: "decimal".to_string(),
                        width: Some(120),
                        format: Some("#.00".to_string()),
                    },
                    ReportColumn {
                        field: "delivery_date".to_string(),
                        title: "交期".to_string(),
                        data_type: "date".to_string(),
                        width: Some(120),
                        format: Some("YYYY-MM-DD".to_string()),
                    },
                ],
                filters: vec![ReportFilter {
                    field: "order_date".to_string(),
                    operator: ">=".to_string(),
                    value: "today-30".to_string(),
                }],
                sort_by: Some("order_date".to_string()),
                sort_order: "desc".to_string(),
            },
        ]
    }

    /// 创建自定义报表模板
    pub fn create_custom_template(req: CreateTemplateRequest) -> Result<ReportTemplate, AppError> {
        if req.name.trim().is_empty() {
            return Err(AppError::validation("模板名称不能为空"));
        }
        if req.columns.is_empty() {
            return Err(AppError::validation("至少需要定义一个列"));
        }

        let report_type = match req.report_type.as_str() {
            "sales" => ReportType::Sales,
            "purchase" => ReportType::Purchase,
            "inventory" => ReportType::Inventory,
            "financial" => ReportType::Financial,
            _ => ReportType::Custom,
        };

        let template_id = format!("custom_{}", Utc::now().timestamp_millis());

        let columns: Vec<ReportColumn> = req
            .columns
            .into_iter()
            .map(|c| ReportColumn {
                field: c.field,
                title: c.title,
                data_type: c.data_type,
                width: c.width,
                format: c.format,
            })
            .collect();

        let filters: Vec<ReportFilter> = req
            .filters
            .into_iter()
            .map(|f| ReportFilter {
                field: f.field,
                operator: f.operator,
                value: f.value,
            })
            .collect();

        Ok(ReportTemplate {
            id: template_id,
            name: req.name,
            report_type,
            columns,
            filters,
            sort_by: req.sort_by,
            sort_order: req.sort_order.unwrap_or_else(|| "asc".to_string()),
        })
    }

    /// 获取所有模板（预定义 + 自定义）
    pub fn get_all_templates(custom_templates: &[ReportTemplate]) -> Vec<ReportTemplate> {
        let mut all = Self::get_predefined_templates();
        all.extend(custom_templates.iter().cloned());
        all
    }

    /// 执行报表查询
    pub async fn execute_report(
        &self,
        template_id: &str,
        custom_filters: Vec<ReportFilter>,
        page: u64,
        page_size: u64,
    ) -> Result<ReportData, AppError> {
        // 生成缓存键
        let cache_key = Self::generate_cache_key(
            "report_",
            &[
                template_id,
                &format!("{:?}", custom_filters),
                &page.to_string(),
                &page_size.to_string(),
            ],
        );

        // 尝试从缓存获取数据
        if let Some(cached) = self.get_cached_data(&cache_key).await {
            return Ok(cached);
        }

        // 清理过期缓存
        self.cleanup_expired_cache().await;

        // 执行查询
        let result = match template_id {
            "sales_daily" => {
                self.query_sales_report(custom_filters, page, page_size)
                    .await
            }
            "inventory_status" => {
                self.query_inventory_report(custom_filters, page, page_size)
                    .await
            }
            "purchase_summary" => {
                self.query_purchase_report(custom_filters, page, page_size)
                    .await
            }
            id if id.starts_with("custom_") => Err(AppError::bad_request(
                "自定义模板需要通过 aggregate_data 接口执行".to_string(),
            )),
            _ => Err(AppError::not_found(format!(
                "报表模板 {} 不存在",
                template_id
            ))),
        }?;

        // 缓存结果（5分钟）
        self.set_cached_data(&cache_key, result.clone(), 5).await;

        Ok(result)
    }

    /// 销售报表查询
    async fn query_sales_report(
        &self,
        filters: Vec<ReportFilter>,
        page: u64,
        page_size: u64,
    ) -> Result<ReportData, AppError> {
        let mut query = SalesOrderEntity::find();

        // 应用筛选条件
        for filter in &filters {
            match filter.field.as_str() {
                "status" => {
                    query = query.filter(
                        crate::models::sales_order::Column::Status.eq(filter.value.clone()),
                    );
                }
                "order_date" => {
                    if filter.operator == ">=" {
                        if let Ok(date) =
                            chrono::NaiveDate::parse_from_str(&filter.value, "%Y-%m-%d")
                        {
                            let datetime = date.and_hms_opt(0, 0, 0).expect("valid time");
                            query = query.filter(
                                crate::models::sales_order::Column::OrderDate.gte(datetime),
                            );
                        }
                    } else if filter.operator == "<=" {
                        if let Ok(date) =
                            chrono::NaiveDate::parse_from_str(&filter.value, "%Y-%m-%d")
                        {
                            let datetime = date.and_hms_opt(23, 59, 59).expect("valid time");
                            query = query.filter(
                                crate::models::sales_order::Column::OrderDate.lte(datetime),
                            );
                        }
                    }
                }
                "customer_id" => {
                    if let Ok(customer_id) = filter.value.parse::<i32>() {
                        query = query
                            .filter(crate::models::sales_order::Column::CustomerId.eq(customer_id));
                    }
                }
                _ => {}
            }
        }

        let paginator = query
            .order_by_desc(crate::models::sales_order::Column::CreatedAt)
            .paginate(&*self.db, page_size);

        let orders = paginator.fetch_page(page - 1).await?;

        let total = SalesOrderEntity::find().count(&*self.db).await?;

        let columns = vec![
            "订单编号".to_string(),
            "客户ID".to_string(),
            "订单金额".to_string(),
            "状态".to_string(),
            "创建时间".to_string(),
        ];

        let rows: Vec<Vec<String>> = orders
            .into_iter()
            .map(|o| {
                vec![
                    o.order_no,
                    o.customer_id.to_string(),
                    o.total_amount.to_string(),
                    o.status,
                    o.created_at.format("%Y-%m-%d %H:%M").to_string(),
                ]
            })
            .collect();

        Ok(ReportData {
            columns,
            rows,
            total_count: total,
        })
    }

    /// 库存报表查询
    async fn query_inventory_report(
        &self,
        filters: Vec<ReportFilter>,
        page: u64,
        page_size: u64,
    ) -> Result<ReportData, AppError> {
        let mut query = InventoryStockEntity::find();

        // 应用筛选条件
        for filter in &filters {
            match filter.field.as_str() {
                "warehouse_id" => {
                    if let Ok(warehouse_id) = filter.value.parse::<i32>() {
                        query = query.filter(
                            crate::models::inventory_stock::Column::WarehouseId.eq(warehouse_id),
                        );
                    }
                }
                "product_id" => {
                    if let Ok(product_id) = filter.value.parse::<i32>() {
                        query = query.filter(
                            crate::models::inventory_stock::Column::ProductId.eq(product_id),
                        );
                    }
                }
                "grade" => {
                    query = query.filter(
                        crate::models::inventory_stock::Column::Grade.eq(filter.value.clone()),
                    );
                }
                _ => {}
            }
        }

        let paginator = query
            .order_by_desc(crate::models::inventory_stock::Column::QuantityAvailable)
            .paginate(&*self.db, page_size);

        let stocks = paginator.fetch_page(page - 1).await?;

        let total = InventoryStockEntity::find().count(&*self.db).await?;

        let columns = vec![
            "产品ID".to_string(),
            "可用库存".to_string(),
            "预留库存".to_string(),
            "在途库存".to_string(),
            "仓库ID".to_string(),
        ];

        let rows: Vec<Vec<String>> = stocks
            .into_iter()
            .map(|s| {
                vec![
                    s.product_id.to_string(),
                    s.quantity_available.to_string(),
                    s.quantity_reserved.to_string(),
                    s.quantity_incoming.to_string(),
                    s.warehouse_id.to_string(),
                ]
            })
            .collect();

        Ok(ReportData {
            columns,
            rows,
            total_count: total,
        })
    }

    /// 采购报表查询
    async fn query_purchase_report(
        &self,
        filters: Vec<ReportFilter>,
        page: u64,
        page_size: u64,
    ) -> Result<ReportData, AppError> {
        let mut query = PurchaseOrderEntity::find();

        // 应用筛选条件
        for filter in &filters {
            match filter.field.as_str() {
                "order_status" => {
                    query = query.filter(
                        crate::models::purchase_order::Column::OrderStatus.eq(filter.value.clone()),
                    );
                }
                "supplier_id" => {
                    if let Ok(supplier_id) = filter.value.parse::<i32>() {
                        query = query.filter(
                            crate::models::purchase_order::Column::SupplierId.eq(supplier_id),
                        );
                    }
                }
                "order_date" => {
                    if filter.operator == ">=" {
                        if let Ok(date) =
                            chrono::NaiveDate::parse_from_str(&filter.value, "%Y-%m-%d")
                        {
                            query = query
                                .filter(crate::models::purchase_order::Column::OrderDate.gte(date));
                        }
                    } else if filter.operator == "<=" {
                        if let Ok(date) =
                            chrono::NaiveDate::parse_from_str(&filter.value, "%Y-%m-%d")
                        {
                            query = query
                                .filter(crate::models::purchase_order::Column::OrderDate.lte(date));
                        }
                    }
                }
                _ => {}
            }
        }

        let paginator = query
            .order_by_desc(crate::models::purchase_order::Column::CreatedAt)
            .paginate(&*self.db, page_size);

        let orders = paginator.fetch_page(page - 1).await?;

        let total = PurchaseOrderEntity::find().count(&*self.db).await?;

        let columns = vec![
            "采购单号".to_string(),
            "供应商ID".to_string(),
            "采购金额".to_string(),
            "状态".to_string(),
            "创建时间".to_string(),
        ];

        let rows: Vec<Vec<String>> = orders
            .into_iter()
            .map(|o| {
                vec![
                    o.order_no,
                    o.supplier_id.to_string(),
                    o.total_amount.to_string(),
                    o.order_status,
                    o.created_at.format("%Y-%m-%d %H:%M").to_string(),
                ]
            })
            .collect();

        Ok(ReportData {
            columns,
            rows,
            total_count: total,
        })
    }

    /// 导出报表数据
    pub fn export_report(
        &self,
        data: &ReportData,
        format: ExportFormat,
    ) -> Result<Vec<u8>, AppError> {
        match format {
            ExportFormat::CSV => self.export_csv(data),
            ExportFormat::JSON => self.export_json(data),
            ExportFormat::Excel => self.export_excel_bytes(data),
            ExportFormat::PDF => self.export_pdf_bytes(data),
        }
    }

    /// PDF 导出
    pub fn export_pdf(&self, data: &ReportData, title: &str) -> Result<PdfExportResult, AppError> {
        let pdf_bytes = self.generate_pdf(data, title)?;
        let filename = format!("{}_{}.pdf", title, Utc::now().format("%Y%m%d%H%M%S"));
        Ok(PdfExportResult {
            data: pdf_bytes,
            filename,
        })
    }

    /// Excel 导出
    pub fn export_excel(
        &self,
        data: &ReportData,
        title: &str,
    ) -> Result<ExcelExportResult, AppError> {
        let xlsx_bytes = self.generate_xlsx(data, title)?;
        let filename = format!("{}_{}.xlsx", title, Utc::now().format("%Y%m%d%H%M%S"));
        Ok(ExcelExportResult {
            data: xlsx_bytes,
            filename,
        })
    }

    /// 生成 PDF 字节流（使用简单的 PDF 1.4 格式）
    fn generate_pdf(&self, data: &ReportData, title: &str) -> Result<Vec<u8>, AppError> {
        let mut pdf = Vec::new();

        // 计算页面尺寸和内容
        let page_width = 595.0_f64;
        let page_height = 842.0_f64;
        let margin = 50.0_f64;
        let line_height = 14.0_f64;
        let col_count = data.columns.len().max(1);
        let usable_width = page_width - 2.0 * margin;
        let _col_width = usable_width / col_count as f64;

        // 构建文本内容
        let mut text_lines: Vec<String> = Vec::new();
        text_lines.push(title.to_string());
        text_lines.push(String::new());

        // 表头
        let header_line = data.columns.join("  |  ");
        let header_len = header_line.len().min(80);
        text_lines.push(header_line);
        text_lines.push("-".repeat(header_len));

        // 数据行
        for row in &data.rows {
            let line = row.join("  |  ");
            // 截断过长行
            if line.len() > 100 {
                text_lines.push(format!("{}...", &line[..97]));
            } else {
                text_lines.push(line);
            }
        }

        let total_lines = text_lines.len();
        let content_height = total_lines as f64 * line_height;
        let pages_needed = ((content_height / (page_height - 2.0 * margin)).ceil() as usize).max(1);

        // PDF Header
        pdf.extend_from_slice(b"%PDF-1.4\n");

        let mut offsets = Vec::new();
        let mut page_refs = Vec::new();

        // Object 1: Catalog
        offsets.push(pdf.len());
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

        // Object 2: Pages
        offsets.push(pdf.len());
        let page_refs_str: String = (3..3 + pages_needed)
            .map(|i| format!("{} 0 R", i))
            .collect::<Vec<_>>()
            .join(" ");
        pdf.extend_from_slice(
            format!(
                "2 0 obj\n<< /Type /Pages /Kids [{}] /Count {} >>\nendobj\n",
                page_refs_str, pages_needed
            )
            .as_bytes(),
        );

        // Page objects and content streams
        let mut obj_num = 3;
        for _page_idx in 0..pages_needed {
            page_refs.push(obj_num);
            let content_obj = obj_num + pages_needed;

            offsets.push(pdf.len());
            pdf.extend_from_slice(format!(
                "{} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Contents {} 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n",
                obj_num, page_width as u32, page_height as u32, content_obj
            ).as_bytes());
            obj_num += 1;
        }

        // Content streams
        for page_idx in 0..pages_needed {
            offsets.push(pdf.len());

            let start_line = page_idx * ((page_height - 2.0 * margin) / line_height) as usize;
            let end_line = (start_line + ((page_height - 2.0 * margin) / line_height) as usize)
                .min(total_lines);

            let mut stream_content = String::new();
            stream_content.push_str("BT\n");
            stream_content.push_str("/F1 10 Tf\n");

            let mut y_pos = page_height - margin;

            #[allow(clippy::needless_range_loop)]
            for line_idx in start_line..end_line {
                y_pos -= line_height;
                let y_str = format!("{:.1}", y_pos);
                let escaped = text_lines[line_idx]
                    .replace('\\', "\\\\")
                    .replace('(', "\\(")
                    .replace(')', "\\)");
                if line_idx == 0 {
                    stream_content.push_str(&format!("{} {} Td\n", margin, y_str));
                    stream_content.push_str("/F1 14 Tf\n");
                    stream_content.push_str(&format!("({}) Tj\n", escaped));
                    stream_content.push_str("/F1 10 Tf\n");
                } else {
                    stream_content.push_str(&format!("0 -{} Td\n", line_height));
                    stream_content.push_str(&format!("({}) Tj\n", escaped));
                }
            }

            stream_content.push_str("ET\n");

            let stream_bytes = stream_content.as_bytes();
            offsets.push(pdf.len());
            pdf.extend_from_slice(
                format!(
                    "{} 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n",
                    obj_num + page_idx,
                    stream_bytes.len(),
                    stream_content
                )
                .as_bytes(),
            );
        }

        // Font object
        offsets.push(pdf.len());
        pdf.extend_from_slice(
            b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
        );

        // Cross-reference table
        let xref_offset = pdf.len();
        pdf.extend_from_slice(b"xref\n");
        pdf.extend_from_slice(format!("0 {}\n", offsets.len() + 1).as_bytes());
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        for offset in &offsets {
            pdf.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
        }

        pdf.extend_from_slice(b"trailer\n");
        pdf.extend_from_slice(
            format!("<< /Size {} /Root 1 0 R >>\n", offsets.len() + 1).as_bytes(),
        );
        pdf.extend_from_slice(b"startxref\n");
        pdf.extend_from_slice(format!("{}\n%%EOF\n", xref_offset).as_bytes());

        Ok(pdf)
    }

    /// 生成 XLSX 字节流（OOXML 格式）
    fn generate_xlsx(&self, data: &ReportData, title: &str) -> Result<Vec<u8>, AppError> {
        let mut buf = Vec::new();
        {
            let writer = std::io::Cursor::new(&mut buf);
            let mut zip = zip::ZipWriter::new(writer);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);

            // [Content_Types].xml
            zip.start_file("[Content_Types].xml", options)
                .map_err(|e| AppError::internal(e.to_string()))?;
            zip.write_all(CONTENT_TYPES_XML.as_bytes())
                .map_err(|e| AppError::internal(e.to_string()))?;

            // _rels/.rels
            zip.start_file("_rels/.rels", options)
                .map_err(|e| AppError::internal(e.to_string()))?;
            zip.write_all(RELS_XML.as_bytes())
                .map_err(|e| AppError::internal(e.to_string()))?;

            // xl/workbook.xml
            zip.start_file("xl/workbook.xml", options)
                .map_err(|e| AppError::internal(e.to_string()))?;
            zip.write_all(WORKBOOK_XML.as_bytes())
                .map_err(|e| AppError::internal(e.to_string()))?;

            // xl/_rels/workbook.xml.rels
            zip.start_file("xl/_rels/workbook.xml.rels", options)
                .map_err(|e| AppError::internal(e.to_string()))?;
            zip.write_all(WORKBOOK_RELS_XML.as_bytes())
                .map_err(|e| AppError::internal(e.to_string()))?;

            // xl/styles.xml
            zip.start_file("xl/styles.xml", options)
                .map_err(|e| AppError::internal(e.to_string()))?;
            zip.write_all(STYLES_XML.as_bytes())
                .map_err(|e| AppError::internal(e.to_string()))?;

            // xl/worksheets/sheet1.xml
            zip.start_file("xl/worksheets/sheet1.xml", options)
                .map_err(|e| AppError::internal(e.to_string()))?;
            let sheet_xml = self.build_sheet_xml(data, title);
            zip.write_all(sheet_xml.as_bytes())
                .map_err(|e| AppError::internal(e.to_string()))?;

            // xl/sharedStrings.xml
            zip.start_file("xl/sharedStrings.xml", options)
                .map_err(|e| AppError::internal(e.to_string()))?;
            let strings_xml = self.build_shared_strings_xml(data, title);
            zip.write_all(strings_xml.as_bytes())
                .map_err(|e| AppError::internal(e.to_string()))?;

            zip.finish()
                .map_err(|e| AppError::internal(e.to_string()))?;
        }
        Ok(buf)
    }

    /// 构建 Sheet XML
    fn build_sheet_xml(&self, data: &ReportData, _title: &str) -> String {
        let mut xml = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
        );
        xml.push_str("\n<sheetData>\n");

        let mut string_index = 0usize;

        // Title row
        xml.push_str(&format!(
            "<row r=\"1\"><c r=\"A1\" t=\"s\"><v>{}</v></c></row>\n",
            string_index
        ));
        string_index += 1;

        // Header row
        xml.push_str("<row r=\"2\">");
        for (col_idx, _col) in data.columns.iter().enumerate() {
            let col_letter = column_letter(col_idx);
            xml.push_str(&format!(
                "<c r=\"{}2\" t=\"s\"><v>{}</v></c>",
                col_letter, string_index
            ));
            string_index += 1;
        }
        xml.push_str("</row>\n");

        // Data rows
        for (row_idx, row) in data.rows.iter().enumerate() {
            let row_num = row_idx + 3;
            xml.push_str(&format!("<row r=\"{}\">", row_num));
            for (col_idx, cell) in row.iter().enumerate() {
                let col_letter = column_letter(col_idx);
                // Try to parse as number
                if let Ok(_num) = cell.parse::<f64>() {
                    xml.push_str(&format!(
                        "<c r=\"{}{}\"><v>{}</v></c>",
                        col_letter, row_num, cell
                    ));
                } else {
                    xml.push_str(&format!(
                        "<c r=\"{}{}\" t=\"s\"><v>{}</v></c>",
                        col_letter, row_num, string_index
                    ));
                    string_index += 1;
                }
            }
            xml.push_str("</row>\n");
        }

        xml.push_str("</sheetData>\n</worksheet>");
        xml
    }

    /// 构建 SharedStrings XML
    fn build_shared_strings_xml(&self, data: &ReportData, title: &str) -> String {
        let mut strings: Vec<String> = Vec::new();
        strings.push(title.to_string());
        for col in &data.columns {
            strings.push(col.clone());
        }
        for row in &data.rows {
            for cell in row {
                // Only add non-numeric strings
                if cell.parse::<f64>().is_err() {
                    strings.push(cell.clone());
                }
            }
        }

        let count = strings.len();
        let mut xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="{}" uniqueCount="{}">"#,
            count, count
        );
        for s in &strings {
            let escaped = s
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;");
            xml.push_str(&format!("\n<si><t>{}</t></si>", escaped));
        }
        xml.push_str("\n</sst>");
        xml
    }

    /// 导出CSV格式
    fn export_csv(&self, data: &ReportData) -> Result<Vec<u8>, AppError> {
        let mut wtr = csv::Writer::from_writer(Vec::new());
        wtr.write_record(&data.columns)
            .map_err(|e| AppError::internal(e.to_string()))?;
        for row in &data.rows {
            wtr.write_record(row)
                .map_err(|e| AppError::internal(e.to_string()))?;
        }
        wtr.into_inner()
            .map_err(|e| AppError::internal(e.to_string()))
    }

    /// 导出JSON格式
    fn export_json(&self, data: &ReportData) -> Result<Vec<u8>, AppError> {
        let json_data: Vec<serde_json::Map<String, serde_json::Value>> = data
            .rows
            .iter()
            .map(|row| {
                let mut map = serde_json::Map::new();
                for (i, col) in data.columns.iter().enumerate() {
                    if let Some(val) = row.get(i) {
                        map.insert(col.clone(), serde_json::Value::String(val.clone()));
                    }
                }
                map
            })
            .collect();

        let json =
            serde_json::to_string(&json_data).map_err(|e| AppError::internal(e.to_string()))?;
        Ok(json.into_bytes())
    }

    /// 导出 Excel 字节（内部方法）
    fn export_excel_bytes(&self, data: &ReportData) -> Result<Vec<u8>, AppError> {
        self.generate_xlsx(data, "Report")
    }

    /// 导出 PDF 字节（内部方法）
    fn export_pdf_bytes(&self, data: &ReportData) -> Result<Vec<u8>, AppError> {
        self.generate_pdf(data, "Report")
    }

    /// 创建报表订阅
    pub fn create_subscription(
        user_id: i32,
        req: CreateSubscriptionRequest,
    ) -> Result<ReportSubscription, AppError> {
        if req.name.trim().is_empty() {
            return Err(AppError::validation("订阅名称不能为空"));
        }
        if req.template_id.trim().is_empty() {
            return Err(AppError::validation("模板ID不能为空"));
        }
        if req.recipients.is_empty() {
            return Err(AppError::validation("至少需要一个收件人"));
        }

        let valid_frequencies = ["daily", "weekly", "monthly", "quarterly"];
        if !valid_frequencies.contains(&req.frequency.as_str()) {
            return Err(AppError::validation(format!(
                "频率必须是以下之一: {:?}",
                valid_frequencies
            )));
        }

        let now = Utc::now().naive_utc();
        let next_run = Self::calculate_next_run(&req.frequency, now);

        Ok(ReportSubscription {
            id: format!("sub_{}", now.and_utc().timestamp_millis()),
            user_id,
            template_id: req.template_id,
            name: req.name,
            frequency: req.frequency,
            recipients: req.recipients,
            format: req.format.unwrap_or_else(|| "pdf".to_string()),
            enabled: true,
            created_at: now,
            next_run_at: Some(next_run),
        })
    }

    /// 计算下次执行时间
    fn calculate_next_run(frequency: &str, from: NaiveDateTime) -> NaiveDateTime {
        use chrono::Duration;
        match frequency {
            "daily" => from + Duration::days(1),
            "weekly" => from + Duration::weeks(1),
            "monthly" => from + Duration::days(30),
            "quarterly" => from + Duration::days(90),
            _ => from + Duration::days(1),
        }
    }
}

/// 生成 Excel 列字母（A, B, ..., Z, AA, AB, ...）
fn column_letter(index: usize) -> String {
    let mut col = String::new();
    let mut n = index;
    loop {
        col.insert(0, (b'A' + (n % 26) as u8) as char);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    col
}

// XLSX 模板常量
const CONTENT_TYPES_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
<Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>
<Override PartName="/xl/sharedStrings.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml"/>
</Types>"#;

const RELS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#;

const WORKBOOK_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<sheets>
<sheet name="Sheet1" sheetId="1" r:id="rId1"/>
</sheets>
</workbook>"#;

const WORKBOOK_RELS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="sharedStrings.xml"/>
</Relationships>"#;

const STYLES_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<fonts count="2">
<font><sz val="10"/><name val="Arial"/></font>
<font><b/><sz val="14"/><name val="Arial"/></font>
</fonts>
<fills count="3">
<fill><patternFill patternType="none"/></fill>
<fill><patternFill patternType="gray125"/></fill>
<fill><patternFill patternType="solid"><fgColor rgb="FF4472C4"/></patternFill></fill>
</fills>
<borders count="1">
<border><left/><right/><top/><bottom/><diagonal/></border>
</borders>
<cellStyleXfs count="1"><xf/></cellStyleXfs>
<cellXfs count="1">
<xf numFmtId="0" fontId="0" fillId="0" borderId="0"/>
</cellXfs>
</styleSheet>"#;
