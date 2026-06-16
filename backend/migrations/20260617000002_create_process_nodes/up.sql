-- 工艺节点表：5 阶段工艺节点（纱线采购/染整/后整理/交付/售后）
CREATE TABLE IF NOT EXISTS "process_nodes" (
    "id" BIGSERIAL PRIMARY KEY,
    "custom_order_id" BIGINT NOT NULL REFERENCES "custom_orders"("id") ON DELETE CASCADE,
    "node_type" VARCHAR(30) NOT NULL,
    "node_name" VARCHAR(100) NOT NULL,
    "sequence" INTEGER NOT NULL,
    "status" VARCHAR(20) NOT NULL DEFAULT 'pending',
    "planned_start_date" TIMESTAMPTZ,
    "planned_end_date" TIMESTAMPTZ,
    "actual_start_date" TIMESTAMPTZ,
    "actual_end_date" TIMESTAMPTZ,
    "operator_id" BIGINT REFERENCES "users"("id"),
    "notes" TEXT,
    "tenant_id" BIGINT NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT "chk_node_type" CHECK ("node_type" IN (
        'yarn_purchasing', 'dyeing', 'finishing', 'delivery', 'after_sales'
    )),
    CONSTRAINT "chk_node_status" CHECK ("status" IN (
        'pending', 'in_progress', 'completed', 'blocked'
    ))
);

CREATE INDEX IF NOT EXISTS "idx_process_nodes_order" ON "process_nodes"("custom_order_id");
CREATE INDEX IF NOT EXISTS "idx_process_nodes_status" ON "process_nodes"("status");
CREATE INDEX IF NOT EXISTS "idx_process_nodes_tenant" ON "process_nodes"("tenant_id");

COMMENT ON TABLE "process_nodes" IS '定制订单工艺节点表';
COMMENT ON COLUMN "process_nodes"."node_type" IS '节点类型：yarn_purchasing(纱线采购) / dyeing(染整) / finishing(后整理) / delivery(交付) / after_sales(售后)';
COMMENT ON COLUMN "process_nodes"."status" IS '节点状态：pending(待开始) / in_progress(进行中) / completed(已完成) / blocked(阻塞)';
