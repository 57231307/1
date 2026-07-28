-- ============================================================================
-- Migration 053: V15 P1 缸号状态机增加 on_hold/failed 异常态
-- 依据：V15 审计报告 类五 P1（batch-05 维度 5.3：状态机闭环 缺陷项 1）
-- 业务背景：染整过程中设备故障、染料异常、停电等场景需要 on_hold(暂停) 状态临时挂起缸号，
--   待恢复后继续流转；failed(失败) 状态标识彻底失败的缸号（需返工或报废）。
--   当前仅有 cancelled/terminated 终态，无法区分"临时暂停"与"彻底终止"。
-- 修复策略：
--   1. 在 dye_batch_state_rule 表预置 on_hold/failed 相关流转规则
--   2. 应用层（dye_batch_state_machine_service.rs）同步扩展状态常量与校验函数
--   3. on_hold 可恢复到染整各工序继续流转；failed 为终态
-- 关联文件：backend/src/services/dye_batch_state_machine_service.rs
--             backend/src/models/status/quality_dyeing.rs
-- ============================================================================

-- ============================================================================
-- 1. 新增 on_hold(暂停) 流转规则
-- 业务场景：染整各工序中设备故障/染料异常/停电等临时挂起，待恢复后继续流转
-- ============================================================================

-- scheduled → on_hold（暂停）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
VALUES ('scheduled', 'on_hold', 'hold', '暂停', TRUE, '已排缸 → 暂停（设备故障/染料异常/停电）')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- preparing → on_hold（暂停）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
VALUES ('preparing', 'on_hold', 'hold', '暂停', TRUE, '备布中 → 暂停（设备故障/染料异常/停电）')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- dyeing → on_hold（暂停）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
VALUES ('dyeing', 'on_hold', 'hold', '暂停', TRUE, '进缸染色 → 暂停（设备故障/染料异常/停电）')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- washing → on_hold（暂停）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
VALUES ('washing', 'on_hold', 'hold', '暂停', TRUE, '皂洗 → 暂停（设备故障/染料异常/停电）')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- fixing → on_hold（暂停）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
VALUES ('fixing', 'on_hold', 'hold', '暂停', TRUE, '固色 → 暂停（设备故障/染料异常/停电）')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- dehydrating → on_hold（暂停）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
VALUES ('dehydrating', 'on_hold', 'hold', '暂停', TRUE, '脱水 → 暂停（设备故障/染料异常/停电）')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- drying → on_hold（暂停）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
VALUES ('drying', 'on_hold', 'hold', '暂停', TRUE, '烘干 → 暂停（设备故障/染料异常/停电）')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- ============================================================================
-- 2. 新增 on_hold → 恢复流转规则（resume）
-- 业务场景：暂停原因消除后，恢复到原工序继续流转
-- ============================================================================

-- on_hold → scheduled（恢复）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_operator", "description")
VALUES ('on_hold', 'scheduled', 'resume', '恢复', TRUE, '暂停 → 已排缸（恢复）')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- on_hold → preparing（恢复）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_operator", "description")
VALUES ('on_hold', 'preparing', 'resume', '恢复', TRUE, '暂停 → 备布中（恢复）')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- on_hold → dyeing（恢复）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_operator", "description")
VALUES ('on_hold', 'dyeing', 'resume', '恢复', TRUE, '暂停 → 进缸染色（恢复）')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- on_hold → washing（恢复）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_operator", "description")
VALUES ('on_hold', 'washing', 'resume', '恢复', TRUE, '暂停 → 皂洗（恢复）')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- on_hold → fixing（恢复）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_operator", "description")
VALUES ('on_hold', 'fixing', 'resume', '恢复', TRUE, '暂停 → 固色（恢复）')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- on_hold → dehydrating（恢复）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_operator", "description")
VALUES ('on_hold', 'dehydrating', 'resume', '恢复', TRUE, '暂停 → 脱水（恢复）')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- on_hold → drying（恢复）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_operator", "description")
VALUES ('on_hold', 'drying', 'resume', '恢复', TRUE, '暂停 → 烘干（恢复）')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- on_hold → cancelled（取消）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_operator", "description")
VALUES ('on_hold', 'cancelled', 'cancel', '取消', FALSE, '暂停 → 取消')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- ============================================================================
-- 3. 新增 failed(失败) 流转规则（终态）
-- 业务场景：染整过程中彻底失败，需返工或报废
-- ============================================================================

-- pending_schedule → failed（失败）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
VALUES ('pending_schedule', 'failed', 'fail', '失败', TRUE, '待排缸 → 失败（终态，需返工或报废）')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- scheduled → failed（失败）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
VALUES ('scheduled', 'failed', 'fail', '失败', TRUE, '已排缸 → 失败（终态，需返工或报废）')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- preparing → failed（失败）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
VALUES ('preparing', 'failed', 'fail', '失败', TRUE, '备布中 → 失败（终态，需返工或报废）')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- dyeing → failed（失败）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
VALUES ('dyeing', 'failed', 'fail', '失败', TRUE, '进缸染色 → 失败（终态，需返工或报废）')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- washing → failed（失败）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
VALUES ('washing', 'failed', 'fail', '失败', TRUE, '皂洗 → 失败（终态，需返工或报废）')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- fixing → failed（失败）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
VALUES ('fixing', 'failed', 'fail', '失败', TRUE, '固色 → 失败（终态，需返工或报废）')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- dehydrating → failed（失败）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
VALUES ('dehydrating', 'failed', 'fail', '失败', TRUE, '脱水 → 失败（终态，需返工或报废）')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- drying → failed（失败）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
VALUES ('drying', 'failed', 'fail', '失败', TRUE, '烘干 → 失败（终态，需返工或报废）')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- inspecting → failed（失败）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
VALUES ('inspecting', 'failed', 'fail', '失败', TRUE, '验布 → 失败（终态，需返工或报废）')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- stored → failed（失败）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
VALUES ('stored', 'failed', 'fail', '失败', TRUE, '入库 → 失败（终态，需返工或报废）')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- rework → failed（失败）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
VALUES ('rework', 'failed', 'fail', '失败', TRUE, '回修中 → 失败（终态，需返工或报废）')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- on_hold → failed（失败）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
VALUES ('on_hold', 'failed', 'fail', '失败', TRUE, '暂停 → 失败（终态，需返工或报废）')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- ============================================================================
-- 4. 更新 dye_batch_lifecycle_log 表注释（反映新状态）
-- ============================================================================
COMMENT ON COLUMN "dye_batch_lifecycle_log"."to_status" IS '流转后状态（16 种状态之一：pending_schedule/scheduled/preparing/dyeing/washing/fixing/dehydrating/drying/inspecting/stored/shipped/cancelled/terminated/rework/on_hold/failed）';
COMMENT ON COLUMN "dye_batch_lifecycle_log"."transition_code" IS '流转操作代码 schedule/prepare/start_dyeing/wash/fix/dehydrate/dry/inspect/store/ship/cancel/rework/terminate/hold/resume/fail';
