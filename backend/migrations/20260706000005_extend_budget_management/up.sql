-- v11 批次 145 P1-8：扩展 budget_items 表，接入预算科目扩展字段
--
-- 背景：
--   budget_management_service.rs 中 CreateBudgetItemRequest / UpdateBudgetItemRequest
--   的 budget_year / planned_amount / remark 字段此前标注为 dead_code，
--   原因是 budget_items 表无对应字段，service.create_item / update_item 无法持久化这些字段。
--   handler 层（budget_management_handler.rs）已经从客户端接收这些字段并传入 service，
--   但 service 层直接丢弃，造成数据丢失。
--
-- 修复：
--   1. budget_year INT4 NULL（可选预算年度，便于按年度筛选科目）
--   2. planned_amount DECIMAL(14,2) NOT NULL DEFAULT 0（计划金额，与 budget_plan.total_amount 类型保持一致）
--   3. remark VARCHAR(500) NULL（备注）
--
-- 关联：
--   - 移除 CreateBudgetItemRequest / UpdateBudgetItemRequest 的 dead_code 标注
--   - create_item / update_item 方法接入这些字段
--   - budget_management 模型同步扩展

ALTER TABLE "budget_items" ADD COLUMN "budget_year" INT4 NULL;
ALTER TABLE "budget_items" ADD COLUMN "planned_amount" DECIMAL(14, 2) NOT NULL DEFAULT 0;
ALTER TABLE "budget_items" ADD COLUMN "remark" VARCHAR(500) NULL;

COMMENT ON COLUMN "budget_items"."budget_year" IS '预算年度（可选，用于按年度筛选预算科目）';
COMMENT ON COLUMN "budget_items"."planned_amount" IS '计划金额（该科目的年度计划预算金额）';
COMMENT ON COLUMN "budget_items"."remark" IS '备注（最多 500 字符）';
