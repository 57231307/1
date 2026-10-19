//! 流转链地基两列：销售订单明细行的链来源 + 采购订单的转采来源销售订单
//!
//! ## 六要素
//! - **功能**：一次域内补两条可空列，把"三条流转链（订货/现货/调货）的真相源"钉在库上。
//!   1. `sales_order_items.fulfillment_source` TEXT 可空，带 CHECK
//!      `chk_sales_order_item_fulfillment_source`，取值恰为
//!      `'production'`（订货：销售→生产领料染色入库出库）/
//!      `'stock'`（现货：生产入库后销售出库）/
//!      `'purchase'`（调货：销售→转采入库出库）三值或 NULL；
//!      粒度是**行级**——同一张销售订单允许既有自产行又有调货行。
//!   2. `purchase_orders.source_sales_order_id` INTEGER 可空，外键指向
//!      `sales_orders(id)`（`ON DELETE SET NULL`）+ 检索索引，把"这张采购单是由哪张
//!      销售订单转出来的"真正落库（此前该值只存在于请求体，用于触发 SKU 对照翻译，
//!      从不入库，转采血缘在库里是断的）。
//! - **调用方**：SeaORM migrator 链，注册在 `domain/fulfillment_source_and_po_origin/`
//!   独立域、`migration/src/lib.rs` 链尾新顶层条目。
//! - **入参**：无（纯 DDL 迁移，不读取应用数据、不回填任何存量行）。
//! - **传给谁**：写入方 `services/so/order_crud.rs`（销售订单建单/改单的明细行落
//!   `fulfillment_source`，入参按权威词表 `models/status/sales.rs::fulfillment_source`
//!   强校验后透传）与 `services/po/order_ops/crud.rs`（采购建单把请求携带的
//!   `source_sales_order_id` 写入本列）；取值集与词表 `ALL` 逐元素相等，由
//!   `tests/fulfillment_source_vocab_check_parity_test.rs` 双向锁定。
//! - **存什么**：information_schema 两条新列与列注释、pg_constraint 中的 CHECK 与 FK
//!   定义、pg_indexes 中的检索索引；不改写任何数据行。
//! - **存哪里**：PostgreSQL `public.sales_order_items` 与 `public.purchase_orders`。
//!
//! ## 为什么是一个域而不是两个
//! 两列同属"流转链真相源"这一处地基，读侧与写侧必须同时到位才有意义；拆成两个域会
//! 让"链来源已定档"这件事在迁移台账上变成两次独立记账，回退时也可能只回退一半。
//!
//! ## 为什么必须新增顶层域
//! schema 变更写进**已应用**域的 `up()` 内部，在复用库上会因该域台账已记账而被整体跳过，
//! 列就永远停在旧型；而 `ADD COLUMN IF NOT EXISTS` 打在已存在的列上是静默 no-op，
//! 连报错都不报。链尾新域名在复用库上属 pending，必然执行（同
//! `domain/bulk_color_item_link/` 的注册理由）。
//!
//! ## 语义与幂等
//! - 两列均可空、无默认值、不回填。`fulfillment_source` 为 NULL 的语义是"链来源未定"，
//!   是合法业务态（存量行、以及尚未按行定档的新行），后续接线方必须按 NULL 处理，
//!   不得把它读成任何一个非空 token；`source_sales_order_id` 为 NULL = 普通采购单
//!   （非转采），不是"值未知"。
//! - CHECK 对 NULL 恒成立（三值逻辑下 `NULL IN (...)` 结果是 NULL，不是 FALSE），
//!   因此约束写成 `"fulfillment_source" IN (...)` 即已放行 NULL，无需再写 `OR ... IS NULL`
//!   ——写了反而让取值清单在两处出现，破坏"值域唯一由 ALLOWED_SQL 注入"的同源形态。
//! - 参照列宽度逐型核对：`sales_orders.id` 为 `SERIAL PRIMARY KEY`（即 INTEGER），
//!   与本列同为 INTEGER，无宽度冲突（外键列与被引用主键类型不一致时 FK 根本建不起来）。
//! - `ON DELETE` 选 SET NULL，选择照 `domain/bulk_color_item_link/` 同型列的先例：
//!   转采血缘是溯源信息，销售订单被删除不应连带删除已开出的采购单，也不应因 FK
//!   阻断销售订单的清理；置 NULL 后该采购单退化为普通采购单，与"无来源"的既有语义一致。
//! - up 全段幂等可重跑：`DROP CONSTRAINT IF EXISTS` → `ADD COLUMN IF NOT EXISTS` →
//!   守卫 → `ADD CONSTRAINT`（PG 的 ADD CONSTRAINT 无 IF NOT EXISTS，故先按名 DROP）→
//!   `CREATE INDEX IF NOT EXISTS` → `COMMENT ON COLUMN`。
//! - 两条守卫都是 fail-visible：新列在干净链上必然零违规，守卫只在"半态库重跑"
//!   （列已存在但约束缺失，期间被旁路脚本直写过词表外取值 / 挂着指向已删销售单的孤儿引用）
//!   时才可能命中；命中即点名样例值/样例行 id 并中止，拒绝在带脏值的列上静默施加约束，
//!   也拒绝在迁移里 UPDATE/DELETE 猜洗数据。
//!
//! ## 回退
//! down 整体回退，顺序与 up 相反：先删 CHECK、再删索引与 FK，最后删两条列。
//! 新增可空列不回填、不改写任何存量值，故无需备份表即可精确回退；删列时 PG 会连带
//! 清理该列上的索引与约束，显式先删是为了让回退语句本身可读、且不依赖隐式级联。
//! 回退不做"存在非 NULL 值即拒绝"的守卫：这两列尚未被任何业务流程消费，
//! 值本身不含需要人工裁量的终态结论（区别于已裁量的审批拒绝态）。

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_fulfillment_source_and_po_origin"
    }
}

