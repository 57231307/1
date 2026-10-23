//! 客户信用评级备注兑现为真列（customer_credit_ratings.remark）。
//!
//! ## 六要素
//! - 功能：给 customer_credit_ratings 增加可空列 remark VARCHAR(500)，把请求 DTO 已在校验、
//!   前端表单已存在 的备注字段真正落进库。此前该字段 DTO 收、UI 有、库里没有这一列，用户写的
//!   备注无处落库。列可空、无默认值、不回填：NULL 的语义是该评级行没有备注，不是备注是空串，
//!   读取方不得把 NULL 读成任何占位文案。
//! - 调用方：SeaORM migrator 链，注册在 domain/customer_credit_remark_column 独立域、
//!   migration/src/lib.rs 链尾新顶层条目（排在 customer_credit_name_backfill 之后）。
//! - 入参：无（纯 DDL，不读取应用数据、不回填任何存量行）。
//! - 传给谁：写入口由 services/customer_credit_limit 的 set_credit_rating 的创建分支直接写入、
//!   更新分支遵守未提交（None）就不碰该列；长度上限 500 与请求 DTO 的字段校验值一致（双侧同源，
//!   DB 侧是最后防线）。
//! - 存什么：新列本体与列注释；information_schema 里该列的存在性即本域的完成标志。
//! - 存哪里：PostgreSQL public.customer_credit_ratings.remark。
//!
//! ## 为什么必须新增顶层域
//! 建列写进已应用域（如 business）的 up，在复用库上会因该域台账已记账而被整体跳过，列永远不存在；
//! 而 ADD COLUMN IF NOT EXISTS 打在已存在的列上是静默 no-op，连报错都不报。链尾新域名在复用库上
//! 属 pending，必然执行（与 fulfillment_source_and_po_origin 域的注册理由同型）。
//!
//! ## 语义与幂等
//! - up 先查 information_schema 再决定加列，两条路径都打印：新增成功 与 列已存在（幂等重放），
//!   并打印新列上非 NULL 的行数（恒为 0，用来证明本域没有替存量造值）。
//! - 整支可重放：重跑走列已存在分支，不再执行 DDL。
//!
//! ## 回退
//! down 删除本列（DROP COLUMN IF EXISTS）并打印删除前后该列上非 NULL 的行数：备注是用户自录入的
//! 显示性数据，删列即丢弃这些值，这是回退掉一次加列的固有后果，因此在删除前把受影响行数点名，不
//! 静默吞掉。列之外的任何东西（评级、额度、账期）都不触碰。

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_customer_credit_remark_column"
    }
}

/// 加列（先查 information_schema 判定是否已存在，两条路径都打印，不静默 no-op）
const ADD_REMARK_COLUMN_SQL: &str = r#"
DO $$
DECLARE
    existed_before INTEGER;
    filled_rows    INTEGER;
BEGIN
    SELECT COUNT(*) INTO existed_before
      FROM information_schema.columns
     WHERE table_schema = current_schema()
       AND table_name = 'customer_credit_ratings'
       AND column_name = 'remark';

    IF existed_before = 0 THEN
        ALTER TABLE "customer_credit_ratings" ADD COLUMN "remark" VARCHAR(500) NULL;
        RAISE NOTICE 'm_customer_credit_remark_column：已新增可空列 customer_credit_ratings.remark（VARCHAR(500)）。';
    ELSE
        RAISE NOTICE 'm_customer_credit_remark_column：列 remark 已存在，跳过 DDL（幂等重放路径）。';
    END IF;

    SELECT COUNT(*) INTO filled_rows FROM "customer_credit_ratings" WHERE "remark" IS NOT NULL;
    RAISE NOTICE 'm_customer_credit_remark_column：remark 非 NULL 行数 = %（本域不回填任何存量值）。', filled_rows;
END;
$$;
"#;

/// 列注释固化宽度与写入方口径（DB 侧可自证，不依赖仓外文档）
const COMMENT_COLUMN_SQL: &str = r#"
COMMENT ON COLUMN "customer_credit_ratings"."remark" IS '客户信用评级备注（可空，最长 500 字符，与请求 DTO 的字段校验同宽）：NULL 表示未填写。写入方为 services/customer_credit_limit 的 set_credit_rating 创建与更新两条路径；更新时未提交该字段则保持原值不变';
"#;

/// 回退：删列前点名该列上还有多少非 NULL 备注会被一并丢弃
const DROP_REMARK_COLUMN_SQL: &str = r#"
DO $$
DECLARE
    has_col      INTEGER;
    dropped_rows INTEGER;
BEGIN
    IF to_regclass('customer_credit_ratings') IS NULL THEN
        RAISE NOTICE 'm_customer_credit_remark_column down：本库还没有 customer_credit_ratings 表，无需删列。';
        RETURN;
    END IF;

    SELECT COUNT(*) INTO has_col
      FROM information_schema.columns
     WHERE table_schema = current_schema()
       AND table_name = 'customer_credit_ratings'
       AND column_name = 'remark';

    IF has_col > 0 THEN
        SELECT COUNT(*) INTO dropped_rows FROM "customer_credit_ratings" WHERE "remark" IS NOT NULL;
        ALTER TABLE "customer_credit_ratings" DROP COLUMN "remark";
        RAISE NOTICE 'm_customer_credit_remark_column down：已删除列 remark，随列一并丢弃的非 NULL 备注 % 行。', dropped_rows;
    ELSE
        RAISE NOTICE 'm_customer_credit_remark_column down：列 remark 不在本库上，无需删除。';
    END IF;
END;
$$;
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(ADD_REMARK_COLUMN_SQL).await?;
        conn.execute_unprepared(COMMENT_COLUMN_SQL).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(DROP_REMARK_COLUMN_SQL).await?;
        Ok(())
    }
}
