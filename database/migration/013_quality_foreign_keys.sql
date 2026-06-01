-- 质量检验模块外键关联迁移
-- 日期: 2026-05-09
-- 描述: 为质量检验记录建立外键关联
-- 注意: purchase_receipt_id 列已在 003_foreign_keys.sql 中添加

BEGIN;

-- 为质量检验记录添加产品外键
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'quality_inspection_records') 
       AND EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'products') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'quality_inspection_records' AND column_name = 'product_id') THEN
            ALTER TABLE quality_inspection_records ADD COLUMN product_id INTEGER;
        END IF;
        IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_quality_product') THEN
            ALTER TABLE quality_inspection_records ADD CONSTRAINT fk_quality_product FOREIGN KEY (product_id) REFERENCES products(id);
        END IF;
        CREATE INDEX IF NOT EXISTS idx_quality_product ON quality_inspection_records(product_id);
    END IF;
END $$;

-- 为质量检验记录添加检验员外键
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'quality_inspection_records') 
       AND EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'users') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'quality_inspection_records' AND column_name = 'inspector_id') THEN
            ALTER TABLE quality_inspection_records ADD COLUMN inspector_id INTEGER;
        END IF;
        IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_quality_inspector') THEN
            ALTER TABLE quality_inspection_records ADD CONSTRAINT fk_quality_inspector FOREIGN KEY (inspector_id) REFERENCES users(id);
        END IF;
        CREATE INDEX IF NOT EXISTS idx_quality_inspector ON quality_inspection_records(inspector_id);
    END IF;
END $$;

-- 为质量缺陷记录添加检验记录外键（如果表存在）
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'quality_defect_records') 
       AND EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'quality_inspection_records') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'quality_defect_records' AND column_name = 'inspection_id') THEN
            ALTER TABLE quality_defect_records ADD COLUMN inspection_id INTEGER;
        END IF;
        IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_defect_inspection') THEN
            ALTER TABLE quality_defect_records ADD CONSTRAINT fk_defect_inspection FOREIGN KEY (inspection_id) REFERENCES quality_inspection_records(id);
        END IF;
        CREATE INDEX IF NOT EXISTS idx_defect_inspection ON quality_defect_records(inspection_id);
    END IF;
END $$;

COMMIT;
