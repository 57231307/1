-- 回滚批次5：采购管理扩展
-- 创建时间: 2026-05-27
-- 描述: 删除采购管理扩展表

-- 删除表（按依赖关系逆序）
DROP TABLE IF EXISTS "purchase_prices" CASCADE;
DROP TABLE IF EXISTS "purchase_return_item" CASCADE;
DROP TABLE IF EXISTS "purchase_return" CASCADE;
DROP TABLE IF EXISTS "purchase_inspection" CASCADE;
DROP TABLE IF EXISTS "purchase_receipt_item" CASCADE;
DROP TABLE IF EXISTS "purchase_receipt" CASCADE;
DROP TABLE IF EXISTS "purchase_contract_executions" CASCADE;
DROP TABLE IF EXISTS "purchase_contracts" CASCADE;
