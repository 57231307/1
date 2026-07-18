-- 色卡发放数据迁移脚本（V15 P0-F13）
--
-- 用途：将旧 color_card_borrow_records 表中的借还记录迁移到新 color_card_issues 表
-- 关联审计：docs/audits/v15/batch-09/audit-report.md §10.7-1（P0）
-- 关联 spec：docs/superpowers/specs/2026-06-16-color-card-design.md §3.4
--
-- 执行前提：
--   1. m0057_create_color_card_issues_and_stock_fields 已执行（color_card_issues 表已存在）
--   2. color_card_borrow_records 旧表存在且有数据
--
-- 字段映射：
--   id                 → id（保留原主键，便于追溯）
--   color_card_id      → color_card_id（不变）
--   customer_id        → customer_id（不变）
--   borrowed_by        → issued_by（重命名）
--   borrowed_at        → issued_at（重命名）
--   expected_return_at → expected_return_date（TIMESTAMPTZ → DATE 转换）
--   actual_return_at   → actual_return_date（TIMESTAMPTZ → DATE 转换）
--   status             → status（'borrowed' → 'issued'，其它状态同名映射）
--   purpose            → purpose（不变）
--   notes              → remark（重命名）
--   compensation_amount → compensation_amount（不变）
--   NULL               → returned_by（旧表无对应字段，设为 NULL）
--   NULL               → dye_lot_no（旧表无对应字段，设为 NULL）
--   created_at         → created_at（不变）
--   updated_at         → updated_at（不变）
--   false              → is_deleted（新字段，默认 false）
--   tenant_id          → （丢弃，多租户残留，V15 已移除租户概念）
--
-- 执行策略：DBA 在维护窗口手动执行，执行前请备份 color_card_borrow_records 表
-- 幂等性：使用 WHERE NOT EXISTS 避免重复迁移，可安全多次执行

BEGIN;

-- 1. 数据迁移：borrow_records → issues（仅迁移未迁移的记录）
INSERT INTO "color_card_issues" (
    "id",
    "color_card_id",
    "customer_id",
    "issue_qty",
    "issued_by",
    "issued_at",
    "expected_return_date",
    "actual_return_date",
    "status",
    "purpose",
    "remark",
    "compensation_amount",
    "returned_by",
    "dye_lot_no",
    "created_at",
    "updated_at",
    "is_deleted"
)
SELECT
    b."id",
    b."color_card_id",
    b."customer_id",
    1 AS "issue_qty",  -- 旧表无 issue_qty 字段，按色卡单张发放的语义默认为 1
    b."borrowed_by",
    b."borrowed_at",
    -- TIMESTAMPTZ → DATE：截断到日期粒度（V15 发放记录使用 DATE 类型）
    b."expected_return_at"::date,
    b."actual_return_at"::date,
    CASE b."status"
        WHEN 'borrowed' THEN 'issued'    -- 借出中 → 发放中
        WHEN 'returned' THEN 'returned'  -- 已归还（同名）
        WHEN 'lost' THEN 'lost'          -- 遗失（同名）
        WHEN 'damaged' THEN 'damaged'    -- 损坏（同名）
        ELSE 'issued'                    -- 未知状态保守映射为发放中
    END AS "status",
    b."purpose",
    b."notes",
    b."compensation_amount",
    NULL AS "returned_by",
    NULL AS "dye_lot_no",
    b."created_at",
    b."updated_at",
    false AS "is_deleted"
FROM "color_card_borrow_records" b
WHERE NOT EXISTS (
    SELECT 1 FROM "color_card_issues" i WHERE i."id" = b."id"
);

-- 2. 同步色卡库存字段：根据迁移后的 issues 记录回填 issued_quantity
--    仅对迁移期间产生 issued 状态记录的色卡进行回填
UPDATE "color_cards" c
SET "issued_quantity" = COALESCE((
        SELECT COUNT(*)::int
        FROM "color_card_issues" i
        WHERE i."color_card_id" = c."id"
          AND i."status" = 'issued'
          AND i."is_deleted" = false
    ), 0),
    "updated_at" = NOW()
WHERE EXISTS (
    SELECT 1 FROM "color_card_issues" i
    WHERE i."color_card_id" = c."id"
);

-- 3. 数据校验：迁移后记录数应与原表一致（DBA 人工核对）
--    以下 SELECT 仅用于校验，不会修改数据
-- SELECT
--     (SELECT COUNT(*) FROM "color_card_borrow_records") AS legacy_count,
--     (SELECT COUNT(*) FROM "color_card_issues") AS new_count;

COMMIT;

-- 4. 旧表保留说明（不在本脚本中执行 DROP/RENAME）
--    旧表 color_card_borrow_records 应保留至少 30 天，供回滚和审计追溯
--    30 天后由 DBA 执行 RENAME 改为 color_card_borrow_records_legacy：
--    ALTER TABLE "color_card_borrow_records" RENAME TO "color_card_borrow_records_legacy";
