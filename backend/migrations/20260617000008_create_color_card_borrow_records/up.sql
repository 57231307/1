-- 色卡仓储管理模块 migration - color_card_borrow_records 表
-- 创建时间: 2026-06-17
-- 关联 spec: docs/superpowers/specs/2026-06-16-color-card-design.md §3.4

-- 色卡借出记录表：跟踪色卡借出/归还/遗失的全生命周期
CREATE TABLE IF NOT EXISTS "color_card_borrow_records" (
    "id" BIGSERIAL PRIMARY KEY,
    "color_card_id" BIGINT NOT NULL REFERENCES "color_cards"("id") ON DELETE RESTRICT,
    "customer_id" BIGINT NOT NULL REFERENCES "customers"("id") ON DELETE RESTRICT,
    "borrowed_by" BIGINT NOT NULL REFERENCES "users"("id") ON DELETE RESTRICT,
    "borrowed_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "expected_return_at" TIMESTAMPTZ,
    "actual_return_at" TIMESTAMPTZ,
    "status" VARCHAR(20) NOT NULL DEFAULT 'borrowed',
    "purpose" TEXT,
    "notes" TEXT,
    "compensation_amount" DECIMAL(15,2) CHECK ("compensation_amount" IS NULL OR "compensation_amount" >= 0),
    "tenant_id" BIGINT NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT "chk_borrow_status" CHECK ("status" IN ('borrowed', 'returned', 'lost', 'damaged'))
);

-- 索引
CREATE INDEX IF NOT EXISTS "idx_borrow_card" ON "color_card_borrow_records"("color_card_id");
CREATE INDEX IF NOT EXISTS "idx_borrow_customer" ON "color_card_borrow_records"("customer_id");
CREATE INDEX IF NOT EXISTS "idx_borrow_status" ON "color_card_borrow_records"("status");
CREATE INDEX IF NOT EXISTS "idx_borrow_tenant" ON "color_card_borrow_records"("tenant_id");
CREATE INDEX IF NOT EXISTS "idx_borrow_borrowed_at" ON "color_card_borrow_records"("borrowed_at" DESC);
CREATE INDEX IF NOT EXISTS "idx_borrow_borrower" ON "color_card_borrow_records"("borrowed_by");

COMMENT ON TABLE "color_card_borrow_records" IS '色卡借出记录 - 色卡借出/归还/遗失的全生命周期跟踪';
COMMENT ON COLUMN "color_card_borrow_records"."status" IS '借出状态：borrowed(借出中) / returned(已归还) / lost(遗失) / damaged(损坏)';
COMMENT ON COLUMN "color_card_borrow_records"."compensation_amount" IS '遗失/损坏赔付金额（CNY）';
COMMENT ON COLUMN "color_card_borrow_records"."borrowed_by" IS '经办员工 ID（关联 users 表）';
