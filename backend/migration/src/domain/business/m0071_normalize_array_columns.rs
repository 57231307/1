//! 数组语义列 TEXT→PG 数组类型归一（"ADD COLUMN IF NOT EXISTS /
//! 列型漂移被静默吞掉"族）
//!
//! 根因形态：模型侧是 SeaORM 原生数组类型（`Option<Vec<String>>` / `Option<Vec<i32>>`，
//! 均**无** `#[sea_orm(column_type = "Json")]` 属性 ⇒ 编解码只认 PG 数组类型），
//! 而生效 DDL 是标量 `TEXT`。后果分两层：
//! - 非 NULL 行读取即 `ColumnDecode { ... is not compatible with SQL type TEXT }`
//!   （#4671 p3 `live_receipt_update_null_clears_nullable_and_absent_keeps` 红 @
//!   `contract_wave3_explicit_null_clear_test.rs:1713`；p2 `add_tags_response_is_masked_lead_row` 500）；
//! - 写入/回读整链带病，属线上数据可见性缺陷，不只是测试红。
//!
//! 同型列穷举依据（模型定义 file:line ↔ 生效 DDL file:line ↔ 写入侧 file:line）：
//! - purchase_receipt.attachment_urls   models/purchase_receipt.rs:75 ↔ business/m0009:82 ↔ services/purchase_receipt_ops/crud.rs:429-430、purchase_receipt_service.rs:127
//! - ap_invoice.attachment_urls         models/ap_invoice.rs:91     ↔ business/m0012:37  ↔ services/ap_invoice_ops/crud.rs:84,150
//! - ap_payment_request.attachment_urls models/ap_payment_request.rs:75 ↔ business/m0012:73 ↔ services/ap_payment_request_service.rs:223,306
//! - ap_payment.attachment_urls         models/ap_payment.rs:71     ↔ business/m0012:124 ↔ services/ap_payment_service.rs:116,173
//! - crm_lead.tags                      models/crm_lead.rs:123      ↔ business/m0013:295 ↔ services/crm/lead.rs:100,575
//! - crm_opportunity.tags               models/crm_opportunity.rs:110 ↔ business/m0013:337 ↔ services/crm/opp.rs:140,491
//! - crm_opportunity.product_ids        models/crm_opportunity.rs（Vec<i32>）↔ business/m0013:322 ↔ services/crm/opp.rs:132,476
//! - crm_opportunity.product_names      models/crm_opportunity.rs（Vec<String>）↔ business/m0013:323 ↔ services/crm/opp.rs:133,479
//! - crm_opportunity.competitor_names   models/crm_opportunity.rs（Vec<String>）↔ business/m0013:333 ↔ 暂无写点（列型仍须与模型声明一致）
//! - purchase_receipt_item.internal_piece_ids    models/purchase_receipt_item.rs（Vec<i32>）↔ business/m0009:129 ↔ services/purchase_receipt_ops/crud.rs:587（暂仅 None 写点）
//! - purchase_receipt_item.internal_piece_nos    models/purchase_receipt_item.rs（Vec<String>）↔ business/m0009:130 ↔ 同上 :588
//! - purchase_receipt_item.supplier_piece_nos    models/purchase_receipt_item.rs（Vec<String>）↔ business/m0009:132 ↔ 同上 :590
//!
//! 为什么方向已定（TEXT 侧只有数组一条修法）：
//! - 既有成文口径：`services/po/order.rs:43` 钉死 purchase_orders.attachment_urls
//!   "真实列 TEXT[]"（建列 DDL `system/mod.rs:332 ADD COLUMN ... "attachment_urls" TEXT[]`），
//!   `sales_crm/mod.rs:356` 明写"保留 crm_lead.tags **TEXT[]** 数组字段向后兼容
//!   （add_tags handler 仍覆盖式更新该数组）"⇒ 同名同语义列的仓库既定形态就是数组；
//! - `production/mod.rs:281` 对 crm_lead.tags 的 `ADD COLUMN IF NOT EXISTS "tags" JSONB`
//!   因列已存在（business/m0013 先执行）是**恒 no-op**，从未生效，不构成反例；
//! - 反向修法（模型降级 `Option<String>`）明确禁止（会分叉出第二套附件口径）；
//!   标量 TEXT 与无 Json 属性的 Vec 模型在读写两侧都不可成立。
//!
//! 本批**不动**的同型列（修复方向在模型侧，交后端 W2 批处理）：
//! bpm_task.assignee_ids/assignee_names/candidate_role_ids/candidate_user_ids、
//! bpm_process_instance.current_handler_ids/current_handler_names（system/mod.rs:75-96
//! 生效 DDL 为 JSONB）、aging_alert_rules.notify_roles（v15 CREATE 为 JSONB）。
//! 依据仓规 `75f2a44c`「模型类型必须跟随 DDL，不得反向改迁移迁就模型」：JSONB 是
//! 这些列的**生效且成文**的 DDL 意图，正解是模型补 `#[sea_orm(column_type =
//! "JsonBinary")]` + 元素 `FromJsonQueryResult`（仓库既有可运行范本 =
//! `models/production_recipe.rs` recipe_detail / `models/lab_dip_sample.rs`
//! formula_detail）；改列型反而是反向迁就，禁止。TEXT 列不存在任何可让 Vec 工作的
//! column_type 属性，故 TEXT 族只有本迁移一条修法——这正是两侧分界的判据。
//!
//! 存量数据策略（fail-visible，照 m0063/m0067/m0068 先例）：
//! - up 逐列先只读探测：非 NULL 且非空串的值若无法无损 cast 为目标数组类型
//!   （PG 数组字面量 `{...}` 或 JSON 风格 `[...]` 均可 cast）则 RAISE EXCEPTION
//!   点名表/列/行数/样例 id，人工改写或确认为空后置 NULL 后重跑；绝不猜测归一、
//!   绝不 COALESCE 洗数据。空串 `''` 按无损解释转空数组 `{}`（不凭空造元素）。
//! - 已是数组类型的列跳过（幂等重跑等价）；表/列不存在视为结构漂移，fail-visible 中止。
//!
//! down 真实可逆（禁止空实现，教训见 rls_dept）：逐列回退 TEXT。回退是无损的当且
//! 仅当每行至多一个元素（单元素→原值、空数组→''）；存在 ≥2 元素行 = 旧标量形态
//! 无成文序列化口径、任何回写都是发明格式吞数据 ⇒ RAISE EXCEPTION 拒绝回滚并给
//! 人工处置规则（人工决定合并/取首/置空并留痕后重跑 down）。非数组列跳过（幂等）。
//!
//! 契约锁：`backend/tests/contract_wave7_column_shape_lock_test.rs`（information_schema
//! 活库断言本批全部列 data_type/udt_name/is_nullable，把"迁移文本写了数组类型但
//! 实际列是 TEXT"这类静默失效永久钉死）。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// 表/列/目标数组类型清单（up/down 同源一张表，禁止两边各写一套）。
/// 目标类型来自本文件头注释的模型声明逐列核对：Vec<String>→text[]，Vec<i32>→integer[]。
const ARRAY_COLUMNS: &[(&str, &str, &str)] = &[
    ("purchase_receipt", "attachment_urls", "text[]"),
    ("ap_invoice", "attachment_urls", "text[]"),
    ("ap_payment_request", "attachment_urls", "text[]"),
    ("ap_payment", "attachment_urls", "text[]"),
    ("crm_lead", "tags", "text[]"),
    ("crm_opportunity", "tags", "text[]"),
    ("crm_opportunity", "product_ids", "integer[]"),
    ("crm_opportunity", "product_names", "text[]"),
    ("crm_opportunity", "competitor_names", "text[]"),
    ("purchase_receipt_item", "internal_piece_ids", "integer[]"),
    ("purchase_receipt_item", "internal_piece_nos", "text[]"),
    ("purchase_receipt_item", "supplier_piece_nos", "text[]"),
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 清单以 VALUES 注入 plpgsql，逐列：结构存在性守卫 → 只读探测 → EXECUTE ALTER。
        let specs: Vec<String> = ARRAY_COLUMNS
            .iter()
            .map(|(t, c, k)| format!("'{t}','{c}','{k}'"))
            .collect();
        let specs = specs.join("),(");
        let sql = format!(
            r#"DO $$
DECLARE
    spec RECORD;
    r RECORD;
    scratch TEXT;
    bad_count INTEGER;
    samples TEXT;
    spec_count INTEGER := 0;
BEGIN
    FOR spec IN
        SELECT tbl, col, target FROM (VALUES ({specs})) AS t(tbl, col, target)
    LOOP
        spec_count := spec_count + 1;
        -- 结构守卫：表/列缺失 = 与本域建表迁移（m0009/m0012/m0013）漂移，fail-visible 中止
        IF to_regclass('public.' || spec.tbl) IS NULL THEN
            RAISE EXCEPTION '表 % 不存在：与本 business 域建表迁移漂移，本迁移拒绝在未知结构上操作，请人工核实库现状。', spec.tbl;
        END IF;
        IF NOT EXISTS (SELECT 1 FROM information_schema.columns
                        WHERE table_schema = 'public' AND table_name = spec.tbl AND column_name = spec.col) THEN
            RAISE EXCEPTION '列 %.% 不存在：与本 business 域建表迁移漂移，请人工核实库现状。', spec.tbl, spec.col;
        END IF;
        -- 已是数组类型 ⇒ 幂等跳过
        IF EXISTS (SELECT 1 FROM information_schema.columns
                    WHERE table_schema = 'public' AND table_name = spec.tbl AND column_name = spec.col
                      AND data_type = 'ARRAY') THEN
            CONTINUE;
        END IF;
        -- fail-visible 探测：非 NULL/非空串而无法无损 cast 为目标数组类型的存量值
        bad_count := 0;
        samples := '';
        FOR r IN EXECUTE format(
                'SELECT id, btrim(%I) AS v FROM %I WHERE %I IS NOT NULL AND btrim(%I) <> %L',
                spec.col, spec.tbl, spec.col, spec.col, '')
        LOOP
            BEGIN
                EXECUTE format('SELECT (%L)::%s', r.v, spec.target) INTO scratch;
            EXCEPTION WHEN others THEN
                bad_count := bad_count + 1;
                IF char_length(samples) < 300 THEN
                    samples := samples || ' | id=' || r.id::text || ' v=' || left(r.v, 60);
                END IF;
            END;
        END LOOP;
        IF bad_count > 0 THEN
            RAISE EXCEPTION '表 % 的列 % 存在 % 个无法无损转换为 % 的存量值（样例:%）。本迁移拒绝猜测归一/UPDATE 洗数据：请人工把这些值改写为 PG 数组字面量或 JSON 数组（与模型数组语义一致），或确认为空后手工置 NULL，然后重跑。',
                spec.tbl, spec.col, bad_count, spec.target, samples;
        END IF;
        EXECUTE format(
            'ALTER TABLE %I ALTER COLUMN %I TYPE %s USING (CASE WHEN %I IS NULL THEN NULL WHEN btrim(%I) = %L THEN ARRAY[]::%s ELSE %I::%s END)',
            spec.tbl, spec.col, spec.target, spec.col, spec.col, '', spec.target, spec.col, spec.target);
    END LOOP;
    IF spec_count <> 12 THEN
        RAISE EXCEPTION '数组列归一清单应有 12 列，实际注入 % 列：清单注册漂移，中止。', spec_count;
    END IF;
END
$$;"#
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 回退 TEXT：单元素→原值、空数组→''（无损）；≥2 元素行拒绝回滚（旧标量形态
        // 无成文序列化口径，任何回写都是发明格式吞数据），给人工处置规则后重跑。
        let rows: Vec<String> = ARRAY_COLUMNS
            .iter()
            .map(|(t, c, _)| format!("'{t}','{c}'"))
            .collect();
        let specs = rows.join("),(");
        let sql = format!(
            r#"DO $$
DECLARE
    spec RECORD;
    multi_count INTEGER;
    samples TEXT;
BEGIN
    FOR spec IN
        SELECT tbl, col FROM (VALUES ({specs})) AS t(tbl, col)
    LOOP
        IF to_regclass('public.' || spec.tbl) IS NULL THEN
            CONTINUE; -- 演练库中表不存在（整链回退时上游 down 已删表）
        END IF;
        IF NOT EXISTS (SELECT 1 FROM information_schema.columns
                        WHERE table_schema = 'public' AND table_name = spec.tbl AND column_name = spec.col) THEN
            CONTINUE;
        END IF;
        IF NOT EXISTS (SELECT 1 FROM information_schema.columns
                        WHERE table_schema = 'public' AND table_name = spec.tbl AND column_name = spec.col
                          AND data_type = 'ARRAY') THEN
            CONTINUE; -- 幂等：未在数组形态（up 未跑/已回退）
        END IF;
        EXECUTE format(
            'SELECT COUNT(*) FROM %I WHERE array_length(%I, 1) > 1', spec.tbl, spec.col)
        INTO multi_count;
        IF multi_count > 0 THEN
            EXECUTE format(
                'SELECT string_agg(id::text, %L ORDER BY id) FROM (SELECT id FROM %I WHERE array_length(%I, 1) > 1 ORDER BY id LIMIT 10) s',
                ', ', spec.tbl, spec.col)
            INTO samples;
            RAISE EXCEPTION '表 % 的列 % 存在 % 行多元素数组（样例 id：%）。回退到旧的标量 TEXT 形态没有成文的多样值序列化口径，本迁移拒绝发明格式回写吞数据：请人工逐行决定取首元素/合并文本/置空并在 remarks 留痕（口径由该列业务 owner 裁定）后重跑 down。',
                spec.tbl, spec.col, multi_count, samples;
        END IF;
        EXECUTE format(
            'ALTER TABLE %I ALTER COLUMN %I TYPE TEXT USING (CASE WHEN %I IS NULL THEN NULL WHEN array_length(%I, 1) IS NULL THEN %L ELSE %I[1]::text END)',
            spec.tbl, spec.col, spec.col, spec.col, '', spec.col);
    END LOOP;
END
$$;"#
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }
}
