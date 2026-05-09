-- 迁移脚本: 003_foreign_keys.sql
-- 描述: 添加核心实体间的外键关联，消除数据孤岛
-- 日期: 2026-05-09
-- 依赖: 002_order_change_history.sql

BEGIN;

-- ========================================================
-- DB-001: BPM流程实例添加业务类型注释和联合索引
-- ========================================================

-- 添加业务类型约束注释（应用层枚举）
-- SALES_ORDER=销售订单, PURCHASE_ORDER=采购订单, PRODUCTION_ORDER=生产订单, FINANCE=财务单据

-- 添加联合索引用于业务关联查询
CREATE INDEX IF NOT EXISTS idx_bpm_process_instance_business 
ON bpm_process_instance(business_type, business_id) 
WHERE business_type IS NOT NULL AND business_id IS NOT NULL;

-- 添加状态索引
CREATE INDEX IF NOT EXISTS idx_bpm_process_instance_status 
ON bpm_process_instance(status);

-- ========================================================
-- DB-002: CRM线索添加客户外键
-- ========================================================

-- 添加 customer_id 字段（如果不存在）
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'crm_lead' AND column_name = 'customer_id'
    ) THEN
        ALTER TABLE crm_lead ADD COLUMN customer_id INTEGER;
    END IF;
END $$;

-- 添加外键约束
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.table_constraints 
        WHERE constraint_name = 'fk_crm_lead_customer'
    ) THEN
        ALTER TABLE crm_lead 
        ADD CONSTRAINT fk_crm_lead_customer 
        FOREIGN KEY (customer_id) REFERENCES customers(id) 
        ON DELETE SET NULL ON UPDATE CASCADE;
    END IF;
END $$;

-- 添加索引
CREATE INDEX IF NOT EXISTS idx_crm_lead_customer_id ON crm_lead(customer_id) WHERE customer_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_crm_lead_status ON crm_lead(status);

-- ========================================================
-- DB-003: 成本归集添加批次外键
-- ========================================================

-- 添加 batch_id 字段（如果不存在）
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'cost_collections' AND column_name = 'batch_id'
    ) THEN
        ALTER TABLE cost_collections ADD COLUMN batch_id INTEGER;
    END IF;
END $$;

-- 添加外键约束
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.table_constraints 
        WHERE constraint_name = 'fk_cost_collection_batch'
    ) THEN
        ALTER TABLE cost_collections 
        ADD CONSTRAINT fk_cost_collection_batch 
        FOREIGN KEY (batch_id) REFERENCES inventory_batches(id) 
        ON DELETE SET NULL ON UPDATE CASCADE;
    END IF;
END $$;

-- 添加索引
CREATE INDEX IF NOT EXISTS idx_cost_collections_batch_id ON cost_collections(batch_id) WHERE batch_id IS NOT NULL;

-- ========================================================
-- DB-004: 质量检验记录增强关联
-- ========================================================

-- 质量检验记录已有 related_type 和 related_id 字段
-- 添加联合索引优化查询
CREATE INDEX IF NOT EXISTS idx_quality_inspection_related 
ON quality_inspection_records(related_type, related_id) 
WHERE related_type IS NOT NULL AND related_id IS NOT NULL;

-- 添加采购入库单ID字段（如果检验类型为来料检验）
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'quality_inspection_records' AND column_name = 'purchase_receipt_id'
    ) THEN
        ALTER TABLE quality_inspection_records ADD COLUMN purchase_receipt_id INTEGER;
    END IF;
END $$;

-- 添加外键约束
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.table_constraints 
        WHERE constraint_name = 'fk_quality_inspection_receipt'
    ) THEN
        ALTER TABLE quality_inspection_records 
        ADD CONSTRAINT fk_quality_inspection_receipt 
        FOREIGN KEY (purchase_receipt_id) REFERENCES purchase_receipts(id) 
        ON DELETE SET NULL ON UPDATE CASCADE;
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_quality_inspection_receipt_id 
ON quality_inspection_records(purchase_receipt_id) 
WHERE purchase_receipt_id IS NOT NULL;

-- ========================================================
-- 额外的性能优化索引
-- ========================================================

-- 销售订单客户外键索引
CREATE INDEX IF NOT EXISTS idx_sales_orders_customer_id ON sales_orders(customer_id);

-- 采购订单供应商外键索引  
CREATE INDEX IF NOT EXISTS idx_purchase_orders_supplier_id ON purchase_orders(supplier_id);

-- 库存产品外键索引
CREATE INDEX IF NOT EXISTS idx_inventory_stock_product_id ON inventory_stock(product_id);

-- 库存仓库外键索引
CREATE INDEX IF NOT EXISTS idx_inventory_stock_warehouse_id ON inventory_stock(warehouse_id);

COMMIT;

-- 验证迁移
-- SELECT 'Migration 003_foreign_keys completed successfully' AS status;
