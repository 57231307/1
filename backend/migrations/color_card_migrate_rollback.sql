-- V15 P2 类九 10.7-3：色卡发放迁移回滚方案
-- 如果上线失败，执行以下操作回滚：

-- 1. 删除新表
DROP TABLE IF EXISTS "color_card_issue_record";

-- 2. 恢复旧表名（如果旧表曾被重命名）
-- ALTER TABLE IF EXISTS "color_card_borrow_record_legacy" RENAME TO "color_card_borrow_records";

-- 3. 删除新索引
DROP INDEX IF EXISTS "idx_issue_status";
DROP INDEX IF EXISTS "idx_issue_customer";
DROP INDEX IF EXISTS "idx_issue_expiry";