CREATE TABLE IF NOT EXISTS logistics_waybills (
    id SERIAL PRIMARY KEY,
    order_id INTEGER NOT NULL REFERENCES sales_orders(id),
    logistics_company VARCHAR(100) NOT NULL,
    tracking_number VARCHAR(100) NOT NULL UNIQUE,
    driver_name VARCHAR(50),
    driver_phone VARCHAR(50),
    freight_fee DECIMAL(12,2) DEFAULT 0.0,
    status VARCHAR(20) DEFAULT 'IN_TRANSIT', -- IN_TRANSIT, DELIVERED, RETURNED
    expected_arrival TIMESTAMPTZ,
    actual_arrival TIMESTAMPTZ,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE logistics_waybills IS '物流运单与发货追踪表';

-- Add barcode field to inventory_pieces if not exists
ALTER TABLE inventory_pieces ADD COLUMN barcode VARCHAR(100) UNIQUE;
