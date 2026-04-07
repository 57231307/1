-- 秉羲管理系统 - 库存盘点表迁移脚本
-- 数据库类型：PostgreSQL 18.0
-- 创建日期：2026-03-15
-- 说明：此脚本用于创建库存盘点相关表结构

-- ==================== 库存盘点表 ====================
-- 存储仓库库存盘点记录
DROP TABLE IF EXISTS inventory_counts CASCADE;
CREATE TABLE IF NOT EXISTS inventory_counts (
    id SERIAL PRIMARY KEY,                              -- 盘点 ID（主键）
    count_no VARCHAR(50) NOT NULL UNIQUE,               -- 盘点单号（唯一）
    warehouse_id INTEGER NOT NULL REFERENCES warehouses(id),  -- 仓库 ID
    count_date TIMESTAMPTZ NOT NULL,                    -- 盘点日期
    status VARCHAR(20) NOT NULL,                        -- 状态：pending-待审核，approved-已审核，rejected-已驳回，completed-已完成
    total_items INTEGER NOT NULL DEFAULT 0,             -- 总盘点项数
    counted_items INTEGER NOT NULL DEFAULT 0,           -- 已盘点项数
    variance_items INTEGER NOT NULL DEFAULT 0,          -- 差异项数
    notes TEXT,                                         -- 备注
    created_by INTEGER,                                 -- 创建人
    approved_by INTEGER,                                -- 审批人
    approved_at TIMESTAMPTZ,                            -- 审批时间
    completed_at TIMESTAMPTZ,                           -- 完成时间
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,  -- 创建时间
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP   -- 更新时间
);

-- 为盘点单号创建索引（高频查询字段）
CREATE INDEX IF NOT EXISTS idx_inventory_counts_count_no ON inventory_counts(count_no);
-- 为仓库 ID 创建索引
CREATE INDEX IF NOT EXISTS idx_inventory_counts_warehouse ON inventory_counts(warehouse_id);
-- 为状态创建索引
CREATE INDEX IF NOT EXISTS idx_inventory_counts_status ON inventory_counts(status);
-- 为盘点日期创建索引
CREATE INDEX IF NOT EXISTS idx_inventory_counts_count_date ON inventory_counts(count_date);

-- 添加表注释
COMMENT ON TABLE inventory_counts IS '库存盘点单表';
COMMENT ON COLUMN inventory_counts.id IS '盘点 ID';
COMMENT ON COLUMN inventory_counts.count_no IS '盘点单号';
COMMENT ON COLUMN inventory_counts.warehouse_id IS '仓库 ID';
COMMENT ON COLUMN inventory_counts.count_date IS '盘点日期';
COMMENT ON COLUMN inventory_counts.status IS '盘点单状态';
COMMENT ON COLUMN inventory_counts.total_items IS '总盘点项数';
COMMENT ON COLUMN inventory_counts.counted_items IS '已盘点项数';
COMMENT ON COLUMN inventory_counts.variance_items IS '差异项数';
COMMENT ON COLUMN inventory_counts.notes IS '备注';
COMMENT ON COLUMN inventory_counts.created_by IS '创建人';
COMMENT ON COLUMN inventory_counts.approved_by IS '审批人';
COMMENT ON COLUMN inventory_counts.approved_at IS '审批时间';
COMMENT ON COLUMN inventory_counts.completed_at IS '完成时间';
COMMENT ON COLUMN inventory_counts.created_at IS '创建时间';
COMMENT ON COLUMN inventory_counts.updated_at IS '更新时间';

-- ==================== 库存盘点明细表 ====================
-- 存储库存盘点单的明细项
DROP TABLE IF EXISTS inventory_count_items CASCADE;
CREATE TABLE IF NOT EXISTS inventory_count_items (
    id SERIAL PRIMARY KEY,                              -- 明细 ID（主键）
    count_id INTEGER NOT NULL REFERENCES inventory_counts(id) ON DELETE CASCADE,  -- 盘点单 ID
    product_id INTEGER NOT NULL REFERENCES products(id),  -- 产品 ID
    bin_location VARCHAR(50),                           -- 库位
    quantity_book DECIMAL(12,2) NOT NULL,               -- 账面数量
    quantity_actual DECIMAL(12,2) NOT NULL,             -- 实际数量
    quantity_variance DECIMAL(12,2) NOT NULL,           -- 差异数量
    unit_cost DECIMAL(12,2),                            -- 单位成本
    variance_amount DECIMAL(12,2),                      -- 差异金额
    notes TEXT,                                         -- 备注
    counted_by INTEGER,                                 -- 盘点人
    counted_at TIMESTAMPTZ,                             -- 盘点时间
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,  -- 创建时间
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP   -- 更新时间
);

-- 为盘点单 ID 创建索引（高频查询字段）
CREATE INDEX IF NOT EXISTS idx_count_items_count_id ON inventory_count_items(count_id);
-- 为产品 ID 创建索引
CREATE INDEX IF NOT EXISTS idx_count_items_product_id ON inventory_count_items(product_id);

-- 添加表注释
COMMENT ON TABLE inventory_count_items IS '库存盘点明细表';
COMMENT ON COLUMN inventory_count_items.id IS '明细 ID';
COMMENT ON COLUMN inventory_count_items.count_id IS '盘点单 ID';
COMMENT ON COLUMN inventory_count_items.product_id IS '产品 ID';
COMMENT ON COLUMN inventory_count_items.bin_location IS '库位';
COMMENT ON COLUMN inventory_count_items.quantity_book IS '账面数量';
COMMENT ON COLUMN inventory_count_items.quantity_actual IS '实际数量';
COMMENT ON COLUMN inventory_count_items.quantity_variance IS '差异数量';
COMMENT ON COLUMN inventory_count_items.unit_cost IS '单位成本';
COMMENT ON COLUMN inventory_count_items.variance_amount IS '差异金额';
COMMENT ON COLUMN inventory_count_items.notes IS '备注';
COMMENT ON COLUMN inventory_count_items.counted_by IS '盘点人';
COMMENT ON COLUMN inventory_count_items.counted_at IS '盘点时间';
COMMENT ON COLUMN inventory_count_items.created_at IS '创建时间';
COMMENT ON COLUMN inventory_count_items.updated_at IS '更新时间';

-- ==================== 触发器：自动更新时间 ====================
-- 为 inventory_counts 表创建触发器
DROP TRIGGER IF EXISTS update_inventory_counts_updated_at ON inventory_counts;
CREATE TRIGGER update_inventory_counts_updated_at BEFORE UPDATE ON inventory_counts
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- 为 inventory_count_items 表创建触发器
DROP TRIGGER IF EXISTS update_inventory_count_items_updated_at ON inventory_count_items;
CREATE TRIGGER update_inventory_count_items_updated_at BEFORE UPDATE ON inventory_count_items
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ==================== 完成提示 ====================
DO $$
BEGIN
    RAISE NOTICE '库存盘点表创建完成！';
END $$;
