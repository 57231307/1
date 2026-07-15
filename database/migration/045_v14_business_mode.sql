-- ============================================================================
-- v14 批次 431：多业务模式支持
-- 依据：面料行业真实业务调研文档 §6 业务模式 6 种
-- 真实业务：面料行业 6 种典型业务模式贯穿采购/库存/生产/委外/销售/结算全链路
--   1. 坯布经销模式（grey_trading）：采购坯布 → 库存 → 销售坯布
--   2. 成品经销模式（finished_trading）：采购坯布 → 染整加工 → 销售成品
--   3. 染整加工模式（dyeing_processing，客供坯布）：客户提供坯布 → 染整加工 → 收取加工费
--   4. 自织自染模式（self_weave_dye）：采购原料 → 纺纱 → 织布 → 染整 → 销售成品
--   5. 委托加工模式（outsourcing）：自制半成品 → 委外加工 → 收回成品 → 销售
--   6. 来料加工模式（toll_processing）：客户来料 → 加工 → 收取加工费
-- 模式维度：
--   - 物料来源：purchase 采购 / customer_provided 客供 / self_made 自制 / toll 来料
--   - 结算方式：sale_settlement 销售结算 / processing_fee_settlement 加工费结算
--   - 库存类型：grey 坯布 / finished 成品 / both / none
--   - 成本核算方法：standard 标准 / actual 实际 / processing_fee 加工费
--   - 模式分类：trading 贸易 / processing 加工 / integrated 集成
-- 复用现有资产：
--   - sales_order / purchase_order / production_order / outsourcing_order 表：业务单据
--   - business_mode_order_link 表：单据与业务模式关联（含快照防止后续修改影响历史）
-- ============================================================================

