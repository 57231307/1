use crate::utils::error::AppError;
use chrono::Utc;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseTransaction, EntityTrait,
    FromQueryResult, PaginatorTrait, QueryFilter, Statement, TransactionSession, TransactionTrait,
};

/// 通用单号生成器
pub struct DocumentNumberGenerator;

impl DocumentNumberGenerator {
    /// 生成标准格式单号: {前缀}{YYYYMMDD}{3位流水号}
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

    /// 生成可指定流水位数的单号
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

        let txn = db
            .begin()
            .await
            .map_err(|e| AppError::internal(format!("开始事务失败: {:?}", e)))?;

        let lock_key = compute_advisory_lock_key(prefix, &today);

        txn.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT pg_advisory_xact_lock($1)",
            [lock_key.into()],
        ))
        .await?;

        let count = E::find()
            .filter(column.starts_with(&date_prefix))
            .count(&txn)
            .await?;

        txn.commit()
            .await
            .map_err(|e| AppError::internal(format!("提交事务失败: {:?}", e)))?;

        let width = std::cmp::Ord::max(width, 1);
        Ok(format!(
            "{}{:0width$}",
            date_prefix,
            count + 1,
            width = width
        ))
    }

    /// 在外部事务内生成单号
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

    /// 在外部事务内生成单号（可指定位数）
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
        let lock_key = compute_advisory_lock_key(prefix, &today);

        txn.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT pg_advisory_xact_lock($1)",
            [lock_key.into()],
        ))
        .await?;

        let count = E::find()
            .filter(column.starts_with(&date_prefix))
            .count(txn)
            .await?;

        let width = std::cmp::Ord::max(width, 1);
        Ok(format!(
            "{}{:0width$}",
            date_prefix,
            count + 1,
            width = width
        ))
    }
}

fn compute_advisory_lock_key(prefix: &str, date: &str) -> i64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    prefix.hash(&mut hasher);
    date.hash(&mut hasher);
    hasher.finish() as i64
}
