//! 客户信用评级存量 customer_name 回填（把冗余列的真名补齐）。
//!
//! ## 六要素
//! - 功能：把 customer_credit_ratings.customer_name 里为 NULL 的存量行，按 customer_id 关联
//!   customers.customer_name 回填成客户真名。该列是冗余显示列（与应收对账、销售合同等表同
//!   范式：建行写入真名加客户改名事件回写），列可空但读出口直接消费本列，存量行大面积 NULL
//!   会让列表与打印单据的客户列空白，而库里根本没有别的名称可读。回填只搬运 customers 里已有
//!   的真名，不拼任何字符串、不造名。
//! - 调用方：SeaORM migrator 链，注册在 domain/customer_credit_name_backfill 独立域、
//!   migration/src/lib.rs 链尾新顶层条目。
//! - 入参：无（读 customers 与 customer_credit_ratings 两表）。
//! - 传给谁：写入口由 services/customer_credit_limit 的 set_credit_rating 建行即写真名；
//!   读出口（信用评级列表、详情、打印数据装配）直接读列，不做 join 富化，与本表既有的冗余列
//!   范式一致。
//! - 存什么：一张只覆盖将被改写的行的备份表（credit_id 加原值），以及回填后的 customer_name；
//!   另在列注释里固化冗余列加改名事件回写的维护口径。
//! - 存哪里：PostgreSQL public.customer_credit_ratings.customer_name 与
//!   public.m_customer_credit_name_backfill_backup（回退用，down 后删除）。
//!
//! ## 为什么必须新增顶层域
//! 回填写进已应用域（如 business）的 up，在复用库上会因该域台账已记账而被整体跳过，存量行永远
//! 停在 NULL；链尾新域名在复用库上属 pending，必然执行（与 fulfillment_source_and_po_origin
//! 域的注册理由同型）。
//!
//! ## 语义与幂等
//! - 备份先于回填：只备份 customer_name IS NULL 的行（与回填同谓词），因此 down 能逐行还原到
//!   回填前恰好是 NULL 的状态，而不会把本来就有真名的行腐蚀成 NULL。
//! - 回填谓词是 customer_name IS NULL：第二次执行起命中 0 行，实写行数如实打印为 0。
//! - 备份表 CREATE TABLE IF NOT EXISTS 加备份 INSERT 带 NOT EXISTS 守卫，整支可重放。
//! - 取不到客户的行（customer_id 在 customers 里不存在）保持 NULL 并点名计数：本表对
//!   customer_id 有外键，正常链路不该出现这种行，一旦出现说明库上有旁路写入或客户被物理删除，
//!   属需要人工裁定的数据问题，绝不在迁移里用 id 拼个名字把它盖掉。
//! - up 与 down 都把实写或回退行数用 RAISE NOTICE 打印，成功失败都不静默。
//! - down 语义：还原到本域 up 之前的快照（把当时为 NULL 的行改回 NULL）；up 之后由业务新写入
//!   的真名不在本域回退契约内，需要保留时勿对本域做 down。

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_customer_credit_name_backfill"
    }
}

/// 备份表：只存将被回填的行 的主键与原值（原值恒为 NULL，记录在案即为可回退凭证）
const CREATE_BACKUP_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS "m_customer_credit_name_backfill_backup" (
    "credit_id" INTEGER PRIMARY KEY,
    "old_customer_name" VARCHAR(200)
);
"#;

/// 回填前备份（NOT EXISTS 守卫保证重放不重复插行）
const BACKUP_NULL_NAME_ROWS_SQL: &str = r#"
INSERT INTO "m_customer_credit_name_backfill_backup" ("credit_id", "old_customer_name")
SELECT ccr."id", ccr."customer_name"
  FROM "customer_credit_ratings" ccr
 WHERE ccr."customer_name" IS NULL
   AND NOT EXISTS (
       SELECT 1 FROM "m_customer_credit_name_backfill_backup" b
       WHERE b."credit_id" = ccr."id"
   );
"#;

