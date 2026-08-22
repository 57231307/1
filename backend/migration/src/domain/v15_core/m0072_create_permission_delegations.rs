use sea_orm_migration::prelude::*;

// V15 P1 Batch-10 12.6：权限委托表（Permission Delegation）
//
// 业务场景：
// - 销售经理请假/出差期间，可将部分审批权限临时委托给销售代表
// - 委托必须有时限（valid_from + valid_until）
// - 委托必须记录审计日志
// - 委托不可再委托（is_chain_allowed 默认 false，禁止链式委托）
//
// 设计要点：
// - delegator_id（委托人） / delegatee_id（被委托人）
// - permission_code（权限码，如 "sales.order.approve"）
// - valid_from / valid_until（时限，过期自动失效）
// - is_chain_allowed（是否允许被委托人再委托，默认 false）
// - status（pending / active / expired / revoked）
// - 审计字段：created_by / created_at / updated_at

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TABLE IF NOT EXISTS "permission_delegations" (
                    "id" BIGSERIAL PRIMARY KEY,
                    "delegator_id" INTEGER NOT NULL REFERENCES "users"("id") ON DELETE CASCADE,
                    "delegatee_id" INTEGER NOT NULL REFERENCES "users"("id") ON DELETE CASCADE,
                    "permission_code" VARCHAR(100) NOT NULL,
                    "valid_from" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "valid_until" TIMESTAMPTZ NOT NULL,
                    "is_chain_allowed" BOOLEAN NOT NULL DEFAULT FALSE,
                    "status" VARCHAR(20) NOT NULL DEFAULT 'active',
                    "reason" VARCHAR(500),
                    "revoked_at" TIMESTAMPTZ,
                    "revoked_by" INTEGER REFERENCES "users"("id") ON DELETE SET NULL,
                    "revoke_reason" VARCHAR(500),
                    "created_by" INTEGER REFERENCES "users"("id") ON DELETE SET NULL,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    -- 委托人与被委托人不可为同一人
                    CONSTRAINT "chk_no_self_delegation" CHECK ("delegator_id" <> "delegatee_id"),
                    -- valid_until 必须晚于 valid_from
                    CONSTRAINT "chk_valid_until_after_from" CHECK ("valid_until" > "valid_from"),
                    -- status 取值约束
                    CONSTRAINT "chk_delegation_status" CHECK (
                        "status" IN ('pending', 'active', 'expired', 'revoked')
                    )
                );

                CREATE INDEX IF NOT EXISTS "idx_delegation_delegator" ON "permission_delegations"("delegator_id");
                CREATE INDEX IF NOT EXISTS "idx_delegation_delegatee" ON "permission_delegations"("delegatee_id");
                CREATE INDEX IF NOT EXISTS "idx_delegation_status" ON "permission_delegations"("status");
                CREATE INDEX IF NOT EXISTS "idx_delegation_valid_until" ON "permission_delegations"("valid_until");

                COMMENT ON TABLE "permission_delegations" IS
                    'V15 P1 12.6：权限委托表，支持时限化临时委托（禁止链式委托）';
                COMMENT ON COLUMN "permission_delegations"."is_chain_allowed" IS
                    '是否允许被委托人再委托（默认 false，禁止链式委托）';
                COMMENT ON COLUMN "permission_delegations"."permission_code" IS
                    '委托的权限码（如 sales.order.approve），与 role_permissions.resource_type+action 对应';
                "#,
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(r#"DROP TABLE IF EXISTS "permission_delegations";"#)
            .await?;
        Ok(())
    }
}
