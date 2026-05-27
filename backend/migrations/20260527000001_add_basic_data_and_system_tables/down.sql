-- 回滚批次1：基础数据扩展 + 系统管理
-- 创建时间: 2026-05-27
-- 描述: 删除基础数据扩展表和系统管理扩展表

-- 删除表（按依赖关系逆序）
DROP TABLE IF EXISTS "audit_alert_rules" CASCADE;
DROP TABLE IF EXISTS "operation_logs" CASCADE;
DROP TABLE IF EXISTS "omni_audit_logs" CASCADE;
DROP TABLE IF EXISTS "log_system" CASCADE;
DROP TABLE IF EXISTS "log_login" CASCADE;
DROP TABLE IF EXISTS "log_api_accesses" CASCADE;
DROP TABLE IF EXISTS "system_version" CASCADE;
DROP TABLE IF EXISTS "webhooks" CASCADE;
DROP TABLE IF EXISTS "api_keys" CASCADE;
DROP TABLE IF EXISTS "data_permissions" CASCADE;
DROP TABLE IF EXISTS "role_permissions" CASCADE;
DROP TABLE IF EXISTS "quality_inspection_records" CASCADE;
DROP TABLE IF EXISTS "quality_inspection_standards" CASCADE;
DROP TABLE IF EXISTS "quality_standards" CASCADE;
DROP TABLE IF EXISTS "supplier_evaluation_indicators" CASCADE;
DROP TABLE IF EXISTS "supplier_grades" CASCADE;
DROP TABLE IF EXISTS "supplier_categories" CASCADE;
DROP TABLE IF EXISTS "exchange_rates" CASCADE;
DROP TABLE IF EXISTS "currencies" CASCADE;
