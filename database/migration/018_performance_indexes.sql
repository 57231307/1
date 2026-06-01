-- ============================================
-- 性能优化索引迁移脚本
-- 添加高频查询所需的索引
-- 注意：大部分索引已在 001_consolidated_schema.sql 中定义
-- ============================================

-- ============================================
-- 采购订单相关索引（使用正确的列名 order_status）
-- ============================================
CREATE INDEX IF NOT EXISTS idx_purchase_orders_supplier_status 
ON purchase_order(supplier_id, order_status);

CREATE INDEX IF NOT EXISTS idx_purchase_orders_order_date 
ON purchase_order(order_date DESC);

CREATE INDEX IF NOT EXISTS idx_purchase_orders_created_at 
ON purchase_order(created_at DESC);

-- ============================================
-- 库存相关索引
-- ============================================
CREATE INDEX IF NOT EXISTS idx_inventory_transactions_product_date 
ON inventory_transactions(product_id, created_at DESC);

-- ============================================
-- 供应商相关索引
-- ============================================
-- 注意：suppliers 表没有 category 列，跳过

-- ============================================
-- 应收应付相关索引
-- ============================================
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

COMMENT ON INDEX idx_purchase_orders_supplier_status IS '采购订单按供应商和状态查询优化';
COMMENT ON INDEX idx_inventory_transactions_product_date IS '库存交易按产品和时间查询优化';
COMMENT ON INDEX idx_ap_invoices_due_date IS '应付发票按到期日查询优化';
COMMENT ON INDEX idx_ar_invoices_customer_status IS '应收发票按客户和状态查询优化';
