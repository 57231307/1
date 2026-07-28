-- 回滚：移除 agreed_to_terms_at 列
ALTER TABLE "users" DROP COLUMN IF EXISTS "agreed_to_terms_at";
