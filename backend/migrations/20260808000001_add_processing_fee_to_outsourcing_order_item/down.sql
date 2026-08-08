-- batch-18 P2-2：回滚 processing_fee 和 freight_fee 字段

ALTER TABLE outsourcing_order_item
DROP COLUMN IF EXISTS processing_fee;

ALTER TABLE outsourcing_order_item
DROP COLUMN IF EXISTS freight_fee;
