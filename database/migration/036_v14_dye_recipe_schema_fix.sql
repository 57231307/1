-- v14 批次 423A：dye_recipe 表 schema 修复
-- 依据：面料行业真实业务调研文档 §11.1 化验室打样流程 + §13.1 批次 423 规划
-- 问题：dye_recipe 模型有 24 字段，但 DB 初始迁移（m0003_add_dye_tables）仅创建 7 字段，
--       导致 recipe_no/status/temperature/liquor_ratio/auxiliaries/version/parent_recipe_id 等
--       18 个字段在 DB 中不存在，handler 直接通过 SeaORM ActiveModel 写入会运行时报错。
-- 修复：补齐缺失字段，保持与 backend/src/models/dye_recipe.rs 模型完全一致。

-- 1. 补齐 dye_recipe 缺失字段
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "recipe_no" VARCHAR(50);
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(50);
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "formula" TEXT;
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "temperature" DECIMAL(5,2);
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "time_minutes" INTEGER;
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "status" VARCHAR(20) DEFAULT '草稿';
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "is_deleted" BOOLEAN DEFAULT false;
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "color_name" VARCHAR(100);
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "fabric_type" VARCHAR(100);
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "dye_type" VARCHAR(50);
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "chemical_formula" VARCHAR(100);
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "ph_value" DECIMAL(5,2);
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "liquor_ratio" DECIMAL(10,2);
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "auxiliaries" JSONB;
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "version" INTEGER DEFAULT 1;
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "parent_recipe_id" INTEGER;
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "approved_by" INTEGER;
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "approved_at" TIMESTAMPTZ;
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "remarks" TEXT;

-- 1.1 recipe_no 兼容历史记录并强制 NOT NULL（模型定义为 String，DB 必须非空）
-- 历史记录无 recipe_no，回填 LEGACY-{id} 格式后再加 NOT NULL 约束
UPDATE "dye_recipe" SET "recipe_no" = 'LEGACY-' || "id"::TEXT WHERE "recipe_no" IS NULL;
ALTER TABLE "dye_recipe" ALTER COLUMN "recipe_no" SET NOT NULL;

-- 2. recipe_no 唯一索引（允许 NULL，避免历史数据冲突）
CREATE UNIQUE INDEX IF NOT EXISTS "idx_dye_recipe_recipe_no" ON "dye_recipe" ("recipe_no") WHERE "recipe_no" IS NOT NULL;

-- 3. parent_recipe_id 外键索引（版本树溯源）
CREATE INDEX IF NOT EXISTS "idx_dye_recipe_parent" ON "dye_recipe" ("parent_recipe_id");

-- 4. color_code + status 复合索引（按色号查询已审核配方）
CREATE INDEX IF NOT EXISTS "idx_dye_recipe_color_status" ON "dye_recipe" ("color_code", "status");

-- 5. 软删除过滤索引（常用查询 is_deleted=false）
CREATE INDEX IF NOT EXISTS "idx_dye_recipe_not_deleted" ON "dye_recipe" ("is_deleted") WHERE "is_deleted" = false;

-- 6. parent_recipe_id 外键约束（引用自身，版本树）
-- 注：使用 DO 块避免重复添加约束报错
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.table_constraints
        WHERE constraint_name = 'fk_dye_recipe_parent'
    ) THEN
        ALTER TABLE "dye_recipe"
            ADD CONSTRAINT "fk_dye_recipe_parent"
            FOREIGN KEY ("parent_recipe_id") REFERENCES "dye_recipe" ("id")
            ON DELETE SET NULL;
    END IF;
END $$;

-- 7. approved_by 外键约束（引用 user 表）
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.table_constraints
        WHERE constraint_name = 'fk_dye_recipe_approved_by'
    ) THEN
        ALTER TABLE "dye_recipe"
            ADD CONSTRAINT "fk_dye_recipe_approved_by"
            FOREIGN KEY ("approved_by") REFERENCES "user" ("id")
            ON DELETE SET NULL;
    END IF;
END $$;
