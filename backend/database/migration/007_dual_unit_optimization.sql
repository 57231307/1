-- 双计量单位优化迁移脚本
-- 创建时间：2026-03-15
-- 说明：添加双计量单位计算字段、触发器和索引

-- 1. 库存表添加计算字段
ALTER TABLE inventory_stock 
ADD COLUMN IF NOT EXISTS calculated_quantity_kg DECIMAL(10,3),
ADD COLUMN IF NOT EXISTS unit_conversion_rate DECIMAL(10,6);

COMMENT ON COLUMN inventory_stock.calculated_quantity_kg IS '计算后的公斤数（自动计算）';
COMMENT ON COLUMN inventory_stock.unit_conversion_rate IS '单位换算率（公斤/米）';

-- 2. 采购入库明细表添加计算字段
ALTER TABLE purchase_receipt_item 
ADD COLUMN IF NOT EXISTS calculated_quantity_kg DECIMAL(10,3),
ADD COLUMN IF NOT EXISTS unit_conversion_rate DECIMAL(10,6);

COMMENT ON COLUMN purchase_receipt_item.calculated_quantity_kg IS '计算后的公斤数（自动计算）';
COMMENT ON COLUMN purchase_receipt_item.unit_conversion_rate IS '单位换算率（公斤/米）';

-- 3. 采购订单明细表添加计算字段
ALTER TABLE purchase_order_item 
ADD COLUMN IF NOT EXISTS calculated_quantity_kg DECIMAL(10,3),
ADD COLUMN IF NOT EXISTS unit_conversion_rate DECIMAL(10,6);

COMMENT ON COLUMN purchase_order_item.calculated_quantity_kg IS '计算后的公斤数（自动计算）';
COMMENT ON COLUMN purchase_order_item.unit_conversion_rate IS '单位换算率（公斤/米）';

-- 4. 销售出库明细表添加计算字段（如果存在该表）
DO $$ 
BEGIN 
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'sales_delivery_item') THEN
        ALTER TABLE sales_delivery_item 
        ADD COLUMN IF NOT EXISTS calculated_quantity_kg DECIMAL(10,3),
        ADD COLUMN IF NOT EXISTS unit_conversion_rate DECIMAL(10,6);
        
        COMMENT ON COLUMN sales_delivery_item.calculated_quantity_kg IS '计算后的公斤数（自动计算）';
        COMMENT ON COLUMN sales_delivery_item.unit_conversion_rate IS '单位换算率（公斤/米）';
    END IF;
END $$;

-- 5. 创建双计量单位自动计算触发器函数
CREATE OR REPLACE FUNCTION calculate_dual_unit_quantity()
RETURNS TRIGGER AS $$
BEGIN
    -- 只有当米数、克重、幅宽都存在时才计算
    IF NEW.quantity_meters IS NOT NULL 
       AND NEW.gram_weight IS NOT NULL 
       AND NEW.width_cm IS NOT NULL 
       AND NEW.quantity_meters > 0
       AND NEW.gram_weight > 0
       AND NEW.width_cm > 0 THEN
        
        -- 计算公斤数：米数 × 克重 (g/m²) × 幅宽 (m) ÷ 1000
        NEW.quantity_kg := ROUND(
            NEW.quantity_meters * NEW.gram_weight * (NEW.width_cm / 100.0) / 1000.0,
            3
        );
        NEW.calculated_quantity_kg := NEW.quantity_kg;
        
        -- 计算换算率：公斤数 ÷ 米数
        NEW.unit_conversion_rate := ROUND(
            NEW.quantity_kg / NULLIF(NEW.quantity_meters, 0),
            6
        );
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 6. 为 inventory_stock 表创建触发器
DROP TRIGGER IF EXISTS trg_calculate_dual_unit_inventory ON inventory_stock;
CREATE TRIGGER trg_calculate_dual_unit_inventory
    BEFORE INSERT OR UPDATE ON inventory_stock
    FOR EACH ROW
    EXECUTE FUNCTION calculate_dual_unit_quantity();

-- 7. 为 purchase_receipt_item 表创建触发器
DROP TRIGGER IF EXISTS trg_calculate_dual_unit_receipt ON purchase_receipt_item;
CREATE TRIGGER trg_calculate_dual_unit_receipt
    BEFORE INSERT OR UPDATE ON purchase_receipt_item
    FOR EACH ROW
    EXECUTE FUNCTION calculate_dual_unit_quantity();

-- 8. 为 purchase_order_item 表创建触发器
DROP TRIGGER IF EXISTS trg_calculate_dual_unit_order ON purchase_order_item;
CREATE TRIGGER trg_calculate_dual_unit_order
    BEFORE INSERT OR UPDATE ON purchase_order_item
    FOR EACH ROW
    EXECUTE FUNCTION calculate_dual_unit_quantity();

