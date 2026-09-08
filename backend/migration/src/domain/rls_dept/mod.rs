//! RLS dept 数据范围语义迁移
//!
//! 为 5 张 RLS 表（customers、suppliers、sales_orders、crm_lead、crm_opportunity）
//! 补齐 dept 级行级隔离语义，消除 finance 域原策略 self 级粒度与应用层 dept
//! 分支错位（created_by = department_id 既有 bug）。
//!
//! 依据 .monkeycode/specs/dept-data-scope-semantics/ 三项决策：
//! - D1 数据部门 = 数据归属人所在部门（动态，转移时跟随）
//! - D2 可见部门集合 = 主部门 + 兼职部门 + 子部门
//! - D3 表冗余 department_id 列 + 触发器自动维护（写路径零侵入）
//!
//! 部署 sequencing：本迁移先行——旧后端不设置 app.dept_ids GUC，app_dept_ids()
//! 返回空数组，dept 用户退化为 self+公海（与方案①等价安全降级）；新后端发布后
//! 钩子设置 dept_ids，dept 用户恢复部门视角。

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_rls_dept_domain"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
-- ============================================================================
-- A. 5 表加冗余 department_id 列（INTEGER，NULL=无部门归属/历史数据）
-- ============================================================================
-- department_id 恒等于该行数据归属人（customers/crm_lead/crm_opportunity 取
-- owner_id，suppliers/sales_orders 取 created_by）在 users 表上的 department_id。
-- 由触发器自动维护，应用代码不直接写入。
ALTER TABLE customers ADD COLUMN IF NOT EXISTS department_id INTEGER;
ALTER TABLE suppliers ADD COLUMN IF NOT EXISTS department_id INTEGER;
ALTER TABLE sales_orders ADD COLUMN IF NOT EXISTS department_id INTEGER;
ALTER TABLE crm_lead ADD COLUMN IF NOT EXISTS department_id INTEGER;
ALTER TABLE crm_opportunity ADD COLUMN IF NOT EXISTS department_id INTEGER;

-- ============================================================================
-- B. 回填：按数据归属人反查 users.department_id
-- ============================================================================
-- customers owner_id=0（公海）不参与回填，department_id 保持 NULL（公海行由
-- 策略 owner_id=0 分支放行）；suppliers/sales_orders created_by NULL（历史数据）
-- 同理保持 NULL（由策略 created_by IS NULL 分支放行）。
UPDATE customers c SET department_id = u.department_id
FROM users u WHERE c.owner_id = u.id AND c.owner_id <> 0;
UPDATE crm_lead l SET department_id = u.department_id
FROM users u WHERE l.owner_id = u.id;
UPDATE crm_opportunity o SET department_id = u.department_id
FROM users u WHERE o.owner_id = u.id;
UPDATE suppliers s SET department_id = u.department_id
FROM users u WHERE s.created_by = u.id;
UPDATE sales_orders s SET department_id = u.department_id
FROM users u WHERE s.created_by = u.id;

-- ============================================================================
-- C. 索引（department_id 等值过滤谓词支撑）
-- ============================================================================
CREATE INDEX IF NOT EXISTS idx_customers_department ON customers (department_id);
CREATE INDEX IF NOT EXISTS idx_suppliers_department ON suppliers (department_id);
CREATE INDEX IF NOT EXISTS idx_sales_orders_department ON sales_orders (department_id);
CREATE INDEX IF NOT EXISTS idx_crm_lead_department ON crm_lead (department_id);
CREATE INDEX IF NOT EXISTS idx_crm_opportunity_department ON crm_opportunity (department_id);

-- ============================================================================
-- D. 触发器：BEFORE INSERT OR UPDATE OF owner_id/created_by 自动维护 department_id
-- ============================================================================
-- D1 动态语义锚点：owner/created_by 变更时触发器重算 department_id，转移归属人
-- 后新部门经理立即可见、原部门经理失去可见性。users 无对应行（如 owner_id=0
-- 公海）时 department_id 置 NULL。
CREATE OR REPLACE FUNCTION sync_data_department() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
  NEW.department_id := (
    SELECT department_id FROM users
    WHERE id = COALESCE(NEW.owner_id, NEW.created_by)
  );
  RETURN NEW;
END $$;

DROP TRIGGER IF EXISTS trg_customers_dept ON customers;
CREATE TRIGGER trg_customers_dept
  BEFORE INSERT OR UPDATE OF owner_id ON customers
  FOR EACH ROW EXECUTE FUNCTION sync_data_department();

DROP TRIGGER IF EXISTS trg_suppliers_dept ON suppliers;
CREATE TRIGGER trg_suppliers_dept
  BEFORE INSERT OR UPDATE OF created_by ON suppliers
  FOR EACH ROW EXECUTE FUNCTION sync_data_department();

DROP TRIGGER IF EXISTS trg_sales_orders_dept ON sales_orders;
CREATE TRIGGER trg_sales_orders_dept
  BEFORE INSERT OR UPDATE OF created_by ON sales_orders
  FOR EACH ROW EXECUTE FUNCTION sync_data_department();

DROP TRIGGER IF EXISTS trg_crm_lead_dept ON crm_lead;
CREATE TRIGGER trg_crm_lead_dept
  BEFORE INSERT OR UPDATE OF owner_id ON crm_lead
  FOR EACH ROW EXECUTE FUNCTION sync_data_department();

