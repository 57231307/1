-- ========================================
-- 秉羲 ERP 系统 - 供应商产品映射模块数据库迁移
-- 版本：2026-03-23
-- 模块：供应商产品映射
-- 说明：创建供应商产品映射相关的表、索引
-- ========================================

-- ========================================
-- 1. 供应商产品表
-- ========================================

-- ==================== 供应商产品表 ====================
CREATE TABLE IF NOT EXISTS supplier_products (
    id SERIAL PRIMARY KEY,
    supplier_id INTEGER NOT NULL REFERENCES suppliers(id) ON DELETE CASCADE,
    product_code VARCHAR(100) NOT NULL,                    -- 供应商产品编码
    product_name VARCHAR(200) NOT NULL,                   -- 供应商产品名称
    product_description TEXT,                               -- 产品描述
    unit VARCHAR(20) DEFAULT '米',                          -- 计量单位
    is_enabled BOOLEAN DEFAULT TRUE,                        -- 是否启用
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                     -- 创建人 ID
    updated_by INTEGER,                                     -- 更新人 ID
    remarks TEXT                                            -- 备注
);

COMMENT ON TABLE supplier_products IS '供应商产品表';
COMMENT ON COLUMN supplier_products.product_code IS '供应商产品编码';
COMMENT ON COLUMN supplier_products.product_name IS '供应商产品名称';

-- 唯一约束
ALTER TABLE supplier_products ADD CONSTRAINT uk_supplier_product_code UNIQUE (supplier_id, product_code);

-- 索引
CREATE INDEX IF NOT EXISTS idx_supplier_products_supplier_id ON supplier_products(supplier_id);
CREATE INDEX IF NOT EXISTS idx_supplier_products_code ON supplier_products(product_code);
CREATE INDEX IF NOT EXISTS idx_supplier_products_enabled ON supplier_products(is_enabled);

-- ========================================
-- 2. 供应商产品颜色表
-- ========================================

-- ==================== 供应商产品颜色表 ====================
CREATE TABLE IF NOT EXISTS supplier_product_colors (
    id SERIAL PRIMARY KEY,
    supplier_product_id INTEGER NOT NULL REFERENCES supplier_products(id) ON DELETE CASCADE,
    color_no VARCHAR(50) NOT NULL,                         -- 供应商色号编码
    color_name VARCHAR(100) NOT NULL,                      -- 供应商颜色名称
    pantone_code VARCHAR(50),                               -- 潘通色号
    extra_cost DECIMAL(10,2) DEFAULT 0.00,                 -- 特殊色号加价
    is_enabled BOOLEAN DEFAULT TRUE,                        -- 是否启用
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    remarks TEXT                                            -- 备注
);

COMMENT ON TABLE supplier_product_colors IS '供应商产品颜色表';
COMMENT ON COLUMN supplier_product_colors.color_no IS '供应商色号编码';

-- 唯一约束
ALTER TABLE supplier_product_colors ADD CONSTRAINT uk_supplier_product_color UNIQUE (supplier_product_id, color_no);

-- 索引
CREATE INDEX IF NOT EXISTS idx_supplier_product_colors_product_id ON supplier_product_colors(supplier_product_id);
CREATE INDEX IF NOT EXISTS idx_supplier_product_colors_color_no ON supplier_product_colors(color_no);
CREATE INDEX IF NOT EXISTS idx_supplier_product_colors_enabled ON supplier_product_colors(is_enabled);

-- ========================================
-- 3. 系统产品与供应商产品映射表
-- ========================================

-- ==================== 系统产品与供应商产品映射表 ====================
CREATE TABLE IF NOT EXISTS product_supplier_mappings (
    id SERIAL PRIMARY KEY,
    product_id INTEGER NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    product_color_id INTEGER REFERENCES product_colors(id) ON DELETE SET NULL,
    supplier_id INTEGER NOT NULL REFERENCES suppliers(id) ON DELETE CASCADE,
    supplier_product_id INTEGER NOT NULL REFERENCES supplier_products(id) ON DELETE CASCADE,
    supplier_product_color_id INTEGER REFERENCES supplier_product_colors(id) ON DELETE SET NULL,
    is_primary BOOLEAN DEFAULT FALSE,                       -- 是否为首选供应商
    priority INTEGER DEFAULT 1,                              -- 优先级（数字越小优先级越高）
    supplier_price DECIMAL(12,2),                            -- 供应商价格
    min_order_quantity DECIMAL(12,2),                        -- 最小起订量
    lead_time INTEGER,                                        -- 交货期（天）
    is_enabled BOOLEAN DEFAULT TRUE,                          -- 是否启用
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                       -- 创建人 ID
    updated_by INTEGER,                                       -- 更新人 ID
    remarks TEXT                                              -- 备注
);

COMMENT ON TABLE product_supplier_mappings IS '系统产品与供应商产品映射表';
COMMENT ON COLUMN product_supplier_mappings.is_primary IS '是否为首选供应商';
COMMENT ON COLUMN product_supplier_mappings.priority IS '优先级（数字越小优先级越高）';

-- 唯一约束
ALTER TABLE product_supplier_mappings ADD CONSTRAINT uk_product_supplier_mapping UNIQUE (
    product_id, 
    COALESCE(product_color_id, 0), 
    supplier_id, 
    supplier_product_id, 
    COALESCE(supplier_product_color_id, 0)
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_product_supplier_mappings_product ON product_supplier_mappings(product_id);
CREATE INDEX IF NOT EXISTS idx_product_supplier_mappings_product_color ON product_supplier_mappings(product_color_id);
CREATE INDEX IF NOT EXISTS idx_product_supplier_mappings_supplier ON product_supplier_mappings(supplier_id);
CREATE INDEX IF NOT EXISTS idx_product_supplier_mappings_supplier_product ON product_supplier_mappings(supplier_product_id);
CREATE INDEX IF NOT EXISTS idx_product_supplier_mappings_primary ON product_supplier_mappings(is_primary);
CREATE INDEX IF NOT EXISTS idx_product_supplier_mappings_enabled ON product_supplier_mappings(is_enabled);

-- ========================================
-- 4. 触发器函数
-- ========================================

-- ==================== 自动更新 updated_at ====================
CREATE OR REPLACE FUNCTION update_supplier_product_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ========================================
-- 5. 应用触发器
-- ========================================

-- 更新 updated_at 触发器
DROP TRIGGER IF EXISTS trg_supplier_products_updated_at ON supplier_products;
CREATE TRIGGER trg_supplier_products_updated_at
    BEFORE UPDATE ON supplier_products
    FOR EACH ROW
    EXECUTE FUNCTION update_supplier_product_updated_at_column();

DROP TRIGGER IF EXISTS trg_supplier_product_colors_updated_at ON supplier_product_colors;
CREATE TRIGGER trg_supplier_product_colors_updated_at
    BEFORE UPDATE ON supplier_product_colors
    FOR EACH ROW
    EXECUTE FUNCTION update_supplier_product_updated_at_column();

DROP TRIGGER IF EXISTS trg_product_supplier_mappings_updated_at ON product_supplier_mappings;
CREATE TRIGGER trg_product_supplier_mappings_updated_at
    BEFORE UPDATE ON product_supplier_mappings
    FOR EACH ROW
    EXECUTE FUNCTION update_supplier_product_updated_at_column();

-- ========================================
-- 迁移完成
-- ========================================
