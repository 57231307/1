//! 销售订单核心服务（so/order）
//!
//! 包含销售订单的 struct + new() 入口。
//! 11 个 pub fn 已按 P9-2 拆分到 3 个子模块：
//! - `order_crud`     销售订单 CRUD（create_order / update_order / delete_order）
//! - `order_query`    销售订单查询（list_orders / get_order_detail / get_order_statistics）
//! - `order_workflow` 销售订单工作流（cancel_order / submit_order / approve_order / complete_order）
//!
//! 通过 `impl SalesService` 跨文件分布，所有方法调用方代码路径不变。

use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::search::{SearchClient, SearchSyncer};

/// 销售订单服务
///
/// 批次 125 v8 复审 P1 修复：注入 search_syncer 实现 PG→ES 写入同步。
/// - create/update/delete 事务提交后调用 sync_sales_order 将最新数据同步到 ES
/// - ES 同步失败仅记录 tracing::warn!（最终一致性），不回滚 PG 事务
pub struct SalesService {
    pub(crate) db: Arc<DatabaseConnection>,
    /// ES 同步器（PG→ES 写入同步），批次 125 接入
    pub(crate) search_syncer: Arc<SearchSyncer>,
}

impl SalesService {
    /// 创建销售订单服务实例
    pub fn new(db: Arc<DatabaseConnection>, search_client: Arc<dyn SearchClient>) -> Self {
        Self {
            db,
            search_syncer: Arc::new(SearchSyncer::new(search_client)),
        }
    }
}
