-- Migration 028: 创建染色批次和坯布表

-- 创建坯布表
CREATE TABLE IF NOT EXISTS "greige_fabric" (
    "id" SERIAL PRIMARY KEY,
    "fabric_no" VARCHAR(50) UNIQUE NOT NULL,
    "fabric_name" VARCHAR(100) NOT NULL,
    "fabric_type" VARCHAR(50),
    "width" DECIMAL(10,2),
    "weight" DECIMAL(10,2),
    "composition" VARCHAR(200),
    "supplier_id" INTEGER,
    "warehouse_id" INTEGER,
    "quantity" DECIMAL(15,2) DEFAULT 0,
    "unit" VARCHAR(20) DEFAULT '米',
    "status" VARCHAR(20) DEFAULT 'active',
    "is_deleted" BOOLEAN DEFAULT false,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_greige_fabric_fabric_no ON "greige_fabric"("fabric_no");
CREATE INDEX IF NOT EXISTS idx_greige_fabric_supplier_id ON "greige_fabric"("supplier_id");
CREATE INDEX IF NOT EXISTS idx_greige_fabric_status ON "greige_fabric"("status");

-- 创建染色批次表
CREATE TABLE IF NOT EXISTS "dye_batch" (
    "id" SERIAL PRIMARY KEY,
    "batch_no" VARCHAR(50) UNIQUE NOT NULL,
    "greige_fabric_id" INTEGER REFERENCES "greige_fabric"("id"),
    "color_no" VARCHAR(50),
    "color_name" VARCHAR(100),
    "dye_recipe_id" INTEGER,
    "planned_quantity" DECIMAL(15,2),
    "actual_quantity" DECIMAL(15,2),
    "unit" VARCHAR(20) DEFAULT '米',
    "status" VARCHAR(20) DEFAULT 'pending',
    "started_at" TIMESTAMPTZ,
    "completed_at" TIMESTAMPTZ,
    "remarks" TEXT,
    "is_deleted" BOOLEAN DEFAULT false,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_dye_batch_batch_no ON "dye_batch"("batch_no");
CREATE INDEX IF NOT EXISTS idx_dye_batch_greige_fabric_id ON "dye_batch"("greige_fabric_id");
CREATE INDEX IF NOT EXISTS idx_dye_batch_status ON "dye_batch"("status");
