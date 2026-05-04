import re

with open('database/migration/001_consolidated_schema.sql', 'r', encoding='utf-8') as f:
    content = f.read()

old_table = """CREATE TABLE IF NOT EXISTS inventory_stocks (
    id SERIAL PRIMARY KEY,
    product_id INTEGER NOT NULL REFERENCES products(id),
    warehouse_id INTEGER NOT NULL REFERENCES warehouses(id),
    batch_no VARCHAR(50) NOT NULL,
    color_code VARCHAR(50),
    color_name VARCHAR(100),
    quantity DECIMAL(10,2) NOT NULL DEFAULT 0,
    unit VARCHAR(20) NOT NULL,
    unit_price DECIMAL(10,2),
    total_amount DECIMAL(10,2),
    min_stock DECIMAL(10,2) DEFAULT 0,
    max_stock DECIMAL(10,2) DEFAULT 0,
    remark TEXT,
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    is_deleted BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);"""

new_table = """CREATE TABLE IF NOT EXISTS inventory_stocks (
    id SERIAL PRIMARY KEY,
    warehouse_id INTEGER NOT NULL REFERENCES warehouses(id),
    product_id INTEGER NOT NULL REFERENCES products(id),
    quantity_on_hand DECIMAL(12,2) NOT NULL DEFAULT 0,
    quantity_available DECIMAL(12,2) NOT NULL DEFAULT 0,
    quantity_reserved DECIMAL(12,2) NOT NULL DEFAULT 0,
    quantity_incoming DECIMAL(12,2) NOT NULL DEFAULT 0,
    reorder_point DECIMAL(12,2) NOT NULL DEFAULT 0,
    reorder_quantity DECIMAL(12,2) NOT NULL DEFAULT 0,
    bin_location VARCHAR(100),
    last_count_date TIMESTAMPTZ,
    last_movement_date TIMESTAMPTZ,
    is_deleted BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,

    batch_no VARCHAR(50) NOT NULL,
    color_no VARCHAR(50) NOT NULL,
    dye_lot_no VARCHAR(50),
    grade VARCHAR(20) NOT NULL,
    production_date TIMESTAMPTZ,
    expiry_date TIMESTAMPTZ,

    quantity_meters DECIMAL(12,2) NOT NULL DEFAULT 0,
    quantity_kg DECIMAL(12,2) NOT NULL DEFAULT 0,
    gram_weight DECIMAL(10,2),
    width DECIMAL(10,2),

    location_id INTEGER,
    shelf_no VARCHAR(50),
    layer_no VARCHAR(50),

    stock_status VARCHAR(20) NOT NULL DEFAULT 'active',
    quality_status VARCHAR(20) NOT NULL DEFAULT 'qualified'
);"""

content = content.replace(old_table, new_table)

with open('database/migration/001_consolidated_schema.sql', 'w', encoding='utf-8') as f:
    f.write(content)
