-- 核心业务流程外键约束
-- 创建时间: 2026-05-09
-- 说明: 为核心业务表添加数据库级外键约束，确保数据一致性

-- 销售订单 → 客户
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'sales_orders') 
       AND EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'customers') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_sales_orders_customer') THEN
            ALTER TABLE sales_orders ADD CONSTRAINT fk_sales_orders_customer FOREIGN KEY (customer_id) REFERENCES customers(id) ON DELETE RESTRICT ON UPDATE CASCADE;
        END IF;
    END IF;
END $$;

-- 采购订单 → 供应商
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'purchase_order') 
       AND EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'suppliers') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_purchase_orders_supplier') THEN
            ALTER TABLE purchase_order ADD CONSTRAINT fk_purchase_orders_supplier FOREIGN KEY (supplier_id) REFERENCES suppliers(id) ON DELETE RESTRICT ON UPDATE CASCADE;
        END IF;
    END IF;
END $$;

-- 库存 → 产品
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'inventory_stocks') 
       AND EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'products') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_inventory_stock_product') THEN
            ALTER TABLE inventory_stocks ADD CONSTRAINT fk_inventory_stock_product FOREIGN KEY (product_id) REFERENCES products(id) ON DELETE RESTRICT ON UPDATE CASCADE;
        END IF;
    END IF;
END $$;

-- 库存 → 仓库
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'inventory_stocks') 
       AND EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'warehouses') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_inventory_stock_warehouse') THEN
            ALTER TABLE inventory_stocks ADD CONSTRAINT fk_inventory_stock_warehouse FOREIGN KEY (warehouse_id) REFERENCES warehouses(id) ON DELETE RESTRICT ON UPDATE CASCADE;
        END IF;
    END IF;
END $$;

-- 采购入库 → 采购订单
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'purchase_receipt') 
       AND EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'purchase_order') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_purchase_receipts_order') THEN
            ALTER TABLE purchase_receipt ADD CONSTRAINT fk_purchase_receipts_order FOREIGN KEY (order_id) REFERENCES purchase_order(id) ON DELETE RESTRICT ON UPDATE CASCADE;
        END IF;
    END IF;
END $$;

-- 销售发货 → 销售订单
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'sales_delivery') 
       AND EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'sales_orders') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_sales_deliveries_order') THEN
            ALTER TABLE sales_delivery ADD CONSTRAINT fk_sales_deliveries_order FOREIGN KEY (sales_order_id) REFERENCES sales_orders(id) ON DELETE RESTRICT ON UPDATE CASCADE;
        END IF;
    END IF;
END $$;

-- 销售退货 → 销售订单
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'sales_return') 
       AND EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'sales_orders') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_sales_returns_order') THEN
            ALTER TABLE sales_return ADD CONSTRAINT fk_sales_returns_order FOREIGN KEY (sales_order_id) REFERENCES sales_orders(id) ON DELETE RESTRICT ON UPDATE CASCADE;
        END IF;
    END IF;
END $$;

-- 采购退货 → 采购订单
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'purchase_return') 
       AND EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'purchase_order') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_purchase_returns_order') THEN
            ALTER TABLE purchase_return ADD CONSTRAINT fk_purchase_returns_order FOREIGN KEY (order_id) REFERENCES purchase_order(id) ON DELETE RESTRICT ON UPDATE CASCADE;
        END IF;
    END IF;
END $$;

-- 用户 → 部门
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'users') 
       AND EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'departments') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_users_department') THEN
            ALTER TABLE users ADD CONSTRAINT fk_users_department FOREIGN KEY (department_id) REFERENCES departments(id) ON DELETE SET NULL ON UPDATE CASCADE;
        END IF;
    END IF;
END $$;

-- 产品 → 产品分类
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'products') 
       AND EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'product_categories') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_products_category') THEN
            ALTER TABLE products ADD CONSTRAINT fk_products_category FOREIGN KEY (category_id) REFERENCES product_categories(id) ON DELETE RESTRICT ON UPDATE CASCADE;
        END IF;
    END IF;
END $$;

