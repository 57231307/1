-- 回滚批次2：总账与财务基础
-- 创建时间: 2026-05-27
-- 描述: 删除总账和财务基础表

-- 删除表（按依赖关系逆序）
DROP TABLE IF EXISTS "assist_accounting_summary" CASCADE;
DROP TABLE IF EXISTS "assist_accounting_record" CASCADE;
DROP TABLE IF EXISTS "assist_accounting_dimension" CASCADE;
DROP TABLE IF EXISTS "voucher_items" CASCADE;
DROP TABLE IF EXISTS "vouchers" CASCADE;
DROP TABLE IF EXISTS "accounting_periods" CASCADE;
DROP TABLE IF EXISTS "account_balances" CASCADE;
DROP TABLE IF EXISTS "account_subjects" CASCADE;
