-- ========================================
-- 面料 ERP 系统 - 面料行业适配迁移脚本（阶段 1）
-- 版本：2026-03-15
-- 说明：产品、库存、总账模块面料行业适配
-- ========================================

-- ========================================
-- 1. 产品管理模块面料行业适配
-- ========================================

-- 1.1 产品表增加面料行业字段
ALTER TABLE products ADD COLUMN IF NOT EXISTS product_type VARCHAR(20) DEFAULT '成品布';
ALTER TABLE products ADD COLUMN IF NOT EXISTS fabric_composition VARCHAR(200);
ALTER TABLE products ADD COLUMN IF NOT EXISTS yarn_count VARCHAR(50);
ALTER TABLE products ADD COLUMN IF NOT EXISTS density VARCHAR(50);
ALTER TABLE products ADD COLUMN IF NOT EXISTS width DECIMAL(10,2);
ALTER TABLE products ADD COLUMN IF NOT EXISTS gram_weight DECIMAL(10,2);
ALTER TABLE products ADD COLUMN IF NOT EXISTS structure VARCHAR(50);
ALTER TABLE products ADD COLUMN IF NOT EXISTS finish VARCHAR(100);
ALTER TABLE products ADD COLUMN IF NOT EXISTS min_order_quantity DECIMAL(12,2);
ALTER TABLE products ADD COLUMN IF NOT EXISTS lead_time INTEGER;

COMMENT ON COLUMN products.product_type IS '产品类型：坯布/成品布/辅料';
COMMENT ON COLUMN products.fabric_composition IS '面料成分：如 65% 棉 35% 涤';
COMMENT ON COLUMN products.yarn_count IS '纱支：如 40S';
COMMENT ON COLUMN products.density IS '密度：如 133x72';
COMMENT ON COLUMN products.width IS '幅宽（cm）';
COMMENT ON COLUMN products.gram_weight IS '克重（g/m²）';
COMMENT ON COLUMN products.structure IS '组织结构：平纹/斜纹/缎纹';
COMMENT ON COLUMN products.finish IS '后整理：防水/防油/阻燃';
COMMENT ON COLUMN products.min_order_quantity IS '最小起订量（米）';
COMMENT ON COLUMN products.lead_time IS '交货期（天）';

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_products_product_type ON products(product_type);
CREATE INDEX IF NOT EXISTS idx_products_gram_weight ON products(gram_weight);

