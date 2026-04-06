-- 秉羲管理系统 - 库存调拨表迁移脚本
-- 数据库类型：PostgreSQL 18.0
-- 创建日期：2026-03-15
-- 说明：此脚本用于创建库存调拨相关表结构

-- ==================== 库存调拨表 ====================
-- 存储仓库之间的库存调拨记录
CREATE TABLE IF NOT EXISTS inventory_transfers (
    id SERIAL PRIMARY KEY,                              -- 调拨 ID（主键）
    transfer_no VARCHAR(50) NOT NULL UNIQUE,            -- 调拨单号（唯一）
    from_warehouse_id INTEGER NOT NULL REFERENCES warehouses(id),  -- 源仓库 ID
    to_warehouse_id INTEGER NOT NULL REFERENCES warehouses(id),    -- 目标仓库 ID
    transfer_date TIMESTAMPTZ NOT NULL,                 -- 调拨日期
    status VARCHAR(20) NOT NULL,                        -- 状态：pending-待审核，approved-已审核，rejected-已驳回，shipped-已发出，completed-已完成
    total_quantity DECIMAL(12,2) NOT NULL,              -- 总数量
    notes TEXT,                                         -- 备注
    created_by INTEGER,                                 -- 创建人
    approved_by INTEGER,                                -- 审批人
    approved_at TIMESTAMPTZ,                            -- 审批时间
    shipped_at TIMESTAMPTZ,                             -- 发出时间
    received_at TIMESTAMPTZ,                            -- 接收时间
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,  -- 创建时间
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP   -- 更新时间
);

-- 为调拨单号创建索引（高频查询字段）
CREATE INDEX IF NOT EXISTS idx_inventory_transfers_transfer_no ON inventory_transfers(transfer_no);
-- 为源仓库 ID 创建索引
CREATE INDEX IF NOT EXISTS idx_inventory_transfers_from_warehouse ON inventory_transfers(from_warehouse_id);
-- 为目标仓库 ID 创建索引
CREATE INDEX IF NOT EXISTS idx_inventory_transfers_to_warehouse ON inventory_transfers(to_warehouse_id);
-- 为状态创建索引
CREATE INDEX IF NOT EXISTS idx_inventory_transfers_status ON inventory_transfers(status);
-- 为调拨日期创建索引
CREATE INDEX IF NOT EXISTS idx_inventory_transfers_transfer_date ON inventory_transfers(transfer_date);

-- 添加表注释
COMMENT ON TABLE inventory_transfers IS '库存调拨单表';
COMMENT ON COLUMN inventory_transfers.id IS '调拨 ID';
COMMENT ON COLUMN inventory_transfers.transfer_no IS '调拨单号';
COMMENT ON COLUMN inventory_transfers.from_warehouse_id IS '源仓库 ID';
COMMENT ON COLUMN inventory_transfers.to_warehouse_id IS '目标仓库 ID';
COMMENT ON COLUMN inventory_transfers.transfer_date IS '调拨日期';
COMMENT ON COLUMN inventory_transfers.status IS '调拨单状态';
COMMENT ON COLUMN inventory_transfers.total_quantity IS '总数量';
COMMENT ON COLUMN inventory_transfers.notes IS '备注';
COMMENT ON COLUMN inventory_transfers.created_by IS '创建人';
COMMENT ON COLUMN inventory_transfers.approved_by IS '审批人';
COMMENT ON COLUMN inventory_transfers.approved_at IS '审批时间';
COMMENT ON COLUMN inventory_transfers.shipped_at IS '发出时间';
COMMENT ON COLUMN inventory_transfers.received_at IS '接收时间';
COMMENT ON COLUMN inventory_transfers.created_at IS '创建时间';
COMMENT ON COLUMN inventory_transfers.updated_at IS '更新时间';

-- ==================== 库存调拨明细表 ====================
-- 存储库存调拨单的明细项
CREATE TABLE IF NOT EXISTS inventory_transfer_items (
    id SERIAL PRIMARY KEY,                              -- 明细 ID（主键）
    transfer_id INTEGER NOT NULL REFERENCES inventory_transfers(id) ON DELETE CASCADE,  -- 调拨单 ID
    product_id INTEGER NOT NULL REFERENCES products(id),  -- 产品 ID
    quantity DECIMAL(12,2) NOT NULL,                    -- 调拨数量
    shipped_quantity DECIMAL(12,2) NOT NULL,            -- 已发出数量
    received_quantity DECIMAL(12,2) NOT NULL,           -- 已接收数量
    unit_cost DECIMAL(12,2),                            -- 单位成本
    notes TEXT,                                         -- 备注
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,  -- 创建时间
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP   -- 更新时间
);

-- 为调拨单 ID 创建索引（高频查询字段）
CREATE INDEX IF NOT EXISTS idx_transfer_items_transfer_id ON inventory_transfer_items(transfer_id);
-- 为产品 ID 创建索引
CREATE INDEX IF NOT EXISTS idx_transfer_items_product_id ON inventory_transfer_items(product_id);

-- 添加表注释
COMMENT ON TABLE inventory_transfer_items IS '库存调拨明细表';
COMMENT ON COLUMN inventory_transfer_items.id IS '明细 ID';
COMMENT ON COLUMN inventory_transfer_items.transfer_id IS '调拨单 ID';
COMMENT ON COLUMN inventory_transfer_items.product_id IS '产品 ID';
COMMENT ON COLUMN inventory_transfer_items.quantity IS '调拨数量';
COMMENT ON COLUMN inventory_transfer_items.shipped_quantity IS '已发出数量';
COMMENT ON COLUMN inventory_transfer_items.received_quantity IS '已接收数量';
COMMENT ON COLUMN inventory_transfer_items.unit_cost IS '单位成本';
COMMENT ON COLUMN inventory_transfer_items.notes IS '备注';
COMMENT ON COLUMN inventory_transfer_items.created_at IS '创建时间';
COMMENT ON COLUMN inventory_transfer_items.updated_at IS '更新时间';

-- ==================== 触发器：自动更新时间 ====================
-- 为 inventory_transfers 表创建触发器
CREATE TRIGGER update_inventory_transfers_updated_at BEFORE UPDATE ON inventory_transfers
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- 为 inventory_transfer_items 表创建触发器
CREATE TRIGGER update_inventory_transfer_items_updated_at BEFORE UPDATE ON inventory_transfer_items
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ==================== 完成提示 ====================
DO $$
BEGIN
    RAISE NOTICE '库存调拨表创建完成！';
END $$;
