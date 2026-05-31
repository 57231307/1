-- 修复 inventory_stocks 表缺少的列
-- 根据 Rust 模型定义补充缺失字段

DO $$ 
BEGIN
    -- stock_status 列
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_stocks' AND column_name = 'stock_status') THEN
        ALTER TABLE inventory_stocks ADD COLUMN stock_status VARCHAR(20) NOT NULL DEFAULT '正常';
        COMMENT ON COLUMN inventory_stocks.stock_status IS '库存状态：正常/冻结/待检';
    END IF;

    -- quality_status 列
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_stocks' AND column_name = 'quality_status') THEN
        ALTER TABLE inventory_stocks ADD COLUMN quality_status VARCHAR(20) NOT NULL DEFAULT '合格';
        COMMENT ON COLUMN inventory_stocks.quality_status IS '质量状态：合格/不合格/待检';
    END IF;

    -- version 列（乐观锁）
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_stocks' AND column_name = 'version') THEN
        ALTER TABLE inventory_stocks ADD COLUMN version INTEGER NOT NULL DEFAULT 1;
        COMMENT ON COLUMN inventory_stocks.version IS '乐观锁版本号';
    END IF;

    -- layer_no 列
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_stocks' AND column_name = 'layer_no') THEN
        ALTER TABLE inventory_stocks ADD COLUMN layer_no VARCHAR(50);
        COMMENT ON COLUMN inventory_stocks.layer_no IS '层号';
    END IF;
END $$;

-- 修复 sales_orders 表缺少的列
DO $$ 
BEGIN
    -- opportunity_id 列（CRM商机关联）
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'sales_orders' AND column_name = 'opportunity_id') THEN
        ALTER TABLE sales_orders ADD COLUMN opportunity_id INTEGER;
        COMMENT ON COLUMN sales_orders.opportunity_id IS 'CRM商机ID';
    END IF;
END $$;

-- 为 inventory_stocks 添加状态索引
CREATE INDEX IF NOT EXISTS idx_inventory_stocks_status ON inventory_stocks(stock_status, quality_status);

COMMENT ON TABLE inventory_stocks IS '库存表 - 已补充缺失字段';
