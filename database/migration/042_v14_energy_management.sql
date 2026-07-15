-- ============================================================================
-- v14 批次 428：能耗管理贯通
-- 依据：面料行业真实业务调研文档 §12.6 能耗管理
-- 真实业务流程（WebSearch 验证）：
--   1. 能源类型：水/电/蒸汽/天然气/压缩空气（占总成本 35%+）
--   2. 采集方式：IoT 设备实时采集（智能电表/蒸汽流量计/水质监测仪）+ 手工录入
--   3. 分摊方式：按工艺路线归集到缸号/工序/订单；按工时×功率系数分摊
--   4. 基准管理：每道工序预设理论能耗基准，超基准预警
--   5. 月末分摊：自动核算每缸布的水电汽实际消耗与标准成本，生成 cost_collection 记录
-- ============================================================================

-- ============================================================================
-- 1. 能源计量设备表（energy_meter）
-- 真实业务：每个车间/机台安装的能源计量设备（电表/水表/汽表）
-- ============================================================================
CREATE TABLE IF NOT EXISTS "energy_meter" (
    "id" SERIAL PRIMARY KEY,
    -- 计量设备编号：EM-YYYYMMDDHHMMSS-NNN
    "meter_no" VARCHAR(64) NOT NULL,
    -- 计量设备名称
    "meter_name" VARCHAR(128) NOT NULL,
    -- 能源类型：water(水) / electricity(电) / steam(蒸汽) / gas(天然气) / compressed_air(压缩空气)
    "meter_type" VARCHAR(32) NOT NULL,
    -- 所属车间
    "workshop" VARCHAR(100),
    -- 关联设备 ID（机台级电表时关联 equipment 表）
    "equipment_id" INTEGER,
    -- 设备名称（冗余）
    "equipment_name" VARCHAR(128),
    -- 安装位置
    "location" VARCHAR(256),
    -- IoT 设备 ID（对接 PLC/智能网关）
    "iot_device_id" VARCHAR(128),
    -- 计量单位（吨/度/立方米）
    "unit" VARCHAR(32) NOT NULL DEFAULT '度',
    -- 当前读数
    "current_reading" DECIMAL(14,2) NOT NULL DEFAULT 0,
    -- 上次读数
    "previous_reading" DECIMAL(14,2) NOT NULL DEFAULT 0,
    -- 上次读数时间
    "last_reading_at" TIMESTAMPTZ,
    -- 单价（元/单位）
    "unit_price" DECIMAL(12,4) NOT NULL DEFAULT 0,
    -- 状态：active(启用) / inactive(停用) / maintenance(维护中)
    "status" VARCHAR(32) NOT NULL DEFAULT 'active',
    -- 备注
    "remarks" VARCHAR(512),
    -- 软删除与审计
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 唯一约束：计量设备编号
CREATE UNIQUE INDEX IF NOT EXISTS "uk_energy_meter_no" ON "energy_meter" ("meter_no") WHERE "is_deleted" = FALSE;
-- 索引：按能源类型查询
CREATE INDEX IF NOT EXISTS "idx_energy_meter_type" ON "energy_meter" ("meter_type") WHERE "is_deleted" = FALSE;
-- 索引：按车间查询
CREATE INDEX IF NOT EXISTS "idx_energy_meter_workshop" ON "energy_meter" ("workshop") WHERE "is_deleted" = FALSE;
-- 索引：按设备查询
CREATE INDEX IF NOT EXISTS "idx_energy_meter_equipment" ON "energy_meter" ("equipment_id") WHERE "is_deleted" = FALSE AND "equipment_id" IS NOT NULL;
-- 索引：按状态查询
CREATE INDEX IF NOT EXISTS "idx_energy_meter_status" ON "energy_meter" ("status") WHERE "is_deleted" = FALSE;
-- 索引：按 IoT 设备 ID 查询（对接采集系统）
CREATE INDEX IF NOT EXISTS "idx_energy_meter_iot" ON "energy_meter" ("iot_device_id") WHERE "is_deleted" = FALSE AND "iot_device_id" IS NOT NULL;

COMMENT ON TABLE "energy_meter" IS '能源计量设备表（水/电/汽表，IoT 对接）';
COMMENT ON COLUMN "energy_meter"."meter_no" IS '计量设备编号 EM-YYYYMMDDHHMMSS-NNN';
COMMENT ON COLUMN "energy_meter"."meter_type" IS '能源类型 water/electricity/steam/gas/compressed_air';
COMMENT ON COLUMN "energy_meter"."iot_device_id" IS 'IoT 设备 ID（对接 PLC/智能网关）';

-- ============================================================================
-- 2. 能耗记录表（energy_consumption_record）
-- 真实业务：按时间段登记能耗（手工或 IoT 自动采集），可关联缸号/工序/订单
-- ============================================================================
CREATE TABLE IF NOT EXISTS "energy_consumption_record" (
    "id" SERIAL PRIMARY KEY,
    -- 记录编号：EC-YYYYMMDDHHMMSS-NNN
    "record_no" VARCHAR(64) NOT NULL,
    -- 关联计量设备 ID
    "meter_id" INTEGER,
    -- 能源类型（冗余）
    "meter_type" VARCHAR(32) NOT NULL,
    -- 所属车间
    "workshop" VARCHAR(100),
    -- 计量单位
    "unit" VARCHAR(32) NOT NULL,
    -- 上次读数
    "previous_reading" DECIMAL(14,2) NOT NULL DEFAULT 0,
    -- 当前读数
    "current_reading" DECIMAL(14,2) NOT NULL DEFAULT 0,
    -- 消耗量（current_reading - previous_reading）
    "consumption" DECIMAL(14,2) NOT NULL DEFAULT 0,
    -- 单价
    "unit_price" DECIMAL(12,4) NOT NULL DEFAULT 0,
    -- 总成本（consumption × unit_price）
    "total_cost" DECIMAL(14,2) NOT NULL DEFAULT 0,
    -- 记录时段开始
    "period_start" TIMESTAMPTZ NOT NULL,
    -- 记录时段结束
    "period_end" TIMESTAMPTZ NOT NULL,
    -- 录入方式：manual(手工) / iot(IoT 自动) / auto_calc(自动计算)
    "recording_method" VARCHAR(32) NOT NULL DEFAULT 'manual',
    -- 关联缸号（直接归集时使用）
    "dye_lot_no" VARCHAR(64),
    -- 关联工序路线 ID（按工序归集时使用）
    "process_route_id" INTEGER,
    -- 工序编码（冗余）
    "route_code" VARCHAR(32),
    -- 关联设备 ID
    "equipment_id" INTEGER,
    -- 设备名称（冗余）
    "equipment_name" VARCHAR(128),
    -- 操作员 ID
    "operator_id" INTEGER,
    -- 录入时间
    "recorded_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- 状态：draft(草稿) → confirmed(已确认) → cancelled(已取消)
    "status" VARCHAR(32) NOT NULL DEFAULT 'draft',
    -- 备注
    "remarks" VARCHAR(512),
    -- 软删除与审计
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 唯一约束：记录编号
CREATE UNIQUE INDEX IF NOT EXISTS "uk_energy_consumption_no" ON "energy_consumption_record" ("record_no") WHERE "is_deleted" = FALSE;
-- 索引：按计量设备查询
CREATE INDEX IF NOT EXISTS "idx_energy_consumption_meter" ON "energy_consumption_record" ("meter_id") WHERE "is_deleted" = FALSE;
-- 索引：按能源类型查询
CREATE INDEX IF NOT EXISTS "idx_energy_consumption_type" ON "energy_consumption_record" ("meter_type") WHERE "is_deleted" = FALSE;
-- 索引：按车间查询
CREATE INDEX IF NOT EXISTS "idx_energy_consumption_workshop" ON "energy_consumption_record" ("workshop") WHERE "is_deleted" = FALSE;
-- 索引：按缸号查询（缸号归集）
CREATE INDEX IF NOT EXISTS "idx_energy_consumption_dyelot" ON "energy_consumption_record" ("dye_lot_no") WHERE "is_deleted" = FALSE AND "dye_lot_no" IS NOT NULL;
-- 索引：按工序路线查询
CREATE INDEX IF NOT EXISTS "idx_energy_consumption_route" ON "energy_consumption_record" ("process_route_id") WHERE "is_deleted" = FALSE AND "process_route_id" IS NOT NULL;
-- 索引：按设备查询
CREATE INDEX IF NOT EXISTS "idx_energy_consumption_equipment" ON "energy_consumption_record" ("equipment_id") WHERE "is_deleted" = FALSE AND "equipment_id" IS NOT NULL;
-- 索引：按记录时段查询（月末汇总）
CREATE INDEX IF NOT EXISTS "idx_energy_consumption_period" ON "energy_consumption_record" ("period_start", "period_end") WHERE "is_deleted" = FALSE;
-- 索引：按状态查询
CREATE INDEX IF NOT EXISTS "idx_energy_consumption_status" ON "energy_consumption_record" ("status") WHERE "is_deleted" = FALSE;
-- 索引：按录入方式查询
CREATE INDEX IF NOT EXISTS "idx_energy_consumption_method" ON "energy_consumption_record" ("recording_method") WHERE "is_deleted" = FALSE;

-- 外键约束
ALTER TABLE "energy_consumption_record" ADD CONSTRAINT "fk_energy_consumption_meter"
    FOREIGN KEY ("meter_id") REFERENCES "energy_meter" ("id") ON UPDATE CASCADE ON DELETE SET NULL;
ALTER TABLE "energy_consumption_record" ADD CONSTRAINT "fk_energy_consumption_route"
    FOREIGN KEY ("process_route_id") REFERENCES "process_route" ("id") ON UPDATE CASCADE ON DELETE SET NULL;

COMMENT ON TABLE "energy_consumption_record" IS '能耗记录表（时间段登记，可关联缸号/工序/订单）';
COMMENT ON COLUMN "energy_consumption_record"."recording_method" IS '录入方式 manual/iot/auto_calc';
COMMENT ON COLUMN "energy_consumption_record"."dye_lot_no" IS '关联缸号（直接归集时使用）';

-- ============================================================================
-- 3. 能耗分摊规则表（energy_allocation_rule）
-- 真实业务：定义如何将车间总能耗分摊到缸号/工序/订单
-- 分摊基准：by_duration(按工时) / by_output(按产量) / by_equipment(按设备) / by_workshop(按车间)
-- ============================================================================
CREATE TABLE IF NOT EXISTS "energy_allocation_rule" (
    "id" SERIAL PRIMARY KEY,
    -- 规则编号：EAR-YYYYMMDDHHMMSS-NNN
    "rule_no" VARCHAR(64) NOT NULL,
    -- 规则名称
    "rule_name" VARCHAR(128) NOT NULL,
    -- 能源类型
    "meter_type" VARCHAR(32) NOT NULL,
    -- 分摊基准：by_duration(按工时) / by_output(按产量) / by_equipment(按设备) / by_workshop(按车间)
    "allocation_basis" VARCHAR(32) NOT NULL,
    -- 所属车间
    "workshop" VARCHAR(100),
    -- 关联工序路线 ID（按工序归集时使用）
    "process_route_id" INTEGER,
    -- 工序编码（冗余）
    "route_code" VARCHAR(32),
    -- 生效日期
    "effective_date" DATE NOT NULL,
    -- 失效日期（NULL 表示长期有效）
    "expiry_date" DATE,
    -- 标准单位能耗（每米布/每缸/每公斤的理论消耗）
    "standard_consumption_per_unit" DECIMAL(14,4) NOT NULL DEFAULT 0,
    -- 标准单位（米/缸/公斤/小时）
    "standard_unit" VARCHAR(32),
    -- 状态：draft(草稿) → active(启用) → disabled(停用)
    "status" VARCHAR(32) NOT NULL DEFAULT 'draft',
    -- 备注
    "remarks" VARCHAR(512),
    -- 软删除与审计
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 唯一约束：规则编号
CREATE UNIQUE INDEX IF NOT EXISTS "uk_energy_allocation_rule_no" ON "energy_allocation_rule" ("rule_no") WHERE "is_deleted" = FALSE;
-- 唯一约束：同车间同能源类型同工序同生效日期只能有一个规则
CREATE UNIQUE INDEX IF NOT EXISTS "uk_energy_allocation_rule_unique" ON "energy_allocation_rule" ("workshop", "meter_type", "process_route_id", "effective_date") WHERE "is_deleted" = FALSE;
-- 索引：按能源类型查询
CREATE INDEX IF NOT EXISTS "idx_energy_allocation_rule_type" ON "energy_allocation_rule" ("meter_type") WHERE "is_deleted" = FALSE;
-- 索引：按车间查询
CREATE INDEX IF NOT EXISTS "idx_energy_allocation_rule_workshop" ON "energy_allocation_rule" ("workshop") WHERE "is_deleted" = FALSE;
-- 索引：按工序路线查询
CREATE INDEX IF NOT EXISTS "idx_energy_allocation_rule_route" ON "energy_allocation_rule" ("process_route_id") WHERE "is_deleted" = FALSE AND "process_route_id" IS NOT NULL;
-- 索引：按状态查询
CREATE INDEX IF NOT EXISTS "idx_energy_allocation_rule_status" ON "energy_allocation_rule" ("status") WHERE "is_deleted" = FALSE;
-- 索引：按分摊基准查询
CREATE INDEX IF NOT EXISTS "idx_energy_allocation_rule_basis" ON "energy_allocation_rule" ("allocation_basis") WHERE "is_deleted" = FALSE;

-- 外键约束
ALTER TABLE "energy_allocation_rule" ADD CONSTRAINT "fk_energy_allocation_rule_route"
    FOREIGN KEY ("process_route_id") REFERENCES "process_route" ("id") ON UPDATE CASCADE ON DELETE SET NULL;

COMMENT ON TABLE "energy_allocation_rule" IS '能耗分摊规则表（定义分摊基准和标准消耗）';
COMMENT ON COLUMN "energy_allocation_rule"."allocation_basis" IS '分摊基准 by_duration/by_output/by_equipment/by_workshop';
COMMENT ON COLUMN "energy_allocation_rule"."standard_consumption_per_unit" IS '标准单位能耗（用于超基准预警）';

-- ============================================================================
-- 4. 能耗分摊记录表（energy_allocation_record）
-- 真实业务：月末将总能耗按规则分摊到缸号/工序/订单，生成 cost_collection 记录
-- ============================================================================
CREATE TABLE IF NOT EXISTS "energy_allocation_record" (
    "id" SERIAL PRIMARY KEY,
    -- 分摊编号：EAR-YYYYMMDDHHMMSS-NNN（与规则编号前缀相同，但表不同）
    "allocation_no" VARCHAR(64) NOT NULL,
    -- 分摊时段开始
    "period_start" TIMESTAMPTZ NOT NULL,
    -- 分摊时段结束
    "period_end" TIMESTAMPTZ NOT NULL,
    -- 能源类型
    "meter_type" VARCHAR(32) NOT NULL,
    -- 所属车间
    "workshop" VARCHAR(100),
    -- 关联分摊规则 ID
    "allocation_rule_id" INTEGER,
    -- 分摊基准（冗余）
    "allocation_basis" VARCHAR(32) NOT NULL,
    -- 总消耗量
    "total_consumption" DECIMAL(14,2) NOT NULL DEFAULT 0,
    -- 总成本
    "total_cost" DECIMAL(14,2) NOT NULL DEFAULT 0,
    -- 关联缸号（分摊到具体缸号时使用）
    "dye_lot_no" VARCHAR(64),
    -- 关联生产订单 ID
    "production_order_id" INTEGER,
    -- 生产订单编号（冗余）
    "production_order_no" VARCHAR(50),
    -- 关联工序路线 ID
    "process_route_id" INTEGER,
    -- 工序编码（冗余）
    "route_code" VARCHAR(32),
    -- 关联流转卡 ID
    "flow_card_id" INTEGER,
    -- 分摊依据量（工时/产量/设备运行时长）
    "allocation_basis_value" DECIMAL(14,2) NOT NULL DEFAULT 0,
    -- 分摊比例（0-1）
    "allocation_ratio" DECIMAL(8,4) NOT NULL DEFAULT 0,
    -- 分摊消耗量
    "allocated_consumption" DECIMAL(14,2) NOT NULL DEFAULT 0,
    -- 分摊成本
    "allocated_cost" DECIMAL(14,2) NOT NULL DEFAULT 0,
    -- 单位产量（米/公斤，用于单位能耗分析）
    "output_quantity" DECIMAL(14,2),
    -- 单位能耗（allocated_consumption / output_quantity）
    "unit_consumption" DECIMAL(14,4),
    -- 关联成本归集 ID（月末分摊到成本时生成）
    "cost_collection_id" INTEGER,
    -- 状态：draft(草稿) → confirmed(已确认) → cancelled(已取消)
    "status" VARCHAR(32) NOT NULL DEFAULT 'draft',
    -- 确认人
    "confirmed_by" INTEGER,
    -- 确认时间
    "confirmed_at" TIMESTAMPTZ,
    -- 备注
    "remarks" VARCHAR(512),
    -- 软删除与审计
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 唯一约束：分摊编号
CREATE UNIQUE INDEX IF NOT EXISTS "uk_energy_allocation_record_no" ON "energy_allocation_record" ("allocation_no") WHERE "is_deleted" = FALSE;
-- 索引：按能源类型查询
CREATE INDEX IF NOT EXISTS "idx_energy_allocation_record_type" ON "energy_allocation_record" ("meter_type") WHERE "is_deleted" = FALSE;
-- 索引：按车间查询
CREATE INDEX IF NOT EXISTS "idx_energy_allocation_record_workshop" ON "energy_allocation_record" ("workshop") WHERE "is_deleted" = FALSE;
-- 索引：按缸号查询
CREATE INDEX IF NOT EXISTS "idx_energy_allocation_record_dyelot" ON "energy_allocation_record" ("dye_lot_no") WHERE "is_deleted" = FALSE AND "dye_lot_no" IS NOT NULL;
-- 索引：按生产订单查询
CREATE INDEX IF NOT EXISTS "idx_energy_allocation_record_order" ON "energy_allocation_record" ("production_order_id") WHERE "is_deleted" = FALSE AND "production_order_id" IS NOT NULL;
-- 索引：按工序路线查询
CREATE INDEX IF NOT EXISTS "idx_energy_allocation_record_route" ON "energy_allocation_record" ("process_route_id") WHERE "is_deleted" = FALSE AND "process_route_id" IS NOT NULL;
-- 索引：按分摊时段查询
CREATE INDEX IF NOT EXISTS "idx_energy_allocation_record_period" ON "energy_allocation_record" ("period_start", "period_end") WHERE "is_deleted" = FALSE;
-- 索引：按状态查询
CREATE INDEX IF NOT EXISTS "idx_energy_allocation_record_status" ON "energy_allocation_record" ("status") WHERE "is_deleted" = FALSE;
-- 索引：按成本归集 ID 查询
CREATE INDEX IF NOT EXISTS "idx_energy_allocation_record_cost" ON "energy_allocation_record" ("cost_collection_id") WHERE "is_deleted" = FALSE AND "cost_collection_id" IS NOT NULL;
-- 索引：按分摊规则查询
CREATE INDEX IF NOT EXISTS "idx_energy_allocation_record_rule" ON "energy_allocation_record" ("allocation_rule_id") WHERE "is_deleted" = FALSE AND "allocation_rule_id" IS NOT NULL;

-- 外键约束
ALTER TABLE "energy_allocation_record" ADD CONSTRAINT "fk_energy_allocation_record_rule"
    FOREIGN KEY ("allocation_rule_id") REFERENCES "energy_allocation_rule" ("id") ON UPDATE CASCADE ON DELETE SET NULL;

COMMENT ON TABLE "energy_allocation_record" IS '能耗分摊记录表（月末分摊到缸号/工序/订单）';
COMMENT ON COLUMN "energy_allocation_record"."allocation_basis" IS '分摊基准 by_duration/by_output/by_equipment/by_workshop';
COMMENT ON COLUMN "energy_allocation_record"."allocated_cost" IS '分摊成本（关联 cost_collection 时同步更新）';
COMMENT ON COLUMN "energy_allocation_record"."cost_collection_id" IS '关联成本归集 ID（月末分摊到成本时生成）';
