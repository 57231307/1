-- ============================================================================
-- PostgreSQL 行级安全策略（Row Level Security, RLS）集中定义
--
-- Batch 464 P0-S25：行级数据权限 RLS
--
-- 设计原则：
-- 1. 纵深防御（defense-in-depth）
--    - 应用层：apply_data_scope（utils/data_scope.rs）已覆盖 5 张敏感表
--    - 数据库层：本文件定义的 RLS 策略，作为兜底防线
--    - 防止 SQL 注入、应用层 bug、内部直连 DB 越权
--
-- 2. 安全降级机制
--    - 当前应用层尚未实现 SET LOCAL app.user_id 机制（计划在后续批次实现）
--    - 当 app.user_id 未设置时，current_setting('app.user_id', true) 返回 NULL
--    - policy 中 NULL IS NULL 为 true，允许所有访问，避免业务停摆
--    - 后续批次实现 SET LOCAL 机制后，RLS 自动激活
--
-- 3. 角色分层
--    - admin/gm/deputy_gm：全公司数据（data_scope=all）
--    - manager 等部门角色：本部门数据（当前退化为本部门所有员工，因表无 department_id）
--    - operator 等员工角色：仅本人数据（data_scope=self）
--
-- 4. 公海数据可见性
--    - customers.owner_id=0 表示公海客户，对所有用户可见
--    - suppliers/sales_orders.created_by IS NULL 表示历史数据，对所有用户可见
--
-- 注意事项：
-- - 不可使用 FORCE ROW LEVEL SECURITY（会限制表 owner，导致迁移用户无法访问）
-- - 应用连接用户必须不是 SUPERUSER 且无 BYPASSRLS 属性
-- - CI 环境的 SUPERUSER 用户会绕过 RLS，仅生产环境生效
-- ============================================================================

-- ============================================================================
-- 1. customers 表 RLS（客户主数据）
-- ============================================================================
-- 过滤列：owner_id（NOT NULL，0 表示公海）
-- 公海客户（owner_id=0）对所有用户可见
ALTER TABLE customers ENABLE ROW LEVEL SECURITY;

CREATE POLICY customers_isolation ON customers
    FOR ALL
    USING (
        -- 安全降级：app.user_id 未设置时允许所有访问（应用层未接入 SET LOCAL）
        current_setting('app.user_id', true) IS NULL
        -- admin/gm/deputy_gm 角色可见所有数据
        OR current_setting('app.role_code', true) IN ('admin', 'gm', 'deputy_gm')
        -- 仅本人负责的客户
        OR owner_id = current_setting('app.user_id', true)::int
        -- 公海客户（owner_id=0）对所有用户可见
        OR owner_id = 0
    )
    WITH CHECK (
        -- 写入校验：同 USING 条件
        current_setting('app.user_id', true) IS NULL
        OR current_setting('app.role_code', true) IN ('admin', 'gm', 'deputy_gm')
        OR owner_id = current_setting('app.user_id', true)::int
        OR owner_id = 0
    );

-- ============================================================================
-- 2. suppliers 表 RLS（供应商主数据）
-- ============================================================================
-- 过滤列：created_by（Option<i32>，历史数据可能 NULL）
ALTER TABLE suppliers ENABLE ROW LEVEL SECURITY;

CREATE POLICY suppliers_isolation ON suppliers
    FOR ALL
    USING (
        current_setting('app.user_id', true) IS NULL
        OR current_setting('app.role_code', true) IN ('admin', 'gm', 'deputy_gm')
        -- 历史数据 NULL 行对所有用户可见（避免数据丢失）
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
-- 3. sales_orders 表 RLS（销售订单）
-- ============================================================================
-- 过滤列：created_by（Option<i32>，历史数据可能 NULL）
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
-- 4. crm_lead 表 RLS（CRM 线索）
-- ============================================================================
-- 过滤列：owner_id（NOT NULL，创建时必填）
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
-- 5. crm_opportunity 表 RLS（CRM 商机）
-- ============================================================================
-- 过滤列：owner_id（NOT NULL，创建时必填）
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

-- ============================================================================
-- 后续批次计划
-- ============================================================================
-- 1. 应用层 SET LOCAL 机制（auth_context middleware）
--    - 每个请求事务开始时执行 SET LOCAL app.user_id / app.role_code
--    - 必须用事务包裹查询（DatabaseConnection.transaction）
--    - 注意连接池 session 变量泄漏风险（SET LOCAL 仅事务内有效）
--
-- 2. 双用户部署架构
--    - bingxi_app：应用连接，NOSUPERUSER 无 BYPASSRLS
--    - bingxi_migrate：迁移用，BYPASSRLS
--
-- 3. department_id 字段扩展（可选）
--    - 当前所有敏感表无 department_id，dept 数据范围退化为 self
--    - 若需恢复 dept 语义，需新增 department_id 列并填充
-- ============================================================================
