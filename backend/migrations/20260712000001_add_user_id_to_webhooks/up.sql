-- 批次 320 v9 中风险修复 M-4：webhooks 表添加 user_id 列（IDOR 防护）
-- 原 webhook 端点仅校验用户登录，未校验 webhook 是否属于该用户，存在 IDOR 漏洞：
-- 用户 A 可删除/修改/重试用户 B 的 webhook。
--
-- 新增 user_id 列（可空），用于记录 webhook 所有者：
-- - NULL：系统级 webhook（历史数据，所有认证用户可访问，向后兼容）
-- - 非 NULL：用户私有 webhook，仅所有者可操作
--
-- 同时添加 user_id 索引，加速按用户过滤查询。

ALTER TABLE "webhooks" ADD COLUMN IF NOT EXISTS "user_id" INTEGER;

CREATE INDEX IF NOT EXISTS "idx_webhooks_user" ON "webhooks" ("user_id");

COMMENT ON COLUMN "webhooks"."user_id" IS 'Webhook 所有者用户 ID（批次 320 M-4 修复：IDOR 防护；NULL 表示系统级 webhook）';
