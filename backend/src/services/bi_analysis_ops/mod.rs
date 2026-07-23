//! BI 分析 ops 子模块入口（bi_analysis_ops）
//!
//! 批次 490 D10-3a 拆分：从原 `bi_analysis_service.rs` 拆出 BiAnalysisService 的 impl 块。
//! - `sales`：销售聚合分析（按时间/客户/产品/区域/品类聚合 + 趋势）
//! - `profit`：利润分析 + 核心 KPI（同比/环比增长率）
//! - `drilldown`：下钻分析（年→月、月→日、客户→订单、产品→订单）
//! - `olap`：OLAP 操作（切片/切块/上卷/透视）
//! - `types`：8 个对外 response struct + 13 个 FromQueryResult 中间结构 + KpiCurrentMetrics
//!
//! BiAnalysisService struct 定义与 `new` / `new_with_data_scope` 构造函数保留在 facade
//! `bi_analysis_service` 中，impl 块分散到本子模块，Rust 允许同一 crate 多文件多 impl 块。

pub mod drilldown;
pub mod olap;
pub mod profit;
pub mod sales;
pub mod types;

// re-export 对外 response struct，facade 通过 `pub use` 二次 re-export 保持外部引用路径不变
pub use types::{
    BiResponse, CategoryStat, CustomerRank, KpiSummary, ProductRank, ProfitAnalysis, RegionStat,
    TimeSeriesPoint,
};
