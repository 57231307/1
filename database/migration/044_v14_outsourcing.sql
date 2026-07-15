-- ============================================================================
-- v14 批次 430：委托加工物资贯通
-- 依据：面料行业真实业务调研文档 §5.4 委托加工物资核算 + §5.5 委外织布场景
--       + §5.7 损耗率标准 + §6.5 委托加工模式
-- 真实业务流程（§5.4 三步分录）：
--   1. 发料：发出自制半成品/坯布给外协加工厂
--      借：委托加工物资 / 贷：自制半成品-胚布
--   2. 加工费：支付染费/加工费（含运费）
--      借：委托加工物资 + 应交税费-进项税额 / 贷：银行存款
--   3. 入库：收回成品入库（合理损耗只影响单位成本，不影响总成本）
--      借：库存商品-成品布 / 贷：委托加工物资
-- 损耗处理规则（§5.4 + §5.7）：
--   - 正常损耗：直接摊入委托加工物资成本，按实际收回数量结转（不单独做分录）
--   - 非正常损耗（丢失/人为损坏）：计入营业外支出/管理费用，不能进成本
-- 行业通用损耗率标准（§5.7）：
--   - 纺纱工序 3%-8%
--   - 织布工序 2%-5%
--   - 印染工序 4%-6%
-- 复用现有资产：
--   - suppliers 表：委外加工厂（supplier_type 扩展取值）
--   - production_orders 表：关联生产订单
--   - dye_batch 表：关联缸号（dye_lot_no 字段）
--   - inventory_stocks / inventory_transactions 表：库存与流水
--   - products / warehouses 表：产品与仓库
-- ============================================================================

