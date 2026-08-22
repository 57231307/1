use sea_orm_migration::prelude::*;

// V15 P1-10：大货批色历史追溯表
//
// 业务背景：审计报告类十一 11.6 / P1-10 要求批色业务必须支持历史追溯，
// 每次状态变更都需记录 old_status / new_status / operator / reason / snapshot，
// 以满足客户投诉追溯、内部责任界定、合规审计三大业务场景。
//
// 设计依据：V15 审计报告 batch-10 P1-10（批色历史追溯）
// 关联文件：models/bulk_color_approval_history.rs /
//          services/bulk_color_approval_service.rs::record_history

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- 大货批色状态变更历史表（V15 P1-10 创建）
                -- 记录 bulk_color_approval 每次状态变更的全量快照
                CREATE TABLE IF NOT EXISTS "bulk_color_approval_history" (
                    "id" BIGSERIAL PRIMARY KEY,
                    "bulk_color_approval_id" BIGINT NOT NULL REFERENCES "bulk_color_approval"("id") ON DELETE CASCADE,
                    "from_status" VARCHAR(20),
                    "to_status" VARCHAR(20) NOT NULL,
                    "operator_id" INTEGER REFERENCES "users"("id") ON DELETE SET NULL,
                    "reason" TEXT,
                    "snapshot" JSONB,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
                );

                CREATE INDEX IF NOT EXISTS "idx_bcah_approval_id" ON "bulk_color_approval_history"("bulk_color_approval_id");
                CREATE INDEX IF NOT EXISTS "idx_bcah_to_status" ON "bulk_color_approval_history"("to_status");
                CREATE INDEX IF NOT EXISTS "idx_bcah_created_at" ON "bulk_color_approval_history"("created_at");

                COMMENT ON TABLE "bulk_color_approval_history" IS '大货批色状态变更历史 - 全量追溯每次状态流转';
                COMMENT ON COLUMN "bulk_color_approval_history"."from_status" IS '变更前状态（首次创建时为 NULL）';
                COMMENT ON COLUMN "bulk_color_approval_history"."to_status" IS '变更后状态';
                COMMENT ON COLUMN "bulk_color_approval_history"."operator_id" IS '操作人用户 ID（客户操作时为关联的业务用户）';
                COMMENT ON COLUMN "bulk_color_approval_history"."reason" IS '变更原因（拒绝/返工/降级/报废时填写）';
                COMMENT ON COLUMN "bulk_color_approval_history"."snapshot" IS '变更后记录完整快照 JSON';
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
                DROP TABLE IF EXISTS "bulk_color_approval_history";
                "#,
            )
            .await?;
        Ok(())
    }
}