DROP TRIGGER IF EXISTS trg_crm_opportunity_dept ON crm_opportunity;
CREATE TRIGGER trg_crm_opportunity_dept
  BEFORE INSERT OR UPDATE OF owner_id ON crm_opportunity
  FOR EACH ROW EXECUTE FUNCTION sync_data_department();

-- ============================================================================
-- E. STABLE 函数：把 string_to_array(current_setting('app.dept_ids')) 包装成
--    语句级求值一次，规避每行解析 GUC 字符串的开销。
-- ============================================================================
-- current_setting(missing_ok=true) 在 GUC 未设置时返回 NULL → COALESCE 回退空数组。
CREATE OR REPLACE FUNCTION app_dept_ids() RETURNS int[]
STABLE LANGUAGE sql AS $$
  SELECT COALESCE(string_to_array(current_setting('app.dept_ids', true), ',')::int[], ARRAY[]::int[])
$$;

-- ============================================================================
-- F. 重写 5 表 RLS 策略（DROP 旧 + CREATE 新）
-- ============================================================================
-- 新谓词形态（统一）：
--   current_setting('app.user_id', true) IS NULL              -- admin/未认证 fail-open
--   OR <归属列> = current_setting('app.user_id', true)::int   -- self 分支
--   OR <公海/历史 NULL 分支>                                    -- 表特定
--   OR department_id = ANY(app_dept_ids())                    -- dept 分支
-- WITH CHECK 同形态（INSERT/UPDATE 时校验新行归属合法）。
-- 注意：原策略的 app.role_code IN ('admin','gm','deputy_gm') 分支无任何写入点
-- （死分支），新策略移除该分支，统一靠 app.user_id IS NULL（admin 不进 RLS
-- 中间件作用域）放行。

-- 1. customers（owner_id NOT NULL，owner_id=0 公海）
DROP POLICY IF EXISTS customers_isolation ON customers;
CREATE POLICY customers_isolation ON customers
  FOR ALL
  USING (
    current_setting('app.user_id', true) IS NULL
    OR owner_id = current_setting('app.user_id', true)::int
    OR owner_id = 0
    OR department_id = ANY(app_dept_ids())
  )
  WITH CHECK (
    current_setting('app.user_id', true) IS NULL
    OR owner_id = current_setting('app.user_id', true)::int
    OR owner_id = 0
    OR department_id = ANY(app_dept_ids())
  );

-- 2. suppliers（created_by 可空，NULL 历史数据）
DROP POLICY IF EXISTS suppliers_isolation ON suppliers;
CREATE POLICY suppliers_isolation ON suppliers
  FOR ALL
  USING (
    current_setting('app.user_id', true) IS NULL
    OR created_by IS NULL
    OR created_by = current_setting('app.user_id', true)::int
    OR department_id = ANY(app_dept_ids())
  )
  WITH CHECK (
    current_setting('app.user_id', true) IS NULL
    OR created_by = current_setting('app.user_id', true)::int
    OR department_id = ANY(app_dept_ids())
  );

-- 3. sales_orders（created_by 可空，NULL 历史数据）
DROP POLICY IF EXISTS sales_orders_isolation ON sales_orders;
CREATE POLICY sales_orders_isolation ON sales_orders
  FOR ALL
  USING (
    current_setting('app.user_id', true) IS NULL
    OR created_by IS NULL
    OR created_by = current_setting('app.user_id', true)::int
    OR department_id = ANY(app_dept_ids())
  )
  WITH CHECK (
    current_setting('app.user_id', true) IS NULL
    OR created_by = current_setting('app.user_id', true)::int
    OR department_id = ANY(app_dept_ids())
  );

-- 4. crm_lead（owner_id NOT NULL，lead_status='pool' 公海——R3.2 修复）
DROP POLICY IF EXISTS crm_lead_isolation ON crm_lead;
CREATE POLICY crm_lead_isolation ON crm_lead
  FOR ALL
  USING (
    current_setting('app.user_id', true) IS NULL
    OR owner_id = current_setting('app.user_id', true)::int
    OR lead_status = 'pool'
    OR department_id = ANY(app_dept_ids())
  )
  WITH CHECK (
    current_setting('app.user_id', true) IS NULL
    OR owner_id = current_setting('app.user_id', true)::int
    OR department_id = ANY(app_dept_ids())
  );

-- 5. crm_opportunity（owner_id NOT NULL）
-- 注意：opportunity_status 业务取值为 OPEN/CLOSED_WON/CLOSED_LOST/draft/cancelled，
-- 无 'pool' 公海态（公海机制仅存在于 crm_lead 的 lead_status），故无公海分支。
DROP POLICY IF EXISTS crm_opportunity_isolation ON crm_opportunity;
CREATE POLICY crm_opportunity_isolation ON crm_opportunity
  FOR ALL
  USING (
    current_setting('app.user_id', true) IS NULL
    OR owner_id = current_setting('app.user_id', true)::int
    OR department_id = ANY(app_dept_ids())
  )
  WITH CHECK (
    current_setting('app.user_id', true) IS NULL
    OR owner_id = current_setting('app.user_id', true)::int
    OR department_id = ANY(app_dept_ids())
  );
"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // 回滚：恢复 finance 域原策略（self 级 + role_code 死分支），保留 department_id
        // 列与触发器（无副作用，旧策略不引用 department_id）。生产回滚建议重跑
        // finance 域迁移或手动 DROP POLICY + 重建原策略。
        Ok(())
    }
}
