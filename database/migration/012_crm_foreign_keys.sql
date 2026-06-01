-- CRM模块外键关联迁移
-- 日期: 2026-05-09
-- 描述: 为CRM线索和客户建立外键关联
-- 注意: crm_lead和crm_opportunity表可能在后续迁移中创建

BEGIN;

-- 为CRM线索添加客户外键（如果表存在）
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'crm_lead') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'crm_lead' AND column_name = 'customer_id') THEN
            ALTER TABLE crm_lead ADD COLUMN customer_id INTEGER;
        END IF;
        IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_crm_lead_customer') THEN
            ALTER TABLE crm_lead ADD CONSTRAINT fk_crm_lead_customer FOREIGN KEY (customer_id) REFERENCES customers(id);
        END IF;
        CREATE INDEX IF NOT EXISTS idx_crm_lead_customer ON crm_lead(customer_id);
        
        IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'crm_lead' AND column_name = 'assigned_to') THEN
            ALTER TABLE crm_lead ADD COLUMN assigned_to INTEGER;
        END IF;
        IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_crm_lead_assigned') THEN
            ALTER TABLE crm_lead ADD CONSTRAINT fk_crm_lead_assigned FOREIGN KEY (assigned_to) REFERENCES users(id);
        END IF;
        CREATE INDEX IF NOT EXISTS idx_crm_lead_assigned ON crm_lead(assigned_to);
    END IF;
END $$;

-- 为CRM商机添加客户外键（如果表存在）
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'crm_opportunity') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'crm_opportunity' AND column_name = 'customer_id') THEN
            ALTER TABLE crm_opportunity ADD COLUMN customer_id INTEGER;
        END IF;
        IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_crm_opportunity_customer') THEN
            ALTER TABLE crm_opportunity ADD CONSTRAINT fk_crm_opportunity_customer FOREIGN KEY (customer_id) REFERENCES customers(id);
        END IF;
        CREATE INDEX IF NOT EXISTS idx_crm_opportunity_customer ON crm_opportunity(customer_id);
        
        IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'crm_opportunity' AND column_name = 'assigned_to') THEN
            ALTER TABLE crm_opportunity ADD COLUMN assigned_to INTEGER;
        END IF;
        IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_crm_opportunity_assigned') THEN
            ALTER TABLE crm_opportunity ADD CONSTRAINT fk_crm_opportunity_assigned FOREIGN KEY (assigned_to) REFERENCES users(id);
        END IF;
        CREATE INDEX IF NOT EXISTS idx_crm_opportunity_assigned ON crm_opportunity(assigned_to);
    END IF;
END $$;

COMMIT;
