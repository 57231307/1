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

-- V15 P1-14.4-C：补齐模块前缀消歧后的资源类型权限（与 extract_resource_info 对齐）
-- 采购域资源（purchase/orders → purchase-orders 等）
(1, 'purchase-orders', 'read', true, NOW(), NOW()),
(1, 'purchase-orders', 'create', true, NOW(), NOW()),
(1, 'purchase-orders', 'update', true, NOW(), NOW()),
(1, 'purchase-orders', 'delete', true, NOW(), NOW()),
(1, 'purchase-orders', 'approve', true, NOW(), NOW()),
(1, 'purchase-orders', 'reject', true, NOW(), NOW()),
(1, 'purchase-receipts', 'read', true, NOW(), NOW()),
(1, 'purchase-receipts', 'create', true, NOW(), NOW()),
(1, 'purchase-returns', 'read', true, NOW(), NOW()),
(1, 'purchase-returns', 'approve', true, NOW(), NOW()),
(1, 'purchase-returns', 'reject', true, NOW(), NOW()),
(1, 'purchase-contracts', 'read', true, NOW(), NOW()),
(1, 'purchase-contracts', 'approve', true, NOW(), NOW()),
(1, 'purchase-prices', 'read', true, NOW(), NOW()),
(1, 'purchase-prices', 'approve', true, NOW(), NOW()),

-- 销售域资源（sales/returns → sales-returns 等，orders 保留原名）
(1, 'orders', 'read', true, NOW(), NOW()),
(1, 'orders', 'create', true, NOW(), NOW()),
(1, 'orders', 'update', true, NOW(), NOW()),
(1, 'orders', 'delete', true, NOW(), NOW()),
(1, 'orders', 'approve', true, NOW(), NOW()),
(1, 'orders', 'reject', true, NOW(), NOW()),
(1, 'sales-returns', 'read', true, NOW(), NOW()),
(1, 'sales-returns', 'approve', true, NOW(), NOW()),
(1, 'sales-returns', 'reject', true, NOW(), NOW()),
(1, 'sales-contracts', 'read', true, NOW(), NOW()),
(1, 'sales-contracts', 'approve', true, NOW(), NOW()),
(1, 'sales-prices', 'read', true, NOW(), NOW()),
(1, 'sales-prices', 'approve', true, NOW(), NOW()),

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

-- V15 P1-14.2-C：admin 不再持有 audit:read，违反职责分离（admin 既是操作者又能审计自己）
-- 审计职责独立到 auditor 角色（见下方 auditor 角色权限种子）
-- 原 (1, 'audit', 'read', true, NOW(), NOW()) 已删除

-- 仪表板
(1, 'dashboard', 'read', true, NOW(), NOW()),

-- V15 P1 10.4-1：色卡发放管理权限（5 个权限码，admin 全部允许）
-- 权限码格式：resource_type=color_card_issue + action=<操作>
-- 业务角色矩阵见 docs/rbac-permission-matrix.md
(1, 'color_card_issue', 'create', true, NOW(), NOW()),   -- 发放色卡（仓库员/仓库经理/销售）
(1, 'color_card_issue', 'return', true, NOW(), NOW()),   -- 归还色卡（仓库员/仓库经理）
(1, 'color_card_issue', 'lost', true, NOW(), NOW()),      -- 登记遗失（仓库员/仓库经理）
(1, 'color_card_issue', 'damaged', true, NOW(), NOW()),   -- 标记损坏（仓库员/仓库经理）
(1, 'color_card_issue', 'cancel', true, NOW(), NOW()),    -- 取消发放（仓库经理/admin）
(1, 'color_card_issue', 'read', true, NOW(), NOW()),      -- 查看发放记录（销售/客户服务/仓库/admin）

