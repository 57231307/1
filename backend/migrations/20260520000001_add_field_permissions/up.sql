-- 添加字段权限表：field_permissions

CREATE TABLE IF NOT EXISTS "field_permissions" (
    "id" SERIAL PRIMARY KEY,
    "role_id" INTEGER NOT NULL,
    "resource_type" VARCHAR(100) NOT NULL,
    "field_name" VARCHAR(100) NOT NULL,
    "can_read" BOOLEAN NOT NULL DEFAULT TRUE,
    "can_write" BOOLEAN NOT NULL DEFAULT TRUE,
    "mask_strategy" VARCHAR(20) NOT NULL DEFAULT 'NONE',
    "is_enabled" BOOLEAN NOT NULL DEFAULT TRUE,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE ("role_id", "resource_type", "field_name")
);

-- 创建索引
CREATE INDEX IF NOT EXISTS "idx_field_permissions_role_id" ON "field_permissions" ("role_id");
CREATE INDEX IF NOT EXISTS "idx_field_permissions_resource_type" ON "field_permissions" ("resource_type");
CREATE INDEX IF NOT EXISTS "idx_field_permissions_role_resource" ON "field_permissions" ("role_id", "resource_type");

-- 添加注释
COMMENT ON TABLE "field_permissions" IS '字段权限表 - 存储角色对资源字段的读写权限';
COMMENT ON COLUMN "field_permissions"."role_id" IS '角色ID';
COMMENT ON COLUMN "field_permissions"."resource_type" IS '资源类型（表名或业务对象）';
COMMENT ON COLUMN "field_permissions"."field_name" IS '字段名';
COMMENT ON COLUMN "field_permissions"."can_read" IS '是否允许读取';
COMMENT ON COLUMN "field_permissions"."can_write" IS '是否允许写入';
COMMENT ON COLUMN "field_permissions"."mask_strategy" IS '掩码策略：NONE-无掩码, MASK-显示为***, HASH-显示哈希值';
COMMENT ON COLUMN "field_permissions"."is_enabled" IS '是否启用';

-- 添加外键约束
ALTER TABLE "field_permissions" ADD CONSTRAINT "fk_field_permissions_role" FOREIGN KEY ("role_id") REFERENCES "roles" ("id");
