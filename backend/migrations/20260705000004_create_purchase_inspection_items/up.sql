-- 批次 131 v9 复审 P0 修复：采购质检明细表
-- 原 purchase_inspection_handler 4 个明细 CRUD 端点全部占位：
--   - list_inspection_items 返回硬编码空列表 {items: [], total: 0}
--   - create_inspection_item 只记录日志，不落库
--   - update_inspection_item 只记录日志，不更新
--   - delete_inspection_item 只记录日志，不删除
-- 现新增 purchase_inspection_items 表存储质检明细（按产品维度记录合格/不合格数量），
-- handler 在操作前调用真实 service 方法落库。

CREATE TABLE IF NOT EXISTS "purchase_inspection_items" (
    "id" SERIAL PRIMARY KEY,
    "inspection_id" INTEGER NOT NULL,
    "product_id" INTEGER NOT NULL,
    "item_name" VARCHAR(100) NOT NULL,
    "qualified_quantity" DECIMAL(12, 2) NOT NULL DEFAULT 0,
    "unqualified_quantity" DECIMAL(12, 2) NOT NULL DEFAULT 0,
    "remark" VARCHAR(500),
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "purchase_inspection_items" IS '采购质检明细表（批次 131 v9 复审 P0 修复：替代 4 个明细 CRUD 端点占位）';
COMMENT ON COLUMN "purchase_inspection_items"."inspection_id" IS '采购质检单 ID（外键 purchase_inspection.id）';
COMMENT ON COLUMN "purchase_inspection_items"."product_id" IS '产品 ID（外键 products.id）';
COMMENT ON COLUMN "purchase_inspection_items"."item_name" IS '检验项目名称';
COMMENT ON COLUMN "purchase_inspection_items"."qualified_quantity" IS '合格数量';
COMMENT ON COLUMN "purchase_inspection_items"."unqualified_quantity" IS '不合格数量';
COMMENT ON COLUMN "purchase_inspection_items"."remark" IS '备注';

-- 按质检单 ID 查询明细列表（list_inspection_items 主查询路径）
CREATE INDEX IF NOT EXISTS "idx_purchase_inspection_items_inspection_id" ON "purchase_inspection_items" ("inspection_id");
-- 按产品 ID 查询（后续可能支持按产品维度统计不合格率）
CREATE INDEX IF NOT EXISTS "idx_purchase_inspection_items_product_id" ON "purchase_inspection_items" ("product_id");
