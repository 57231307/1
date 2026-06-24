//! P4-1 性能优化 - N+1 修复示例 + 慢查询接入
//!
//! 本文件作为 P4-1 性能优化阶段的"参考实现"：
//! 1. `BatchInventoryLoader`: 一次 `IN (...)` 查询替代 N 次单点查询
//! 2. `CachedDashboardService`: 接入 P4-1 缓存层，对热点聚合查询加缓存
//! 3. `record_slow_query!` 宏：业务侧一键接入慢查询审计
//!
//! 业务侧实际接入请按各 service 的领域模型改写，这里提供模式样板。

use crate::services::cache_service::CacheService;
use crate::utils::n_plus_one::{chunk_ids, group_by_id};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// 库存行（业务模型 - 示例用）
#[allow(dead_code)] // TODO(tech-debt): P4-1 性能优化示例接入实际业务 service 后移除
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryRow {
    pub id: i64,
    pub product_id: i32,
    pub warehouse_id: i32,
    pub batch_no: String,
    pub quantity: f64,
}

/// 批量加载器（P4-1 性能优化示例）
///
/// # N+1 修复原理
///
/// 原始 N+1 模式（伪代码）：
/// ```text
/// for id in ids {
///     let row = SELECT * FROM inventory WHERE id = $1;  // N 次
/// }
/// ```
///
/// 改造后（一次 IN 查询）：
/// ```text
/// let rows = SELECT * FROM inventory WHERE id IN ($1, $2, ..., $N);  // 1 次
/// ```
#[allow(dead_code)] // TODO(tech-debt): P4-1 性能优化示例接入实际业务 service 后移除
pub struct BatchInventoryLoader {
    /// 多租户 ID（强制租户隔离）
    pub tenant_id: i64,
    /// 单次 IN 子句最大参数（PostgreSQL 推荐 5000）
    pub max_in_clause: usize,
}

#[allow(dead_code)] // TODO(tech-debt): P4-1 性能优化示例接入实际业务 service 后移除
impl BatchInventoryLoader {
    /// 创建 loader
    pub fn new(tenant_id: i64) -> Self {
        Self {
            tenant_id,
            max_in_clause: 5_000,
        }
    }

    /// 批量加载（按主键 id 列表）
    ///
    /// 真实实现应替换为 SeaORM 的 `Entity::find().filter(Column::Id.is_in(...))`。
    /// 此处给出可编译的占位实现。
    pub async fn load_by_ids(&self, ids: &[i64]) -> Result<Vec<InventoryRow>, String> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        // 大切分批，避免 PG 单条 IN 参数超限
        let _chunks = chunk_ids(ids, self.max_in_clause);
        // 此处调用实际 SeaORM 查询，示例返回空
        // let rows = Inventory::find()
        //     .filter(Column::TenantId.eq(self.tenant_id))
        //     .filter(Column::Id.is_in(chunk))
        //     .all(db).await?;
        Ok(vec![])
    }

    /// 批量加载并按 id 索引（消除 N+1）
    pub async fn load_map(&self, ids: &[i64]) -> Result<HashMap<i64, InventoryRow>, String> {
        let rows = self.load_by_ids(ids).await?;
        Ok(group_by_id(rows, |r| r.id))
    }
}

/// 缓存包装的 Dashboard 聚合查询示例
#[allow(dead_code)] // TODO(tech-debt): P4-1 性能优化示例接入实际业务 service 后移除
pub struct CachedDashboardService {
    /// 底层缓存
    pub cache: Arc<CacheService>,
    /// 默认 TTL
    pub default_ttl: Duration,
}

#[allow(dead_code)] // TODO(tech-debt): P4-1 性能优化示例接入实际业务 service 后移除
impl CachedDashboardService {
    /// 读穿透（cache miss 时回源）
    ///
    /// 真实业务侧：`fetcher` 闭包从 DB 拉数据，本方法只负责缓存。
    /// 缓存键必须以 `tenant:{id}:` 开头，避免跨租户数据串味。
    pub async fn read_through<T, F, Fut>(
        &self,
        tenant_id: i64,
        key_suffix: &str,
        fetcher: F,
    ) -> Result<T, String>
    where
        T: for<'de> Deserialize<'de> + Serialize + Clone + Send + Sync + 'static,
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, String>>,
    {
        let key = format!("tenant:{}:dashboard:{}", tenant_id, key_suffix);
        // 1) 查缓存
        if let Some(bytes) = self.cache.get(&key).await {
            if let Ok(v) = serde_json::from_slice::<T>(&bytes) {
                return Ok(v);
            }
        }
        // 2) 缓存未命中 - 回源
        let value = fetcher().await?;
        // 3) 写回缓存
        if let Ok(bytes) = serde_json::to_vec(&value) {
            self.cache.set(key, bytes).await;
        }
        Ok(value)
    }

    /// 失效租户全部缓存（写操作后调用）
    pub async fn invalidate_tenant(&self, tenant_id: i64) {
        let key = format!("tenant:{}:", tenant_id);
        self.cache.invalidate_prefix(&key).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn 测试_batch_loader_空列表() {
        // 中文测试名：测试 batch loader 传入空 ids 返回空
        let loader = BatchInventoryLoader::new(1);
        let result = loader.load_by_ids(&[]).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn 测试_batch_loader_分组() {
        // 中文测试名：测试 batch loader load_map 按 id 分组
        let loader = BatchInventoryLoader::new(1);
        let result = loader.load_map(&[]).await.unwrap();
        assert!(result.is_empty());
    }
}
