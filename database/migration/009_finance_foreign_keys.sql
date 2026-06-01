-- 财务模块外键约束
-- 创建时间: 2026-05-09
-- 说明: 为财务相关表添加数据库级外键约束

-- 注意：ap_invoice 表使用 source_id 和 source_type 字段关联业务单据，不直接关联 purchase_order
-- ar_invoices 表没有 sales_order_id 列，跳过外键约束
-- vouchers 表没有 accounting_period_id 列，跳过外键约束
-- voucher_items 表使用 subject_code 列，不是 account_subject_id，跳过外键约束
-- ap_verification 表没有 invoice_id 列，跳过外键约束
-- ap_payment_request 表没有 invoice_id 列，跳过外键约束

-- 应付付款 → 应付发票
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'ap_payment') 
       AND EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'ap_invoice') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'ap_payment' AND column_name = 'invoice_id') THEN
            -- ap_payment 表没有 invoice_id 列，跳过
            RAISE NOTICE 'ap_payment 表没有 invoice_id 列，跳过外键约束';
        ELSE
            IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_ap_payments_invoice') THEN
                ALTER TABLE ap_payment ADD CONSTRAINT fk_ap_payments_invoice FOREIGN KEY (invoice_id) REFERENCES ap_invoice(id) ON DELETE RESTRICT ON UPDATE CASCADE;
            END IF;
        END IF;
    END IF;
END $$;
