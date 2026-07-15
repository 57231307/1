-- v14 复审批次 416：面料行业核心数据模型唯一约束补全
-- 修复问题：D-P0-1（product_colors 缺少 UNIQUE(product_id, color_no)）
--          D-P0-2（inventory_stocks 缺少四维联合唯一索引）
--          D-P1-2（inventory_piece piece_no 全局 UNIQUE 应为 (dye_lot_id, piece_no) 联合唯一）
--          D-P1-1（inventory_piece Rust 模型与 SQL 表字段不同步，补齐 DB 缺失字段）
-- 核心特性：一个面料有多个颜色（面料→颜色→缸号→批号四层级联关系）

-- ==================== 1. product_colors: 同面料同色号唯一约束 ====================
-- 业务语义：一个面料有多个颜色，但同一面料+色号组合唯一
-- 修复 D-P0-1：原只有 color_name 单字段 UNIQUE，缺少 (product_id, color_no) 联合唯一
SELECT safe_add_constraint('product_colors', 'uk_pc_product_color_no', 'UNIQUE (product_id, color_no)');

-- ==================== 2. inventory_stocks: 库存四维联合唯一索引 ====================
-- 业务语义：库存唯一标识 = 仓库 + 产品 + 色号 + 批号 + 缸号
-- 修复 D-P0-2：原只有普通索引，无联合唯一约束，同维度可存在多条库存记录
-- 使用 COALESCE(dye_lot_no, '') 处理 NULL 值（白坯布无缸号时 dye_lot_no 为 NULL）
CREATE UNIQUE INDEX IF NOT EXISTS idx_inv_stock_four_dim_unique
ON inventory_stocks (warehouse_id, product_id, color_no, batch_no, COALESCE(dye_lot_no, ''));

-- ==================== 3. inventory_piece: 匹号唯一约束修正 ====================
-- 业务语义：同一缸号下不能有相同的匹号（匹号唯一约束）
-- 修复 D-P1-2：原 piece_no 全局 UNIQUE，应为 (dye_lot_id, piece_no) 联合唯一

-- 3.1 删除 piece_no 全局唯一约束（PostgreSQL 自动命名为 inventory_piece_piece_no_key）
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'inventory_piece_piece_no_key'
        AND conrelid = 'inventory_piece'::regclass
    ) THEN
        ALTER TABLE inventory_piece DROP CONSTRAINT inventory_piece_piece_no_key;
    END IF;
EXCEPTION WHEN OTHERS THEN
    -- 约束不存在时忽略
END $$;

-- 3.2 添加 (dye_lot_id, piece_no) 联合唯一约束
SELECT safe_add_constraint('inventory_piece', 'uk_ip_dye_lot_piece_no', 'UNIQUE (dye_lot_id, piece_no)');

-- ==================== 4. inventory_piece: 补齐 Rust 模型已有但 DB 缺失的字段 ====================
-- 修复 D-P1-1：Rust 模型有 batch_no/product_id/parent_piece_id/location_id/scan_type/status 字段，
-- 但 SQL 表定义中缺失这些字段，导致 SeaORM 查询时 SELECT 不存在的列会失败

-- 产品 ID（外键，通过 dye_lot_id 可间接关联，但 Rust 模型直接引用 product_id 便于查询）
ALTER TABLE inventory_piece ADD COLUMN IF NOT EXISTS product_id INTEGER;

-- 批号（面料行业批号，与缸号配合使用）
ALTER TABLE inventory_piece ADD COLUMN IF NOT EXISTS batch_no VARCHAR(50) DEFAULT '';

-- 母卷 ID（拆分或剪裁而来的布卷指向原始布卷）
ALTER TABLE inventory_piece ADD COLUMN IF NOT EXISTS parent_piece_id INTEGER;

-- 库位 ID（外键）
ALTER TABLE inventory_piece ADD COLUMN IF NOT EXISTS location_id INTEGER;

-- 扫码类型（SHIP=扫码发货，INVENTORY=扫码盘库）
ALTER TABLE inventory_piece ADD COLUMN IF NOT EXISTS scan_type VARCHAR(20);

-- 库存状态（AVAILABLE/RESERVED/SHIPPED/DEFECT/UNAVAILABLE，与 quality_status/inventory_status 并存）
ALTER TABLE inventory_piece ADD COLUMN IF NOT EXISTS status VARCHAR(20) DEFAULT 'AVAILABLE';

-- 为新增字段创建索引
CREATE INDEX IF NOT EXISTS idx_inventory_piece_product_id ON inventory_piece(product_id);
CREATE INDEX IF NOT EXISTS idx_inventory_piece_batch_no ON inventory_piece(batch_no);
CREATE INDEX IF NOT EXISTS idx_inventory_piece_parent_piece ON inventory_piece(parent_piece_id);
CREATE INDEX IF NOT EXISTS idx_inventory_piece_status ON inventory_piece(status);
