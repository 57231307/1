-- 质量异常表：记录色差、色牢度等质量问题
CREATE TABLE IF NOT EXISTS "quality_issues" (
    "id" BIGSERIAL PRIMARY KEY,
    "custom_order_id" BIGINT NOT NULL REFERENCES "custom_orders"("id") ON DELETE CASCADE,
    "process_node_id" BIGINT REFERENCES "process_nodes"("id"),
    "issue_type" VARCHAR(50) NOT NULL,
    "severity" VARCHAR(20) NOT NULL DEFAULT 'medium',
    "description" TEXT NOT NULL,
    "discovered_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "resolved_at" TIMESTAMPTZ,
    "resolution" TEXT,
    "status" VARCHAR(20) NOT NULL DEFAULT 'open',
    "tenant_id" BIGINT NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT "chk_issue_severity" CHECK ("severity" IN ('low', 'medium', 'high', 'critical')),
    CONSTRAINT "chk_issue_status" CHECK ("status" IN ('open', 'investigating', 'resolved', 'closed'))
);

CREATE INDEX IF NOT EXISTS "idx_quality_issues_order" ON "quality_issues"("custom_order_id");
CREATE INDEX IF NOT EXISTS "idx_quality_issues_status" ON "quality_issues"("status");
CREATE INDEX IF NOT EXISTS "idx_quality_issues_tenant" ON "quality_issues"("tenant_id");

COMMENT ON TABLE "quality_issues" IS '定制订单质量异常表';
COMMENT ON COLUMN "quality_issues"."issue_type" IS '异常类型：color_diff(色差) / color_fastness(色牢度) / spec(规格不符) / damage(破损) / other';
COMMENT ON COLUMN "quality_issues"."severity" IS '严重度：low(低) / medium(中) / high(高) / critical(严重)';
COMMENT ON COLUMN "quality_issues"."status" IS '状态：open(待处理) / investigating(调查中) / resolved(已解决) / closed(已关闭)';
