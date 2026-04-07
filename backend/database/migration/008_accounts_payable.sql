-- 应付管理模块数据库迁移脚本
-- 创建时间：2026-03-15
-- 功能说明：创建应付单、付款申请、付款执行、应付核销、供应商对账相关表及索引

-- =====================================================
-- 1. 应付单表 (ap_invoice)
-- =====================================================
CREATE TABLE ap_invoice (
    id SERIAL PRIMARY KEY,                              -- 主键 ID
    invoice_no VARCHAR(50) NOT NULL UNIQUE,             -- 应付单号（AP20260315001）
    supplier_id INTEGER NOT NULL,                       -- 供应商 ID（外键）
    invoice_type VARCHAR(20) NOT NULL,                  -- 应付类型：PURCHASE=采购应付，EXPENSE=费用应付，OTHER=其他
    source_type VARCHAR(20),                            -- 来源类型：PURCHASE_RECEIPT=采购入库，PURCHASE_RETURN=采购退货，MANUAL=手工录入
    source_id INTEGER,                                  -- 来源单 ID（采购入库单 ID 或退货单 ID）
    invoice_date DATE NOT NULL,                         -- 应付日期
    due_date DATE NOT NULL,                             -- 到期日期
    payment_terms INTEGER DEFAULT 30,                   -- 账期（天）
    amount DECIMAL(18,2) NOT NULL,                      -- 应付金额
    paid_amount DECIMAL(18,2) DEFAULT 0.00,             -- 已付金额
    unpaid_amount DECIMAL(18,2) DEFAULT 0.00,           -- 未付金额
    invoice_status VARCHAR(20) DEFAULT 'DRAFT',         -- 应付状态：DRAFT=草稿，AUDITED=已审核，PARTIAL_PAID=部分付款，PAID=已付清，CANCELLED=已取消
    currency VARCHAR(10) DEFAULT 'CNY',                 -- 币种
    exchange_rate DECIMAL(18,6) DEFAULT 1.000000,       -- 汇率
    amount_foreign DECIMAL(18,2),                       -- 外币金额
    tax_amount DECIMAL(18,2) DEFAULT 0.00,              -- 税额
    notes TEXT,                                         -- 备注
    attachment_urls TEXT[],                             -- 附件 URL 列表
    created_by INTEGER NOT NULL,                        -- 创建人 ID
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 创建时间
    updated_by INTEGER,                                 -- 更新人 ID
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 更新时间
    approved_by INTEGER,                                -- 审核人 ID
    approved_at TIMESTAMP,                              -- 审核时间
    cancelled_by INTEGER,                               -- 取消人 ID
    cancelled_at TIMESTAMP,                             -- 取消时间
    cancelled_reason TEXT                               -- 取消原因
);

-- 外键约束
ALTER TABLE ap_invoice ADD CONSTRAINT fk_ap_invoice_supplier
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id);
ALTER TABLE ap_invoice ADD CONSTRAINT fk_ap_invoice_created_by
    FOREIGN KEY (created_by) REFERENCES users(id);
ALTER TABLE ap_invoice ADD CONSTRAINT fk_ap_invoice_updated_by
    FOREIGN KEY (updated_by) REFERENCES users(id);
ALTER TABLE ap_invoice ADD CONSTRAINT fk_ap_invoice_approved_by
    FOREIGN KEY (approved_by) REFERENCES users(id);
ALTER TABLE ap_invoice ADD CONSTRAINT fk_ap_invoice_cancelled_by
    FOREIGN KEY (cancelled_by) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_ap_invoice_no ON ap_invoice(invoice_no);
CREATE INDEX IF NOT EXISTS idx_ap_invoice_supplier_id ON ap_invoice(supplier_id);
CREATE INDEX IF NOT EXISTS idx_ap_invoice_date ON ap_invoice(invoice_date);
CREATE INDEX IF NOT EXISTS idx_ap_invoice_due_date ON ap_invoice(due_date);
CREATE INDEX IF NOT EXISTS idx_ap_invoice_status ON ap_invoice(invoice_status);
CREATE INDEX IF NOT EXISTS idx_ap_invoice_source ON ap_invoice(source_type, source_id);
CREATE INDEX IF NOT EXISTS idx_ap_invoice_created_by ON ap_invoice(created_by);

