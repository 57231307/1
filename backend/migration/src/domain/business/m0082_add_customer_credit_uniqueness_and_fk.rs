//! `customer_credit_ratings.customer_id` 补引用与单行唯一约束（FK + 全表 UNIQUE）
//!
//! ## 六要素
//! - **功能**：把「每客户当前一条信用评级」钉进数据库：
//!   ① 唯一索引 `uq_customer_credit_ratings_customer`（`customer_id` 全表唯一）；
//!   ② 外键 `fk_customer_credit_ratings_customer` → `customers(id)`（不带
//!   ON DELETE 子句 = 默认 NO ACTION，与仓内既有 FK 如
//!   `fk_sales_prices_customer` 同形）。
//! - **调用方**：SeaORM migrator 链，注册在 `domain/business` 域 up 链尾、down 链首；
//!   本表由本域 m0012 建（m0012_add_ap_ar_finance_analysis.rs:615），参照表
//!   `customers` 由 system 域 m0001 建（system/m0001_initial_schema.rs:324，
//!   `"id" SERIAL PRIMARY KEY`），system 域先于 business 域执行，两者均早于本迁移。
//! - **入参**：无（纯约束构建，不读取应用之外的数据）。
//! - **传给谁**：唯一写入路径 `services/customer_credit_limit.rs::set_credit_rating`
//!   ——先按 `customer_id` 查（`.one()`），存在则更新、不存在才插入（upsert 单行）；
//!   占用/释放/调整/停用路径（occupy/release/adjust/deactivate）全部按
//!   `customer_id` `.one()` 取单行，停用仅把 status 置 inactive 且**保留原行**，
//!   后续 set 复用该行。评估链路 `customer_credit_evaluate.rs` 只计算不落库。
//!   即读写契约 = 每客户恒定一行，与全表 UNIQUE 一致。
//! - **存什么**：pg_constraint 中的 FK 定义与 pg_class 中的唯一索引；
//!   **不改写任何数据行**。
//! - **存哪里**：PostgreSQL `public.customer_credit_ratings` 表约束/索引。
//!
//! ## 语义查证：为什么是全表 UNIQUE 而不是部分唯一索引/复合唯一
//! 表上虽有 `last_assessment_date`/`next_assessment_date` 列，但写入方从不按期间
//! 或版本追加行（`set_credit_rating` 的 INSERT 分支仅在「该客户无行」时到达，
//! 且这两个日期列在写入路径中从未赋值），不构成评级历史多条语义；若退化成
//! `WHERE status = 'active'` 的部分唯一索引，客户停用后再设新评级即可落进第二条
//! 行，而所有 `.one()` 读路径的既定契约会在第一个停用客户处破功。故落全表 UNIQUE。
//! 列型对齐（本仓有 customer_id INTEGER/BIGINT 漂移前科，逐型实测）：
//! `customer_credit_ratings.customer_id` INTEGER NOT NULL（m0012:617），
//! `customers.id` SERIAL/INTEGER（system m0001:325）；全仓 grep
//! `ALTER COLUMN "customer_id" TYPE` 仅命中 custom_orders 加宽，未触及本表；
//! 实体两侧同为 i32（models/customer_credit.rs:13、models/customer.rs:12），
//! 宽度一致，FK 可建，本迁移不设「暂缓列」。
//!
//! ## 存量安全性：fail-visible 守卫（禁止 UPDATE/DELETE 清洗、禁止静默 DROP）
//! DO 守卫前置探测两类违例存量，任一命中即 RAISE EXCEPTION 点名中止，本迁移
//! 不加约束、不在迁移里造值或删行圆场：
//! - 孤儿引用（`customer_id` 在 `customers` 中不存在）——口径照 price_fk
//!   「补 FK 前先扫孤儿行」范式，样例行 id 点名；
//! - 同客户多行（与单行契约冲突，疑似历史旁路写入，如客户合并转移时目标客户
//!   已有评级行）——点名重复组与行数，处置（哪行为当前值、其余行额度占用
//!   如何并）属业务取舍，本迁移只摊出不对策。
//!
//! ## 幂等与回退
//! up 为「DROP IF EXISTS → 守卫 → ADD CONSTRAINT / CREATE UNIQUE INDEX
//! IF NOT EXISTS」，重跑等价；down 对称移除唯一索引与 FK，不触碰任何数据行，
//! 回滚后回到「仅应用层判重/判存在」的现状。
//!
//! ## 上线前只读摸底（需活库执行；与守卫同源，先摸再跑）
//! ```text
//! -- 1) 孤儿引用摸底（>0 行则本迁移守卫会中止，交业务逐行定性）
//! SELECT ccr.id, ccr.customer_id
//!   FROM customer_credit_ratings ccr
//!  WHERE NOT EXISTS (SELECT 1 FROM customers c WHERE c.id = ccr.customer_id);
//! -- 2) 同客户多行摸底（任一行 n>1 需业务裁定当前值并人工合并）
//! SELECT customer_id, COUNT(*) AS n
//!   FROM customer_credit_ratings
//!  GROUP BY customer_id
//! HAVING COUNT(*) > 1
//!  ORDER BY n DESC;
//! -- 3) 现行约束/索引形态（确认无同名对象残留或第二条 customer_id UNIQUE）
//! SELECT indexname, indexdef FROM pg_indexes
//!  WHERE tablename = 'customer_credit_ratings';
//! SELECT conname, pg_get_constraintdef(oid) FROM pg_constraint
//!  WHERE conrelid = 'customer_credit_ratings'::regclass;
//! ```

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1) 幂等前置：按名删旧对象（上一轮已加或历史遗留同名均可清出干净位），
        //    本段之外的一切存量判定交给 DO 守卫，此处不动任何数据。
        let setup = r#"
