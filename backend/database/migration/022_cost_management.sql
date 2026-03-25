-- ============================================
-- 成本管理模块 - 基础表结构
-- ============================================
-- 文档编号：MIGRATION-022-COST
-- 创建日期：2026-03-15
-- 说明：成本管理模块表结构，面料行业成本核算
-- ============================================

-- 1. 成本归集表
-- ============================================
CREATE TABLE cost_collections (
    id SERIAL PRIMARY KEY,
    collection_no VARCHAR(50) NOT NULL UNIQUE,
    collection_date DATE NOT NULL,
    
    -- 成本对象
    cost_object_type VARCHAR(20),
    cost_object_id INTEGER,
    cost_object_no VARCHAR(50),
    
    -- 面料行业字段
    batch_no VARCHAR(50),
    color_no VARCHAR(50),
    dye_lot_no VARCHAR(50),
    workshop VARCHAR(100),
    production_order_no VARCHAR(50),
    
    -- 成本构成
    direct_material DECIMAL(14,2) DEFAULT 0,
    direct_labor DECIMAL(14,2) DEFAULT 0,
    manufacturing_overhead DECIMAL(14,2) DEFAULT 0,
    processing_fee DECIMAL(14,2) DEFAULT 0,
    dyeing_fee DECIMAL(14,2) DEFAULT 0,
    
    -- 总金额
    total_cost DECIMAL(14,2) DEFAULT 0,
    
    -- 双计量单位
    output_quantity_meters DECIMAL(14,2),
    output_quantity_kg DECIMAL(14,2),
    unit_cost_meters DECIMAL(14,2),
    unit_cost_kg DECIMAL(14,2),
    
    -- 状态
    status VARCHAR(20) DEFAULT 'draft',
    
    created_by INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE cost_collections IS '成本归集表';
COMMENT ON COLUMN cost_collections.collection_no IS '成本归集单编号';
COMMENT ON COLUMN cost_collections.batch_no IS '批次号';
COMMENT ON COLUMN cost_collections.color_no IS '色号';
COMMENT ON COLUMN cost_collections.direct_material IS '直接材料';
COMMENT ON COLUMN cost_collections.direct_labor IS '直接人工';
COMMENT ON COLUMN cost_collections.manufacturing_overhead IS '制造费用';
COMMENT ON COLUMN cost_collections.processing_fee IS '委托加工费';
COMMENT ON COLUMN cost_collections.dyeing_fee IS '染费';
COMMENT ON COLUMN cost_collections.total_cost IS '总成本';
COMMENT ON COLUMN cost_collections.unit_cost_meters IS '单位成本（米）';
COMMENT ON COLUMN cost_collections.unit_cost_kg IS '单位成本（公斤）';

CREATE INDEX idx_cost_collections_no ON cost_collections(collection_no);
CREATE INDEX idx_cost_collections_batch ON cost_collections(batch_no);
CREATE INDEX idx_cost_collections_color_no ON cost_collections(color_no);
CREATE INDEX idx_cost_collections_date ON cost_collections(collection_date);

-- 2. 直接材料明细表
-- ============================================
CREATE TABLE cost_direct_materials (
    id SERIAL PRIMARY KEY,
    collection_id INTEGER NOT NULL REFERENCES cost_collections(id),
    
    -- 物料信息
    material_id INTEGER,
    material_name VARCHAR(200),
    material_code VARCHAR(50),
    
    -- 面料行业字段
    batch_no VARCHAR(50),
    color_no VARCHAR(50),
    
    -- 数量和金额
    quantity_meters DECIMAL(14,2),
    quantity_kg DECIMAL(14,2),
    unit_price DECIMAL(12,2),
    amount DECIMAL(14,2),
    
    -- 来源单据
    source_type VARCHAR(20),
    source_bill_id INTEGER,
    source_bill_no VARCHAR(50),
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE cost_direct_materials IS '直接材料明细表';
COMMENT ON COLUMN cost_direct_materials.collection_id IS '成本归集 ID';
COMMENT ON COLUMN cost_direct_materials.material_id IS '物料 ID';
COMMENT ON COLUMN cost_direct_materials.amount IS '金额';

CREATE INDEX idx_cost_direct_materials_collection ON cost_direct_materials(collection_id);
CREATE INDEX idx_cost_direct_materials_material ON cost_direct_materials(material_id);

-- 3. 直接人工明细表
-- ============================================
CREATE TABLE cost_direct_labors (
    id SERIAL PRIMARY KEY,
    collection_id INTEGER NOT NULL REFERENCES cost_collections(id),
    
    -- 员工信息
    employee_id INTEGER,
    employee_name VARCHAR(100),
    employee_code VARCHAR(50),
    
    -- 工时和工资
    work_hours DECIMAL(10,2),
    hourly_rate DECIMAL(10,2),
    amount DECIMAL(14,2),
    
    -- 工作描述
    work_description TEXT,
    workshop VARCHAR(100),
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE cost_direct_labors IS '直接人工明细表';
COMMENT ON COLUMN cost_direct_labors.collection_id IS '成本归集 ID';
COMMENT ON COLUMN cost_direct_labors.work_hours IS '工时';
COMMENT ON COLUMN cost_direct_labors.amount IS '工资金额';

CREATE INDEX idx_cost_direct_labors_collection ON cost_direct_labors(collection_id);

-- 4. 制造费用明细表
-- ============================================
CREATE TABLE cost_manufacturing_overheads (
    id SERIAL PRIMARY KEY,
    collection_id INTEGER NOT NULL REFERENCES cost_collections(id),
    
    -- 费用类型
    expense_type VARCHAR(50),
    expense_name VARCHAR(200),
    
    -- 金额
    amount DECIMAL(14,2),
    
    -- 分摊方式
    allocation_method VARCHAR(20),
    allocation_base DECIMAL(14,2),
    
    -- 描述
    description TEXT,
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE cost_manufacturing_overheads IS '制造费用明细表';
COMMENT ON COLUMN cost_manufacturing_overheads.collection_id IS '成本归集 ID';
COMMENT ON COLUMN cost_manufacturing_overheads.expense_type IS '费用类型';
COMMENT ON COLUMN cost_manufacturing_overheads.amount IS '金额';
COMMENT ON COLUMN cost_manufacturing_overheads.allocation_method IS '分摊方式';

CREATE INDEX idx_cost_manufacturing_overheads_collection ON cost_manufacturing_overheads(collection_id);

-- 5. 染费明细表
-- ============================================
CREATE TABLE cost_dyeing_fees (
    id SERIAL PRIMARY KEY,
    collection_id INTEGER NOT NULL REFERENCES cost_collections(id),
    
    -- 印染厂信息
    dyeing_factory_id INTEGER,
    dyeing_factory_name VARCHAR(200),
    
    -- 面料行业字段
    batch_no VARCHAR(50),
    color_no VARCHAR(50),
    dye_lot_no VARCHAR(50),
    
    -- 数量和金额
    quantity_meters DECIMAL(14,2),
    quantity_kg DECIMAL(14,2),
    unit_price DECIMAL(12,2),
    amount DECIMAL(14,2),
    
    -- 染费类型
    dyeing_type VARCHAR(20),
    
    -- 来源单据
    invoice_id INTEGER,
    invoice_no VARCHAR(50),
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE cost_dyeing_fees IS '染费明细表';
COMMENT ON COLUMN cost_dyeing_fees.collection_id IS '成本归集 ID';
COMMENT ON COLUMN cost_dyeing_fees.dyeing_factory_id IS '印染厂 ID';
COMMENT ON COLUMN cost_dyeing_fees.quantity_meters IS '数量（米）';
COMMENT ON COLUMN cost_dyeing_fees.amount IS '染费金额';

CREATE INDEX idx_cost_dyeing_fees_collection ON cost_dyeing_fees(collection_id);
CREATE INDEX idx_cost_dyeing_fees_factory ON cost_dyeing_fees(dyeing_factory_id);

-- 6. 成本分析表
-- ============================================
CREATE TABLE cost_analyses (
    id SERIAL PRIMARY KEY,
    analysis_no VARCHAR(50) NOT NULL UNIQUE,
    analysis_date DATE NOT NULL,
    
    -- 分析维度
    period VARCHAR(7),
    batch_no VARCHAR(50),
    color_no VARCHAR(50),
    workshop VARCHAR(100),
    
    -- 成本数据
    total_direct_material DECIMAL(14,2),
    total_direct_labor DECIMAL(14,2),
    total_overhead DECIMAL(14,2),
    total_processing_fee DECIMAL(14,2),
    total_dyeing_fee DECIMAL(14,2),
    total_cost DECIMAL(14,2),
    
    -- 产量数据
    total_output_meters DECIMAL(14,2),
    total_output_kg DECIMAL(14,2),
    
    -- 单位成本
    avg_unit_cost_meters DECIMAL(14,2),
    avg_unit_cost_kg DECIMAL(14,2),
    
    -- 对比分析
    standard_cost DECIMAL(14,2),
    variance DECIMAL(14,2),
    variance_rate DECIMAL(8,4),
    
    -- 分析结论
    conclusion TEXT,
    
    created_by INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE cost_analyses IS '成本分析表';
COMMENT ON COLUMN cost_analyses.analysis_no IS '成本分析单编号';
COMMENT ON COLUMN cost_analyses.period IS '会计期间';
COMMENT ON COLUMN cost_analyses.total_direct_material IS '直接材料合计';
COMMENT ON COLUMN cost_analyses.total_cost IS '总成本';
COMMENT ON COLUMN cost_analyses.avg_unit_cost_meters IS '平均单位成本（米）';
COMMENT ON COLUMN cost_analyses.variance IS '差异';
COMMENT ON COLUMN cost_analyses.variance_rate IS '差异率';

CREATE INDEX idx_cost_analyses_no ON cost_analyses(analysis_no);
CREATE INDEX idx_cost_analyses_period ON cost_analyses(period);
CREATE INDEX idx_cost_analyses_batch ON cost_analyses(batch_no);
CREATE INDEX idx_cost_analyses_color_no ON cost_analyses(color_no);

-- ============================================
-- 迁移完成
-- ============================================
