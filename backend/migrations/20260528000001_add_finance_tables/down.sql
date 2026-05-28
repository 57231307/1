-- 回滚：删除财务管理模块的表
-- 删除顺序：先删有外键依赖的子表，再删主表

-- 9. 资金账户表
DROP TABLE IF EXISTS "fund_accounts";

-- 8. 固定资产表
DROP TABLE IF EXISTS "fixed_assets";

-- 7. 预算科目表
DROP TABLE IF EXISTS "budget_items";

-- 6. 预算计划表
DROP TABLE IF EXISTS "budget_plans";

-- 5. 应付对账表
DROP TABLE IF EXISTS "ap_reconciliation";

-- 4. 应收对账表
DROP TABLE IF EXISTS "ar_reconciliations";

-- 3. 应收发票表
DROP TABLE IF EXISTS "ar_invoices";

-- 2. 应付付款表
DROP TABLE IF EXISTS "ap_payment";

-- 1. 应付发票表
DROP TABLE IF EXISTS "ap_invoice";
