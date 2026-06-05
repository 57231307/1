-- 权限查询优化
CREATE INDEX IF NOT EXISTS idx_role_permissions_role_allowed ON role_permissions(role_id, allowed);

-- 低库存预警优化
CREATE INDEX IF NOT EXISTS idx_inventory_stocks_quantity_status ON inventory_stocks(quantity_meters, stock_status);

-- 销售统计优化
CREATE INDEX IF NOT EXISTS idx_sales_orders_date_status ON sales_orders(order_date, status);

-- 审计日志查询优化
CREATE INDEX IF NOT EXISTS idx_omni_audit_logs_trace_id ON omni_audit_logs(trace_id);