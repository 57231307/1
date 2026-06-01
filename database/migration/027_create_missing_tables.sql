-- Migration 027: 创建缺失的表
-- 创建以下表:
-- 1. assignment_histories - CRM分配历史
-- 2. dye_recipe - 染色配方
-- 3. user_notification_setting - 用户通知设置
-- 4. audit_logs - 审计日志
-- 5. field_permissions - 字段权限

-- 1. 创建 assignment_histories 表
CREATE TABLE IF NOT EXISTS "assignment_histories" (
    "id" SERIAL PRIMARY KEY,
    "customer_id" INTEGER NOT NULL,
    "from_owner_id" INTEGER,
    "to_owner_id" INTEGER NOT NULL,
    "assign_type" VARCHAR(50) NOT NULL DEFAULT 'manual',
    "reason" TEXT,
    "assigned_by" INTEGER NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_assignment_histories_customer_id ON "assignment_histories"("customer_id");
CREATE INDEX IF NOT EXISTS idx_assignment_histories_to_owner_id ON "assignment_histories"("to_owner_id");
CREATE INDEX IF NOT EXISTS idx_assignment_histories_created_at ON "assignment_histories"("created_at");

-- 2. 创建 dye_recipe 表
CREATE TABLE IF NOT EXISTS "dye_recipe" (
    "id" SERIAL PRIMARY KEY,
    "recipe_no" VARCHAR(50) UNIQUE,
    "recipe_name" VARCHAR(100) NOT NULL,
    "color_no" VARCHAR(50),
    "formula" TEXT,
    "color_code" VARCHAR(50),
    "color_name" VARCHAR(100),
    "fabric_type" VARCHAR(100),
    "dye_type" VARCHAR(100),
    "chemical_formula" TEXT,
    "temperature" DECIMAL(5,2),
    "time_minutes" INTEGER,
    "ph_value" DECIMAL(5,2),
    "liquor_ratio" DECIMAL(10,2),
    "auxiliaries" JSONB,
    "status" VARCHAR(20) DEFAULT '草稿',
    "is_deleted" BOOLEAN DEFAULT false,
    "version" INTEGER DEFAULT 1,
    "parent_recipe_id" INTEGER,
    "approved_by" INTEGER,
    "approved_at" TIMESTAMPTZ,
    "remarks" TEXT,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_dye_recipe_recipe_no ON "dye_recipe"("recipe_no");
CREATE INDEX IF NOT EXISTS idx_dye_recipe_color_no ON "dye_recipe"("color_no");
CREATE INDEX IF NOT EXISTS idx_dye_recipe_status ON "dye_recipe"("status");

-- 3. 创建 user_notification_setting 表
CREATE TABLE IF NOT EXISTS "user_notification_setting" (
    "id" SERIAL PRIMARY KEY,
    "user_id" INTEGER NOT NULL,
    "setting_key" VARCHAR(100) NOT NULL,
    "setting_value" TEXT,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE("user_id", "setting_key")
);

CREATE INDEX IF NOT EXISTS idx_user_notification_setting_user_id ON "user_notification_setting"("user_id");

-- 4. 创建 audit_logs 表
CREATE TABLE IF NOT EXISTS "audit_logs" (
    "id" SERIAL PRIMARY KEY,
    "user_id" INTEGER,
    "username" VARCHAR(100),
    "action" VARCHAR(50) NOT NULL,
    "resource_type" VARCHAR(100),
    "resource_id" VARCHAR(100),
    "resource_name" VARCHAR(200),
    "description" TEXT,
    "ip_address" VARCHAR(50),
    "user_agent" TEXT,
    "old_value" JSONB,
    "new_value" JSONB,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_audit_logs_user_id ON "audit_logs"("user_id");
CREATE INDEX IF NOT EXISTS idx_audit_logs_action ON "audit_logs"("action");
CREATE INDEX IF NOT EXISTS idx_audit_logs_resource_type ON "audit_logs"("resource_type");
CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at ON "audit_logs"("created_at");

-- 5. 创建 field_permissions 表
CREATE TABLE IF NOT EXISTS "field_permissions" (
    "id" SERIAL PRIMARY KEY,
    "role_id" INTEGER NOT NULL,
    "resource_type" VARCHAR(100) NOT NULL,
    "field_name" VARCHAR(100) NOT NULL,
    "is_visible" BOOLEAN DEFAULT true,
    "is_editable" BOOLEAN DEFAULT true,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE("role_id", "resource_type", "field_name")
);

CREATE INDEX IF NOT EXISTS idx_field_permissions_role_id ON "field_permissions"("role_id");
CREATE INDEX IF NOT EXISTS idx_field_permissions_resource_type ON "field_permissions"("resource_type");
