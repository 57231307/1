-- 批次 251 v14 中风险修复：webhooks 表添加 last_payload + last_event 列
-- 原 webhook 发送时 payload 仅存内存，发送失败后丢失，retry 重构假 payload 无法重投原始数据。
-- 新增 last_payload（原始业务负载）+ last_event（原始事件类型）列，支持真实重试。

ALTER TABLE "webhooks" ADD COLUMN IF NOT EXISTS "last_payload" TEXT;
ALTER TABLE "webhooks" ADD COLUMN IF NOT EXISTS "last_event" VARCHAR(100);

COMMENT ON COLUMN "webhooks"."last_payload" IS '最后一次发送的原始业务负载（批次 251 修复：retry 重投原始数据用）';
COMMENT ON COLUMN "webhooks"."last_event" IS '最后一次发送的事件类型（批次 251 修复：retry 重投原始事件用）';
