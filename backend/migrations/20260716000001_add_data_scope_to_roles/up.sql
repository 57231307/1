-- V15 P0-S01 修复：role 表新增 data_scope 字段（行级数据权限）
-- data_scope 取值：
--   'all'  - 全部数据（管理员/总经理）
--   'dept' - 本部门数据（部门经理）
--   'self' - 仅本人数据（普通员工）
-- 默认 'self'，确保最小权限原则：未配置的角色只能访问本人创建的数据。

ALTER TABLE roles ADD COLUMN IF NOT EXISTS data_scope VARCHAR(10) NOT NULL DEFAULT 'self';

-- 为现有角色配置默认 data_scope
-- admin / gm / deputy_gm → all（全公司数据）
UPDATE roles SET data_scope = 'all' WHERE code IN ('admin', 'gm', 'deputy_gm');

-- 各业务域 manager → dept（本部门数据）
UPDATE roles SET data_scope = 'dept' WHERE code IN (
    'manager',
    'sales_manager', 'purchase_manager', 'inventory_manager',
    'production_manager', 'qc_manager', 'finance_manager',
    'crm_manager', 'hr_manager'
);

-- operator 及各业务域执行角色 → self（仅本人数据）
UPDATE roles SET data_scope = 'self' WHERE code IN (
    'operator',
    'sales_rep', 'purchase_clerk', 'sourcing_specialist',
    'warehouse_keeper', 'dyeing_master', 'finishing_master',
    'lab_technician', 'greige_manager', 'chemical_manager',
    'maintenance_supervisor', 'quality_inspector', 'fabric_inspector',
    'accountant', 'cashier', 'cost_accountant',
    'crm_rep', 'logistics_coordinator', 'customs_specialist',
    'hr_specialist', 'safety_officer', 'system_admin',
    'data_analyst', 'admin_assistant'
);
