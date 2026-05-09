-- 库存模块外键约束
-- 创建时间: 2026-05-09
-- 说明: 为库存相关表添加数据库级外键约束

-- 库存调拨 → 仓库（出库）
ALTER TABLE inventory_transfers
    ADD CONSTRAINT fk_inventory_transfers_from_warehouse
    FOREIGN KEY (from_warehouse_id) REFERENCES warehouses(id)
    ON DELETE RESTRICT ON UPDATE CASCADE;

-- 库存调拨 → 仓库（入库）
ALTER TABLE inventory_transfers
    ADD CONSTRAINT fk_inventory_transfers_to_warehouse
    FOREIGN KEY (to_warehouse_id) REFERENCES warehouses(id)
    ON DELETE RESTRICT ON UPDATE CASCADE;

-- 库存盘点 → 仓库
ALTER TABLE inventory_counts
    ADD CONSTRAINT fk_inventory_counts_warehouse
    FOREIGN KEY (warehouse_id) REFERENCES warehouses(id)
    ON DELETE RESTRICT ON UPDATE CASCADE;

-- 库存调整 → 仓库
ALTER TABLE inventory_adjustments
    ADD CONSTRAINT fk_inventory_adjustments_warehouse
    FOREIGN KEY (warehouse_id) REFERENCES warehouses(id)
    ON DELETE RESTRICT ON UPDATE CASCADE;

-- 库存调整明细 → 库存调整
ALTER TABLE inventory_adjustment_items
    ADD CONSTRAINT fk_inventory_adjustment_items_adjustment
    FOREIGN KEY (adjustment_id) REFERENCES inventory_adjustments(id)
    ON DELETE CASCADE ON UPDATE CASCADE;

-- 库存盘点明细 → 库存盘点
ALTER TABLE inventory_count_items
    ADD CONSTRAINT fk_inventory_count_items_count
    FOREIGN KEY (count_id) REFERENCES inventory_counts(id)
    ON DELETE CASCADE ON UPDATE CASCADE;

-- 库存调拨明细 → 库存调拨
ALTER TABLE inventory_transfer_items
    ADD CONSTRAINT fk_inventory_transfer_items_transfer
    FOREIGN KEY (transfer_id) REFERENCES inventory_transfers(id)
    ON DELETE CASCADE ON UPDATE CASCADE;

-- 库存预留 → 产品
ALTER TABLE inventory_reservations
    ADD CONSTRAINT fk_inventory_reservations_product
    FOREIGN KEY (product_id) REFERENCES products(id)
    ON DELETE RESTRICT ON UPDATE CASCADE;

-- 库存预留 → 仓库
ALTER TABLE inventory_reservations
    ADD CONSTRAINT fk_inventory_reservations_warehouse
    FOREIGN KEY (warehouse_id) REFERENCES warehouses(id)
    ON DELETE RESTRICT ON UPDATE CASCADE;

-- 库存交易记录 → 产品
ALTER TABLE inventory_transactions
    ADD CONSTRAINT fk_inventory_transactions_product
    FOREIGN KEY (product_id) REFERENCES products(id)
    ON DELETE RESTRICT ON UPDATE CASCADE;

-- 库存交易记录 → 仓库
ALTER TABLE inventory_transactions
    ADD CONSTRAINT fk_inventory_transactions_warehouse
    FOREIGN KEY (warehouse_id) REFERENCES warehouses(id)
    ON DELETE RESTRICT ON UPDATE CASCADE;
