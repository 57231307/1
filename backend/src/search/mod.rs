//! P9-8 搜索模块
//!
//! Elasticsearch 集成入口

pub mod elastic;

// 批次 104 P0-1 修复：仅 re-export 外部实际使用的项
// DocType / SalesOrderItemDoc / SearchError / SearchHit / SearchResult / SearchSyncer
// 在 elastic.rs 内部为 pub，外部如需访问可通过 crate::search::elastic::X 路径
pub use elastic::{
    indices, CustomerDoc, ElasticClient, ProductDoc, SalesOrderDoc, SearchClient, SearchQuery,
};
