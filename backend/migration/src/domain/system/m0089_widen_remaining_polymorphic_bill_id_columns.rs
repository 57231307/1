//! 其余多态单据 ID 列统一拓宽为 BIGINT
//!
//! 功能：把 6 个"按类型判别被引用表"的松散引用列由 INTEGER 改为 BIGINT——
//! `period_adjustment_record.source_bill_id`、`inventory_transactions.source_bill_id`、
//! `ar_invoices.source_bill_id`、`bpm_process_instance.business_id`、
//! `bpm_task.business_id`、`notifications.business_id`。
//!
//! 调用方：迁移链。注册位置在 `migration/src/domain/v15/mod.rs` 的 up 链末尾、
//! down 链首位——6 张目标表分散在 system / business / v15 三个域，只有走完 v15
//! 建表后它们才全部在场（照本仓 m0058/m0063/m0068/m0075 "文件留归属域、执行后置
//! 到最晚建表域"的先例）。模块声明在 `migration/src/domain/system/mod.rs`。
//!
//! 入参：无。传给谁：`SchemaManager` 的裸 SQL 连接。存什么/存哪里：只改这 6 列的
//! 类型与列注释，不动任何数据行取值。
//!
//! 为什么与凭证列同批：凭证侧 `vouchers.source_bill_id` 与辅助核算
//! `assist_accounting_record.business_id` 已按 BIGINT 落地；期末调整记录、库存流水、
//! 应收发票这几种单据自身也带同名的来源单据列，BPM 流程实例/任务与站内通知的
//! `business_id` 则按 `business_type` 指向多种单据表。只要其中任何一列仍是 INTEGER，
//! 同一个 BIGSERIAL 主键流到该列时就会撞类型落差，被迫重复凭证侧已经删掉的
//! "窄化失败即丢数值关联"降级。本批把剩余同源列一次收敛，禁止两口径并存。
//!
//! 无损性：INTEGER 值域是 BIGINT 的真子集，存量行按类型放大重存，取值不变；
//! 6 列均无 NOT NULL、无外键、无 CHECK、无表达式依赖、无索引引用（逐表建表语句
//! 与全目录索引语句核对过），改类型不触发约束重建或数据校验失败。
//!
//! 幂等：`ALTER COLUMN ... TYPE` 重放等价，`COMMENT ON` 覆盖同文 ⇒ up 可重跑。
//!
//! down：逐列恢复 INTEGER。回滚前先对 6 列各做一次溢出探测，任一行超出 INTEGER
//! 上界即 RAISE EXCEPTION 点名"哪一列、多少行"，绝不靠 `::integer` 静默截断。
//! 消息只含表列名与行数，不含记录 ID；该路径仅 CLI 可达。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
-- 1) 期末调整记录的来源单据 ID：该表主键本身是 BIGSERIAL，其来源列须能容纳同类主键。
ALTER TABLE "period_adjustment_record" ALTER COLUMN "source_bill_id" TYPE BIGINT;

COMMENT ON COLUMN "period_adjustment_record"."source_bill_id"
    IS '来源单据 ID（多态松散引用，无外键；宽度取 BIGINT 以容纳 BIGSERIAL 主键的被引用单据）';

-- 2) 库存流水的来源单据 ID：出入库流水可由多张单据触发。
ALTER TABLE "inventory_transactions" ALTER COLUMN "source_bill_id" TYPE BIGINT;

COMMENT ON COLUMN "inventory_transactions"."source_bill_id"
    IS '来源单据 ID（多态松散引用，随触发单据记录；宽度取 BIGINT 以容纳 BIGSERIAL 主键的被引用单据）';

-- 3) 应收发票的来源单据 ID：与销售发货/手工开票同源，无外键。
ALTER TABLE "ar_invoices" ALTER COLUMN "source_bill_id" TYPE BIGINT;

COMMENT ON COLUMN "ar_invoices"."source_bill_id"
    IS '来源单据 ID（多态松散引用，无外键；宽度取 BIGINT 以容纳 BIGSERIAL 主键的被引用单据）';

-- 4) BPM 流程实例与任务的业务单据 ID：按 business_type 判别被引用表，二者须同宽。
ALTER TABLE "bpm_process_instance" ALTER COLUMN "business_id" TYPE BIGINT;

COMMENT ON COLUMN "bpm_process_instance"."business_id"
    IS '业务单据 ID（按 business_type 判别被引用表的多态松散引用；宽度取 BIGINT）';