-- 触发器：更新时间
CREATE TRIGGER update_ap_invoice_updated_at
BEFORE UPDATE ON ap_invoice
FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- 触发器：更新已付金额和状态
CREATE OR REPLACE FUNCTION update_ap_invoice_status()
RETURNS TRIGGER AS $$
BEGIN
    -- 更新应付单的已付金额和未付金额
    NEW.paid_amount = COALESCE(NEW.paid_amount, 0.00);
    NEW.unpaid_amount = NEW.amount - NEW.paid_amount;
    
    -- 更新应付状态
    IF NEW.invoice_status = 'AUDITED' THEN
        IF NEW.paid_amount = 0 THEN
            NEW.invoice_status = 'AUDITED';
        ELSIF NEW.paid_amount >= NEW.amount THEN
            NEW.invoice_status = 'PAID';
        ELSE
            NEW.invoice_status = 'PARTIAL_PAID';
        END IF;
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_ap_invoice_status
BEFORE UPDATE OF paid_amount ON ap_invoice
FOR EACH ROW EXECUTE FUNCTION update_ap_invoice_status();


-- =====================================================
-- 2. 付款申请表 (ap_payment_request)
-- =====================================================
CREATE TABLE ap_payment_request (
    id SERIAL PRIMARY KEY,                              -- 主键 ID
    request_no VARCHAR(50) NOT NULL UNIQUE,             -- 付款申请单号（PR20260315001）
    request_date DATE NOT NULL,                         -- 申请日期
    supplier_id INTEGER NOT NULL,                       -- 供应商 ID（外键）
    payment_type VARCHAR(20) NOT NULL,                  -- 付款类型：PREPAYMENT=预付款，PROGRESS=进度款，FINAL=尾款，WARRANTY=质保金
    payment_method VARCHAR(20) NOT NULL,                -- 付款方式：TT=电汇，LC=信用证，DP=付款交单，DA=承兑交单，CHECK=支票，CASH=现金
    request_amount DECIMAL(18,2) NOT NULL,              -- 申请金额
    approval_status VARCHAR(20) DEFAULT 'DRAFT',        -- 审批状态：DRAFT=草稿，APPROVING=审批中，APPROVED=已审批，REJECTED=已拒绝
    currency VARCHAR(10) DEFAULT 'CNY',                 -- 币种
    exchange_rate DECIMAL(18,6) DEFAULT 1.000000,       -- 汇率
    request_amount_foreign DECIMAL(18,2),               -- 外币金额
    expected_payment_date DATE,                         -- 期望付款日期
    bank_name VARCHAR(200),                             -- 收款银行
    bank_account VARCHAR(50),                           -- 收款账号
    bank_account_name VARCHAR(200),                     -- 收款账户名
    notes TEXT,                                         -- 备注
    attachment_urls TEXT[],                             -- 附件 URL 列表
    created_by INTEGER NOT NULL,                        -- 创建人 ID
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 创建时间
    updated_by INTEGER,                                 -- 更新人 ID
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 更新时间
    submitted_by INTEGER,                               -- 提交人 ID
    submitted_at TIMESTAMP,                             -- 提交时间
    approved_by INTEGER,                                -- 审批人 ID
    approved_at TIMESTAMP,                              -- 审批时间
    rejected_by INTEGER,                                -- 拒绝人 ID
    rejected_at TIMESTAMP,                              -- 拒绝时间
    rejected_reason TEXT                                -- 拒绝原因
);

-- 外键约束
ALTER TABLE ap_payment_request ADD CONSTRAINT fk_ap_request_supplier
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id);
ALTER TABLE ap_payment_request ADD CONSTRAINT fk_ap_request_created_by
    FOREIGN KEY (created_by) REFERENCES users(id);
ALTER TABLE ap_payment_request ADD CONSTRAINT fk_ap_request_updated_by
    FOREIGN KEY (updated_by) REFERENCES users(id);
ALTER TABLE ap_payment_request ADD CONSTRAINT fk_ap_request_submitted_by
    FOREIGN KEY (submitted_by) REFERENCES users(id);
ALTER TABLE ap_payment_request ADD CONSTRAINT fk_ap_request_approved_by
    FOREIGN KEY (approved_by) REFERENCES users(id);
ALTER TABLE ap_payment_request ADD CONSTRAINT fk_ap_request_rejected_by
    FOREIGN KEY (rejected_by) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_ap_request_no ON ap_payment_request(request_no);
CREATE INDEX IF NOT EXISTS idx_ap_request_supplier_id ON ap_payment_request(supplier_id);
CREATE INDEX IF NOT EXISTS idx_ap_request_date ON ap_payment_request(request_date);
CREATE INDEX IF NOT EXISTS idx_ap_request_status ON ap_payment_request(approval_status);
CREATE INDEX IF NOT EXISTS idx_ap_request_created_by ON ap_payment_request(created_by);