/// 存量回填 加 逐段打印实写行数与仍取不到真名的行数
const BACKFILL_AND_REPORT_SQL: &str = r#"
DO $$
DECLARE
    written       INTEGER;
    still_null    INTEGER;
    orphan_rows   INTEGER;
    sample_ids    TEXT;
BEGIN
    UPDATE "customer_credit_ratings" ccr
       SET "customer_name" = c."customer_name"
      FROM "customers" c
     WHERE c."id" = ccr."customer_id"
       AND ccr."customer_name" IS NULL;
    GET DIAGNOSTICS written = ROW_COUNT;
    RAISE NOTICE 'm_customer_credit_name_backfill：customer_name 按 customers 真名回填，实写 % 行。', written;

    SELECT COUNT(*) INTO still_null FROM "customer_credit_ratings" WHERE "customer_name" IS NULL;
    SELECT COUNT(*) INTO orphan_rows FROM "customer_credit_ratings" ccr
      WHERE ccr."customer_name" IS NULL
        AND NOT EXISTS (SELECT 1 FROM "customers" c WHERE c."id" = ccr."customer_id");

    IF still_null = 0 THEN
        RAISE NOTICE 'm_customer_credit_name_backfill：评级行的 customer_name 已全部取到客户真名。';
    ELSE
        SELECT string_agg(sample, ', ') INTO sample_ids
          FROM (SELECT ccr."id"::text AS sample
                  FROM "customer_credit_ratings" ccr
                 WHERE ccr."customer_name" IS NULL
                 LIMIT 10) s;
        RAISE NOTICE 'm_customer_credit_name_backfill：仍有 % 行取不到客户真名（其中 % 行在 customers 里已无对应客户，样例行 id：%）；本迁移拒绝造名，这些行保持 NULL，交业务侧核实是补客户还是清行。', still_null, orphan_rows, sample_ids;
    END IF;
END;
$$;
"#;

/// 列注释固化冗余列的维护口径（建行写真实客户名，改名由事件回写，取不到就留 NULL）
const COMMENT_COLUMN_SQL: &str = r#"
COMMENT ON COLUMN "customer_credit_ratings"."customer_name" IS '客户名称冗余列：建行时写 customers.customer_name 真名，客户改名由改名事件回写；取不到真名保持 NULL（禁止用 customer_id 拼假名或写占位文案）。读取方：信用评级列表、详情、打印数据装配直接消费本列，不做 join 富化';
"#;

/// 回退：按备份逐行还原（备份表不存在时如实说明，不误报回退成功）
const RESTORE_FROM_BACKUP_SQL: &str = r#"
DO $$
DECLARE
    restored INTEGER;
BEGIN
    IF to_regclass('public.m_customer_credit_name_backfill_backup') IS NULL THEN
        RAISE NOTICE 'm_customer_credit_name_backfill down：备份表不在本库上（本域未在此执行过 up），无可回退内容。';
        RETURN;
    END IF;
    UPDATE "customer_credit_ratings" ccr
       SET "customer_name" = b."old_customer_name"
      FROM "m_customer_credit_name_backfill_backup" b
     WHERE b."credit_id" = ccr."id";
    GET DIAGNOSTICS restored = ROW_COUNT;
    RAISE NOTICE 'm_customer_credit_name_backfill down：按备份逐行还原 customer_name，回退 % 行。', restored;
END;
$$;
"#;

const DROP_BACKUP_TABLE_SQL: &str = r#"
DROP TABLE IF EXISTS "m_customer_credit_name_backfill_backup";
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(CREATE_BACKUP_TABLE_SQL).await?;
        conn.execute_unprepared(BACKUP_NULL_NAME_ROWS_SQL).await?;
        conn.execute_unprepared(BACKFILL_AND_REPORT_SQL).await?;
        conn.execute_unprepared(COMMENT_COLUMN_SQL).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(RESTORE_FROM_BACKUP_SQL).await?;
        conn.execute_unprepared(DROP_BACKUP_TABLE_SQL).await?;
        Ok(())
    }
}
