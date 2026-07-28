use sea_orm_migration::prelude::*;

// V15 P1 batch-16 缺陷 1.1/4.1 修复：
//
// 1.1: 报表模板版本管理 — 新增 report_template_versions 表
//      存储 update 前的快照，配合 ReportTemplateService::list_versions / rollback_version
//      实现 GET /reports/templates/:id/versions 和 POST /reports/templates/:id/rollback/:version
//
// 4.1: 仪表板自定义卡片持久化 — 新增 dashboard_layouts 表
//      存储 user_id + card_config JSON，支持用户按角色定制关注的卡片
//
// 关联文件：
//   - models/report_template_version.rs（新增历史版本模型）
//   - models/dashboard_layout.rs（已有布局模型，补齐迁移）
//   - services/report_template_service.rs（list_versions / rollback_version）
//   - handlers/report_enhanced_handler.rs（list_versions / rollback_version 接口）
//   - handlers/dashboard_handler.rs（get_dashboard_layout / save_dashboard_layout）
//   - routes/analytics.rs（挂载 versions/rollback 路由）
//   - routes/system.rs（挂载 dashboard layout 路由）

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- ============================================================
                -- P1 batch-16 缺陷 1.1：报表模板版本管理（report_template_versions）
                -- ============================================================

                CREATE TABLE IF NOT EXISTS "report_template_versions" (
                    "id" SERIAL PRIMARY KEY,
                    "template_id" INTEGER NOT NULL,
                    "version" INTEGER NOT NULL,
                    "name" VARCHAR(100) NOT NULL,
                    "code" VARCHAR(50) NOT NULL,
                    "report_type" VARCHAR(50) NOT NULL,
                    "category" VARCHAR(50),
                    "data_source" VARCHAR(100),
                    "columns" JSONB NOT NULL,
                    "filters" JSONB,
                    "parameters" JSONB,
                    "supported_formats" JSONB,
                    "sort_by" VARCHAR(100),
                    "sort_order" VARCHAR(10),
                    "data_source_sql" TEXT,
                    "description" TEXT,
                    "is_public" BOOLEAN NOT NULL DEFAULT false,
                    "required_permission" VARCHAR(100),
                    "snapshot_by" INTEGER NOT NULL,
                    "snapshot_at" TIMESTAMP NOT NULL DEFAULT NOW()
                );

                CREATE INDEX IF NOT EXISTS "idx_report_template_versions_template_id"
                    ON "report_template_versions"("template_id", "version" DESC);

                COMMENT ON TABLE "report_template_versions" IS '报表模板历史版本表（每次 update 前写入快照，支持回滚）';
                COMMENT ON COLUMN "report_template_versions"."template_id" IS '关联的报表模板 ID';
                COMMENT ON COLUMN "report_template_versions"."version" IS '版本号（与 report_templates.version 对应）';
                COMMENT ON COLUMN "report_template_versions"."snapshot_by" IS '执行 update 的用户 ID（快照创建者）';
                COMMENT ON COLUMN "report_template_versions"."snapshot_at" IS '快照时间（update 前写入）';

                -- ============================================================
                -- P1 batch-16 缺陷 1.1/1.2：补齐 report_templates 表的 version / required_permission 字段
                -- ============================================================
                ALTER TABLE "report_templates"
                    ADD COLUMN IF NOT EXISTS "version" INTEGER NOT NULL DEFAULT 1;
                ALTER TABLE "report_templates"
                    ADD COLUMN IF NOT EXISTS "required_permission" VARCHAR(100);
                COMMENT ON COLUMN "report_templates"."version" IS '模板版本号（每次 update 前递增，支持回滚）';
                COMMENT ON COLUMN "report_templates"."required_permission" IS '必需权限码（如 report:sales:view），为空表示按 is_public/created_by 过滤';

                -- ============================================================
                -- P1 batch-16 缺陷 2.3：补齐 report_subscriptions 表的重试字段
                -- ============================================================
                ALTER TABLE "report_subscriptions"
                    ADD COLUMN IF NOT EXISTS "retry_count" INTEGER NOT NULL DEFAULT 0;
                ALTER TABLE "report_subscriptions"
                    ADD COLUMN IF NOT EXISTS "max_retries" INTEGER NOT NULL DEFAULT 3;
                ALTER TABLE "report_subscriptions"
                    ADD COLUMN IF NOT EXISTS "next_retry_at" TIMESTAMP;
                COMMENT ON COLUMN "report_subscriptions"."retry_count" IS '当前重试次数（成功后清零）';
                COMMENT ON COLUMN "report_subscriptions"."max_retries" IS '最大重试次数（默认 3，超过即转入死信状态）';
                COMMENT ON COLUMN "report_subscriptions"."next_retry_at" IS '下次重试时间（按指数退避：1min/5min/30min）';

                -- ============================================================
                -- P1 batch-16 缺陷 4.1：仪表板自定义卡片持久化（dashboard_layouts）
                -- ============================================================

                CREATE TABLE IF NOT EXISTS "dashboard_layouts" (
                    "id" SERIAL PRIMARY KEY,
                    "user_id" INTEGER NOT NULL UNIQUE,
                    "card_config" JSONB NOT NULL,
                    "is_default" BOOLEAN NOT NULL DEFAULT false,
                    "created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
                );

                CREATE INDEX IF NOT EXISTS "idx_dashboard_layouts_user_id"
                    ON "dashboard_layouts"("user_id");

                COMMENT ON TABLE "dashboard_layouts" IS '用户仪表板布局配置表（卡片顺序/可见性/尺寸）';
                COMMENT ON COLUMN "dashboard_layouts"."user_id" IS '用户 ID（每个用户独立布局）';
                COMMENT ON COLUMN "dashboard_layouts"."card_config" IS '卡片配置 JSON（卡片顺序、可见性、尺寸等）';
                COMMENT ON COLUMN "dashboard_layouts"."is_default" IS '是否默认布局（true 时其他用户初始可见）';
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
                DROP TABLE IF EXISTS "dashboard_layouts";
                DROP TABLE IF EXISTS "report_template_versions";
                "#,
            )
            .await?;
        Ok(())
    }
}
