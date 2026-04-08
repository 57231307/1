with open('/workspace/backend/database/migration/001_consolidated_schema.sql', 'a') as f:
    f.write("\n\n-- =========================================================\n")
    f.write("-- RUST MODEL COMPATIBILITY PATCHES\n")
    f.write("-- =========================================================\n")
    
    # Inventory Stocks
    f.write("\n-- inventory_stocks\n")
    f.write("ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS quantity_on_hand DECIMAL(12,2) NOT NULL DEFAULT 0;\n")
    f.write("ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS quantity_available DECIMAL(12,2) NOT NULL DEFAULT 0;\n")
    f.write("ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS quantity_reserved DECIMAL(12,2) NOT NULL DEFAULT 0;\n")
    f.write("ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS quantity_incoming DECIMAL(12,2) NOT NULL DEFAULT 0;\n")
    f.write("ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS reorder_point DECIMAL(12,2) NOT NULL DEFAULT 0;\n")
    f.write("ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS reorder_quantity DECIMAL(12,2) NOT NULL DEFAULT 0;\n")
    f.write("ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS bin_location VARCHAR(100);\n")
    f.write("ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS last_count_date TIMESTAMPTZ;\n")
    f.write("ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS last_movement_date TIMESTAMPTZ;\n")
    
    # Sales Orders
    f.write("\n-- sales_orders\n")
    f.write("ALTER TABLE sales_orders ADD COLUMN IF NOT EXISTS customer_id INTEGER;\n")
    f.write("ALTER TABLE sales_orders ADD COLUMN IF NOT EXISTS required_date TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP;\n")
    f.write("ALTER TABLE sales_orders ADD COLUMN IF NOT EXISTS ship_date TIMESTAMPTZ;\n")
    f.write("ALTER TABLE sales_orders ADD COLUMN IF NOT EXISTS subtotal DECIMAL(12,2) NOT NULL DEFAULT 0;\n")
    f.write("ALTER TABLE sales_orders ADD COLUMN IF NOT EXISTS tax_amount DECIMAL(12,2) NOT NULL DEFAULT 0;\n")
    f.write("ALTER TABLE sales_orders ADD COLUMN IF NOT EXISTS discount_amount DECIMAL(12,2) NOT NULL DEFAULT 0;\n")
    f.write("ALTER TABLE sales_orders ADD COLUMN IF NOT EXISTS shipping_cost DECIMAL(12,2) NOT NULL DEFAULT 0;\n")
    f.write("ALTER TABLE sales_orders ADD COLUMN IF NOT EXISTS paid_amount DECIMAL(12,2) NOT NULL DEFAULT 0;\n")
    f.write("ALTER TABLE sales_orders ADD COLUMN IF NOT EXISTS balance_amount DECIMAL(12,2) NOT NULL DEFAULT 0;\n")
    f.write("ALTER TABLE sales_orders ADD COLUMN IF NOT EXISTS shipping_address TEXT;\n")
    f.write("ALTER TABLE sales_orders ADD COLUMN IF NOT EXISTS billing_address TEXT;\n")
    
    # Sales Order Items
    f.write("\n-- sales_order_items\n")
    f.write("ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS discount_percent DECIMAL(5,2) NOT NULL DEFAULT 0;\n")
    f.write("ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS tax_percent DECIMAL(5,2) NOT NULL DEFAULT 0;\n")
    f.write("ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS subtotal DECIMAL(12,2) NOT NULL DEFAULT 0;\n")
    f.write("ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS tax_amount DECIMAL(12,2) NOT NULL DEFAULT 0;\n")
    f.write("ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS discount_amount DECIMAL(12,2) NOT NULL DEFAULT 0;\n")
    f.write("ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS shipped_quantity DECIMAL(12,2) NOT NULL DEFAULT 0;\n")
    
    # Finance Payments
    f.write("\n-- finance_payments\n")
    f.write("ALTER TABLE finance_payments ADD COLUMN IF NOT EXISTS payment_type VARCHAR(50) NOT NULL DEFAULT '';\n")
    f.write("ALTER TABLE finance_payments ADD COLUMN IF NOT EXISTS order_type VARCHAR(50) NOT NULL DEFAULT '';\n")
    f.write("ALTER TABLE finance_payments ADD COLUMN IF NOT EXISTS order_id INTEGER;\n")
    f.write("ALTER TABLE finance_payments ADD COLUMN IF NOT EXISTS customer_id INTEGER;\n")
    f.write("ALTER TABLE finance_payments ADD COLUMN IF NOT EXISTS supplier_id INTEGER;\n")
    f.write("ALTER TABLE finance_payments ADD COLUMN IF NOT EXISTS paid_amount DECIMAL(12,2) NOT NULL DEFAULT 0;\n")
    f.write("ALTER TABLE finance_payments ADD COLUMN IF NOT EXISTS balance_amount DECIMAL(12,2) NOT NULL DEFAULT 0;\n")
    f.write("ALTER TABLE finance_payments ADD COLUMN IF NOT EXISTS reference_no VARCHAR(100);\n")
    f.write("ALTER TABLE finance_payments ADD COLUMN IF NOT EXISTS approved_by INTEGER;\n")
    f.write("ALTER TABLE finance_payments ADD COLUMN IF NOT EXISTS approved_at TIMESTAMPTZ;\n")
    
    # Customer Credit Changes
    f.write("\n-- customer_credit_changes\n")
    f.write("ALTER TABLE customer_credit_changes ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP;\n")
    
    # Products
    f.write("\n-- products\n")
    f.write("ALTER TABLE products ADD COLUMN IF NOT EXISTS product_type VARCHAR(50) NOT NULL DEFAULT '';\n")
    f.write("ALTER TABLE products ADD COLUMN IF NOT EXISTS fabric_composition VARCHAR(200);\n")
    f.write("ALTER TABLE products ADD COLUMN IF NOT EXISTS yarn_count VARCHAR(100);\n")
    f.write("ALTER TABLE products ADD COLUMN IF NOT EXISTS density VARCHAR(100);\n")
    f.write("ALTER TABLE products ADD COLUMN IF NOT EXISTS width DECIMAL(10,2);\n")
    f.write("ALTER TABLE products ADD COLUMN IF NOT EXISTS gram_weight DECIMAL(10,2);\n")
    f.write("ALTER TABLE products ADD COLUMN IF NOT EXISTS structure VARCHAR(100);\n")
    f.write("ALTER TABLE products ADD COLUMN IF NOT EXISTS finish VARCHAR(100);\n")
    f.write("ALTER TABLE products ADD COLUMN IF NOT EXISTS min_order_quantity DECIMAL(12,2);\n")
    f.write("ALTER TABLE products ADD COLUMN IF NOT EXISTS lead_time INTEGER;\n")
    
    print("Appended patches to schema.")
