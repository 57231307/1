-- Batch 464 P0-S25 回滚：移除行级数据权限 RLS 策略
--
-- 回滚顺序：先 DROP POLICY，再 DISABLE ROW LEVEL SECURITY
-- 注意：回滚后所有用户可见所有行（无行级隔离）

-- 5. crm_opportunity
DROP POLICY IF EXISTS crm_opportunity_isolation ON crm_opportunity;
ALTER TABLE crm_opportunity DISABLE ROW LEVEL SECURITY;

-- 4. crm_lead
DROP POLICY IF EXISTS crm_lead_isolation ON crm_lead;
ALTER TABLE crm_lead DISABLE ROW LEVEL SECURITY;

-- 3. sales_orders
DROP POLICY IF EXISTS sales_orders_isolation ON sales_orders;
ALTER TABLE sales_orders DISABLE ROW LEVEL SECURITY;

-- 2. suppliers
DROP POLICY IF EXISTS suppliers_isolation ON suppliers;
ALTER TABLE suppliers DISABLE ROW LEVEL SECURITY;

-- 1. customers
DROP POLICY IF EXISTS customers_isolation ON customers;
ALTER TABLE customers DISABLE ROW LEVEL SECURITY;
