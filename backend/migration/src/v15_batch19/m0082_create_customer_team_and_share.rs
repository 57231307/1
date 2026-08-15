use sea_orm_migration::prelude::*;

// V15 P1 18.4-D2/D3：CRM 团队协作 + 数据共享时效
//
// 业务背景：
// - 18.4-D2：缺少团队协作机制，大客户需多人跟进时无法协作
// - 18.4-D3：缺少数据共享时效，共享客户无时效控制，权限收回困难
//
// 设计依据：V15 审计报告 batch-15 维度 18.4 缺陷 D2/D3（P1）
// 关联文件：
//   models/customer_team_member.rs - 客户团队成员关联模型
//   models/customer_share.rs       - 客户数据共享模型
//   services/crm/customer_team_share_service.rs - 团队协作与共享服务

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- =====================================================
                -- 18.4-D2：客户团队成员关联表
                -- =====================================================
                CREATE TABLE IF NOT EXISTS "customer_team_members" (
                    "id" BIGSERIAL PRIMARY KEY,
                    "customer_id" INTEGER NOT NULL,
                    "user_id" INTEGER NOT NULL,
                    "user_name" VARCHAR(100),
                    "team_role" VARCHAR(20) NOT NULL DEFAULT 'member',
                    "is_active" BOOLEAN NOT NULL DEFAULT TRUE,
                    "joined_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "left_at" TIMESTAMPTZ,
                    "notes" TEXT,
                    "created_by" INTEGER,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    CONSTRAINT "fk_ctm_customer_id" FOREIGN KEY ("customer_id")
                        REFERENCES "customers"("id") ON DELETE CASCADE,
                    CONSTRAINT "uk_ctm_customer_user" UNIQUE ("customer_id", "user_id"),
                    CONSTRAINT "chk_ctm_team_role" CHECK (
                        "team_role" IN ('primary', 'member', 'assistant')
                    )
                );

                CREATE INDEX IF NOT EXISTS "idx_ctm_customer_id" ON "customer_team_members"("customer_id");
                CREATE INDEX IF NOT EXISTS "idx_ctm_user_id" ON "customer_team_members"("user_id");
                CREATE INDEX IF NOT EXISTS "idx_ctm_active" ON "customer_team_members"("is_active");

                COMMENT ON TABLE "customer_team_members" IS '客户团队成员关联 - 支持多人协作跟进同一客户（18.4-D2）';
                COMMENT ON COLUMN "customer_team_members"."team_role" IS '团队角色：primary=主负责人，member=团队成员，assistant=协助人员';

                -- =====================================================
                -- 18.4-D3：客户数据共享表（带时效控制）
                -- =====================================================
                CREATE TABLE IF NOT EXISTS "customer_shares" (
                    "id" BIGSERIAL PRIMARY KEY,
                    "customer_id" INTEGER NOT NULL,
                    "shared_by_user_id" INTEGER NOT NULL,
                    "shared_by_user_name" VARCHAR(100),
                    "shared_to_user_id" INTEGER NOT NULL,
                    "shared_to_user_name" VARCHAR(100),
                    "permission" VARCHAR(20) NOT NULL DEFAULT 'view',
                    "status" VARCHAR(20) NOT NULL DEFAULT 'active',
                    "shared_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "expire_at" TIMESTAMPTZ,
                    "revoked_at" TIMESTAMPTZ,
                    "revoked_by" INTEGER,
                    "revoke_reason" TEXT,
                    "share_reason" TEXT,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    CONSTRAINT "fk_cs_customer_id" FOREIGN KEY ("customer_id")
                        REFERENCES "customers"("id") ON DELETE CASCADE,
                    CONSTRAINT "uk_cs_customer_to_user_active" UNIQUE ("customer_id", "shared_to_user_id", "status"),
                    CONSTRAINT "chk_cs_permission" CHECK (
                        "permission" IN ('view', 'edit', 'full')
                    ),
                    CONSTRAINT "chk_cs_status" CHECK (
                        "status" IN ('active', 'expired', 'revoked')
                    )
                );

                CREATE INDEX IF NOT EXISTS "idx_cs_customer_id" ON "customer_shares"("customer_id");
                CREATE INDEX IF NOT EXISTS "idx_cs_shared_to_user" ON "customer_shares"("shared_to_user_id");
                CREATE INDEX IF NOT EXISTS "idx_cs_shared_by_user" ON "customer_shares"("shared_by_user_id");
                CREATE INDEX IF NOT EXISTS "idx_cs_status" ON "customer_shares"("status");
                CREATE INDEX IF NOT EXISTS "idx_cs_expire_at" ON "customer_shares"("expire_at");

                COMMENT ON TABLE "customer_shares" IS '客户数据共享 - 支持时效控制和权限回收（18.4-D3）';
                COMMENT ON COLUMN "customer_shares"."permission" IS '共享权限：view=只读，edit=编辑，full=完全';
                COMMENT ON COLUMN "customer_shares"."status" IS '共享状态：active=生效中，expired=已过期，revoked=已撤销';
                COMMENT ON COLUMN "customer_shares"."expire_at" IS '共享过期时间，NULL 表示永久共享（建议设置时效）';
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
                DROP TABLE IF EXISTS "customer_shares";
                DROP TABLE IF EXISTS "customer_team_members";
                "#,
            )
            .await?;
        Ok(())
    }
}
