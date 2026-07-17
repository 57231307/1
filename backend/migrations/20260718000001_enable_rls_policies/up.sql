-- Batch 464 P0-S25：行级数据权限 RLS 策略启用
--
-- 为 5 张敏感表启用 PostgreSQL RLS，定义隔离策略：
--   customers / suppliers / sales_orders / crm_lead / crm_opportunity
--
-- 设计原则与策略详见 database/rls.sql（集中定义文件）
--
-- 安全降级机制：
--   当 app.user_id 未设置时（应用层未接入 SET LOCAL），current_setting 返回 NULL，
--   NULL IS NULL 为 true，允许所有访问，避免业务停摆。
--   后续批次实现 SET LOCAL 机制后，RLS 自动激活。
--
-- 注意：不可使用 FORCE ROW LEVEL SECURITY（会限制表 owner）

-- ============================================================================
-- 1. customers 表（owner_id NOT NULL，0 表示公海）
-- ============================================================================
ALTER TABLE customers ENABLE ROW LEVEL SECURITY;

CREATE POLICY customers_isolation ON customers
    FOR ALL
    USING (
        current_setting('app.user_id', true) IS NULL
        OR current_setting('app.role_code', true) IN ('admin', 'gm', 'deputy_gm')
        OR owner_id = current_setting('app.user_id', true)::int
        OR owner_id = 0
    )
    WITH CHECK (
        current_setting('app.user_id', true) IS NULL
        OR current_setting('app.role_code', true) IN ('admin', 'gm', 'deputy_gm')
        OR owner_id = current_setting('app.user_id', true)::int
        OR owner_id = 0
    );

-- ============================================================================
-- 2. suppliers 表（created_by 可空，历史数据 NULL）
-- ============================================================================
ALTER TABLE suppliers ENABLE ROW LEVEL SECURITY;

CREATE POLICY suppliers_isolation ON suppliers
    FOR ALL
    USING (
        current_setting('app.user_id', true) IS NULL
        OR current_setting('app.role_code', true) IN ('admin', 'gm', 'deputy_gm')
        OR created_by IS NULL
        OR created_by = current_setting('app.user_id', true)::int
    )
    WITH CHECK (
        current_setting('app.user_id', true) IS NULL
        OR current_setting('app.role_code', true) IN ('admin', 'gm', 'deputy_gm')
        OR created_by IS NULL
        OR created_by = current_setting('app.user_id', true)::int
    );

-- ============================================================================
-- 3. sales_orders 表（created_by 可空，历史数据 NULL）
-- ============================================================================
ALTER TABLE sales_orders ENABLE ROW LEVEL SECURITY;

CREATE POLICY sales_orders_isolation ON sales_orders
    FOR ALL
    USING (
        current_setting('app.user_id', true) IS NULL
        OR current_setting('app.role_code', true) IN ('admin', 'gm', 'deputy_gm')
        OR created_by IS NULL
        OR created_by = current_setting('app.user_id', true)::int
    )
    WITH CHECK (
        current_setting('app.user_id', true) IS NULL
        OR current_setting('app.role_code', true) IN ('admin', 'gm', 'deputy_gm')
        OR created_by IS NULL
        OR created_by = current_setting('app.user_id', true)::int
    );

-- ============================================================================
-- 4. crm_lead 表（owner_id NOT NULL）
-- ============================================================================
ALTER TABLE crm_lead ENABLE ROW LEVEL SECURITY;

CREATE POLICY crm_lead_isolation ON crm_lead
    FOR ALL
    USING (
        current_setting('app.user_id', true) IS NULL
        OR current_setting('app.role_code', true) IN ('admin', 'gm', 'deputy_gm')
        OR owner_id = current_setting('app.user_id', true)::int
    )
    WITH CHECK (
        current_setting('app.user_id', true) IS NULL
        OR current_setting('app.role_code', true) IN ('admin', 'gm', 'deputy_gm')
        OR owner_id = current_setting('app.user_id', true)::int
    );

-- ============================================================================
-- 5. crm_opportunity 表（owner_id NOT NULL）
-- ============================================================================
ALTER TABLE crm_opportunity ENABLE ROW LEVEL SECURITY;

CREATE POLICY crm_opportunity_isolation ON crm_opportunity
    FOR ALL
    USING (
        current_setting('app.user_id', true) IS NULL
        OR current_setting('app.role_code', true) IN ('admin', 'gm', 'deputy_gm')
        OR owner_id = current_setting('app.user_id', true)::int
    )
    WITH CHECK (
        current_setting('app.user_id', true) IS NULL
        OR current_setting('app.role_code', true) IN ('admin', 'gm', 'deputy_gm')
        OR owner_id = current_setting('app.user_id', true)::int
    );
