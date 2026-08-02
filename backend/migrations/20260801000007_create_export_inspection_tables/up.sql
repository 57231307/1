-- V15 P2 B08-12：出口商检记录表
CREATE TABLE IF NOT EXISTS "export_inspection" (
    "id" SERIAL PRIMARY KEY,
    "inspection_no" VARCHAR(50) NOT NULL UNIQUE,
    "sales_order_id" INTEGER NOT NULL REFERENCES sales_orders(id),
    "delivery_id" INTEGER REFERENCES deliveries(id),
    "product_name" VARCHAR(200) NOT NULL,
    "hs_code" VARCHAR(20) NOT NULL,
    "inspection_type" VARCHAR(20) NOT NULL DEFAULT 'first',  -- first/regular/random
    "inspection_agency" VARCHAR(200) NOT NULL,
    "inspection_date" DATE NOT NULL,
    "result" VARCHAR(20) NOT NULL DEFAULT 'pending',  -- pending/qualified/unqualified
    "report_url" VARCHAR(500),
    "certificate_no" VARCHAR(50),
    "certificate_expiry" DATE,
    "remarks" TEXT,
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- 出口产地证表
CREATE TABLE IF NOT EXISTS "certificate_of_origin" (
    "id" SERIAL PRIMARY KEY,
    "certificate_no" VARCHAR(50) NOT NULL UNIQUE,
    "inspection_id" INTEGER REFERENCES export_inspection(id),
    "product_name" VARCHAR(200) NOT NULL,
    "hs_code" VARCHAR(20) NOT NULL,
    "origin_country" VARCHAR(100) NOT NULL DEFAULT 'China',
    "destination_country" VARCHAR(100) NOT NULL,
    "quantity" DECIMAL(15,2) NOT NULL,
    "unit" VARCHAR(20) NOT NULL,
    "invoice_amount" DECIMAL(15,2),
    "certificate_type" VARCHAR(20) NOT NULL DEFAULT 'general',  -- general/preferential
    "issue_date" DATE NOT NULL,
    "expiry_date" DATE,
    "status" VARCHAR(20) NOT NULL DEFAULT 'active',  -- active/expired/revoked
    "remarks" TEXT,
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS "idx_export_inspection_sales_order" ON "export_inspection"("sales_order_id");
CREATE INDEX IF NOT EXISTS "idx_export_inspection_certificate_no" ON "export_inspection"("certificate_no");
CREATE INDEX IF NOT EXISTS "idx_certificate_of_origin_inspection" ON "certificate_of_origin"("inspection_id");