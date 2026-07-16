-- ============================================================================
-- v14 批次 432：缸号全生命周期状态机
-- 依据：面料行业真实业务调研文档 §12.7 缸号状态机 + §3.2 缸号全生命周期追踪
-- 真实业务：缸号（染色批次）从待排缸到发货/取消/终止的全生命周期状态流转
--   14 种状态：pending_schedule/scheduled/preparing/dyeing/washing/fixing/
--              dehydrating/drying/inspecting/stored/shipped/cancelled/terminated/rework
--   终态：shipped 发货 / cancelled 取消 / terminated 终止
--   回修：rework 可回到 dyeing 重新进缸
-- 详细工序（§3.2）：投缸 → 染色 → 皂洗 → 固色 → 脱水 → 烘干 → 验布 → 入库
--   每一环节的操作通过 PDA 扫码或工控终端确认，自动捕获：
--   时间戳、操作人、设备 ID、实时采集参数
-- 状态变更操作（§12.7）：
--   生产计划变更、缸变更、合缸、缸终止、缸优先级调整、回修订单重新进缸
-- 复用现有资产：
--   - dye_batch 表（已有，仅基本字段）：缸号主表，本批次不加外键约束，用应用层校验
-- ============================================================================

-- ============================================================================
-- 1. 缸号生命周期日志表（dye_batch_lifecycle_log）
-- 真实业务：记录缸号每次状态流转事件，PDA 扫码或工控终端确认时写入
-- ============================================================================
CREATE TABLE IF NOT EXISTS "dye_batch_lifecycle_log" (
    "id" SERIAL PRIMARY KEY,
    -- 缸号 ID（关联 dye_batch.id，不加外键约束，用应用层校验）
    "batch_id" INTEGER NOT NULL,
    -- 缸号（冗余便于查询）
    "batch_no" VARCHAR(100) NOT NULL,
    -- 流转前状态（首次创建为 NULL）
    "from_status" VARCHAR(50),
    -- 流转后状态
    "to_status" VARCHAR(50) NOT NULL,
    -- 流转操作代码：schedule/prepare/start_dyeing/wash/fix/dehydrate/dry/inspect/store/ship/cancel/rework/terminate
    "transition_code" VARCHAR(50) NOT NULL,
    -- 流转操作名称：排缸/备布/进缸染色/皂洗/固色/脱水/烘干/验布/入库/发货/取消/回修/终止
    "transition_name" VARCHAR(100) NOT NULL,
    -- 操作人 ID
    "operator_id" INTEGER,
    -- 操作人姓名
    "operator_name" VARCHAR(100),
    -- 设备 ID
    "equipment_id" INTEGER,
    -- 设备名称
    "equipment_name" VARCHAR(100),
    -- 班次：morning 早班/day 白班/night 夜班
    "work_shift" VARCHAR(20),
    -- PDA/工控终端采集参数：温度/色差ΔE/时间戳等
    "captured_params" JSONB,
    -- 备注
    "remarks" TEXT,
    -- 操作发生时间
    "transition_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 索引：按缸号 ID 查询
CREATE INDEX IF NOT EXISTS "idx_dye_batch_lifecycle_log_batch" ON "dye_batch_lifecycle_log" ("batch_id");
-- 索引：按缸号查询
CREATE INDEX IF NOT EXISTS "idx_dye_batch_lifecycle_log_batch_no" ON "dye_batch_lifecycle_log" ("batch_no");
-- 索引：按流转操作代码查询
CREATE INDEX IF NOT EXISTS "idx_dye_batch_lifecycle_log_transition" ON "dye_batch_lifecycle_log" ("transition_code");
-- 索引：按操作时间查询
CREATE INDEX IF NOT EXISTS "idx_dye_batch_lifecycle_log_time" ON "dye_batch_lifecycle_log" ("transition_at");

COMMENT ON TABLE "dye_batch_lifecycle_log" IS '缸号生命周期日志表（v14 批次 432：记录缸号每次状态流转事件）';
COMMENT ON COLUMN "dye_batch_lifecycle_log"."from_status" IS '流转前状态（首次创建为 NULL）';
COMMENT ON COLUMN "dye_batch_lifecycle_log"."to_status" IS '流转后状态（14 种状态之一）';
COMMENT ON COLUMN "dye_batch_lifecycle_log"."transition_code" IS '流转操作代码 schedule/prepare/start_dyeing/wash/fix/dehydrate/dry/inspect/store/ship/cancel/rework/terminate';
COMMENT ON COLUMN "dye_batch_lifecycle_log"."work_shift" IS '班次 morning 早班/day 白班/night 夜班';
COMMENT ON COLUMN "dye_batch_lifecycle_log"."captured_params" IS 'PDA/工控终端采集参数（温度/色差ΔE/时间戳等）';

-- ============================================================================
-- 2. 缸号状态流转规则表（dye_batch_state_rule）
-- 真实业务：定义允许的状态转换，用于校验缸号状态流转合法性
-- ============================================================================
CREATE TABLE IF NOT EXISTS "dye_batch_state_rule" (
    "id" SERIAL PRIMARY KEY,
    -- 流转前状态（NULL 表示初始状态 pending_schedule）
    "from_status" VARCHAR(50) NOT NULL,
    -- 流转后状态
    "to_status" VARCHAR(50) NOT NULL,
    -- 流转操作代码
    "transition_code" VARCHAR(50) NOT NULL,
    -- 流转操作名称
    "transition_name" VARCHAR(100) NOT NULL,
    -- 是否允许
    "is_allowed" BOOLEAN NOT NULL DEFAULT TRUE,
    -- 是否必须记录操作人
    "require_operator" BOOLEAN NOT NULL DEFAULT TRUE,
    -- 是否必须记录设备
    "require_equipment" BOOLEAN NOT NULL DEFAULT FALSE,
    -- 是否必须填写备注
    "require_remarks" BOOLEAN NOT NULL DEFAULT FALSE,
    -- 额外校验逻辑描述（JSONB）
    "validation_logic" JSONB,
    -- 规则描述
    "description" TEXT,
    -- 是否启用
    "is_active" BOOLEAN NOT NULL DEFAULT TRUE,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 唯一约束：同一 from_status + to_status + transition_code 唯一
CREATE UNIQUE INDEX IF NOT EXISTS "uk_dye_batch_state_rule_trans" ON "dye_batch_state_rule" ("from_status", "to_status", "transition_code");
-- 索引：按流转前状态查询
CREATE INDEX IF NOT EXISTS "idx_dye_batch_state_rule_from" ON "dye_batch_state_rule" ("from_status");
-- 索引：按流转后状态查询
CREATE INDEX IF NOT EXISTS "idx_dye_batch_state_rule_to" ON "dye_batch_state_rule" ("to_status");

COMMENT ON TABLE "dye_batch_state_rule" IS '缸号状态流转规则表（v14 批次 432：定义允许的状态转换）';
COMMENT ON COLUMN "dye_batch_state_rule"."from_status" IS '流转前状态（pending_schedule/scheduled/preparing/dyeing/washing/fixing/dehydrating/drying/inspecting/stored/rework）';
COMMENT ON COLUMN "dye_batch_state_rule"."to_status" IS '流转后状态（14 种状态之一）';
COMMENT ON COLUMN "dye_batch_state_rule"."transition_code" IS '流转操作代码 schedule/prepare/start_dyeing/wash/fix/dehydrate/dry/inspect/store/ship/cancel/rework/terminate';
COMMENT ON COLUMN "dye_batch_state_rule"."require_operator" IS '是否必须记录操作人';
COMMENT ON COLUMN "dye_batch_state_rule"."require_equipment" IS '是否必须记录设备';

-- ============================================================================
-- 3. 缸号回修记录表（dye_batch_rework）
-- 真实业务：记录回修订单重新进缸，回修类型色差/疵点/规格不符/其他
-- ============================================================================
CREATE TABLE IF NOT EXISTS "dye_batch_rework" (
    "id" SERIAL PRIMARY KEY,
    -- 原缸号 ID
    "original_batch_id" INTEGER NOT NULL,
    -- 原缸号
    "original_batch_no" VARCHAR(100) NOT NULL,
    -- 回修缸号 ID（若同缸回修则为原缸号）
    "rework_batch_id" INTEGER,
    -- 回修缸号（若同缸回修则同原缸号）
    "rework_batch_no" VARCHAR(100),
    -- 回修类型：color_difference 色差/defect 疵点/specification_unqualified 规格不符/other 其他
    "rework_type" VARCHAR(50) NOT NULL,
    -- 回修原因
    "rework_reason" TEXT NOT NULL,
    -- 回修前状态：inspecting 或 stored
    "original_status" VARCHAR(50) NOT NULL,
    -- 审批人 ID
    "approved_by" INTEGER,
    -- 审批时间
    "approved_at" TIMESTAMPTZ,
    -- 状态：draft 草稿/approved 已审批/in_progress 回修中/completed 已完成/cancelled 已取消
    "status" VARCHAR(30) NOT NULL DEFAULT 'draft',
    -- 回修开始时间
    "started_at" TIMESTAMPTZ,
    -- 回修完成时间
    "completed_at" TIMESTAMPTZ,
    -- 备注
    "remarks" TEXT,
    -- 软删除
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 索引：按原缸号 ID 查询
CREATE INDEX IF NOT EXISTS "idx_dye_batch_rework_original" ON "dye_batch_rework" ("original_batch_id");
-- 索引：按回修缸号 ID 查询
CREATE INDEX IF NOT EXISTS "idx_dye_batch_rework_rework" ON "dye_batch_rework" ("rework_batch_id");
-- 索引：按状态查询
CREATE INDEX IF NOT EXISTS "idx_dye_batch_rework_status" ON "dye_batch_rework" ("status");

COMMENT ON TABLE "dye_batch_rework" IS '缸号回修记录表（v14 批次 432：记录回修订单重新进缸）';
COMMENT ON COLUMN "dye_batch_rework"."rework_type" IS '回修类型 color_difference 色差/defect 疵点/specification_unqualified 规格不符/other 其他';
COMMENT ON COLUMN "dye_batch_rework"."original_status" IS '回修前状态 inspecting 或 stored';
COMMENT ON COLUMN "dye_batch_rework"."status" IS '状态 draft 草稿/approved 已审批/in_progress 回修中/completed 已完成/cancelled 已取消';

-- ============================================================================
-- 4. 缸号操作记录表（dye_batch_operation）
-- 真实业务：记录合缸/分缸/优先级调整/缸变更等操作
-- ============================================================================
CREATE TABLE IF NOT EXISTS "dye_batch_operation" (
    "id" SERIAL PRIMARY KEY,
    -- 操作类型：merge 合缸/split 分缸/priority_adjust 优先级调整/batch_change 缸变更/schedule_change 计划变更/terminate 终止
    "operation_type" VARCHAR(50) NOT NULL,
    -- 操作名称
    "operation_name" VARCHAR(100) NOT NULL,
    -- 目标缸号 ID（主操作缸号）
    "target_batch_id" INTEGER NOT NULL,
    -- 目标缸号
    "target_batch_no" VARCHAR(100) NOT NULL,
    -- 源缸号 ID 列表（合缸/分缸时使用，JSONB 数组）
    "source_batch_ids" JSONB,
    -- 源缸号列表（JSONB 数组）
    "source_batch_nos" JSONB,
    -- 操作数据：优先级值、变更前后信息等
    "operation_data" JSONB,
    -- 操作人 ID
    "operator_id" INTEGER,
    -- 操作人姓名
    "operator_name" VARCHAR(100),
    -- 操作时间
    "operation_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- 备注
    "remarks" TEXT,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 索引：按目标缸号 ID 查询
CREATE INDEX IF NOT EXISTS "idx_dye_batch_operation_target" ON "dye_batch_operation" ("target_batch_id");
-- 索引：按操作类型查询
CREATE INDEX IF NOT EXISTS "idx_dye_batch_operation_type" ON "dye_batch_operation" ("operation_type");
-- 索引：按操作时间查询
CREATE INDEX IF NOT EXISTS "idx_dye_batch_operation_time" ON "dye_batch_operation" ("operation_at");

COMMENT ON TABLE "dye_batch_operation" IS '缸号操作记录表（v14 批次 432：记录合缸/分缸/优先级调整/缸变更等操作）';
COMMENT ON COLUMN "dye_batch_operation"."operation_type" IS '操作类型 merge 合缸/split 分缸/priority_adjust 优先级调整/batch_change 缸变更/schedule_change 计划变更/terminate 终止';
COMMENT ON COLUMN "dye_batch_operation"."source_batch_ids" IS '源缸号 ID 列表（合缸/分缸时使用，JSONB 数组）';
COMMENT ON COLUMN "dye_batch_operation"."operation_data" IS '操作数据（优先级值、变更前后信息等）';

-- ============================================================================
-- 5. 预置数据：缸号状态流转规则
-- 依据：§12.7 允许的状态流转
-- pending_schedule → scheduled / cancelled
-- scheduled → preparing / cancelled / terminated
-- preparing → dyeing / cancelled / terminated
-- dyeing → washing / cancelled / terminated
-- washing → fixing / cancelled
-- fixing → dehydrating / cancelled
-- dehydrating → drying / cancelled
-- drying → inspecting / cancelled
-- inspecting → stored / rework / cancelled
-- stored → shipped / rework / cancelled
-- rework → dyeing（回修重新进缸）/ cancelled / terminated
-- shipped / cancelled / terminated 为终态，不可流转
-- ============================================================================

-- pending_schedule → scheduled（排缸）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "description")
VALUES ('pending_schedule', 'scheduled', 'schedule', '排缸', '待排缸 → 已排缸')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- pending_schedule → cancelled（取消）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_operator", "description")
VALUES ('pending_schedule', 'cancelled', 'cancel', '取消', FALSE, '待排缸 → 取消')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- scheduled → preparing（备布）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_equipment", "description")
VALUES ('scheduled', 'preparing', 'prepare', '备布', TRUE, '已排缸 → 备布中')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- scheduled → cancelled（取消）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_operator", "description")
VALUES ('scheduled', 'cancelled', 'cancel', '取消', FALSE, '已排缸 → 取消')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- scheduled → terminated（终止）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
VALUES ('scheduled', 'terminated', 'terminate', '终止', TRUE, '已排缸 → 终止')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- preparing → dyeing（进缸染色）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_equipment", "description")
VALUES ('preparing', 'dyeing', 'start_dyeing', '进缸染色', TRUE, '备布中 → 进缸染色')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- preparing → cancelled（取消）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_operator", "description")
VALUES ('preparing', 'cancelled', 'cancel', '取消', FALSE, '备布中 → 取消')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- preparing → terminated（终止）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
VALUES ('preparing', 'terminated', 'terminate', '终止', TRUE, '备布中 → 终止')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- dyeing → washing（皂洗）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_equipment", "description")
VALUES ('dyeing', 'washing', 'wash', '皂洗', TRUE, '进缸染色 → 皂洗')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- dyeing → cancelled（取消）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_operator", "description")
VALUES ('dyeing', 'cancelled', 'cancel', '取消', FALSE, '进缸染色 → 取消')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- dyeing → terminated（终止）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
VALUES ('dyeing', 'terminated', 'terminate', '终止', TRUE, '进缸染色 → 终止')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- washing → fixing（固色）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_equipment", "description")
VALUES ('washing', 'fixing', 'fix', '固色', TRUE, '皂洗 → 固色')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- washing → cancelled（取消）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_operator", "description")
VALUES ('washing', 'cancelled', 'cancel', '取消', FALSE, '皂洗 → 取消')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- fixing → dehydrating（脱水）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_equipment", "description")
VALUES ('fixing', 'dehydrating', 'dehydrate', '脱水', TRUE, '固色 → 脱水')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- fixing → cancelled（取消）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_operator", "description")
VALUES ('fixing', 'cancelled', 'cancel', '取消', FALSE, '固色 → 取消')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- dehydrating → drying（烘干）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_equipment", "description")
VALUES ('dehydrating', 'drying', 'dry', '烘干', TRUE, '脱水 → 烘干')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- dehydrating → cancelled（取消）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_operator", "description")
VALUES ('dehydrating', 'cancelled', 'cancel', '取消', FALSE, '脱水 → 取消')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- drying → inspecting（验布）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_equipment", "description")
VALUES ('drying', 'inspecting', 'inspect', '验布', TRUE, '烘干 → 验布')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- drying → cancelled（取消）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_operator", "description")
VALUES ('drying', 'cancelled', 'cancel', '取消', FALSE, '烘干 → 取消')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- inspecting → stored（入库）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "description")
VALUES ('inspecting', 'stored', 'store', '入库', '验布 → 入库')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- inspecting → rework（回修）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
VALUES ('inspecting', 'rework', 'rework', '回修', TRUE, '验布 → 回修中（回修重新进缸）')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- inspecting → cancelled（取消）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_operator", "description")
VALUES ('inspecting', 'cancelled', 'cancel', '取消', FALSE, '验布 → 取消')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- stored → shipped（发货）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "description")
VALUES ('stored', 'shipped', 'ship', '发货', '入库 → 发货（终态）')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- stored → rework（回修）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
VALUES ('stored', 'rework', 'rework', '回修', TRUE, '入库 → 回修中（回修重新进缸）')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- stored → cancelled（取消）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_operator", "description")
VALUES ('stored', 'cancelled', 'cancel', '取消', FALSE, '入库 → 取消')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- rework → dyeing（回修重新进缸）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_equipment", "description")
VALUES ('rework', 'dyeing', 'start_dyeing', '进缸染色', TRUE, '回修中 → 进缸染色（回修重新进缸）')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- rework → cancelled（取消）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_operator", "description")
VALUES ('rework', 'cancelled', 'cancel', '取消', FALSE, '回修中 → 取消')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;

-- rework → terminated（终止）
INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
VALUES ('rework', 'terminated', 'terminate', '终止', TRUE, '回修中 → 终止')
ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;
