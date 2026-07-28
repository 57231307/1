-- P1-08-1：users 表添加 agreed_to_terms_at 列（用户协议同意时间）
-- 用于记录用户是否已同意用户协议和隐私政策，满足《个人信息保护法》第 14 条同意要求。
-- 管理员创建的用户首次登录时需确认同意，自助注册用户注册时即设置。

ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "agreed_to_terms_at" TIMESTAMP WITH TIME ZONE;

COMMENT ON COLUMN "users"."agreed_to_terms_at" IS '用户协议/隐私政策同意时间（P1-08-1 法律合规，None 表示未同意）';
