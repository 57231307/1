-- ============================================
-- 辅助核算管理模块 - 基础表结构
-- ============================================
-- 文档编号：MIGRATION-060-ASSIST-ACCOUNTING
-- 创建日期：2026-03-17
-- 说明：辅助核算管理模块基础表结构，包含维度定义、核算记录、汇总数据
-- ============================================

-- 1. 辅助核算维度表
-- ============================================
CREATE TABLE IF NOT EXISTS assist_accounting_dimension (
    id SERIAL PRIMARY KEY,
    dimension_code VARCHAR(50) NOT NULL UNIQUE,
    dimension_name VARCHAR(200) NOT NULL,
    description TEXT,
    is_active BOOLEAN DEFAULT TRUE,
    sort_order INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE assist_accounting_dimension IS '辅助核算维度表';
COMMENT ON COLUMN assist_accounting_dimension.dimension_code IS '维度编码：BATCH, COLOR, DYE_LOT, GRADE, WORKSHOP, WAREHOUSE, CUSTOMER, SUPPLIER';
COMMENT ON COLUMN assist_accounting_dimension.dimension_name IS '维度名称';
COMMENT ON COLUMN assist_accounting_dimension.description IS '维度描述';
COMMENT ON COLUMN assist_accounting_dimension.is_active IS '是否启用';
COMMENT ON COLUMN assist_accounting_dimension.sort_order IS '排序顺序';

-- 索引
CREATE INDEX IF NOT EXISTS idx_dimension_code ON assist_accounting_dimension(dimension_code);
CREATE INDEX IF NOT EXISTS idx_dimension_active ON assist_accounting_dimension(is_active);

-- 2. 辅助核算记录表
-- ============================================
CREATE TABLE IF NOT EXISTS assist_accounting_record (
    id SERIAL PRIMARY KEY,
    business_type VARCHAR(50) NOT NULL,
    business_no VARCHAR(100) NOT NULL,
    business_id INTEGER NOT NULL,
    account_subject_id INTEGER NOT NULL REFERENCES account_subjects(id),
    debit_amount DECIMAL(12,2) NOT NULL DEFAULT 0,
    credit_amount DECIMAL(12,2) NOT NULL DEFAULT 0,
    five_dimension_id VARCHAR(255) NOT NULL,
    product_id INTEGER NOT NULL,
    batch_no VARCHAR(100) NOT NULL,
    color_no VARCHAR(100) NOT NULL,
    dye_lot_no VARCHAR(100),
    grade VARCHAR(50) NOT NULL,
    workshop_id INTEGER,
    warehouse_id INTEGER NOT NULL,
    customer_id INTEGER,
    supplier_id INTEGER,
    quantity_meters DECIMAL(12,2) NOT NULL DEFAULT 0,
    quantity_kg DECIMAL(12,2) NOT NULL DEFAULT 0,
    remarks TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    created_by INTEGER
);

COMMENT ON TABLE assist_accounting_record IS '辅助核算记录表';
COMMENT ON COLUMN assist_accounting_record.business_type IS '业务类型：PURCHASE, SALES, INVENTORY, PRODUCTION';
COMMENT ON COLUMN assist_accounting_record.business_no IS '业务单号';
COMMENT ON COLUMN assist_accounting_record.business_id IS '业务单 ID';
COMMENT ON COLUMN assist_accounting_record.account_subject_id IS '会计科目 ID';
COMMENT ON COLUMN assist_accounting_record.debit_amount IS '借方金额';
COMMENT ON COLUMN assist_accounting_record.credit_amount IS '贷方金额';
COMMENT ON COLUMN assist_accounting_record.five_dimension_id IS '五维 ID';
COMMENT ON COLUMN assist_accounting_record.product_id IS '产品 ID';
COMMENT ON COLUMN assist_accounting_record.batch_no IS '批次号';
COMMENT ON COLUMN assist_accounting_record.color_no IS '色号';
COMMENT ON COLUMN assist_accounting_record.dye_lot_no IS '缸号';
COMMENT ON COLUMN assist_accounting_record.grade IS '等级';
COMMENT ON COLUMN assist_accounting_record.workshop_id IS '车间 ID';
COMMENT ON COLUMN assist_accounting_record.warehouse_id IS '仓库 ID';
COMMENT ON COLUMN assist_accounting_record.customer_id IS '客户 ID';
COMMENT ON COLUMN assist_accounting_record.supplier_id IS '供应商 ID';
COMMENT ON COLUMN assist_accounting_record.quantity_meters IS '数量（米）';
COMMENT ON COLUMN assist_accounting_record.quantity_kg IS '数量（公斤）';
COMMENT ON COLUMN assist_accounting_record.remarks IS '备注';
COMMENT ON COLUMN assist_accounting_record.created_by IS '创建人 ID';

-- 索引
CREATE INDEX IF NOT EXISTS idx_record_business ON assist_accounting_record(business_type, business_no);
CREATE INDEX IF NOT EXISTS idx_record_five_dimension ON assist_accounting_record(five_dimension_id);
CREATE INDEX IF NOT EXISTS idx_record_subject ON assist_accounting_record(account_subject_id);
CREATE INDEX IF NOT EXISTS idx_record_batch ON assist_accounting_record(batch_no);
CREATE INDEX IF NOT EXISTS idx_record_color_no ON assist_accounting_record(color_no);
CREATE INDEX IF NOT EXISTS idx_record_warehouse ON assist_accounting_record(warehouse_id);
CREATE INDEX IF NOT EXISTS idx_record_created_at ON assist_accounting_record(created_at);
CREATE INDEX IF NOT EXISTS idx_record_business_five ON assist_accounting_record(business_type, five_dimension_id);
CREATE INDEX IF NOT EXISTS idx_record_period ON assist_accounting_record(created_at, account_subject_id);

-- 3. 辅助核算汇总表
-- ============================================
CREATE TABLE IF NOT EXISTS assist_accounting_summary (
    id SERIAL PRIMARY KEY,
    accounting_period VARCHAR(7) NOT NULL,
    dimension_code VARCHAR(50) NOT NULL,
    dimension_value_id INTEGER NOT NULL,
    dimension_value_name VARCHAR(200) NOT NULL,
    account_subject_id INTEGER NOT NULL REFERENCES account_subjects(id),
    total_debit DECIMAL(12,2) NOT NULL DEFAULT 0,
    total_credit DECIMAL(12,2) NOT NULL DEFAULT 0,
    total_quantity_meters DECIMAL(12,2) NOT NULL DEFAULT 0,
    total_quantity_kg DECIMAL(12,2) NOT NULL DEFAULT 0,
    record_count BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    -- 唯一约束：按期间 + 维度 + 科目汇总
    UNIQUE(accounting_period, dimension_code, dimension_value_id, account_subject_id)
);

COMMENT ON TABLE assist_accounting_summary IS '辅助核算汇总表';
COMMENT ON COLUMN assist_accounting_summary.accounting_period IS '会计期间（格式：YYYY-MM）';
COMMENT ON COLUMN assist_accounting_summary.dimension_code IS '维度编码';
COMMENT ON COLUMN assist_accounting_summary.dimension_value_id IS '维度值 ID（如批次 ID、色号 ID 等）';
COMMENT ON COLUMN assist_accounting_summary.dimension_value_name IS '维度值名称';
COMMENT ON COLUMN assist_accounting_summary.account_subject_id IS '会计科目 ID';
COMMENT ON COLUMN assist_accounting_summary.total_debit IS '借方金额合计';
COMMENT ON COLUMN assist_accounting_summary.total_credit IS '贷方金额合计';
COMMENT ON COLUMN assist_accounting_summary.total_quantity_meters IS '数量（米）合计';
COMMENT ON COLUMN assist_accounting_summary.total_quantity_kg IS '数量（公斤）合计';
COMMENT ON COLUMN assist_accounting_summary.record_count IS '记录数';

-- 索引
CREATE INDEX IF NOT EXISTS idx_summary_period ON assist_accounting_summary(accounting_period);
CREATE INDEX IF NOT EXISTS idx_summary_dimension ON assist_accounting_summary(dimension_code);
CREATE INDEX IF NOT EXISTS idx_summary_subject ON assist_accounting_summary(account_subject_id);
CREATE INDEX IF NOT EXISTS idx_summary_period_dimension ON assist_accounting_summary(accounting_period, dimension_code, dimension_value_id);

-- 4. 插入预设的 8 个辅助核算维度
-- ============================================
INSERT INTO assist_accounting_dimension (dimension_code, dimension_name, description, is_active, sort_order) VALUES
('BATCH', '批次核算', '按生产批次进行辅助核算', TRUE, 1),
('COLOR', '色号核算', '按产品色号进行辅助核算', TRUE, 2),
('DYE_LOT', '缸号核算', '按染色缸次进行辅助核算', TRUE, 3),
('GRADE', '等级核算', '按产品质量等级进行辅助核算', TRUE, 4),
('WORKSHOP', '车间核算', '按生产车间进行辅助核算', TRUE, 5),
('WAREHOUSE', '仓库核算', '按仓库进行辅助核算', TRUE, 6),
('CUSTOMER', '客户核算', '按客户进行辅助核算', TRUE, 7),
('SUPPLIER', '供应商核算', '按供应商进行辅助核算', TRUE, 8) ON CONFLICT DO NOTHING;

COMMENT ON INSERT: '插入 8 个辅助核算维度';

-- 4. 创建辅助核算专用的 updated_at 更新函数
-- ============================================
CREATE OR REPLACE FUNCTION update_account_subject_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION update_account_subject_timestamp() IS '辅助核算表专用时间戳更新函数';

-- 5. 触发器：自动更新 updated_at 字段
-- ============================================
DROP TRIGGER IF EXISTS trg_update_assist_dimension ON assist_accounting_dimension;
CREATE TRIGGER trg_update_assist_dimension
BEFORE UPDATE ON assist_accounting_dimension
FOR EACH ROW
EXECUTE FUNCTION update_account_subject_timestamp();

DROP TRIGGER IF EXISTS trg_update_assist_summary ON assist_accounting_summary;
CREATE TRIGGER trg_update_assist_summary
BEFORE UPDATE ON assist_accounting_summary
FOR EACH ROW
EXECUTE FUNCTION update_account_subject_timestamp();

COMMENT ON TRIGGER trg_update_assist_dimension IS '自动更新辅助核算维度 updated_at 字段';
COMMENT ON TRIGGER trg_update_assist_summary IS '自动更新辅助核算汇总 updated_at 字段';

-- ============================================
-- 迁移完成
-- ============================================
