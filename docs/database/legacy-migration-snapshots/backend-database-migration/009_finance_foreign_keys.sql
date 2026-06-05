-- 财务模块外键约束
-- 创建时间: 2026-05-09
-- 说明: 为财务相关表添加数据库级外键约束

-- 应付发票 → 采购订单
ALTER TABLE ap_invoices
    ADD CONSTRAINT fk_ap_invoices_purchase_order
    FOREIGN KEY (purchase_order_id) REFERENCES purchase_orders(id)
    ON DELETE RESTRICT ON UPDATE CASCADE;

-- 应付付款 → 应付发票
ALTER TABLE ap_payments
    ADD CONSTRAINT fk_ap_payments_invoice
    FOREIGN KEY (invoice_id) REFERENCES ap_invoices(id)
    ON DELETE RESTRICT ON UPDATE CASCADE;

-- 应收发票 → 销售订单
ALTER TABLE ar_invoices
    ADD CONSTRAINT fk_ar_invoices_sales_order
    FOREIGN KEY (sales_order_id) REFERENCES sales_orders(id)
    ON DELETE RESTRICT ON UPDATE CASCADE;

-- 凭证 → 会计期间
ALTER TABLE vouchers
    ADD CONSTRAINT fk_vouchers_accounting_period
    FOREIGN KEY (accounting_period_id) REFERENCES accounting_periods(id)
    ON DELETE RESTRICT ON UPDATE CASCADE;

-- 凭证明细 → 凭证
ALTER TABLE voucher_items
    ADD CONSTRAINT fk_voucher_items_voucher
    FOREIGN KEY (voucher_id) REFERENCES vouchers(id)
    ON DELETE CASCADE ON UPDATE CASCADE;

-- 凭证明细 → 科目
ALTER TABLE voucher_items
    ADD CONSTRAINT fk_voucher_items_account_subject
    FOREIGN KEY (account_subject_id) REFERENCES account_subjects(id)
    ON DELETE RESTRICT ON UPDATE CASCADE;

-- 应付核销 → 应付发票
ALTER TABLE ap_verifications
    ADD CONSTRAINT fk_ap_verifications_invoice
    FOREIGN KEY (invoice_id) REFERENCES ap_invoices(id)
    ON DELETE RESTRICT ON UPDATE CASCADE;

-- 应付核销明细 → 应付核销
ALTER TABLE ap_verification_items
    ADD CONSTRAINT fk_ap_verification_items_verification
    FOREIGN KEY (verification_id) REFERENCES ap_verifications(id)
    ON DELETE CASCADE ON UPDATE CASCADE;

-- 应付付款申请 → 应付发票
ALTER TABLE ap_payment_requests
    ADD CONSTRAINT fk_ap_payment_requests_invoice
    FOREIGN KEY (invoice_id) REFERENCES ap_invoices(id)
    ON DELETE RESTRICT ON UPDATE CASCADE;

-- 应付付款申请明细 → 应付付款申请
ALTER TABLE ap_payment_request_items
    ADD CONSTRAINT fk_ap_payment_request_items_request
    FOREIGN KEY (request_id) REFERENCES ap_payment_requests(id)
    ON DELETE CASCADE ON UPDATE CASCADE;
