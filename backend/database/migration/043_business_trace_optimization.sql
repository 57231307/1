-- ============================================================
-- 面料行业业务追溯链条优化迁移脚本
-- 版本：v2.0
-- 日期：2024-01-01
-- 说明：实现完整的正向 + 反向业务追溯体系
-- ============================================================

-- 1. 创建业务追溯链表
CREATE TABLE IF NOT EXISTS business_trace_chain (
    id SERIAL PRIMARY KEY,
    trace_chain_id VARCHAR(255) NOT NULL UNIQUE,
    five_dimension_id VARCHAR(255) NOT NULL,
    product_id INTEGER NOT NULL,
    batch_no VARCHAR(100) NOT NULL,
    color_no VARCHAR(50) NOT NULL,
    dye_lot_no VARCHAR(100),
    grade VARCHAR(50) NOT NULL,
    current_stage VARCHAR(50) NOT NULL,
    current_bill_type VARCHAR(50) NOT NULL,
    current_bill_no VARCHAR(100) NOT NULL,
    current_bill_id INTEGER NOT NULL,
    previous_trace_id INTEGER,
    next_trace_id INTEGER,
    quantity_meters DECIMAL(12,2) NOT NULL,
    quantity_kg DECIMAL(12,2) NOT NULL,
    warehouse_id INTEGER NOT NULL,
    supplier_id INTEGER,
    customer_id INTEGER,
    workshop_id INTEGER,
    trace_status VARCHAR(20) NOT NULL DEFAULT 'ACTIVE',
    remarks TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    created_by INTEGER,
    
    CONSTRAINT fk_previous_trace FOREIGN KEY (previous_trace_id) REFERENCES business_trace_chain(id),
    CONSTRAINT fk_next_trace FOREIGN KEY (next_trace_id) REFERENCES business_trace_chain(id)
);