-- 9. 为 sales_delivery_item 表创建触发器（如果存在）
DO $$ 
BEGIN 
    IF EXISTS (
        SELECT 1 FROM information_schema.triggers 
        WHERE trigger_name = 'trg_calculate_dual_unit_sales'
    ) THEN
        DROP TRIGGER trg_calculate_dual_unit_sales ON sales_delivery_item;
    END IF;
    
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'sales_delivery_item') THEN
        CREATE TRIGGER trg_calculate_dual_unit_sales
            BEFORE INSERT OR UPDATE ON sales_delivery_item
            FOR EACH ROW
            EXECUTE FUNCTION calculate_dual_unit_quantity();
    END IF;
END $$;

-- 10. 更新现有数据（一次性操作）
UPDATE inventory_stock
SET 
    quantity_kg = ROUND(quantity_meters * gram_weight * (width_cm / 100.0) / 1000.0, 3),
    calculated_quantity_kg = ROUND(quantity_meters * gram_weight * (width_cm / 100.0) / 1000.0, 3),
    unit_conversion_rate = ROUND(
        ROUND(quantity_meters * gram_weight * (width_cm / 100.0) / 1000.0, 3) / NULLIF(quantity_meters, 0),
        6
    )
WHERE quantity_meters IS NOT NULL 
  AND gram_weight IS NOT NULL 
  AND width_cm IS NOT NULL
  AND quantity_meters > 0
  AND gram_weight > 0
  AND width_cm > 0;

UPDATE purchase_receipt_item
SET 
    quantity_kg = ROUND(quantity_received * gram_weight * (width_cm / 100.0) / 1000.0, 3),
    calculated_quantity_kg = ROUND(quantity_received * gram_weight * (width_cm / 100.0) / 1000.0, 3),
    unit_conversion_rate = ROUND(
        ROUND(quantity_received * gram_weight * (width_cm / 100.0) / 1000.0, 3) / NULLIF(quantity_received, 0),
        6
    )
WHERE quantity_received IS NOT NULL 
  AND gram_weight IS NOT NULL 
  AND width_cm IS NOT NULL
  AND quantity_received > 0
  AND gram_weight > 0
  AND width_cm > 0;

UPDATE purchase_order_item
SET 
    quantity_kg = ROUND(quantity_ordered * gram_weight * (width_cm / 100.0) / 1000.0, 3),
    calculated_quantity_kg = ROUND(quantity_ordered * gram_weight * (width_cm / 100.0) / 1000.0, 3),
    unit_conversion_rate = ROUND(
        ROUND(quantity_ordered * gram_weight * (width_cm / 100.0) / 1000.0, 3) / NULLIF(quantity_ordered, 0),
        6
    )
WHERE quantity_ordered IS NOT NULL 
  AND gram_weight IS NOT NULL 
  AND width_cm IS NOT NULL
  AND quantity_ordered > 0
  AND gram_weight > 0
  AND width_cm > 0;

-- 11. 创建索引优化查询性能
CREATE INDEX IF NOT EXISTS idx_inventory_dual_unit 
ON inventory_stock(quantity_meters, quantity_kg, gram_weight, width_cm);

CREATE INDEX IF NOT EXISTS idx_receipt_dual_unit 
ON purchase_receipt_item(quantity_received, quantity_kg, gram_weight, width_cm);

CREATE INDEX IF NOT EXISTS idx_order_dual_unit 
ON purchase_order_item(quantity_ordered, quantity_kg, gram_weight, width_cm);

-- 12. 创建视图方便双计量单位查询
CREATE OR REPLACE VIEW v_inventory_dual_unit AS
SELECT 
    id,
    product_id,
    batch_no,
    color_no,
    dye_lot_no,
    grade,
    quantity_meters,
    quantity_kg,
    calculated_quantity_kg,
    unit_conversion_rate,
    gram_weight,
    width_cm,
    warehouse_id,
    stock_status,
    quality_status,
    created_at,
    updated_at
FROM inventory_stock
WHERE quantity_meters IS NOT NULL;

COMMENT ON VIEW v_inventory_dual_unit IS '库存双计量单位视图（包含换算信息）';

-- 迁移完成提示
DO $$
BEGIN
    RAISE NOTICE '双计量单位优化迁移完成！';
    RAISE NOTICE '- 新增字段：calculated_quantity_kg, unit_conversion_rate';
    RAISE NOTICE '- 新增触发器：自动计算公斤数和换算率';
    RAISE NOTICE '- 新增索引：优化双计量单位查询';
    RAISE NOTICE '- 新增视图：v_inventory_dual_unit';
END $$;
