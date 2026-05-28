-- 回滚批次2：CRM和供应商模块数据库迁移
-- 删除创建的表

-- 删除销售合同表
DROP TABLE IF EXISTS "sales_contracts" CASCADE;

-- 删除采购合同表
DROP TABLE IF EXISTS "purchase_contracts" CASCADE;

-- 删除供应商评估指标表
DROP TABLE IF EXISTS "supplier_evaluation_indicators" CASCADE;

-- 删除供应商产品表
DROP TABLE IF EXISTS "supplier_products" CASCADE;

-- 删除供应商资质表
DROP TABLE IF EXISTS "supplier_qualifications" CASCADE;

-- 删除供应商联系人表
DROP TABLE IF EXISTS "supplier_contacts" CASCADE;

-- 删除客户信用评级表
DROP TABLE IF EXISTS "customer_credit_ratings" CASCADE;

-- 删除CRM商机表
DROP TABLE IF EXISTS "crm_opportunity" CASCADE;

-- 删除CRM线索表
DROP TABLE IF EXISTS "crm_lead" CASCADE;