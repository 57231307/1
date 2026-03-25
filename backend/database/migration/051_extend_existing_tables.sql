-- ========================================
-- 秉羲 ERP 系统 - 扩展现有数据库表
-- 版本：2026-03-16
-- 模块：四级批次管理扩展
-- 说明：扩展产品、色号、销售订单、采购订单表以支持四级批次管理
-- ========================================

-- ========================================
-- 1. 扩展产品表（products）
-- ========================================

-- 添加供应商相关字段
ALTER TABLE products 
ADD COLUMN supplier_product_code VARCHAR(100),           -- 供应商成品编码
ADD COLUMN supplier_id INTEGER,                          -- 供应商 ID
ADD COLUMN is_batch_managed BOOLEAN DEFAULT TRUE,        -- 是否启用批次管理
ADD COLUMN batch_level VARCHAR(20) DEFAULT 'four_level', -- 批次级别（two_level/three_level/four_level）
ADD CONSTRAINT fk_products_supplier
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id);

-- 添加注释
COMMENT ON COLUMN products.supplier_product_code IS '供应商成品编码';
COMMENT ON COLUMN products.supplier_id IS '供应商 ID';
COMMENT ON COLUMN products.is_batch_managed IS '是否启用批次管理';
COMMENT ON COLUMN products.batch_level IS '批次级别（two_level/three_level/four_level）';

-- 创建索引
CREATE INDEX idx_products_supplier ON products(supplier_id);
CREATE INDEX idx_products_batch_managed ON products(is_batch_managed);

-- ========================================
-- 2. 扩展产品色号表（product_colors）
-- ========================================

-- 添加供应商色号相关字段
ALTER TABLE product_colors
ADD COLUMN supplier_color_code VARCHAR(100),             -- 供应商色号
ADD COLUMN color_difference_notes TEXT,                  -- 色差说明
ADD COLUMN is_active BOOLEAN DEFAULT TRUE;               -- 是否启用

-- 添加注释
COMMENT ON COLUMN product_colors.supplier_color_code IS '供应商色号';
COMMENT ON COLUMN product_colors.color_difference_notes IS '色差说明';
COMMENT ON COLUMN product_colors.is_active IS '是否启用';

-- 创建索引
CREATE INDEX idx_product_colors_supplier_color ON product_colors(supplier_color_code);
CREATE INDEX idx_product_colors_active ON product_colors(is_active);

-- ========================================
-- 3. 扩展销售订单明细表（sales_order_items）
-- ========================================

-- 查看现有表结构（确认表名）
-- 注意：根据更正，销售订单只需要成品 + 色号，不需要缸号/匹号
-- 这里添加批次管理相关字段，但缸号/匹号在发货单中才使用

-- 添加批次管理字段（销售订单只需要到色号级别）
ALTER TABLE sales_order_items
ADD COLUMN batch_required BOOLEAN DEFAULT FALSE,         -- 是否需要批次管理
ADD COLUMN allocated_dye_lot_ids INTEGER[],              -- 已分配的缸号 ID 列表（用于预留）
ADD COLUMN allocated_piece_ids INTEGER[],                -- 已分配的匹号 ID 列表（用于预留）
ADD COLUMN delivery_batch_info JSONB;                    -- 发货批次信息（JSON 格式，发货时填充）

-- 添加注释
COMMENT ON COLUMN sales_order_items.batch_required IS '是否需要批次管理';
COMMENT ON COLUMN sales_order_items.allocated_dye_lot_ids IS '已分配的缸号 ID 列表（用于预留）';
COMMENT ON COLUMN sales_order_items.allocated_piece_ids IS '已分配的匹号 ID 列表（用于预留）';
COMMENT ON COLUMN sales_order_items.delivery_batch_info IS '发货批次信息（JSON 格式，发货时填充）';

-- ========================================
-- 4. 扩展采购订单明细表（purchase_order_item）
-- ========================================

