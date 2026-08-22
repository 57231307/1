//! users 表 password_changed_at 列迁移（批次 198 P0-2）
//!
//! 创建时间: 2026-07-08
//! 关联修复: v12 复审 P0-2 — PasswordPolicyService::is_expired 未接入登录流程
//!
//! 向 users 表添加 password_changed_at 列（TIMESTAMP WITH TIME ZONE，可空），
//! 作为密码过期策略的时间锚点。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"-- 批次 198 P0-2：users 表添加 password_changed_at 列
-- 原 users 表无 password_changed_at 列，PasswordPolicyService::is_expired 无法持久化密码修改时间锚点。
-- 现新增 password_changed_at 列（可空，兼容历史数据），由 change_password 注入当前时间，
-- 由 login 调用 PasswordPolicyService::is_expired 检查密码是否过期（默认 90 天）。

ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "password_changed_at" TIMESTAMP WITH TIME ZONE;

COMMENT ON COLUMN "users"."password_changed_at" IS '密码最后修改时间（批次 198 P0-2 修复：密码过期策略锚点，None 表示历史用户未设置）';

-- 历史用户初始化为当前时间，避免存量用户登录即被判为过期
UPDATE "users" SET "password_changed_at" = CURRENT_TIMESTAMP WHERE "password_changed_at" IS NULL;"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"-- 批次 198 P0-2 回滚：移除 users.password_changed_at 列
ALTER TABLE "users" DROP COLUMN IF EXISTS "password_changed_at";"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
