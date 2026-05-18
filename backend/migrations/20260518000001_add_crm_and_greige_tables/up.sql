-- 添加缺失的表：greige_fabric
-- 修复 CRM 表结构

-- 创建 greige_fabric 表（坯布管理）
CREATE TABLE IF NOT EXISTS "greige_fabric" (
    "id" SERIAL PRIMARY KEY,
    "fabric_no" VARCHAR(50) NOT NULL UNIQUE,
    "fabric_name" VARCHAR(200) NOT NULL,
    "fabric_type" VARCHAR(50) NOT NULL,
    "color_code" VARCHAR(50),
    "width_cm" DECIMAL(10, 2),
    "weight_kg" DECIMAL(10, 2),
    "length_m" DECIMAL(10, 2),
    "supplier_id" INTEGER,
    "batch_no" VARCHAR(50),
    "warehouse_id" INTEGER,
    "location" VARCHAR(100),
    "status" VARCHAR(20) DEFAULT 'available',
    "quality_grade" VARCHAR(20),
    "purchase_date" DATE,
    "remarks" TEXT,
    "created_by" INTEGER,
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 创建索引
CREATE INDEX IF NOT EXISTS "idx_greige_fabric_fabric_no" ON "greige_fabric" ("fabric_no");
CREATE INDEX IF NOT EXISTS "idx_greige_fabric_supplier_id" ON "greige_fabric" ("supplier_id");
CREATE INDEX IF NOT EXISTS "idx_greige_fabric_warehouse_id" ON "greige_fabric" ("warehouse_id");
CREATE INDEX IF NOT EXISTS "idx_greige_fabric_status" ON "greige_fabric" ("status");

-- 添加注释
COMMENT ON TABLE "greige_fabric" IS '坯布管理表 - 存储原料布匹信息';
COMMENT ON COLUMN "greige_fabric"."fabric_no" IS '坯布编号';
COMMENT ON COLUMN "greige_fabric"."fabric_name" IS '坯布名称';
COMMENT ON COLUMN "greige_fabric"."fabric_type" IS '坯布类型';
COMMENT ON COLUMN "greige_fabric"."color_code" IS '颜色编码';
COMMENT ON COLUMN "greige_fabric"."width_cm" IS '幅宽(cm)';
COMMENT ON COLUMN "greige_fabric"."weight_kg" IS '克重(kg/m²)';
COMMENT ON COLUMN "greige_fabric"."length_m" IS '长度(m)';
COMMENT ON COLUMN "greige_fabric"."supplier_id" IS '供应商ID';
COMMENT ON COLUMN "greige_fabric"."batch_no" IS '批次号';
COMMENT ON COLUMN "greige_fabric"."warehouse_id" IS '仓库ID';
COMMENT ON COLUMN "greige_fabric"."location" IS '库位';
COMMENT ON COLUMN "greige_fabric"."status" IS '状态: available/reserved/consumed/damaged';
COMMENT ON COLUMN "greige_fabric"."quality_grade" IS '质量等级: A/B/C';

-- 添加外键约束
ALTER TABLE "greige_fabric" ADD CONSTRAINT "fk_greige_fabric_supplier" FOREIGN KEY ("supplier_id") REFERENCES "suppliers" ("id");
ALTER TABLE "greige_fabric" ADD CONSTRAINT "fk_greige_fabric_warehouse" FOREIGN KEY ("warehouse_id") REFERENCES "warehouses" ("id");