-- 注意：根据更正，采购订单只需要供应商成品 + 色号，不需要缸号/匹号
-- 缸号/匹号在采购收货单中才使用

-- 添加批次管理字段（采购订单只需要到色号级别）
ALTER TABLE purchase_order_item
ADD COLUMN batch_required BOOLEAN DEFAULT FALSE,         -- 是否需要批次管理
ADD COLUMN expected_dye_lot_info TEXT,                   -- 预计缸号信息（仅供参考）
ADD COLUMN receipt_batch_info JSONB;                     -- 收货批次信息（JSON 格式，收货时填充）

-- 添加注释
COMMENT ON COLUMN purchase_order_item.batch_required IS '是否需要批次管理';
COMMENT ON COLUMN purchase_order_item.expected_dye_lot_info IS '预计缸号信息（仅供参考）';
COMMENT ON COLUMN purchase_order_item.receipt_batch_info IS '收货批次信息（JSON 格式，收货时填充）';

-- ========================================
-- 5. 扩展示销售发货单表（新增）
-- ========================================

-- 如果销售发货单表不存在，先创建主表
-- 注意：这里假设已有 sales_deliveries 表，如果没有需要创建

-- 添加四级批次字段到销售发货明细
-- 由于原表可能不存在，这里提供完整的表创建脚本

CREATE TABLE IF NOT EXISTS sales_delivery (
    id SERIAL PRIMARY KEY,
    delivery_no VARCHAR(50) NOT NULL UNIQUE,             -- 发货单号
    sales_order_id INTEGER NOT NULL,                     -- 销售订单 ID
    customer_id INTEGER NOT NULL,                        -- 客户 ID
    delivery_date DATE NOT NULL,                         -- 发货日期
    warehouse_id INTEGER NOT NULL,                       -- 仓库 ID
    total_quantity DECIMAL(10,2) DEFAULT 0.00,           -- 总数量
    total_amount DECIMAL(10,2) DEFAULT 0.00,             -- 总金额
    delivery_status VARCHAR(20) DEFAULT 'draft',         -- 发货状态
    shipped_at TIMESTAMP,                                -- 发货时间
    confirmed_at TIMESTAMP,                              -- 确认时间
    notes TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,
    updated_by INTEGER
);

COMMENT ON TABLE sales_delivery IS '销售发货单表';

-- 外键约束
ALTER TABLE sales_delivery ADD CONSTRAINT fk_sd_sales_order
    FOREIGN KEY (sales_order_id) REFERENCES sales_orders(id);
ALTER TABLE sales_delivery ADD CONSTRAINT fk_sd_customer
    FOREIGN KEY (customer_id) REFERENCES customers(id);
ALTER TABLE sales_delivery ADD CONSTRAINT fk_sd_warehouse
    FOREIGN KEY (warehouse_id) REFERENCES warehouses(id);
ALTER TABLE sales_delivery ADD CONSTRAINT fk_sd_created_by
    FOREIGN KEY (created_by) REFERENCES users(id);
ALTER TABLE sales_delivery ADD CONSTRAINT fk_sd_updated_by
    FOREIGN KEY (updated_by) REFERENCES users(id);

-- 索引
CREATE INDEX idx_sd_delivery_no ON sales_delivery(delivery_no);
CREATE INDEX idx_sd_sales_order ON sales_delivery(sales_order_id);
CREATE INDEX idx_sd_customer ON sales_delivery(customer_id);
CREATE INDEX idx_sd_delivery_date ON sales_delivery(delivery_date);
CREATE INDEX idx_sd_delivery_status ON sales_delivery(delivery_status);

-- ========================================
-- 6. 扩展示销售发货明细表（新增）
-- ========================================

