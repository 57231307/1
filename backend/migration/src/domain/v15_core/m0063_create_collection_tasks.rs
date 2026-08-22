use sea_orm_migration::prelude::*;

// Batch 481 P0-B03：催收任务管理表
//
// 业务背景：逾期应收需按账龄自动生成催收任务，分配给销售员，记录催收结果与下次行动。
// 催收任务类型：phone(电话) / visit(上门) / email(邮件) / letter(函件)
//
// 设计依据：V15 审计报告 batch-15 维度 17.3 缺陷 D3（P0）
// 关联文件：models/collection_task.rs / services/collection_task_service.rs /
//          handlers/collection_task_handler.rs / routes/collection_task.rs

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- 催收任务表（V15 P0-B03 创建）
                -- 按账龄自动生成催收任务，分配给销售员，记录催收结果
                CREATE TABLE IF NOT EXISTS "collection_tasks" (
                    "id" BIGSERIAL PRIMARY KEY,
                    "task_no" VARCHAR(50) NOT NULL UNIQUE,
                    -- 业务关联
                    "customer_id" BIGINT NOT NULL REFERENCES "customers"("id") ON DELETE RESTRICT,
                    "ar_invoice_id" INTEGER REFERENCES "ar_invoices"("id") ON DELETE SET NULL,
                    -- 任务内容
                    "overdue_amount" DECIMAL(15,2) NOT NULL CHECK ("overdue_amount" >= 0),
                    "overdue_days" INTEGER NOT NULL CHECK ("overdue_days" >= 0),
                    "task_type" VARCHAR(20) NOT NULL,
                    "priority" VARCHAR(20) NOT NULL DEFAULT 'normal',
                    "due_date" DATE NOT NULL,
                    -- 分配
                    "assigned_to" INTEGER NOT NULL REFERENCES "users"("id") ON DELETE RESTRICT,
                    "assigned_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "assigned_by" INTEGER REFERENCES "users"("id") ON DELETE SET NULL,
                    -- 执行
                    "status" VARCHAR(20) NOT NULL DEFAULT 'pending',
                    "contact_result" TEXT,
                    "contact_at" TIMESTAMPTZ,
                    "next_action_date" DATE,
                    "next_action_type" VARCHAR(20),
                    -- 扩展
                    "remark" TEXT,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    -- 约束
                    CONSTRAINT "chk_ct_task_type" CHECK (
                        "task_type" IN ('phone', 'visit', 'email', 'letter')
                    ),
                    CONSTRAINT "chk_ct_priority" CHECK (
                        "priority" IN ('low', 'normal', 'high', 'urgent')
                    ),
                    CONSTRAINT "chk_ct_status" CHECK (
                        "status" IN ('pending', 'in_progress', 'completed', 'cancelled')
                    ),
                    CONSTRAINT "chk_ct_next_action_type" CHECK (
                        "next_action_type" IS NULL OR "next_action_type" IN ('phone', 'visit', 'email', 'letter')
                    )
                );

                -- 索引（6 个）
                CREATE INDEX IF NOT EXISTS "idx_ct_customer_id" ON "collection_tasks"("customer_id");
                CREATE INDEX IF NOT EXISTS "idx_ct_ar_invoice_id" ON "collection_tasks"("ar_invoice_id");
                CREATE INDEX IF NOT EXISTS "idx_ct_assigned_to" ON "collection_tasks"("assigned_to");
                CREATE INDEX IF NOT EXISTS "idx_ct_status" ON "collection_tasks"("status");
                CREATE INDEX IF NOT EXISTS "idx_ct_due_date" ON "collection_tasks"("due_date");
                CREATE INDEX IF NOT EXISTS "idx_ct_priority" ON "collection_tasks"("priority");

                COMMENT ON TABLE "collection_tasks" IS '催收任务表 - 按账龄自动生成,分配销售员,记录催收结果';
                COMMENT ON COLUMN "collection_tasks"."task_type" IS '催收类型：phone(电话) / visit(上门) / email(邮件) / letter(函件)';
                COMMENT ON COLUMN "collection_tasks"."priority" IS '优先级：low / normal / high / urgent（按逾期天数自动评估）';
                COMMENT ON COLUMN "collection_tasks"."status" IS '状态：pending(待处理) / in_progress(处理中) / completed(已完成) / cancelled(已取消)';
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
                DROP TABLE IF EXISTS "collection_tasks";
                "#,
            )
            .await?;
        Ok(())
    }
}
