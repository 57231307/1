-- 迁移脚本: 020_fix_purchase_orders_table.sql
-- 描述: 为 purchase_order 表创建 purchase_orders 视图，解决外键引用和代码不一致问题
-- 日期: 2026-05-19
-- 依赖: 001_consolidated_schema.sql (创建 purchase_order 表)

BEGIN;

-- =====================================================
-- 1. 检查 purchase_order 表是否存在而 purchase_orders 不存在
-- =====================================================
DO $$
BEGIN
    -- 如果 purchase_orders 表/视图不存在但 purchase_order 存在，创建视图
    IF EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_name = 'purchase_order' AND table_schema = CURRENT_SCHEMA()
    ) AND NOT EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_name = 'purchase_orders' AND table_schema = CURRENT_SCHEMA()
    ) THEN
        -- 创建同义词视图，使 purchase_orders 可用
        CREATE OR REPLACE VIEW purchase_orders AS
        SELECT
            id,
            order_no,
            supplier_id,
            order_date,
            expected_delivery_date,
            actual_delivery_date,
            warehouse_id,
            department_id,
            purchaser_id,
            currency,
            exchange_rate,
            total_amount,
            total_amount_foreign,
            total_quantity,
            total_quantity_alt,
            order_status,
            payment_terms,
            shipping_terms,
            notes,
            attachment_urls,
            created_by,
            created_at,
            updated_by,
            updated_at,
            approved_by,
            approved_at,
            rejected_reason
        FROM purchase_order;

        COMMENT ON VIEW purchase_orders IS '采购订单视图（purchase_order 的别名，解决表名不一致问题）';

        RAISE NOTICE '已创建 purchase_orders 视图';
    END IF;
END $$;

-- =====================================================
-- 2. 如果 purchase_orders 表已存在（可能是后来创建的），检查结构一致性
-- =====================================================
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_name = 'purchase_orders' AND table_schema = CURRENT_SCHEMA()
            AND table_type = 'BASE TABLE'
    ) AND EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_name = 'purchase_order' AND table_schema = CURRENT_SCHEMA()
            AND table_type = 'BASE TABLE'
    ) THEN
        RAISE WARNING 'purchase_order 和 purchase_orders 两个表同时存在，请检查数据一致性';
    END IF;
END $$;

COMMIT;

-- 验证迁移
SELECT 'Migration 020_fix_purchase_orders_table completed successfully' AS status;
