//! 给 customers 和 suppliers 表补 RLS 需要的列
//!
//! 问题：m0054_enable_rls_policies 引用 customers.owner_id 和 suppliers.created_by，
//! 但 m0001 创建这两张表时没有这些列，导致迁移执行到 m0054 时报
//! "column owner_id does not exist"（E2E 失败根因）。
//!
//! 本迁移在 m0054 之前执行，补上缺失的列。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- customers 表补 owner_id 列（RLS 行级安全需要，0 表示公海客户）
                ALTER TABLE "customers" ADD COLUMN IF NOT EXISTS "owner_id" INTEGER NOT NULL DEFAULT 0;
                COMMENT ON COLUMN "customers"."owner_id" IS '客户归属人 ID（0=公海客户，对所有用户可见）';
                CREATE INDEX IF NOT EXISTS "idx_customers_owner" ON "customers" ("owner_id");

                -- suppliers 表补 created_by 列（RLS 行级安全需要，NULL 表示历史数据）
                ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "created_by" INTEGER;
                COMMENT ON COLUMN "suppliers"."created_by" IS '供应商创建人 ID（NULL=历史数据，对所有用户可见）';
                CREATE INDEX IF NOT EXISTS "idx_suppliers_created_by" ON "suppliers" ("created_by");
                "#,
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                DROP INDEX IF EXISTS "idx_suppliers_created_by";
                ALTER TABLE "suppliers" DROP COLUMN IF EXISTS "created_by";
                DROP INDEX IF EXISTS "idx_customers_owner";
                ALTER TABLE "customers" DROP COLUMN IF EXISTS "owner_id";
                "#,
            )
            .await?;
        Ok(())
    }
}
