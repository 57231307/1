-- P3-4 BI 数据仓库：销售事实表
-- 多租户隔离：tenant_id 必填
-- 索引：tenant_id + order_date 倒序（按时间分析）

CREATE TABLE IF NOT EXISTS sales_facts (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    order_id BIGINT NOT NULL,
    order_date DATE NOT NULL,
    customer_id BIGINT NOT NULL,
    product_id BIGINT NOT NULL,
    region_id BIGINT,
    quantity NUMERIC(18, 4) NOT NULL,
    unit_price NUMERIC(18, 4) NOT NULL,
    total_amount NUMERIC(18, 4) NOT NULL,
    cost_amount NUMERIC(18, 4) NOT NULL,
    profit_amount NUMERIC(18, 4) NOT NULL,
    status VARCHAR(20) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 多租户 + 时间倒序联合索引（按时间分析）
CREATE INDEX IF NOT EXISTS idx_sales_facts_tenant_date
    ON sales_facts (tenant_id, order_date DESC);

-- 多租户 + 客户（按客户分析）
CREATE INDEX IF NOT EXISTS idx_sales_facts_tenant_customer
    ON sales_facts (tenant_id, customer_id, order_date DESC);

-- 多租户 + 产品（按产品分析）
CREATE INDEX IF NOT EXISTS idx_sales_facts_tenant_product
    ON sales_facts (tenant_id, product_id, order_date DESC);

-- 多租户 + 区域（按区域分析）
CREATE INDEX IF NOT EXISTS idx_sales_facts_tenant_region
    ON sales_facts (tenant_id, region_id, order_date DESC);

COMMENT ON TABLE sales_facts IS 'P3-4 BI 数据仓库：销售事实表（Star Schema fact table）';
COMMENT ON COLUMN sales_facts.tenant_id IS '租户 ID（多租户隔离强制字段）';
COMMENT ON COLUMN sales_facts.total_amount IS '销售额（quantity * unit_price）';
COMMENT ON COLUMN sales_facts.profit_amount IS '利润（total - cost）';
