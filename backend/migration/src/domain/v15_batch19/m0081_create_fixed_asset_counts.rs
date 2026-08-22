use sea_orm_migration::prelude::*;

// V15 P1 17.8-D4：固定资产盘点表
//
// 业务背景：固定资产盘点闭环（盘点计划-盘点执行-差异处理-凭证生成），
// 解决资产账实不符无法发现、资产流失风险问题。
//
// 设计依据：V15 审计报告 batch-15 维度 17.8 缺陷 D4（P1）
// 关联文件：models/fixed_asset_count.rs / models/fixed_asset_count_item.rs
//           services/fixed_asset_service.rs

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- 固定资产盘点单表（V15 P1 17.8-D4 创建）
                CREATE TABLE IF NOT EXISTS "fixed_asset_counts" (
                    "id" BIGSERIAL PRIMARY KEY,
                    "count_no" VARCHAR(50) NOT NULL,
                    "plan_name" VARCHAR(200) NOT NULL,
                    "count_date" DATE NOT NULL,
                    "asset_category" VARCHAR(100),
                    "use_location" VARCHAR(200),
                    "status" VARCHAR(20) NOT NULL DEFAULT 'DRAFT',
                    "total_items" INTEGER NOT NULL DEFAULT 0,
                    "counted_items" INTEGER NOT NULL DEFAULT 0,
                    "surplus_items" INTEGER NOT NULL DEFAULT 0,
                    "shortage_items" INTEGER NOT NULL DEFAULT 0,
                    "notes" TEXT,
                    "created_by" INTEGER NOT NULL,
                    "approved_by" INTEGER,
                    "completed_at" TIMESTAMPTZ,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    CONSTRAINT "uk_fac_count_no" UNIQUE ("count_no"),
                    CONSTRAINT "chk_fac_status" CHECK (
                        "status" IN ('DRAFT', 'COUNTING', 'COMPLETED')
                    )
                );

                -- 固定资产盘点明细表
                CREATE TABLE IF NOT EXISTS "fixed_asset_count_items" (
                    "id" BIGSERIAL PRIMARY KEY,
                    "count_id" BIGINT NOT NULL,
                    "asset_id" INTEGER NOT NULL,
                    "asset_no" VARCHAR(50) NOT NULL,
                    "asset_name" VARCHAR(200) NOT NULL,
                    "book_original_value" DECIMAL(18,2) NOT NULL,
                    "book_net_value" DECIMAL(18,2),
                    "book_use_location" VARCHAR(200),
                    "actual_original_value" DECIMAL(18,2),
                    "actual_net_value" DECIMAL(18,2),
                    "actual_use_location" VARCHAR(200),
                    "count_result" VARCHAR(20),
                    "variance_type" VARCHAR(20),
                    "variance_amount" DECIMAL(18,2),
                    "remarks" TEXT,
                    "counted_by" INTEGER,
                    "counted_at" TIMESTAMPTZ,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    CONSTRAINT "fk_fac_count_id" FOREIGN KEY ("count_id")
                        REFERENCES "fixed_asset_counts"("id") ON DELETE CASCADE,
                    CONSTRAINT "uk_fac_count_asset" UNIQUE ("count_id", "asset_id"),
                    CONSTRAINT "chk_fac_count_result" CHECK (
                        "count_result" IN ('consistent', 'surplus', 'shortage', 'damaged')
                    )
                );

                -- 索引
                CREATE INDEX IF NOT EXISTS "idx_fac_status" ON "fixed_asset_counts"("status");
                CREATE INDEX IF NOT EXISTS "idx_fac_count_date" ON "fixed_asset_counts"("count_date");
                CREATE INDEX IF NOT EXISTS "idx_faci_count_id" ON "fixed_asset_count_items"("count_id");
                CREATE INDEX IF NOT EXISTS "idx_faci_asset_id" ON "fixed_asset_count_items"("asset_id");
                CREATE INDEX IF NOT EXISTS "idx_faci_variance_type" ON "fixed_asset_count_items"("variance_type");

                COMMENT ON TABLE "fixed_asset_counts" IS '固定资产盘点单 - 盘点计划-执行-差异闭环';
                COMMENT ON TABLE "fixed_asset_count_items" IS '固定资产盘点明细 - 账实对比记录';
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
                DROP TABLE IF EXISTS "fixed_asset_count_items";
                DROP TABLE IF EXISTS "fixed_asset_counts";
                "#,
            )
            .await?;
        Ok(())
    }
}
