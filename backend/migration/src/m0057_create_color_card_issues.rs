use sea_orm_migration::prelude::*;

// Batch 477 P0-F13 修复：创建 color_card_issues 表
//
// 关键背景：
//   - V15 Batch 471 PR #646 创建了 color_card_issue.rs model + color_card_issue_service.rs
//     + handlers/color_card/issue.rs + routes/color_card.rs，但遗漏了数据库表迁移。
//   - model 声明 #[sea_orm(table_name = "color_card_issues")] 但 migrations 目录下
//     无对应 CREATE TABLE，导致 API 运行时直接报 "relation color_card_issues does not exist"。
//
// 本迁移根据 model 定义 + 旧表 color_card_borrow_records 字段补齐建表 SQL：
//   - 状态枚举扩展为 issued/returned/lost/damaged/cancelled（含 cancelled 终态）
//   - 字段对齐 model：id/color_card_id/customer_id/issue_qty/issued_by/issued_at/
//     expected_return_date/actual_return_date/status/purpose/remark/
//     compensation_amount/returned_by/dye_lot_no/created_at/updated_at/is_deleted
//   - 索引：color_card_id/customer_id/status/issued_at/issued_by
//
// 设计依据：V15 审计报告 类九 P0-F13
// 关联文件：models/color_card_issue.rs / services/color_card_issue_service.rs

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- 色卡发放记录表（V15 P0-F04 设计，本批次补齐建表迁移）
                -- 替代旧 color_card_borrow_records（旧表保留不重命名以保护 Rust migration m0029 链路）
                CREATE TABLE IF NOT EXISTS "color_card_issues" (
                    "id" BIGSERIAL PRIMARY KEY,
                    "color_card_id" BIGINT NOT NULL REFERENCES "color_cards"("id") ON DELETE RESTRICT,
                    "customer_id" BIGINT NOT NULL REFERENCES "customers"("id") ON DELETE RESTRICT,
                    "issue_qty" INT NOT NULL DEFAULT 1,
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
                    CONSTRAINT "chk_color_card_issue_status" CHECK ("status" IN ('issued', 'returned', 'lost', 'damaged', 'cancelled'))
                );

                -- 索引
                CREATE INDEX IF NOT EXISTS "idx_color_card_issue_card" ON "color_card_issues"("color_card_id");
                CREATE INDEX IF NOT EXISTS "idx_color_card_issue_customer" ON "color_card_issues"("customer_id");
                CREATE INDEX IF NOT EXISTS "idx_color_card_issue_status" ON "color_card_issues"("status");
                CREATE INDEX IF NOT EXISTS "idx_color_card_issue_issued_at" ON "color_card_issues"("issued_at" DESC);
                CREATE INDEX IF NOT EXISTS "idx_color_card_issue_issuer" ON "color_card_issues"("issued_by");
                CREATE INDEX IF NOT EXISTS "idx_color_card_issue_is_deleted" ON "color_card_issues"("is_deleted");

                COMMENT ON TABLE "color_card_issues" IS '色卡发放记录 - 发放/归还/遗失/损坏/取消全生命周期';
                COMMENT ON COLUMN "color_card_issues"."status" IS '状态：issued(发放中) / returned(已归还) / lost(已遗失) / damaged(已损坏) / cancelled(已取消)';
                COMMENT ON COLUMN "color_card_issues"."issue_qty" IS '发放数量（色卡单张发放，默认 1）';
                COMMENT ON COLUMN "color_card_issues"."compensation_amount" IS '遗失/损坏赔付金额（CNY）';
                COMMENT ON COLUMN "color_card_issues"."dye_lot_no" IS '染色批号（lot 概念，防色差混批）';
                COMMENT ON COLUMN "color_card_issues"."is_deleted" IS '软删除标记';
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
                DROP TABLE IF EXISTS "color_card_issues";
                "#,
            )
            .await?;
        Ok(())
    }
}