-- V15 P1 4.1：AI 工艺优化端点权限码（admin 全部允许）
(1, 'process-optimizations', 'read', true, NOW(), NOW()),
(1, 'process-optimizations', 'create', true, NOW(), NOW()),
(1, 'process-optimizations', 'update', true, NOW(), NOW()),
(1, 'process-optimizations', 'delete', true, NOW(), NOW()),

-- V15 P1 4.1：AI 质量预测端点权限码
(1, 'quality-predictions', 'read', true, NOW(), NOW()),
(1, 'quality-predictions', 'create', true, NOW(), NOW()),
(1, 'quality-predictions', 'update', true, NOW(), NOW()),
(1, 'quality-predictions', 'delete', true, NOW(), NOW()),

-- V15 P1 4.1：AI 看板与健康检查权限码
(1, 'summary', 'read', true, NOW(), NOW()),
(1, 'health', 'read', true, NOW(), NOW()),

-- V15 P1 4.1：advanced 域 AI 子资源细粒度权限码（6 个 AI 子资源）
(1, 'recipe-optimization', 'create', true, NOW(), NOW()),
(1, 'quality-prediction', 'create', true, NOW(), NOW()),
(1, 'sales-forecast', 'create', true, NOW(), NOW()),
(1, 'inventory-optimization', 'create', true, NOW(), NOW()),
(1, 'anomaly-detection', 'create', true, NOW(), NOW()),
(1, 'recommendations', 'read', true, NOW(), NOW())

ON CONFLICT (role_id, resource_type, action) DO NOTHING;

-- V15 P1-14.2-C：auditor 角色权限种子（审计职责独立，admin 不再持有 audit:read）
-- auditor 角色 id 通过子查询获取（避免硬编码 role_id），仅授予 audit:read 权限
-- 注意：auditor 角色需先由 init_service.rs::create_default_roles 创建（P0 14.1-B 修复后）
INSERT INTO role_permissions (role_id, resource_type, action, allowed, created_at, updated_at)
SELECT r.id, 'audit', 'read', true, NOW(), NOW()
FROM roles r
WHERE r.code = 'auditor'
ON CONFLICT (role_id, resource_type, action) DO NOTHING;

-- V15 P1-14.7-B：为所有非 admin 角色分配 dashboard:read 权限
-- 业务角色（manager/operator/auditor 等）登录后需进入仪表板，原仅 admin 持有 dashboard:read
INSERT INTO role_permissions (role_id, resource_type, action, allowed, created_at, updated_at)
SELECT r.id, 'dashboard', 'read', true, NOW(), NOW()
FROM roles r
WHERE r.code != 'admin' AND r.is_system = false
ON CONFLICT (role_id, resource_type, action) DO NOTHING;

