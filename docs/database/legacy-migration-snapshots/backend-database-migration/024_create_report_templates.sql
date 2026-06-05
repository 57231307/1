-- 创建报表模板表
CREATE TABLE IF NOT EXISTS report_templates (
    id SERIAL PRIMARY KEY,
    tenant_id INTEGER NOT NULL DEFAULT 0,
    name VARCHAR(200) NOT NULL,
    code VARCHAR(100) NOT NULL,
    report_type VARCHAR(50) NOT NULL,
    columns JSONB NOT NULL DEFAULT '[]',
    filters JSONB,
    sort_by VARCHAR(100),
    sort_order VARCHAR(10),
    data_source_sql TEXT,
    description TEXT,
    is_public BOOLEAN NOT NULL DEFAULT false,
    status VARCHAR(20) NOT NULL DEFAULT 'ACTIVE',
    created_by INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(tenant_id, code)
);

CREATE INDEX IF NOT EXISTS idx_report_templates_tenant ON report_templates(tenant_id);
CREATE INDEX IF NOT EXISTS idx_report_templates_type ON report_templates(report_type);
