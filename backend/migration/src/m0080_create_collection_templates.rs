use sea_orm_migration::prelude::*;

// V15 P1 17.3-D5：催收模板管理表
//
// 业务背景：催收任务需要标准化话术支持，按催收类型(phone/visit/email/letter)
// 和逾期阶段(early/middle/late)配置模板，创建任务时自动匹配并填充话术。
//
// 设计依据：V15 审计报告 batch-15 维度 17.3 缺陷 D5（P1）
// 关联文件：models/collection_template.rs / services/collection_task_service.rs

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- 催收模板表（V15 P1 17.3-D5 创建）
                -- 按催收类型与逾期阶段配置标准化话术
                CREATE TABLE IF NOT EXISTS "collection_templates" (
                    "id" BIGSERIAL PRIMARY KEY,
                    "name" VARCHAR(100) NOT NULL,
                    "task_type" VARCHAR(20) NOT NULL,
                    "overdue_stage" VARCHAR(20) NOT NULL DEFAULT 'all',
                    "title" VARCHAR(200),
                    "content" TEXT NOT NULL,
                    "is_enabled" BOOLEAN NOT NULL DEFAULT TRUE,
                    "sort_order" INTEGER NOT NULL DEFAULT 0,
                    "remark" TEXT,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    CONSTRAINT "uk_ct_name" UNIQUE ("name"),
                    CONSTRAINT "chk_ct_task_type" CHECK (
                        "task_type" IN ('phone', 'visit', 'email', 'letter')
                    ),
                    CONSTRAINT "chk_ct_overdue_stage" CHECK (
                        "overdue_stage" IN ('early', 'middle', 'late', 'all')
                    )
                );

                -- 索引
                CREATE INDEX IF NOT EXISTS "idx_ctpl_task_type" ON "collection_templates"("task_type");
                CREATE INDEX IF NOT EXISTS "idx_ctpl_overdue_stage" ON "collection_templates"("overdue_stage");
                CREATE INDEX IF NOT EXISTS "idx_ctpl_is_enabled" ON "collection_templates"("is_enabled");

                COMMENT ON TABLE "collection_templates" IS '催收模板表 - 按类型/阶段配置标准化话术';
                COMMENT ON COLUMN "collection_templates"."task_type" IS '催收类型：phone(电话) / visit(上门) / email(邮件) / letter(函件)';
                COMMENT ON COLUMN "collection_templates"."overdue_stage" IS '适用逾期阶段：early(0-30天) / middle(31-90天) / late(90+天) / all(全部)';

                -- 预置 4 套默认模板（按催收类型）
                INSERT INTO "collection_templates" ("name", "task_type", "overdue_stage", "title", "content", "is_enabled", "sort_order", "remark") VALUES
                ('电话催收-早期模板', 'phone', 'early', NULL, '您好，我是XX公司的财务专员，您有一笔账款已逾期{overdue_days}天，金额{overdue_amount}元，请尽快安排付款，谢谢配合。', TRUE, 1, '默认电话催收话术-早期'),
                ('上门催收-中期模板', 'visit', 'middle', NULL, '尊敬的客户，您司账款已逾期{overdue_days}天，累计欠款{overdue_amount}元。我司将安排专人上门沟通，请配合核实并安排付款。', TRUE, 1, '默认上门催收话术-中期'),
                ('邮件催收-通用模板', 'email', 'all', '【账款催收通知】逾期{overdue_days}天 - 金额{overdue_amount}元', '尊敬的客户：\n\n经核对，您司于我司的应收账款已逾期{overdue_days}天，未付金额{overdue_amount}元。\n\n请于收到本邮件后7个工作日内安排付款，如有疑问请及时联系我司财务部。\n\n此致\nXX公司财务部', TRUE, 1, '默认邮件催收话术-通用'),
                ('函件催收-晚期模板', 'letter', 'late', '关于催收逾期账款的函', '致：{customer_name}\n\n经我司财务部门核对，截至发函日，贵司尚欠我司货款{overdue_amount}元，已逾期{overdue_days}天。\n\n鉴于逾期时间较长，特发此函正式催收。请贵司于收到本函后15日内付清上述款项，逾期未付我司将依法采取进一步措施。\n\n特此函告。\n\nXX公司\n{date}', TRUE, 1, '默认函件催收话术-晚期')
                ON CONFLICT ("name") DO NOTHING;
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
                DROP TABLE IF EXISTS "collection_templates";
                "#,
            )
            .await?;
        Ok(())
    }
}
