//! 库存质量状态取值规范化（历史 `passed` 行归一为「合格」）
//!
//! 验布放行与批色放行两处历史上把 `inventory_stocks.quality_status` 写成
//! 其他检验域的字面量 `passed`，而可用量计算、缺料预警与可出库筛选一律按中文
//! 「合格」过滤，导致这批已放行的库存在所有可用性查询中永久不可见
//! （账上有货、界面无货）。写入侧已统一为 `inventory_stock_quality_status::PASS`，
//! 本迁移把存量行一并归一，避免同一缺陷在历史数据上继续生效。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
UPDATE "inventory_stocks"
   SET "quality_status" = '合格'
 WHERE "quality_status" = 'passed';
"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 归一后无法区分哪些行原本是 passed，回滚不可逆；
        // 保留 UPDATE 的幂等语义，反向不做任何写操作，避免把正确数据改回错误取值。
        let sql = "SELECT 1;";
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
