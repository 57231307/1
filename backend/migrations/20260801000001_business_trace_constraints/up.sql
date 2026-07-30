-- V15 主线审计 P2-06 修复：业务追溯三张表加唯一性/CHECK 约束与逻辑外键触发器
--
-- 背景：
--   business_trace_chain / business_trace_snapshot / business_trace_assist_links
--   在 20260527000009 迁移中创建时只建了普通索引，没有 UNIQUE / CHECK / 触发器，
--   存在以下数据完整性风险：
--     1) snapshot 表的 trace_chain_id 没有任何 FK 约束，可能出现"孤儿快照"。
--     2) snapshot 同一 trace_chain_id 重复插入，UI 取"最新快照"行为未定义。
--     3) assist_links 同一 (trace_id, assist_type, assist_id) 重复插入，污染关联图。
--     4) chain 表的数量字段允许负值，账实方向不可控。
--     5) chain 表自身环 (previous_trace_id = next_trace_id) 未禁止。
--     6) chain 同一 trace_chain_id 可能存在多个 head（previous_trace_id IS NULL）
--        或多个 tail（next_trace_id IS NULL），形成链分叉。
--
-- 注意：
--   * chain.trace_chain_id 不是唯一键 —— 一个 chain 可有多个阶段行。
--     head 唯一用部分索引：UNIQUE (trace_chain_id) WHERE previous_trace_id IS NULL
--     tail 唯一用部分索引：UNIQUE (trace_chain_id) WHERE next_trace_id IS NULL
--   * snapshot.trace_chain_id 是唯一键（每 chain 一份最新快照）。
--   * 不引入真外键（chain.trace_chain_id 是 VARCHAR 全局业务键，不是 chain.id 数字主键），
--     用 BEFORE 触发器做"逻辑外键"。

-- ============================================================
-- 1) chain head/tail 部分唯一（防链分叉 / 孤儿）
-- ============================================================
CREATE UNIQUE INDEX IF NOT EXISTS "uniq_business_trace_chain_head"
    ON "business_trace_chain" ("trace_chain_id")
    WHERE "previous_trace_id" IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS "uniq_business_trace_chain_tail"
    ON "business_trace_chain" ("trace_chain_id")
    WHERE "next_trace_id" IS NULL;

-- ============================================================
-- 2) snapshot.trace_chain_id 唯一（每 chain 一份最新快照）
-- ============================================================
CREATE UNIQUE INDEX IF NOT EXISTS "uniq_business_trace_snapshot_chain_id"
    ON "business_trace_snapshot" ("trace_chain_id");

-- ============================================================
-- 3) assist_links 联合唯一（防重复关联）
-- ============================================================
CREATE UNIQUE INDEX IF NOT EXISTS "uniq_business_trace_assist_links"
    ON "business_trace_assist_links" ("trace_id", "assist_type", "assist_id");

-- ============================================================
-- 4) chain 形状校验
-- ============================================================
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'chk_business_trace_chain_quantities_nonneg'
    ) THEN
        ALTER TABLE "business_trace_chain"
            ADD CONSTRAINT "chk_business_trace_chain_quantities_nonneg"
            CHECK ("quantity_meters" >= 0 AND "quantity_kg" >= 0);
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'chk_business_trace_chain_no_self_loop'
    ) THEN
        ALTER TABLE "business_trace_chain"
            ADD CONSTRAINT "chk_business_trace_chain_no_self_loop"
            CHECK (
                "previous_trace_id" IS NULL
                OR "next_trace_id" IS NULL
                OR "previous_trace_id" <> "next_trace_id"
            );
    END IF;
END $$;

-- ============================================================
-- 5) snapshot 形状校验
-- ============================================================
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'chk_business_trace_snapshot_quantities_nonneg'
    ) THEN
        ALTER TABLE "business_trace_snapshot"
            ADD CONSTRAINT "chk_business_trace_snapshot_quantities_nonneg"
            CHECK ("current_quantity_meters" >= 0 AND "current_quantity_kg" >= 0);
    END IF;
END $$;

-- ============================================================
-- 6) 逻辑外键：snapshot.trace_chain_id 必须存在 head 节点
--    （避免孤儿快照；chain 表是上游真源）
-- ============================================================
CREATE OR REPLACE FUNCTION "fn_business_trace_snapshot_chain_fk"()
RETURNS TRIGGER AS $$
DECLARE
    head_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO head_count
    FROM "business_trace_chain"
    WHERE "trace_chain_id" = NEW."trace_chain_id"
      AND "previous_trace_id" IS NULL;

    IF head_count = 0 THEN
        RAISE EXCEPTION
            'business_trace_snapshot.trace_chain_id=% 在 business_trace_chain 不存在 head 节点',
            NEW."trace_chain_id"
            USING ERRCODE = 'foreign_key_violation';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS "trg_business_trace_snapshot_chain_fk" ON "business_trace_snapshot";
CREATE TRIGGER "trg_business_trace_snapshot_chain_fk"
    BEFORE INSERT OR UPDATE OF "trace_chain_id" ON "business_trace_snapshot"
    FOR EACH ROW
    EXECUTE FUNCTION "fn_business_trace_snapshot_chain_fk"();