-- ============================================================================
-- 1. 委外加工订单主表（outsourcing_order）
-- 真实业务：委外加工订单贯穿发料→加工费→入库三步分录，记录三种凭证号
-- 状态机：draft(草稿) → issued(已发料) → processing(加工中) → received(已收回)
--        → settled(已结算) → closed(已关闭)
--        任意非 closed 状态 → cancelled(已取消)
-- ============================================================================
CREATE TABLE IF NOT EXISTS "outsourcing_order" (
    "id" SERIAL PRIMARY KEY,
    -- 委外订单号（唯一）
    "order_no" VARCHAR(64) NOT NULL,
    -- 委外类型：dyeing(染色) / printing(印花) / weaving(织布) / finishing(后整理) / other(其他)
    "order_type" VARCHAR(32) NOT NULL,
    -- 委外加工厂 ID（外键 → suppliers）
    "supplier_id" INTEGER NOT NULL,
    -- 关联生产订单 ID（外键 → production_orders，可空）
    "production_order_id" INTEGER,
    -- 关联缸号 ID（外键 → dye_batch，可空）
    "dye_batch_id" INTEGER,
    -- 色号（面料行业追溯）
    "color_no" VARCHAR(64),
    -- 缸号（面料行业追溯）
    "dye_lot_no" VARCHAR(64),
    -- 发料日期
    "issue_date" DATE NOT NULL,
    -- 预计收回日期
    "expected_return_date" DATE,
    -- 实际收回日期
    "actual_return_date" DATE,
    -- 发出数量
    "issue_quantity" DECIMAL(14,4) NOT NULL DEFAULT 0,
    -- 发出单位：kg/m/匹
    "issue_unit" VARCHAR(16) NOT NULL DEFAULT 'kg',
    -- 收回数量
    "return_quantity" DECIMAL(14,4) NOT NULL DEFAULT 0,
    -- 损耗数量 = 发出 - 收回
    "loss_quantity" DECIMAL(14,4) NOT NULL DEFAULT 0,
    -- 损耗类型：normal(正常) / abnormal(非正常) / NULL(无损耗)
    "loss_type" VARCHAR(16),
    -- 实际损耗率 = loss_quantity / issue_quantity
    "loss_rate" DECIMAL(8,4),
    -- 标准损耗率（按工序：dyeing=0.05 / weaving=0.035 / spinning=0.055）
    "standard_loss_rate" DECIMAL(8,4),
    -- 发出材料成本
    "material_cost" DECIMAL(14,4) NOT NULL DEFAULT 0,
    -- 加工费
    "processing_fee" DECIMAL(14,4) NOT NULL DEFAULT 0,
    -- 运费
    "freight_fee" DECIMAL(14,4) NOT NULL DEFAULT 0,
    -- 进项税额
    "tax_amount" DECIMAL(14,4) NOT NULL DEFAULT 0,
    -- 非正常损耗金额（计入营业外支出）
    "abnormal_loss_amount" DECIMAL(14,4) NOT NULL DEFAULT 0,
    -- 总成本 = 材料成本 + 加工费 + 运费 - 非正常损耗金额
    "total_cost" DECIMAL(14,4) NOT NULL DEFAULT 0,
    -- 单位成本 = total_cost / return_quantity
    "unit_cost" DECIMAL(14,4) NOT NULL DEFAULT 0,
    -- 状态：draft/issued/processing/received/settled/closed/cancelled
    "status" VARCHAR(32) NOT NULL DEFAULT 'draft',
    -- 发料凭证号
    "voucher_no_issue" VARCHAR(64),
    -- 加工费凭证号
    "voucher_no_fee" VARCHAR(64),
    -- 入库凭证号
    "voucher_no_receipt" VARCHAR(64),
    -- 备注
    "remarks" TEXT,
    -- 软删除与审计
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 唯一约束：委外订单号
CREATE UNIQUE INDEX IF NOT EXISTS "uk_outsourcing_order_no" ON "outsourcing_order" ("order_no") WHERE "is_deleted" = FALSE;
-- 索引：按委外类型查询
CREATE INDEX IF NOT EXISTS "idx_outsourcing_order_type" ON "outsourcing_order" ("order_type") WHERE "is_deleted" = FALSE;
-- 索引：按加工厂查询
CREATE INDEX IF NOT EXISTS "idx_outsourcing_order_supplier" ON "outsourcing_order" ("supplier_id") WHERE "is_deleted" = FALSE;
-- 索引：按生产订单查询
CREATE INDEX IF NOT EXISTS "idx_outsourcing_order_production" ON "outsourcing_order" ("production_order_id") WHERE "is_deleted" = FALSE AND "production_order_id" IS NOT NULL;
-- 索引：按缸号 ID 查询
CREATE INDEX IF NOT EXISTS "idx_outsourcing_order_dyebatch" ON "outsourcing_order" ("dye_batch_id") WHERE "is_deleted" = FALSE AND "dye_batch_id" IS NOT NULL;
-- 索引：按缸号字符串查询（面料追溯）
CREATE INDEX IF NOT EXISTS "idx_outsourcing_order_dyelot" ON "outsourcing_order" ("dye_lot_no") WHERE "is_deleted" = FALSE AND "dye_lot_no" IS NOT NULL;
-- 索引：按发料日期查询
CREATE INDEX IF NOT EXISTS "idx_outsourcing_order_issue_date" ON "outsourcing_order" ("issue_date") WHERE "is_deleted" = FALSE;
-- 索引：按状态查询
CREATE INDEX IF NOT EXISTS "idx_outsourcing_order_status" ON "outsourcing_order" ("status") WHERE "is_deleted" = FALSE;

-- 外键约束
ALTER TABLE "outsourcing_order" ADD CONSTRAINT "fk_outsourcing_order_supplier"
    FOREIGN KEY ("supplier_id") REFERENCES "suppliers" ("id") ON UPDATE CASCADE ON DELETE RESTRICT;
ALTER TABLE "outsourcing_order" ADD CONSTRAINT "fk_outsourcing_order_production"
    FOREIGN KEY ("production_order_id") REFERENCES "production_orders" ("id") ON UPDATE CASCADE ON DELETE SET NULL;
ALTER TABLE "outsourcing_order" ADD CONSTRAINT "fk_outsourcing_order_dyebatch"
    FOREIGN KEY ("dye_batch_id") REFERENCES "dye_batch" ("id") ON UPDATE CASCADE ON DELETE SET NULL;

COMMENT ON TABLE "outsourcing_order" IS '委外加工订单主表（贯穿发料→加工费→入库三步分录）';
COMMENT ON COLUMN "outsourcing_order"."order_type" IS '委外类型 dyeing/printing/weaving/finishing/other';
COMMENT ON COLUMN "outsourcing_order"."loss_type" IS '损耗类型 normal(正常摊入成本) / abnormal(非正常计入营业外支出)';
COMMENT ON COLUMN "outsourcing_order"."standard_loss_rate" IS '标准损耗率（§5.7：dyeing=0.05/weaving=0.035/spinning=0.055）';
COMMENT ON COLUMN "outsourcing_order"."abnormal_loss_amount" IS '非正常损耗金额（超定额损耗，计入营业外支出，不进成本）';
COMMENT ON COLUMN "outsourcing_order"."total_cost" IS '总成本 = 材料成本 + 加工费 + 运费 - 非正常损耗金额';
COMMENT ON COLUMN "outsourcing_order"."status" IS '状态 draft/issued/processing/received/settled/closed/cancelled';

-- ============================================================================
-- 2. 委外加工发料明细表（outsourcing_order_item）
-- 真实业务：每个委外订单可发出多种物料（坯布/棉纱等），按面料四维标识追溯
-- 面料四维标识：product_id + color_no + dye_lot_no + batch_no
-- ============================================================================
CREATE TABLE IF NOT EXISTS "outsourcing_order_item" (
    "id" SERIAL PRIMARY KEY,
    -- 委外订单 ID（外键 → outsourcing_order）
    "outsourcing_order_id" INTEGER NOT NULL,
    -- 发出的物料 ID（外键 → products）
    "product_id" INTEGER NOT NULL,
    -- 色号
    "color_no" VARCHAR(64),
    -- 缸号
    "dye_lot_no" VARCHAR(64),
    -- 匹号（面料四维标识 product_id+color_no+dye_lot_no+batch_no）
    "batch_no" VARCHAR(64),
    -- 发出仓库 ID（外键 → warehouses）
    "warehouse_id" INTEGER,
    -- 发出数量
    "quantity" DECIMAL(14,4) NOT NULL DEFAULT 0,
    -- 单位
    "unit" VARCHAR(16) NOT NULL DEFAULT 'kg',
    -- 单位成本
    "unit_cost" DECIMAL(14,4) NOT NULL DEFAULT 0,
    -- 明细总成本 = quantity × unit_cost
    "total_cost" DECIMAL(14,4) NOT NULL DEFAULT 0,
    -- 关联库存流水 ID（可空）
    "inventory_transaction_id" INTEGER,
    -- 备注
    "remarks" TEXT,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 索引：按委外订单查询
CREATE INDEX IF NOT EXISTS "idx_outsourcing_item_order" ON "outsourcing_order_item" ("outsourcing_order_id");
-- 索引：按物料查询
CREATE INDEX IF NOT EXISTS "idx_outsourcing_item_product" ON "outsourcing_order_item" ("product_id");
-- 索引：按仓库查询
CREATE INDEX IF NOT EXISTS "idx_outsourcing_item_warehouse" ON "outsourcing_order_item" ("warehouse_id") WHERE "warehouse_id" IS NOT NULL;
-- 索引：按缸号查询（面料追溯）
CREATE INDEX IF NOT EXISTS "idx_outsourcing_item_dyelot" ON "outsourcing_order_item" ("dye_lot_no") WHERE "dye_lot_no" IS NOT NULL;

-- 外键约束
ALTER TABLE "outsourcing_order_item" ADD CONSTRAINT "fk_outsourcing_item_order"
    FOREIGN KEY ("outsourcing_order_id") REFERENCES "outsourcing_order" ("id") ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE "outsourcing_order_item" ADD CONSTRAINT "fk_outsourcing_item_product"
    FOREIGN KEY ("product_id") REFERENCES "products" ("id") ON UPDATE CASCADE ON DELETE RESTRICT;
ALTER TABLE "outsourcing_order_item" ADD CONSTRAINT "fk_outsourcing_item_warehouse"
    FOREIGN KEY ("warehouse_id") REFERENCES "warehouses" ("id") ON UPDATE CASCADE ON DELETE SET NULL;

COMMENT ON TABLE "outsourcing_order_item" IS '委外加工发料明细表（按面料四维标识追溯）';
COMMENT ON COLUMN "outsourcing_order_item"."batch_no" IS '匹号（面料四维标识 product_id+color_no+dye_lot_no+batch_no）';

-- ============================================================================
-- 3. 委外收回入库单（outsourcing_receipt）
-- 真实业务：委外加工完成后收回成品入库，含损耗分类与质量等级
-- 状态机：draft(草稿) → confirmed(已确认) → cancelled(已取消)
-- ============================================================================
CREATE TABLE IF NOT EXISTS "outsourcing_receipt" (
    "id" SERIAL PRIMARY KEY,
    -- 收回单号（唯一）
    "receipt_no" VARCHAR(64) NOT NULL,
    -- 委外订单 ID（外键 → outsourcing_order）
    "outsourcing_order_id" INTEGER NOT NULL,
    -- 收回日期
    "receipt_date" DATE NOT NULL,
    -- 收回的成品 ID（外键 → products）
    "product_id" INTEGER NOT NULL,
    -- 色号
    "color_no" VARCHAR(64),
    -- 缸号
    "dye_lot_no" VARCHAR(64),
    -- 匹号
    "batch_no" VARCHAR(64),
    -- 入库仓库 ID（外键 → warehouses）
    "warehouse_id" INTEGER,
    -- 收回数量
    "return_quantity" DECIMAL(14,4) NOT NULL DEFAULT 0,
    -- 损耗数量
    "loss_quantity" DECIMAL(14,4) NOT NULL DEFAULT 0,
    -- 损耗类型：normal/abnormal/NULL
    "loss_type" VARCHAR(16),
    -- 损耗率
    "loss_rate" DECIMAL(8,4),
    -- 是否正常损耗
    "is_loss_normal" BOOLEAN NOT NULL DEFAULT TRUE,
    -- 单位成本 = total_cost / return_quantity
    "unit_cost" DECIMAL(14,4) NOT NULL DEFAULT 0,
    -- 入库总成本
    "total_cost" DECIMAL(14,4) NOT NULL DEFAULT 0,
    -- 非正常损耗金额
    "abnormal_loss_amount" DECIMAL(14,4) NOT NULL DEFAULT 0,
    -- 质量状态：pending(待检) / passed(合格) / failed(不合格)
    "quality_status" VARCHAR(16),
    -- 等级：A/B/C
    "grade" VARCHAR(8),
    -- 关联库存流水 ID
    "inventory_transaction_id" INTEGER,
    -- 状态：draft/confirmed/cancelled
    "status" VARCHAR(32) NOT NULL DEFAULT 'draft',
    -- 备注
    "remarks" TEXT,
    -- 软删除与审计
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 唯一约束：收回单号
CREATE UNIQUE INDEX IF NOT EXISTS "uk_outsourcing_receipt_no" ON "outsourcing_receipt" ("receipt_no") WHERE "is_deleted" = FALSE;
-- 索引：按委外订单查询
CREATE INDEX IF NOT EXISTS "idx_outsourcing_receipt_order" ON "outsourcing_receipt" ("outsourcing_order_id");
-- 索引：按收回日期查询
CREATE INDEX IF NOT EXISTS "idx_outsourcing_receipt_date" ON "outsourcing_receipt" ("receipt_date");
-- 索引：按成品查询
CREATE INDEX IF NOT EXISTS "idx_outsourcing_receipt_product" ON "outsourcing_receipt" ("product_id");
-- 索引：按入库仓库查询
CREATE INDEX IF NOT EXISTS "idx_outsourcing_receipt_warehouse" ON "outsourcing_receipt" ("warehouse_id") WHERE "warehouse_id" IS NOT NULL;
-- 索引：按缸号查询（面料追溯）
CREATE INDEX IF NOT EXISTS "idx_outsourcing_receipt_dyelot" ON "outsourcing_receipt" ("dye_lot_no") WHERE "dye_lot_no" IS NOT NULL;
-- 索引：按状态查询
CREATE INDEX IF NOT EXISTS "idx_outsourcing_receipt_status" ON "outsourcing_receipt" ("status") WHERE "is_deleted" = FALSE;
-- 索引：按质量状态查询
CREATE INDEX IF NOT EXISTS "idx_outsourcing_receipt_quality" ON "outsourcing_receipt" ("quality_status") WHERE "quality_status" IS NOT NULL;

-- 外键约束
ALTER TABLE "outsourcing_receipt" ADD CONSTRAINT "fk_outsourcing_receipt_order"
    FOREIGN KEY ("outsourcing_order_id") REFERENCES "outsourcing_order" ("id") ON UPDATE CASCADE ON DELETE RESTRICT;
ALTER TABLE "outsourcing_receipt" ADD CONSTRAINT "fk_outsourcing_receipt_product"
    FOREIGN KEY ("product_id") REFERENCES "products" ("id") ON UPDATE CASCADE ON DELETE RESTRICT;
ALTER TABLE "outsourcing_receipt" ADD CONSTRAINT "fk_outsourcing_receipt_warehouse"
    FOREIGN KEY ("warehouse_id") REFERENCES "warehouses" ("id") ON UPDATE CASCADE ON DELETE SET NULL;

COMMENT ON TABLE "outsourcing_receipt" IS '委外收回入库单（含损耗分类与质量等级）';
COMMENT ON COLUMN "outsourcing_receipt"."loss_type" IS '损耗类型 normal(正常摊入成本) / abnormal(非正常计入营业外支出)';
COMMENT ON COLUMN "outsourcing_receipt"."quality_status" IS '质量状态 pending/passed/failed';
COMMENT ON COLUMN "outsourcing_receipt"."grade" IS '等级 A/B/C';

-- ============================================================================
-- 4. 委外加工会计分录凭证（outsourcing_voucher）
-- 真实业务：§5.4 委托加工物资核算三步分录
--   - issue(发料)：借 委托加工物资 / 贷 自制半成品-胚布
--   - fee(加工费)：借 委托加工物资 + 应交税费-进项税额 / 贷 银行存款
--   - receipt(入库)：借 库存商品-成品布 / 贷 委托加工物资
--   - loss(损耗处理)：借 营业外支出 / 贷 委托加工物资（非正常损耗单独追责）
-- ============================================================================
CREATE TABLE IF NOT EXISTS "outsourcing_voucher" (
    "id" SERIAL PRIMARY KEY,
    -- 凭证号（唯一）
    "voucher_no" VARCHAR(64) NOT NULL,
    -- 委外订单 ID（外键 → outsourcing_order）
    "outsourcing_order_id" INTEGER NOT NULL,
    -- 凭证类型：issue(发料) / fee(加工费) / receipt(入库) / loss(损耗处理)
    "voucher_type" VARCHAR(32) NOT NULL,
    -- 借方科目（如 委托加工物资 / 库存商品-成品布 / 应交税费-进项税额 / 营业外支出）
    "debit_account" VARCHAR(128) NOT NULL,
    -- 贷方科目（如 自制半成品-胚布 / 库存商品-棉纱 / 委托加工物资 / 银行存款）
    "credit_account" VARCHAR(128) NOT NULL,
    -- 金额
    "amount" DECIMAL(14,4) NOT NULL DEFAULT 0,
    -- 税额（仅加工费凭证有）
    "tax_amount" DECIMAL(14,4) NOT NULL DEFAULT 0,
    -- 凭证日期
    "voucher_date" DATE NOT NULL,
    -- 是否已过账
    "is_posted" BOOLEAN NOT NULL DEFAULT FALSE,
    -- 过账时间
    "posted_at" TIMESTAMPTZ,
    -- 备注
    "remarks" TEXT,
    -- 审计
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 唯一约束：凭证号
CREATE UNIQUE INDEX IF NOT EXISTS "uk_outsourcing_voucher_no" ON "outsourcing_voucher" ("voucher_no");
-- 索引：按委外订单查询
CREATE INDEX IF NOT EXISTS "idx_outsourcing_voucher_order" ON "outsourcing_voucher" ("outsourcing_order_id");
-- 索引：按凭证类型查询
CREATE INDEX IF NOT EXISTS "idx_outsourcing_voucher_type" ON "outsourcing_voucher" ("voucher_type");
-- 索引：按凭证日期查询
CREATE INDEX IF NOT EXISTS "idx_outsourcing_voucher_date" ON "outsourcing_voucher" ("voucher_date");
-- 索引：按过账状态查询
CREATE INDEX IF NOT EXISTS "idx_outsourcing_voucher_posted" ON "outsourcing_voucher" ("is_posted");

-- 外键约束
ALTER TABLE "outsourcing_voucher" ADD CONSTRAINT "fk_outsourcing_voucher_order"
    FOREIGN KEY ("outsourcing_order_id") REFERENCES "outsourcing_order" ("id") ON UPDATE CASCADE ON DELETE CASCADE;

COMMENT ON TABLE "outsourcing_voucher" IS '委外加工会计分录凭证（§5.4 三步分录：发料/加工费/入库/损耗）';
COMMENT ON COLUMN "outsourcing_voucher"."voucher_type" IS '凭证类型 issue(发料)/fee(加工费)/receipt(入库)/loss(损耗处理)';
COMMENT ON COLUMN "outsourcing_voucher"."debit_account" IS '借方科目（委托加工物资/库存商品-成品布/应交税费-进项税额/营业外支出）';
COMMENT ON COLUMN "outsourcing_voucher"."credit_account" IS '贷方科目（自制半成品-胚布/库存商品-棉纱/委托加工物资/银行存款）';
