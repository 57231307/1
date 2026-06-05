-- 核心业务流程外键约束
-- 创建时间: 2026-05-09
-- 说明: 为核心业务表添加数据库级外键约束，确保数据一致性

-- 销售订单 → 客户
ALTER TABLE sales_orders
    ADD CONSTRAINT fk_sales_orders_customer
    FOREIGN KEY (customer_id) REFERENCES customers(id)
    ON DELETE RESTRICT ON UPDATE CASCADE;

-- 采购订单 → 供应商
ALTER TABLE purchase_orders
    ADD CONSTRAINT fk_purchase_orders_supplier
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id)
    ON DELETE RESTRICT ON UPDATE CASCADE;

-- 库存 → 产品
ALTER TABLE inventory_stock
    ADD CONSTRAINT fk_inventory_stock_product
    FOREIGN KEY (product_id) REFERENCES products(id)
    ON DELETE RESTRICT ON UPDATE CASCADE;

-- 库存 → 仓库
ALTER TABLE inventory_stock
    ADD CONSTRAINT fk_inventory_stock_warehouse
    FOREIGN KEY (warehouse_id) REFERENCES warehouses(id)
    ON DELETE RESTRICT ON UPDATE CASCADE;

-- 采购入库 → 采购订单
ALTER TABLE purchase_receipts
    ADD CONSTRAINT fk_purchase_receipts_order
    FOREIGN KEY (order_id) REFERENCES purchase_orders(id)
    ON DELETE RESTRICT ON UPDATE CASCADE;

-- 销售发货 → 销售订单
ALTER TABLE sales_deliveries
    ADD CONSTRAINT fk_sales_deliveries_order
    FOREIGN KEY (order_id) REFERENCES sales_orders(id)
    ON DELETE RESTRICT ON UPDATE CASCADE;

-- 销售退货 → 销售订单
ALTER TABLE sales_returns
    ADD CONSTRAINT fk_sales_returns_order
    FOREIGN KEY (order_id) REFERENCES sales_orders(id)
    ON DELETE RESTRICT ON UPDATE CASCADE;

-- 采购退货 → 采购订单
ALTER TABLE purchase_returns
    ADD CONSTRAINT fk_purchase_returns_order
    FOREIGN KEY (order_id) REFERENCES purchase_orders(id)
    ON DELETE RESTRICT ON UPDATE CASCADE;

-- 用户 → 部门
ALTER TABLE users
    ADD CONSTRAINT fk_users_department
    FOREIGN KEY (department_id) REFERENCES departments(id)
    ON DELETE SET NULL ON UPDATE CASCADE;

-- 产品 → 产品分类
ALTER TABLE products
    ADD CONSTRAINT fk_products_category
    FOREIGN KEY (category_id) REFERENCES product_categories(id)
    ON DELETE RESTRICT ON UPDATE CASCADE;
