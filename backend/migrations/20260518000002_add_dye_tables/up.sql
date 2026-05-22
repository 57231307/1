-- 创建染色批次和配方表
CREATE TABLE IF NOT EXISTS "dye_batch" (
    "id" SERIAL PRIMARY KEY,
    "batch_no" VARCHAR(50) NOT NULL UNIQUE,
    "color_code" VARCHAR(50) NOT NULL,
    "color_name" VARCHAR(100) NOT NULL,
    "fabric_type" VARCHAR(100),
    "weight_kg" DECIMAL(10,2),
    "status" VARCHAR(20) NOT NULL DEFAULT 'pending',
    "production_date" TIMESTAMPTZ,
    "completion_date" TIMESTAMPTZ,
    "quality_grade" VARCHAR(20),
    "remarks" TEXT,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS "dye_recipe" (
    "id" SERIAL PRIMARY KEY,
    "recipe_name" VARCHAR(100) NOT NULL,
    "color_code" VARCHAR(50) NOT NULL,
    "ingredients" JSONB,
    "instructions" TEXT,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
