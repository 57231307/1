-- BPM流程实例外键关联迁移
-- 日期: 2026-05-09
-- 描述: 为BPM流程实例添加业务类型枚举约束和索引

BEGIN;

-- 添加业务类型枚举约束（通过CHECK约束实现）
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'bpm_process_instance') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_bpm_business_type' AND table_name = 'bpm_process_instance') THEN
            ALTER TABLE bpm_process_instance 
                ADD CONSTRAINT fk_bpm_business_type 
                CHECK (business_type IN ('SALES_ORDER', 'PURCHASE_ORDER', 'INVOICE', 'PAYMENT', 'INVENTORY_TRANSFER', 'QUALITY_INSPECTION', 'BUDGET_APPROVAL'));
        END IF;
    END IF;
END $$;

-- 添加联合索引优化业务查询
CREATE INDEX IF NOT EXISTS idx_bpm_business ON bpm_process_instance(business_type, business_id);

-- 添加流程定义外键（如果bpm_process_definition表存在）
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'bpm_process_definition') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_bpm_process_definition') THEN
            ALTER TABLE bpm_process_instance 
                ADD CONSTRAINT fk_bpm_process_definition 
                FOREIGN KEY (process_definition_id) REFERENCES bpm_process_definition(id);
        END IF;
    END IF;
END $$;

-- 添加创建者外键（使用 initiator_id 列）
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'bpm_process_instance') 
       AND EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'users') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_bpm_initiator') THEN
            ALTER TABLE bpm_process_instance 
                ADD CONSTRAINT fk_bpm_initiator 
                FOREIGN KEY (initiator_id) REFERENCES users(id);
        END IF;
    END IF;
END $$;

COMMIT;
