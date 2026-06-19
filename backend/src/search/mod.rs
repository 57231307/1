//! P9-8 搜索模块
//!
//! Elasticsearch 集成入口

pub mod elastic;

pub use elastic::{
    CustomerDoc, DocType, ElasticClient, ProductDoc, SalesOrderDoc, SalesOrderItemDoc, SearchClient,
    SearchError, SearchHit, SearchQuery, SearchResult, SearchSyncer,
};
