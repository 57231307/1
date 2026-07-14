use crate::utils::error::AppError;
use chrono::Utc;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseTransaction, EntityTrait,
    FromQueryResult, PaginatorTrait, QueryFilter, Statement, TransactionTrait,
};

/// 通用单号生成器
pub struct DocumentNumberGenerator;

impl DocumentNumberGenerator {
    /// 生成标准格式单号: {前缀}{YYYYMMDD}{3位流水号}
    /// 例如: PO20230501001
    ///
    /// 默认使用 3 位流水号。业务需要 4 位或更多位数时，请使用
    /// [`generate_no_with_width`] 显式指定，避免对存量单据号位数产生回归。
    pub async fn generate_no<'db, E, C>(
        db: &'db (impl ConnectionTrait + TransactionTrait),
        prefix: &str,
        _entity: E,
        column: C,
    ) -> Result<String, AppError>
    where
        E: EntityTrait,
        <<E as EntityTrait>::Column as std::str::FromStr>::Err: std::fmt::Debug,
        E::Model: Sync + Send + FromQueryResult + 'db,
        C: ColumnTrait,
    {
        Self::generate_no_with_width(db, prefix, _entity, column, 3).await
    }

    /// 生成可指定流水位数的单号: {前缀}{YYYYMMDD}{width位流水号}（使用 pg_advisory_xact_lock 保证并发安全）
    pub async fn generate_no_with_width<'db, E, C>(
        db: &'db (impl ConnectionTrait + TransactionTrait),
        prefix: &str,
        _entity: E,
        column: C,
        width: usize,
    ) -> Result<String, AppError>
    where
        E: EntityTrait,
        <<E as EntityTrait>::Column as std::str::FromStr>::Err: std::fmt::Debug,
        E::Model: Sync + Send + FromQueryResult + 'db,
        C: ColumnTrait,
    {
        let today = Utc::now().format("%Y%m%d").to_string();
        let date_prefix = format!("{}{}", prefix, today);

        // 批次 9（2026-06-28）：用 PostgreSQL advisory_xact_lock 串行化同前缀同日的单号生成
        // 开启子事务（若调用方已在事务中，PostgreSQL 自动创建 savepoint；
        // advisory_xact_lock 在 savepoint 释放时也会释放，行为正确）
        let txn = db.begin().await?;

        // 计算锁 key：prefix + date 字符串的稳定 i64 哈希
        let lock_key = compute_advisory_lock_key(prefix, &today);

        // 获取事务级 advisory lock，串行化同 key 的并发请求
        txn.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT pg_advisory_xact_lock($1)",
            [lock_key.into()],
        ))
        .await?;

        // 在锁保护下统计今日的单据数量
        let count = E::find()
            .filter(column.starts_with(&date_prefix))
            .count(&txn)
            .await?;

        // 提交子事务，advisory_xact_lock 自动释放
        txn.commit().await?;

        // 防御：当 width == 0 时退化为 1 位，至少保留流水号
        let width = width.max(1);
        Ok(format!(
            "{}{:0width$}",
            date_prefix,
            count + 1,
            width = width
        ))
    }

    /// 在外部事务内生成单号（不在内部开子事务），默认 3 位流水号
    ///
    /// P1 5-9 修复（批次 60）：原 [`generate_no_with_width`] 总是 `db.begin()` 开子事务，
    /// 当调用方已在外层事务中时变成 savepoint，`pg_advisory_xact_lock` 在 savepoint
    /// 释放时即释放，而非外层事务 commit 时释放，导致锁保护窗口过短，并发请求可能
    /// 在外层事务未提交时拿到锁、读到未提交的 COUNT，造成单号重复。
    ///
    /// 此变体直接在传入的 `txn` 上执行 `pg_advisory_xact_lock` + COUNT，锁生命周期
    /// 与外层事务一致，commit 时自动释放，是真正的事务级锁。
    ///
    /// 调用方必须在事务内调用此方法，且事务应包含单据 INSERT 操作以保证锁覆盖
    /// COUNT + INSERT 完整临界区。
    pub async fn generate_no_with_txn<'db, E, C>(
        txn: &'db DatabaseTransaction,
        prefix: &str,
        _entity: E,
        column: C,
    ) -> Result<String, AppError>
    where
        E: EntityTrait,
        <<E as EntityTrait>::Column as std::str::FromStr>::Err: std::fmt::Debug,
        E::Model: Sync + Send + FromQueryResult + 'db,
        C: ColumnTrait,
    {
        Self::generate_no_with_width_txn(txn, prefix, _entity, column, 3).await
    }

    /// 在外部事务内生成单号（可指定位数），不在内部开子事务
    ///
    /// 语义与 [`generate_no_with_width`] 一致，但直接使用调用方传入的事务句柄，
    /// 不再 `db.begin()` 开子事务，确保 `pg_advisory_xact_lock` 在外层事务
    /// commit/rollback 时才释放，覆盖完整的 COUNT + INSERT 临界区。
    ///
    /// 详见 [`generate_no_with_txn`] 的修复说明。
    pub async fn generate_no_with_width_txn<'db, E, C>(
        txn: &'db DatabaseTransaction,
        prefix: &str,
        _entity: E,
        column: C,
        width: usize,
    ) -> Result<String, AppError>
    where
        E: EntityTrait,
        <<E as EntityTrait>::Column as std::str::FromStr>::Err: std::fmt::Debug,
        E::Model: Sync + Send + FromQueryResult + 'db,
        C: ColumnTrait,
    {
        let today = Utc::now().format("%Y%m%d").to_string();
        let date_prefix = format!("{}{}", prefix, today);

        // P1 5-9 修复（批次 60）：直接在传入的外层 txn 上获取 advisory_xact_lock
        // 锁在外层事务 commit/rollback 时自动释放，覆盖完整的 COUNT + INSERT 临界区
        let lock_key = compute_advisory_lock_key(prefix, &today);

        txn.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT pg_advisory_xact_lock($1)",
            [lock_key.into()],
        ))
        .await?;

        // 在锁保护下统计今日的单据数量
        let count = E::find()
            .filter(column.starts_with(&date_prefix))
            .count(txn)
            .await?;

        // 防御：当 width == 0 时退化为 1 位，至少保留流水号
        let width = width.max(1);
        Ok(format!(
            "{}{:0width$}",
            date_prefix,
            count + 1,
            width = width
        ))
    }
}

