//! 客户服务（facade）
//!
//! 拆分：本文件作为 facade，仅保留 CustomerService struct + new 构造函数 + DTOs 重导出。
//! impl 块迁移至 `customer_ops` 子模块（crud / query / contact / update / types），
//! 通过 db / search_syncer 字段 pub(crate) 让子模块访问，外部引用路径保持不变。
//!
//! 子模块对外可见的方法（pub fn）签名不变，外部调用方仍可通过
//! `crate::services::customer_service::{CustomerService, CreateCustomerArgs, ...}` 访问。

use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::search::{SearchClient, SearchSyncer};

// 重新导出 DTOs（迁移至 customer_ops::types），保持外部引用路径不变
pub use crate::services::customer_ops::types::{
    CreateCustomerArgs, CreateCustomerContactRequest, UpdateCustomerArgs,
    UpdateCustomerContactRequest,
};

/// 客户服务
///
/// 批次 124 v8 复审 P1 修复：注入 search_syncer 实现 PG→ES 写入同步。
/// - create/update/delete 事务提交后调用 sync_customer 将最新数据同步到 ES
/// - ES 同步失败仅记录 tracing::warn!（最终一致性），不回滚 PG 事务
pub struct CustomerService {
    /// 数据库连接（pub(crate) 供 customer_ops 子模块访问）
    pub(crate) db: Arc<DatabaseConnection>,
    /// ES 同步器（PG→ES 写入同步），批次 124 接入
    pub(crate) search_syncer: Arc<SearchSyncer>,
}

impl CustomerService {
    /// 创建服务实例（注入 db 与 search_client）
    pub fn new(db: Arc<DatabaseConnection>, search_client: Arc<dyn SearchClient>) -> Self {
        Self {
            db,
            search_syncer: Arc::new(SearchSyncer::new(search_client)),
        }
    }
}
