use crate::utils::error::AppError;
use chrono::Utc;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, FromQueryResult, PaginatorTrait, QueryFilter,
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
        db: &'db impl ConnectionTrait,
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

    /// 生成可指定流水位数的单号: {前缀}{YYYYMMDD}{width位流水号}
    /// 例如 width=4 时: `IC202605140001`
    ///
    /// # 并发安全说明
    /// 当前实现基于"读当日数量 + 1"策略（无锁），存在理论上并发请求产生相同流水号
    /// 的窗口（窗口大小 = 单次 COUNT 查询耗时）。业务侧应在创建单据的事务中依赖
    /// 单据号列的 `UNIQUE` 约束进行最终去重，或后续接入 PostgreSQL 序列
    /// (`SEQUENCE`) + `nextval` 实现真正无并发冲突的生成方案。
    pub async fn generate_no_with_width<'db, E, C>(
        db: &'db impl ConnectionTrait,
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

        // 统计今日的单据数量
        let count = E::find()
            .filter(column.starts_with(&date_prefix))
            .count(db)
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
