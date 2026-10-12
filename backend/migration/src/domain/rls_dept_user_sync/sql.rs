//! m_rls_dept_user_sync 的 SQL 常量(唯一来源)。
//!
//! 迁移 up/down 与 `backend/tests/rls_dept_user_sync_live_test.rs`(经 `#[path]` 直引)共用
//! 本文件,保证"真库活体回放的语句"与"迁移实际执行的语句"逐字符同一,杜绝测试侧
//! 另抄一份 SQL 造成漂移。全部语句仅 PostgreSQL 方言(本仓迁移即 PG)。
//!
//! 表清单与归属列与 m_rls_dept_domain(rls_dept/mod.rs:48-57)一一对应:
//! customers / crm_lead / crm_opportunity 按 owner_id,suppliers / sales_orders 按 created_by。

/// 备份表:存量回填前,把"将被改写的行"原值入表(参照 m0007 的精确可回退做法)。
/// old_department_id 可空——回填前的滞留值本身就可能是 NULL(公海/历史行)。
/// IF NOT EXISTS + 主键 (table_name,row_id) 保证重放安全。
pub const CREATE_BACKUP_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS "mig_user_dept_sync_backup" (
    "table_name"        TEXT    NOT NULL,
    "row_id"            INTEGER NOT NULL,
    "old_department_id" INTEGER,
    PRIMARY KEY ("table_name", "row_id")
)"#;

/// 回填前备份将被对齐的行(每表一条 SELECT,合并为单条 INSERT)。
/// WHERE 与 BACKFILL_ALIGN_SQL 完全同谓词:只备份真正会变的行;
/// NOT EXISTS 守卫使重放不重复插入(幂等)。
pub const BACKUP_BEFORE_ALIGN_SQL: &str = r#"
INSERT INTO "mig_user_dept_sync_backup" ("table_name", "row_id", "old_department_id")
SELECT 'customers', c.id, c.department_id
  FROM "customers" c JOIN "users" u ON c.owner_id = u.id
 WHERE c.owner_id <> 0 AND c.department_id IS DISTINCT FROM u.department_id
   AND NOT EXISTS (SELECT 1 FROM "mig_user_dept_sync_backup" b
                   WHERE b.table_name = 'customers' AND b.row_id = c.id)
UNION ALL
SELECT 'crm_lead', l.id, l.department_id
  FROM "crm_lead" l JOIN "users" u ON l.owner_id = u.id
 WHERE l.department_id IS DISTINCT FROM u.department_id
   AND NOT EXISTS (SELECT 1 FROM "mig_user_dept_sync_backup" b
                   WHERE b.table_name = 'crm_lead' AND b.row_id = l.id)
UNION ALL
SELECT 'crm_opportunity', o.id, o.department_id
  FROM "crm_opportunity" o JOIN "users" u ON o.owner_id = u.id
 WHERE o.department_id IS DISTINCT FROM u.department_id
   AND NOT EXISTS (SELECT 1 FROM "mig_user_dept_sync_backup" b
                   WHERE b.table_name = 'crm_opportunity' AND b.row_id = o.id)
UNION ALL
SELECT 'suppliers', s.id, s.department_id
  FROM "suppliers" s JOIN "users" u ON s.created_by = u.id
 WHERE s.department_id IS DISTINCT FROM u.department_id
   AND NOT EXISTS (SELECT 1 FROM "mig_user_dept_sync_backup" b
                   WHERE b.table_name = 'suppliers' AND b.row_id = s.id)
UNION ALL
SELECT 'sales_orders', so.id, so.department_id
  FROM "sales_orders" so JOIN "users" u ON so.created_by = u.id
 WHERE so.department_id IS DISTINCT FROM u.department_id
   AND NOT EXISTS (SELECT 1 FROM "mig_user_dept_sync_backup" b
                   WHERE b.table_name = 'sales_orders' AND b.row_id = so.id)"#;

