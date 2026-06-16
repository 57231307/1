-- P0-5 初始化数据库
-- 创建租户、用户、基础数据

-- 1. 默认租户
INSERT INTO tenants (id, name, code, is_active, created_at, updated_at)
VALUES (1, '默认租户', 'default', true, NOW(), NOW())
ON CONFLICT (id) DO NOTHING;

-- 2. 管理员用户（密码 admin123，bcrypt 哈希）
INSERT INTO users (id, tenant_id, username, password_hash, email, is_admin, is_active, created_at, updated_at)
VALUES (1, 1, 'admin', '$2b$12$LQv3c1yqBwEHFL0w8S8yxe.YHPjKpZh7xZjJhKZpZj3pH8oQ4hU3.', 'admin@bingxi.com', true, true, NOW(), NOW())
ON CONFLICT (id) DO NOTHING;

-- 3. 销售员用户（密码 sales123）
INSERT INTO users (id, tenant_id, username, password_hash, email, is_admin, is_active, created_at, updated_at)
VALUES (2, 1, 'sales', '$2b$12$LQv3c1yqBwEHFL0w8S8yxe.YHPjKpZh7xZjJhKZpZj3pH8oQ4hU3.', 'sales@bingxi.com', false, true, NOW(), NOW())
ON CONFLICT (id) DO NOTHING;

-- 4. 示例客户
INSERT INTO customers (id, tenant_id, name, code, customer_level, contact_person, phone, is_active, created_at, updated_at)
VALUES
  (1, 1, '战略客户 A', 'CUST001', 'VIP', '张总', '13800138000', true, NOW(), NOW()),
  (2, 1, '金牌客户 B', 'CUST002', 'GOLD', '李经理', '13900139000', true, NOW(), NOW()),
  (3, 1, '普通客户 C', 'CUST003', 'NORMAL', '王先生', '13700137000', true, NOW(), NOW())
ON CONFLICT (id) DO NOTHING;

-- 5. 示例产品品类
INSERT INTO product_categories (id, tenant_id, name, code, parent_id, created_at, updated_at)
VALUES
  (1, 1, '棉布', 'CAT-COTTON', NULL, NOW(), NOW()),
  (2, 1, '丝绸', 'CAT-SILK', NULL, NOW(), NOW())
ON CONFLICT (id) DO NOTHING;

-- 6. 示例产品
INSERT INTO products (id, tenant_id, name, code, category_id, unit, is_active, created_at, updated_at)
VALUES
  (1, 1, '棉府绸', 'P-COT-001', 1, '米', true, NOW(), NOW()),
  (2, 1, '真丝双绉', 'P-SLK-001', 2, '米', true, NOW(), NOW())
ON CONFLICT (id) DO NOTHING;

-- 7. 示例色号
INSERT INTO product_colors (id, tenant_id, product_id, color_code, color_name, hex_value, is_active, created_at, updated_at)
VALUES
  (1, 1, 1, 'RED-001', '中国红', '#FF0000', true, NOW(), NOW()),
  (2, 1, 1, 'BLU-001', '宝石蓝', '#0000FF', true, NOW(), NOW()),
  (3, 1, 2, 'GRN-001', '翡翠绿', '#00FF00', true, NOW(), NOW())
ON CONFLICT (id) DO NOTHING;
