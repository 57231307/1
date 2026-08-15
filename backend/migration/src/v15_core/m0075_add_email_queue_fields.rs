use sea_orm_migration::prelude::*;

// V15 P1 batch-16 缺陷 6.1/6.2/6.3 修复：
// 为 email_logs 表新增异步队列调度所需字段：
// - next_retry_at：下次重试时间（指数退避：1min/5min/30min）
// - attachments：附件 JSON（[{filename, content_base64, content_type}]）
// - html_content：HTML 正文（与 body 区分，body 保留为兼容字段）
// - text_content：纯文本正文
//
// 关联文件：
//   - models/email_log.rs（新增对应字段）
//   - services/email_log_service.rs（increment_retry 接入指数退避 + 死信上限）
//   - services/email_queue_worker.rs（新增后台 worker 扫描 PENDING 重试）
//   - handlers/email_handler.rs（send_email 改为入队后立即返回）
//   - services/email_service.rs（三个 provider 接入附件发送）

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
                -- P1 batch-16 缺陷 6.1/6.2/6.3：邮件异步队列 + 重试 + 附件
                -- ============================================================

                -- 1. next_retry_at：下次重试时间（指数退避调度使用）
                ALTER TABLE "email_logs" ADD COLUMN IF NOT EXISTS "next_retry_at" TIMESTAMP;

                -- 2. attachments：附件 JSON 数组
                --    格式：[{"filename": "report.pdf", "content_base64": "...", "content_type": "application/pdf"}]
                ALTER TABLE "email_logs" ADD COLUMN IF NOT EXISTS "attachments" JSONB;

                -- 3. html_content / text_content：区分 HTML 与纯文本正文（原 body 字段保留兼容）
                ALTER TABLE "email_logs" ADD COLUMN IF NOT EXISTS "html_content" TEXT;
                ALTER TABLE "email_logs" ADD COLUMN IF NOT EXISTS "text_content" TEXT;

                -- 索引：扫描 PENDING + next_retry_at 邮件的高频查询
                CREATE INDEX IF NOT EXISTS "idx_email_logs_pending_retry"
                    ON "email_logs"("status", "next_retry_at", "retry_count")
                    WHERE "status" = 'PENDING';

                COMMENT ON COLUMN "email_logs"."next_retry_at" IS '下次重试时间（指数退避：1min/5min/30min，NULL 表示立即可重试）';
                COMMENT ON COLUMN "email_logs"."attachments" IS '附件 JSON 数组：[{filename, content_base64, content_type}]';
                COMMENT ON COLUMN "email_logs"."html_content" IS 'HTML 正文（与 body 区分，body 保留为兼容字段）';
                COMMENT ON COLUMN "email_logs"."text_content" IS '纯文本正文';
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
                DROP INDEX IF EXISTS "idx_email_logs_pending_retry";
                ALTER TABLE "email_logs" DROP COLUMN IF EXISTS "text_content";
                ALTER TABLE "email_logs" DROP COLUMN IF EXISTS "html_content";
                ALTER TABLE "email_logs" DROP COLUMN IF EXISTS "attachments";
                ALTER TABLE "email_logs" DROP COLUMN IF EXISTS "next_retry_at";
                "#,
            )
            .await?;
        Ok(())
    }
}
