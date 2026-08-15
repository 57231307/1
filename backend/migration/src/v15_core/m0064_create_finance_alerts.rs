use sea_orm_migration::prelude::*;

// Batch 481 P0-B04：财务预警表
//
// 业务背景：财务风险需主动预警（应收超额 / 库存积压 / 现金流不足 / 预算超支 4 类），
// 触发通知管理层。预警状态：active → acknowledged → resolved / expired。
//
// 设计依据：V15 审计报告 batch-15 维度 17.5 缺陷 D1（P0）
// 关联文件：models/finance_alert.rs / services/finance_alert_service.rs /
//          handlers/finance_alert_handler.rs / routes/finance_alert.rs

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- 财务预警表（V15 P0-B04 创建）
                -- 4 类预警：ar_overdue(应收超额) / inventory_backlog(库存积压) / cash_flow_shortage(现金流不足) / budget_overrun(预算超支)
                CREATE TABLE IF NOT EXISTS "finance_alerts" (
                    "id" BIGSERIAL PRIMARY KEY,
                    "alert_no" VARCHAR(50) NOT NULL UNIQUE,
                    -- 预警类型与级别
                    "alert_type" VARCHAR(30) NOT NULL,
                    "alert_level" VARCHAR(20) NOT NULL,
                    -- 内容
                    "title" VARCHAR(200) NOT NULL,
                    "content" TEXT NOT NULL,
                    -- 关联目标
                    "target_module" VARCHAR(50),
                    "target_id" BIGINT,
                    -- 阈值与实际值
                    "threshold_value" DECIMAL(15,2),
                    "actual_value" DECIMAL(15,2),
                    "value_unit" VARCHAR(20),
                    -- 触发与处理
                    "triggered_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "triggered_by" INTEGER REFERENCES "users"("id") ON DELETE SET NULL,
                    "status" VARCHAR(20) NOT NULL DEFAULT 'active',
                    "acknowledged_at" TIMESTAMPTZ,
                    "acknowledged_by" INTEGER REFERENCES "users"("id") ON DELETE SET NULL,
                    "resolved_at" TIMESTAMPTZ,
                    "resolved_by" INTEGER REFERENCES "users"("id") ON DELETE SET NULL,
                    "resolve_note" TEXT,
                    "expired_at" TIMESTAMPTZ,
                    -- 通知关联
                    "notification_id" INTEGER,
                    "remark" TEXT,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    -- 约束
                    CONSTRAINT "chk_fa_alert_type" CHECK (
                        "alert_type" IN ('ar_overdue', 'inventory_backlog', 'cash_flow_shortage', 'budget_overrun')
                    ),
                    CONSTRAINT "chk_fa_alert_level" CHECK (
                        "alert_level" IN ('info', 'warning', 'critical')
                    ),
                    CONSTRAINT "chk_fa_status" CHECK (
                        "status" IN ('active', 'acknowledged', 'resolved', 'expired')
                    )
                );

                -- 索引（6 个）
                CREATE INDEX IF NOT EXISTS "idx_fa_alert_type" ON "finance_alerts"("alert_type");
                CREATE INDEX IF NOT EXISTS "idx_fa_alert_level" ON "finance_alerts"("alert_level");
                CREATE INDEX IF NOT EXISTS "idx_fa_status" ON "finance_alerts"("status");
                CREATE INDEX IF NOT EXISTS "idx_fa_triggered_at" ON "finance_alerts"("triggered_at");
                CREATE INDEX IF NOT EXISTS "idx_fa_target" ON "finance_alerts"("target_module", "target_id");
                CREATE INDEX IF NOT EXISTS "idx_fa_notification_id" ON "finance_alerts"("notification_id");

                COMMENT ON TABLE "finance_alerts" IS '财务预警表 - 4 类预警(ar_overdue/inventory_backlog/cash_flow_shortage/budget_overrun)主动通知管理层';
                COMMENT ON COLUMN "finance_alerts"."alert_type" IS '预警类型：ar_overdue(应收超额) / inventory_backlog(库存积压) / cash_flow_shortage(现金流不足) / budget_overrun(预算超支)';
                COMMENT ON COLUMN "finance_alerts"."alert_level" IS '预警级别：info / warning / critical';
                COMMENT ON COLUMN "finance_alerts"."status" IS '状态：active(活跃) / acknowledged(已确认) / resolved(已解决) / expired(已过期)';
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
                DROP TABLE IF EXISTS "finance_alerts";
                "#,
            )
            .await?;
        Ok(())
    }
}
