use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"-- 添加 opportunity_id 字段到 sales_orders 表
-- 用于关联 CRM 商机

-- 添加 opportunity_id 字段
ALTER TABLE "sales_orders" ADD COLUMN IF NOT EXISTS "opportunity_id" INTEGER;

-- 添加外键约束
ALTER TABLE "sales_orders" ADD CONSTRAINT "fk_sales_orders_opportunity" 
    FOREIGN KEY ("opportunity_id") REFERENCES "crm_opportunity" ("id");

-- 添加索引
CREATE INDEX IF NOT EXISTS "idx_sales_orders_opportunity_id" ON "sales_orders" ("opportunity_id");

-- 添加注释
COMMENT ON COLUMN "sales_orders"."opportunity_id" IS '关联的CRM商机ID';"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"-- 回滚：移除 sales_orders 表的 opportunity_id 字段

-- 删除索引
DROP INDEX IF EXISTS "idx_sales_orders_opportunity_id";

-- 删除外键约束
ALTER TABLE "sales_orders" DROP CONSTRAINT IF EXISTS "fk_sales_orders_opportunity";

-- 删除字段
ALTER TABLE "sales_orders" DROP COLUMN IF EXISTS "opportunity_id";"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
