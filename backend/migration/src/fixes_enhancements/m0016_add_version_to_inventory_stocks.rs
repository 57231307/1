use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"-- 添加乐观锁版本号到库存表
-- 用于并发控制，防止库存超卖

ALTER TABLE "inventory_stocks" ADD COLUMN "version" INTEGER NOT NULL DEFAULT 0;

COMMENT ON COLUMN "inventory_stocks"."version" IS '乐观锁版本号，用于并发控制';"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"-- 回滚：移除库存表的乐观锁版本号

ALTER TABLE "inventory_stocks" DROP COLUMN "version";"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