-- 触发器：更新时间
CREATE TRIGGER update_ap_payment_request_updated_at
BEFORE UPDATE ON ap_payment_request
FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();


-- =====================================================
-- 3. 付款申请明细表 (ap_payment_request_item)
-- =====================================================
CREATE TABLE ap_payment_request_item (
    id SERIAL PRIMARY KEY,                              -- 主键 ID
    request_id INTEGER NOT NULL,                        -- 付款申请 ID（外键）
    invoice_id INTEGER NOT NULL,                        -- 应付单 ID（外键）
    apply_amount DECIMAL(18,2) NOT NULL,                -- 申请金额
    notes TEXT,                                         -- 备注
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 创建时间
    CONSTRAINT fk_ap_request_item_request
        FOREIGN KEY (request_id) REFERENCES ap_payment_request(id) ON DELETE CASCADE,
    CONSTRAINT fk_ap_request_item_invoice
        FOREIGN KEY (invoice_id) REFERENCES ap_invoice(id)
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_ap_request_item_request_id ON ap_payment_request_item(request_id);
CREATE INDEX IF NOT EXISTS idx_ap_request_item_invoice_id ON ap_payment_request_item(invoice_id);


-- =====================================================
-- 4. 付款单表 (ap_payment)
-- =====================================================
CREATE TABLE ap_payment (
    id SERIAL PRIMARY KEY,                              -- 主键 ID
    payment_no VARCHAR(50) NOT NULL UNIQUE,             -- 付款单号（PAY20260315001）
    payment_date DATE NOT NULL,                         -- 付款日期
    supplier_id INTEGER NOT NULL,                       -- 供应商 ID（外键）
    request_id INTEGER,                                 -- 付款申请 ID（外键）
    payment_method VARCHAR(20) NOT NULL,                -- 付款方式：TT/LC/DP/DA/CHECK/CASH
    payment_amount DECIMAL(18,2) NOT NULL,              -- 付款金额
    payment_status VARCHAR(20) DEFAULT 'REGISTERED',    -- 付款状态：REGISTERED=已登记，CONFIRMED=已确认
    currency VARCHAR(10) DEFAULT 'CNY',                 -- 币种
    exchange_rate DECIMAL(18,6) DEFAULT 1.000000,       -- 汇率
    payment_amount_foreign DECIMAL(18,2),               -- 外币金额
    bank_name VARCHAR(200),                             -- 付款银行
    bank_account VARCHAR(50),                           -- 付款账号
    transaction_no VARCHAR(100),                        -- 交易流水号
    notes TEXT,                                         -- 备注
    attachment_urls TEXT[],                             -- 附件 URL 列表（付款凭证）
    created_by INTEGER NOT NULL,                        -- 创建人 ID
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 创建时间
    updated_by INTEGER,                                 -- 更新人 ID
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 更新时间
    confirmed_by INTEGER,                               -- 确认人 ID
    confirmed_at TIMESTAMP                              -- 确认时间
);

-- 外键约束
ALTER TABLE ap_payment ADD CONSTRAINT fk_ap_payment_supplier
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id);
ALTER TABLE ap_payment ADD CONSTRAINT fk_ap_payment_request
    FOREIGN KEY (request_id) REFERENCES ap_payment_request(id);
ALTER TABLE ap_payment ADD CONSTRAINT fk_ap_payment_created_by
    FOREIGN KEY (created_by) REFERENCES users(id);
ALTER TABLE ap_payment ADD CONSTRAINT fk_ap_payment_updated_by
    FOREIGN KEY (updated_by) REFERENCES users(id);
ALTER TABLE ap_payment ADD CONSTRAINT fk_ap_payment_confirmed_by
    FOREIGN KEY (confirmed_by) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_ap_payment_no ON ap_payment(payment_no);
CREATE INDEX IF NOT EXISTS idx_ap_payment_supplier_id ON ap_payment(supplier_id);
CREATE INDEX IF NOT EXISTS idx_ap_payment_date ON ap_payment(payment_date);
CREATE INDEX IF NOT EXISTS idx_ap_payment_status ON ap_payment(payment_status);
CREATE INDEX IF NOT EXISTS idx_ap_payment_request_id ON ap_payment(request_id);
CREATE INDEX IF NOT EXISTS idx_ap_payment_created_by ON ap_payment(created_by);

-- 触发器：更新时间
CREATE TRIGGER update_ap_payment_updated_at
BEFORE UPDATE ON ap_payment
FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();