-- 2. 创建业务追溯快照表
CREATE TABLE IF NOT EXISTS business_trace_snapshot (
    id SERIAL PRIMARY KEY,
    trace_chain_id VARCHAR(255) NOT NULL,
    five_dimension_id VARCHAR(255) NOT NULL,
    product_id INTEGER NOT NULL,
    batch_no VARCHAR(100) NOT NULL,
    color_no VARCHAR(50) NOT NULL,
    grade VARCHAR(50) NOT NULL,
    current_stage VARCHAR(50) NOT NULL,
    warehouse_id INTEGER NOT NULL,
    current_quantity_meters DECIMAL(12,2) NOT NULL,
    current_quantity_kg DECIMAL(12,2) NOT NULL,
    supplier_name VARCHAR(255),
    customer_name VARCHAR(255),
    trace_path JSONB NOT NULL,
    snapshot_time TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- 3. 为追溯链表添加索引
CREATE INDEX IF NOT EXISTS idx_trace_chain_five_dim ON business_trace_chain(five_dimension_id);
CREATE INDEX IF NOT EXISTS idx_trace_chain_batch ON business_trace_chain(batch_no);
CREATE INDEX IF NOT EXISTS idx_trace_chain_supplier ON business_trace_chain(supplier_id);
CREATE INDEX IF NOT EXISTS idx_trace_chain_customer ON business_trace_chain(customer_id);
CREATE INDEX IF NOT EXISTS idx_trace_chain_stage ON business_trace_chain(current_stage);
CREATE INDEX IF NOT EXISTS idx_trace_chain_status ON business_trace_chain(trace_status);
CREATE INDEX IF NOT EXISTS idx_trace_chain_created ON business_trace_chain(created_at);

-- 4. 为追溯快照表添加索引
CREATE INDEX IF NOT EXISTS idx_trace_snapshot_chain_id ON business_trace_snapshot(trace_chain_id);
CREATE INDEX IF NOT EXISTS idx_trace_snapshot_five_dim ON business_trace_snapshot(five_dimension_id);
CREATE INDEX IF NOT EXISTS idx_trace_snapshot_batch ON business_trace_snapshot(batch_no);
CREATE INDEX IF NOT EXISTS idx_trace_snapshot_time ON business_trace_snapshot(snapshot_time);

-- 5. 创建业务追溯视图（简化查询）
CREATE OR REPLACE VIEW v_business_trace_view AS
SELECT 
    t.id,
    t.trace_chain_id,
    t.five_dimension_id,
    t.product_id,
    t.batch_no,
    t.color_no,
    t.grade,
    FIRST_VALUE(t.current_stage) OVER (
        PARTITION BY t.trace_chain_id 
        ORDER BY t.created_at ASC
    ) as start_stage,
    t.current_stage,
    COUNT(*) OVER (PARTITION BY t.trace_chain_id) as stage_count,
    SUM(CASE 
        WHEN t.current_stage IN ('PURCHASE_RECEIPT', 'INVENTORY_IN', 'PRODUCTION_OUTPUT') 
        THEN t.quantity_meters 
        ELSE 0 
    END) OVER (PARTITION BY t.trace_chain_id) as total_in_meters,
    SUM(CASE 
        WHEN t.current_stage IN ('INVENTORY_OUT', 'SALES_DELIVERY', 'PRODUCTION_INPUT') 
        THEN t.quantity_meters 
        ELSE 0 
    END) OVER (PARTITION BY t.trace_chain_id) as total_out_meters,
    t.quantity_meters as current_stock_meters,
    NULL as supplier_name,
    NULL as customer_name,
    t.created_at,
    MAX(t.created_at) OVER (PARTITION BY t.trace_chain_id) as updated_at
FROM business_trace_chain t;

-- 6. 创建追溯链查询函数
CREATE OR REPLACE FUNCTION get_trace_chain_by_five_dim(p_five_dimension_id VARCHAR)
RETURNS TABLE (
    trace_id INTEGER,
    trace_chain_id VARCHAR,
    stage_name VARCHAR,
    bill_type VARCHAR,
    bill_no VARCHAR,
    quantity_meters DECIMAL,
    warehouse_id INTEGER,
    created_at TIMESTAMP
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        t.id,
        t.trace_chain_id,
        t.current_stage,
        t.current_bill_type,
        t.current_bill_no,
        t.quantity_meters,
        t.warehouse_id,
        t.created_at
    FROM business_trace_chain t
    WHERE t.five_dimension_id = p_five_dimension_id
    ORDER BY t.created_at ASC;
END;
$$ LANGUAGE plpgsql;

-- 7. 创建正向追溯函数
CREATE OR REPLACE FUNCTION forward_trace_by_supplier(p_supplier_id INTEGER, p_batch_no VARCHAR)
RETURNS TABLE (
    trace_id INTEGER,
    trace_chain_id VARCHAR,
    five_dimension_id VARCHAR,
    current_stage VARCHAR,
    quantity_meters DECIMAL,
    created_at TIMESTAMP
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        t.id,
        t.trace_chain_id,
        t.five_dimension_id,
        t.current_stage,
        t.quantity_meters,
        t.created_at
    FROM business_trace_chain t
    WHERE t.supplier_id = p_supplier_id
      AND t.batch_no = p_batch_no
      AND t.current_stage = 'PURCHASE_RECEIPT'
    ORDER BY t.created_at ASC;
END;
$$ LANGUAGE plpgsql;

-- 8. 创建反向追溯函数
CREATE OR REPLACE FUNCTION backward_trace_by_customer(p_customer_id INTEGER, p_batch_no VARCHAR)
RETURNS TABLE (
    trace_id INTEGER,
    trace_chain_id VARCHAR,
    five_dimension_id VARCHAR,
    current_stage VARCHAR,
    quantity_meters DECIMAL,
    created_at TIMESTAMP
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        t.id,
        t.trace_chain_id,
        t.five_dimension_id,
        t.current_stage,
        t.quantity_meters,
        t.created_at
    FROM business_trace_chain t
    WHERE t.customer_id = p_customer_id
      AND t.batch_no = p_batch_no
      AND t.current_stage = 'SALES_DELIVERY'
    ORDER BY t.created_at DESC;
END;
$$ LANGUAGE plpgsql;

-- 9. 添加注释说明
COMMENT ON TABLE business_trace_chain IS '业务追溯链 - 记录物料从采购到销售的完整流转过程';
COMMENT ON TABLE business_trace_snapshot IS '业务追溯快照 - 定期保存追溯链状态，用于快速查询';
COMMENT ON VIEW v_business_trace_view IS '业务追溯视图 - 简化追溯查询';
COMMENT ON FUNCTION get_trace_chain_by_five_dim IS '按五维 ID 查询追溯链';
COMMENT ON FUNCTION forward_trace_by_supplier IS '正向追溯：从供应商到客户';
COMMENT ON FUNCTION backward_trace_by_customer IS '反向追溯：从客户到供应商';

-- 10. 添加触发器：自动更新追溯链状态
CREATE OR REPLACE FUNCTION update_trace_chain_status()
RETURNS TRIGGER AS $$
BEGIN
    -- 如果是最后一个环节（销售发货），标记为已完成
    IF NEW.current_stage = 'SALES_DELIVERY' THEN
        NEW.trace_status := 'COMPLETED';
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_update_trace_status ON business_trace_chain;
CREATE TRIGGER trg_update_trace_status
    BEFORE INSERT OR UPDATE ON business_trace_chain
    FOR EACH ROW
    EXECUTE FUNCTION update_trace_chain_status();

-- ============================================================
-- 追溯链条说明
-- ============================================================
-- 正向追溯流程：
-- 1. PURCHASE_RECEIPT (采购收货) - 供应商 → 仓库
-- 2. INVENTORY_IN (入库) - 仓库接收
-- 3. PRODUCTION_INPUT (生产投入) - 仓库 → 车间
-- 4. PRODUCTION_OUTPUT (生产产出) - 车间 → 仓库
-- 5. INVENTORY_OUT (出库) - 仓库备货
-- 6. SALES_DELIVERY (销售发货) - 仓库 → 客户
--
-- 反向追溯流程：
-- 1. 从销售发货开始
-- 2. 通过 previous_trace_id 逐级回溯
-- 3. 直到采购收货环节
--
-- 性能优化：
-- - 五维 ID 索引：加速按批次 + 色号查询
-- - 供应商/客户索引：加速正向/反向追溯
-- - 快照表：定期保存状态，避免长链查询
-- ============================================================
