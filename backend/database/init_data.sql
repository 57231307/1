-- 秉羲面料管理系统 - 示例数据初始化脚本
-- 使用方法: psql -U bingxi -d bingxi -f init_data.sql

-- 1. 仓库数据
INSERT INTO warehouses (warehouse_code, name, address, city, province, country, is_active, is_deleted, created_at, updated_at)
VALUES
    ('WH001', '主仓库', '工业园区A栋', '苏州', '江苏', '中国', true, false, NOW(), NOW()),
    ('WH002', '原料仓库', '工业园区B栋', '苏州', '江苏', '中国', true, false, NOW(), NOW()),
    ('WH003', '成品仓库', '工业园区C栋', '苏州', '江苏', '中国', true, false, NOW(), NOW())
ON CONFLICT DO NOTHING;

-- 2. 产品分类
INSERT INTO product_categories (category_code, name, description, sort_order, is_active, is_deleted, created_at, updated_at)
VALUES
    ('FABRIC', '面料', '各类面料产品', 1, true, false, NOW(), NOW()),
    ('YARN', '纱线', '各类纱线原料', 2, true, false, NOW(), NOW()),
    ('DYE', '染料', '染色用化学品', 3, true, false, NOW(), NOW()),
    ('ACCESSORY', '辅料', '拉链、纽扣等辅料', 4, true, false, NOW(), NOW())
ON CONFLICT DO NOTHING;

-- 3. 产品数据
INSERT INTO products (code, name, specification, category_id, unit, standard_price, cost_price, status, is_deleted, created_at, updated_at)
VALUES
    ('FB001', '棉布A', '60*60/90*88', 1, '米', 25.50, 18.00, 'active', false, NOW(), NOW()),
    ('FB002', '涤纶布B', '75D/36F', 1, '米', 32.00, 22.00, 'active', false, NOW(), NOW()),
    ('FB003', '混纺布C', 'TC 65/35', 1, '米', 28.80, 20.00, 'active', false, NOW(), NOW()),
    ('YR001', '纯棉纱32S', '32S/1', 2, '公斤', 45.00, 35.00, 'active', false, NOW(), NOW()),
    ('YR002', '涤纶纱75D', '75D/36F', 2, '公斤', 38.00, 28.00, 'active', false, NOW(), NOW())
ON CONFLICT DO NOTHING;

-- 4. 客户数据
INSERT INTO customers (customer_code, customer_name, contact_person, contact_phone, contact_email, address, customer_type, credit_limit, payment_terms, status, is_deleted, created_at, updated_at)
VALUES
    ('CUS001', '服装厂A', '张三', '13800138001', 'zhangsan@a.com', '上海市浦东新区', 'wholesale', 100000, 30, 'active', false, NOW(), NOW()),
    ('CUS002', '贸易公司B', '李四', '13800138002', 'lisi@b.com', '广州市天河区', 'wholesale', 200000, 45, 'active', false, NOW(), NOW()),
    ('CUS003', '制衣厂C', '王五', '13800138003', 'wangwu@c.com', '深圳市南山区', 'vip', 500000, 60, 'active', false, NOW(), NOW())
ON CONFLICT DO NOTHING;

-- 5. 供应商数据
INSERT INTO suppliers (supplier_code, supplier_name, supplier_short_name, supplier_type, credit_code, registered_address, legal_representative, registered_capital, establishment_date, taxpayer_type, bank_name, bank_account, contact_phone, grade, grade_score, status, is_enabled, is_deleted, created_at, updated_at)
VALUES
    ('SUP001', '纺织原料供应商A', '供A', 'material', '91310000MA1FL8XQ3K', '江苏省苏州市', '赵六', 500, '2010-01-01', 'general', '工商银行', '6222021234567890123', '13900139001', 'A', 95, 'active', true, false, NOW(), NOW()),
    ('SUP002', '染料化工公司B', '供B', 'material', '91320000MA1FL8XQ4L', '浙江省杭州市', '钱七', 300, '2012-05-15', 'general', '建设银行', '6227001234567890124', '13900139002', 'A', 90, 'active', true, false, NOW(), NOW()),
    ('SUP003', '辅料供应商C', '供C', 'accessory', '91330000MA1FL8XQ5M', '广东省东莞市', '孙八', 200, '2015-08-20', 'small', '农业银行', '6228481234567890125', '13900139003', 'B', 80, 'active', true, false, NOW(), NOW())
ON CONFLICT DO NOTHING;

