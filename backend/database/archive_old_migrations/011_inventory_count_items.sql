-- ============================================
-- 库存盘点明细表 - 补充表
-- ============================================

DROP TABLE IF NOT EXISTS inventory_count_items CASCADE;
CREATE TABLE IF NOT EXISTS inventory_count_items (
    id SERIAL PRIMARY KEY,
    count_id INTEGER NOT NULL REFERENCES inventory_counts(id) ON DELETE CASCADE,
    stock_id INTEGER NOT NULL REFERENCES inventory_stocks(id) ON DELETE CASCADE,
    product_id INTEGER NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    warehouse_id INTEGER NOT NULL REFERENCES warehouses(id) ON DELETE CASCADE,
    quantity_before DECIMAL(10,2) NOT NULL DEFAULT 0,  -- 盘点前数量
    quantity_actual DECIMAL(10,2) NOT NULL DEFAULT 0,  -- 实际盘点数量
    quantity_difference DECIMAL(10,2) NOT NULL DEFAULT 0,  -- 差异数量（实际 - 账面）
    unit_cost DECIMAL(12,2) DEFAULT 0,  -- 单位成本
    total_cost DECIMAL(12,2) DEFAULT 0,  -- 总成本差异
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_inventory_count_items_count_id ON inventory_count_items(count_id);
CREATE INDEX IF NOT EXISTS idx_inventory_count_items_stock_id ON inventory_count_items(stock_id);
CREATE INDEX IF NOT EXISTS idx_inventory_count_items_product_id ON inventory_count_items(product_id);
CREATE INDEX IF NOT EXISTS idx_inventory_count_items_warehouse_id ON inventory_count_items(warehouse_id);

-- 添加注释
COMMENT ON TABLE inventory_count_items IS '库存盘点明细表';
COMMENT ON COLUMN inventory_count_items.id IS '明细 ID';
COMMENT ON COLUMN inventory_count_items.count_id IS '盘点单 ID';
COMMENT ON COLUMN inventory_count_items.stock_id IS '库存 ID';
COMMENT ON COLUMN inventory_count_items.product_id IS '产品 ID';
COMMENT ON COLUMN inventory_count_items.warehouse_id IS '仓库 ID';
COMMENT ON COLUMN inventory_count_items.quantity_before IS '盘点前数量（账面数量）';
COMMENT ON COLUMN inventory_count_items.quantity_actual IS '实际盘点数量';
COMMENT ON COLUMN inventory_count_items.quantity_difference IS '差异数量（实际 - 账面）';
COMMENT ON COLUMN inventory_count_items.unit_cost IS '单位成本';
COMMENT ON COLUMN inventory_count_items.total_cost IS '总成本差异';
COMMENT ON COLUMN inventory_count_items.notes IS '备注';
COMMENT ON COLUMN inventory_count_items.created_at IS '创建时间';
COMMENT ON COLUMN inventory_count_items.updated_at IS '更新时间';

-- ============================================
-- 更新 inventory_counts 表，添加差异汇总字段
-- ============================================

ALTER TABLE inventory_counts 
ADD COLUMN IF NOT EXISTS total_quantity_before DECIMAL(10,2) DEFAULT 0,
ADD COLUMN IF NOT EXISTS total_quantity_actual DECIMAL(10,2) DEFAULT 0,
ADD COLUMN IF NOT EXISTS total_quantity_difference DECIMAL(10,2) DEFAULT 0,
ADD COLUMN IF NOT EXISTS total_cost_difference DECIMAL(12,2) DEFAULT 0;

-- 添加注释
COMMENT ON COLUMN inventory_counts.total_quantity_before IS '盘点前总数量';
COMMENT ON COLUMN inventory_counts.total_quantity_actual IS '实际盘点总数量';
COMMENT ON COLUMN inventory_counts.total_quantity_difference IS '总差异数量';
COMMENT ON COLUMN inventory_counts.total_cost_difference IS '总成本差异';
