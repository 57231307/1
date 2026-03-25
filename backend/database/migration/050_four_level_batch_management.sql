-- ========================================
-- 秉羲 ERP 系统 - 四级批次管理数据库迁移
-- 版本：2026-03-16
-- 模块：四级批次管理（成品 - 色号 - 缸号 - 匹号）
-- 说明：创建四级批次管理相关的所有表、索引、触发器
-- ========================================

-- ========================================
-- 1. 缸号管理表
-- ========================================

-- ==================== 缸号表 ====================
CREATE TABLE batch_dye_lot (
    id SERIAL PRIMARY KEY,
    dye_lot_no VARCHAR(100) NOT NULL,                    -- 缸号（内部编码）
    product_id INTEGER NOT NULL,                         -- 成品 ID
    color_id INTEGER NOT NULL,                           -- 色号 ID
    supplier_dye_lot_no VARCHAR(100),                    -- 供应商缸号
    supplier_id INTEGER NOT NULL,                        -- 供应商 ID
    production_date DATE,                                -- 生产日期
    machine_no VARCHAR(50),                              -- 机台号
    batch_weight DECIMAL(10,2),                          -- 缸重（kg）
    total_length DECIMAL(10,2),                          -- 总长度（米）
    total_pieces INTEGER DEFAULT 0,                      -- 总匹数
    quality_grade VARCHAR(10) DEFAULT 'A',               -- 质量等级（A/B/C/D）
    quality_status VARCHAR(20) DEFAULT 'pending',        -- 质检状态（pending/inspecting/passed/failed）
    inspection_date DATE,                                -- 质检日期
    inspector_id INTEGER,                                -- 质检员 ID
    remarks TEXT,                                        -- 备注
    is_active BOOLEAN DEFAULT TRUE,                      -- 是否启用
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE batch_dye_lot IS '缸号管理表';
COMMENT ON COLUMN batch_dye_lot.dye_lot_no IS '缸号（内部编码，自动生成）';
COMMENT ON COLUMN batch_dye_lot.product_id IS '成品 ID';
COMMENT ON COLUMN batch_dye_lot.color_id IS '色号 ID';
COMMENT ON COLUMN batch_dye_lot.supplier_dye_lot_no IS '供应商缸号（外部编码）';
COMMENT ON COLUMN batch_dye_lot.supplier_id IS '供应商 ID';
COMMENT ON COLUMN batch_dye_lot.quality_grade IS '质量等级（A/B/C/D）';
COMMENT ON COLUMN batch_dye_lot.quality_status IS '质检状态（pending/inspecting/passed/failed）';
COMMENT ON COLUMN batch_dye_lot.is_active IS '是否启用';

-- 外键约束
ALTER TABLE batch_dye_lot ADD CONSTRAINT fk_dye_lot_product
    FOREIGN KEY (product_id) REFERENCES products(id);
ALTER TABLE batch_dye_lot ADD CONSTRAINT fk_dye_lot_color
    FOREIGN KEY (color_id) REFERENCES product_colors(id);
ALTER TABLE batch_dye_lot ADD CONSTRAINT fk_dye_lot_supplier
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id);

-- 唯一约束
ALTER TABLE batch_dye_lot ADD CONSTRAINT uk_dye_lot_no UNIQUE (dye_lot_no);
ALTER TABLE batch_dye_lot ADD CONSTRAINT uk_dye_lot_product_color_supplier UNIQUE (product_id, color_id, supplier_id, dye_lot_no);

-- 索引
CREATE INDEX idx_batch_dye_lot_product ON batch_dye_lot(product_id);
CREATE INDEX idx_batch_dye_lot_color ON batch_dye_lot(color_id);
CREATE INDEX idx_batch_dye_lot_supplier ON batch_dye_lot(supplier_id);
CREATE INDEX idx_batch_dye_lot_dye_lot_no ON batch_dye_lot(dye_lot_no);
CREATE INDEX idx_batch_dye_lot_quality_status ON batch_dye_lot(quality_status);
CREATE INDEX idx_batch_dye_lot_production_date ON batch_dye_lot(production_date);
CREATE INDEX idx_batch_dye_lot_created_at ON batch_dye_lot(created_at DESC);

