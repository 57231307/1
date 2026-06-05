-- 质量检验模块外键关联迁移
-- 日期: 2026-05-09
-- 描述: 为质量检验记录建立外键关联

BEGIN;

-- 为质量检验记录添加采购入库单外键
ALTER TABLE quality_inspection_records 
    ADD COLUMN IF NOT EXISTS receipt_id INTEGER REFERENCES purchase_receipts(id);

CREATE INDEX IF NOT EXISTS idx_quality_receipt ON quality_inspection_records(receipt_id);

-- 为质量检验记录添加产品外键
ALTER TABLE quality_inspection_records 
    ADD COLUMN IF NOT EXISTS product_id INTEGER REFERENCES products(id);

CREATE INDEX IF NOT EXISTS idx_quality_product ON quality_inspection_records(product_id);

-- 为质量检验记录添加检验员外键
ALTER TABLE quality_inspection_records 
    ADD COLUMN IF NOT EXISTS inspector_id INTEGER REFERENCES users(id);

CREATE INDEX IF NOT EXISTS idx_quality_inspector ON quality_inspection_records(inspector_id);

-- 为质量缺陷记录添加检验记录外键
ALTER TABLE quality_defect_records 
    ADD COLUMN IF NOT EXISTS inspection_id INTEGER REFERENCES quality_inspection_records(id);

CREATE INDEX IF NOT EXISTS idx_defect_inspection ON quality_defect_records(inspection_id);

COMMIT;
