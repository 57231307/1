-- 回滚批次10：SaaS多租户 + 通知 + 报表 + 邮件 + OA
-- 创建时间: 2026-05-27
-- 描述: 删除SaaS多租户、通知、报表、邮件和OA相关表

-- 删除表（按依赖关系逆序）
DROP TABLE IF EXISTS "oa_announcement" CASCADE;
DROP TABLE IF EXISTS "email_logs" CASCADE;
DROP TABLE IF EXISTS "email_templates" CASCADE;
DROP TABLE IF EXISTS "report_subscriptions" CASCADE;
DROP TABLE IF EXISTS "report_templates" CASCADE;
DROP TABLE IF EXISTS "report_definition" CASCADE;
DROP TABLE IF EXISTS "user_notification_setting" CASCADE;
DROP TABLE IF EXISTS "notification_settings" CASCADE;
DROP TABLE IF EXISTS "notifications" CASCADE;
DROP TABLE IF EXISTS "tenant_invoices" CASCADE;
DROP TABLE IF EXISTS "tenant_usage" CASCADE;
DROP TABLE IF EXISTS "tenant_subscriptions" CASCADE;
DROP TABLE IF EXISTS "tenant_configs" CASCADE;
DROP TABLE IF EXISTS "tenant_users" CASCADE;
DROP TABLE IF EXISTS "tenants" CASCADE;
DROP TABLE IF EXISTS "tenant_plans" CASCADE;
