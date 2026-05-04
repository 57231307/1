import re

with open('database/migration/001_consolidated_schema.sql', 'r', encoding='utf-8') as f:
    content = f.read()

old_view = """CREATE OR REPLACE VIEW v_low_stock_alerts AS
SELECT 
    s.id,
    s.batch_no,
    p.name AS product_name,
    p.code AS product_code,
    w.name AS warehouse_name,
    s.quantity,
    s.min_stock,
    s.max_stock,
    CASE 
        WHEN s.quantity < s.min_stock THEN 'low'
        WHEN s.quantity > s.max_stock THEN 'high'
        ELSE 'normal'
    END AS stock_status
FROM inventory_stocks s
JOIN products p ON s.product_id = p.id
JOIN warehouses w ON s.warehouse_id = w.id
WHERE s.quantity < s.min_stock OR s.quantity > s.max_stock;"""

new_view = """CREATE OR REPLACE VIEW v_low_stock_alerts AS
SELECT 
    s.id,
    s.batch_no,
    p.name AS product_name,
    p.code AS product_code,
    w.name AS warehouse_name,
    s.quantity_meters AS quantity,
    s.reorder_point AS min_stock,
    s.quantity_available,
    CASE 
        WHEN s.quantity_available < s.reorder_point THEN 'low'
        ELSE 'normal'
    END AS stock_status
FROM inventory_stocks s
JOIN products p ON s.product_id = p.id
JOIN warehouses w ON s.warehouse_id = w.id
WHERE s.quantity_available < s.reorder_point;"""

content = content.replace(old_view, new_view)

with open('database/migration/001_consolidated_schema.sql', 'w', encoding='utf-8') as f:
    f.write(content)
