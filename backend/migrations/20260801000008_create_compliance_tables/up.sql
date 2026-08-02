-- V15 P2 B08-16：存货跌价准备表
CREATE TABLE IF NOT EXISTS "inventory_write_down" (
    "id" SERIAL PRIMARY KEY,
    "product_id" INTEGER NOT NULL REFERENCES products(id),
    "write_down_type" VARCHAR(30) NOT NULL,  -- seasonal/sluggish/expired
    "original_cost" DECIMAL(15,2) NOT NULL,
    "net_realizable_value" DECIMAL(15,2) NOT NULL,
    "write_down_amount" DECIMAL(15,2) NOT NULL,
    "reason" TEXT,
    "period" DATE NOT NULL,
    "status" VARCHAR(20) NOT NULL DEFAULT 'draft',  -- draft/confirmed/cancelled
    "created_by" INTEGER NOT NULL,
    "confirmed_by" INTEGER,
    "confirmed_at" TIMESTAMP WITH TIME ZONE,
    "created_at" TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- V15 P2 B08-20：环评文件存档表
CREATE TABLE IF NOT EXISTS "environmental_assessment" (
    "id" SERIAL PRIMARY KEY,
    "doc_type" VARCHAR(30) NOT NULL,  -- eia_report/eia_approval/completion_acceptance
    "doc_name" VARCHAR(200) NOT NULL,
    "doc_url" VARCHAR(500) NOT NULL,
    "approval_date" DATE,
    "approval_authority" VARCHAR(200),
    "remarks" TEXT,
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- V15 P2 B08-25：女职工三期保护记录表
CREATE TABLE IF NOT EXISTS "female_worker_protection" (
    "id" SERIAL PRIMARY KEY,
    "worker_id" INTEGER NOT NULL REFERENCES users(id),
    "protection_type" VARCHAR(30) NOT NULL,  -- pregnancy/maternity/lactation
    "expected_start_date" DATE,
    "expected_end_date" DATE,
    "actual_start_date" DATE,
    "actual_end_date" DATE,
    "status" VARCHAR(20) NOT NULL DEFAULT 'active',  -- active/expired
    "remarks" TEXT,
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- V15 P2 B08-25：特种设备操作证管理表
CREATE TABLE IF NOT EXISTS "operation_certificate" (
    "id" SERIAL PRIMARY KEY,
    "worker_id" INTEGER NOT NULL REFERENCES users(id),
    "certificate_no" VARCHAR(50) NOT NULL UNIQUE,
    "certificate_type" VARCHAR(50) NOT NULL,  -- dye_vat/stenter/dryer/boiler/forklift
    "equipment_name" VARCHAR(200),
    "issue_date" DATE NOT NULL,
    "expiry_date" DATE NOT NULL,
    "issuing_authority" VARCHAR(200),
    "status" VARCHAR(20) NOT NULL DEFAULT 'active',  -- active/expired/revoked
    "remarks" TEXT,
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS "idx_inventory_write_down_product" ON "inventory_write_down"("product_id");
CREATE INDEX IF NOT EXISTS "idx_inventory_write_down_period" ON "inventory_write_down"("period");
CREATE INDEX IF NOT EXISTS "idx_female_worker_protection_worker" ON "female_worker_protection"("worker_id");
CREATE INDEX IF NOT EXISTS "idx_operation_certificate_worker" ON "operation_certificate"("worker_id");
CREATE INDEX IF NOT EXISTS "idx_operation_certificate_expiry" ON "operation_certificate"("expiry_date");