-- 流程日志表：记录节点操作日志（时间戳/操作人/前后状态/附件）
CREATE TABLE IF NOT EXISTS "process_logs" (
    "id" BIGSERIAL PRIMARY KEY,
    "process_node_id" BIGINT NOT NULL REFERENCES "process_nodes"("id") ON DELETE CASCADE,
    "action" VARCHAR(50) NOT NULL,
    "operator_id" BIGINT REFERENCES "users"("id"),
    "before_status" VARCHAR(20),
    "after_status" VARCHAR(20),
    "log_time" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "log_content" TEXT,
    "attachments" JSONB NOT NULL DEFAULT '[]'::jsonb,
    "tenant_id" BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS "idx_process_logs_node" ON "process_logs"("process_node_id");
CREATE INDEX IF NOT EXISTS "idx_process_logs_time" ON "process_logs"("log_time" DESC);
CREATE INDEX IF NOT EXISTS "idx_process_logs_tenant" ON "process_logs"("tenant_id");

COMMENT ON TABLE "process_logs" IS '定制订单工艺节点操作日志表';
COMMENT ON COLUMN "process_logs"."action" IS '操作类型：start/pause/resume/complete/block/unblock';
COMMENT ON COLUMN "process_logs"."attachments" IS '操作附件 URL 列表 JSONB';