-- ============================================================================
-- 1. 业务模式配置主表（business_mode_config）
-- 真实业务：业务模式配置主表，定义 6 种典型业务模式的属性与模块开关
-- ============================================================================
CREATE TABLE IF NOT EXISTS "business_mode_config" (
    "id" SERIAL PRIMARY KEY,
    -- 模式代码（唯一）：grey_trading/finished_trading/dyeing_processing/self_weave_dye/outsourcing/toll_processing
    "mode_code" VARCHAR(50) NOT NULL,
    -- 模式名称
    "mode_name" VARCHAR(100) NOT NULL,
    -- 模式描述
    "description" TEXT,
    -- 是否启用
    "is_active" BOOLEAN NOT NULL DEFAULT TRUE,
    -- 是否默认模式（同时只能有一个默认模式）
    "is_default" BOOLEAN NOT NULL DEFAULT FALSE,
    -- 业务流程链 JSONB 数组（例如 ["purchase","inventory_in","production","inventory_out","sales"]）
    "process_chain" JSONB NOT NULL DEFAULT '[]'::jsonb,
    -- 物料来源：purchase 采购/customer_provided 客供/self_made 自制/toll 来料
    "material_source" VARCHAR(50) NOT NULL,
    -- 结算方式：sale_settlement 销售结算/processing_fee_settlement 加工费结算
    "settlement_method" VARCHAR(50) NOT NULL,
    -- 库存类型：grey 坯布/finished 成品/both/none
    "inventory_type" VARCHAR(50) NOT NULL,
    -- 成本核算方法：standard 标准/actual 实际/processing_fee 加工费
    "cost_method" VARCHAR(50) NOT NULL,
    -- 是否需要采购模块
    "require_purchase" BOOLEAN NOT NULL DEFAULT FALSE,
    -- 是否需要生产模块
    "require_production" BOOLEAN NOT NULL DEFAULT FALSE,
    -- 是否需要委外模块
    "require_outsourcing" BOOLEAN NOT NULL DEFAULT FALSE,
    -- 是否需要销售模块
    "require_sales" BOOLEAN NOT NULL DEFAULT FALSE,
    -- 模式分类：trading 贸易/processing 加工/integrated 集成
    "mode_category" VARCHAR(50) NOT NULL,
    -- 备注
    "remarks" TEXT,
    -- 软删除与审计
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 唯一约束：模式代码
CREATE UNIQUE INDEX IF NOT EXISTS "uk_business_mode_config_code" ON "business_mode_config" ("mode_code") WHERE "is_deleted" = FALSE;
-- 索引：按模式代码查询
CREATE INDEX IF NOT EXISTS "idx_business_mode_config_code" ON "business_mode_config" ("mode_code");
-- 索引：按启用与软删除状态查询
CREATE INDEX IF NOT EXISTS "idx_business_mode_config_active" ON "business_mode_config" ("is_active", "is_deleted");
-- 索引：按模式分类查询
CREATE INDEX IF NOT EXISTS "idx_business_mode_config_category" ON "business_mode_config" ("mode_category");

COMMENT ON TABLE "business_mode_config" IS '业务模式配置主表（v14 批次 431：6 种典型业务模式配置）';
COMMENT ON COLUMN "business_mode_config"."mode_code" IS '模式代码 grey_trading/finished_trading/dyeing_processing/self_weave_dye/outsourcing/toll_processing';
COMMENT ON COLUMN "business_mode_config"."process_chain" IS '业务流程链 JSONB 数组';
COMMENT ON COLUMN "business_mode_config"."material_source" IS '物料来源 purchase 采购/customer_provided 客供/self_made 自制/toll 来料';
COMMENT ON COLUMN "business_mode_config"."settlement_method" IS '结算方式 sale_settlement 销售结算/processing_fee_settlement 加工费结算';
COMMENT ON COLUMN "business_mode_config"."inventory_type" IS '库存类型 grey 坯布/finished 成品/both/none';
COMMENT ON COLUMN "business_mode_config"."cost_method" IS '成本核算方法 standard 标准/actual 实际/processing_fee 加工费';
COMMENT ON COLUMN "business_mode_config"."mode_category" IS '模式分类 trading 贸易/processing 加工/integrated 集成';

-- ============================================================================
-- 2. 业务模式流程节点表（business_mode_flow_step）
-- 真实业务：每个业务模式对应若干流程节点，节点按 step_no 顺序流转
-- ============================================================================
CREATE TABLE IF NOT EXISTS "business_mode_flow_step" (
    "id" SERIAL PRIMARY KEY,
    -- 业务模式 ID（外键 → business_mode_config）
    "mode_id" INTEGER NOT NULL,
    -- 步骤序号（从 1 开始）
    "step_no" INTEGER NOT NULL,
    -- 步骤代码：purchase/inventory_in/production/outsourcing/inventory_out/sales/settlement
    "step_code" VARCHAR(50) NOT NULL,
    -- 步骤名称
    "step_name" VARCHAR(100) NOT NULL,
    -- 模块名：purchase/inventory/production/outsourcing/sales/cost
    "module_name" VARCHAR(50) NOT NULL,
    -- 是否必需
    "is_required" BOOLEAN NOT NULL DEFAULT TRUE,
    -- 步骤描述
    "description" TEXT,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 唯一约束：同一模式内步骤序号唯一
CREATE UNIQUE INDEX IF NOT EXISTS "uk_business_mode_flow_step_no" ON "business_mode_flow_step" ("mode_id", "step_no");
-- 唯一约束：同一模式内步骤代码唯一
CREATE UNIQUE INDEX IF NOT EXISTS "uk_business_mode_flow_step_code" ON "business_mode_flow_step" ("mode_id", "step_code");
-- 索引：按业务模式查询
CREATE INDEX IF NOT EXISTS "idx_business_mode_flow_step_mode" ON "business_mode_flow_step" ("mode_id");

-- 外键约束
ALTER TABLE "business_mode_flow_step" ADD CONSTRAINT "fk_business_mode_flow_step_mode"
    FOREIGN KEY ("mode_id") REFERENCES "business_mode_config" ("id") ON UPDATE CASCADE ON DELETE CASCADE;

COMMENT ON TABLE "business_mode_flow_step" IS '业务模式流程节点表（v14 批次 431：每个业务模式对应的流程节点）';
COMMENT ON COLUMN "business_mode_flow_step"."step_code" IS '步骤代码 purchase/inventory_in/production/outsourcing/inventory_out/sales/settlement';
COMMENT ON COLUMN "business_mode_flow_step"."module_name" IS '模块名 purchase/inventory/production/outsourcing/sales/cost';

-- ============================================================================
-- 3. 业务模式规则表（business_mode_rule）
-- 真实业务：每种业务模式定义一组规则（必需/可选/禁止），用于校验单据流转合法性
-- ============================================================================
CREATE TABLE IF NOT EXISTS "business_mode_rule" (
    "id" SERIAL PRIMARY KEY,
    -- 业务模式 ID（外键 → business_mode_config）
    "mode_id" INTEGER NOT NULL,
    -- 规则代码
    "rule_code" VARCHAR(50) NOT NULL,
    -- 规则名称
    "rule_name" VARCHAR(100) NOT NULL,
    -- 规则类型：required 必需/optional 可选/forbidden 禁止
    "rule_type" VARCHAR(30) NOT NULL,
    -- 模块名
    "module_name" VARCHAR(50) NOT NULL,
    -- 校验逻辑描述（JSONB）
    "validation_logic" JSONB,
    -- 规则描述
    "description" TEXT,
    -- 是否启用
    "is_active" BOOLEAN NOT NULL DEFAULT TRUE,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 唯一约束：同一模式内规则代码唯一
CREATE UNIQUE INDEX IF NOT EXISTS "uk_business_mode_rule_code" ON "business_mode_rule" ("mode_id", "rule_code");
-- 索引：按业务模式查询
CREATE INDEX IF NOT EXISTS "idx_business_mode_rule_mode" ON "business_mode_rule" ("mode_id");
-- 索引：按规则类型查询
CREATE INDEX IF NOT EXISTS "idx_business_mode_rule_type" ON "business_mode_rule" ("rule_type");

-- 外键约束
ALTER TABLE "business_mode_rule" ADD CONSTRAINT "fk_business_mode_rule_mode"
    FOREIGN KEY ("mode_id") REFERENCES "business_mode_config" ("id") ON UPDATE CASCADE ON DELETE CASCADE;

COMMENT ON TABLE "business_mode_rule" IS '业务模式规则表（v14 批次 431：必需/可选/禁止三类规则校验）';
COMMENT ON COLUMN "business_mode_rule"."rule_type" IS '规则类型 required 必需/optional 可选/forbidden 禁止';
COMMENT ON COLUMN "business_mode_rule"."validation_logic" IS '校验逻辑描述（JSONB）';

-- ============================================================================
-- 4. 单据-业务模式关联表（business_mode_order_link）
-- 真实业务：销售订单/采购订单/生产订单/委外订单关联业务模式，含模式快照防止后续修改影响历史单据
-- ============================================================================
CREATE TABLE IF NOT EXISTS "business_mode_order_link" (
    "id" SERIAL PRIMARY KEY,
    -- 业务模式 ID（外键 → business_mode_config，ON DELETE RESTRICT 防止误删模式导致历史单据丢失关联）
    "mode_id" INTEGER NOT NULL,
    -- 单据类型：sales_order/purchase_order/production_order/outsourcing_order
    "document_type" VARCHAR(50) NOT NULL,
    -- 单据 ID
    "document_id" INTEGER NOT NULL,
    -- 单据号
    "document_no" VARCHAR(100) NOT NULL,
    -- 业务模式快照（防止后续修改影响历史单据）
    "mode_snapshot" JSONB,
    -- 备注
    "remarks" TEXT,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 唯一约束：同一单据只能关联一个业务模式
CREATE UNIQUE INDEX IF NOT EXISTS "uk_business_mode_order_link_doc" ON "business_mode_order_link" ("document_type", "document_id");
-- 索引：按业务模式查询
CREATE INDEX IF NOT EXISTS "idx_business_mode_order_link_mode" ON "business_mode_order_link" ("mode_id");
-- 索引：按单据类型+ID 查询
CREATE INDEX IF NOT EXISTS "idx_business_mode_order_link_doc" ON "business_mode_order_link" ("document_type", "document_id");
-- 索引：按单据号查询
CREATE INDEX IF NOT EXISTS "idx_business_mode_order_link_no" ON "business_mode_order_link" ("document_no");

-- 外键约束
ALTER TABLE "business_mode_order_link" ADD CONSTRAINT "fk_business_mode_order_link_mode"
    FOREIGN KEY ("mode_id") REFERENCES "business_mode_config" ("id") ON UPDATE CASCADE ON DELETE RESTRICT;

COMMENT ON TABLE "business_mode_order_link" IS '单据-业务模式关联表（v14 批次 431：单据关联业务模式，含模式快照）';
COMMENT ON COLUMN "business_mode_order_link"."document_type" IS '单据类型 sales_order/purchase_order/production_order/outsourcing_order';
COMMENT ON COLUMN "business_mode_order_link"."mode_snapshot" IS '业务模式快照（防止后续修改影响历史单据）';

-- ============================================================================
-- 5. 预置数据：6 种业务模式配置 + 流程节点
-- ============================================================================

-- 5.1 坯布经销模式（grey_trading）：采购坯布 → 库存 → 销售坯布
INSERT INTO "business_mode_config" (
    "mode_code", "mode_name", "description", "is_active", "is_default",
    "process_chain", "material_source", "settlement_method", "inventory_type",
    "cost_method", "require_purchase", "require_production", "require_outsourcing",
    "require_sales", "mode_category", "remarks"
) VALUES (
    'grey_trading', '坯布经销模式', '采购坯布 → 库存 → 销售坯布', TRUE, FALSE,
    '["purchase","inventory_in","sales"]'::jsonb, 'purchase', 'sale_settlement', 'grey',
    'actual', TRUE, FALSE, FALSE, TRUE, 'trading', '面料行业典型贸易模式：低价采购坯布高价销售'
) ON CONFLICT ("mode_code") WHERE "is_deleted" = FALSE DO NOTHING;

INSERT INTO "business_mode_flow_step" ("mode_id", "step_no", "step_code", "step_name", "module_name", "is_required", "description")
SELECT id, 1, 'purchase', '采购坯布', 'purchase', TRUE, '采购入库坯布'
FROM "business_mode_config" WHERE "mode_code" = 'grey_trading'
ON CONFLICT ("mode_id", "step_no") DO NOTHING;

INSERT INTO "business_mode_flow_step" ("mode_id", "step_no", "step_code", "step_name", "module_name", "is_required", "description")
SELECT id, 2, 'inventory_in', '坯布入库', 'inventory', TRUE, '坯布入库上架'
FROM "business_mode_config" WHERE "mode_code" = 'grey_trading'
ON CONFLICT ("mode_id", "step_no") DO NOTHING;

INSERT INTO "business_mode_flow_step" ("mode_id", "step_no", "step_code", "step_name", "module_name", "is_required", "description")
SELECT id, 3, 'sales', '销售出库', 'sales', TRUE, '销售出库结算'
FROM "business_mode_config" WHERE "mode_code" = 'grey_trading'
ON CONFLICT ("mode_id", "step_no") DO NOTHING;

-- 5.2 成品经销模式（finished_trading）：采购坯布 → 染整加工 → 销售成品
INSERT INTO "business_mode_config" (
    "mode_code", "mode_name", "description", "is_active", "is_default",
    "process_chain", "material_source", "settlement_method", "inventory_type",
    "cost_method", "require_purchase", "require_production", "require_outsourcing",
    "require_sales", "mode_category", "remarks"
) VALUES (
    'finished_trading', '成品经销模式', '采购坯布 → 染整加工 → 销售成品', TRUE, FALSE,
    '["purchase","inventory_in","production","inventory_out","sales"]'::jsonb, 'purchase', 'sale_settlement', 'finished',
    'actual', TRUE, TRUE, FALSE, TRUE, 'integrated', '面料行业典型工贸一体化模式：采购坯布+染整加工销售成品'
) ON CONFLICT ("mode_code") WHERE "is_deleted" = FALSE DO NOTHING;

INSERT INTO "business_mode_flow_step" ("mode_id", "step_no", "step_code", "step_name", "module_name", "is_required", "description")
SELECT id, 1, 'purchase', '采购坯布', 'purchase', TRUE, '采购入库坯布'
FROM "business_mode_config" WHERE "mode_code" = 'finished_trading'
ON CONFLICT ("mode_id", "step_no") DO NOTHING;

INSERT INTO "business_mode_flow_step" ("mode_id", "step_no", "step_code", "step_name", "module_name", "is_required", "description")
SELECT id, 2, 'inventory_in', '坯布入库', 'inventory', TRUE, '坯布入库上架待加工'
FROM "business_mode_config" WHERE "mode_code" = 'finished_trading'
ON CONFLICT ("mode_id", "step_no") DO NOTHING;

INSERT INTO "business_mode_flow_step" ("mode_id", "step_no", "step_code", "step_name", "module_name", "is_required", "description")
SELECT id, 3, 'production', '染整加工', 'production', TRUE, '内部染整加工坯布变成品'
FROM "business_mode_config" WHERE "mode_code" = 'finished_trading'
ON CONFLICT ("mode_id", "step_no") DO NOTHING;

INSERT INTO "business_mode_flow_step" ("mode_id", "step_no", "step_code", "step_name", "module_name", "is_required", "description")
SELECT id, 4, 'inventory_out', '成品入库', 'inventory', TRUE, '成品入库上架'
FROM "business_mode_config" WHERE "mode_code" = 'finished_trading'
ON CONFLICT ("mode_id", "step_no") DO NOTHING;

INSERT INTO "business_mode_flow_step" ("mode_id", "step_no", "step_code", "step_name", "module_name", "is_required", "description")
SELECT id, 5, 'sales', '成品销售', 'sales', TRUE, '销售成品结算'
FROM "business_mode_config" WHERE "mode_code" = 'finished_trading'
ON CONFLICT ("mode_id", "step_no") DO NOTHING;

-- 5.3 染整加工模式（dyeing_processing，客供坯布）：客户提供坯布 → 染整加工 → 收取加工费
INSERT INTO "business_mode_config" (
    "mode_code", "mode_name", "description", "is_active", "is_default",
    "process_chain", "material_source", "settlement_method", "inventory_type",
    "cost_method", "require_purchase", "require_production", "require_outsourcing",
    "require_sales", "mode_category", "remarks"
) VALUES (
    'dyeing_processing', '染整加工模式', '客户提供坯布 → 染整加工 → 收取加工费', TRUE, FALSE,
    '["inventory_in","production","settlement"]'::jsonb, 'customer_provided', 'processing_fee_settlement', 'none',
    'processing_fee', FALSE, TRUE, FALSE, FALSE, 'processing', '面料行业典型加工模式：客供坯布只收加工费'
) ON CONFLICT ("mode_code") WHERE "is_deleted" = FALSE DO NOTHING;

INSERT INTO "business_mode_flow_step" ("mode_id", "step_no", "step_code", "step_name", "module_name", "is_required", "description")
SELECT id, 1, 'inventory_in', '客供坯布入库', 'inventory', TRUE, '客供坯布暂存入库'
FROM "business_mode_config" WHERE "mode_code" = 'dyeing_processing'
ON CONFLICT ("mode_id", "step_no") DO NOTHING;

INSERT INTO "business_mode_flow_step" ("mode_id", "step_no", "step_code", "step_name", "module_name", "is_required", "description")
SELECT id, 2, 'production', '染整加工', 'production', TRUE, '按客户要求染整加工'
FROM "business_mode_config" WHERE "mode_code" = 'dyeing_processing'
ON CONFLICT ("mode_id", "step_no") DO NOTHING;

INSERT INTO "business_mode_flow_step" ("mode_id", "step_no", "step_code", "step_name", "module_name", "is_required", "description")
SELECT id, 3, 'settlement', '加工费结算', 'cost', TRUE, '按加工费结算收取费用'
FROM "business_mode_config" WHERE "mode_code" = 'dyeing_processing'
ON CONFLICT ("mode_id", "step_no") DO NOTHING;

-- 5.4 自织自染模式（self_weave_dye）：采购原料 → 纺纱 → 织布 → 染整 → 销售成品
INSERT INTO "business_mode_config" (
    "mode_code", "mode_name", "description", "is_active", "is_default",
    "process_chain", "material_source", "settlement_method", "inventory_type",
    "cost_method", "require_purchase", "require_production", "require_outsourcing",
    "require_sales", "mode_category", "remarks"
) VALUES (
    'self_weave_dye', '自织自染模式', '采购原料 → 纺纱 → 织布 → 染整 → 销售成品', TRUE, FALSE,
    '["purchase","production","inventory_out","sales"]'::jsonb, 'purchase', 'sale_settlement', 'finished',
    'actual', TRUE, TRUE, FALSE, TRUE, 'integrated', '面料行业全工序一体化模式：从原料采购到成品销售全链路'
) ON CONFLICT ("mode_code") WHERE "is_deleted" = FALSE DO NOTHING;

INSERT INTO "business_mode_flow_step" ("mode_id", "step_no", "step_code", "step_name", "module_name", "is_required", "description")
SELECT id, 1, 'purchase', '采购原料', 'purchase', TRUE, '采购棉纱等原料'
FROM "business_mode_config" WHERE "mode_code" = 'self_weave_dye'
ON CONFLICT ("mode_id", "step_no") DO NOTHING;

INSERT INTO "business_mode_flow_step" ("mode_id", "step_no", "step_code", "step_name", "module_name", "is_required", "description")
SELECT id, 2, 'production', '纺纱织布染整', 'production', TRUE, '内部完成纺纱织布染整全工序'
FROM "business_mode_config" WHERE "mode_code" = 'self_weave_dye'
ON CONFLICT ("mode_id", "step_no") DO NOTHING;

INSERT INTO "business_mode_flow_step" ("mode_id", "step_no", "step_code", "step_name", "module_name", "is_required", "description")
SELECT id, 3, 'inventory_out', '成品入库', 'inventory', TRUE, '成品入库上架'
FROM "business_mode_config" WHERE "mode_code" = 'self_weave_dye'
ON CONFLICT ("mode_id", "step_no") DO NOTHING;

INSERT INTO "business_mode_flow_step" ("mode_id", "step_no", "step_code", "step_name", "module_name", "is_required", "description")
SELECT id, 4, 'sales', '成品销售', 'sales', TRUE, '销售成品结算'
FROM "business_mode_config" WHERE "mode_code" = 'self_weave_dye'
ON CONFLICT ("mode_id", "step_no") DO NOTHING;

-- 5.5 委托加工模式（outsourcing）：自制半成品 → 委外加工 → 收回成品 → 销售
INSERT INTO "business_mode_config" (
    "mode_code", "mode_name", "description", "is_active", "is_default",
    "process_chain", "material_source", "settlement_method", "inventory_type",
    "cost_method", "require_purchase", "require_production", "require_outsourcing",
    "require_sales", "mode_category", "remarks"
) VALUES (
    'outsourcing', '委托加工模式', '自制半成品 → 委外加工 → 收回成品 → 销售', TRUE, FALSE,
    '["production","outsourcing","inventory_out","sales"]'::jsonb, 'self_made', 'sale_settlement', 'finished',
    'actual', FALSE, TRUE, TRUE, TRUE, 'integrated', '面料行业委外加工模式：自制半成品外协加工销售成品'
) ON CONFLICT ("mode_code") WHERE "is_deleted" = FALSE DO NOTHING;

INSERT INTO "business_mode_flow_step" ("mode_id", "step_no", "step_code", "step_name", "module_name", "is_required", "description")
SELECT id, 1, 'production', '自制半成品', 'production', TRUE, '内部生产半成品'
FROM "business_mode_config" WHERE "mode_code" = 'outsourcing'
ON CONFLICT ("mode_id", "step_no") DO NOTHING;

INSERT INTO "business_mode_flow_step" ("mode_id", "step_no", "step_code", "step_name", "module_name", "is_required", "description")
SELECT id, 2, 'outsourcing', '委外加工', 'outsourcing', TRUE, '委外加工厂加工'
FROM "business_mode_config" WHERE "mode_code" = 'outsourcing'
ON CONFLICT ("mode_id", "step_no") DO NOTHING;

INSERT INTO "business_mode_flow_step" ("mode_id", "step_no", "step_code", "step_name", "module_name", "is_required", "description")
SELECT id, 3, 'inventory_out', '成品入库', 'inventory', TRUE, '收回成品入库'
FROM "business_mode_config" WHERE "mode_code" = 'outsourcing'
ON CONFLICT ("mode_id", "step_no") DO NOTHING;

INSERT INTO "business_mode_flow_step" ("mode_id", "step_no", "step_code", "step_name", "module_name", "is_required", "description")
SELECT id, 4, 'sales', '成品销售', 'sales', TRUE, '销售成品结算'
FROM "business_mode_config" WHERE "mode_code" = 'outsourcing'
ON CONFLICT ("mode_id", "step_no") DO NOTHING;

-- 5.6 来料加工模式（toll_processing）：客户来料 → 加工 → 收取加工费
INSERT INTO "business_mode_config" (
    "mode_code", "mode_name", "description", "is_active", "is_default",
    "process_chain", "material_source", "settlement_method", "inventory_type",
    "cost_method", "require_purchase", "require_production", "require_outsourcing",
    "require_sales", "mode_category", "remarks"
) VALUES (
    'toll_processing', '来料加工模式', '客户来料 → 加工 → 收取加工费', TRUE, FALSE,
    '["inventory_in","production","settlement"]'::jsonb, 'toll', 'processing_fee_settlement', 'none',
    'processing_fee', FALSE, TRUE, FALSE, FALSE, 'processing', '面料行业来料加工模式：客户来料只收加工费'
) ON CONFLICT ("mode_code") WHERE "is_deleted" = FALSE DO NOTHING;

INSERT INTO "business_mode_flow_step" ("mode_id", "step_no", "step_code", "step_name", "module_name", "is_required", "description")
SELECT id, 1, 'inventory_in', '来料入库', 'inventory', TRUE, '客户来料暂存入库'
FROM "business_mode_config" WHERE "mode_code" = 'toll_processing'
ON CONFLICT ("mode_id", "step_no") DO NOTHING;

INSERT INTO "business_mode_flow_step" ("mode_id", "step_no", "step_code", "step_name", "module_name", "is_required", "description")
SELECT id, 2, 'production', '加工', 'production', TRUE, '按客户要求加工'
FROM "business_mode_config" WHERE "mode_code" = 'toll_processing'
ON CONFLICT ("mode_id", "step_no") DO NOTHING;

INSERT INTO "business_mode_flow_step" ("mode_id", "step_no", "step_code", "step_name", "module_name", "is_required", "description")
SELECT id, 3, 'settlement', '加工费结算', 'cost', TRUE, '按加工费结算收取费用'
FROM "business_mode_config" WHERE "mode_code" = 'toll_processing'
ON CONFLICT ("mode_id", "step_no") DO NOTHING;
