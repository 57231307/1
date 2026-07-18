use sea_orm_migration::prelude::*;

// Batch 481 P0-B01：坏账准备计提表
//
// 业务背景：企业会计准则第 22 号要求期末按账龄分析法计提坏账准备。
// 账龄法计提比例：1 年内 5% / 1-2 年 20% / 2-3 年 50% / 3 年以上 100%。
// 月末 cron 自动扫描 ar_invoice 未收金额，按客户+账龄桶聚合计提，生成借资产减值损失/贷坏账准备凭证。
//
// 设计依据：V15 审计报告 batch-15 维度 17.3 缺陷 D1（P0）
// 关联文件：models/bad_debt_provision.rs / services/bad_debt_service.rs /
//          handlers/bad_debt_handler.rs / routes/bad_debt.rs

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- 坏账准备计提表（V15 P0-B01 创建）
                -- 按客户+账龄桶+期间记录每期计提/转回，关联凭证
                CREATE TABLE IF NOT EXISTS "bad_debt_provisions" (
                    "id" BIGSERIAL PRIMARY KEY,
                    -- 业务关联
                    "customer_id" BIGINT NOT NULL REFERENCES "customers"("id") ON DELETE RESTRICT,
                    "customer_name" VARCHAR(200),
                    -- 期间
                    "period_year" INTEGER NOT NULL,
                    "period_month" INTEGER NOT NULL CHECK ("period_month" BETWEEN 1 AND 12),
                    -- 账龄桶（按 ar_invoice.due_date 计算）
                    "aging_bucket" VARCHAR(20) NOT NULL,
                    -- 计提基数与比例
                    "base_amount" DECIMAL(15,2) NOT NULL CHECK ("base_amount" >= 0),
                    "provision_rate" DECIMAL(5,4) NOT NULL CHECK ("provision_rate" >= 0 AND "provision_rate" <= 1),
                    "provision_amount" DECIMAL(15,2) NOT NULL CHECK ("provision_amount" >= 0),
                    -- 凭证关联
                    "voucher_id" BIGINT,
                    -- 状态：draft（草稿）/ confirmed（已确认计提）/ reversed（已转回）
                    "status" VARCHAR(20) NOT NULL DEFAULT 'draft',
                    -- 操作人
                    "created_by" INTEGER NOT NULL REFERENCES "users"("id") ON DELETE RESTRICT,
                    "confirmed_at" TIMESTAMPTZ,
                    "reversed_at" TIMESTAMPTZ,
                    "reverse_voucher_id" BIGINT,
                    "remark" TEXT,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    -- 约束
                    CONSTRAINT "chk_bdp_aging_bucket" CHECK (
                        "aging_bucket" IN ('within_1y', '1_to_2y', '2_to_3y', 'over_3y')
                    ),
                    CONSTRAINT "chk_bdp_status" CHECK (
                        "status" IN ('draft', 'confirmed', 'reversed')
                    ),
                    CONSTRAINT "chk_bdp_period" CHECK ("period_year" >= 2000 AND "period_year" <= 2100)
                );

                -- 索引（5 个）
                CREATE INDEX IF NOT EXISTS "idx_bdp_customer_id" ON "bad_debt_provisions"("customer_id");
                CREATE INDEX IF NOT EXISTS "idx_bdp_period" ON "bad_debt_provisions"("period_year", "period_month");
                CREATE INDEX IF NOT EXISTS "idx_bdp_status" ON "bad_debt_provisions"("status");
                CREATE INDEX IF NOT EXISTS "idx_bdp_aging_bucket" ON "bad_debt_provisions"("aging_bucket");
                CREATE INDEX IF NOT EXISTS "idx_bdp_voucher_id" ON "bad_debt_provisions"("voucher_id");

                COMMENT ON TABLE "bad_debt_provisions" IS '坏账准备计提表 - 账龄法按客户+期间+账龄桶记录计提与转回';
                COMMENT ON COLUMN "bad_debt_provisions"."aging_bucket" IS '账龄桶：within_1y(1年内5%) / 1_to_2y(1-2年20%) / 2_to_3y(2-3年50%) / over_3y(3年以上100%)';
                COMMENT ON COLUMN "bad_debt_provisions"."provision_rate" IS '计提比例（0~1）：within_1y=0.05 / 1_to_2y=0.20 / 2_to_3y=0.50 / over_3y=1.00';
                COMMENT ON COLUMN "bad_debt_provisions"."status" IS '状态：draft(草稿) / confirmed(已确认计提) / reversed(已转回)';
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
                DROP TABLE IF EXISTS "bad_debt_provisions";
                "#,
            )
            .await?;
        Ok(())
    }
}
