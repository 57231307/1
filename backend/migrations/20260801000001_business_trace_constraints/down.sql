-- V15 P2-06 回滚：移除业务追溯三张表加的唯一性/CHECK 约束与逻辑外键触发器

DROP TRIGGER IF EXISTS "trg_business_trace_snapshot_self_consistency" ON "business_trace_snapshot";
DROP TRIGGER IF EXISTS "trg_business_trace_snapshot_chain_fk" ON "business_trace_snapshot";
DROP TRIGGER IF EXISTS "trg_business_trace_assist_links_chain_fk" ON "business_trace_assist_links";

DROP FUNCTION IF EXISTS "fn_business_trace_snapshot_self_consistency"();
DROP FUNCTION IF EXISTS "fn_business_trace_snapshot_chain_fk"();
DROP FUNCTION IF EXISTS "fn_business_trace_assist_links_chain_fk"();

ALTER TABLE "business_trace_snapshot"
    DROP CONSTRAINT IF EXISTS "chk_business_trace_snapshot_quantities_nonneg";

ALTER TABLE "business_trace_chain"
    DROP CONSTRAINT IF EXISTS "chk_business_trace_chain_no_self_loop";
ALTER TABLE "business_trace_chain"
    DROP CONSTRAINT IF EXISTS "chk_business_trace_chain_quantities_nonneg";

DROP INDEX IF EXISTS "uniq_business_trace_assist_links";
DROP INDEX IF EXISTS "uniq_business_trace_snapshot_chain_id";
DROP INDEX IF EXISTS "uniq_business_trace_chain_tail";
DROP INDEX IF EXISTS "uniq_business_trace_chain_head";