ALTER TABLE "bpm_task" ALTER COLUMN "business_id" TYPE BIGINT;

COMMENT ON COLUMN "bpm_task"."business_id"
    IS '业务单据 ID（与 bpm_process_instance.business_id 同源同值，任务须能指向同一张单据；宽度取 BIGINT）';

-- 5) 站内通知的业务单据 ID：跳转依据同为多态松散引用。
ALTER TABLE "notifications" ALTER COLUMN "business_id" TYPE BIGINT;

COMMENT ON COLUMN "notifications"."business_id"
    IS '业务单据 ID（按 business_type 判别被引用表的多态松散引用；宽度取 BIGINT）';
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 拒滚探测：任一列存在超出 INTEGER 上界的取值即中止，绝不静默截断。
        let probe = r#"
DO $$
DECLARE
    v_overflow BIGINT;
BEGIN
    SELECT COUNT(*) INTO v_overflow FROM "period_adjustment_record"
        WHERE "source_bill_id" > 2147483647 OR "source_bill_id" < -2147483648;
    IF v_overflow > 0 THEN
        RAISE EXCEPTION '拒绝回滚：period_adjustment_record.source_bill_id 有 % 行取值超出 INTEGER 上界，收窄会丢数据', v_overflow;
    END IF;

    SELECT COUNT(*) INTO v_overflow FROM "inventory_transactions"
        WHERE "source_bill_id" > 2147483647 OR "source_bill_id" < -2147483648;
    IF v_overflow > 0 THEN
        RAISE EXCEPTION '拒绝回滚：inventory_transactions.source_bill_id 有 % 行取值超出 INTEGER 上界，收窄会丢数据', v_overflow;
    END IF;

    SELECT COUNT(*) INTO v_overflow FROM "ar_invoices"
        WHERE "source_bill_id" > 2147483647 OR "source_bill_id" < -2147483648;
    IF v_overflow > 0 THEN
        RAISE EXCEPTION '拒绝回滚：ar_invoices.source_bill_id 有 % 行取值超出 INTEGER 上界，收窄会丢数据', v_overflow;
    END IF;

    SELECT COUNT(*) INTO v_overflow FROM "bpm_process_instance"
        WHERE "business_id" > 2147483647 OR "business_id" < -2147483648;
    IF v_overflow > 0 THEN
        RAISE EXCEPTION '拒绝回滚：bpm_process_instance.business_id 有 % 行取值超出 INTEGER 上界，收窄会丢数据', v_overflow;
    END IF;

    SELECT COUNT(*) INTO v_overflow FROM "bpm_task"
        WHERE "business_id" > 2147483647 OR "business_id" < -2147483648;
    IF v_overflow > 0 THEN
        RAISE EXCEPTION '拒绝回滚：bpm_task.business_id 有 % 行取值超出 INTEGER 上界，收窄会丢数据', v_overflow;
    END IF;

    SELECT COUNT(*) INTO v_overflow FROM "notifications"
        WHERE "business_id" > 2147483647 OR "business_id" < -2147483648;
    IF v_overflow > 0 THEN
        RAISE EXCEPTION '拒绝回滚：notifications.business_id 有 % 行取值超出 INTEGER 上界，收窄会丢数据', v_overflow;
    END IF;
END;
$$;
"#;
        manager.get_connection().execute_unprepared(probe).await?;

        let restore = r#"
ALTER TABLE "notifications" ALTER COLUMN "business_id" TYPE INTEGER USING "business_id"::integer;
ALTER TABLE "bpm_task" ALTER COLUMN "business_id" TYPE INTEGER USING "business_id"::integer;
ALTER TABLE "bpm_process_instance" ALTER COLUMN "business_id" TYPE INTEGER USING "business_id"::integer;
ALTER TABLE "ar_invoices" ALTER COLUMN "source_bill_id" TYPE INTEGER USING "source_bill_id"::integer;
ALTER TABLE "inventory_transactions" ALTER COLUMN "source_bill_id" TYPE INTEGER USING "source_bill_id"::integer;
ALTER TABLE "period_adjustment_record" ALTER COLUMN "source_bill_id" TYPE INTEGER USING "source_bill_id"::integer;
"#;
        manager.get_connection().execute_unprepared(restore).await?;
        Ok(())
    }
}
