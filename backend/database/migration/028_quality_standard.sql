-- P1 级模块：质量标准
-- 创建时间：2026-03-15
-- 功能：质量标准管理、质量标准版本控制

-- 质量标准表
CREATE TABLE IF NOT EXISTS quality_standards (
    id SERIAL PRIMARY KEY,
    standard_name VARCHAR(100) NOT NULL,
    standard_code VARCHAR(50) NOT NULL UNIQUE,
    standard_type VARCHAR(20) NOT NULL,
    product_id INTEGER REFERENCES products(id),
    product_category_id INTEGER REFERENCES product_categories(id),
    version VARCHAR(20) NOT NULL,
    previous_version_id INTEGER REFERENCES quality_standards(id),
    content TEXT NOT NULL,
    technical_requirements TEXT,
    testing_methods TEXT,
    acceptance_criteria TEXT,
    effective_date DATE NOT NULL,
    expiry_date DATE,
    status VARCHAR(20) DEFAULT 'active',
    approved_by INTEGER,
    approved_at TIMESTAMP,
    created_by INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 质量标准版本历史表
CREATE TABLE IF NOT EXISTS quality_standard_versions (
    id SERIAL PRIMARY KEY,
    standard_id INTEGER NOT NULL REFERENCES quality_standards(id),
    version VARCHAR(20) NOT NULL,
    change_type VARCHAR(20) NOT NULL,
    change_description TEXT,
    old_content TEXT,
    new_content TEXT,
    changed_by INTEGER,
    changed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    remark TEXT
);

-- 质量标准引用表
CREATE TABLE IF NOT EXISTS quality_standard_references (
    id SERIAL PRIMARY KEY,
    standard_id INTEGER NOT NULL REFERENCES quality_standards(id),
    reference_type VARCHAR(20) NOT NULL,
    reference_name VARCHAR(200) NOT NULL,
    reference_content TEXT,
    is_mandatory BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_quality_standards_type ON quality_standards(standard_type);
CREATE INDEX IF NOT EXISTS idx_quality_standards_product ON quality_standards(product_id);
CREATE INDEX IF NOT EXISTS idx_quality_standard_versions_standard ON quality_standard_versions(standard_id);
CREATE INDEX IF NOT EXISTS idx_quality_standard_references_standard ON quality_standard_references(standard_id);

-- 添加中文注释
COMMENT ON TABLE quality_standards IS '质量标准表';
COMMENT ON COLUMN quality_standards.standard_type IS '标准类型（国标/行标/企标/客标）';
COMMENT ON COLUMN quality_standards.version IS '版本号';
COMMENT ON COLUMN quality_standards.content IS '标准内容';
COMMENT ON COLUMN quality_standards.technical_requirements IS '技术要求';
COMMENT ON COLUMN quality_standards.testing_methods IS '测试方法';
COMMENT ON COLUMN quality_standards.acceptance_criteria IS '验收标准';

COMMENT ON TABLE quality_standard_versions IS '质量标准版本历史表';
COMMENT ON COLUMN quality_standard_versions.change_type IS '变更类型（新增/修改/废止）';
COMMENT ON COLUMN quality_standard_versions.change_description IS '变更描述';

COMMENT ON TABLE quality_standard_references IS '质量标准引用表';
COMMENT ON COLUMN quality_standard_references.reference_type IS '引用类型（参考/强制）';
COMMENT ON COLUMN quality_standard_references.is_mandatory IS '是否强制';
