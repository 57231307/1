use sea_orm_migration::prelude::*;

// V15 P1 batch-16 缺陷 7.2/7.3/8.3/8.4 修复：
//
// 7.2: OA 公告可见性控制 — oa_announcement 表新增 visibility_scope + visible_scope_config
//      支持 ALL/DEPT/ROLE/CUSTOM 四种可见性范围
// 7.3: 用户行为采集隐私合规 — 新增 user_consents 表（记录用户同意/退出追踪状态）
// 8.3: 页面浏览 90 天保留策略 — 新增 page_view_daily_summary 表（归档汇总目标）
// 8.4: 用户行为 90 天保留策略 — 新增 user_behavior_daily_summary 表（归档汇总目标）
//
// 关联文件：
//   - models/oa_announcement.rs（新增 visibility_scope/visible_scope_config 字段）
//   - models/user_consent.rs（新增 user_consents 表模型）
//   - services/oa_announcement_service.rs（list 按 visibility_scope 过滤）
//   - services/user_consent_service.rs（新增 consent 服务）
//   - services/tracking_service.rs（record_behavior 接入脱敏）
//   - services/tracking_cleanup_service.rs（新增 90 天归档清理服务）
//   - handlers/tracking_handler.rs（追踪前校验 consent + 新增 consent/opt-out 端点）

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
                -- P1 batch-16 缺陷 7.2：OA 公告可见性控制
                -- ============================================================

                -- visibility_scope：可见性范围枚举
                --   ALL=全员可见（默认）
                --   DEPT=指定部门可见（visible_scope_config = {"department_ids": [1,2,3]}）
                --   ROLE=指定角色可见（visible_scope_config = {"role_ids": [1,2,3]}）
                --   CUSTOM=自定义用户列表（visible_scope_config = {"user_ids": [1,2,3]}）
                ALTER TABLE "oa_announcement" ADD COLUMN IF NOT EXISTS "visibility_scope" VARCHAR(20) NOT NULL DEFAULT 'ALL';
                ALTER TABLE "oa_announcement" ADD COLUMN IF NOT EXISTS "visible_scope_config" JSONB;

                COMMENT ON COLUMN "oa_announcement"."visibility_scope" IS '可见性范围：ALL=全员/DEPT=指定部门/ROLE=指定角色/CUSTOM=自定义用户';
                COMMENT ON COLUMN "oa_announcement"."visible_scope_config" IS '可见性配置 JSON：{"department_ids":[...]}/{"role_ids":[...]}/{"user_ids":[...]}';

                -- ============================================================
                -- P1 batch-16 缺陷 7.3：用户行为采集隐私合规
                -- ============================================================

                -- user_consents：用户隐私同意记录表
                -- 每次 consent 变更新增一条记录，保留审计轨迹
                CREATE TABLE IF NOT EXISTS "user_consents" (
                    "id" SERIAL PRIMARY KEY,
                    "user_id" INTEGER NOT NULL,
                    "consent_type" VARCHAR(50) NOT NULL,
                    "consent_given" BOOLEAN NOT NULL,
                    "consent_text_version" VARCHAR(20),
                    "consented_at" TIMESTAMP NOT NULL DEFAULT NOW(),
                    "revoked_at" TIMESTAMP,
                    "ip_address" VARCHAR(64),
                    "user_agent" VARCHAR(512),
                    "created_at" TIMESTAMP NOT NULL DEFAULT NOW()
                );

                -- 单用户单类型当前最新状态查询索引
                CREATE INDEX IF NOT EXISTS "idx_user_consents_user_type"
                    ON "user_consents"("user_id", "consent_type", "consented_at" DESC);

                -- 约束：consent_type 必须为预定义类型
                ALTER TABLE "user_consents" DROP CONSTRAINT IF EXISTS "chk_user_consents_consent_type";
                ALTER TABLE "user_consents" ADD CONSTRAINT "chk_user_consents_consent_type"
                    CHECK ("consent_type" IN ('behavior_tracking', 'page_view_tracking', 'cookie_usage', 'marketing_email'));

                COMMENT ON TABLE "user_consents" IS '用户隐私同意记录表（GDPR/个人信息保护法合规）';
                COMMENT ON COLUMN "user_consents"."consent_type" IS '同意类型：behavior_tracking/page_view_tracking/cookie_usage/marketing_email';
                COMMENT ON COLUMN "user_consents"."consent_given" IS '是否同意：true=同意采集，false=退出';
                COMMENT ON COLUMN "user_consents"."consent_text_version" IS '隐私政策文本版本号（如 v1.0）';

                -- ============================================================
                -- P1 batch-16 缺陷 8.3/8.4：用户行为日志 90 天保留策略归档表
                -- ============================================================

                -- page_view_daily_summary：按 path + date 聚合的页面访问汇总
                CREATE TABLE IF NOT EXISTS "page_view_daily_summary" (
                    "stat_date" DATE NOT NULL,
                    "path" VARCHAR(2048) NOT NULL,
                    "total_views" BIGINT NOT NULL DEFAULT 0,
                    "unique_sessions" BIGINT NOT NULL DEFAULT 0,
                    "unique_users" BIGINT NOT NULL DEFAULT 0,
                    "created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
                    PRIMARY KEY ("stat_date", "path")
                );

                COMMENT ON TABLE "page_view_daily_summary" IS '页面访问日聚合表（page_views 90 天归档汇总目标）';

                -- user_behavior_daily_summary：按 event_type + date 聚合的行为汇总
                CREATE TABLE IF NOT EXISTS "user_behavior_daily_summary" (
                    "stat_date" DATE NOT NULL,
                    "event_type" VARCHAR(128) NOT NULL,
                    "total_count" BIGINT NOT NULL DEFAULT 0,
                    "unique_users" BIGINT NOT NULL DEFAULT 0,
                    "unique_sessions" BIGINT NOT NULL DEFAULT 0,
                    "created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
                    PRIMARY KEY ("stat_date", "event_type")
                );

                COMMENT ON TABLE "user_behavior_daily_summary" IS '用户行为日聚合表（user_behaviors 90 天归档汇总目标）';
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
                DROP TABLE IF EXISTS "user_behavior_daily_summary";
                DROP TABLE IF EXISTS "page_view_daily_summary";
                DROP TABLE IF EXISTS "user_consents";
                ALTER TABLE "oa_announcement" DROP COLUMN IF EXISTS "visible_scope_config";
                ALTER TABLE "oa_announcement" DROP COLUMN IF EXISTS "visibility_scope";
                "#,
            )
            .await?;
        Ok(())
    }
}