-- 销售发货明细表（包含四级批次信息）
CREATE TABLE IF NOT EXISTS sales_delivery_item (
    id SERIAL PRIMARY KEY,
    delivery_id INTEGER NOT NULL,                        -- 发货单 ID
    sales_order_item_id INTEGER,                         -- 销售订单明细 ID
    line_no INTEGER NOT NULL,                            -- 行号
    product_id INTEGER NOT NULL,                         -- 成品 ID
    product_code VARCHAR(50) NOT NULL,                   -- 成品编码
    product_name VARCHAR(200) NOT NULL,                  -- 成品名称
    color_id INTEGER NOT NULL,                           -- 色号 ID
    color_no VARCHAR(50) NOT NULL,                       -- 色号
    dye_lot_id INTEGER NOT NULL,                         -- 缸号 ID ✅
    dye_lot_no VARCHAR(100) NOT NULL,                    -- 缸号 ✅
    piece_ids INTEGER[] NOT NULL,                        -- 匹号 ID 列表 ✅
    piece_nos VARCHAR(100)[] NOT NULL,                   -- 匹号列表 ✅
    quantity DECIMAL(10,2) NOT NULL,                     -- 数量
    unit_price DECIMAL(10,2),                            -- 单价
    amount DECIMAL(10,2),                                -- 金额
    warehouse_id INTEGER,                                -- 仓库 ID
    location_code VARCHAR(50),                           -- 库位编码
    notes TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE sales_delivery_item IS '销售发货明细表（含四级批次）';
COMMENT ON COLUMN sales_delivery_item.dye_lot_id IS '缸号 ID';
COMMENT ON COLUMN sales_delivery_item.dye_lot_no IS '缸号';
COMMENT ON COLUMN sales_delivery_item.piece_ids IS '匹号 ID 列表';
COMMENT ON COLUMN sales_delivery_item.piece_nos IS '匹号列表';

-- 外键约束
ALTER TABLE sales_delivery_item ADD CONSTRAINT fk_sdi_delivery
    FOREIGN KEY (delivery_id) REFERENCES sales_delivery(id) ON DELETE CASCADE;
ALTER TABLE sales_delivery_item ADD CONSTRAINT fk_sdi_sales_order_item
    FOREIGN KEY (sales_order_item_id) REFERENCES sales_order_items(id);
ALTER TABLE sales_delivery_item ADD CONSTRAINT fk_sdi_product
    FOREIGN KEY (product_id) REFERENCES products(id);
ALTER TABLE sales_delivery_item ADD CONSTRAINT fk_sdi_color
    FOREIGN KEY (color_id) REFERENCES product_colors(id);
ALTER TABLE sales_delivery_item ADD CONSTRAINT fk_sdi_dye_lot
    FOREIGN KEY (dye_lot_id) REFERENCES batch_dye_lot(id);

-- 唯一约束
ALTER TABLE sales_delivery_item ADD CONSTRAINT uk_sdi_delivery_line
    UNIQUE (delivery_id, line_no);

-- 索引
CREATE INDEX idx_sdi_delivery ON sales_delivery_item(delivery_id);
CREATE INDEX idx_sdi_product ON sales_delivery_item(product_id);
CREATE INDEX idx_sdi_color ON sales_delivery_item(color_id);
CREATE INDEX idx_sdi_dye_lot ON sales_delivery_item(dye_lot_id);
CREATE INDEX idx_sdi_piece_ids ON sales_delivery_item USING GIN (piece_ids);

-- ========================================
-- 7. 扩展采购收货单表（已存在，添加批次字段）
-- ========================================

-- 采购收货单主表已存在，这里添加批次相关字段
ALTER TABLE purchase_receipt
ADD COLUMN has_batch_info BOOLEAN DEFAULT FALSE,         -- 是否有批次信息
ADD COLUMN batch_validation_status VARCHAR(20) DEFAULT 'pending'; -- 批次验证状态

COMMENT ON COLUMN purchase_receipt.has_batch_info IS '是否有批次信息';
COMMENT ON COLUMN purchase_receipt.batch_validation_status IS '批次验证状态（pending/validated/failed）';

CREATE INDEX idx_pr_has_batch ON purchase_receipt(has_batch_info);

-- ========================================
-- 8. 扩展采购收货明细表（添加四级批次字段）
-- ========================================

-- 采购收货明细表已存在，添加四级批次字段
ALTER TABLE purchase_receipt_item
ADD COLUMN internal_dye_lot_id INTEGER,                  -- 内部缸号 ID
ADD COLUMN internal_dye_lot_no VARCHAR(100),             -- 内部缸号
ADD COLUMN internal_piece_ids INTEGER[],                 -- 内部匹号 ID 列表
ADD COLUMN internal_piece_nos VARCHAR(100)[],            -- 内部匹号列表
ADD COLUMN supplier_dye_lot_no VARCHAR(100),             -- 供应商缸号
ADD COLUMN supplier_piece_nos VARCHAR(100)[],            -- 供应商匹号列表
ADD COLUMN batch_conversion_log_id INTEGER;              -- 批次转换日志 ID

-- 添加注释
COMMENT ON COLUMN purchase_receipt_item.internal_dye_lot_id IS '内部缸号 ID';
COMMENT ON COLUMN purchase_receipt_item.internal_dye_lot_no IS '内部缸号';
COMMENT ON COLUMN purchase_receipt_item.internal_piece_ids IS '内部匹号 ID 列表';
COMMENT ON COLUMN purchase_receipt_item.internal_piece_nos IS '内部匹号列表';
COMMENT ON COLUMN purchase_receipt_item.supplier_dye_lot_no IS '供应商缸号';
COMMENT ON COLUMN purchase_receipt_item.supplier_piece_nos IS '供应商匹号列表';
COMMENT ON COLUMN purchase_receipt_item.batch_conversion_log_id IS '批次转换日志 ID';

-- 外键约束
ALTER TABLE purchase_receipt_item ADD CONSTRAINT fk_pri_internal_dye_lot
    FOREIGN KEY (internal_dye_lot_id) REFERENCES batch_dye_lot(id);
ALTER TABLE purchase_receipt_item ADD CONSTRAINT fk_pri_conversion_log
    FOREIGN KEY (batch_conversion_log_id) REFERENCES batch_trace_log(id);

-- 索引
CREATE INDEX idx_pri_internal_dye_lot ON purchase_receipt_item(internal_dye_lot_id);
CREATE INDEX idx_pri_internal_piece_ids ON purchase_receipt_item USING GIN (internal_piece_ids);
CREATE INDEX idx_pri_supplier_dye_lot ON purchase_receipt_item(supplier_dye_lot_no);
CREATE INDEX idx_pri_supplier_piece_nos ON purchase_receipt_item USING GIN (supplier_piece_nos);

-- ========================================
-- 9. 更新触发器
-- ========================================

-- 更新销售发货单的 updated_at
CREATE TRIGGER trg_sales_delivery_updated_at
    BEFORE UPDATE ON sales_delivery
    FOR EACH ROW
    EXECUTE FUNCTION update_batch_updated_at_column();

-- ========================================
-- 10. 数据完整性约束
-- ========================================

-- 添加检查约束，确保销售订单明细的批次字段正确
ALTER TABLE sales_order_items
ADD CONSTRAINT chk_soi_batch_allocated
    CHECK (
        (batch_required = FALSE AND allocated_dye_lot_ids IS NULL)
        OR 
        (batch_required = TRUE)
    );

-- 添加检查约束，确保采购订单明细的批次字段正确
ALTER TABLE purchase_order_item
ADD CONSTRAINT chk_poi_batch_required
    CHECK (
        (batch_required = FALSE AND expected_dye_lot_info IS NULL)
        OR 
        (batch_required = TRUE)
    );

-- ========================================
-- 迁移完成
-- ========================================