-- P1 batch-11/12：为 admin 补齐 print/export action 权限码（否则导出/打印全量 403）
INSERT INTO role_permissions (role_id, resource_type, action, allowed, created_at, updated_at)
VALUES
(1, 'purchases', 'print', true, NOW(), NOW()),
(1, 'purchases', 'export', true, NOW(), NOW()),
(1, 'sales', 'print', true, NOW(), NOW()),
(1, 'sales', 'export', true, NOW(), NOW()),
(1, 'inventory', 'print', true, NOW(), NOW()),
(1, 'inventory', 'export', true, NOW(), NOW()),
(1, 'finance', 'print', true, NOW(), NOW()),
(1, 'finance', 'export', true, NOW(), NOW()),
(1, 'customers', 'print', true, NOW(), NOW()),
(1, 'customers', 'export', true, NOW(), NOW()),
(1, 'suppliers', 'print', true, NOW(), NOW()),
(1, 'suppliers', 'export', true, NOW(), NOW()),
(1, 'products', 'print', true, NOW(), NOW()),
(1, 'products', 'export', true, NOW(), NOW()),
(1, 'warehouses', 'print', true, NOW(), NOW()),
(1, 'warehouses', 'export', true, NOW(), NOW()),
(1, 'orders', 'print', true, NOW(), NOW()),
(1, 'orders', 'export', true, NOW(), NOW()),
(1, 'color_card_issue', 'print', true, NOW(), NOW()),
(1, 'color_card_issue', 'export', true, NOW(), NOW()),
(1, 'audit', 'export', true, NOW(), NOW()),
(1, 'reports', 'print', true, NOW(), NOW()),
(1, 'reports', 'export', true, NOW(), NOW()),
(1, 'dye_batches', 'print', true, NOW(), NOW()),
(1, 'dye_batches', 'export', true, NOW(), NOW()),
(1, 'wage_records', 'print', true, NOW(), NOW()),
(1, 'wage_records', 'export', true, NOW(), NOW()),
(1, 'energy', 'print', true, NOW(), NOW()),
(1, 'energy', 'export', true, NOW(), NOW()),
-- 采购域消歧资源（与首段 read/approve/reject 对齐）
(1, 'purchase-orders', 'print', true, NOW(), NOW()),
(1, 'purchase-orders', 'export', true, NOW(), NOW()),
(1, 'purchase-receipts', 'print', true, NOW(), NOW()),
(1, 'purchase-receipts', 'export', true, NOW(), NOW()),
(1, 'purchase-returns', 'print', true, NOW(), NOW()),
(1, 'purchase-returns', 'export', true, NOW(), NOW()),
(1, 'purchase-contracts', 'print', true, NOW(), NOW()),
(1, 'purchase-contracts', 'export', true, NOW(), NOW()),
(1, 'purchase-prices', 'print', true, NOW(), NOW()),
(1, 'purchase-prices', 'export', true, NOW(), NOW()),
-- 销售域消歧资源（orders 保留原名，returns/contracts/prices 消歧）
(1, 'sales-returns', 'print', true, NOW(), NOW()),
(1, 'sales-returns', 'export', true, NOW(), NOW()),
(1, 'sales-contracts', 'print', true, NOW(), NOW()),
(1, 'sales-contracts', 'export', true, NOW(), NOW()),
(1, 'sales-prices', 'print', true, NOW(), NOW()),
(1, 'sales-prices', 'export', true, NOW(), NOW()),
-- 其他有 read 但缺 print/export 的资源
(1, 'users', 'print', true, NOW(), NOW()),
(1, 'users', 'export', true, NOW(), NOW()),
(1, 'process-optimizations', 'print', true, NOW(), NOW()),
(1, 'process-optimizations', 'export', true, NOW(), NOW()),
(1, 'quality-predictions', 'print', true, NOW(), NOW()),
(1, 'quality-predictions', 'export', true, NOW(), NOW()),
(1, 'recommendations', 'print', true, NOW(), NOW()),
(1, 'recommendations', 'export', true, NOW(), NOW())
ON CONFLICT (role_id, resource_type, action) DO NOTHING;

-- 6 个业务角色差异化权限矩阵（替代原 4 角色共享矩阵）
-- 依据 docs/rbac-permission-matrix.md 与 init_service_ops/permission.rs 角色定义

-- sales_manager：销售经理（订单审批 + 客户管理 + 销售分析，SoD 拆分 create 与 approve）
INSERT INTO role_permissions (role_id, resource_type, action, allowed, created_at, updated_at)
SELECT r.id, t.resource_type, t.action, true, NOW(), NOW()
FROM roles r
CROSS JOIN (
    VALUES
    ('dashboard', 'read'),
    ('orders', 'read'), ('orders', 'update'), ('orders', 'approve'), ('orders', 'reject'),
    ('customers', 'read'), ('customers', 'create'), ('customers', 'update'),
    ('products', 'read'),
    ('sales-returns', 'read'), ('sales-returns', 'approve'), ('sales-returns', 'reject'),
    ('sales-contracts', 'read'), ('sales-contracts', 'approve'),
    ('sales-prices', 'read'), ('sales-prices', 'approve'),
    ('inventory', 'read'),
    ('reports', 'read'), ('reports', 'export'),
    ('color_card_issue', 'read'), ('color_card_issue', 'create')
) AS t(resource_type, action)
WHERE r.code = 'sales_manager' AND r.is_system = true
ON CONFLICT (role_id, resource_type, action) DO NOTHING;

