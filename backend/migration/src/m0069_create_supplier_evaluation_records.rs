use sea_orm_migration::prelude::*;

// V15 批次 15 P1 修复：补齐 supplier_evaluation_records 表迁移
//
// 业务背景：
//   models/supplier_evaluation_record.rs 已定义 Entity（表名 supplier_evaluation_records），
//   services/supplier_evaluation_service.rs 已实现 create_evaluation_record /
//   get_supplier_score / get_supplier_rankings 等业务方法，但全代码库无对应建表迁移，
//   导致运行时表不存在、所有评估记录读写失败（Rule 0/1/2：真实实现禁止 stub）。
//
//   现有迁移 20260528000001_add_crm_supplier_tables 仅创建了 supplier_evaluation_indicators
//   指标表，遗漏了 supplier_evaluation_records 评估记录表。
//
// 修复方案：
//   新建 supplier_evaluation_records 表，字段与 model 定义严格一致：
//   - id（主键）/ supplier_id（FK suppliers）/ evaluation_period / indicator_id（FK indicators）
//   - score / max_score / weighted_score / evaluator_id / evaluation_date / remark / created_at
//   - 索引：supplier_id / indicator_id / evaluation_period / evaluation_date
//   - CHECK：score >= 0

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TABLE IF NOT EXISTS "supplier_evaluation_records" (
                    "id" SERIAL PRIMARY KEY,
                    "supplier_id" INTEGER NOT NULL,
                    "evaluation_period" VARCHAR(50) NOT NULL,
                    "indicator_id" INTEGER NOT NULL,
                    "score" DECIMAL(10,2) NOT NULL,
                    "max_score" INTEGER,
                    "weighted_score" DECIMAL(10,2),
                    "evaluator_id" INTEGER,
                    "evaluation_date" DATE,
                    "remark" TEXT,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    CONSTRAINT "fk_supplier_evaluation_records_supplier"
                        FOREIGN KEY ("supplier_id") REFERENCES "suppliers" ("id"),
                    CONSTRAINT "fk_supplier_evaluation_records_indicator"
                        FOREIGN KEY ("indicator_id") REFERENCES "supplier_evaluation_indicators" ("id"),
                    CONSTRAINT "chk_supplier_evaluation_records_score"
                        CHECK ("score" >= 0)
                );

                CREATE INDEX IF NOT EXISTS "idx_supplier_evaluation_records_supplier"
                    ON "supplier_evaluation_records"("supplier_id");
                CREATE INDEX IF NOT EXISTS "idx_supplier_evaluation_records_indicator"
                    ON "supplier_evaluation_records"("indicator_id");
                CREATE INDEX IF NOT EXISTS "idx_supplier_evaluation_records_period"
                    ON "supplier_evaluation_records"("evaluation_period");
                CREATE INDEX IF NOT EXISTS "idx_supplier_evaluation_records_date"
                    ON "supplier_evaluation_records"("evaluation_date");

                COMMENT ON TABLE "supplier_evaluation_records" IS '供应商评估记录表（每次评估的指标得分明细）';
                COMMENT ON COLUMN "supplier_evaluation_records"."evaluation_period" IS '评估周期（如 2024Q1）';
                COMMENT ON COLUMN "supplier_evaluation_records"."weighted_score" IS '加权得分 = score * weight / max_score';
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
                DROP TABLE IF EXISTS "supplier_evaluation_records";
                "#,
            )
            .await?;
        Ok(())
    }
}