-- 6. 坯布数据
INSERT INTO greige_fabric (fabric_no, fabric_name, fabric_type, color_code, width_cm, weight_kg, length_m, supplier_id, batch_no, warehouse_id, status, quality_grade, is_deleted, created_at, updated_at)
VALUES
    ('GF001', '纯棉坯布A', '棉布', 'WHITE', 150, 0.25, 1000, 1, 'BATCH001', 2, 'active', 'A', false, NOW(), NOW()),
    ('GF002', '涤纶坯布B', '涤纶', 'WHITE', 160, 0.30, 800, 1, 'BATCH002', 2, 'active', 'A', false, NOW(), NOW()),
    ('GF003', '混纺坯布C', '混纺', 'WHITE', 145, 0.28, 500, 1, 'BATCH003', 2, 'active', 'B', false, NOW(), NOW())
ON CONFLICT DO NOTHING;

-- 7. 染色配方
INSERT INTO dye_recipe (recipe_no, color_code, color_name, fabric_type, dye_type, chemical_formula, temperature, time_minutes, ph_value, liquor_ratio, status, version, is_deleted, created_at, updated_at)
VALUES
    ('DR001', 'RED001', '大红色', '棉布', '活性染料', 'C20H11N2Na3O10S3', 60, 45, 10.5, '1:10', 'active', 1, false, NOW(), NOW()),
    ('DR002', 'BLUE001', '宝蓝色', '棉布', '还原染料', 'C28H14N2O4', 95, 60, 11.0, '1:15', 'active', 1, false, NOW(), NOW()),
    ('DR003', 'BLACK001', '纯黑色', '涤纶', '分散染料', 'C22H14N4O6S', 130, 30, 4.5, '1:8', 'active', 1, false, NOW(), NOW())
ON CONFLICT DO NOTHING;

-- 8. 销售订单示例
INSERT INTO sales_orders (order_no, customer_id, order_date, delivery_date, status, total_amount, discount_amount, final_amount, paid_amount, notes, created_by, is_deleted, created_at, updated_at)
VALUES
    ('SO20260517001', 1, '2026-05-17', '2026-06-17', 'draft', 25500.00, 0, 25500.00, 0, '样品订单', 1, false, NOW(), NOW()),
    ('SO20260517002', 2, '2026-05-17', '2026-06-01', 'confirmed', 64000.00, 3200.00, 60800.00, 30000.00, '常规订单', 1, false, NOW(), NOW())
ON CONFLICT DO NOTHING;

-- 9. 采购订单示例
INSERT INTO purchase_orders (order_no, supplier_id, order_date, expected_delivery_date, warehouse_id, department_id, purchaser_id, currency, exchange_rate, total_amount, total_amount_foreign, total_quantity, total_quantity_alt, order_status, created_by, is_deleted, created_at, updated_at)
VALUES
    ('PO20260517001', 1, '2026-05-17', '2026-05-27', 2, 1, 1, 'CNY', 1.000000, 35000.00, 35000.00, 1000, 0, 'DRAFT', 1, false, NOW(), NOW()),
    ('PO20260517002', 2, '2026-05-17', '2026-05-25', 2, 1, 1, 'CNY', 1.000000, 12000.00, 12000.00, 500, 0, 'APPROVED', 1, false, NOW(), NOW())
ON CONFLICT DO NOTHING;

-- 10. 库存数据
INSERT INTO inventory_stocks (product_id, warehouse_id, batch_no, quantity, reserved_quantity, available_quantity, unit_cost, total_cost, stock_status, quality_status, is_deleted, created_at, updated_at)
VALUES
    (1, 1, 'BATCH001', 5000, 0, 5000, 18.00, 90000.00, 'normal', 'qualified', false, NOW(), NOW()),
    (2, 1, 'BATCH002', 3000, 0, 3000, 22.00, 66000.00, 'normal', 'qualified', false, NOW(), NOW()),
    (3, 1, 'BATCH003', 2000, 0, 2000, 20.00, 40000.00, 'normal', 'qualified', false, NOW(), NOW()),
    (4, 2, 'BATCH004', 800, 0, 800, 35.00, 28000.00, 'normal', 'qualified', false, NOW(), NOW()),
    (5, 2, 'BATCH005', 600, 0, 600, 28.00, 16800.00, 'normal', 'qualified', false, NOW(), NOW())
ON CONFLICT DO NOTHING;

-- 完成提示
DO $$
BEGIN
    RAISE NOTICE '========================================';
    RAISE NOTICE '  示例数据初始化完成！';
    RAISE NOTICE '========================================';
    RAISE NOTICE '  仓库: 3 条';
    RAISE NOTICE '  产品分类: 4 条';
    RAISE NOTICE '  产品: 5 条';
    RAISE NOTICE '  客户: 3 条';
    RAISE NOTICE '  供应商: 3 条';
    RAISE NOTICE '  坯布: 3 条';
    RAISE NOTICE '  染色配方: 3 条';
    RAISE NOTICE '  销售订单: 2 条';
    RAISE NOTICE '  采购订单: 2 条';
    RAISE NOTICE '  库存: 5 条';
    RAISE NOTICE '========================================';
END $$;
