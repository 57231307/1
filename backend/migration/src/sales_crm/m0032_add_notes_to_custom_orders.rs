//! custom_orders 备注列迁移（批次 88 PH-1）
//!
//! 创建时间: 2026-07-03
//! 关联修复: 占位符 PH-1 — DTO 有 notes 字段但 service 层 `let _ = v;` 丢弃
//!
//! 向 custom_orders 表添加 notes 列（TEXT，可选），存储订单备注。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql =
            r#"-- custom_orders 备注列迁移（批次 88 PH-1）
-- 创建时间: 2026-07-03
-- 关联修复: 占位符 PH-1 — DTO 有 notes 字段但 service 层 `let _ = v;` 丢弃
--
-- 向 custom_orders 表添加 notes 列（TEXT，可选），存储订单备注。
-- 使用 ADD COLUMN IF NOT EXISTS 防止迁移重入。
--
-- 顺序保护：custom_orders 表由 production_quality 域的 m0044 创建，
-- 本迁移位于 sales_crm 域，执行早于 m0044。表尚未创建时用 information_schema
-- 检查跳过，避免 "relation custom_orders does not exist" 中断整个迁移链。
-- notes 列在 m0044 的 CREATE TABLE 中已声明，表创建后即具备该列，本迁移为幂等兜底。

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'custom_orders') THEN
        ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "notes" TEXT;
        COMMENT ON COLUMN "custom_orders"."notes" IS '订单备注（批次 88 PH-1 占位符实现）';
    END IF;
END $$;"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql =
            r#"-- custom_orders 备注列回滚（批次 88 PH-1）
ALTER TABLE "custom_orders" DROP COLUMN IF EXISTS "notes";"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