-- =====================================================
-- 5. 应付核销表 (ap_verification)
-- =====================================================
CREATE TABLE ap_verification (
    id SERIAL PRIMARY KEY,                              -- 主键 ID
    verification_no VARCHAR(50) NOT NULL UNIQUE,        -- 核销单号（VER20260315001）
    verification_date DATE NOT NULL,                    -- 核销日期
    supplier_id INTEGER NOT NULL,                       -- 供应商 ID（外键）
    verification_type VARCHAR(20) NOT NULL,             -- 核销方式：AUTO=自动核销，MANUAL=手工核销
    total_amount DECIMAL(18,2) NOT NULL,                -- 核销总金额
    verification_status VARCHAR(20) DEFAULT 'COMPLETED',-- 核销状态：COMPLETED=已完成，CANCELLED=已取消
    notes TEXT,                                         -- 备注
    created_by INTEGER NOT NULL,                        -- 创建人 ID
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 创建时间
    cancelled_by INTEGER,                               -- 取消人 ID
    cancelled_at TIMESTAMP,                             -- 取消时间
    cancelled_reason TEXT                               -- 取消原因
);

-- 外键约束
ALTER TABLE ap_verification ADD CONSTRAINT fk_ap_verification_supplier
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id);
ALTER TABLE ap_verification ADD CONSTRAINT fk_ap_verification_created_by
    FOREIGN KEY (created_by) REFERENCES users(id);
ALTER TABLE ap_verification ADD CONSTRAINT fk_ap_verification_cancelled_by
    FOREIGN KEY (cancelled_by) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_ap_verification_no ON ap_verification(verification_no);
CREATE INDEX IF NOT EXISTS idx_ap_verification_supplier_id ON ap_verification(supplier_id);
CREATE INDEX IF NOT EXISTS idx_ap_verification_date ON ap_verification(verification_date);
CREATE INDEX IF NOT EXISTS idx_ap_verification_status ON ap_verification(verification_status);
CREATE INDEX IF NOT EXISTS idx_ap_verification_created_by ON ap_verification(created_by);

-- 触发器：更新时间
CREATE TRIGGER update_ap_verification_updated_at
BEFORE UPDATE ON ap_verification
FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();


-- =====================================================
-- 6. 核销明细表 (ap_verification_item)
-- =====================================================
CREATE TABLE ap_verification_item (
    id SERIAL PRIMARY KEY,                              -- 主键 ID
    verification_id INTEGER NOT NULL,                   -- 核销单 ID（外键）
    invoice_id INTEGER NOT NULL,                        -- 应付单 ID（外键）
    payment_id INTEGER NOT NULL,                        -- 付款单 ID（外键）
    verify_amount DECIMAL(18,2) NOT NULL,               -- 核销金额
    notes TEXT,                                         -- 备注
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 创建时间
    CONSTRAINT fk_ap_verification_item_verification
        FOREIGN KEY (verification_id) REFERENCES ap_verification(id) ON DELETE CASCADE,
    CONSTRAINT fk_ap_verification_item_invoice
        FOREIGN KEY (invoice_id) REFERENCES ap_invoice(id),
    CONSTRAINT fk_ap_verification_item_payment
        FOREIGN KEY (payment_id) REFERENCES ap_payment(id)
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_ap_verification_item_verification_id ON ap_verification_item(verification_id);
CREATE INDEX IF NOT EXISTS idx_ap_verification_item_invoice_id ON ap_verification_item(invoice_id);
CREATE INDEX IF NOT EXISTS idx_ap_verification_item_payment_id ON ap_verification_item(payment_id);


-- =====================================================
-- 7. 供应商对账单 (ap_reconciliation)
-- =====================================================
CREATE TABLE ap_reconciliation (
    id SERIAL PRIMARY KEY,                              -- 主键 ID
    reconciliation_no VARCHAR(50) NOT NULL UNIQUE,      -- 对账单号（REC20260315001）
    supplier_id INTEGER NOT NULL,                       -- 供应商 ID（外键）
    start_date DATE NOT NULL,                           -- 对账开始日期
    end_date DATE NOT NULL,                             -- 对账结束日期
    opening_balance DECIMAL(18,2) DEFAULT 0.00,         -- 期初余额
    total_invoice DECIMAL(18,2) DEFAULT 0.00,           -- 本期应付合计
    total_payment DECIMAL(18,2) DEFAULT 0.00,           -- 本期付款合计
    closing_balance DECIMAL(18,2) DEFAULT 0.00,         -- 期末余额
    reconciliation_status VARCHAR(20) DEFAULT 'PENDING',-- 对账状态：PENDING=待确认，CONFIRMED=已确认，DISPUTED=有争议
    notes TEXT,                                         -- 备注
    created_by INTEGER NOT NULL,                        -- 创建人 ID
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 创建时间
    confirmed_by INTEGER,                               -- 确认人 ID（供应商确认）
    confirmed_at TIMESTAMP,                             -- 确认时间
    disputed_by INTEGER,                                -- 争议人 ID
    disputed_at TIMESTAMP,                              -- 争议时间
    disputed_reason TEXT                                -- 争议原因
);

-- 外键约束
ALTER TABLE ap_reconciliation ADD CONSTRAINT fk_ap_reconciliation_supplier
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id);
ALTER TABLE ap_reconciliation ADD CONSTRAINT fk_ap_reconciliation_created_by
    FOREIGN KEY (created_by) REFERENCES users(id);
