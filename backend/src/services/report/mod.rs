//! 报表引擎服务模块（report）
//!
//! 由原 `services/report_engine_service.rs`（2122 行）按业务子领域拆分而来。
//! 子模块：
//! - `tpl`  报表模板管理（预定义模板 + 自定义模板）
//! - `ds`   数据源：聚合查询、报表执行、缓存管理
//! - `exp`  导出器：PDF / Excel / CSV / JSON 多种格式输出
//! - `job`  调度器：报表订阅、cron 表达式、next_run 计算
//!
//! 兼容说明：原 `crate::services::report_engine_service::*` 路径需要由上层
//! `services/mod.rs` 通过 `pub use super::report::*;` 重新导出以保持向后兼容。

#![allow(dead_code)]

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod ds;
pub mod exp;
pub mod job;
pub mod tpl;

// =====================================================
// 共享 DTO（与原 report_engine_service.rs 保持一致）
// =====================================================

/// 预定义报表模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub data_source: String,
    /// 报表类型（sales/purchase/inventory/financial/custom）
    pub report_type: String,
    pub columns: Vec<ReportColumn>,
    pub filters: Vec<ReportFilter>,
    pub supported_formats: Vec<String>,
    pub parameters: Vec<ReportParameter>,
}

/// 报表列定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportColumn {
    /// 字段名（数据中 key）
    pub key: String,
    /// 显示名（handler 使用 title 字段名时通过 alias 反序列化）
    #[serde(alias = "title")]
    pub label: String,
    /// 字段别名 - 兼容 handler 使用的 field/title
    #[serde(alias = "field")]
    pub field_alias: Option<String>,
    pub data_type: String,
    pub format: Option<String>,
    pub aggregation: Option<String>,
    pub sortable: bool,
    pub filterable: bool,
    pub width: Option<u32>,
    pub alignment: Option<String>,
}

/// 报表筛选条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportFilter {
    pub key: String,
    /// 字段名（兼容 handler）
    #[serde(alias = "field")]
    pub field_alias: Option<String>,
    pub label: String,
    /// 算子（兼容 handler）
    #[serde(default)]
    pub operator: Option<String>,
    /// 值（兼容 handler）
    #[serde(default)]
    pub value: Option<String>,
    pub filter_type: String,
    pub default_value: Option<serde_json::Value>,
    pub options: Option<Vec<serde_json::Value>>,
    pub required: bool,
}

/// 报表参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportParameter {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    pub default_value: Option<serde_json::Value>,
    pub description: Option<String>,
}

/// 创建自定义模板请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTemplateRequest {
    pub name: String,
    pub description: String,
    pub category: String,
    pub data_source: String,
    pub report_type: Option<String>,
    pub columns: Vec<ReportColumn>,
    pub filters: Vec<ReportFilter>,
    pub parameters: Vec<ReportParameter>,
    pub supported_formats: Vec<String>,
}

/// 数据源（业务枚举，handler 直接使用字符串变体名）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum DataSource {
    Sales,
    Purchase,
    Inventory,
    Finance,
}

impl DataSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            DataSource::Sales => "sales",
            DataSource::Purchase => "purchase",
            DataSource::Inventory => "inventory",
            DataSource::Finance => "finance",
        }
    }
}

impl std::str::FromStr for DataSource {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "sales" => Ok(DataSource::Sales),
            "purchase" => Ok(DataSource::Purchase),
            "inventory" => Ok(DataSource::Inventory),
            "finance" => Ok(DataSource::Finance),
            _ => Err(format!("不支持的数据源: {}", s)),
        }
    }
}

/// 聚合类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AggregationType {
    Sum,
    Average,
    Count,
    Min,
    Max,
    GroupBy,
    None,
}

impl AggregationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AggregationType::Sum => "sum",
            AggregationType::Average => "average",
            AggregationType::Count => "count",
            AggregationType::Min => "min",
            AggregationType::Max => "max",
            AggregationType::GroupBy => "group_by",
            AggregationType::None => "none",
        }
    }
}

impl std::str::FromStr for AggregationType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "sum" => Ok(AggregationType::Sum),
            "average" | "avg" => Ok(AggregationType::Average),
            "count" => Ok(AggregationType::Count),
            "min" => Ok(AggregationType::Min),
            "max" => Ok(AggregationType::Max),
            "group_by" | "group" => Ok(AggregationType::GroupBy),
            _ => Err(format!("不支持的聚合类型: {}", s)),
        }
    }
}

