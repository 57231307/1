-- 撤销 P3-4 销售事实表
DROP INDEX IF EXISTS idx_sales_facts_tenant_date;
DROP INDEX IF EXISTS idx_sales_facts_tenant_customer;
DROP INDEX IF EXISTS idx_sales_facts_tenant_product;
DROP INDEX IF EXISTS idx_sales_facts_tenant_region;
DROP TABLE IF EXISTS sales_facts;