ALTER TABLE ap_reconciliation ADD CONSTRAINT fk_ap_reconciliation_confirmed_by
    FOREIGN KEY (confirmed_by) REFERENCES users(id);
ALTER TABLE ap_reconciliation ADD CONSTRAINT fk_ap_reconciliation_disputed_by
    FOREIGN KEY (disputed_by) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_ap_reconciliation_no ON ap_reconciliation(reconciliation_no);
CREATE INDEX IF NOT EXISTS idx_ap_reconciliation_supplier_id ON ap_reconciliation(supplier_id);
CREATE INDEX IF NOT EXISTS idx_ap_reconciliation_date ON ap_reconciliation(start_date, end_date);
CREATE INDEX IF NOT EXISTS idx_ap_reconciliation_status ON ap_reconciliation(reconciliation_status);
CREATE INDEX IF NOT EXISTS idx_ap_reconciliation_created_by ON ap_reconciliation(created_by);

-- 触发器：更新时间
CREATE TRIGGER update_ap_reconciliation_updated_at
BEFORE UPDATE ON ap_reconciliation
FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();


-- =====================================================
-- 8. 物化视图：供应商应付汇总表
-- =====================================================
CREATE MATERIALIZED VIEW mv_supplier_ap_summary AS
SELECT
    s.id AS supplier_id,
    s.supplier_code,
    s.supplier_name,
    COUNT(DISTINCT inv.id) AS total_invoice_count,
    SUM(inv.amount) AS total_invoice_amount,
    SUM(inv.paid_amount) AS total_paid_amount,
    SUM(inv.unpaid_amount) AS total_unpaid_amount,
    COUNT(DISTINCT CASE WHEN inv.invoice_status = 'PAID' THEN inv.id END) AS paid_invoice_count,
    COUNT(DISTINCT CASE WHEN inv.invoice_status = 'PARTIAL_PAID' THEN inv.id END) AS partial_paid_invoice_count,
    COUNT(DISTINCT CASE WHEN inv.unpaid_amount > 0 AND inv.due_date < CURRENT_DATE THEN inv.id END) AS overdue_invoice_count,
    SUM(CASE WHEN inv.unpaid_amount > 0 AND inv.due_date < CURRENT_DATE THEN inv.unpaid_amount ELSE 0 END) AS overdue_amount
FROM suppliers s
LEFT JOIN ap_invoice inv ON s.id = inv.supplier_id AND inv.invoice_status NOT IN ('DRAFT', 'CANCELLED')
GROUP BY s.id, s.supplier_code, s.supplier_name;

-- 物化视图索引
CREATE INDEX IF NOT EXISTS idx_mv_supplier_ap_summary_supplier_id ON mv_supplier_ap_summary(supplier_id);
CREATE INDEX IF NOT EXISTS idx_mv_supplier_ap_summary_code ON mv_supplier_ap_summary(supplier_code);
CREATE INDEX IF NOT EXISTS idx_mv_supplier_ap_summary_name ON mv_supplier_ap_summary(supplier_name);

-- 刷新物化视图的函数
CREATE OR REPLACE FUNCTION refresh_mv_supplier_ap_summary()
RETURNS void AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY mv_supplier_ap_summary;
END;
$$ LANGUAGE plpgsql;


-- =====================================================
-- 9. 回滚语句（仅在需要时手动执行）
-- =====================================================
-- DROP MATERIALIZED VIEW IF EXISTS mv_supplier_ap_summary CASCADE;
-- DROP TABLE IF EXISTS ap_reconciliation CASCADE;
-- DROP TABLE IF EXISTS ap_verification_item CASCADE;
-- DROP TABLE IF EXISTS ap_verification CASCADE;
-- DROP TABLE IF EXISTS ap_payment CASCADE;
-- DROP TABLE IF EXISTS ap_payment_request_item CASCADE;
-- DROP TABLE IF EXISTS ap_payment_request CASCADE;
-- DROP TABLE IF EXISTS ap_invoice CASCADE;
