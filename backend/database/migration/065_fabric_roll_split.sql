ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS paper_tube_weight DECIMAL(10,2);
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS is_net_weight BOOLEAN;
