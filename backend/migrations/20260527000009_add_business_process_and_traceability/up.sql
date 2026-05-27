-- 批次9：业务流程与追溯
-- 创建时间: 2026-05-27
-- 描述: 创建审批流程、业务追溯、批次追溯和CRM相关表

-- ============================================
-- 审批流程（4张表）
-- ============================================

-- 1. 审批模板表
CREATE TABLE IF NOT EXISTS "approval_templates" (
    "id" SERIAL PRIMARY KEY,
    "name" VARCHAR(100) NOT NULL,
    "resource_type" VARCHAR(100) NOT NULL UNIQUE,
    "description" TEXT,
    "is_active" BOOLEAN NOT NULL DEFAULT true,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "approval_templates" IS '审批模板表';

-- 2. 审批节点表
CREATE TABLE IF NOT EXISTS "approval_nodes" (
    "id" SERIAL PRIMARY KEY,
    "template_id" INTEGER NOT NULL,
    "step_order" INTEGER NOT NULL,
    "approver_role_id" INTEGER,
    "approver_user_id" INTEGER,
    "condition_expr" JSONB,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_approval_nodes_template" FOREIGN KEY ("template_id") REFERENCES "approval_templates" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_approval_nodes_template" ON "approval_nodes" ("template_id");
COMMENT ON TABLE "approval_nodes" IS '审批节点表';

-- 3. 审批实例表
CREATE TABLE IF NOT EXISTS "approval_instances" (
    "id" SERIAL PRIMARY KEY,
    "template_id" INTEGER NOT NULL,
    "resource_id" INTEGER NOT NULL,
    "status" VARCHAR(20) NOT NULL,
    "current_step_order" INTEGER NOT NULL,
    "applicant_id" INTEGER NOT NULL,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_approval_instances_template" FOREIGN KEY ("template_id") REFERENCES "approval_templates" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_approval_instances_template" ON "approval_instances" ("template_id");
CREATE INDEX IF NOT EXISTS "idx_approval_instances_resource" ON "approval_instances" ("resource_id");
COMMENT ON TABLE "approval_instances" IS '审批实例表';

-- 4. 审批日志表
CREATE TABLE IF NOT EXISTS "approval_logs" (
    "id" SERIAL PRIMARY KEY,
    "instance_id" INTEGER NOT NULL,
    "node_id" INTEGER,
    "approver_id" INTEGER NOT NULL,
    "action" VARCHAR(20) NOT NULL,
    "comments" TEXT,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_approval_logs_instance" FOREIGN KEY ("instance_id") REFERENCES "approval_instances" ("id"),
    CONSTRAINT "fk_approval_logs_node" FOREIGN KEY ("node_id") REFERENCES "approval_nodes" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_approval_logs_instance" ON "approval_logs" ("instance_id");
COMMENT ON TABLE "approval_logs" IS '审批日志表';

-- ============================================
-- 业务追溯（5张表 + 1视图）
-- ============================================

-- 5. 业务追溯表
CREATE TABLE IF NOT EXISTS "business_traces" (
    "id" SERIAL PRIMARY KEY,
    "batch_no" VARCHAR(50) NOT NULL,
    "product_id" INTEGER NOT NULL,
    "warehouse_id" INTEGER,
    "current_stage" VARCHAR(50) NOT NULL,
    "quantity" DECIMAL(12, 2) NOT NULL,
    "unit" VARCHAR(20) NOT NULL,
    "remarks" TEXT,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_business_traces_batch" ON "business_traces" ("batch_no");
CREATE INDEX IF NOT EXISTS "idx_business_traces_product" ON "business_traces" ("product_id");
COMMENT ON TABLE "business_traces" IS '业务追溯表';

-- 6. 业务追溯链表
CREATE TABLE IF NOT EXISTS "business_trace_chain" (
    "id" SERIAL PRIMARY KEY,
    "trace_chain_id" VARCHAR(100) NOT NULL,
    "five_dimension_id" VARCHAR(100) NOT NULL,
    "product_id" INTEGER NOT NULL,
    "batch_no" VARCHAR(50) NOT NULL,
    "color_no" VARCHAR(50) NOT NULL,
    "dye_lot_no" VARCHAR(50),
    "grade" VARCHAR(20) NOT NULL,
    "current_stage" VARCHAR(50) NOT NULL,
    "current_bill_type" VARCHAR(50) NOT NULL,
    "current_bill_no" VARCHAR(50) NOT NULL,
    "current_bill_id" INTEGER NOT NULL,
    "previous_trace_id" INTEGER,
    "next_trace_id" INTEGER,
    "quantity_meters" DECIMAL(12, 2) NOT NULL,
    "quantity_kg" DECIMAL(12, 2) NOT NULL,
    "warehouse_id" INTEGER NOT NULL,
    "supplier_id" INTEGER,
    "customer_id" INTEGER,
    "workshop_id" INTEGER,
    "trace_status" VARCHAR(20) NOT NULL,
    "remarks" TEXT,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "created_by" INTEGER
);

CREATE INDEX IF NOT EXISTS "idx_business_trace_chain_chain_id" ON "business_trace_chain" ("trace_chain_id");
CREATE INDEX IF NOT EXISTS "idx_business_trace_chain_batch" ON "business_trace_chain" ("batch_no");
CREATE INDEX IF NOT EXISTS "idx_business_trace_chain_product" ON "business_trace_chain" ("product_id");
COMMENT ON TABLE "business_trace_chain" IS '业务追溯链表';

-- 7. 业务追溯快照表
CREATE TABLE IF NOT EXISTS "business_trace_snapshot" (
    "id" SERIAL PRIMARY KEY,
    "trace_chain_id" VARCHAR(100) NOT NULL,
    "five_dimension_id" VARCHAR(100) NOT NULL,
    "product_id" INTEGER NOT NULL,
    "batch_no" VARCHAR(50) NOT NULL,
    "color_no" VARCHAR(50) NOT NULL,
    "grade" VARCHAR(20) NOT NULL,
    "current_stage" VARCHAR(50) NOT NULL,
    "warehouse_id" INTEGER NOT NULL,
    "current_quantity_meters" DECIMAL(12, 2) NOT NULL,
    "current_quantity_kg" DECIMAL(12, 2) NOT NULL,
    "supplier_name" VARCHAR(200),
    "customer_name" VARCHAR(200),
    "trace_path" JSONB NOT NULL,
    "snapshot_time" TIMESTAMP NOT NULL
);

CREATE INDEX IF NOT EXISTS "idx_business_trace_snapshot_chain_id" ON "business_trace_snapshot" ("trace_chain_id");
COMMENT ON TABLE "business_trace_snapshot" IS '业务追溯快照表';

-- 8. 业务追溯辅助关联表
CREATE TABLE IF NOT EXISTS "business_trace_assist_links" (
    "id" SERIAL PRIMARY KEY,
    "trace_id" INTEGER NOT NULL,
    "assist_type" VARCHAR(50) NOT NULL,
    "assist_id" INTEGER NOT NULL,
    "assist_code" VARCHAR(50) NOT NULL,
    "assist_name" VARCHAR(100) NOT NULL,
    "remarks" TEXT,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_business_trace_assist_links_trace" ON "business_trace_assist_links" ("trace_id");
COMMENT ON TABLE "business_trace_assist_links" IS '业务追溯辅助关联表';

-- 9. 业务追溯视图
CREATE OR REPLACE VIEW "v_business_trace_view" AS
SELECT 
    btc.id,
    btc.trace_chain_id,
    btc.five_dimension_id,
    btc.product_id,
    btc.batch_no,
    btc.color_no,
    btc.dye_lot_no,
    btc.grade,
    btc.current_stage,
    btc.current_bill_type,
    btc.current_bill_no,
    btc.current_bill_id,
    btc.quantity_meters,
    btc.quantity_kg,
    btc.warehouse_id,
    btc.supplier_id,
    btc.customer_id,
    btc.workshop_id,
    btc.trace_status,
    btc.created_at,
    p.name as product_name,
    w.name as warehouse_name,
    s.name as supplier_name,
    c.name as customer_name
FROM business_trace_chain btc
LEFT JOIN products p ON btc.product_id = p.id
LEFT JOIN warehouses w ON btc.warehouse_id = w.id
LEFT JOIN suppliers s ON btc.supplier_id = s.id
LEFT JOIN customers c ON btc.customer_id = c.id;

COMMENT ON VIEW "v_business_trace_view" IS '业务追溯视图';

-- ============================================
-- 批次追溯（3张表）
-- ============================================

-- 10. 批次染色批次表
CREATE TABLE IF NOT EXISTS "batch_dye_lot" (
    "id" SERIAL PRIMARY KEY,
    "batch_no" VARCHAR(50) NOT NULL UNIQUE,
    "product_id" INTEGER NOT NULL,
    "color_id" INTEGER,
    "dye_lot_no" VARCHAR(50) NOT NULL,
    "dye_date" DATE NOT NULL,
    "quantity" DECIMAL(12, 2) NOT NULL,
    "color_code" VARCHAR(50),
    "status" VARCHAR(20) NOT NULL DEFAULT 'ACTIVE',
    "remarks" TEXT,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_batch_dye_lot_product" FOREIGN KEY ("product_id") REFERENCES "products" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_batch_dye_lot_product" ON "batch_dye_lot" ("product_id");
COMMENT ON TABLE "batch_dye_lot" IS '批次染色批次表';

-- 11. 批次追溯日志表
CREATE TABLE IF NOT EXISTS "batch_trace_log" (
    "id" SERIAL PRIMARY KEY,
    "batch_no" VARCHAR(50) NOT NULL,
    "operation_type" VARCHAR(20) NOT NULL,
    "source_type" VARCHAR(50),
    "source_id" INTEGER,
    "source_no" VARCHAR(50),
    "quantity" DECIMAL(12, 2),
    "quantity_before" DECIMAL(12, 2),
    "quantity_after" DECIMAL(12, 2),
    "remarks" TEXT,
    "operated_by" INTEGER,
    "operated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_batch_trace_log_batch" ON "batch_trace_log" ("batch_no");
COMMENT ON TABLE "batch_trace_log" IS '批次追溯日志表';

-- 12. 缸号映射表
CREATE TABLE IF NOT EXISTS "dye_lot_mapping" (
    "id" SERIAL PRIMARY KEY,
    "dye_batch_id" INTEGER NOT NULL,
    "lot_no" VARCHAR(50) NOT NULL,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_dye_lot_mapping_batch" ON "dye_lot_mapping" ("dye_batch_id");
COMMENT ON TABLE "dye_lot_mapping" IS '缸号映射表';

-- ============================================
-- CRM（3张表）
-- ============================================

-- 13. CRM线索表
CREATE TABLE IF NOT EXISTS "crm_lead" (
    "id" SERIAL PRIMARY KEY,
    "lead_no" VARCHAR(50) NOT NULL UNIQUE,
    "lead_source" VARCHAR(50) NOT NULL,
    "lead_status" VARCHAR(20),
    "company_name" VARCHAR(200),
    "contact_name" VARCHAR(100) NOT NULL,
    "contact_title" VARCHAR(50),
    "mobile_phone" VARCHAR(20),
    "tel_phone" VARCHAR(20),
    "email" VARCHAR(100),
    "wechat" VARCHAR(50),
    "qq" VARCHAR(30),
    "address" TEXT,
    "product_interest" TEXT,
    "estimated_quantity" DECIMAL(12, 2),
    "estimated_amount" DECIMAL(15, 2),
    "expected_delivery_date" DATE,
    "requirement_desc" TEXT,
    "owner_id" INTEGER NOT NULL,
    "owner_name" VARCHAR(100) NOT NULL,
    "last_follow_up_date" DATE,
    "next_follow_up_date" DATE,
    "follow_up_plan" TEXT,
    "converted_at" TIMESTAMP,
    "converted_customer_id" INTEGER,
    "converted_opportunity_id" INTEGER,
    "lost_reason" TEXT,
    "priority" VARCHAR(20),
    "rating" INTEGER,
    "tags" TEXT,
    "created_at" TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    "created_by" INTEGER,
    "updated_by" INTEGER,
    CONSTRAINT "fk_crm_lead_customer" FOREIGN KEY ("converted_customer_id") REFERENCES "customers" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_crm_lead_owner" ON "crm_lead" ("owner_id");
CREATE INDEX IF NOT EXISTS "idx_crm_lead_status" ON "crm_lead" ("lead_status");
COMMENT ON TABLE "crm_lead" IS 'CRM线索表';

-- 14. CRM商机表
CREATE TABLE IF NOT EXISTS "crm_opportunity" (
    "id" SERIAL PRIMARY KEY,
    "opportunity_no" VARCHAR(50) NOT NULL UNIQUE,
    "opportunity_name" VARCHAR(200) NOT NULL,
    "customer_id" INTEGER NOT NULL,
    "lead_id" INTEGER,
    "opportunity_type" VARCHAR(50),
    "opportunity_stage" VARCHAR(50),
    "win_probability" DECIMAL(5, 2),
    "estimated_amount" DECIMAL(15, 2),
    "actual_amount" DECIMAL(15, 2),
    "currency" VARCHAR(10),
    "expected_close_date" DATE,
    "actual_close_date" DATE,
    "product_ids" TEXT,
    "product_names" TEXT,
    "product_desc" TEXT,
    "owner_id" INTEGER NOT NULL,
    "owner_name" VARCHAR(100) NOT NULL,
    "last_follow_up_date" DATE,
    "next_follow_up_date" DATE,
    "follow_up_plan" TEXT,
    "competitor_names" TEXT,
    "competitive_advantage" TEXT,
    "opportunity_status" VARCHAR(20),
    "won_reason" TEXT,
    "lost_reason" TEXT,
    "priority" VARCHAR(20),
    "rating" INTEGER,
    "tags" TEXT,
    "created_at" TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    "created_by" INTEGER,
    "updated_by" INTEGER,
    CONSTRAINT "fk_crm_opportunity_customer" FOREIGN KEY ("customer_id") REFERENCES "customers" ("id"),
    CONSTRAINT "fk_crm_opportunity_lead" FOREIGN KEY ("lead_id") REFERENCES "crm_lead" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_crm_opportunity_customer" ON "crm_opportunity" ("customer_id");
CREATE INDEX IF NOT EXISTS "idx_crm_opportunity_owner" ON "crm_opportunity" ("owner_id");
COMMENT ON TABLE "crm_opportunity" IS 'CRM商机表';

-- 15. 客户分配历史表
CREATE TABLE IF NOT EXISTS "assignment_histories" (
    "id" SERIAL PRIMARY KEY,
    "tenant_id" INTEGER NOT NULL,
    "lead_id" INTEGER NOT NULL,
    "lead_no" VARCHAR(50) NOT NULL,
    "company_name" VARCHAR(200),
    "from_user_id" INTEGER,
    "from_user_name" VARCHAR(100),
    "to_user_id" INTEGER,
    "to_user_name" VARCHAR(100),
    "action" VARCHAR(20) NOT NULL,
    "reason" TEXT,
    "notes" TEXT,
    "operated_by" INTEGER NOT NULL,
    "operated_by_name" VARCHAR(100) NOT NULL,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_assignment_histories_lead" ON "assignment_histories" ("lead_id");
COMMENT ON TABLE "assignment_histories" IS '客户分配历史表';

-- 16. 不合格品表
CREATE TABLE IF NOT EXISTS "unqualified_products" (
    "id" SERIAL PRIMARY KEY,
    "unqualified_no" VARCHAR(50) NOT NULL,
    "inspection_id" INTEGER,
    "product_id" INTEGER NOT NULL,
    "batch_no" VARCHAR(50),
    "unqualified_qty" DECIMAL(12, 2) NOT NULL,
    "unqualified_reason" TEXT NOT NULL,
    "handling_method" VARCHAR(50) NOT NULL,
    "handling_status" VARCHAR(20) NOT NULL,
    "handling_by" INTEGER,
    "handling_at" TIMESTAMP,
    "remark" TEXT,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "unqualified_products" IS '不合格品表';
