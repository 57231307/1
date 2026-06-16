-- 回滚 color_card_borrow_records 表
DROP INDEX IF EXISTS "idx_borrow_borrower";
DROP INDEX IF EXISTS "idx_borrow_borrowed_at";
DROP INDEX IF EXISTS "idx_borrow_tenant";
DROP INDEX IF EXISTS "idx_borrow_status";
DROP INDEX IF EXISTS "idx_borrow_customer";
DROP INDEX IF EXISTS "idx_borrow_card";
DROP TABLE IF EXISTS "color_card_borrow_records";