/// 存量回填:5 表 department_id 按当前 users.department_id 对齐。
/// 形态复用 m_rls_dept_domain(rls_dept/mod.rs:48-57)的 UPDATE ... FROM(单条批量,非游标),
/// 额外 IS DISTINCT FROM 谓词使语句可重入:第二次执行起匹配 0 行、零影响。
pub const BACKFILL_ALIGN_SQL: &str = r#"
UPDATE customers c SET department_id = u.department_id
FROM users u WHERE c.owner_id = u.id AND c.owner_id <> 0
  AND c.department_id IS DISTINCT FROM u.department_id;
UPDATE crm_lead l SET department_id = u.department_id
FROM users u WHERE l.owner_id = u.id
  AND l.department_id IS DISTINCT FROM u.department_id;
UPDATE crm_opportunity o SET department_id = u.department_id
FROM users u WHERE o.owner_id = u.id
  AND o.department_id IS DISTINCT FROM u.department_id;
UPDATE suppliers s SET department_id = u.department_id
FROM users u WHERE s.created_by = u.id
  AND s.department_id IS DISTINCT FROM u.department_id;
UPDATE sales_orders so SET department_id = u.department_id
FROM users u WHERE so.created_by = u.id
  AND so.department_id IS DISTINCT FROM u.department_id;"#;

/// 性能支撑:新触发器按 created_by = NEW.id 批量重算 sales_orders,而该表现有索引
/// 不含 created_by(m0001:405-408 只有 order_no/customer_id/status/order_date;
/// 对照 finance 域已为 suppliers 建过 idx_suppliers_created_by,finance/mod.rs:115)。
/// 缺了它,每次用户换部门都对 sales_orders 全表顺序扫描。IF NOT EXISTS 幂等。
pub const CREATED_BY_INDEX_SQL: &str = r#"
CREATE INDEX IF NOT EXISTS idx_sales_orders_created_by ON sales_orders (created_by);"#;

/// 触发器函数:归属人在 users 表换部门时,对 5 表各一条批量 UPDATE 重算冗余列。
/// 在 users 行锁/事务内执行(同事务语义),不允许应用层逐行查询。
pub const SYNC_FUNCTION_SQL: &str = r#"
CREATE OR REPLACE FUNCTION sync_data_department_by_user_dept() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
  -- 表清单/归属列复用 m_rls_dept_domain(rls_dept/mod.rs:48-57)。只 SET department_id、
  -- 不触碰归属列,5 表各自的 BEFORE UPDATE OF owner_id/created_by 触发器不会再次触发
  -- (无递归)。NEW.department_id 为 NULL(用户被移出部门)时各行同步置 NULL——与
  -- rls_dept 触发函数反查不到 users 行置 NULL 的语义一致(rls_dept/mod.rs:72-73、81-83)。
  -- 公海哨兵(rls_dept/mod.rs:45-49):customers.owner_id=0 表示无归属人;users.id 自增
  -- 从 1 起,NEW.id=0 不应存在,显式排除以防异常行把全部公海行误染成某部门。
  IF NEW.id <> 0 THEN
    UPDATE customers c SET department_id = NEW.department_id
    WHERE c.owner_id = NEW.id AND c.owner_id <> 0;
  END IF;
  UPDATE crm_lead l SET department_id = NEW.department_id
  WHERE l.owner_id = NEW.id;
  UPDATE crm_opportunity o SET department_id = NEW.department_id
  WHERE o.owner_id = NEW.id;
  UPDATE suppliers s SET department_id = NEW.department_id
  WHERE s.created_by = NEW.id;
  UPDATE sales_orders so SET department_id = NEW.department_id
  WHERE so.created_by = NEW.id;
  RETURN NULL;
END $$;"#;

