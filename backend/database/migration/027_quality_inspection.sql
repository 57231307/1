-- P2 级模块：质量检验
-- 创建时间：2026-03-15
-- 功能：质量检验管理、检验标准、不合格品处理

-- 质量检验标准表
CREATE TABLE IF NOT EXISTS quality_inspection_standards (
    id SERIAL PRIMARY KEY,
    standard_name VARCHAR(100) NOT NULL,
    standard_code VARCHAR(50) NOT NULL UNIQUE,
    product_id INTEGER REFERENCES products(id),
    product_category_id INTEGER REFERENCES product_categories(id),
    inspection_type VARCHAR(20) NOT NULL,
    inspection_items JSONB,
    sampling_method VARCHAR(50),
    sampling_rate DECIMAL(5,2),
    acceptance_criteria TEXT,
    status VARCHAR(20) DEFAULT 'active',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 质量检验记录表
CREATE TABLE IF NOT EXISTS quality_inspection_records (
    id SERIAL PRIMARY KEY,
    inspection_no VARCHAR(50) NOT NULL UNIQUE,
    inspection_type VARCHAR(20) NOT NULL,
    related_type VARCHAR(50),
    related_id INTEGER,
    product_id INTEGER NOT NULL REFERENCES products(id),
    batch_no VARCHAR(50),
    supplier_id INTEGER REFERENCES suppliers(id),
    customer_id INTEGER REFERENCES customers(id),
    inspection_date DATE NOT NULL,
    inspector_id INTEGER,
    total_qty DECIMAL(14,4) NOT NULL,
    inspected_qty DECIMAL(14,4) NOT NULL,
    qualified_qty DECIMAL(14,4),
    unqualified_qty DECIMAL(14,4),
    qualification_rate DECIMAL(5,2),
    inspection_result VARCHAR(20) NOT NULL,
    remark TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 质量检验明细表
CREATE TABLE IF NOT EXISTS quality_inspection_details (
    id SERIAL PRIMARY KEY,
    inspection_id INTEGER NOT NULL REFERENCES quality_inspection_records(id),
    inspection_item VARCHAR(100) NOT NULL,
    standard_value VARCHAR(100),
    actual_value VARCHAR(100),
    result VARCHAR(20) NOT NULL,
    remark TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 不合格品处理表
CREATE TABLE IF NOT EXISTS unqualified_products (
    id SERIAL PRIMARY KEY,
    unqualified_no VARCHAR(50) NOT NULL UNIQUE,
    inspection_id INTEGER REFERENCES quality_inspection_records(id),
    product_id INTEGER NOT NULL REFERENCES products(id),
    batch_no VARCHAR(50),
    unqualified_qty DECIMAL(14,4) NOT NULL,
    unqualified_reason TEXT NOT NULL,
    handling_method VARCHAR(20) NOT NULL,
    handling_status VARCHAR(20) DEFAULT 'pending',
    handling_by INTEGER,
    handling_at TIMESTAMP,
    remark TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 质量统计表
CREATE TABLE IF NOT EXISTS quality_statistics (
    id SERIAL PRIMARY KEY,
    period VARCHAR(7) NOT NULL,
    product_id INTEGER REFERENCES products(id),
    supplier_id INTEGER REFERENCES suppliers(id),
    inspection_count INTEGER DEFAULT 0,
    total_qty DECIMAL(14,4) DEFAULT 0,
    qualified_qty DECIMAL(14,4) DEFAULT 0,
    unqualified_qty DECIMAL(14,4) DEFAULT 0,
    qualification_rate DECIMAL(5,2),
    unqualified_reasons JSONB,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_quality_standards_product ON quality_inspection_standards(product_id);
CREATE INDEX IF NOT EXISTS idx_quality_records_inspection_date ON quality_inspection_records(inspection_date);
CREATE INDEX IF NOT EXISTS idx_quality_records_product ON quality_inspection_records(product_id);
CREATE INDEX IF NOT EXISTS idx_unqualified_products_batch ON unqualified_products(batch_no);
CREATE INDEX IF NOT EXISTS idx_quality_statistics_period ON quality_statistics(period);

-- 添加中文注释
COMMENT ON TABLE quality_inspection_standards IS '质量检验标准表';
COMMENT ON COLUMN quality_inspection_standards.inspection_type IS '检验类型（来料/过程/成品/出货）';
COMMENT ON COLUMN quality_inspection_standards.inspection_items IS '检验项目（JSON 格式）';
COMMENT ON COLUMN quality_inspection_standards.sampling_method IS '抽样方法';

COMMENT ON TABLE quality_inspection_records IS '质量检验记录表';
COMMENT ON COLUMN quality_inspection_records.related_type IS '关联类型（采购入库/生产完工/销售出库）';
COMMENT ON COLUMN quality_inspection_records.qualification_rate IS '合格率';
COMMENT ON COLUMN quality_inspection_records.inspection_result IS '检验结果（合格/不合格/特采）';

COMMENT ON TABLE quality_inspection_details IS '质量检验明细表';
COMMENT ON COLUMN quality_inspection_details.inspection_item IS '检验项目';
COMMENT ON COLUMN quality_inspection_details.result IS '检验结果（合格/不合格）';

COMMENT ON TABLE unqualified_products IS '不合格品处理表';
COMMENT ON COLUMN unqualified_products.handling_method IS '处理方式（退货/返工/报废/特采）';
COMMENT ON COLUMN unqualified_products.handling_status IS '处理状态（待处理/处理中/已完成）';

COMMENT ON TABLE quality_statistics IS '质量统计表';
COMMENT ON COLUMN quality_statistics.unqualified_reasons IS '不合格原因统计（JSON 格式）';
