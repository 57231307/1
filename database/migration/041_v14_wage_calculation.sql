-- v14 批次 427：产量工资核算贯通
-- 依据：面料行业真实业务调研文档 §12.5 产量工资（计件计时）
-- 真实业务流程：
--   1. 工序流转扫码 → process_step_record 自动记录工人 IDs + 实际产量 + 合格产量（批次 425 已建）
--   2. 工价方案定义：每道工序的计件单价/计时单价 + A/B/C 等级系数（A 全额/B 折扣/C 不计）
--   3. 工资计算：按工序记录 + 工价方案 + 等级系数自动计算每个工人的应得工资
--   4. 班组汇总：按车间/周期汇总工资，自动进入财务工资核算模块
-- 三维度产量统计：工序产量 + 设备产量 + 工人产量工资
-- 等级系数业务规则：
--   A 级（合格率≥95%）：全额 grade_ratio=1.0
--   B 级（合格率 80-95%）：8 折 grade_ratio=0.8
--   C 级（合格率<80%）：不计 grade_ratio=0.0

-- ============================================================================
-- 1. 工序工价表（process_wage_rate）
-- 业务来源：工价方案定义，每道工序的计件/计时单价 + 等级系数
-- 真实业务：工序工价定义 → 自动汇总进入财务工资核算模块
-- ============================================================================
CREATE TABLE IF NOT EXISTS "process_wage_rate" (
    "id" SERIAL PRIMARY KEY,
    -- 工价单号：PWR-YYYYMMDDHHMMSS-NNN
    "rate_no" VARCHAR(64) NOT NULL,
    -- 关联工序路线 ID（指定哪道工序的工价）
    "process_route_id" INTEGER NOT NULL,
    -- 工序编码（冗余，便于查询）
    "route_code" VARCHAR(32) NOT NULL,
    -- 工序名称（冗余）
    "route_name" VARCHAR(64) NOT NULL,
    -- 工价类型：piece(计件) / time(计时) / mixed(混合)
    "wage_type" VARCHAR(16) NOT NULL DEFAULT 'piece',
    -- 计件单价（元/单位产量，kg 或 m）
    "piece_price" DECIMAL(12,4) NOT NULL DEFAULT 0,
    -- 计时单价（元/分钟）
    "time_price" DECIMAL(12,4) NOT NULL DEFAULT 0,
    -- A 级等级系数（合格率≥95%，默认全额 1.0）
    "grade_a_ratio" DECIMAL(5,4) NOT NULL DEFAULT 1.0,
    -- B 级等级系数（合格率 80-95%，默认 8 折 0.8）
    "grade_b_ratio" DECIMAL(5,4) NOT NULL DEFAULT 0.8,
    -- C 级等级系数（合格率<80%，默认不计 0.0）
    "grade_c_ratio" DECIMAL(5,4) NOT NULL DEFAULT 0.0,
    -- 生效日期
    "effective_date" DATE NOT NULL DEFAULT CURRENT_DATE,
    -- 失效日期（NULL 表示长期有效）
    "expiry_date" DATE,
    -- 车间（用于按车间汇总）
    "workshop" VARCHAR(64),
    -- 备注
    "remarks" VARCHAR(512),
    -- 状态：draft(草稿) → active(启用) → disabled(停用)
    "status" VARCHAR(16) NOT NULL DEFAULT 'draft',
    -- 软删除与审计
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 唯一约束：工价单号唯一
CREATE UNIQUE INDEX IF NOT EXISTS "uk_process_wage_rate_no" ON "process_wage_rate" ("rate_no") WHERE "is_deleted" = FALSE;
-- 唯一约束：同一工序同一生效日期只能有一个工价（避免歧义）
CREATE UNIQUE INDEX IF NOT EXISTS "uk_process_wage_rate_route_effective"
    ON "process_wage_rate" ("process_route_id", "effective_date") WHERE "is_deleted" = FALSE;
-- 索引：按工序路线查询
CREATE INDEX IF NOT EXISTS "idx_wage_rate_route" ON "process_wage_rate" ("process_route_id") WHERE "is_deleted" = FALSE;
-- 索引：按状态查询
CREATE INDEX IF NOT EXISTS "idx_wage_rate_status" ON "process_wage_rate" ("status") WHERE "is_deleted" = FALSE;
-- 索引：按生效日期查询（查找当前生效的工价）
CREATE INDEX IF NOT EXISTS "idx_wage_rate_effective" ON "process_wage_rate" ("effective_date", "expiry_date") WHERE "is_deleted" = FALSE;
-- 索引：按车间查询
CREATE INDEX IF NOT EXISTS "idx_wage_rate_workshop" ON "process_wage_rate" ("workshop") WHERE "is_deleted" = FALSE AND "workshop" IS NOT NULL;

-- 外键约束
ALTER TABLE "process_wage_rate" ADD CONSTRAINT "fk_wage_rate_process_route"
    FOREIGN KEY ("process_route_id") REFERENCES "process_route" ("id") ON UPDATE CASCADE ON DELETE RESTRICT;

-- ============================================================================
-- 2. 工资记录表（wage_record）
-- 业务来源：按周期/车间批量计算工人工资，生成工资单
-- 真实业务：月末/旬末按车间汇总工人产量 → 按工价方案计算 → 进入财务工资核算
-- ============================================================================
CREATE TABLE IF NOT EXISTS "wage_record" (
    "id" SERIAL PRIMARY KEY,
    -- 工资单号：WR-YYYYMM-NNN
    "record_no" VARCHAR(64) NOT NULL,
    -- 统计周期开始日期
    "period_start" DATE NOT NULL,
    -- 统计周期结束日期
    "period_end" DATE NOT NULL,
    -- 车间（用于按车间汇总）
    "workshop" VARCHAR(64),
    -- 总人数（冗余统计字段）
    "total_workers" INTEGER NOT NULL DEFAULT 0,
    -- 总工序记录数
    "total_step_records" INTEGER NOT NULL DEFAULT 0,
    -- 总产量（kg/m，所有工序合格产量之和）
    "total_qualified_quantity" DECIMAL(14,2) NOT NULL DEFAULT 0,
    -- 总工时（分钟）
    "total_duration_minutes" INTEGER NOT NULL DEFAULT 0,
    -- 工资总金额
    "total_amount" DECIMAL(14,2) NOT NULL DEFAULT 0,
    -- 状态：draft(草稿) → confirmed(已确认) → paid(已发放) → cancelled(已取消)
    "status" VARCHAR(16) NOT NULL DEFAULT 'draft',
    -- 确认人 ID
    "confirmed_by" INTEGER,
    -- 确认时间
    "confirmed_at" TIMESTAMPTZ,
    -- 发放人 ID
    "paid_by" INTEGER,
    -- 发放时间
    "paid_at" TIMESTAMPTZ,
    -- 备注
    "remarks" VARCHAR(512),
    -- 软删除与审计
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 唯一约束：工资单号唯一
CREATE UNIQUE INDEX IF NOT EXISTS "uk_wage_record_no" ON "wage_record" ("record_no") WHERE "is_deleted" = FALSE;
-- 索引：按周期查询
CREATE INDEX IF NOT EXISTS "idx_wage_record_period" ON "wage_record" ("period_start", "period_end") WHERE "is_deleted" = FALSE;
-- 索引：按车间查询
CREATE INDEX IF NOT EXISTS "idx_wage_record_workshop" ON "wage_record" ("workshop") WHERE "is_deleted" = FALSE AND "workshop" IS NOT NULL;
-- 索引：按状态查询
CREATE INDEX IF NOT EXISTS "idx_wage_record_status" ON "wage_record" ("status") WHERE "is_deleted" = FALSE;

-- 添加约束：周期结束必须 ≥ 周期开始
ALTER TABLE "wage_record" ADD CONSTRAINT "ck_wage_record_period"
    CHECK ("period_end" >= "period_start");

-- ============================================================================
-- 3. 工资明细表（wage_record_detail）
-- 业务来源：工资计算生成的每个工人每道工序的明细记录
-- 真实业务：工资单详情，按工人查看每道工序的产量/等级/工价/应得工资
-- ============================================================================
CREATE TABLE IF NOT EXISTS "wage_record_detail" (
    "id" SERIAL PRIMARY KEY,
    -- 关联工资记录 ID
    "wage_record_id" INTEGER NOT NULL,
    -- 关联工序记录 ID（数据来源）
    "step_record_id" INTEGER NOT NULL,
    -- 关联流转卡 ID（冗余，便于追溯）
    "flow_card_id" INTEGER,
    -- 缸号（冗余）
    "dye_lot_no" VARCHAR(64),
    -- 关联工序路线 ID
    "process_route_id" INTEGER,
    -- 工序编码（冗余）
    "route_code" VARCHAR(32),
    -- 工序名称（冗余）
    "route_name" VARCHAR(64),
    -- 工序类型（冗余）
    "process_type" VARCHAR(32),
    -- 工人 ID
    "worker_id" INTEGER NOT NULL,
    -- 工人姓名（冗余，便于报表）
    "worker_name" VARCHAR(128),
    -- 设备 ID（冗余，设备产量统计维度）
    "equipment_id" INTEGER,
    -- 设备名称（冗余）
    "equipment_name" VARCHAR(128),
    -- 工价类型快照：piece/time/mixed
    "wage_type" VARCHAR(16) NOT NULL,
    -- 质检等级：A/B/C（依据合格率判定）
    "grade" VARCHAR(2) NOT NULL,
    -- 实际产量（kg/m，来自工序记录）
    "actual_quantity" DECIMAL(12,2) NOT NULL DEFAULT 0,
    -- 合格产量（kg/m，来自工序记录）
    "qualified_quantity" DECIMAL(12,2) NOT NULL DEFAULT 0,
    -- 合格率（百分比，0-100）
    "qualification_rate" DECIMAL(6,2) NOT NULL DEFAULT 0,
    -- 计件单价快照（元/单位产量）
    "piece_price" DECIMAL(12,4) NOT NULL DEFAULT 0,
    -- 计时单价快照（元/分钟）
    "time_price" DECIMAL(12,4) NOT NULL DEFAULT 0,
    -- 等级系数快照（依据等级确定）
    "grade_ratio" DECIMAL(5,4) NOT NULL DEFAULT 0,
    -- 工时（分钟，来自工序记录）
    "duration_minutes" INTEGER NOT NULL DEFAULT 0,
    -- 计件工资部分（合格产量 × 计件单价 × 等级系数）
    "piece_wage" DECIMAL(12,2) NOT NULL DEFAULT 0,
    -- 计时工资部分（工时 × 计时单价 × 等级系数）
    "time_wage" DECIMAL(12,2) NOT NULL DEFAULT 0,
    -- 应得工资（piece_wage + time_wage）
    "wage_amount" DECIMAL(12,2) NOT NULL DEFAULT 0,
    -- 备注
    "remarks" VARCHAR(512),
    -- 软删除与审计
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 索引：按工资记录查询明细
CREATE INDEX IF NOT EXISTS "idx_wage_detail_record" ON "wage_record_detail" ("wage_record_id") WHERE "is_deleted" = FALSE;
-- 索引：按工人查询历史工资
CREATE INDEX IF NOT EXISTS "idx_wage_detail_worker" ON "wage_record_detail" ("worker_id") WHERE "is_deleted" = FALSE;
-- 索引：按工序记录查询（防止重复计算）
CREATE INDEX IF NOT EXISTS "idx_wage_detail_step_record" ON "wage_record_detail" ("step_record_id") WHERE "is_deleted" = FALSE;
-- 索引：按流转卡查询
CREATE INDEX IF NOT EXISTS "idx_wage_detail_flow_card" ON "wage_record_detail" ("flow_card_id") WHERE "is_deleted" = FALSE;
-- 索引：按等级查询
CREATE INDEX IF NOT EXISTS "idx_wage_detail_grade" ON "wage_record_detail" ("grade") WHERE "is_deleted" = FALSE;
-- 索引：按工序类型查询（工序产量统计维度）
CREATE INDEX IF NOT EXISTS "idx_wage_detail_process_type" ON "wage_record_detail" ("process_type") WHERE "is_deleted" = FALSE;

-- 外键约束
ALTER TABLE "wage_record_detail" ADD CONSTRAINT "fk_wage_detail_record"
    FOREIGN KEY ("wage_record_id") REFERENCES "wage_record" ("id") ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE "wage_record_detail" ADD CONSTRAINT "fk_wage_detail_step_record"
    FOREIGN KEY ("step_record_id") REFERENCES "process_step_record" ("id") ON UPDATE CASCADE ON DELETE RESTRICT;
ALTER TABLE "wage_record_detail" ADD CONSTRAINT "fk_wage_detail_flow_card"
    FOREIGN KEY ("flow_card_id") REFERENCES "production_flow_card" ("id") ON UPDATE CASCADE ON DELETE SET NULL;
ALTER TABLE "wage_record_detail" ADD CONSTRAINT "fk_wage_detail_process_route"
    FOREIGN KEY ("process_route_id") REFERENCES "process_route" ("id") ON UPDATE CASCADE ON DELETE SET NULL;

-- ============================================================================
-- 说明：本批次不初始化占位数据
-- 真实业务中工价方案由管理员在系统启用后录入，避免占位数据被外键约束冲突阻塞
-- ============================================================================