-- ========================================
-- 2. 匹号管理表
-- ========================================

-- ==================== 库存匹号表 ====================
CREATE TABLE inventory_piece (
    id SERIAL PRIMARY KEY,
    piece_no VARCHAR(100) NOT NULL UNIQUE,               -- 匹号（内部编码）
    dye_lot_id INTEGER NOT NULL,                         -- 缸号 ID
    supplier_piece_no VARCHAR(100),                      -- 供应商匹号
    length DECIMAL(10,2) NOT NULL,                       -- 长度（米）
    weight DECIMAL(10,2),                                -- 重量（kg）
    width DECIMAL(10,2),                                 -- 幅宽（cm）
    gram_weight DECIMAL(10,2),                           -- 克重（g/m²）
    position_no VARCHAR(50),                             -- 库位号
    package_no VARCHAR(50),                              -- 包号
    production_date DATE,                                -- 生产日期
    shelf_life INTEGER,                                  -- 保质期（天）
    quality_status VARCHAR(20) DEFAULT 'pending',        -- 质检状态（pending/inspecting/passed/failed）
    inventory_status VARCHAR(20) DEFAULT 'available',    -- 库存状态（available/reserved/locked/sold）
    warehouse_id INTEGER,                                -- 仓库 ID
    remarks TEXT,                                        -- 备注
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE inventory_piece IS '库存匹号管理表';
COMMENT ON COLUMN inventory_piece.piece_no IS '匹号（内部编码，唯一）';
COMMENT ON COLUMN inventory_piece.dye_lot_id IS '缸号 ID（关联 batch_dye_lot）';
COMMENT ON COLUMN inventory_piece.supplier_piece_no IS '供应商匹号（外部编码）';
COMMENT ON COLUMN inventory_piece.length IS '长度（米）';
COMMENT ON COLUMN inventory_piece.weight IS '重量（kg）';
COMMENT ON COLUMN inventory_piece.quality_status IS '质检状态（pending/inspecting/passed/failed）';
COMMENT ON COLUMN inventory_piece.inventory_status IS '库存状态（available/reserved/locked/sold）';

-- 外键约束
ALTER TABLE inventory_piece ADD CONSTRAINT fk_piece_dye_lot
    FOREIGN KEY (dye_lot_id) REFERENCES batch_dye_lot(id);
ALTER TABLE inventory_piece ADD CONSTRAINT fk_piece_warehouse
    FOREIGN KEY (warehouse_id) REFERENCES warehouses(id);

-- 索引
CREATE INDEX idx_inventory_piece_piece_no ON inventory_piece(piece_no);
CREATE INDEX idx_inventory_piece_dye_lot ON inventory_piece(dye_lot_id);
CREATE INDEX idx_inventory_piece_supplier_piece ON inventory_piece(supplier_piece_no);
CREATE INDEX idx_inventory_piece_quality_status ON inventory_piece(quality_status);
CREATE INDEX idx_inventory_piece_inventory_status ON inventory_piece(inventory_status);
CREATE INDEX idx_inventory_piece_warehouse ON inventory_piece(warehouse_id);
CREATE INDEX idx_inventory_piece_created_at ON inventory_piece(created_at DESC);

-- ========================================
-- 3. 编码映射表
-- ========================================

-- ==================== 成品编码映射表 ====================
CREATE TABLE product_code_mapping (
    id SERIAL PRIMARY KEY,
    internal_product_code VARCHAR(100) NOT NULL,         -- 内部成品编码
    supplier_product_code VARCHAR(100) NOT NULL,         -- 供应商成品编码
    supplier_id INTEGER NOT NULL,                        -- 供应商 ID
    is_active BOOLEAN DEFAULT TRUE,                      -- 是否启用
    mapping_date DATE NOT NULL,                          -- 映射日期
    validation_status VARCHAR(20) DEFAULT 'pending',     -- 验证状态（pending/validated/failed）
    validated_at TIMESTAMP,                              -- 验证时间
    validated_by INTEGER,                                -- 验证人 ID
    remarks TEXT,                                        -- 备注
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE product_code_mapping IS '成品编码映射表（内部编码 <-> 供应商编码）';
COMMENT ON COLUMN product_code_mapping.internal_product_code IS '内部成品编码';
COMMENT ON COLUMN product_code_mapping.supplier_product_code IS '供应商成品编码';
COMMENT ON COLUMN product_code_mapping.supplier_id IS '供应商 ID';
COMMENT ON COLUMN product_code_mapping.validation_status IS '验证状态（pending/validated/failed）';

-- 外键约束
ALTER TABLE product_code_mapping ADD CONSTRAINT fk_pcm_supplier
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id);

-- 唯一约束
ALTER TABLE product_code_mapping ADD CONSTRAINT uk_pcm_internal_supplier UNIQUE (internal_product_code, supplier_product_code, supplier_id);
ALTER TABLE product_code_mapping ADD CONSTRAINT uk_pcm_internal UNIQUE (internal_product_code, supplier_id);
ALTER TABLE product_code_mapping ADD CONSTRAINT uk_pcm_supplier UNIQUE (supplier_product_code, supplier_id);

-- 索引
CREATE INDEX idx_pcm_internal_code ON product_code_mapping(internal_product_code);
CREATE INDEX idx_pcm_supplier_code ON product_code_mapping(supplier_product_code);
CREATE INDEX idx_pcm_supplier ON product_code_mapping(supplier_id);
CREATE INDEX idx_pcm_validation_status ON product_code_mapping(validation_status);

-- ==================== 色号编码映射表 ====================
CREATE TABLE color_code_mapping (
    id SERIAL PRIMARY KEY,
    internal_color_no VARCHAR(100) NOT NULL,             -- 内部色号
    supplier_color_code VARCHAR(100) NOT NULL,           -- 供应商色号
    supplier_id INTEGER NOT NULL,                        -- 供应商 ID
    product_code VARCHAR(100),                           -- 成品编码（可选，用于更精确的映射）
    is_active BOOLEAN DEFAULT TRUE,                      -- 是否启用
    mapping_date DATE NOT NULL,                          -- 映射日期
    validation_status VARCHAR(20) DEFAULT 'pending',     -- 验证状态（pending/validated/failed）
    validated_at TIMESTAMP,                              -- 验证时间
    validated_by INTEGER,                                -- 验证人 ID
    color_difference_notes TEXT,                         -- 色差说明
    remarks TEXT,                                        -- 备注
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE color_code_mapping IS '色号编码映射表（内部色号 <-> 供应商色号）';
COMMENT ON COLUMN color_code_mapping.internal_color_no IS '内部色号';
COMMENT ON COLUMN color_code_mapping.supplier_color_code IS '供应商色号';
COMMENT ON COLUMN color_code_mapping.color_difference_notes IS '色差说明';

-- 外键约束
ALTER TABLE color_code_mapping ADD CONSTRAINT fk_ccm_supplier
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id);

-- 唯一约束
ALTER TABLE color_code_mapping ADD CONSTRAINT uk_ccm_internal_supplier UNIQUE (internal_color_no, supplier_color_code, supplier_id);
ALTER TABLE color_code_mapping ADD CONSTRAINT uk_ccm_internal UNIQUE (internal_color_no, supplier_id);
ALTER TABLE color_code_mapping ADD CONSTRAINT uk_ccm_supplier UNIQUE (supplier_color_code, supplier_id);

-- 索引
CREATE INDEX idx_ccm_internal_color ON color_code_mapping(internal_color_no);
CREATE INDEX idx_ccm_supplier_color ON color_code_mapping(supplier_color_code);
CREATE INDEX idx_ccm_supplier ON color_code_mapping(supplier_id);
CREATE INDEX idx_ccm_product_code ON color_code_mapping(product_code);

-- ==================== 缸号映射表 ====================
CREATE TABLE dye_lot_mapping (
    id SERIAL PRIMARY KEY,
    internal_dye_lot_no VARCHAR(100) NOT NULL,           -- 内部缸号
    supplier_dye_lot_no VARCHAR(100) NOT NULL,           -- 供应商缸号
    supplier_id INTEGER NOT NULL,                        -- 供应商 ID
    product_code VARCHAR(100),                           -- 成品编码
    color_no VARCHAR(100),                               -- 色号
    batch_dye_lot_id INTEGER,                            -- 内部缸号 ID
    is_active BOOLEAN DEFAULT TRUE,                      -- 是否启用
    mapping_date DATE NOT NULL,                          -- 映射日期
    validation_status VARCHAR(20) DEFAULT 'pending',     -- 验证状态（pending/validated/failed）
    validated_at TIMESTAMP,                              -- 验证时间
    validated_by INTEGER,                                -- 验证人 ID
    remarks TEXT,                                        -- 备注
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE dye_lot_mapping IS '缸号映射表（内部缸号 <-> 供应商缸号）';
COMMENT ON COLUMN dye_lot_mapping.batch_dye_lot_id IS '内部缸号 ID（关联 batch_dye_lot）';

-- 外键约束
ALTER TABLE dye_lot_mapping ADD CONSTRAINT fk_dlm_supplier
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id);
ALTER TABLE dye_lot_mapping ADD CONSTRAINT fk_dlm_batch_dye_lot
    FOREIGN KEY (batch_dye_lot_id) REFERENCES batch_dye_lot(id);

-- 唯一约束
ALTER TABLE dye_lot_mapping ADD CONSTRAINT uk_dlm_internal_supplier UNIQUE (internal_dye_lot_no, supplier_dye_lot_no, supplier_id);

-- 索引
CREATE INDEX idx_dlm_internal_dye_lot ON dye_lot_mapping(internal_dye_lot_no);
CREATE INDEX idx_dlm_supplier_dye_lot ON dye_lot_mapping(supplier_dye_lot_no);
CREATE INDEX idx_dlm_supplier ON dye_lot_mapping(supplier_id);
CREATE INDEX idx_dlm_batch_dye_lot ON dye_lot_mapping(batch_dye_lot_id);

-- ==================== 匹号映射表 ====================
CREATE TABLE piece_mapping (
    id SERIAL PRIMARY KEY,
    internal_piece_no VARCHAR(100) NOT NULL,             -- 内部匹号
    supplier_piece_no VARCHAR(100) NOT NULL,             -- 供应商匹号
    supplier_id INTEGER NOT NULL,                        -- 供应商 ID
    dye_lot_no VARCHAR(100),                             -- 缸号
    inventory_piece_id INTEGER,                          -- 内部匹号 ID
    is_active BOOLEAN DEFAULT TRUE,                      -- 是否启用
    mapping_date DATE NOT NULL,                          -- 映射日期
    validation_status VARCHAR(20) DEFAULT 'pending',     -- 验证状态（pending/validated/failed）
    validated_at TIMESTAMP,                              -- 验证时间
    validated_by INTEGER,                                -- 验证人 ID
    remarks TEXT,                                        -- 备注
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE piece_mapping IS '匹号映射表（内部匹号 <-> 供应商匹号）';
COMMENT ON COLUMN piece_mapping.inventory_piece_id IS '内部匹号 ID（关联 inventory_piece）';

-- 外键约束
ALTER TABLE piece_mapping ADD CONSTRAINT fk_pm_supplier
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id);
ALTER TABLE piece_mapping ADD CONSTRAINT fk_pm_inventory_piece
    FOREIGN KEY (inventory_piece_id) REFERENCES inventory_piece(id);

-- 唯一约束
ALTER TABLE piece_mapping ADD CONSTRAINT uk_pm_internal_supplier UNIQUE (internal_piece_no, supplier_piece_no, supplier_id);

-- 索引
CREATE INDEX idx_pm_internal_piece ON piece_mapping(internal_piece_no);
CREATE INDEX idx_pm_supplier_piece ON piece_mapping(supplier_piece_no);
CREATE INDEX idx_pm_supplier ON piece_mapping(supplier_id);
CREATE INDEX idx_pm_inventory_piece ON piece_mapping(inventory_piece_id);

-- ========================================
-- 4. 批次追溯日志表
-- ========================================

-- ==================== 批次追溯日志表 ====================
CREATE TABLE batch_trace_log (
    id SERIAL PRIMARY KEY,
    trace_no VARCHAR(100) NOT NULL UNIQUE,               -- 追溯编号（自动生成）
    business_type VARCHAR(50) NOT NULL,                  -- 业务类型（sales_order/delivery/purchase_order/receipt）
    business_id INTEGER NOT NULL,                        -- 业务 ID
    trace_direction VARCHAR(20) NOT NULL,                -- 追溯方向（internal_to_supplier/supplier_to_internal）
    
    -- 内部编码信息
    internal_product_code VARCHAR(100),                  -- 内部成品编码
    internal_color_no VARCHAR(100),                      -- 内部色号
    internal_dye_lot_no VARCHAR(100),                    -- 内部缸号
    internal_piece_nos TEXT[],                           -- 内部匹号列表
    
    -- 供应商编码信息
    supplier_product_code VARCHAR(100),                  -- 供应商成品编码
    supplier_color_code VARCHAR(100),                    -- 供应商色号
    supplier_dye_lot_no VARCHAR(100),                    -- 供应商缸号
    supplier_piece_nos TEXT[],                           -- 供应商匹号列表
    
    -- 转换详情
    conversion_details JSONB,                            -- 转换详情（JSON 格式）
    validation_result VARCHAR(50) DEFAULT 'pending',     -- 验证结果（pending/success/failed）
    validation_message TEXT,                             -- 验证信息
    
    -- 操作追踪
    operator_id INTEGER NOT NULL,                        -- 操作人 ID
    operation_time TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP, -- 操作时间
    operation_type VARCHAR(50) NOT NULL,                 -- 操作类型（create/update/convert/validate）
    ip_address VARCHAR(50),                              -- 操作 IP
    device_info VARCHAR(200),                            -- 设备信息
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    remarks TEXT                                         -- 备注
);

COMMENT ON TABLE batch_trace_log IS '批次追溯日志表（记录所有编码转换和追溯操作）';
COMMENT ON COLUMN batch_trace_log.business_type IS '业务类型（sales_order/delivery/purchase_order/receipt）';
COMMENT ON COLUMN batch_trace_log.trace_direction IS '追溯方向（internal_to_supplier/supplier_to_internal）';
COMMENT ON COLUMN batch_trace_log.internal_piece_nos IS '内部匹号列表（数组）';
COMMENT ON COLUMN batch_trace_log.supplier_piece_nos IS '供应商匹号列表（数组）';
COMMENT ON COLUMN batch_trace_log.conversion_details IS '转换详情（JSON 格式）';
COMMENT ON COLUMN batch_trace_log.validation_result IS '验证结果（pending/success/failed）';
COMMENT ON COLUMN batch_trace_log.operation_type IS '操作类型（create/update/convert/validate）';

-- 外键约束
ALTER TABLE batch_trace_log ADD CONSTRAINT fk_btl_operator
    FOREIGN KEY (operator_id) REFERENCES users(id);

-- 索引
CREATE INDEX idx_btl_trace_no ON batch_trace_log(trace_no);
CREATE INDEX idx_btl_business ON batch_trace_log(business_type, business_id);
CREATE INDEX idx_btl_internal_product ON batch_trace_log(internal_product_code);
CREATE INDEX idx_btl_supplier_product ON batch_trace_log(supplier_product_code);
CREATE INDEX idx_btl_internal_dye_lot ON batch_trace_log(internal_dye_lot_no);
CREATE INDEX idx_btl_supplier_dye_lot ON batch_trace_log(supplier_dye_lot_no);
CREATE INDEX idx_btl_operation_time ON batch_trace_log(operation_time DESC);
CREATE INDEX idx_btl_operator ON batch_trace_log(operator_id);
CREATE INDEX idx_btl_validation_result ON batch_trace_log(validation_result);

-- ========================================
-- 5. 触发器函数
-- ========================================

-- ==================== 自动更新 updated_at ====================
CREATE OR REPLACE FUNCTION update_batch_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ==================== 缸号总匹数自动更新 ====================
CREATE OR REPLACE FUNCTION update_dye_lot_total_pieces()
RETURNS TRIGGER AS $$
BEGIN
    -- 更新缸号的总匹数
    UPDATE batch_dye_lot
    SET total_pieces = (
        SELECT COUNT(*) 
        FROM inventory_piece 
        WHERE dye_lot_id = NEW.dye_lot_id
    ),
    updated_at = CURRENT_TIMESTAMP
    WHERE id = NEW.dye_lot_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ==================== 匹号自动生成（如果需要） ====================
CREATE OR REPLACE FUNCTION generate_piece_no()
RETURNS TRIGGER AS $$
BEGIN
    -- 如果匹号为空，自动生成
    IF NEW.piece_no IS NULL OR NEW.piece_no = '' THEN
        NEW.piece_no := 'P' || TO_CHAR(CURRENT_DATE, 'YYYYMMDD') || LPAD(NEXTVAL('inventory_piece_id_seq')::TEXT, 6, '0');
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ========================================
-- 6. 应用触发器
-- ========================================

-- 更新 updated_at 触发器
CREATE TRIGGER trg_batch_dye_lot_updated_at
    BEFORE UPDATE ON batch_dye_lot
    FOR EACH ROW
    EXECUTE FUNCTION update_batch_updated_at_column();

CREATE TRIGGER trg_inventory_piece_updated_at
    BEFORE UPDATE ON inventory_piece
    FOR EACH ROW
    EXECUTE FUNCTION update_batch_updated_at_column();

CREATE TRIGGER trg_product_code_mapping_updated_at
    BEFORE UPDATE ON product_code_mapping
    FOR EACH ROW
    EXECUTE FUNCTION update_batch_updated_at_column();

CREATE TRIGGER trg_color_code_mapping_updated_at
    BEFORE UPDATE ON color_code_mapping
    FOR EACH ROW
    EXECUTE FUNCTION update_batch_updated_at_column();

CREATE TRIGGER trg_dye_lot_mapping_updated_at
    BEFORE UPDATE ON dye_lot_mapping
    FOR EACH ROW
    EXECUTE FUNCTION update_batch_updated_at_column();

CREATE TRIGGER trg_piece_mapping_updated_at
    BEFORE UPDATE ON piece_mapping
    FOR EACH ROW
    EXECUTE FUNCTION update_batch_updated_at_column();

-- 缸号总匹数自动更新触发器
CREATE TRIGGER trg_inventory_piece_insert_update_pieces
    AFTER INSERT OR UPDATE ON inventory_piece
    FOR EACH ROW
    EXECUTE FUNCTION update_dye_lot_total_pieces();

CREATE TRIGGER trg_inventory_piece_delete_update_pieces
    AFTER DELETE ON inventory_piece
    FOR EACH ROW
    EXECUTE FUNCTION update_dye_lot_total_pieces();

-- 匹号自动生成触发器（可选）
CREATE TRIGGER trg_inventory_piece_generate_no
    BEFORE INSERT ON inventory_piece
    FOR EACH ROW
    EXECUTE FUNCTION generate_piece_no();

-- ========================================
-- 7. 初始化数据
-- ========================================

-- 初始化批次追溯编号序列
CREATE SEQUENCE IF NOT EXISTS batch_trace_log_trace_no_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

-- ========================================
-- 迁移完成
-- ========================================
