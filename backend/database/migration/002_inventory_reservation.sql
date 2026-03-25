-- ============================================
-- 库存预留表 - 用于销售订单锁定库存
-- ============================================

CREATE TABLE IF NOT EXISTS inventory_reservations (
    id SERIAL PRIMARY KEY,
    order_id INTEGER NOT NULL REFERENCES sales_orders(id) ON DELETE CASCADE,
    product_id INTEGER NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    warehouse_id INTEGER NOT NULL REFERENCES warehouses(id) ON DELETE CASCADE,
    quantity DECIMAL(10,2) NOT NULL DEFAULT 0,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',  -- pending-待处理，locked-已锁定，released-已释放，used-已使用
    reserved_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    released_at TIMESTAMPTZ,
    notes TEXT,
    created_by INTEGER REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 创建索引
CREATE INDEX idx_inventory_reservations_order_id ON inventory_reservations(order_id);
CREATE INDEX idx_inventory_reservations_product_id ON inventory_reservations(product_id);
CREATE INDEX idx_inventory_reservations_warehouse_id ON inventory_reservations(warehouse_id);
CREATE INDEX idx_inventory_reservations_status ON inventory_reservations(status);
CREATE INDEX idx_inventory_reservations_reserved_at ON inventory_reservations(reserved_at);

-- 添加注释
COMMENT ON TABLE inventory_reservations IS '库存预留表 - 用于销售订单锁定库存';
COMMENT ON COLUMN inventory_reservations.id IS '预留 ID';
COMMENT ON COLUMN inventory_reservations.order_id IS '销售订单 ID';
COMMENT ON COLUMN inventory_reservations.product_id IS '产品 ID';
COMMENT ON COLUMN inventory_reservations.warehouse_id IS '仓库 ID';
COMMENT ON COLUMN inventory_reservations.quantity IS '预留数量';
COMMENT ON COLUMN inventory_reservations.status IS '预留状态：pending-待处理，locked-已锁定，released-已释放，used-已使用';
COMMENT ON COLUMN inventory_reservations.reserved_at IS '预留时间';
COMMENT ON COLUMN inventory_reservations.released_at IS '释放时间';
COMMENT ON COLUMN inventory_reservations.notes IS '备注';
COMMENT ON COLUMN inventory_reservations.created_by IS '创建人';
COMMENT ON COLUMN inventory_reservations.created_at IS '创建时间';
COMMENT ON COLUMN inventory_reservations.updated_at IS '更新时间';
