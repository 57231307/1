-- v14 批次 424：大货处方与加料处方流程
-- 依据：面料行业真实业务调研文档 §11.2 大货处方（染色配料单）与加料处方（染色补料单）
-- 真实业务流程：
--   大货处方单：扫描流转卡条码 → 依据备布数量 → 加载小样处方/历史大货处方 → 根据浴比/浴量
--              → 填写物料明细 → 计算用量 → 开具大货处方单 → 审核后自动建立生产领用单据
--   加料处方单：扫描流转卡 → 加载已审核大货处方 → 登记加料物料 → 生成加料处方单
--   关键约束：同一工单号只能开一张大货处方单，追加物料须开加料处方单
-- 来源：印染厂技术部生产配料操作规程 / 染厂车间工艺执行手册（2026-07-15 真实调研）

-- ============================================================================
-- 1. 大货处方表（production_recipe）
-- 业务来源：车间技术员扫描流转卡，依据备布数量与浴比开具染色配料单
-- 真实必填字段：recipe_no(单号)、fabric_weight(备布重量)、liquor_ratio(浴比)
-- 关键约束：同一 work_order_id 仅允许一张未删除的大货处方（业务校验在 Service 层实现）
-- ============================================================================
CREATE TABLE IF NOT EXISTS "production_recipe" (
    "id" SERIAL PRIMARY KEY,
    -- 大货处方单号：PR-YYYYMMDDHHMMSS-NNN
    "recipe_no" VARCHAR(64) NOT NULL,
    -- 关联工单/生产订单
    "work_order_id" INTEGER,
    -- 关联缸号
    "dye_batch_id" INTEGER,
    -- 引用的 dye_recipe 处方模板 id（可为空，表示手工录入）
    "source_recipe_id" INTEGER,
    -- 关联化验室复样记录（可为空，表示非复样升级路径）
    "lab_dip_resample_id" INTEGER,
    -- 客户信息
    "customer_id" INTEGER,
    -- 色号
    "color_no" VARCHAR(64),
    -- 布种名称
    "fabric_name" VARCHAR(128),
    -- 规格：纱支/密度/成分
    "fabric_spec" VARCHAR(256),
    -- 门幅 cm
    "fabric_width" DECIMAL(10,2),
    -- 克重 g/m²
    "gram_weight" DECIMAL(10,2),
    -- 备布重量 kg（用量计算依据，真实业务必填）
    "fabric_weight" DECIMAL(12,2) NOT NULL,
    -- 染缸设备编号
    "equipment_no" VARCHAR(64),
    -- 浴比如 1:8（真实业务必填）
    "liquor_ratio" VARCHAR(32) NOT NULL,
    -- 浴量/升（= 布重 × 浴比）
    "bath_volume" DECIMAL(12,2),
    -- 加成系数（小样→大货得色差异修正）
    "adjustment_factor" DECIMAL(5,2) DEFAULT 1.00,
    -- 处方明细 JSON：[{material_code, material_name, concentration, unit, amount, category}]
    -- category: dye(染料) / auxiliary(助剂)
    "recipe_detail" JSONB,
    -- 染料成本合计
    "total_dye_cost" DECIMAL(12,4),
    -- 助剂成本合计
    "total_auxiliary_cost" DECIMAL(12,4),
    -- 状态机：draft → approved → closed → cancelled
    "status" VARCHAR(32) NOT NULL DEFAULT 'draft',
    -- 审核人
    "approved_by" INTEGER,
    "approved_at" TIMESTAMPTZ,
    -- 开单人
    "issued_by" INTEGER,
    -- 打印次数
    "printed_count" INTEGER DEFAULT 0,
    -- 备注
    "remarks" TEXT,
    -- 软删除与审计
    "is_deleted" BOOLEAN NOT NULL DEFAULT false,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 大货处方单号唯一
CREATE UNIQUE INDEX IF NOT EXISTS "idx_production_recipe_no" ON "production_recipe" ("recipe_no");
-- 工单外键索引
CREATE INDEX IF NOT EXISTS "idx_production_recipe_work_order" ON "production_recipe" ("work_order_id");
-- 缸号外键索引
CREATE INDEX IF NOT EXISTS "idx_production_recipe_dye_batch" ON "production_recipe" ("dye_batch_id");
-- 源处方模板外键索引
CREATE INDEX IF NOT EXISTS "idx_production_recipe_source_recipe" ON "production_recipe" ("source_recipe_id");
-- 客户外键索引
CREATE INDEX IF NOT EXISTS "idx_production_recipe_customer" ON "production_recipe" ("customer_id");
-- 色号索引（按色号检索同类处方）
CREATE INDEX IF NOT EXISTS "idx_production_recipe_color_no" ON "production_recipe" ("color_no");
-- 状态索引
CREATE INDEX IF NOT EXISTS "idx_production_recipe_status" ON "production_recipe" ("status");
-- 软删除过滤索引
CREATE INDEX IF NOT EXISTS "idx_production_recipe_not_deleted" ON "production_recipe" ("is_deleted") WHERE "is_deleted" = false;

-- ============================================================================
-- 2. 加料处方表（production_recipe_addition）
-- 业务来源：大货处方审核后，生产过程中出现色差/助剂不足/工艺调整，须开补料单
-- 关键约束：关联的大货处方必须为 approved 状态
-- ============================================================================
CREATE TABLE IF NOT EXISTS "production_recipe_addition" (
    "id" SERIAL PRIMARY KEY,
    -- 加料处方单号：PA-YYYYMMDDHHMMSS-NNN
    "addition_no" VARCHAR(64) NOT NULL,
    -- 关联大货处方（必填）
    "production_recipe_id" INTEGER NOT NULL,
    -- 关联工单
    "work_order_id" INTEGER,
    -- 关联缸号
    "dye_batch_id" INTEGER,
    -- 加料原因：色差/助剂不足/工艺调整
    "addition_reason" VARCHAR(256),
    -- 加料明细 JSON：[{material_code, material_name, amount, unit, category}]
    "addition_detail" JSONB,
    -- 加料成本合计
    "total_cost" DECIMAL(12,4),
    -- 状态机：draft → approved → closed
    "status" VARCHAR(32) NOT NULL DEFAULT 'draft',
    -- 审核人
    "approved_by" INTEGER,
    "approved_at" TIMESTAMPTZ,
    -- 开单人
    "issued_by" INTEGER,
    -- 备注
    "remarks" TEXT,
    -- 软删除与审计
    "is_deleted" BOOLEAN NOT NULL DEFAULT false,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 加料处方单号唯一
CREATE UNIQUE INDEX IF NOT EXISTS "idx_production_recipe_addition_no" ON "production_recipe_addition" ("addition_no");
-- 大货处方外键索引
CREATE INDEX IF NOT EXISTS "idx_production_recipe_addition_recipe" ON "production_recipe_addition" ("production_recipe_id");
-- 工单外键索引
CREATE INDEX IF NOT EXISTS "idx_production_recipe_addition_work_order" ON "production_recipe_addition" ("work_order_id");
-- 缸号外键索引
CREATE INDEX IF NOT EXISTS "idx_production_recipe_addition_dye_batch" ON "production_recipe_addition" ("dye_batch_id");
-- 状态索引
CREATE INDEX IF NOT EXISTS "idx_production_recipe_addition_status" ON "production_recipe_addition" ("status");
-- 软删除过滤索引
CREATE INDEX IF NOT EXISTS "idx_production_recipe_addition_not_deleted" ON "production_recipe_addition" ("is_deleted") WHERE "is_deleted" = false;

-- ============================================================================
-- 3. 外键约束（DO 块幂等）
-- ============================================================================
DO $$
BEGIN
    -- production_recipe.work_order_id → production_orders.id
    IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_prod_recipe_work_order') THEN
        ALTER TABLE "production_recipe" ADD CONSTRAINT "fk_prod_recipe_work_order"
            FOREIGN KEY ("work_order_id") REFERENCES "production_orders" ("id") ON DELETE SET NULL;
    END IF;
    -- production_recipe.dye_batch_id → dye_batch.id
    IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_prod_recipe_dye_batch') THEN
        ALTER TABLE "production_recipe" ADD CONSTRAINT "fk_prod_recipe_dye_batch"
            FOREIGN KEY ("dye_batch_id") REFERENCES "dye_batch" ("id") ON DELETE SET NULL;
    END IF;
    -- production_recipe.source_recipe_id → dye_recipe.id
    IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_prod_recipe_source') THEN
        ALTER TABLE "production_recipe" ADD CONSTRAINT "fk_prod_recipe_source"
            FOREIGN KEY ("source_recipe_id") REFERENCES "dye_recipe" ("id") ON DELETE SET NULL;
    END IF;
    -- production_recipe.customer_id → customers.id
    IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_prod_recipe_customer') THEN
        ALTER TABLE "production_recipe" ADD CONSTRAINT "fk_prod_recipe_customer"
            FOREIGN KEY ("customer_id") REFERENCES "customers" ("id") ON DELETE SET NULL;
    END IF;

    -- production_recipe_addition.production_recipe_id → production_recipe.id
    IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_prod_recipe_addition_recipe') THEN
        ALTER TABLE "production_recipe_addition" ADD CONSTRAINT "fk_prod_recipe_addition_recipe"
            FOREIGN KEY ("production_recipe_id") REFERENCES "production_recipe" ("id") ON DELETE CASCADE;
    END IF;
    -- production_recipe_addition.work_order_id → production_orders.id
    IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_prod_recipe_addition_work_order') THEN
        ALTER TABLE "production_recipe_addition" ADD CONSTRAINT "fk_prod_recipe_addition_work_order"
            FOREIGN KEY ("work_order_id") REFERENCES "production_orders" ("id") ON DELETE SET NULL;
    END IF;
    -- production_recipe_addition.dye_batch_id → dye_batch.id
    IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_prod_recipe_addition_dye_batch') THEN
        ALTER TABLE "production_recipe_addition" ADD CONSTRAINT "fk_prod_recipe_addition_dye_batch"
            FOREIGN KEY ("dye_batch_id") REFERENCES "dye_batch" ("id") ON DELETE SET NULL;
    END IF;
END $$;
