-- CRM模块外键关联迁移
-- 日期: 2026-05-09
-- 描述: 为CRM线索和客户建立外键关联

BEGIN;

-- 为CRM线索添加客户外键（支持线索转换为客户后的关联）
ALTER TABLE crm_leads 
    ADD COLUMN IF NOT EXISTS customer_id INTEGER REFERENCES customers(id);

CREATE INDEX IF NOT EXISTS idx_crm_lead_customer ON crm_leads(customer_id);

-- 为CRM线索添加负责人外键
ALTER TABLE crm_leads 
    ADD COLUMN IF NOT EXISTS assigned_to INTEGER REFERENCES users(id);

CREATE INDEX IF NOT EXISTS idx_crm_lead_assigned ON crm_leads(assigned_to);

-- 为CRM商机添加客户外键
ALTER TABLE crm_opportunities 
    ADD COLUMN IF NOT EXISTS customer_id INTEGER REFERENCES customers(id);

CREATE INDEX IF NOT EXISTS idx_crm_opportunity_customer ON crm_opportunities(customer_id);

-- 为CRM商机添加负责人外键
ALTER TABLE crm_opportunities 
    ADD COLUMN IF NOT EXISTS assigned_to INTEGER REFERENCES users(id);

CREATE INDEX IF NOT EXISTS idx_crm_opportunity_assigned ON crm_opportunities(assigned_to);

COMMIT;