-- warehouse_manager：仓库经理（库存全权 + 色卡发放全流程含 cancel + 入库验收只读）
INSERT INTO role_permissions (role_id, resource_type, action, allowed, created_at, updated_at)
SELECT r.id, t.resource_type, t.action, true, NOW(), NOW()
FROM roles r
CROSS JOIN (
    VALUES
    ('dashboard', 'read'),
    ('inventory', 'read'), ('inventory', 'create'), ('inventory', 'update'), ('inventory', 'delete'),
    ('warehouses', 'read'),
    ('products', 'read'),
    ('orders', 'read'),
    ('purchase-orders', 'read'),
    ('customers', 'read'), ('suppliers', 'read'),
    ('color_card_issue', 'read'), ('color_card_issue', 'create'),
    ('color_card_issue', 'return'), ('color_card_issue', 'lost'),
    ('color_card_issue', 'damaged'), ('color_card_issue', 'cancel'),
    ('reports', 'read'), ('reports', 'export')
) AS t(resource_type, action)
WHERE r.code = 'warehouse_manager' AND r.is_system = true
ON CONFLICT (role_id, resource_type, action) DO NOTHING;

-- production_manager：生产经理（染缸全权 + 生产计划 + 工艺只读）
INSERT INTO role_permissions (role_id, resource_type, action, allowed, created_at, updated_at)
SELECT r.id, t.resource_type, t.action, true, NOW(), NOW()
FROM roles r
CROSS JOIN (
    VALUES
    ('dashboard', 'read'),
    ('dye_batches', 'read'), ('dye_batches', 'create'), ('dye_batches', 'update'), ('dye_batches', 'export'),
    ('inventory', 'read'),
    ('products', 'read'),
    ('orders', 'read'),
    ('color_card_issue', 'read'),
    ('reports', 'read'), ('reports', 'export')
) AS t(resource_type, action)
WHERE r.code = 'production_manager' AND r.is_system = true
ON CONFLICT (role_id, resource_type, action) DO NOTHING;

-- lab_technician：化验室技术员（染缸只读 + 色卡只读 + 产品只读）
INSERT INTO role_permissions (role_id, resource_type, action, allowed, created_at, updated_at)
SELECT r.id, t.resource_type, t.action, true, NOW(), NOW()
FROM roles r
CROSS JOIN (
    VALUES
    ('dashboard', 'read'),
    ('dye_batches', 'read'), ('dye_batches', 'export'),
    ('color_card_issue', 'read'),
    ('products', 'read'),
    ('inventory', 'read'),
    ('reports', 'read')
) AS t(resource_type, action)
WHERE r.code = 'lab_technician' AND r.is_system = true
ON CONFLICT (role_id, resource_type, action) DO NOTHING;

-- dye_recipe_master：染色配方主管（染缸更新 + 配方审批 + 报表导出）
INSERT INTO role_permissions (role_id, resource_type, action, allowed, created_at, updated_at)
SELECT r.id, t.resource_type, t.action, true, NOW(), NOW()
FROM roles r
CROSS JOIN (
    VALUES
    ('dashboard', 'read'),
    ('dye_batches', 'read'), ('dye_batches', 'update'), ('dye_batches', 'export'),
    ('color_card_issue', 'read'),
    ('products', 'read'),
    ('inventory', 'read'),
    ('reports', 'read'), ('reports', 'export')
) AS t(resource_type, action)
WHERE r.code = 'dye_recipe_master' AND r.is_system = true
ON CONFLICT (role_id, resource_type, action) DO NOTHING;

