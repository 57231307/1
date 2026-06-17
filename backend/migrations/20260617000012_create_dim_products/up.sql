-- P3-4 BI 数据仓库：产品维表（SCD Type 2）
-- 保留历史版本：valid_from / valid_to / is_current
-- 多租户隔离：tenant_id 必填

CREATE TABLE IF NOT EXISTS dim_products (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    product_id BIGINT NOT NULL,
    product_code VARCHAR(50) NOT NULL,
    product_name VARCHAR(255) NOT NULL,
    category VARCHAR(100),
    color_no VARCHAR(50),
    fabric_type VARCHAR(50),
    valid_from DATE NOT NULL,
    valid_to DATE NOT NULL DEFAULT '9999-12-31',
    is_current BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 多租户 + 当前版本（业务查询）
CREATE INDEX IF NOT EXISTS idx_dim_products_tenant_current
    ON dim_products (tenant_id, product_id) WHERE is_current = true;

-- 多租户 + 时间范围（历史查询）
CREATE INDEX IF NOT EXISTS idx_dim_products_tenant_history
    ON dim_products (tenant_id, product_id, valid_from, valid_to);

-- 多租户 + 品类（按品类分析）
CREATE INDEX IF NOT EXISTS idx_dim_products_tenant_category
    ON dim_products (tenant_id, category) WHERE is_current = true;

COMMENT ON TABLE dim_products IS 'P3-4 BI 产品维表（SCD Type 2）';
COMMENT ON COLUMN dim_products.is_current IS '是否当前版本（SCD Type 2）';