-- 1.2 创建产品色号表
CREATE TABLE IF NOT EXISTS product_colors (
    id SERIAL PRIMARY KEY,
    product_id INTEGER NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    color_no VARCHAR(50) NOT NULL,
    color_name VARCHAR(100) NOT NULL,
    pantone_code VARCHAR(50),
    color_type VARCHAR(20) DEFAULT '常规色',
    dye_formula TEXT,                     -- 染色配方（保密）
    extra_cost DECIMAL(10,2) DEFAULT 0,   -- 特殊色号加价（元/米）
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE product_colors IS '产品色号表（面料行业）';
COMMENT ON COLUMN product_colors.color_no IS '色号';
COMMENT ON COLUMN product_colors.color_name IS '颜色名称';
COMMENT ON COLUMN product_colors.pantone_code IS '潘通色号';
COMMENT ON COLUMN product_colors.color_type IS '色号类型：常规色/定制色';
COMMENT ON COLUMN product_colors.dye_formula IS '染色配方（保密）';
COMMENT ON COLUMN product_colors.extra_cost IS '特殊色号加价（元/米）';

CREATE INDEX IF NOT EXISTS idx_product_colors_product_id ON product_colors(product_id);
CREATE INDEX IF NOT EXISTS idx_product_colors_color_no ON product_colors(color_no);
CREATE INDEX IF NOT EXISTS idx_product_colors_color_type ON product_colors(color_type);

-- 创建触发器：自动更新 updated_at
CREATE OR REPLACE FUNCTION update_product_colors_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_product_colors_updated_at ON product_colors;
CREATE TRIGGER trg_product_colors_updated_at
    BEFORE UPDATE ON product_colors
    FOR EACH ROW
    EXECUTE FUNCTION update_product_colors_updated_at();

-- ========================================
-- 2. 仓库管理模块面料行业适配
-- ========================================

-- 2.1 仓库表增加面料行业字段
ALTER TABLE warehouses ADD COLUMN IF NOT EXISTS warehouse_type VARCHAR(20) DEFAULT '成品库';
ALTER TABLE warehouses ADD COLUMN IF NOT EXISTS temperature_control BOOLEAN DEFAULT false;
ALTER TABLE warehouses ADD COLUMN IF NOT EXISTS humidity_control BOOLEAN DEFAULT false;
ALTER TABLE warehouses ADD COLUMN IF NOT EXISTS fire_protection_level VARCHAR(10);

COMMENT ON COLUMN warehouses.warehouse_type IS '仓库类型：原料库/坯布库/成品库/辅料库';
COMMENT ON COLUMN warehouses.temperature_control IS '是否温控';
COMMENT ON COLUMN warehouses.humidity_control IS '是否湿度控制';
COMMENT ON COLUMN warehouses.fire_protection_level IS '消防等级：甲/乙/丙';

CREATE INDEX IF NOT EXISTS idx_warehouses_type ON warehouses(warehouse_type);

-- 2.2 创建库位表
CREATE TABLE IF NOT EXISTS warehouse_locations (
    id SERIAL PRIMARY KEY,
    warehouse_id INTEGER NOT NULL REFERENCES warehouses(id) ON DELETE CASCADE,
    location_code VARCHAR(50) NOT NULL,
    location_type VARCHAR(20) DEFAULT '平面库',
    max_weight DECIMAL(10,2),
    max_height DECIMAL(10,2),
    is_batch_managed BOOLEAN DEFAULT true,
    is_color_managed BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE warehouse_locations IS '库位表（面料行业）';
COMMENT ON COLUMN warehouse_locations.location_code IS '库位编码';
COMMENT ON COLUMN warehouse_locations.location_type IS '库位类型：平面库/货架库/卷装库';
COMMENT ON COLUMN warehouse_locations.is_batch_managed IS '是否批次管理';
COMMENT ON COLUMN warehouse_locations.is_color_managed IS '是否色号管理';

CREATE UNIQUE INDEX IF NOT EXISTS idx_location_code ON warehouse_locations(warehouse_id, location_code);

-- 创建触发器：自动更新 updated_at
DROP TRIGGER IF EXISTS trg_warehouse_locations_updated_at ON warehouse_locations;
CREATE TRIGGER trg_warehouse_locations_updated_at
    BEFORE UPDATE ON warehouse_locations
    FOR EACH ROW
    EXECUTE FUNCTION update_product_colors_updated_at();

-- ========================================
-- 3. 库存管理模块面料行业适配（核心）
-- ========================================

-- 3.1 库存表增加面料行业核心字段
ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS batch_no VARCHAR(50) NOT NULL DEFAULT '';
ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS color_no VARCHAR(50) NOT NULL DEFAULT '';
ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS dye_lot_no VARCHAR(50);
ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS grade VARCHAR(20) DEFAULT '一等品';
ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS production_date TIMESTAMPTZ;
ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS expiry_date TIMESTAMPTZ;

-- 双计量单位
ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS quantity_meters DECIMAL(12,2) NOT NULL DEFAULT 0;
ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS quantity_kg DECIMAL(12,2) NOT NULL DEFAULT 0;
ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS gram_weight DECIMAL(10,2);
ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS width DECIMAL(10,2);

-- 库位管理
ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS location_id INTEGER REFERENCES warehouse_locations(id);
ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS shelf_no VARCHAR(20);
ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS layer_no VARCHAR(20);

-- 状态管理
ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS stock_status VARCHAR(20) DEFAULT '正常';
ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS quality_status VARCHAR(20) DEFAULT '合格';

COMMENT ON COLUMN inventory_stocks.batch_no IS '批次号';
COMMENT ON COLUMN inventory_stocks.color_no IS '色号';
COMMENT ON COLUMN inventory_stocks.dye_lot_no IS '缸号';
COMMENT ON COLUMN inventory_stocks.grade IS '等级：一等品/二等品/等外品';
COMMENT ON COLUMN inventory_stocks.production_date IS '生产日期';
COMMENT ON COLUMN inventory_stocks.expiry_date IS '保质期';
COMMENT ON COLUMN inventory_stocks.quantity_meters IS '数量（米）';
COMMENT ON COLUMN inventory_stocks.quantity_kg IS '数量（公斤）';
COMMENT ON COLUMN inventory_stocks.gram_weight IS '克重（g/m²）';
COMMENT ON COLUMN inventory_stocks.width IS '幅宽（cm）';
COMMENT ON COLUMN inventory_stocks.location_id IS '库位 ID';
COMMENT ON COLUMN inventory_stocks.shelf_no IS '货架号';
COMMENT ON COLUMN inventory_stocks.layer_no IS '层号';
COMMENT ON COLUMN inventory_stocks.stock_status IS '库存状态：正常/冻结/待检';
COMMENT ON COLUMN inventory_stocks.quality_status IS '质量状态：合格/不合格/待检';

-- 创建索引（重要：提高查询性能）
CREATE INDEX IF NOT EXISTS idx_inventory_stocks_batch ON inventory_stocks(batch_no);
CREATE INDEX IF NOT EXISTS idx_inventory_stocks_color ON inventory_stocks(color_no);
CREATE INDEX IF NOT EXISTS idx_inventory_stocks_dye_lot ON inventory_stocks(dye_lot_no);
CREATE INDEX IF NOT EXISTS idx_inventory_stocks_grade ON inventory_stocks(grade);
CREATE INDEX IF NOT EXISTS idx_inventory_stocks_location ON inventory_stocks(location_id);
CREATE INDEX IF NOT EXISTS idx_inventory_stocks_quality ON inventory_stocks(quality_status);

-- 复合索引（常用查询）
CREATE INDEX IF NOT EXISTS idx_inventory_batch_color ON inventory_stocks(batch_no, color_no);
CREATE INDEX IF NOT EXISTS idx_inventory_warehouse_batch ON inventory_stocks(warehouse_id, batch_no, color_no);

-- 3.2 创建库存流水表（面料行业）
CREATE TABLE IF NOT EXISTS inventory_transactions (
    id SERIAL PRIMARY KEY,
    transaction_type VARCHAR(20) NOT NULL,
    product_id INTEGER NOT NULL REFERENCES products(id),
    warehouse_id INTEGER NOT NULL REFERENCES warehouses(id),
    batch_no VARCHAR(50) NOT NULL,
    color_no VARCHAR(50) NOT NULL,
    dye_lot_no VARCHAR(50),
    grade VARCHAR(20) DEFAULT '一等品',
    
    -- 双计量单位
    quantity_meters DECIMAL(12,2) NOT NULL,
    quantity_kg DECIMAL(12,2) NOT NULL,
    
    -- 关联单据
    source_bill_type VARCHAR(50),
    source_bill_no VARCHAR(50),
    source_bill_id INTEGER,
    
    -- 库存变化
    quantity_before_meters DECIMAL(12,2),
    quantity_before_kg DECIMAL(12,2),
    quantity_after_meters DECIMAL(12,2),
    quantity_after_kg DECIMAL(12,2),
    
    -- 备注
    notes TEXT,
    created_by INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE inventory_transactions IS '库存流水表（面料行业）';
COMMENT ON COLUMN inventory_transactions.transaction_type IS '交易类型：采购入库/生产入库/销售出库/调拨/盘点/调整';
COMMENT ON COLUMN inventory_transactions.batch_no IS '批次号';
COMMENT ON COLUMN inventory_transactions.color_no IS '色号';
COMMENT ON COLUMN inventory_transactions.quantity_meters IS '数量（米）';
COMMENT ON COLUMN inventory_transactions.quantity_kg IS '数量（公斤）';
COMMENT ON COLUMN inventory_transactions.source_bill_type IS '来源单据类型';
COMMENT ON COLUMN inventory_transactions.source_bill_no IS '来源单据号';

CREATE INDEX IF NOT EXISTS idx_inventory_transactions_batch ON inventory_transactions(batch_no, color_no);
CREATE INDEX IF NOT EXISTS idx_inventory_transactions_product ON inventory_transactions(product_id);
CREATE INDEX IF NOT EXISTS idx_inventory_transactions_warehouse ON inventory_transactions(warehouse_id);
CREATE INDEX IF NOT EXISTS idx_inventory_transactions_source ON inventory_transactions(source_bill_type, source_bill_id);
CREATE INDEX IF NOT EXISTS idx_inventory_transactions_created ON inventory_transactions(created_at);

-- ========================================
-- 4. 销售订单模块面料行业适配
-- ========================================

-- 4.1 销售订单表增加面料行业字段
ALTER TABLE sales_orders ADD COLUMN IF NOT EXISTS delivery_type VARCHAR(20) DEFAULT '一次性交货';
ALTER TABLE sales_orders ADD COLUMN IF NOT EXISTS partial_delivery_allowed BOOLEAN DEFAULT false;
ALTER TABLE sales_orders ADD COLUMN IF NOT EXISTS max_partial_count INTEGER;
ALTER TABLE sales_orders ADD COLUMN IF NOT EXISTS quality_standard VARCHAR(100);
ALTER TABLE sales_orders ADD COLUMN IF NOT EXISTS inspection_standard VARCHAR(100);
ALTER TABLE sales_orders ADD COLUMN IF NOT EXISTS packaging_requirement TEXT;
ALTER TABLE sales_orders ADD COLUMN IF NOT EXISTS shipping_mark TEXT;

COMMENT ON COLUMN sales_orders.delivery_type IS '交货方式：分批交货/一次性交货';
COMMENT ON COLUMN sales_orders.partial_delivery_allowed IS '是否允许分批交货';
COMMENT ON COLUMN sales_orders.quality_standard IS '质量标准';
COMMENT ON COLUMN sales_orders.inspection_standard IS '检验标准';
COMMENT ON COLUMN sales_orders.packaging_requirement IS '包装要求';
COMMENT ON COLUMN sales_orders.shipping_mark IS '唛头';

-- 4.2 销售订单明细表增加面料行业字段
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS color_no VARCHAR(50) NOT NULL DEFAULT '';
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS color_name VARCHAR(100);
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS pantone_code VARCHAR(50);
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS grade_required VARCHAR(20) DEFAULT '一等品';
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS quantity_meters DECIMAL(12,2) NOT NULL DEFAULT 0;
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS quantity_kg DECIMAL(12,2) DEFAULT 0;
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS gram_weight DECIMAL(10,2);
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS width DECIMAL(10,2);
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS batch_requirement VARCHAR(50) DEFAULT '可混批';
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS dye_lot_requirement VARCHAR(50) DEFAULT '可混缸';
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS base_price DECIMAL(10,2);
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS color_extra_cost DECIMAL(10,2) DEFAULT 0;
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS grade_price_diff DECIMAL(10,2) DEFAULT 0;
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS final_price DECIMAL(10,2);
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS shipped_quantity_meters DECIMAL(12,2) DEFAULT 0;
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS shipped_quantity_kg DECIMAL(12,2) DEFAULT 0;

COMMENT ON COLUMN sales_order_items.color_no IS '色号';
COMMENT ON COLUMN sales_order_items.color_name IS '颜色名称';
COMMENT ON COLUMN sales_order_items.pantone_code IS '潘通色号';
COMMENT ON COLUMN sales_order_items.grade_required IS '要求等级';
COMMENT ON COLUMN sales_order_items.quantity_meters IS '数量（米）';
COMMENT ON COLUMN sales_order_items.quantity_kg IS '数量（公斤）';
COMMENT ON COLUMN sales_order_items.gram_weight IS '克重';
COMMENT ON COLUMN sales_order_items.batch_requirement IS '批次要求：同批次/可混批';
COMMENT ON COLUMN sales_order_items.dye_lot_requirement IS '缸号要求：同缸号/可混缸';
COMMENT ON COLUMN sales_order_items.base_price IS '基价';
COMMENT ON COLUMN sales_order_items.color_extra_cost IS '色号加价';
COMMENT ON COLUMN sales_order_items.grade_price_diff IS '等级差价';
COMMENT ON COLUMN sales_order_items.final_price IS '最终单价';
COMMENT ON COLUMN sales_order_items.shipped_quantity_meters IS '已发货数量（米）';
COMMENT ON COLUMN sales_order_items.shipped_quantity_kg IS '已发货数量（公斤）';

CREATE INDEX IF NOT EXISTS idx_sales_order_items_color ON sales_order_items(color_no);
CREATE INDEX IF NOT EXISTS idx_sales_order_items_grade ON sales_order_items(grade_required);

-- ========================================
-- 5. 客户管理模块面料行业适配
-- ========================================

ALTER TABLE customers ADD COLUMN IF NOT EXISTS customer_industry VARCHAR(50);
ALTER TABLE customers ADD COLUMN IF NOT EXISTS main_products VARCHAR(200);
ALTER TABLE customers ADD COLUMN IF NOT EXISTS annual_purchase DECIMAL(14,2);
ALTER TABLE customers ADD COLUMN IF NOT EXISTS quality_requirement VARCHAR(50) DEFAULT '一般';
ALTER TABLE customers ADD COLUMN IF NOT EXISTS inspection_standard VARCHAR(100);
ALTER TABLE customers ADD COLUMN IF NOT EXISTS payment_terms VARCHAR(50);

COMMENT ON COLUMN customers.customer_industry IS '客户行业：服装/家纺/产业用';
COMMENT ON COLUMN customers.main_products IS '主营产品';
COMMENT ON COLUMN customers.annual_purchase IS '年采购量（万米）';
COMMENT ON COLUMN customers.quality_requirement IS '质量要求：一般/较高/严格';
COMMENT ON COLUMN customers.inspection_standard IS '检验标准偏好';
COMMENT ON COLUMN customers.payment_terms IS '付款条件';

CREATE INDEX IF NOT EXISTS idx_customers_industry ON customers(customer_industry);

-- ========================================
-- 6. 初始化数据（产品色号示例）
-- ========================================

-- 插入示例色号数据
INSERT INTO product_colors (product_id, color_no, color_name, pantone_code, color_type, extra_cost)
SELECT 
    p.id,
    colors.color_no,
    colors.color_name,
    colors.pantone_code,
    colors.color_type,
    colors.extra_cost
FROM products p
CROSS JOIN (
    VALUES 
        ('C001', '藏青色', '19-4052 TCX', '常规色', 0.00),
        ('C002', '大红色', '18-1664 TCX', '常规色', 0.00),
        ('C003', '军绿色', '18-0527 TCX', '常规色', 0.00),
        ('C004', '荧光绿', '802 C', '定制色', 2.50),
        ('C005', '迷彩色', 'CUSTOM', '定制色', 3.00)
) AS colors(color_no, color_name, pantone_code, color_type, extra_cost)
WHERE p.product_type = '成品布'
ON CONFLICT DO NOTHING;

-- ========================================
-- 7. 数据迁移（将现有数据迁移到新字段）
-- ========================================

-- 7.1 迁移现有库存数据到双计量单位字段
UPDATE inventory_stocks
SET 
    quantity_meters = quantity,
    quantity_kg = quantity * COALESCE(gram_weight, 150) / 1000,
    batch_no = COALESCE(batch_no, 'B' || TO_CHAR(CURRENT_DATE, 'YYYYMMDD') || LPAD(id::text, 4, '0')),
    color_no = COALESCE(color_no, 'C001'),
    grade = '一等品'
WHERE quantity_meters = 0;

-- 7.2 迁移现有销售订单数据
UPDATE sales_order_items
SET 
    quantity_meters = quantity,
    quantity_kg = quantity * COALESCE(gram_weight, 150) / 1000,
    color_no = 'C001',
    grade_required = '一等品',
    final_price = total_price / NULLIF(quantity, 0)
WHERE quantity_meters = 0;

-- ========================================
-- 8. 创建视图（面料行业常用查询）
-- ========================================

-- 8.1 库存汇总视图（按批次 + 色号）
CREATE OR REPLACE VIEW v_inventory_summary AS
SELECT 
    p.id AS product_id,
    p.name AS product_name,
    p.code AS product_code,
    p.product_type,
    w.id AS warehouse_id,
    w.name AS warehouse_name,
    s.batch_no,
    s.color_no,
    s.dye_lot_no,
    s.grade,
    SUM(s.quantity_meters) AS total_quantity_meters,
    SUM(s.quantity_kg) AS total_quantity_kg,
    AVG(s.gram_weight) AS avg_gram_weight,
    SUM(s.quantity_meters * COALESCE(s.unit_price, 0)) AS total_amount
FROM inventory_stocks s
INNER JOIN products p ON s.product_id = p.id
INNER JOIN warehouses w ON s.warehouse_id = w.id
WHERE s.stock_status = '正常'
  AND s.quality_status = '合格'
GROUP BY p.id, p.name, p.code, p.product_type, w.id, w.name, 
         s.batch_no, s.color_no, s.dye_lot_no, s.grade;

COMMENT ON VIEW v_inventory_summary IS '库存汇总视图（面料行业）';

-- 8.2 色号销售分析视图
CREATE OR REPLACE VIEW v_color_sales_analysis AS
SELECT 
    soi.color_no,
    soi.color_name,
    soi.pantone_code,
    COUNT(DISTINCT so.id) AS order_count,
    SUM(soi.quantity_meters) AS total_quantity_meters,
    SUM(soi.quantity_kg) AS total_quantity_kg,
    SUM(soi.quantity_meters * soi.final_price) AS total_amount,
    AVG(soi.final_price) AS avg_price,
    SUM(soi.quantity_meters * (soi.final_price - soi.base_price)) AS total_extra_cost
FROM sales_order_items soi
INNER JOIN sales_orders so ON soi.order_id = so.id
WHERE so.status NOT IN ('cancelled')
GROUP BY soi.color_no, soi.color_name, soi.pantone_code
ORDER BY total_quantity_meters DESC;

COMMENT ON VIEW v_color_sales_analysis IS '色号销售分析视图';

-- ========================================
-- 迁移完成提示
-- ========================================

DO $$
BEGIN
    RAISE NOTICE '========================================';
    RAISE NOTICE '面料 ERP 系统 - 面料行业适配迁移完成';
    RAISE NOTICE '版本：2026-03-15';
    RAISE NOTICE '========================================';
    RAISE NOTICE '新增表：2 个';
    RAISE NOTICE '  - product_colors (产品色号表)';
    RAISE NOTICE '  - warehouse_locations (库位表)';
    RAISE NOTICE '  - inventory_transactions (库存流水表)';
    RAISE NOTICE '';
    RAISE NOTICE '改造表：6 个';
    RAISE NOTICE '  - products (增加面料字段)';
    RAISE NOTICE '  - warehouses (增加仓库类型)';
    RAISE NOTICE '  - inventory_stocks (批次 + 色号 + 双计量单位)';
    RAISE NOTICE '  - sales_orders (增加面料字段)';
    RAISE NOTICE '  - sales_order_items (色号 + 双计量单位)';
    RAISE NOTICE '  - customers (增加行业字段)';
    RAISE NOTICE '';
    RAISE NOTICE '创建视图：2 个';
    RAISE NOTICE '  - v_inventory_summary (库存汇总)';
    RAISE NOTICE '  - v_color_sales_analysis (色号销售分析)';
    RAISE NOTICE '========================================';
END $$;
