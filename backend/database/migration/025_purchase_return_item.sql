CREATE TABLE purchase_return_item (
    id SERIAL PRIMARY KEY,
    return_id INTEGER NOT NULL,
    line_no INTEGER NOT NULL,
    product_id INTEGER NOT NULL,
    quantity DECIMAL(18,4) NOT NULL,
    quantity_alt DECIMAL(18,4) DEFAULT 0.0000,
    unit_price DECIMAL(18,6) NOT NULL,
    unit_price_foreign DECIMAL(18,6) DEFAULT 0.000000,
    discount_percent DECIMAL(5,2) DEFAULT 0.00,
    tax_percent DECIMAL(5,2) DEFAULT 0.00,
    subtotal DECIMAL(18,2) DEFAULT 0.00,
    tax_amount DECIMAL(18,2) DEFAULT 0.00,
    discount_amount DECIMAL(18,2) DEFAULT 0.00,
    total_amount DECIMAL(18,2) DEFAULT 0.00,
    notes TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

ALTER TABLE purchase_return_item ADD CONSTRAINT fk_pret_item_return
    FOREIGN KEY (return_id) REFERENCES purchase_return(id) ON DELETE CASCADE;
ALTER TABLE purchase_return_item ADD CONSTRAINT fk_pret_item_product
    FOREIGN KEY (product_id) REFERENCES products(id);
