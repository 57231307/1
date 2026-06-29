-- 回滚：删除 CRM 公海回收规则表
-- 创建时间: 2026-06-29
-- 关联修复: 批次 23 v5 P0-4

DROP INDEX IF EXISTS "idx_crm_recycle_rules_enabled";
DROP TABLE IF EXISTS "crm_recycle_rules";
