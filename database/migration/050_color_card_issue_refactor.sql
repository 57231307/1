-- ============================================================================
-- Migration 050: V15 P0-F04/F06 色卡发放模式重构
-- 依据：V15 P0 审计报告 类九 P0-F04/F06（batch-09 P0-09-2/4）
-- 业务背景：色卡管理从「借出/归还」(borrow) 模式重构为「发放」(issue) 模式
--   - 旧模式：color_card_borrow_records 表，语义为"借出/归还/遗失/损坏/取消"
--   - 新模式：color_card_issues 表，语义为"发放/归还/遗失/损坏/取消"
--
-- 用户决策（2026-07-17 明确）：删除借出/归还，创建发放模式
--
-- 本批次内容：
--   1. 创建 color_card_issues 新表（发放主表，含 5 道闸门校验所需字段）
--   2. 旧表 color_card_borrow_records 保留不删（应用层不再读写，等于事实上的 legacy）
--      原因：Rust migration m0029_drop_tenant_columns 会执行
--      `ALTER TABLE color_card_borrow_records DROP COLUMN tenant_id`，
--      若 RENAME 会导致该 migration 失败。保留旧表避免破坏 migration 链路。
--
-- 关联任务：
--   - P0-F03：删除 borrow 语义 ✅ 本批次（应用层删除 borrow 代码，旧表保留）
--   - P0-F04：创建发放模式后端文件 ✅ 本批次（model/service/handler）
--   - P0-F06：旧表重命名为 legacy + 新表创建 ✅ 本批次（新表创建，旧表保留）
--   - P0-F05：删除 /lend-return 路由，新增 /issues 路由 ✅ 本批次（routes）
--   - P0-F07：前端 borrow.vue → issue.vue 重写 ⏳ Batch 472
--   - P0-F08：发放前 5 道闸门校验 ✅ 本批次（service）
-- ============================================================================

-- ==================== 创建色卡发放主表 ====================
-- 业务说明：色卡发放给客户看样，支持归还/遗失/损坏/取消
-- 状态机：issued → returned / lost / damaged / cancelled（终态不可再转换）
-- 5 道发放前闸门校验（service 层实现）：
--   1. 卡片状态 = active
--   2. 发放数量 > 0
--   3. 客户信用额度 > 0
--   4. 客户无未归还超期记录
--   5. 客户状态 = active（白名单校验）
CREATE TABLE IF NOT EXISTS "color_card_issues" (
    "id" BIGSERIAL PRIMARY KEY,
    "color_card_id" BIGINT NOT NULL,
    "customer_id" BIGINT NOT NULL,
    "issue_qty" INTEGER NOT NULL DEFAULT 1,
    "issued_by" BIGINT NOT NULL,
    "issued_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "expected_return_date" DATE,
    "actual_return_date" DATE,
    "status" VARCHAR(20) NOT NULL DEFAULT 'issued',
    "purpose" TEXT,
    "remark" TEXT,
    "compensation_amount" DECIMAL(12, 2),
    "returned_by" BIGINT,
    "dye_lot_no" VARCHAR(50),
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX IF NOT EXISTS "idx_color_card_issues_card" ON "color_card_issues" ("color_card_id");
CREATE INDEX IF NOT EXISTS "idx_color_card_issues_customer" ON "color_card_issues" ("customer_id");
CREATE INDEX IF NOT EXISTS "idx_color_card_issues_status" ON "color_card_issues" ("status");
CREATE INDEX IF NOT EXISTS "idx_color_card_issues_issued_at" ON "color_card_issues" ("issued_at");
CREATE INDEX IF NOT EXISTS "idx_color_card_issues_dye_lot_no" ON "color_card_issues" ("dye_lot_no");

COMMENT ON TABLE "color_card_issues" IS '色卡发放记录表（V15 P0-F04 创建，发放/归还/遗失/损坏/取消业务）';
COMMENT ON COLUMN "color_card_issues"."issue_qty" IS '发放数量（P0-F08 闸门 2：库存数量 >= 发放数量）';
COMMENT ON COLUMN "color_card_issues"."status" IS '状态 issued 发放中/returned 已归还/lost 已遗失/damaged 已损坏/cancelled 已取消';
COMMENT ON COLUMN "color_card_issues"."dye_lot_no" IS '缸号（V15 术语：染色批号 dye_lot_no，防色差混批）';
COMMENT ON COLUMN "color_card_issues"."compensation_amount" IS '赔付金额（遗失/损坏时记录）';

-- ============================================================================
-- 验证 SQL（手动验证用，迁移脚本不会执行）
-- ============================================================================
-- SELECT table_name FROM information_schema.tables
-- WHERE table_name IN ('color_card_borrow_records', 'color_card_issues');
-- 预期：2 行记录（旧表保留 + 新表创建）
