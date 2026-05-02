use sea_orm::{ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, PaginatorTrait};
use chrono::Utc;
use crate::utils::error::AppError;

/// 通用单号生成器
pub struct DocumentNumberGenerator;

impl DocumentNumberGenerator {
    /// 生成标准格式单号: {前缀}{YYYYMMDD}{3位流水号}
    /// 例如: PO20230501001
    pub async fn generate_no<E, C>(
        db: &DatabaseConnection,
        prefix: &str,
        entity: E,
        column: C,
    ) -> Result<String, AppError>
    where
        E: EntityTrait,
        C: ColumnTrait,
    {
        let today = Utc::now().format("%Y%m%d").to_string();
        let date_prefix = format!("{}{}", prefix, today);

        // 统计今日的单据数量
        let count = entity
            .find()
            .filter(column.starts_with(&date_prefix))
            .count(db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(format!("{}{:03}", date_prefix, count + 1))
    }
}