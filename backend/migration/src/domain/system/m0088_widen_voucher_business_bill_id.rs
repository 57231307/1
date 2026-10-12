//! 凭证与辅助核算的多态来源单据 ID 列拓宽为 BIGINT
//!
//! 功能：把 `vouchers.source_bill_id` 与 `assist_accounting_record.business_id`
//! 两列由 INTEGER 改为 BIGINT，使凭证能无损承载被引用单据的主键。
//!
//! 根因（引用面全部为仓内实测）：这两列是多态松散引用——无外键，靠
//! `source_type`/`source_module` 判别被引用表，建表见
//! `migration/src/domain/system/m0006_add_general_ledger_and_finance_base.rs`。
//! 写入方来自多张单据表，其中 `period_adjustment_record` 的主键是 BIGSERIAL
//! （建表见 `migration/src/domain/v15/mod.rs`），期末调整确认时把该主键写进
//! `vouchers.source_bill_id`（调用方
//! `src/services/period_adjustment_service.rs` 的凭证生成路径）。INTEGER 列
//! 装不下 BIGINT 值，原实现只能在写入前做窄化检查、溢出时降级为"仅保留单号"，
//! 数值关联随之丢失。
//!
//! 为什么两列一起改：辅助核算的 `business_id` 与凭证的 `source_bill_id` 同源同值
//! （凭证过账时由 `src/services/voucher_ops/assist.rs` 从凭证行取该值再落库），
//! 只拓宽凭证列会让 BIGINT 值原样传到仍是 INTEGER 的 business_id 列，把同一个
//! 类型落差下移一层。
//!
//! 无损性：INTEGER 值域是 BIGINT 的真子集，存量行按类型放大重存，不改变任何取值；
//! 这两列无外键、无 CHECK、无表达式依赖（全仓引用面已核对），
//! `idx_assist_accounting_record_business` 索引由数据库在改类型时自动重建。
//!
//! 幂等：`ALTER COLUMN ... TYPE` 重放结果等价，`COMMENT ON` 覆盖同文 ⇒ up 可重跑。
//!
//! down：恢复 INTEGER。回滚前 fail-visible 探测——任一行超出 INTEGER 上界即
//! RAISE EXCEPTION 点名样例，绝不靠 `::integer` 静默截断丢数据。
//!
//! 注册位置：`migration/src/domain/system/mod.rs` 域 up 链末尾（晚于建表所在迁移）、
//! down 链首位。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
-- 1) 凭证来源单据 ID 拓宽：BIGSERIAL 主键的单据（期末调整记录等）不再被窄化。
ALTER TABLE "vouchers" ALTER COLUMN "source_bill_id" TYPE BIGINT;

COMMENT ON COLUMN "vouchers"."source_bill_id"
    IS '来源单据 ID（多态松散引用，无外键；宽度取 BIGINT 以容纳 BIGSERIAL 主键的被引用单据）';

-- 2) 辅助核算业务 ID 配套拓宽：凭证过账时该列直接取 vouchers.source_bill_id。
ALTER TABLE "assist_accounting_record" ALTER COLUMN "business_id" TYPE BIGINT;

COMMENT ON COLUMN "assist_accounting_record"."business_id"
    IS '业务单据 ID（多态松散引用，与 vouchers.source_bill_id 同源同值；宽度取 BIGINT）';
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
-- 拒滚探测：任一列存在超出 INTEGER 上界的取值即中止，绝不静默截断。
DO $$
DECLARE
    voucher_overflow BIGINT;
    assist_overflow BIGINT;
BEGIN
    SELECT COUNT(*) INTO voucher_overflow
      FROM "vouchers"
     WHERE "source_bill_id" > 2147483647;
    SELECT COUNT(*) INTO assist_overflow
      FROM "assist_accounting_record"
     WHERE "business_id" > 2147483647;
    IF voucher_overflow > 0 OR assist_overflow > 0 THEN
        RAISE EXCEPTION
            '拒绝回滚：vouchers.source_bill_id 有 % 行、assist_accounting_record.business_id 有 % 行取值超出 INTEGER 上界，收窄会丢数据（样例凭证号：%）',
            voucher_overflow, assist_overflow,
            (SELECT string_agg("voucher_no", ', ' ORDER BY "id")
               FROM (SELECT "voucher_no", "id" FROM "vouchers"
                      WHERE "source_bill_id" > 2147483647
                      ORDER BY "id" LIMIT 5) s);
    END IF;
END
$$;

ALTER TABLE "assist_accounting_record"
    ALTER COLUMN "business_id" TYPE INTEGER USING "business_id"::integer;
ALTER TABLE "vouchers"
    ALTER COLUMN "source_bill_id" TYPE INTEGER USING "source_bill_id"::integer;

COMMENT ON COLUMN "assist_accounting_record"."business_id" IS '业务单据 ID';
COMMENT ON COLUMN "vouchers"."source_bill_id" IS '来源单据 ID';
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
