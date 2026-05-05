-- 为Admin角色(role_id=1)初始化所有权限
-- 这将确保admin用户可以访问所有资源

INSERT INTO role_permissions (role_id, resource_type, action, allowed, created_at, updated_at)
VALUES
-- 采购管理
(1, 'purchases', 'read', true, NOW(), NOW()),
(1, 'purchases', 'create', true, NOW(), NOW()),
(1, 'purchases', 'update', true, NOW(), NOW()),
(1, 'purchases', 'delete', true, NOW(), NOW()),

-- 销售管理
(1, 'sales', 'read', true, NOW(), NOW()),
(1, 'sales', 'create', true, NOW(), NOW()),
(1, 'sales', 'update', true, NOW(), NOW()),
(1, 'sales', 'delete', true, NOW(), NOW()),

-- 库存管理
(1, 'inventory', 'read', true, NOW(), NOW()),
(1, 'inventory', 'create', true, NOW(), NOW()),
(1, 'inventory', 'update', true, NOW(), NOW()),
(1, 'inventory', 'delete', true, NOW(), NOW()),

-- 财务管理
(1, 'finance', 'read', true, NOW(), NOW()),
(1, 'finance', 'create', true, NOW(), NOW()),
(1, 'finance', 'update', true, NOW(), NOW()),
(1, 'finance', 'delete', true, NOW(), NOW()),

-- 客户管理
(1, 'customers', 'read', true, NOW(), NOW()),
(1, 'customers', 'create', true, NOW(), NOW()),
(1, 'customers', 'update', true, NOW(), NOW()),
(1, 'customers', 'delete', true, NOW(), NOW()),

-- 供应商管理
(1, 'suppliers', 'read', true, NOW(), NOW()),
(1, 'suppliers', 'create', true, NOW(), NOW()),
(1, 'suppliers', 'update', true, NOW(), NOW()),
(1, 'suppliers', 'delete', true, NOW(), NOW()),

-- 产品管理
(1, 'products', 'read', true, NOW(), NOW()),
(1, 'products', 'create', true, NOW(), NOW()),
(1, 'products', 'update', true, NOW(), NOW()),
(1, 'products', 'delete', true, NOW(), NOW()),

-- 仓库管理
(1, 'warehouses', 'read', true, NOW(), NOW()),
(1, 'warehouses', 'create', true, NOW(), NOW()),
(1, 'warehouses', 'update', true, NOW(), NOW()),
(1, 'warehouses', 'delete', true, NOW(), NOW()),

-- 用户管理
(1, 'users', 'read', true, NOW(), NOW()),
(1, 'users', 'create', true, NOW(), NOW()),
(1, 'users', 'update', true, NOW(), NOW()),
(1, 'users', 'delete', true, NOW(), NOW()),

-- 审计跟踪
(1, 'audit', 'read', true, NOW(), NOW()),

-- 仪表板
(1, 'dashboard', 'read', true, NOW(), NOW())

ON CONFLICT (role_id, resource_type, action) DO NOTHING;

-- 验证插入结果
SELECT rp.*, r.name as role_name 
FROM role_permissions rp
LEFT JOIN roles r ON rp.role_id = r.id
WHERE rp.role_id = 1
ORDER BY rp.resource_type, rp.action;