/// 链来源词表的 SQL 字面量清单（与 models/status/sales.rs::fulfillment_source::ALL
/// 逐元素同源；守卫与 CHECK 两处共用此一份，禁止另写一套，取值集相等由契约锁
/// tests/fulfillment_source_vocab_check_parity_test.rs 判定）。
const ALLOWED_SQL: &str = "'production', 'stock', 'purchase'";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();

        // 1) 幂等前置：按名删旧约束（重跑/半态库不撞重名），本段不动任何数据。
        let setup = r#"
ALTER TABLE "sales_order_items" DROP CONSTRAINT IF EXISTS "chk_sales_order_item_fulfillment_source";
ALTER TABLE "purchase_orders" DROP CONSTRAINT IF EXISTS "fk_purchase_orders_source_sales_order";
"#;
        conn.execute_unprepared(setup).await?;

        // 2) 补两条可空列：无默认值、不回填（默认值会替没做过决定的行编一个决定）。
        conn.execute_unprepared(
            r#"ALTER TABLE "sales_order_items" ADD COLUMN IF NOT EXISTS "fulfillment_source" TEXT;"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"ALTER TABLE "purchase_orders" ADD COLUMN IF NOT EXISTS "source_sales_order_id" INTEGER;"#,
        )
        .await?;

        // 3) 词表守卫（fail-visible）：统计链来源列上的词表外非 NULL 取值并采样例值，
        //    非 0 即点名中止，拒绝带着脏值静默施加 CHECK。
        let vocab_guard = format!(
            r#"
DO $$
DECLARE
    bad_rows INTEGER;
    samples  TEXT;
BEGIN
    SELECT COUNT(*) INTO bad_rows FROM "sales_order_items"
      WHERE "fulfillment_source" IS NOT NULL
        AND "fulfillment_source" NOT IN ({allowed});
    IF bad_rows = 0 THEN
        RAISE NOTICE 'm_fulfillment_source_and_po_origin：sales_order_items.fulfillment_source 无词表外存量取值，可安全施加 CHECK chk_sales_order_item_fulfillment_source。';
    ELSE
        SELECT string_agg(sample, ', ') INTO samples
          FROM (SELECT DISTINCT "fulfillment_source" AS sample
                  FROM "sales_order_items"
                 WHERE "fulfillment_source" IS NOT NULL
                   AND "fulfillment_source" NOT IN ({allowed})
                 LIMIT 10) s;
        RAISE EXCEPTION 'm_fulfillment_source_and_po_origin：sales_order_items.fulfillment_source 存在 % 行词表外取值（样例值：%），拒绝施加 CHECK chk_sales_order_item_fulfillment_source。', bad_rows, samples
            USING HINT = '本迁移拒绝 UPDATE 造值：请按权威词表 models/status/sales.rs::fulfillment_source（production/stock/purchase，全小写）逐行核实更正存量，更正后重跑迁移；若脏值来自旁路脚本直写，先修写入方，不要在迁移里圆场。';
    END IF;
END;
$$;
"#,
            allowed = ALLOWED_SQL,
        );
        conn.execute_unprepared(&vocab_guard).await?;

        // 4) 施加 CHECK：值域唯一由 ALLOWED_SQL 注入；NULL 由三值逻辑天然放行。
        let apply_check = format!(
            r#"ALTER TABLE "sales_order_items" ADD CONSTRAINT "chk_sales_order_item_fulfillment_source" CHECK ("fulfillment_source" IN ({allowed}));"#,
            allowed = ALLOWED_SQL,
        );
        conn.execute_unprepared(&apply_check).await?;

        // 5) 外键孤儿守卫（fail-visible）：统计指向不存在销售订单的非 NULL 引用并采样例行 id，
        //    非 0 即点名中止。拒绝在带孤儿引用的列上静默加 FK，也拒绝在迁移里猜洗引用。
        let fk_guard = r#"
