-- v14 复审批次 417：业务单据明细补全缸号字段
-- 修复问题：D-P1-3/4/5/6 + T-P0-1/2/4
-- 业务语义：所有业务单据（订单/发货/入库/退货）明细必须包含 color_no + dye_lot_no + batch_no
-- 核心特性：一个面料有多个颜色（面料→颜色→缸号→批号四层级联关系）

-- ==================== 1. sales_return_item: 添加缸号/色号/批号字段（D-P1-3） ====================
ALTER TABLE sales_return_item ADD COLUMN IF NOT EXISTS color_no VARCHAR(50) DEFAULT '';
ALTER TABLE sales_return_item ADD COLUMN IF NOT EXISTS dye_lot_no VARCHAR(50);
ALTER TABLE sales_return_item ADD COLUMN IF NOT EXISTS batch_no VARCHAR(50) DEFAULT '';

CREATE INDEX IF NOT EXISTS idx_sales_return_item_color_no ON sales_return_item(color_no);
CREATE INDEX IF NOT EXISTS idx_sales_return_item_dye_lot_no ON sales_return_item(dye_lot_no);
CREATE INDEX IF NOT EXISTS idx_sales_return_item_batch_no ON sales_return_item(batch_no);

-- ==================== 2. purchase_return_item: 添加缸号/色号/批号字段（D-P1-4） ====================
ALTER TABLE purchase_return_item ADD COLUMN IF NOT EXISTS color_no VARCHAR(50) DEFAULT '';
ALTER TABLE purchase_return_item ADD COLUMN IF NOT EXISTS dye_lot_no VARCHAR(50);
ALTER TABLE purchase_return_item ADD COLUMN IF NOT EXISTS batch_no VARCHAR(50) DEFAULT '';

CREATE INDEX IF NOT EXISTS idx_purchase_return_item_color_no ON purchase_return_item(color_no);
CREATE INDEX IF NOT EXISTS idx_purchase_return_item_dye_lot_no ON purchase_return_item(dye_lot_no);
CREATE INDEX IF NOT EXISTS idx_purchase_return_item_batch_no ON purchase_return_item(batch_no);

-- ==================== 3. sales_delivery_item: Rust 模型同步（D-P1-5） ====================
-- SQL 表已包含 dye_lot_id/dye_lot_no/color_no 等字段（001_consolidated_schema.sql:5255），
-- 此处仅补充 Rust 模型缺失的字段，无需 DB 迁移

-- ==================== 4. purchase_order_item: Rust 模型同步（D-P1-6） ====================
-- SQL 表已包含 batch_no/color_code/lot_no 字段（001_consolidated_schema.sql:1498），
-- 此处仅补充 Rust 模型缺失的字段，无需 DB 迁移
-- 注：SQL 表使用旧命名 color_code/lot_no，与项目统一术语 color_no/dye_lot_no 不一致，
-- 但为保持向后兼容，本批次不重命名 DB 字段，仅同步 Rust 模型

-- ==================== 5. inventory_transfer_items: 添加缸号/色号/批号字段（T-P0-1） ====================
ALTER TABLE inventory_transfer_items ADD COLUMN IF NOT EXISTS color_no VARCHAR(50) DEFAULT '';
ALTER TABLE inventory_transfer_items ADD COLUMN IF NOT EXISTS dye_lot_no VARCHAR(50);
ALTER TABLE inventory_transfer_items ADD COLUMN IF NOT EXISTS batch_no VARCHAR(50) DEFAULT '';

CREATE INDEX IF NOT EXISTS idx_transfer_items_color_no ON inventory_transfer_items(color_no);
CREATE INDEX IF NOT EXISTS idx_transfer_items_dye_lot_no ON inventory_transfer_items(dye_lot_no);
CREATE INDEX IF NOT EXISTS idx_transfer_items_batch_no ON inventory_transfer_items(batch_no);

-- ==================== 6. inventory_count_items: 添加缸号/批号字段（T-P0-4） ====================
-- inventory_count_items 通过 stock_id 关联 inventory_stocks，已间接关联缸号，
-- 但为便于盘点差异按缸号/批号筛选，直接在明细表添加字段
ALTER TABLE inventory_count_items ADD COLUMN IF NOT EXISTS color_no VARCHAR(50) DEFAULT '';
ALTER TABLE inventory_count_items ADD COLUMN IF NOT EXISTS dye_lot_no VARCHAR(50);
ALTER TABLE inventory_count_items ADD COLUMN IF NOT EXISTS batch_no VARCHAR(50) DEFAULT '';

CREATE INDEX IF NOT EXISTS idx_inventory_count_items_color_no ON inventory_count_items(color_no);
CREATE INDEX IF NOT EXISTS idx_inventory_count_items_dye_lot_no ON inventory_count_items(dye_lot_no);
CREATE INDEX IF NOT EXISTS idx_inventory_count_items_batch_no ON inventory_count_items(batch_no);
