-- 初始化角色权限数据迁移
-- 日期: 2026-05-09
-- 描述: 为系统角色初始化基础权限数据

BEGIN;

-- 确保管理员角色拥有所有权限
-- 注意: role_id=1 在代码中被硬编码为管理员角色，自动绕过权限检查

-- 为普通用户角色分配基础只读权限
INSERT INTO role_permissions (role_id, permission_code, permission_name)
SELECT 2, 'products.read', '查看产品'
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 2)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 2 AND permission_code = 'products.read');

INSERT INTO role_permissions (role_id, permission_code, permission_name)
SELECT 2, 'customers.read', '查看客户'
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 2)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 2 AND permission_code = 'customers.read');

INSERT INTO role_permissions (role_id, permission_code, permission_name)
SELECT 2, 'suppliers.read', '查看供应商'
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 2)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 2 AND permission_code = 'suppliers.read');

INSERT INTO role_permissions (role_id, permission_code, permission_name)
SELECT 2, 'sales.read', '查看销售'
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 2)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 2 AND permission_code = 'sales.read');

INSERT INTO role_permissions (role_id, permission_code, permission_name)
SELECT 2, 'purchases.read', '查看采购'
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 2)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 2 AND permission_code = 'purchases.read');

INSERT INTO role_permissions (role_id, permission_code, permission_name)
SELECT 2, 'inventory.read', '查看库存'
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 2)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 2 AND permission_code = 'inventory.read');

INSERT INTO role_permissions (role_id, permission_code, permission_name)
SELECT 2, 'finance.read', '查看财务'
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 2)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 2 AND permission_code = 'finance.read');

-- 为销售经理角色分配销售相关权限
INSERT INTO role_permissions (role_id, permission_code, permission_name)
SELECT 3, 'sales.create', '创建销售'
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 3)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 3 AND permission_code = 'sales.create');

INSERT INTO role_permissions (role_id, permission_code, permission_name)
SELECT 3, 'sales.update', '编辑销售'
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 3)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 3 AND permission_code = 'sales.update');

INSERT INTO role_permissions (role_id, permission_code, permission_name)
SELECT 3, 'customers.create', '创建客户'
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 3)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 3 AND permission_code = 'customers.create');

-- 为采购经理角色分配采购相关权限
INSERT INTO role_permissions (role_id, permission_code, permission_name)
SELECT 4, 'purchases.create', '创建采购'
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 4)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 4 AND permission_code = 'purchases.create');

INSERT INTO role_permissions (role_id, permission_code, permission_name)
SELECT 4, 'purchases.update', '编辑采购'
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 4)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 4 AND permission_code = 'purchases.update');

INSERT INTO role_permissions (role_id, permission_code, permission_name)
SELECT 4, 'suppliers.create', '创建供应商'
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 4)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 4 AND permission_code = 'suppliers.create');

-- 为财务经理角色分配财务相关权限
INSERT INTO role_permissions (role_id, permission_code, permission_name)
SELECT 5, 'finance.create', '创建财务'
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 5)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 5 AND permission_code = 'finance.create');

INSERT INTO role_permissions (role_id, permission_code, permission_name)
SELECT 5, 'finance.update', '编辑财务'
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 5)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 5 AND permission_code = 'finance.update');

INSERT INTO role_permissions (role_id, permission_code, permission_name)
SELECT 5, 'finance.delete', '删除财务'
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 5)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 5 AND permission_code = 'finance.delete');

COMMIT;