/// 触发时机取 AFTER UPDATE 而非 BEFORE:AFTER 时 users 行新值已写入本事务,
/// 与之并发提交的其他写路径经 5 表 BEFORE 触发函数反查 users.department_id
/// (rls_dept/mod.rs:78-85)读到的即改后值,两条触发路径在同事务内读同一部门,
/// 不存在"先改冗余列再被旧值覆写"的窗口;BEFORE 则函数体里反查到的仍是旧部门。
/// WHEN 谓词只在 department_id 真正变化时触发(UPDATE OF 列名命中但值不变时
/// PostgreSQL 默认也会逐行触发,这里显式收掉空转重算)。
pub const SYNC_TRIGGER_SQL: &str = r#"
DROP TRIGGER IF EXISTS trg_users_dept_sync ON users;
CREATE TRIGGER trg_users_dept_sync
  AFTER UPDATE OF department_id ON users
  FOR EACH ROW
  WHEN (OLD.department_id IS DISTINCT FROM NEW.department_id)
  EXECUTE FUNCTION sync_data_department_by_user_dept();"#;

/// 对齐不变量巡检(非 DDL,供迁移后校验与活体测试断言用):返回全部偏离
/// "行 department_id ≡ 归属人在 users 上的 department_id" 的 (表,行)。
/// 期望恒 0 行。不带结尾分号,便于嵌入子查询。
pub const ALIGNMENT_CHECK_SQL: &str = r#"
SELECT 'customers' AS table_name, c.id AS row_id
  FROM "customers" c JOIN "users" u ON c.owner_id = u.id
 WHERE c.owner_id <> 0 AND c.department_id IS DISTINCT FROM u.department_id
UNION ALL
SELECT 'crm_lead', l.id
  FROM "crm_lead" l JOIN "users" u ON l.owner_id = u.id
 WHERE l.department_id IS DISTINCT FROM u.department_id
UNION ALL
SELECT 'crm_opportunity', o.id
  FROM "crm_opportunity" o JOIN "users" u ON o.owner_id = u.id
 WHERE o.department_id IS DISTINCT FROM u.department_id
UNION ALL
SELECT 'suppliers', s.id
  FROM "suppliers" s JOIN "users" u ON s.created_by = u.id
 WHERE s.department_id IS DISTINCT FROM u.department_id
UNION ALL
SELECT 'sales_orders', so.id
  FROM "sales_orders" so JOIN "users" u ON so.created_by = u.id
 WHERE so.department_id IS DISTINCT FROM u.department_id"#;

/// down:先摘触发器与函数(回滚第一步——还原期间不得再重算)。
pub const DROP_TRIGGER_SQL: &str = r#"
DROP TRIGGER IF EXISTS trg_users_dept_sync ON users;"#;

pub const DROP_FUNCTION_SQL: &str = r#"
DROP FUNCTION IF EXISTS sync_data_department_by_user_dept() CASCADE;"#;

/// down:按备份表逐行还原回填前的原值(仅回填动过的行;备份没有的行=回填没改的行,
/// 保持原样)。UPDATE ... FROM 谓词限定表名字面量,重放安全。
/// 诚实边界:down 还原的是"本迁移 up 执行时点"的值;up 之后由本触发器产生的
/// 部门变化无法逆向(回滚语义即"回到 up 之前",与 m0007 同口径)。
pub const RESTORE_BACKUP_SQL: &str = r#"
UPDATE customers c SET department_id = b.old_department_id
FROM "mig_user_dept_sync_backup" b
WHERE b.table_name = 'customers' AND b.row_id = c.id;
UPDATE crm_lead l SET department_id = b.old_department_id
FROM "mig_user_dept_sync_backup" b
WHERE b.table_name = 'crm_lead' AND b.row_id = l.id;
UPDATE crm_opportunity o SET department_id = b.old_department_id
FROM "mig_user_dept_sync_backup" b
WHERE b.table_name = 'crm_opportunity' AND b.row_id = o.id;
UPDATE suppliers s SET department_id = b.old_department_id
FROM "mig_user_dept_sync_backup" b
WHERE b.table_name = 'suppliers' AND b.row_id = s.id;
UPDATE sales_orders so SET department_id = b.old_department_id
FROM "mig_user_dept_sync_backup" b
WHERE b.table_name = 'sales_orders' AND b.row_id = so.id;"#;

/// down:备份表用完即弃(回滚第二步;IF EXISTS 幂等)。
pub const DROP_BACKUP_TABLE_SQL: &str = r#"
DROP TABLE IF EXISTS "mig_user_dept_sync_backup";"#;
