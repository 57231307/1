use sea_orm_migration::prelude::*;

// V15 P0-F10 Batch 477：色卡发放库存联动
// 1. 创建 color_card_issues 表（补齐 Batch 471 遗漏的 migration）
// 2. color_cards 表新增 stock_quantity（总库存）/ issued_quantity（已发放数量）字段

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. 创建 color_card_issues 表（与 color_card_issue.rs model 字段对齐，17 字段 + 5 索引）
        manager
            .get_connection()
            .execute_unprepared(
                r#"
CREATE TABLE IF NOT EXISTS "color_card_issues" (
    "id" BIGSERIAL PRIMARY KEY,
    "color_card_id" BIGINT NOT NULL REFERENCES "color_cards"("id") ON DELETE RESTRICT,
    "customer_id" BIGINT NOT NULL REFERENCES "customers"("id") ON DELETE RESTRICT,
    "issue_qty" INTEGER NOT NULL CHECK ("issue_qty" > 0),
    "issued_by" BIGINT NOT NULL REFERENCES "users"("id") ON DELETE RESTRICT,
    "issued_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "expected_return_date" DATE,
    "actual_return_date" DATE,
    "status" VARCHAR(20) NOT NULL DEFAULT 'issued',
    "purpose" TEXT,
    "remark" TEXT,
    "compensation_amount" DECIMAL(15,2) CHECK ("compensation_amount" IS NULL OR "compensation_amount" >= 0),
    "returned_by" BIGINT REFERENCES "users"("id") ON DELETE SET NULL,
    "dye_lot_no" VARCHAR(50),
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    CONSTRAINT "chk_issue_status" CHECK ("status" IN ('issued', 'returned', 'lost', 'damaged', 'cancelled'))
);

CREATE INDEX IF NOT EXISTS "idx_issue_card" ON "color_card_issues"("color_card_id");
CREATE INDEX IF NOT EXISTS "idx_issue_customer" ON "color_card_issues"("customer_id");
CREATE INDEX IF NOT EXISTS "idx_issue_status" ON "color_card_issues"("status");
CREATE INDEX IF NOT EXISTS "idx_issue_issued_at" ON "color_card_issues"("issued_at" DESC);
CREATE INDEX IF NOT EXISTS "idx_issue_issued_by" ON "color_card_issues"("issued_by");

COMMENT ON TABLE "color_card_issues" IS '色卡发放记录 - 发放/归还/遗失/损坏/取消全生命周期跟踪（V15 P0-F04 替代旧 color_card_borrow_records）';
COMMENT ON COLUMN "color_card_issues"."status" IS '发放状态：issued(发放中) / returned(已归还) / lost(遗失) / damaged(损坏) / cancelled(已取消)';
COMMENT ON COLUMN "color_card_issues"."issue_qty" IS '发放数量（必须 > 0）';
COMMENT ON COLUMN "color_card_issues"."dye_lot_no" IS '染色批号（lot 概念，防色差混批）';
"#,
            )
            .await?;

        // 2. color_cards 表新增 stock_quantity / issued_quantity 字段（库存联动基础）
        manager
            .get_connection()
            .execute_unprepared(
                r#"
ALTER TABLE "color_cards"
    ADD COLUMN IF NOT EXISTS "stock_quantity" INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS "issued_quantity" INTEGER NOT NULL DEFAULT 0;

COMMENT ON COLUMN "color_cards"."stock_quantity" IS '色卡总库存数量（V15 P0-F10 库存联动）';
COMMENT ON COLUMN "color_cards"."issued_quantity" IS '已发放数量（V15 P0-F10 库存联动，issued_qty <= stock_quantity）';
"#,
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
ALTER TABLE "color_cards" DROP COLUMN IF EXISTS "issued_quantity";
ALTER TABLE "color_cards" DROP COLUMN IF EXISTS "stock_quantity";
DROP TABLE IF EXISTS "color_card_issues";
"#,
            )
            .await?;
        Ok(())
    }
}