-- ============================================================
-- 7) 逻辑外键：assist_links.trace_id 必须存在于 chain.id
-- ============================================================
CREATE OR REPLACE FUNCTION "fn_business_trace_assist_links_chain_fk"()
RETURNS TRIGGER AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM "business_trace_chain"
        WHERE "id" = NEW."trace_id"
    ) THEN
        RAISE EXCEPTION
            'business_trace_assist_links.trace_id=% 不存在于 business_trace_chain',
            NEW."trace_id"
            USING ERRCODE = 'foreign_key_violation';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS "trg_business_trace_assist_links_chain_fk" ON "business_trace_assist_links";
CREATE TRIGGER "trg_business_trace_assist_links_chain_fk"
    BEFORE INSERT OR UPDATE OF "trace_id" ON "business_trace_assist_links"
    FOR EACH ROW
    EXECUTE FUNCTION "fn_business_trace_assist_links_chain_fk"();

-- ============================================================
-- 8) 自洽校验：snapshot 的五维 ID / product_id / batch_no / color_no / grade
--    必须等于 chain head 的同字段（防快照漂移）
-- ============================================================
CREATE OR REPLACE FUNCTION "fn_business_trace_snapshot_self_consistency"()
RETURNS TRIGGER AS $$
DECLARE
    chain_record "business_trace_chain"%ROWTYPE;
BEGIN
    SELECT * INTO chain_record
    FROM "business_trace_chain"
    WHERE "trace_chain_id" = NEW."trace_chain_id"
      AND "previous_trace_id" IS NULL
    LIMIT 1;

    IF chain_record.id IS NULL THEN
        RAISE EXCEPTION
            'business_trace_snapshot 自洽校验失败：chain head 不存在 trace_chain_id=%',
            NEW."trace_chain_id"
            USING ERRCODE = 'foreign_key_violation';
    END IF;

    IF chain_record.five_dimension_id <> NEW.five_dimension_id THEN
        RAISE EXCEPTION
            'snapshot.five_dimension_id 与 chain head 不一致 (chain=%, snapshot=%)',
            chain_record.five_dimension_id, NEW.five_dimension_id
            USING ERRCODE = 'check_violation';
    END IF;

    IF chain_record.product_id <> NEW.product_id THEN
        RAISE EXCEPTION
            'snapshot.product_id 与 chain head 不一致 (chain=%, snapshot=%)',
            chain_record.product_id, NEW.product_id
            USING ERRCODE = 'check_violation';
    END IF;

    IF chain_record.batch_no <> NEW.batch_no THEN
        RAISE EXCEPTION
            'snapshot.batch_no 与 chain head 不一致 (chain=%, snapshot=%)',
            chain_record.batch_no, NEW.batch_no
            USING ERRCODE = 'check_violation';
    END IF;

    IF chain_record.color_no <> NEW.color_no THEN
        RAISE EXCEPTION
            'snapshot.color_no 与 chain head 不一致 (chain=%, snapshot=%)',
            chain_record.color_no, NEW.color_no
            USING ERRCODE = 'check_violation';
    END IF;

    IF chain_record.grade <> NEW.grade THEN
        RAISE EXCEPTION
            'snapshot.grade 与 chain head 不一致 (chain=%, snapshot=%)',
            chain_record.grade, NEW.grade
            USING ERRCODE = 'check_violation';
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS "trg_business_trace_snapshot_self_consistency" ON "business_trace_snapshot";
CREATE TRIGGER "trg_business_trace_snapshot_self_consistency"
    BEFORE INSERT OR UPDATE OF
        "trace_chain_id", "five_dimension_id", "product_id",
        "batch_no", "color_no", "grade"
    ON "business_trace_snapshot"
    FOR EACH ROW
    EXECUTE FUNCTION "fn_business_trace_snapshot_self_consistency"();

COMMENT ON INDEX "uniq_business_trace_chain_head" IS
    'V15 P2-06: chain 每条链只能有一个 head 节点 (previous_trace_id IS NULL)';
COMMENT ON INDEX "uniq_business_trace_chain_tail" IS
    'V15 P2-06: chain 每条链只能有一个 tail 节点 (next_trace_id IS NULL)';
COMMENT ON INDEX "uniq_business_trace_snapshot_chain_id" IS
    'V15 P2-06: snapshot 每 chain 一份最新快照';
COMMENT ON INDEX "uniq_business_trace_assist_links" IS
    'V15 P2-06: 同一 (trace_id, assist_type, assist_id) 不允许重复';
COMMENT ON CONSTRAINT "chk_business_trace_chain_quantities_nonneg" ON "business_trace_chain" IS
    'V15 P2-06: 数量字段非负';
COMMENT ON CONSTRAINT "chk_business_trace_chain_no_self_loop" ON "business_trace_chain" IS
    'V15 P2-06: 禁止 previous_trace_id = next_trace_id 自环';
COMMENT ON CONSTRAINT "chk_business_trace_snapshot_quantities_nonneg" ON "business_trace_snapshot" IS
    'V15 P2-06: 快照数量非负';
