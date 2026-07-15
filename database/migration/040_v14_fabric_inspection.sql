-- v14 批次 426：验布打卷流程贯通
-- 依据：面料行业真实业务调研文档 §12.4 验布打卷与成品入库
-- 真实业务流程：
--   验布机对接码表/电子称 → 疵点采集 → 生成验布报告
--   → 卷唛标签打印 → PDA 扫描卷唛条码 → 自动入库
-- 真实业务要点（WebSearch 验证）：
--   1. 四分制评分（AATCC/ASTM D5430，针织+梭织通用）：
--      疵点长度 ≤3寸=1分, 3-6寸=2分, 6-9寸=3分, >9寸=4分
--      破洞/连续性疵点=4分，同一码内不超过4分，布边1寸内不扣分
--      等级：每百平方码分数 ≤40 = 首级(first)，>40 = 次级(second)
--      计算公式：每百平方码分数 = (总扣分 × 36 × 100) / (受检码数 × 幅宽英寸)
--   2. 十分制评分（梭织布）：
--      经向：1寸以下=1分, 1-5寸=3分, 5-10寸=5分, 10-36寸=10分
--      纬向：1寸以下=1分, 1-5寸=3分, 5寸-半门幅=5分, 半门幅以上=10分
--      破洞=10分，同一码内不超过10分，布边半英寸内不扣分
--      等级：总扣分 < 总码数 = 首级(first)，≥ 总码数 = 次级(second)
--   3. 疵点类型：断经/油污/色花/破洞/纬斜/横档/色差/窄封/折痕/染色不均匀等
--   4. 打卷入库：每卷生成唯一匹号（缸号-序号），PDA 扫码卷唛条码自动入库
-- 来源：AATCC 检验标准 / ASTM D5430 / 面料验货基础知识（2026-07 真实业务资料）