ALTER TABLE "customer_credit_ratings" DROP CONSTRAINT IF EXISTS "fk_customer_credit_ratings_customer";
DROP INDEX IF EXISTS "uq_customer_credit_ratings_customer";
"#;
        manager.get_connection().execute_unprepared(setup).await?;

        // 2) fail-visible 存量守卫（孤儿引用 + 同客户多行，严禁 UPDATE/DELETE 猜洗）：
        //    命中即点名中止，本迁移不施加任何约束。
        let guard = r#"
DO $$
DECLARE
    bad_rows  INTEGER;
    dup_groups INTEGER;
    samples   TEXT;
BEGIN
    SELECT COUNT(*) INTO bad_rows FROM "customer_credit_ratings" ccr
      WHERE NOT EXISTS (SELECT 1 FROM "customers" c WHERE c."id" = ccr."customer_id");
    IF bad_rows = 0 THEN
        RAISE NOTICE 'm0082：customer_credit_ratings.customer_id 无孤儿引用行，可安全施加 FK fk_customer_credit_ratings_customer。';
    ELSE
        SELECT string_agg(sample, ', ') INTO samples
          FROM (SELECT ccr."id"::text AS sample
                  FROM "customer_credit_ratings" ccr
                 WHERE NOT EXISTS (SELECT 1 FROM "customers" c WHERE c."id" = ccr."customer_id")
                 LIMIT 10) s;
        RAISE EXCEPTION 'm0082：customer_credit_ratings.customer_id 存在 % 行孤儿引用（指向 customers.id 不存在，样例行 id：%），拒绝添加 FK fk_customer_credit_ratings_customer。', bad_rows, samples
            USING HINT = '本迁移拒绝 UPDATE/DELETE 猜洗数据：孤儿行只能由业务定性（补建被删客户，或把该评级行改挂有效客户/删除），先跑文件头只读摸底 SQL 逐行核实，更正后重跑迁移。';
    END IF;

    SELECT COUNT(*) INTO dup_groups FROM (
        SELECT "customer_id"
          FROM "customer_credit_ratings"
         GROUP BY "customer_id"
        HAVING COUNT(*) > 1
    ) d;
    IF dup_groups = 0 THEN
        RAISE NOTICE 'm0082：customer_credit_ratings.customer_id 无同客户多行组，可安全施加唯一索引 uq_customer_credit_ratings_customer。';
    ELSE
        SELECT string_agg(sample, ', ') INTO samples
          FROM (SELECT "customer_id"::text || '(行数=' || COUNT(*)::text || ')' AS sample
                  FROM "customer_credit_ratings"
                 GROUP BY "customer_id"
                HAVING COUNT(*) > 1
                 LIMIT 10) s;
        RAISE EXCEPTION 'm0082：customer_credit_ratings.customer_id 存在 % 个同客户多行组（明细：%），该表读写契约为每客户当前一行，拒绝施加唯一索引 uq_customer_credit_ratings_customer。', dup_groups, samples
            USING HINT = '本迁移拒绝静默去重/删行洗数据：多行组说明存量与单行语义冲突（疑似旁路写入或客户合并转移所致），哪行是当前值、其余行的额度占用如何合并属业务取舍，请人工逐组裁定合并后重跑迁移。';
    END IF;
END;
$$;
"#;
        manager.get_connection().execute_unprepared(guard).await?;

        // 3) 守卫全部通过后施加约束：FK 引用列 INTEGER 与 customers.id 同宽
        //    （文件头核查），默认 NO ACTION；唯一索引名沿用仓内 uq_<表>_<列> 形态。
        let apply = r#"
ALTER TABLE "customer_credit_ratings" ADD CONSTRAINT "fk_customer_credit_ratings_customer" FOREIGN KEY ("customer_id") REFERENCES "customers" ("id");
CREATE UNIQUE INDEX IF NOT EXISTS "uq_customer_credit_ratings_customer" ON "customer_credit_ratings" ("customer_id");
"#;
        manager.get_connection().execute_unprepared(apply).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 回滚仅对称移除本迁移施加的唯一索引与 FK，不触碰任何数据行
        // （不留空 down；回滚后退回"仅应用层判重"的现状）。
        let sql = r#"
DROP INDEX IF EXISTS "uq_customer_credit_ratings_customer";
ALTER TABLE "customer_credit_ratings" DROP CONSTRAINT IF EXISTS "fk_customer_credit_ratings_customer";
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