-- cost_accountant：成本会计（成本核算 + 生产/采购只读 + 报表导出）
INSERT INTO role_permissions (role_id, resource_type, action, allowed, created_at, updated_at)
SELECT r.id, t.resource_type, t.action, true, NOW(), NOW()
FROM roles r
CROSS JOIN (
    VALUES
    ('dashboard', 'read'),
    ('dye_batches', 'read'), ('dye_batches', 'export'),
    ('orders', 'read'),
    ('purchase-orders', 'read'),
    ('inventory', 'read'),
    ('products', 'read'),
    ('customers', 'read'), ('suppliers', 'read'),
    ('reports', 'read'), ('reports', 'export')
) AS t(resource_type, action)
WHERE r.code = 'cost_accountant' AND r.is_system = true
ON CONFLICT (role_id, resource_type, action) DO NOTHING;

-- ============================================================
-- V15 P1 batch-16 缺陷 1.2：报表分域查看权限注册
-- ============================================================
-- 权限码格式：report:<domain>:view → 注册为 (resource_type='report-<domain>', action='view')
-- 由 ReportTemplateService::parse_required_permission 解析为 (report-<domain>, view) 二元组
-- 报表模板按 report_type 自动绑定权限码（sales/purchase/inventory/financial）

-- admin 持有全部 4 个报表分域 view 权限
INSERT INTO role_permissions (role_id, resource_type, action, allowed, created_at, updated_at)
VALUES
(1, 'report-sales', 'view', true, NOW(), NOW()),
(1, 'report-purchase', 'view', true, NOW(), NOW()),
(1, 'report-inventory', 'view', true, NOW(), NOW()),
(1, 'report-finance', 'view', true, NOW(), NOW())
ON CONFLICT (role_id, resource_type, action) DO NOTHING;

-- sales_manager：销售经理仅可查看销售域报表
INSERT INTO role_permissions (role_id, resource_type, action, allowed, created_at, updated_at)
SELECT r.id, 'report-sales', 'view', true, NOW(), NOW()
FROM roles r
WHERE r.code = 'sales_manager' AND r.is_system = true
ON CONFLICT (role_id, resource_type, action) DO NOTHING;

-- warehouse_manager：仓库经理可查看库存域与采购域报表（含入库/退货统计）
INSERT INTO role_permissions (role_id, resource_type, action, allowed, created_at, updated_at)
SELECT r.id, t.resource_type, t.action, true, NOW(), NOW()
FROM roles r
CROSS JOIN (
    VALUES
    ('report-inventory', 'view'),
    ('report-purchase', 'view')
) AS t(resource_type, action)
WHERE r.code = 'warehouse_manager' AND r.is_system = true
ON CONFLICT (role_id, resource_type, action) DO NOTHING;

-- production_manager：生产经理可查看库存域报表（生产领料/库存周转）
INSERT INTO role_permissions (role_id, resource_type, action, allowed, created_at, updated_at)
SELECT r.id, 'report-inventory', 'view', true, NOW(), NOW()
FROM roles r
WHERE r.code = 'production_manager' AND r.is_system = true
ON CONFLICT (role_id, resource_type, action) DO NOTHING;

-- cost_accountant：成本会计可查看全部分域报表（成本核算需要全量数据）
INSERT INTO role_permissions (role_id, resource_type, action, allowed, created_at, updated_at)
SELECT r.id, t.resource_type, t.action, true, NOW(), NOW()
FROM roles r
CROSS JOIN (
    VALUES
    ('report-sales', 'view'),
    ('report-purchase', 'view'),
    ('report-inventory', 'view'),
    ('report-finance', 'view')
) AS t(resource_type, action)
WHERE r.code = 'cost_accountant' AND r.is_system = true
ON CONFLICT (role_id, resource_type, action) DO NOTHING;

-- 验证插入结果
SELECT rp.*, r.name as role_name 
FROM role_permissions rp
LEFT JOIN roles r ON rp.role_id = r.id
WHERE rp.role_id = 1
ORDER BY rp.resource_type, rp.action;
