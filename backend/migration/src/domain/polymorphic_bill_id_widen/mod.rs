//! 多态单据 ID 列加宽：把 `ar_invoices.source_bill_id` 与 `inventory_transactions.source_bill_id`
//! 由 INTEGER 放宽为 BIGINT，与模型字段 `source_bill_id: Option<i64>` 类型一致，使销售退货审批路径
//! （红字应收幂等查询/写入、退货入库流水）按 `i64` 绑定这两列时列宽匹配。
//!
//! 作为链尾新顶层 domain 注册（复用库上属 pending、必然执行）。INTEGER 值域是 BIGINT 真子集，放大无损；
//! `ALTER COLUMN ... TYPE BIGINT` 幂等可重放，新库与复用库均可跑；两列均无 NOT NULL/外键/CHECK/索引依赖，
//! 改类型不触发约束重建。down 收窄前先做 INTEGER 上界溢出探测，超限即中止，绝不静默截断。

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
        // 放宽两列到 BIGINT，与模型 `source_bill_id: Option<i64>` 对齐（int8）。
        // 单条 execute_unprepared 内两条语句同事务原子：要么两列一起加宽，要么一起回滚，
        // 不留“一列 int8 一列 int4”的半态。
        let conn = manager.get_connection();
        conn.execute_unprepared(
            r#"
ALTER TABLE "ar_invoices" ALTER COLUMN "source_bill_id" TYPE BIGINT;
ALTER TABLE "inventory_transactions" ALTER COLUMN "source_bill_id" TYPE BIGINT;
"#,
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 回退前先逐列溢出探测：任一行取值超出 INTEGER 上界即 RAISE 中止，绝不靠
        // `::integer` 静默截断丢数值关联（该 down 仅 CLI 可达）。
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
        RAISE EXCEPTION '拒绝回滚：ar_invoices.source_bill_id 有 % 行取值超出 INTEGER 上界，收窄会丢数据', v_overflow;
    END IF;

    SELECT COUNT(*) INTO v_overflow FROM "inventory_transactions"
        WHERE "source_bill_id" > 2147483647 OR "source_bill_id" < -2147483648;
    IF v_overflow > 0 THEN
        RAISE EXCEPTION '拒绝回滚：inventory_transactions.source_bill_id 有 % 行取值超出 INTEGER 上界，收窄会丢数据', v_overflow;
    END IF;
END;
$$;
"#,
        )
        .await?;

        conn.execute_unprepared(
            r#"
ALTER TABLE "inventory_transactions" ALTER COLUMN "source_bill_id" TYPE INTEGER USING "source_bill_id"::integer;
ALTER TABLE "ar_invoices" ALTER COLUMN "source_bill_id" TYPE INTEGER USING "source_bill_id"::integer;
"#,
        )
        .await?;
        Ok(())
    }
}
