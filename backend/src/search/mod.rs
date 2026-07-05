//! P9-8 搜索模块
//!
//! Elasticsearch 集成入口

pub mod elastic;

// 批次 104 P0-1 修复：仅 re-export 外部实际使用的项
// DocType / SalesOrderItemDoc / SearchError / SearchHit / SearchResult
// 在 elastic.rs 内部为 pub，外部如需访问可通过 crate::search::elastic::X 路径
// 批次 123 v8 复审 P1 修复：新增导出 ensure_indices 供 app_state 启动时调用
// 批次 124 v8 复审 P1 修复：新增导出 SearchSyncer 供 customer_service 注入
// 批次 125 v8 复审 P1 修复：新增导出 SalesOrderItemDoc 供 order_crud.build_sales_order_doc 使用
pub use elastic::{
    ensure_indices, indices, CustomerDoc, ElasticClient, ProductDoc, SalesOrderDoc,
    SalesOrderItemDoc, SearchClient, SearchQuery, SearchSyncer,
};
