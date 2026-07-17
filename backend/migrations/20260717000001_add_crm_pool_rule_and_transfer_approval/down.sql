-- 回滚 V15 P0-S08 修复

-- 删除大客户阈值配置（若存在）
DELETE FROM system_settings WHERE key = 'crm.large_customer_credit_threshold';

-- 删除客户转移审批表
DROP TABLE IF EXISTS customer_transfer_approvals CASCADE;

-- 删除公海规则配置表
DROP TABLE IF EXISTS customer_pool_rules CASCADE;

-- 删除 customers 表的 owner_id 字段
ALTER TABLE customers DROP COLUMN IF EXISTS owner_assigned_at;
ALTER TABLE customers DROP COLUMN IF EXISTS owner_id;
