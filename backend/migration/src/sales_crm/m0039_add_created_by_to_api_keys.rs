//! api_keys 表 created_by 列迁移（批次 112 P1-9）
//!
//! 创建时间: 2026-07-05
//! 关联修复: v7 复审 P1-9 — api_keys 表无 created_by 列，handler 传 0 占位
//!
//! 向 api_keys 表添加 created_by 列（INTEGER，可空），存储 API 密钥创建者用户 ID。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql =
            r#"-- 批次 112 P1-9：api_keys 表添加 created_by 列
-- 原 api_keys 表无 created_by 列，list/get 历史密钥无法回溯创建者，handler 传 0 占位。
-- 现新增 created_by 列（可空，兼容历史数据），由 create_api_key / regenerate_api_key 注入真实 user_id。

ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "created_by" INTEGER;

COMMENT ON COLUMN "api_keys"."created_by" IS 'API 密钥创建者用户 ID（批次 112 P1-9 修复：原表无此列，handler 传 0 占位）';

-- 创建外键索引便于按创建者查询
CREATE INDEX IF NOT EXISTS "idx_api_keys_created_by" ON "api_keys" ("created_by");"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql =
            r#"-- 批次 112 P1-9：回滚 api_keys 表的 created_by 列

DROP INDEX IF EXISTS "idx_api_keys_created_by";
ALTER TABLE "api_keys" DROP COLUMN IF EXISTS "created_by";"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
