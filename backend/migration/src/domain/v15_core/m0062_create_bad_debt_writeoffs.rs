use sea_orm_migration::prelude::*;

// Batch 481 P0-B02：坏账核销与审批流表
//
// 业务背景：实际坏账需经二级审批（申请人→财务经理→总经理）后核销，
// 核销时生成借坏账准备/贷应收账款凭证，并更新 ar_invoice 状态为 written_off。
//
// 设计依据：V15 审计报告 batch-15 维度 17.3 缺陷 D2（P0）
// 关联文件：models/bad_debt_writeoff.rs / services/bad_debt_service.rs /
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
                -- 坏账核销审批表（V15 P0-B02 创建）
                -- 二级审批流：申请人→财务经理（level 1）→总经理（level 2）→核销执行
                CREATE TABLE IF NOT EXISTS "bad_debt_writeoffs" (
                    "id" BIGSERIAL PRIMARY KEY,
                    -- 业务关联
                    "customer_id" BIGINT NOT NULL REFERENCES "customers"("id") ON DELETE RESTRICT,
                    "ar_invoice_id" INTEGER NOT NULL REFERENCES "ar_invoices"("id") ON DELETE RESTRICT,
                    "writeoff_amount" DECIMAL(15,2) NOT NULL CHECK ("writeoff_amount" > 0),
                    "reason" TEXT NOT NULL,
                    -- 申请人
                    "applicant_user_id" INTEGER NOT NULL REFERENCES "users"("id") ON DELETE RESTRICT,
                    "applicant_username" VARCHAR(100) NOT NULL,
                    "applicant_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    -- 审批流（二级）
                    "approval_level" SMALLINT NOT NULL DEFAULT 1 CHECK ("approval_level" BETWEEN 1 AND 2),
                    "approval_status" VARCHAR(20) NOT NULL DEFAULT 'pending',
                    -- 一级审批（财务经理）
                    "finance_manager_id" INTEGER REFERENCES "users"("id") ON DELETE SET NULL,
                    "finance_manager_at" TIMESTAMPTZ,
                    "finance_manager_comment" TEXT,
                    -- 二级审批（总经理）
                    "general_manager_id" INTEGER REFERENCES "users"("id") ON DELETE SET NULL,
                    "general_manager_at" TIMESTAMPTZ,
                    "general_manager_comment" TEXT,
                    -- 核销执行
                    "voucher_id" BIGINT,
                    "completed_at" TIMESTAMPTZ,
                    "cancelled_at" TIMESTAMPTZ,
                    "cancel_reason" TEXT,
                    "remark" TEXT,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    -- 约束
                    CONSTRAINT "chk_bdw_status" CHECK (
                        "approval_status" IN ('pending', 'finance_approved', 'approved', 'rejected', 'cancelled')
                    )
                );

                -- 索引（5 个）
                CREATE INDEX IF NOT EXISTS "idx_bdw_customer_id" ON "bad_debt_writeoffs"("customer_id");
                CREATE INDEX IF NOT EXISTS "idx_bdw_ar_invoice_id" ON "bad_debt_writeoffs"("ar_invoice_id");
                CREATE INDEX IF NOT EXISTS "idx_bdw_approval_status" ON "bad_debt_writeoffs"("approval_status");
                CREATE INDEX IF NOT EXISTS "idx_bdw_applicant_user_id" ON "bad_debt_writeoffs"("applicant_user_id");
                CREATE INDEX IF NOT EXISTS "idx_bdw_voucher_id" ON "bad_debt_writeoffs"("voucher_id");

                COMMENT ON TABLE "bad_debt_writeoffs" IS '坏账核销审批表 - 二级审批流（申请人→财务经理→总经理）';
                COMMENT ON COLUMN "bad_debt_writeoffs"."approval_level" IS '当前审批层级：1=待财务经理审批 / 2=待总经理审批';
                COMMENT ON COLUMN "bad_debt_writeoffs"."approval_status" IS '状态：pending(待审) / finance_approved(财务经理通过,待总经理) / approved(最终通过,已核销) / rejected(拒绝) / cancelled(取消)';
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
                DROP TABLE IF EXISTS "bad_debt_writeoffs";
                "#,
            )
            .await?;
        Ok(())
    }
}
