-- V15 P1-14.8-B：字段级权限种子数据
--
-- 审计计划 14.8.2 要求的 4 个字段级权限场景：
-- 1. 销售员不能看成本价：sales_order.cost_price HIDDEN
-- 2. 销售员不能看客户信用额度：customer.credit_limit HIDDEN
-- 3. 采购员不能看供应商底价：supplier.floor_price HIDDEN
-- 4. 化验员不能看配方用量：dye_recipe.quantity HIDDEN
--
-- 通过子查询获取角色 ID（避免硬编码 role_id），幂等插入（ON CONFLICT DO NOTHING）。
-- mask_strategy：MASK（显示为 ***），can_read=false（不可读），can_write=false（不可写）。

-- 1. 销售员（sales_rep / sales）不可读 sales_order.cost_price（成本价）
INSERT INTO field_permissions (role_id, resource_type, field_name, can_read, can_write, mask_strategy, is_enabled, created_at, updated_at)
SELECT r.id, 'sales_order', 'cost_price', false, false, 'MASK', true, NOW(), NOW()
FROM roles r
WHERE r.code IN ('sales_rep', 'sales')
ON CONFLICT (role_id, resource_type, field_name) DO NOTHING;

-- 2. 销售员（sales_rep / sales）不可读 customer.credit_limit（客户信用额度）
INSERT INTO field_permissions (role_id, resource_type, field_name, can_read, can_write, mask_strategy, is_enabled, created_at, updated_at)
SELECT r.id, 'customer', 'credit_limit', false, false, 'MASK', true, NOW(), NOW()
FROM roles r
WHERE r.code IN ('sales_rep', 'sales')
ON CONFLICT (role_id, resource_type, field_name) DO NOTHING;

-- 3. 采购员（purchase_clerk）不可读 supplier.floor_price（供应商底价）
INSERT INTO field_permissions (role_id, resource_type, field_name, can_read, can_write, mask_strategy, is_enabled, created_at, updated_at)
SELECT r.id, 'supplier', 'floor_price', false, false, 'MASK', true, NOW(), NOW()
FROM roles r
WHERE r.code = 'purchase_clerk'
ON CONFLICT (role_id, resource_type, field_name) DO NOTHING;

-- 4. 化验员（lab_technician）不可读 dye_recipe.quantity（配方用量）
INSERT INTO field_permissions (role_id, resource_type, field_name, can_read, can_write, mask_strategy, is_enabled, created_at, updated_at)
SELECT r.id, 'dye_recipe', 'quantity', false, false, 'MASK', true, NOW(), NOW()
FROM roles r
WHERE r.code = 'lab_technician'
ON CONFLICT (role_id, resource_type, field_name) DO NOTHING;

-- 5. V15 P1-14.8-B 扩展：销售员不可读 sales_order.profit_rate（利润率，衍生自成本价）
INSERT INTO field_permissions (role_id, resource_type, field_name, can_read, can_write, mask_strategy, is_enabled, created_at, updated_at)
SELECT r.id, 'sales_order', 'profit_rate', false, false, 'MASK', true, NOW(), NOW()
FROM roles r
WHERE r.code IN ('sales_rep', 'sales')
ON CONFLICT (role_id, resource_type, field_name) DO NOTHING;

-- 6. V15 P1-14.8-B 扩展：采购员不可读 purchase_order.unit_cost（采购单价成本）
INSERT INTO field_permissions (role_id, resource_type, field_name, can_read, can_write, mask_strategy, is_enabled, created_at, updated_at)
SELECT r.id, 'purchase_order', 'unit_cost', false, false, 'MASK', true, NOW(), NOW()
FROM roles r
WHERE r.code = 'purchase_clerk'
ON CONFLICT (role_id, resource_type, field_name) DO NOTHING;
