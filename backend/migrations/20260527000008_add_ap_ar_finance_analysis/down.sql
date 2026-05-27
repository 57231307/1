-- 回滚批次8：财务应收应付 + 多币种 + 财务分析
-- 创建时间: 2026-05-27
-- 描述: 删除财务应收应付、多币种和财务分析相关表

-- 删除表（按依赖关系逆序）
DROP TABLE IF EXISTS "customer_credit_ratings" CASCADE;
DROP TABLE IF EXISTS "fund_transfers" CASCADE;
DROP TABLE IF EXISTS "fund_accounts" CASCADE;
DROP TABLE IF EXISTS "fixed_asset_disposals" CASCADE;
DROP TABLE IF EXISTS "fixed_assets" CASCADE;
DROP TABLE IF EXISTS "budget_adjustments" CASCADE;
DROP TABLE IF EXISTS "budget_executions" CASCADE;
DROP TABLE IF EXISTS "budget_items" CASCADE;
DROP TABLE IF EXISTS "budget_plans" CASCADE;
DROP TABLE IF EXISTS "cost_analyses" CASCADE;
DROP TABLE IF EXISTS "cost_collections" CASCADE;
DROP TABLE IF EXISTS "financial_analysis_results" CASCADE;
DROP TABLE IF EXISTS "financial_indicators" CASCADE;
DROP TABLE IF EXISTS "ar_aging_analysis" CASCADE;
DROP TABLE IF EXISTS "ar_reconciliation_items" CASCADE;
DROP TABLE IF EXISTS "ar_reconciliations" CASCADE;
DROP TABLE IF EXISTS "ar_collections" CASCADE;
DROP TABLE IF EXISTS "ar_invoices" CASCADE;
DROP TABLE IF EXISTS "ap_verification_item" CASCADE;
DROP TABLE IF EXISTS "ap_verification" CASCADE;
DROP TABLE IF EXISTS "ap_reconciliation" CASCADE;
DROP TABLE IF EXISTS "ap_payment" CASCADE;
DROP TABLE IF EXISTS "ap_payment_request_item" CASCADE;
DROP TABLE IF EXISTS "ap_payment_request" CASCADE;
DROP TABLE IF EXISTS "ap_invoice" CASCADE;
