-- Migration 026: 修复缺失的表和列
-- 修复以下问题:
-- 1. assignment_histories 表 - TIMESTAMPTZ 改为 TIMESTAMPTZ
-- 2. dye_recipe 表 - 添加缺失的列
-- 3. user_notification_setting 表 - TIMESTAMPTZ 改为 TIMESTAMPTZ
-- 4. audit_logs 表 - 重命名列并添加缺失列
-- 5. omni_audit_logs 表 - 确保存在

-- 1. 修复 assignment_histories 表的时间戳类型
DO $$ 
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'assignment_histories') THEN
        ALTER TABLE "assignment_histories" 
          ALTER COLUMN "created_at" TYPE TIMESTAMPTZ USING "created_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- 2. 修复 dye_recipe 表 - 添加缺失的列
DO $$ 
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'dye_recipe') THEN
        -- 添加缺失的列
        ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "recipe_no" VARCHAR(50) UNIQUE;
        ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(50);
        ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "formula" TEXT;
        ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "color_name" VARCHAR(100);
        ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "fabric_type" VARCHAR(100);
        ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "dye_type" VARCHAR(100);
        ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "chemical_formula" TEXT;
        ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "temperature" DECIMAL(5,2);
        ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "time_minutes" INTEGER;
        ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "ph_value" DECIMAL(5,2);
        ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "liquor_ratio" DECIMAL(10,2);
        ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "status" VARCHAR(20) DEFAULT '草稿';
        ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "is_deleted" BOOLEAN DEFAULT false;
        ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "version" INTEGER DEFAULT 1;
        ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "parent_recipe_id" INTEGER;
        ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "approved_by" INTEGER;
        ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "approved_at" TIMESTAMPTZ;
        ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "remarks" TEXT;
        ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "auxiliaries" JSONB;
        
        -- 迁移旧列数据
        UPDATE "dye_recipe" SET "auxiliaries" = "ingredients" WHERE "auxiliaries" IS NULL AND "ingredients" IS NOT NULL;
        UPDATE "dye_recipe" SET "remarks" = "instructions" WHERE "remarks" IS NULL AND "instructions" IS NOT NULL;
    END IF;
END $$;

-- 3. 修复 user_notification_setting 表的时间戳类型
DO $$ 
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'user_notification_setting') THEN
        ALTER TABLE "user_notification_setting" 
          ALTER COLUMN "created_at" TYPE TIMESTAMPTZ USING "created_at" AT TIME ZONE 'UTC',
          ALTER COLUMN "updated_at" TYPE TIMESTAMPTZ USING "updated_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- 4. 修复 audit_logs 表 - 重命名列并添加缺失列
DO $$ 
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'audit_logs') THEN
        -- 重命名列
        IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'audit_logs' AND column_name = 'table_name') THEN
            ALTER TABLE "audit_logs" RENAME COLUMN "table_name" TO "resource_type";
        END IF;
        
        IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'audit_logs' AND column_name = 'record_id') THEN
            ALTER TABLE "audit_logs" RENAME COLUMN "record_id" TO "resource_id";
        END IF;
        
        -- 添加缺失的列
        ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "ip_address" VARCHAR(50);
    END IF;
END $$;

-- 5. 确保 omni_audit_logs 表存在
CREATE TABLE IF NOT EXISTS "omni_audit_logs" (
    "id" SERIAL PRIMARY KEY,
    "tenant_id" INTEGER NOT NULL DEFAULT 1,
    "user_id" INTEGER,
    "username" VARCHAR(100),
    "module" VARCHAR(100) NOT NULL,
    "action" VARCHAR(50) NOT NULL,
    "resource_type" VARCHAR(100),
    "resource_id" VARCHAR(100),
    "resource_name" VARCHAR(200),
    "description" TEXT,
    "ip_address" VARCHAR(50),
    "user_agent" TEXT,
    "request_method" VARCHAR(10),
    "request_path" VARCHAR(500),
    "request_body" TEXT,
    "response_status" INTEGER,
    "duration_ms" INTEGER,
    "old_value" JSONB,
    "new_value" JSONB,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_omni_audit_logs_tenant_id ON "omni_audit_logs"("tenant_id");
CREATE INDEX IF NOT EXISTS idx_omni_audit_logs_user_id ON "omni_audit_logs"("user_id");
CREATE INDEX IF NOT EXISTS idx_omni_audit_logs_module ON "omni_audit_logs"("module");
CREATE INDEX IF NOT EXISTS idx_omni_audit_logs_action ON "omni_audit_logs"("action");
CREATE INDEX IF NOT EXISTS idx_omni_audit_logs_created_at ON "omni_audit_logs"("created_at");