DO $$
DECLARE
    orphan_rows INTEGER;
    samples     TEXT;
BEGIN
    SELECT COUNT(*) INTO orphan_rows FROM "purchase_orders" po
      WHERE po."source_sales_order_id" IS NOT NULL
        AND NOT EXISTS (SELECT 1 FROM "sales_orders" so WHERE so."id" = po."source_sales_order_id");
    IF orphan_rows = 0 THEN
        RAISE NOTICE 'm_fulfillment_source_and_po_origin：purchase_orders.source_sales_order_id 无孤儿引用（NULL 的普通采购单不违反 FK），可安全施加 FK fk_purchase_orders_source_sales_order。';
    ELSE
        SELECT string_agg(sample, ', ') INTO samples
          FROM (SELECT po."id"::text AS sample
                  FROM "purchase_orders" po
                 WHERE po."source_sales_order_id" IS NOT NULL
                   AND NOT EXISTS (SELECT 1 FROM "sales_orders" so WHERE so."id" = po."source_sales_order_id")
                 LIMIT 10) s;
        RAISE EXCEPTION 'm_fulfillment_source_and_po_origin：purchase_orders.source_sales_order_id 存在 % 行孤儿引用（样例行 id：%），拒绝添加 FK fk_purchase_orders_source_sales_order。', orphan_rows, samples
            USING HINT = '本迁移拒绝 UPDATE/DELETE 猜洗数据：孤儿引用只能由业务定性（补建被删销售订单、把该行来源改挂有效销售订单、或置 NULL 退化为普通采购单），核实后重跑迁移。';
    END IF;
END;
$$;
"#;
        conn.execute_unprepared(fk_guard).await?;

        // 6) 施加外键（ON DELETE SET NULL，理由见文件头「语义与幂等」）+ 检索索引。
        conn.execute_unprepared(
            r#"ALTER TABLE "purchase_orders" ADD CONSTRAINT "fk_purchase_orders_source_sales_order" FOREIGN KEY ("source_sales_order_id") REFERENCES "sales_orders"("id") ON DELETE SET NULL;"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"CREATE INDEX IF NOT EXISTS "idx_purchase_orders_source_sales_order_id" ON "purchase_orders"("source_sales_order_id");"#,
        )
        .await?;

        // 7) 列注释固化权威词表与写入方指向（DB 侧可自证同源，不依赖仓外文档）。
        conn.execute_unprepared(
            r#"COMMENT ON COLUMN "sales_order_items"."fulfillment_source" IS '流转链来源（行级，可空）：NULL=未定档，production=订货自产 / stock=现货 / purchase=调货转采。权威词表 models/status/sales.rs::fulfillment_source，取值集与 CHECK chk_sales_order_item_fulfillment_source 逐项相等（契约锁 tests/fulfillment_source_vocab_check_parity_test.rs）；写入方 services/so/order_crud.rs 建单与改单';"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"COMMENT ON COLUMN "purchase_orders"."source_sales_order_id" IS '转采来源销售订单 ID（可空）：NULL=普通采购单，非空=由该销售订单转出。写入方 services/po/order_ops/crud.rs 建单；外键 fk_purchase_orders_source_sales_order ON DELETE SET NULL（销售单被删后本单退化为普通采购单，血缘不断在请求体里）';"#,
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        let sql = r#"
ALTER TABLE "sales_order_items" DROP CONSTRAINT IF EXISTS "chk_sales_order_item_fulfillment_source";
DROP INDEX IF EXISTS "idx_purchase_orders_source_sales_order_id";
ALTER TABLE "purchase_orders" DROP CONSTRAINT IF EXISTS "fk_purchase_orders_source_sales_order";
ALTER TABLE "purchase_orders" DROP COLUMN IF EXISTS "source_sales_order_id";
ALTER TABLE "sales_order_items" DROP COLUMN IF EXISTS "fulfillment_source";
"#;
        conn.execute_unprepared(sql).await?;
        Ok(())
    }
}
