-- 创建 sales_return 表
CREATE TABLE IF NOT EXISTS sales_return (
    id SERIAL PRIMARY KEY,
    return_no VARCHAR(50) NOT NULL UNIQUE,
    sales_order_id INTEGER REFERENCES sales_orders(id),
    customer_id INTEGER NOT NULL REFERENCES customers(id),
    return_date DATE NOT NULL,
    warehouse_id INTEGER NOT NULL REFERENCES warehouses(id),
    reason TEXT NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'DRAFT',
    total_amount DECIMAL(18,2) NOT NULL DEFAULT 0.00,
    remarks TEXT,
    approved_by INTEGER REFERENCES users(id),
    approved_at TIMESTAMP WITH TIME ZONE,
    rejected_reason TEXT,
    created_by INTEGER NOT NULL REFERENCES users(id),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 创建 sales_return_item 表
CREATE TABLE IF NOT EXISTS sales_return_item (
    id SERIAL PRIMARY KEY,
    return_id INTEGER NOT NULL REFERENCES sales_return(id),
    line_no INTEGER NOT NULL,
    product_id INTEGER NOT NULL REFERENCES products(id),
    quantity DECIMAL(18,4) NOT NULL DEFAULT 0,
    quantity_alt DECIMAL(18,4) NOT NULL DEFAULT 0,
    unit_price DECIMAL(18,6) NOT NULL DEFAULT 0,
    unit_price_foreign DECIMAL(18,6) NOT NULL DEFAULT 0,
    discount_percent DECIMAL(5,2) NOT NULL DEFAULT 0,
    tax_percent DECIMAL(5,2) NOT NULL DEFAULT 0,
    subtotal DECIMAL(18,2) NOT NULL DEFAULT 0,
    tax_amount DECIMAL(18,2) NOT NULL DEFAULT 0,
    discount_amount DECIMAL(18,2) NOT NULL DEFAULT 0,
    total_amount DECIMAL(18,2) NOT NULL DEFAULT 0,
    notes TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_sales_return_customer ON sales_return(customer_id);
CREATE INDEX IF NOT EXISTS idx_sales_return_order ON sales_return(sales_order_id);
CREATE INDEX IF NOT EXISTS idx_sales_return_status ON sales_return(status);
CREATE INDEX IF NOT EXISTS idx_sales_return_date ON sales_return(return_date);
CREATE INDEX IF NOT EXISTS idx_sales_return_item_return ON sales_return_item(return_id);
CREATE INDEX IF NOT EXISTS idx_sales_return_item_product ON sales_return_item(product_id);

-- 添加注释
COMMENT ON TABLE sales_return IS '销售退货单';
COMMENT ON TABLE sales_return_item IS '销售退货明细';
