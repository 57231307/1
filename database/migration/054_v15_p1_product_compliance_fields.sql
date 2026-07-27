-- ============================================================================
-- Migration 054: V15 P1 产品模型新增合规字段
-- 依据：V15 审计报告 类八 P1（batch-08 维度 8.5 缺陷项 9：面料执行标准登记缺失）
-- 业务背景：《产品质量法》第 27 条要求产品标识需含"产品质量检验合格证明、中文标明的产品名称、
--   厂名、厂址、规格、等级、主要成分、执行标准号"。当前 products 表缺少执行标准号/厂名/厂址/
--   产品等级字段，导致产品标签/吊牌生成无执行标准号，违反《产品质量法》。
-- 修复策略：
--   1. products 表新增 execution_standard/factory_name/factory_address/product_grade 四个字段
--   2. 后端 model 同步扩展
--   3. 应用层 create/update 校验执行标准号格式（GB/T 系列、FZ/T 系列、QB/T 系列）
-- 关联文件：backend/src/models/product.rs
--             backend/src/services/product_service.rs
-- ============================================================================

-- ============================================================================
-- 1. products 表新增合规字段
-- ============================================================================
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "execution_standard" VARCHAR(50);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "factory_name" VARCHAR(200);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "factory_address" VARCHAR(500);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "product_grade" VARCHAR(10);

-- ============================================================================
-- 2. 字段注释
-- ============================================================================
COMMENT ON COLUMN "products"."execution_standard" IS '面料执行标准号（GB/T 系列，如 GB/T 406-2018 棉本色布、GB/T 411-2017 印染棉布、FZ/T 13001-2013 色织棉布）';
COMMENT ON COLUMN "products"."factory_name" IS '生产厂名（《产品质量法》第 27 条要求产品标识含中文标明的厂名）';
COMMENT ON COLUMN "products"."factory_address" IS '生产厂址（《产品质量法》第 27 条要求产品标识含中文标明的厂址）';
COMMENT ON COLUMN "products"."product_grade" IS '产品等级（优等品/一等品/合格品，对应 GB/T 产品质量等级）';

-- ============================================================================
-- 3. 索引（按执行标准号查询、按产品等级筛选）
-- ============================================================================
CREATE INDEX IF NOT EXISTS "idx_products_execution_standard" ON "products" ("execution_standard") WHERE "execution_standard" IS NOT NULL;
CREATE INDEX IF NOT EXISTS "idx_products_product_grade" ON "products" ("product_grade") WHERE "product_grade" IS NOT NULL;
