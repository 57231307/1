//! N+1 查询修复工具集（P4-1 性能优化）
//!
//! 提供：
//! - `batch_lookup`: 一次 `IN (...)` 查询替代 N 次单点查询
//! - `group_by_id`: 把批量查询结果按 id 索引
//!
//! SeaORM 的 `find_in` API 本身就是为解决 N+1 设计的：
//! ```rust,ignore
//! use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};
//! let orders = Order::find()
//!     .filter(Column::CustomerId.is_in(customer_ids.clone()))
//!     .all(db).await?;
//! ```
//!
//! 本工具提供"按主键集合批量加载 + 内存分组"的样板代码。

use std::collections::HashMap;
use std::hash::Hash;

/// 内存按主键分组（N+1 修复的客户端组装步骤）
///
/// 使用场景：
/// 1. `let rows = batch_query(ids).await?;` 一次 `IN (...)` 查询
/// 2. `let map = group_by_id(rows, |r| r.id);` 构造 id -> row 索引
/// 3. 业务侧按 id 顺序 O(1) 取值
pub fn group_by_id<T, K, F>(rows: Vec<T>, key_fn: F) -> HashMap<K, T>
where
    K: Eq + Hash,
    F: Fn(&T) -> K,
{
    let mut map = HashMap::with_capacity(rows.len());
    for row in rows {
        map.insert(key_fn(&row), row);
    }
    map
}

/// 批量 ID 切分器：把大列表切成 N 个 chunk，避免 `IN (...)` 参数超限
///
/// PostgreSQL 单条 IN 子句的参数上限约为 65535 个，
/// 业务侧传 10 万 ID 时必须分批。
pub fn chunk_ids<T: Clone>(ids: &[T], chunk_size: usize) -> Vec<Vec<T>> {
    if chunk_size == 0 {
        return vec![ids.to_vec()];
    }
    ids.chunks(chunk_size).map(|c| c.to_vec()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 测试_group_by_id_按主键索引() {
        // 中文测试名：测试 group_by_id 按主键索引
        #[derive(Clone, Debug)]
        struct Row {
            id: i64,
            name: String,
        }
        let rows = vec![
            Row { id: 1, name: "甲".to_string() },
            Row { id: 2, name: "乙".to_string() },
            Row { id: 3, name: "丙".to_string() },
        ];
        let map = group_by_id(rows, |r| r.id);
        assert_eq!(map.len(), 3);
        assert_eq!(map.get(&2).unwrap().name, "乙");
    }

    #[test]
    fn 测试_chunk_ids_分批() {
        // 中文测试名：测试 chunk_ids 大列表分批
        let ids: Vec<i64> = (1..=10).collect();
        let chunks = chunk_ids(&ids, 3);
        assert_eq!(chunks.len(), 4);
        assert_eq!(chunks[0], vec![1, 2, 3]);
        assert_eq!(chunks[3], vec![10]);
    }

    #[test]
    fn 测试_chunk_ids_空列表() {
        // 中文测试名：测试 chunk_ids 空列表
        let ids: Vec<i64> = vec![];
        let chunks = chunk_ids(&ids, 3);
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].is_empty());
    }
}
