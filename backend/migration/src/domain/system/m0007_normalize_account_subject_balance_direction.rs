//! account_subjects.balance_direction 存量归一（中文 → 权威英文词表）
//!
//! 背景：该列的写入方权威词表为英文 debit/credit（m0006 DDL 默认值、迁移种子 28 行、
//! 前端两处科目 Tab 提交值三端同源），但历史 e2e 直灌/legacy 快照混入中文「借/贷」，
//! 旧比较点只认中文导致英文行被反向分支吞掉、科目余额与试算平衡系统性反号。
//! 本迁移把中文行归一为英文：借→debit、贷→credit。
//!
//! 设计：
//! - 精确可回退：up 先把将被改写的行的原值备份到专用小表，down 按备份逐行还原并删除备份表。
//!   不用「英文全量反向映射」做 down——那会把本来就合法的 debit/credit 种子/生产行腐蚀成中文，
//!   恰好重新引入本次要修的缺陷。
//! - 幂等（三条语句均可重放）：备份表 CREATE IF NOT EXISTS；备份 INSERT 带 NOT EXISTS 守卫；
//!   归一 UPDATE 的 WHERE 只命中中文 token，第二次执行起匹配 0 行、零影响。
//! - 双方言：仅用 CREATE TABLE / INSERT...SELECT / 关联子查询 UPDATE / DROP 标准语法，
//!   PG 与 sqlite 均可执行（不用 UPDATE...FROM、DO $$ 等 PG 专有形态）。
//! - 本批不做（用户拍板项）：CHECK 约束、NULL 回填、历史 account_balance/ending_* 重算。

use sea_orm_migration::prelude::*;

/// 备份表建表（up 第一步；IF NOT EXISTS 保证重放安全）
pub const CREATE_BACKUP_TABLE_SQL: &str = r#"CREATE TABLE IF NOT EXISTS "mig198_account_subject_direction_backup" (
    "subject_id" INTEGER PRIMARY KEY,
    "old_balance_direction" VARCHAR(20) NOT NULL
)"#;

/// 备份将被归一的行原值（up 第二步；NOT EXISTS 守卫保证幂等，不重复插入）
pub const BACKUP_LEGACY_ROWS_SQL: &str = r#"INSERT INTO "mig198_account_subject_direction_backup" ("subject_id", "old_balance_direction")
SELECT s."id", s."balance_direction"
FROM "account_subjects" s
WHERE s."balance_direction" IN ('借', '贷')
  AND NOT EXISTS (
      SELECT 1 FROM "mig198_account_subject_direction_backup" b
      WHERE b."subject_id" = s."id"
  )"#;

/// 存量归一：借→debit、贷→credit（up 第三步；WHERE 只命中中文，重放时 0 行受影响）
pub const NORMALIZE_TO_ENGLISH_SQL: &str = r#"UPDATE "account_subjects"
SET "balance_direction" = CASE "balance_direction"
    WHEN '借' THEN 'debit'
    WHEN '贷' THEN 'credit'
    ELSE "balance_direction"
END
WHERE "balance_direction" IN ('借', '贷')"#;

/// 回滚：按备份表逐行精确还原（down 第一步；关联子查询形态，PG/sqlite 通用）
pub const RESTORE_FROM_BACKUP_SQL: &str = r#"UPDATE "account_subjects"
SET "balance_direction" = (
    SELECT b."old_balance_direction"
    FROM "mig198_account_subject_direction_backup" b
    WHERE b."subject_id" = "account_subjects"."id"
)
WHERE "id" IN (SELECT "subject_id" FROM "mig198_account_subject_direction_backup")"#;

/// 回滚：删除备份表（down 第二步）
pub const DROP_BACKUP_TABLE_SQL: &str =
    r#"DROP TABLE IF EXISTS "mig198_account_subject_direction_backup""#;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(CREATE_BACKUP_TABLE_SQL).await?;
        conn.execute_unprepared(BACKUP_LEGACY_ROWS_SQL).await?;
        conn.execute_unprepared(NORMALIZE_TO_ENGLISH_SQL).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(RESTORE_FROM_BACKUP_SQL).await?;
        conn.execute_unprepared(DROP_BACKUP_TABLE_SQL).await?;
        Ok(())
    }
}
