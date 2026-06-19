-- 初始化角色权限数据迁移
-- 日期: 2026-05-09
-- 描述: 为系统角色初始化基础权限数据

BEGIN;

-- 确保管理员角色拥有所有权限
-- 注意: role_id=1 在代码中被硬编码为管理员角色，自动绕过权限检查

-- 为普通用户角色(role_id=2，假设存在)分配基础只读权限
INSERT INTO role_permissions (role_id, resource_type, resource_id, action, allowed, created_at, updated_at)
SELECT 2, 'products', NULL, 'read', true, NOW(), NOW()
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 2)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 2 AND resource_type = 'products' AND action = 'read');

INSERT INTO role_permissions (role_id, resource_type, resource_id, action, allowed, created_at, updated_at)
SELECT 2, 'customers', NULL, 'read', true, NOW(), NOW()
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 2)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 2 AND resource_type = 'customers' AND action = 'read');

INSERT INTO role_permissions (role_id, resource_type, resource_id, action, allowed, created_at, updated_at)
SELECT 2, 'suppliers', NULL, 'read', true, NOW(), NOW()
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 2)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 2 AND resource_type = 'suppliers' AND action = 'read');

INSERT INTO role_permissions (role_id, resource_type, resource_id, action, allowed, created_at, updated_at)
SELECT 2, 'sales', NULL, 'read', true, NOW(), NOW()
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 2)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 2 AND resource_type = 'sales' AND action = 'read');

INSERT INTO role_permissions (role_id, resource_type, resource_id, action, allowed, created_at, updated_at)
SELECT 2, 'purchases', NULL, 'read', true, NOW(), NOW()
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 2)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 2 AND resource_type = 'purchases' AND action = 'read');

INSERT INTO role_permissions (role_id, resource_type, resource_id, action, allowed, created_at, updated_at)
SELECT 2, 'inventory', NULL, 'read', true, NOW(), NOW()
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 2)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 2 AND resource_type = 'inventory' AND action = 'read');

INSERT INTO role_permissions (role_id, resource_type, resource_id, action, allowed, created_at, updated_at)
SELECT 2, 'finance', NULL, 'read', true, NOW(), NOW()
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 2)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 2 AND resource_type = 'finance' AND action = 'read');

-- 为销售经理角色(role_id=3，假设存在)分配销售相关权限
INSERT INTO role_permissions (role_id, resource_type, resource_id, action, allowed, created_at, updated_at)
SELECT 3, 'sales', NULL, 'create', true, NOW(), NOW()
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 3)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 3 AND resource_type = 'sales' AND action = 'create');

INSERT INTO role_permissions (role_id, resource_type, resource_id, action, allowed, created_at, updated_at)
SELECT 3, 'sales', NULL, 'update', true, NOW(), NOW()
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 3)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 3 AND resource_type = 'sales' AND action = 'update');

INSERT INTO role_permissions (role_id, resource_type, resource_id, action, allowed, created_at, updated_at)
SELECT 3, 'customers', NULL, 'create', true, NOW(), NOW()
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 3)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 3 AND resource_type = 'customers' AND action = 'create');

INSERT INTO role_permissions (role_id, resource_type, resource_id, action, allowed, created_at, updated_at)
SELECT 3, 'customers', NULL, 'update', true, NOW(), NOW()
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 3)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 3 AND resource_type = 'customers' AND action = 'update');

-- 为采购经理角色(role_id=4，假设存在)分配采购相关权限
INSERT INTO role_permissions (role_id, resource_type, resource_id, action, allowed, created_at, updated_at)
SELECT 4, 'purchases', NULL, 'create', true, NOW(), NOW()
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 4)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 4 AND resource_type = 'purchases' AND action = 'create');

INSERT INTO role_permissions (role_id, resource_type, resource_id, action, allowed, created_at, updated_at)
SELECT 4, 'purchases', NULL, 'update', true, NOW(), NOW()
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 4)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 4 AND resource_type = 'purchases' AND action = 'update');

INSERT INTO role_permissions (role_id, resource_type, resource_id, action, allowed, created_at, updated_at)
SELECT 4, 'suppliers', NULL, 'create', true, NOW(), NOW()
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 4)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 4 AND resource_type = 'suppliers' AND action = 'create');

INSERT INTO role_permissions (role_id, resource_type, resource_id, action, allowed, created_at, updated_at)
SELECT 4, 'suppliers', NULL, 'update', true, NOW(), NOW()
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 4)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 4 AND resource_type = 'suppliers' AND action = 'update');

-- 为财务角色(role_id=5，假设存在)分配财务相关权限
INSERT INTO role_permissions (role_id, resource_type, resource_id, action, allowed, created_at, updated_at)
SELECT 5, 'finance', NULL, 'create', true, NOW(), NOW()
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 5)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 5 AND resource_type = 'finance' AND action = 'create');

INSERT INTO role_permissions (role_id, resource_type, resource_id, action, allowed, created_at, updated_at)
SELECT 5, 'finance', NULL, 'update', true, NOW(), NOW()
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 5)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 5 AND resource_type = 'finance' AND action = 'update');

INSERT INTO role_permissions (role_id, resource_type, resource_id, action, allowed, created_at, updated_at)
SELECT 5, 'ap', NULL, 'read', true, NOW(), NOW()
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 5)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 5 AND resource_type = 'ap' AND action = 'read');

INSERT INTO role_permissions (role_id, resource_type, resource_id, action, allowed, created_at, updated_at)
SELECT 5, 'ar', NULL, 'read', true, NOW(), NOW()
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 5)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 5 AND resource_type = 'ar' AND action = 'read');

-- 为库存管理员角色(role_id=6，假设存在)分配库存相关权限
INSERT INTO role_permissions (role_id, resource_type, resource_id, action, allowed, created_at, updated_at)
SELECT 6, 'inventory', NULL, 'create', true, NOW(), NOW()
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 6)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 6 AND resource_type = 'inventory' AND action = 'create');

INSERT INTO role_permissions (role_id, resource_type, resource_id, action, allowed, created_at, updated_at)
SELECT 6, 'inventory', NULL, 'update', true, NOW(), NOW()
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 6)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 6 AND resource_type = 'inventory' AND action = 'update');

INSERT INTO role_permissions (role_id, resource_type, resource_id, action, allowed, created_at, updated_at)
SELECT 6, 'warehouses', NULL, 'read', true, NOW(), NOW()
WHERE EXISTS (SELECT 1 FROM roles WHERE id = 6)
  AND NOT EXISTS (SELECT 1 FROM role_permissions WHERE role_id = 6 AND resource_type = 'warehouses' AND action = 'read');

COMMIT;
