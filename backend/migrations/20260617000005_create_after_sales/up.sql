-- 售后工单表：4 种类型（客诉/维修/换货/退款）
CREATE TABLE IF NOT EXISTS "after_sales" (
    "id" BIGSERIAL PRIMARY KEY,
    "custom_order_id" BIGINT NOT NULL REFERENCES "custom_orders"("id"),
    "issue_type" VARCHAR(30) NOT NULL,
    "customer_id" BIGINT NOT NULL REFERENCES "customers"("id"),
    "description" TEXT NOT NULL,
    "status" VARCHAR(20) NOT NULL DEFAULT 'opened',
    "opened_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "closed_at" TIMESTAMPTZ,
    "resolution" TEXT,
    "refund_amount" DECIMAL(18,2),
    "tenant_id" BIGINT NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT "chk_aftersales_type" CHECK ("issue_type" IN (
        'complaint', 'repair', 'exchange', 'refund'
    )),
    CONSTRAINT "chk_aftersales_status" CHECK ("status" IN (
        'opened', 'processing', 'resolved', 'closed', 'rejected'
    ))
);

CREATE INDEX IF NOT EXISTS "idx_aftersales_order" ON "after_sales"("custom_order_id");
CREATE INDEX IF NOT EXISTS "idx_aftersales_customer" ON "after_sales"("customer_id");
CREATE INDEX IF NOT EXISTS "idx_aftersales_status" ON "after_sales"("status");
CREATE INDEX IF NOT EXISTS "idx_aftersales_tenant" ON "after_sales"("tenant_id");

COMMENT ON TABLE "after_sales" IS '定制订单售后工单表';
COMMENT ON COLUMN "after_sales"."issue_type" IS '售后类型：complaint(客诉) / repair(维修) / exchange(换货) / refund(退款)';
COMMENT ON COLUMN "after_sales"."status" IS '状态：opened(已开) / processing(处理中) / resolved(已解决) / closed(已关闭) / rejected(已拒绝)';
