-- 财务模块外键约束
-- 创建时间: 2026-05-09
-- 说明: 为财务相关表添加数据库级外键约束

-- 应付发票 → 采购订单
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'ap_invoice') 
       AND EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'purchase_order') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_ap_invoices_purchase_order') THEN
            ALTER TABLE ap_invoice ADD CONSTRAINT fk_ap_invoices_purchase_order FOREIGN KEY (purchase_order_id) REFERENCES purchase_order(id) ON DELETE RESTRICT ON UPDATE CASCADE;
        END IF;
    END IF;
END $$;

-- 应付付款 → 应付发票
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'ap_payment') 
       AND EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'ap_invoice') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_ap_payments_invoice') THEN
            ALTER TABLE ap_payment ADD CONSTRAINT fk_ap_payments_invoice FOREIGN KEY (invoice_id) REFERENCES ap_invoice(id) ON DELETE RESTRICT ON UPDATE CASCADE;
        END IF;
    END IF;
END $$;

-- 应收发票 → 销售订单
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'ar_invoices') 
       AND EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'sales_orders') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_ar_invoices_sales_order') THEN
            ALTER TABLE ar_invoices ADD CONSTRAINT fk_ar_invoices_sales_order FOREIGN KEY (sales_order_id) REFERENCES sales_orders(id) ON DELETE RESTRICT ON UPDATE CASCADE;
        END IF;
    END IF;
END $$;

-- 凭证 → 会计期间（如果表存在）
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'vouchers') 
       AND EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'accounting_periods') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_vouchers_accounting_period') THEN
            ALTER TABLE vouchers ADD CONSTRAINT fk_vouchers_accounting_period FOREIGN KEY (accounting_period_id) REFERENCES accounting_periods(id) ON DELETE RESTRICT ON UPDATE CASCADE;
        END IF;
    END IF;
END $$;

-- 凭证明细 → 凭证
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'voucher_items') 
       AND EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'vouchers') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_voucher_items_voucher') THEN
            ALTER TABLE voucher_items ADD CONSTRAINT fk_voucher_items_voucher FOREIGN KEY (voucher_id) REFERENCES vouchers(id) ON DELETE CASCADE ON UPDATE CASCADE;
        END IF;
    END IF;
END $$;

-- 凭证明细 → 科目
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'voucher_items') 
       AND EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'account_subjects') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_voucher_items_account_subject') THEN
            ALTER TABLE voucher_items ADD CONSTRAINT fk_voucher_items_account_subject FOREIGN KEY (account_subject_id) REFERENCES account_subjects(id) ON DELETE RESTRICT ON UPDATE CASCADE;
        END IF;
    END IF;
END $$;

-- 应付核销 → 应付发票
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'ap_verification') 
       AND EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'ap_invoice') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_ap_verifications_invoice') THEN
            ALTER TABLE ap_verification ADD CONSTRAINT fk_ap_verifications_invoice FOREIGN KEY (invoice_id) REFERENCES ap_invoice(id) ON DELETE RESTRICT ON UPDATE CASCADE;
        END IF;
    END IF;
END $$;

-- 应付核销明细 → 应付核销
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'ap_verification_item') 
       AND EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'ap_verification') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_ap_verification_items_verification') THEN
            ALTER TABLE ap_verification_item ADD CONSTRAINT fk_ap_verification_items_verification FOREIGN KEY (verification_id) REFERENCES ap_verification(id) ON DELETE CASCADE ON UPDATE CASCADE;
        END IF;
    END IF;
END $$;

-- 应付付款申请 → 应付发票
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'ap_payment_request') 
       AND EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'ap_invoice') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_ap_payment_requests_invoice') THEN
            ALTER TABLE ap_payment_request ADD CONSTRAINT fk_ap_payment_requests_invoice FOREIGN KEY (invoice_id) REFERENCES ap_invoice(id) ON DELETE RESTRICT ON UPDATE CASCADE;
        END IF;
    END IF;
END $$;

-- 应付付款申请明细 → 应付付款申请
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'ap_payment_request_item') 
       AND EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'ap_payment_request') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_ap_payment_request_items_request') THEN
            ALTER TABLE ap_payment_request_item ADD CONSTRAINT fk_ap_payment_request_items_request FOREIGN KEY (request_id) REFERENCES ap_payment_request(id) ON DELETE CASCADE ON UPDATE CASCADE;
        END IF;
    END IF;
END $$;
