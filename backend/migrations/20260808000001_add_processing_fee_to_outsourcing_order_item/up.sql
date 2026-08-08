-- batch-18 P2-2：委外加工费按缸号/匹号核算
-- 为 outsourcing_order_item 表添加 processing_fee 和 freight_fee 字段

ALTER TABLE outsourcing_order_item
ADD COLUMN IF NOT EXISTS processing_fee DECIMAL(14, 4) NOT NULL DEFAULT 0;

ALTER TABLE outsourcing_order_item
ADD COLUMN IF NOT EXISTS freight_fee DECIMAL(14, 4) NOT NULL DEFAULT 0;
