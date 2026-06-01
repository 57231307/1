-- ============================================
-- 性能优化索引迁移脚本
-- 添加高频查询所需的索引
-- ============================================

-- ============================================
-- 销售订单相关索引
-- ============================================
CREATE INDEX IF NOT EXISTS idx_sales_orders_customer_date 
ON sales_orders(customer_id, order_date DESC);

CREATE INDEX IF NOT EXISTS idx_sales_orders_status 
ON sales_orders(status);

CREATE INDEX IF NOT EXISTS idx_sales_orders_created_at 
ON sales_orders(created_at DESC);

-- ============================================
-- 采购订单相关索引
-- ============================================
CREATE INDEX IF NOT EXISTS idx_purchase_orders_supplier_status 
ON purchase_order(supplier_id, status);

CREATE INDEX IF NOT EXISTS idx_purchase_orders_order_date 
ON purchase_order(order_date DESC);

CREATE INDEX IF NOT EXISTS idx_purchase_orders_created_at 
ON purchase_order(created_at DESC);

-- ============================================
-- 库存相关索引
-- ============================================
CREATE INDEX IF NOT EXISTS idx_inventory_stocks_product_warehouse 
ON inventory_stocks(product_id, warehouse_id);

CREATE INDEX IF NOT EXISTS idx_inventory_stocks_batch_no 
ON inventory_stocks(batch_no);

CREATE INDEX IF NOT EXISTS idx_inventory_stocks_status 
ON inventory_stocks(stock_status);

CREATE INDEX IF NOT EXISTS idx_inventory_transactions_product_date 
ON inventory_transactions(product_id, created_at DESC);

-- ============================================
-- 客户相关索引
-- ============================================
CREATE INDEX IF NOT EXISTS idx_customers_customer_type 
ON customers(customer_type);

CREATE INDEX IF NOT EXISTS idx_customers_status 
ON customers(status);

-- ============================================
-- 供应商相关索引
-- ============================================
CREATE INDEX IF NOT EXISTS idx_suppliers_status 
ON suppliers(status);

-- ============================================
-- 产品相关索引
-- ============================================
CREATE INDEX IF NOT EXISTS idx_products_category 
ON products(category_id);

CREATE INDEX IF NOT EXISTS idx_products_code 
ON products(code);

-- ============================================
-- 应收应付相关索引
-- ============================================
CREATE INDEX IF NOT EXISTS idx_ap_invoices_supplier_status 
ON ap_invoice(supplier_id, invoice_status);

CREATE INDEX IF NOT EXISTS idx_ap_invoices_due_date 
ON ap_invoice(due_date);

CREATE INDEX IF NOT EXISTS idx_ar_invoices_customer_status 
ON ar_invoices(customer_id, status);

CREATE INDEX IF NOT EXISTS idx_ar_invoices_due_date 
ON ar_invoices(due_date);

-- ============================================
-- 凭证相关索引
-- ============================================
CREATE INDEX IF NOT EXISTS idx_vouchers_voucher_date 
ON vouchers(voucher_date DESC);

CREATE INDEX IF NOT EXISTS idx_vouchers_status 
ON vouchers(status);

-- ============================================
-- 通知相关索引
-- ============================================
CREATE INDEX IF NOT EXISTS idx_notifications_user_status 
ON notifications(user_id, status);

CREATE INDEX IF NOT EXISTS idx_notifications_created_at 
ON notifications(created_at DESC);

-- ============================================
-- 操作日志相关索引
-- ============================================
CREATE INDEX IF NOT EXISTS idx_operation_logs_user_date 
ON operation_logs(user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_operation_logs_action 
ON operation_logs(action);

COMMENT ON INDEX idx_sales_orders_customer_date IS '销售订单按客户和时间查询优化';
COMMENT ON INDEX idx_purchase_orders_supplier_status IS '采购订单按供应商和状态查询优化';
COMMENT ON INDEX idx_inventory_stocks_product_warehouse IS '库存按产品和仓库查询优化';
COMMENT ON INDEX idx_ap_invoices_supplier_status IS '应付发票按供应商和状态查询优化';
COMMENT ON INDEX idx_ar_invoices_customer_status IS '应收发票按客户和状态查询优化';