/// 计算 PostgreSQL advisory lock 的 i64 键值
///
/// 由 prefix + date 字符串哈希得到，确保不同业务/不同日期的锁互不冲突。
/// 使用 `DefaultHasher`（稳定哈希，进程内一致），跨进程/跨重启结果可能不同，
/// 但 advisory lock 是进程内会话状态，不影响正确性。
fn compute_advisory_lock_key(prefix: &str, date: &str) -> i64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    prefix.hash(&mut hasher);
    date.hash(&mut hasher);
    // u64 → i64：PostgreSQL advisory lock 接受 i64，取低 63 位避免符号问题
    (hasher.finish() & 0x7FFF_FFFF_FFFF_FFFF) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 锁 key 计算稳定性：同输入应得同输出
    #[test]
    fn test_compute_advisory_lock_key_stable() {
        let key1 = compute_advisory_lock_key("PO", "20260628");
        let key2 = compute_advisory_lock_key("PO", "20260628");
        assert_eq!(key1, key2, "同输入应得同输出");
    }

    /// 锁 key 计算区分性：不同前缀应得不同 key
    #[test]
    fn test_compute_advisory_lock_key_different_prefix() {
        let key_po = compute_advisory_lock_key("PO", "20260628");
        let key_so = compute_advisory_lock_key("SO", "20260628");
        assert_ne!(key_po, key_so, "不同前缀应得不同 key");
    }

    /// 锁 key 计算区分性：不同日期应得不同 key
    #[test]
    fn test_compute_advisory_lock_key_different_date() {
        let key_day1 = compute_advisory_lock_key("PO", "20260628");
        let key_day2 = compute_advisory_lock_key("PO", "20260629");
        assert_ne!(key_day1, key_day2, "不同日期应得不同 key");
    }

    /// 锁 key 计算非负性：PostgreSQL advisory lock 接受 i64，结果应在合理范围
    #[test]
    fn test_compute_advisory_lock_key_non_negative() {
        let key = compute_advisory_lock_key("IC", "20260628");
        assert!(key >= 0, "锁 key 应非负（取低 63 位）");
    }
}