-- ============================================================================
-- 1. 验布记录表（fabric_inspection_record）
-- 业务来源：验布机对接码表/电子称 → 疵点采集 → 生成验布报告
-- 真实业务：一缸一验布记录，记录评分制式/总扣分/每百平方码分数/等级判定
-- ============================================================================
CREATE TABLE IF NOT EXISTS "fabric_inspection_record" (
    "id" SERIAL PRIMARY KEY,
    -- 验布单号（FIR-YYYYMMDDHHMMSS-NNN）
    "inspection_no" VARCHAR(32) NOT NULL,
    -- 关联流转卡（可选，验布环节流转卡状态应为 inspecting）
    "flow_card_id" INTEGER,
    -- 缸号（面料行业追溯核心字段）
    "dye_lot_no" VARCHAR(64),
    -- 产品信息（冗余存储便于直接查询）
    "product_id" INTEGER,
    "product_name" VARCHAR(128),
    "color_no" VARCHAR(64),
    -- 验布信息
    "inspection_date" DATE NOT NULL,
    "inspector_id" INTEGER,
    "inspector_name" VARCHAR(64),
    "machine_no" VARCHAR(32),
    -- 评分制式：four_point(四分制) / ten_point(十分制)
    "scoring_system" VARCHAR(16) NOT NULL DEFAULT 'four_point',
    -- 受检码数（验布机码表读数）
    "inspected_yards" NUMERIC(12, 2) NOT NULL DEFAULT 0,
    -- 幅宽（英寸，用于四分制每百平方码分数计算）
    "fabric_width_inches" NUMERIC(8, 2),
    -- 总扣分（所有疵点扣分之和）
    "total_defect_points" INTEGER NOT NULL DEFAULT 0,
    -- 每百平方码分数（四分制等级判定依据，计算字段）
    "points_per_100_sq_yards" NUMERIC(10, 2),
    -- 验布等级：first(首级) / second(次级)
    "grade" VARCHAR(16),
    -- 合格率（百分比，用于联动 A/B/C 分级）
    "qualification_rate" NUMERIC(5, 2),
    -- A/B/C 级（联动 determine_quality_grade：A 级合格/B 级让步接收/C 级返工报废）
    "abc_grade" VARCHAR(4),
    -- 打卷汇总
    "total_rolls" INTEGER NOT NULL DEFAULT 0,
    "total_roll_length" NUMERIC(12, 2) NOT NULL DEFAULT 0,
    "total_roll_weight" NUMERIC(12, 2) NOT NULL DEFAULT 0,
    -- 状态：pending/inspecting/graded/rolled/closed
    "status" VARCHAR(16) NOT NULL DEFAULT 'pending',
    -- 软删除与审计
    "remarks" VARCHAR(256),
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 唯一约束：验布单号唯一
CREATE UNIQUE INDEX IF NOT EXISTS "uk_fabric_inspection_no" ON "fabric_inspection_record" ("inspection_no") WHERE "is_deleted" = FALSE;
-- 按缸号查询
CREATE INDEX IF NOT EXISTS "idx_fabric_inspection_dye_lot" ON "fabric_inspection_record" ("dye_lot_no") WHERE "is_deleted" = FALSE;
-- 按流转卡查询
CREATE INDEX IF NOT EXISTS "idx_fabric_inspection_flow_card" ON "fabric_inspection_record" ("flow_card_id") WHERE "is_deleted" = FALSE;
-- 按产品查询
CREATE INDEX IF NOT EXISTS "idx_fabric_inspection_product" ON "fabric_inspection_record" ("product_id") WHERE "is_deleted" = FALSE;
-- 按状态查询
CREATE INDEX IF NOT EXISTS "idx_fabric_inspection_status" ON "fabric_inspection_record" ("status") WHERE "is_deleted" = FALSE;
-- 按验布日期查询
CREATE INDEX IF NOT EXISTS "idx_fabric_inspection_date" ON "fabric_inspection_record" ("inspection_date") WHERE "is_deleted" = FALSE;

-- 外键约束（DEFERRABLE 避免循环依赖）
ALTER TABLE "fabric_inspection_record"
    ADD CONSTRAINT "fk_fabric_inspection_flow_card"
    FOREIGN KEY ("flow_card_id") REFERENCES "production_flow_card" ("id") DEFERRABLE INITIALLY DEFERRED;

-- ============================================================================
-- 2. 疵点明细表（fabric_defect_record）
-- 业务来源：验布过程疵点采集，记录每个疵点的类型/位置/长度/扣分
-- 真实业务：验布机采集疵点 → 系统按评分制式自动计算扣分 → 汇总到验布记录
-- ============================================================================
CREATE TABLE IF NOT EXISTS "fabric_defect_record" (
    "id" SERIAL PRIMARY KEY,
    -- 关联验布记录
    "inspection_id" INTEGER NOT NULL,
    -- 疵点类型（标准编码）：
    --   broken_end(断经) / oil_stain(油污) / color_spot(色花) / hole(破洞)
    --   skew_lane(纬斜) / streak(横档) / color_diff(色差) / narrow_width(窄封)
    --   crease(折痕) / uneven_dye(染色不均匀) / lint(飞花) / other(其他)
    "defect_type" VARCHAR(32) NOT NULL,
    -- 疵点位置（码数，验布机码表读数）
    "position_yards" NUMERIC(10, 2) NOT NULL DEFAULT 0,
    -- 疵点长度（英寸，评分计算依据）
    "defect_length_inches" NUMERIC(8, 2) NOT NULL DEFAULT 0,
    -- 方向：warp(经向) / weft(纬向) / other(其他)
    "direction" VARCHAR(8) NOT NULL DEFAULT 'other',
    -- 是否破洞（破洞不论大小一律最高扣分：四分制4分/十分制10分）
    "is_hole" BOOLEAN NOT NULL DEFAULT FALSE,
    -- 是否连续性疵点（横档/色差/窄封/折痕/染色不均匀等，每码最高扣分）
    "is_continuous" BOOLEAN NOT NULL DEFAULT FALSE,
    -- 是否超过半门幅（十分制纬向评分依据：半门幅以上=10分，以下=5分）
    "is_half_width" BOOLEAN NOT NULL DEFAULT FALSE,
    -- 扣分（四分制 1/2/3/4，十分制 1/3/5/10）
    "points" INTEGER NOT NULL DEFAULT 0,
    -- 疵点描述
    "description" VARCHAR(256),
    -- 审计
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 按验布记录查询疵点明细
CREATE INDEX IF NOT EXISTS "idx_fabric_defect_inspection" ON "fabric_defect_record" ("inspection_id");
-- 按疵点类型统计
CREATE INDEX IF NOT EXISTS "idx_fabric_defect_type" ON "fabric_defect_record" ("defect_type");

-- 外键约束：疵点明细关联验布记录
ALTER TABLE "fabric_defect_record"
    ADD CONSTRAINT "fk_fabric_defect_inspection"
    FOREIGN KEY ("inspection_id") REFERENCES "fabric_inspection_record" ("id") ON DELETE CASCADE;

-- ============================================================================
-- 3. 扩展库存匹数表（inventory_piece）添加验布关联字段
-- 业务来源：打卷入库 → 每卷生成唯一匹号 → PDA 扫码卷唛条码自动入库
-- 真实业务：复用 inventory_piece 表存储打卷后的成品布卷，新增验布关联字段
-- ============================================================================
-- 关联验布记录（nullable，仅验布打卷产生的布卷才有）
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "inspection_id" INTEGER;
-- 缸号内匹号序号（用于匹号生成：{dye_lot_no}-{seq:03}）
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "piece_seq" INTEGER;

-- 按验布记录查询布卷
CREATE INDEX IF NOT EXISTS "idx_inventory_piece_inspection" ON "inventory_piece" ("inspection_id");
-- 按缸号内序号查询（匹号唯一性辅助校验）
CREATE INDEX IF NOT EXISTS "idx_inventory_piece_seq" ON "inventory_piece" ("dye_lot_id", "piece_seq");
