//! 多态单据 ID 列统一加宽：把这批“指向多种来源单据”的松散引用列由 INTEGER 放宽为 BIGINT，
//! 与对应模型字段（`i64`/`Option<i64>`）类型一致，消除“按 i64 绑定却落入 int4 列”的运行期
//! 二进制格式错（incorrect binary data format）。
//!
//! 覆盖列（模型均为 i64，故列必须同为 BIGINT）：
//! - `ar_invoices.source_bill_id`
//! - `inventory_transactions.source_bill_id`
//! - `vouchers.source_bill_id`
//! - `period_adjustment_record.source_bill_id`
//! - `assist_accounting_record.business_id`
//! - `bpm_process_instance.business_id`
//! - `bpm_task.business_id`
//! - `notifications.business_id`
//!
//! 上述加宽此前定义在各建表域内部、按“顶层 domain”粒度记账，复用库上该域已记入
//! `seaql_migrations`、`Migrator::up` 视其无 pending 整体跳过，导致列停留在 INTEGER。
//! 本迁移作为链尾新顶层 domain 注册（新名 → 复用库上属 pending → 必然执行），统一收敛到位。
//!
//! 性质：INTEGER 值域是 BIGINT 真子集，放大无损；`ALTER COLUMN ... TYPE BIGINT` 幂等可重放，
//! 新库与复用库均可跑；“窄值写宽列”由 Postgres 兼容，加宽不依赖具体绑定宽度。down 收窄前逐列
//! 做 INTEGER 上下界溢出探测，任一行超限即 RAISE 中止，绝不靠 `::integer` 静默截断丢数据。

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_polymorphic_bill_id_widen_ar_invoice_txn"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 八列一起放宽到 BIGINT，单事务原子：要么全部加宽、要么全部回滚，不留半态。
        let conn = manager.get_connection();
        conn.execute_unprepared(
            r#"
ALTER TABLE "ar_invoices" ALTER COLUMN "source_bill_id" TYPE BIGINT;
ALTER TABLE "inventory_transactions" ALTER COLUMN "source_bill_id" TYPE BIGINT;
ALTER TABLE "vouchers" ALTER COLUMN "source_bill_id" TYPE BIGINT;
ALTER TABLE "period_adjustment_record" ALTER COLUMN "source_bill_id" TYPE BIGINT;
ALTER TABLE "assist_accounting_record" ALTER COLUMN "business_id" TYPE BIGINT;
ALTER TABLE "bpm_process_instance" ALTER COLUMN "business_id" TYPE BIGINT;
ALTER TABLE "bpm_task" ALTER COLUMN "business_id" TYPE BIGINT;
ALTER TABLE "notifications" ALTER COLUMN "business_id" TYPE BIGINT;
"#,
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 回退前逐列溢出探测：任一行取值超出 INTEGER 范围即 RAISE 中止（该 down 仅 CLI 可达）。
        let conn = manager.get_connection();
        conn.execute_unprepared(
            r#"
DO $$
DECLARE
    v_overflow BIGINT;
BEGIN
    SELECT COUNT(*) INTO v_overflow FROM "ar_invoices"
        WHERE "source_bill_id" > 2147483647 OR "source_bill_id" < -2147483648;
    IF v_overflow > 0 THEN
        RAISE EXCEPTION '拒绝回滚：ar_invoices.source_bill_id 有 % 行取值超出 INTEGER 范围', v_overflow;
    END IF;
    SELECT COUNT(*) INTO v_overflow FROM "inventory_transactions"
        WHERE "source_bill_id" > 2147483647 OR "source_bill_id" < -2147483648;
    IF v_overflow > 0 THEN
        RAISE EXCEPTION '拒绝回滚：inventory_transactions.source_bill_id 有 % 行取值超出 INTEGER 范围', v_overflow;
    END IF;
    SELECT COUNT(*) INTO v_overflow FROM "vouchers"
        WHERE "source_bill_id" > 2147483647 OR "source_bill_id" < -2147483648;
    IF v_overflow > 0 THEN
        RAISE EXCEPTION '拒绝回滚：vouchers.source_bill_id 有 % 行取值超出 INTEGER 范围', v_overflow;
    END IF;
    SELECT COUNT(*) INTO v_overflow FROM "period_adjustment_record"
        WHERE "source_bill_id" > 2147483647 OR "source_bill_id" < -2147483648;
    IF v_overflow > 0 THEN
        RAISE EXCEPTION '拒绝回滚：period_adjustment_record.source_bill_id 有 % 行取值超出 INTEGER 范围', v_overflow;
    END IF;
    SELECT COUNT(*) INTO v_overflow FROM "assist_accounting_record"
        WHERE "business_id" > 2147483647 OR "business_id" < -2147483648;
    IF v_overflow > 0 THEN
        RAISE EXCEPTION '拒绝回滚：assist_accounting_record.business_id 有 % 行取值超出 INTEGER 范围', v_overflow;
    END IF;
    SELECT COUNT(*) INTO v_overflow FROM "bpm_process_instance"
        WHERE "business_id" > 2147483647 OR "business_id" < -2147483648;
    IF v_overflow > 0 THEN
        RAISE EXCEPTION '拒绝回滚：bpm_process_instance.business_id 有 % 行取值超出 INTEGER 范围', v_overflow;
    END IF;
    SELECT COUNT(*) INTO v_overflow FROM "bpm_task"
        WHERE "business_id" > 2147483647 OR "business_id" < -2147483648;
    IF v_overflow > 0 THEN
        RAISE EXCEPTION '拒绝回滚：bpm_task.business_id 有 % 行取值超出 INTEGER 范围', v_overflow;
    END IF;
    SELECT COUNT(*) INTO v_overflow FROM "notifications"
        WHERE "business_id" > 2147483647 OR "business_id" < -2147483648;
    IF v_overflow > 0 THEN
        RAISE EXCEPTION '拒绝回滚：notifications.business_id 有 % 行取值超出 INTEGER 范围', v_overflow;
    END IF;
END;
$$;
"#,
        )
        .await?;

        conn.execute_unprepared(
            r#"
ALTER TABLE "notifications" ALTER COLUMN "business_id" TYPE INTEGER USING "business_id"::integer;
ALTER TABLE "bpm_task" ALTER COLUMN "business_id" TYPE INTEGER USING "business_id"::integer;
ALTER TABLE "bpm_process_instance" ALTER COLUMN "business_id" TYPE INTEGER USING "business_id"::integer;
ALTER TABLE "assist_accounting_record" ALTER COLUMN "business_id" TYPE INTEGER USING "business_id"::integer;
ALTER TABLE "period_adjustment_record" ALTER COLUMN "source_bill_id" TYPE INTEGER USING "source_bill_id"::integer;
ALTER TABLE "vouchers" ALTER COLUMN "source_bill_id" TYPE INTEGER USING "source_bill_id"::integer;
ALTER TABLE "inventory_transactions" ALTER COLUMN "source_bill_id" TYPE INTEGER USING "source_bill_id"::integer;
ALTER TABLE "ar_invoices" ALTER COLUMN "source_bill_id" TYPE INTEGER USING "source_bill_id"::integer;
"#,
        )
        .await?;
        Ok(())
    }
}
