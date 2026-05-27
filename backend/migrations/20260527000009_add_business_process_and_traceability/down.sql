-- 回滚批次9：业务流程与追溯
-- 创建时间: 2026-05-27
-- 描述: 删除审批流程、业务追溯、批次追溯和CRM相关表

-- 删除视图
DROP VIEW IF EXISTS "v_business_trace_view" CASCADE;

-- 删除表（按依赖关系逆序）
DROP TABLE IF EXISTS "unqualified_products" CASCADE;
DROP TABLE IF EXISTS "assignment_histories" CASCADE;
DROP TABLE IF EXISTS "crm_opportunity" CASCADE;
DROP TABLE IF EXISTS "crm_lead" CASCADE;
DROP TABLE IF EXISTS "dye_lot_mapping" CASCADE;
DROP TABLE IF EXISTS "batch_trace_log" CASCADE;
DROP TABLE IF EXISTS "batch_dye_lot" CASCADE;
DROP TABLE IF EXISTS "business_trace_assist_links" CASCADE;
DROP TABLE IF EXISTS "business_trace_snapshot" CASCADE;
DROP TABLE IF EXISTS "business_trace_chain" CASCADE;
DROP TABLE IF EXISTS "business_traces" CASCADE;
DROP TABLE IF EXISTS "approval_logs" CASCADE;
DROP TABLE IF EXISTS "approval_instances" CASCADE;
DROP TABLE IF EXISTS "approval_nodes" CASCADE;
DROP TABLE IF EXISTS "approval_templates" CASCADE;
