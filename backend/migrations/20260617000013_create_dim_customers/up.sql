-- P3-4 BI 数据仓库：客户维表（SCD Type 2）
-- 保留历史版本：valid_from / valid_to / is_current

CREATE TABLE IF NOT EXISTS dim_customers (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    customer_id BIGINT NOT NULL,
    customer_code VARCHAR(50) NOT NULL,
    customer_name VARCHAR(255) NOT NULL,
    customer_type VARCHAR(50),
    region VARCHAR(100),
    industry VARCHAR(100),
    valid_from DATE NOT NULL,
    valid_to DATE NOT NULL DEFAULT '9999-12-31',
    is_current BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 多租户 + 当前版本
CREATE INDEX IF NOT EXISTS idx_dim_customers_tenant_current
    ON dim_customers (tenant_id, customer_id) WHERE is_current = true;

-- 多租户 + 区域（按区域分析）
CREATE INDEX IF NOT EXISTS idx_dim_customers_tenant_region
    ON dim_customers (tenant_id, region) WHERE is_current = true;

-- 多租户 + 客户类型
CREATE INDEX IF NOT EXISTS idx_dim_customers_tenant_type
    ON dim_customers (tenant_id, customer_type) WHERE is_current = true;

COMMENT ON TABLE dim_customers IS 'P3-4 BI 客户维表（SCD Type 2）';