/// 聚合请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateRequest {
    pub data_source: DataSource,
    pub aggregation_type: AggregationType,
    /// 兼容字符串调用方
    pub data_source_str: Option<String>,
    pub group_by: Vec<String>,
    pub filters: Vec<ReportFilter>,
    pub date_range: Option<DateRange>,
    pub parameters: Option<serde_json::Value>,
    pub limit: Option<u32>,
    pub aggregation_field: Option<String>,
}

/// 日期范围
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

/// 聚合结果（业务字段：columns/rows/total_count，方便 handler 直接序列化）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub total_count: u64,
    /// 兼容旧版的 groups/aggregations/count 字段
    #[serde(default)]
    pub groups: Vec<(String, serde_json::Value)>,
    #[serde(default)]
    pub aggregations: Vec<(String, serde_json::Value)>,
    #[serde(default)]
    pub count: u64,
}

/// 报表数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportData {
    pub columns: Vec<ReportColumn>,
    pub rows: Vec<serde_json::Value>,
    /// handler 使用 total_count
    #[serde(alias = "total_count")]
    pub total_rows: u64,
    pub generated_at: DateTime<Utc>,
    pub summary: Option<serde_json::Value>,
    pub metadata: ReportMetadata,
}

/// 报表元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportMetadata {
    pub data_source: String,
    pub query_time_ms: u64,
    pub cache_hit: bool,
    pub parameters: Option<serde_json::Value>,
}

/// 缓存条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub data: ReportData,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub hit_count: u64,
}

/// 报表执行请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteReportRequest {
    pub template_id: String,
    pub filters: Vec<ReportFilter>,
    pub parameters: Option<serde_json::Value>,
    pub date_range: Option<DateRange>,
    pub format: String,
    pub use_cache: Option<bool>,
}

/// 导出格式（兼容 csv/csv 字符串）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Pdf,
    Excel,
    Csv,
    Json,
}

impl std::str::FromStr for ExportFormat {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pdf" => Ok(ExportFormat::Pdf),
            "excel" | "xlsx" => Ok(ExportFormat::Excel),
            "csv" => Ok(ExportFormat::Csv),
            "json" => Ok(ExportFormat::Json),
            _ => Err(format!("不支持的导出格式: {}", s)),
        }
    }
}

/// PDF 导出结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfExportResult {
    pub file_name: String,
    pub file_size: u64,
    pub page_count: u32,
    pub content: Vec<u8>,
}

/// Excel 导出结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcelExportResult {
    pub file_name: String,
    pub file_size: u64,
    pub sheet_count: u32,
    pub row_count: u64,
    pub content: Vec<u8>,
}

/// 报表订阅
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSubscription {
    pub id: i32,
    pub user_id: i32,
    pub template_id: String,
    pub template_name: String,
    pub cron_expression: String,
    pub format: String,
    pub filters: Vec<ReportFilter>,
    pub parameters: Option<serde_json::Value>,
    pub recipients: Vec<String>,
    pub enabled: bool,
    pub next_run_at: Option<DateTime<Utc>>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub last_status: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建订阅请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSubscriptionRequest {
    pub template_id: String,
    pub cron_expression: String,
    pub format: String,
    pub filters: Vec<ReportFilter>,
    pub parameters: Option<serde_json::Value>,
    pub recipients: Vec<String>,
    pub enabled: bool,
}

// =====================================================
// 共享 Service 结构体（子模块均通过 impl ReportEngineService 扩展）
// =====================================================

/// 报表引擎 Service
pub struct ReportEngineService {
    pub(crate) db: Arc<DatabaseConnection>,
    pub(crate) cache: Arc<RwLock<std::collections::HashMap<String, CacheEntry>>>,
}

impl ReportEngineService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            db,
            cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }
}

// =====================================================
// 共享内部辅助（供 ds.rs 缓存管理使用）
// =====================================================

/// 默认缓存 TTL：5 分钟
pub(crate) const DEFAULT_CACHE_TTL_SECONDS: i64 = 300;

/// 抑制未使用导入警告
#[allow(dead_code)]
fn _unused() {
    let _: Option<Decimal> = None;
}
