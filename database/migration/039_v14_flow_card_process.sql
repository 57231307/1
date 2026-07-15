-- v14 批次 425：流转卡条码与车间工序流转
-- 依据：面料行业真实业务调研文档 §12.1 流转卡条码管理 + §12.2 生产计划与排缸 + §12.3 车间工序流转
-- 真实业务流程：
--   生产计划单 → 备布 → 排缸执行 → 流转卡打印（含条码）
--   白坯仓库：扫描缸号条码 → 自动出库
--   染色车间：扫描缸卡条码 → 输入生产进度（进缸/出缸状态）
--   称料室：  扫描流转卡条码 → 加载大货处方 → 称料
--   车间流转：扫描流转卡条码 → 登记工人 → 自动跟进工序和产量
--   成品入库：PDA 扫描卷唛条码 → 自动入库
--   发货：    输入或扫描缸号 → 获取缸号所有信息 → 发货
-- 真实业务要点：
--   1. 流转卡承载：缸号(唯一标识) + 订单信息 + 染整要求 + 工序路线 + 计划配布数量 + 条码
--   2. 工序路线后台自定义：根据车间布局配置关键工序
--   3. 扫码登记工人后自动统计产量
--   4. 工序质量反馈单：异常/回修/处理意见登记
--   5. 缸号状态机：待排缸→已排缸→备布中→进缸染色→出缸→验布→入库→发货
-- 来源：同凯染色 ERP / 环思印染 ERP / 飞创纺织 ERP（2026-07 真实业务资料）

-- ============================================================================
-- 1. 工序路线模板表（process_route）
-- 业务来源：后台自定义车间工序，根据实际车间布局配置关键工序
-- 真实业务：前处理→染色→印花→后整理→验布（可增删调整顺序）
-- ============================================================================
CREATE TABLE IF NOT EXISTS "process_route" (
    "id" SERIAL PRIMARY KEY,
    -- 工序编码（如 PRE_TREAT/DYE/PRINT/FINISH/INSPECT）
    "route_code" VARCHAR(32) NOT NULL,
    -- 工序名称（如 前处理/染色/印花/后整理/验布）
    "route_name" VARCHAR(64) NOT NULL,
    -- 工序序号（流转顺序，1=第一道工序）
    "seq" INTEGER NOT NULL,
    -- 工序类型：pretreat(前处理)/dye(染色)/print(印花)/finish(后整理)/inspect(验布)/other
    "process_type" VARCHAR(32) NOT NULL,
    -- 默认工时（分钟）
    "default_duration_minutes" INTEGER,
    -- 是否需要扫码确认
    "require_scan" BOOLEAN NOT NULL DEFAULT TRUE,
    -- 是否启用
    "is_active" BOOLEAN NOT NULL DEFAULT TRUE,
    -- 备注
    "remarks" VARCHAR(256),
    -- 软删除与审计
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 唯一约束：工序编码唯一
CREATE UNIQUE INDEX IF NOT EXISTS "uk_process_route_code" ON "process_route" ("route_code") WHERE "is_deleted" = FALSE;
-- 索引：按启用状态+序号查询
CREATE INDEX IF NOT EXISTS "idx_process_route_active_seq" ON "process_route" ("is_active", "seq") WHERE "is_deleted" = FALSE;

-- ============================================================================
-- 2. 生产流转卡表（production_flow_card）
-- 业务来源：生产计划单生成流转卡，打印含条码，车间各环节扫码操作
-- 真实业务：一缸一卡，卡随缸走，扫码即获取全部信息
-- ============================================================================
CREATE TABLE IF NOT EXISTS "production_flow_card" (
    "id" SERIAL PRIMARY KEY,
    -- 流转卡号：FC-YYYYMMDDHHMMSS-NNN
    "card_no" VARCHAR(64) NOT NULL,
    -- 流转卡条码（一维码/二维码内容，全局唯一）
    "barcode" VARCHAR(128) NOT NULL,
    -- 关联生产订单 ID
    "production_order_id" INTEGER NOT NULL,
    -- 关联缸号（dye_batch.id）
    "dye_batch_id" INTEGER,
    -- 缸号字符串（冗余便于扫码查询，dye_batch.batch_no）
    "dye_lot_no" VARCHAR(64),
    -- 关联工序路线 ID
    "process_route_id" INTEGER,
    -- 客户信息（冗余自订单，便于流转卡直接展示）
    "customer_id" INTEGER,
    "customer_name" VARCHAR(128),
    -- 订单号（冗余）
    "order_no" VARCHAR(64),
    -- 产品信息（冗余）
    "product_id" INTEGER,
    "product_name" VARCHAR(128),
    -- 色号
    "color_no" VARCHAR(64),
    -- 染整要求与注意事项
    "dyeing_requirements" VARCHAR(512),
    -- 计划配布数量（kg）
    "planned_fabric_weight" DECIMAL(12,2),
    -- 实际配布数量（kg，备布后回填）
    "actual_fabric_weight" DECIMAL(12,2),
    -- 当前工序序号
    "current_step_seq" INTEGER NOT NULL DEFAULT 1,
    -- 流转卡状态：pending(待排缸) → scheduled(已排缸) → preparing(备布中) → dyeing(染色中) → dyed(已出缸) → inspecting(验布中) → completed(已完成) → shipped(已发货) → terminated(已终止)
    "status" VARCHAR(32) NOT NULL DEFAULT 'pending',
    -- 排缸时间
    "scheduled_at" TIMESTAMPTZ,
    -- 备布完成时间
    "prepared_at" TIMESTAMPTZ,
    -- 进缸时间
    "dye_start_at" TIMESTAMPTZ,
    -- 出缸时间
    "dye_end_at" TIMESTAMPTZ,
    -- 验布时间
    "inspected_at" TIMESTAMPTZ,
    -- 完成时间
    "completed_at" TIMESTAMPTZ,
    -- 发货时间
    "shipped_at" TIMESTAMPTZ,
    -- 优先级（数字越大越优先）
    "priority" INTEGER NOT NULL DEFAULT 0,
    -- 备注
    "remarks" VARCHAR(512),
    -- 软删除与审计
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 唯一约束：流转卡号唯一
CREATE UNIQUE INDEX IF NOT EXISTS "uk_flow_card_no" ON "production_flow_card" ("card_no") WHERE "is_deleted" = FALSE;
-- 唯一约束：条码全局唯一
CREATE UNIQUE INDEX IF NOT EXISTS "uk_flow_card_barcode" ON "production_flow_card" ("barcode") WHERE "is_deleted" = FALSE;
-- 索引：按缸号查询
CREATE INDEX IF NOT EXISTS "idx_flow_card_dye_lot" ON "production_flow_card" ("dye_lot_no") WHERE "is_deleted" = FALSE;
-- 索引：按生产订单查询
CREATE INDEX IF NOT EXISTS "idx_flow_card_order" ON "production_flow_card" ("production_order_id") WHERE "is_deleted" = FALSE;
-- 索引：按状态查询
CREATE INDEX IF NOT EXISTS "idx_flow_card_status" ON "production_flow_card" ("status") WHERE "is_deleted" = FALSE;
-- 索引：按客户+产品查询
CREATE INDEX IF NOT EXISTS "idx_flow_card_customer_product" ON "production_flow_card" ("customer_id", "product_id") WHERE "is_deleted" = FALSE;

-- 外键约束
ALTER TABLE "production_flow_card" ADD CONSTRAINT "fk_flow_card_production_order"
    FOREIGN KEY ("production_order_id") REFERENCES "production_orders" ("id") ON UPDATE CASCADE ON DELETE RESTRICT;
ALTER TABLE "production_flow_card" ADD CONSTRAINT "fk_flow_card_dye_batch"
    FOREIGN KEY ("dye_batch_id") REFERENCES "dye_batch" ("id") ON UPDATE CASCADE ON DELETE SET NULL;
ALTER TABLE "production_flow_card" ADD CONSTRAINT "fk_flow_card_process_route"
    FOREIGN KEY ("process_route_id") REFERENCES "process_route" ("id") ON UPDATE CASCADE ON DELETE SET NULL;
ALTER TABLE "production_flow_card" ADD CONSTRAINT "fk_flow_card_customer"
    FOREIGN KEY ("customer_id") REFERENCES "customers" ("id") ON UPDATE CASCADE ON DELETE SET NULL;
ALTER TABLE "production_flow_card" ADD CONSTRAINT "fk_flow_card_product"
    FOREIGN KEY ("product_id") REFERENCES "products" ("id") ON UPDATE CASCADE ON DELETE RESTRICT;

-- ============================================================================
-- 3. 工序流转记录表（process_step_record）
-- 业务来源：扫描流转卡条码 → 登记工人 → 自动跟进工序和产量
-- 真实业务：每道工序扫码登记开始/结束，记录工人+产量+异常
-- ============================================================================
CREATE TABLE IF NOT EXISTS "process_step_record" (
    "id" SERIAL PRIMARY KEY,
    -- 关联流转卡 ID
    "flow_card_id" INTEGER NOT NULL,
    -- 关联工序路线 ID
    "process_route_id" INTEGER,
    -- 工序序号
    "step_seq" INTEGER NOT NULL,
    -- 工序编码
    "route_code" VARCHAR(32) NOT NULL,
    -- 工序名称
    "route_name" VARCHAR(64) NOT NULL,
    -- 工序类型
    "process_type" VARCHAR(32) NOT NULL,
    -- 操作工人 ID（可多个，逗号分隔）
    "worker_ids" VARCHAR(256),
    -- 工人姓名（冗余，便于报表）
    "worker_names" VARCHAR(512),
    -- 设备/机台 ID
    "equipment_id" INTEGER,
    -- 设备名称（冗余）
    "equipment_name" VARCHAR(128),
    -- 开始时间
    "start_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- 结束时间
    "end_at" TIMESTAMPTZ,
    -- 工时（分钟，end_at - start_at 计算）
    "duration_minutes" INTEGER,
    -- 计划产量（kg/m，从流转卡计划配布数量继承）
    "planned_quantity" DECIMAL(12,2),
    -- 实际产量（kg/m，工人上报）
    "actual_quantity" DECIMAL(12,2),
    -- 合格产量（kg/m，扣除疵点）
    "qualified_quantity" DECIMAL(12,2),
    -- 状态：pending(待开始) → in_progress(进行中) → completed(已完成) → abnormal(异常) → rework(回修)
    "status" VARCHAR(32) NOT NULL DEFAULT 'pending',
    -- 异常情况描述
    "abnormal_description" VARCHAR(512),
    -- 处理意见和方式
    "handling_opinion" VARCHAR(512),
    -- 回修关联原工序记录 ID（回修时指向原记录）
    "rework_source_id" INTEGER,
    -- 备注
    "remarks" VARCHAR(512),
    -- 软删除与审计
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 索引：按流转卡查询工序记录
CREATE INDEX IF NOT EXISTS "idx_step_record_flow_card" ON "process_step_record" ("flow_card_id") WHERE "is_deleted" = FALSE;
-- 索引：按流转卡+工序序号查询
CREATE INDEX IF NOT EXISTS "idx_step_record_card_seq" ON "process_step_record" ("flow_card_id", "step_seq") WHERE "is_deleted" = FALSE;
-- 索引：按工人查询（产量统计）
CREATE INDEX IF NOT EXISTS "idx_step_record_worker" ON "process_step_record" ("worker_ids") WHERE "is_deleted" = FALSE;
-- 索引：按状态查询
CREATE INDEX IF NOT EXISTS "idx_step_record_status" ON "process_step_record" ("status") WHERE "is_deleted" = FALSE;
-- 索引：按工序类型查询
CREATE INDEX IF NOT EXISTS "idx_step_record_type" ON "process_step_record" ("process_type") WHERE "is_deleted" = FALSE;
-- 索引：按时间范围查询（产量统计）
CREATE INDEX IF NOT EXISTS "idx_step_record_start_at" ON "process_step_record" ("start_at") WHERE "is_deleted" = FALSE;

-- 外键约束
ALTER TABLE "process_step_record" ADD CONSTRAINT "fk_step_record_flow_card"
    FOREIGN KEY ("flow_card_id") REFERENCES "production_flow_card" ("id") ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE "process_step_record" ADD CONSTRAINT "fk_step_record_process_route"
    FOREIGN KEY ("process_route_id") REFERENCES "process_route" ("id") ON UPDATE CASCADE ON DELETE SET NULL;
-- 回修自引用外键
ALTER TABLE "process_step_record" ADD CONSTRAINT "fk_step_record_rework_source"
    FOREIGN KEY ("rework_source_id") REFERENCES "process_step_record" ("id") ON UPDATE CASCADE ON DELETE SET NULL;

-- ============================================================================
-- 4. 工序质量反馈单表（process_quality_feedback）
-- 业务来源：工序质量问题反馈登记，包括处理意见和方式，异常情况、回修情况登记
-- 真实业务：各生产环节共同查找原因及处理办法
-- ============================================================================
CREATE TABLE IF NOT EXISTS "process_quality_feedback" (
    "id" SERIAL PRIMARY KEY,
    -- 反馈单号：QF-YYYYMMDDHHMMSS-NNN
    "feedback_no" VARCHAR(64) NOT NULL,
    -- 关联流转卡 ID
    "flow_card_id" INTEGER NOT NULL,
    -- 关联工序记录 ID
    "step_record_id" INTEGER,
    -- 反馈类型：abnormal(异常) / rework(回修) / defect(疵点) / other(其他)
    "feedback_type" VARCHAR(32) NOT NULL,
    -- 问题描述
    "description" VARCHAR(1024) NOT NULL,
    -- 严重等级：low(低) / medium(中) / high(高) / critical(严重)
    "severity" VARCHAR(16) NOT NULL DEFAULT 'medium',
    -- 发现人 ID
    "found_by" INTEGER,
    -- 发现时间
    "found_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- 处理意见和方式
    "handling_opinion" VARCHAR(1024),
    -- 处理人 ID
    "handled_by" INTEGER,
    -- 处理时间
    "handled_at" TIMESTAMPTZ,
    -- 处理结果
    "handling_result" VARCHAR(1024),
    -- 状态：pending(待处理) → processing(处理中) → resolved(已解决) → closed(已关闭)
    "status" VARCHAR(32) NOT NULL DEFAULT 'pending',
    -- 备注
    "remarks" VARCHAR(512),
    -- 软删除与审计
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 唯一约束：反馈单号唯一
CREATE UNIQUE INDEX IF NOT EXISTS "uk_quality_feedback_no" ON "process_quality_feedback" ("feedback_no") WHERE "is_deleted" = FALSE;
-- 索引：按流转卡查询
CREATE INDEX IF NOT EXISTS "idx_quality_feedback_flow_card" ON "process_quality_feedback" ("flow_card_id") WHERE "is_deleted" = FALSE;
-- 索引：按状态查询
CREATE INDEX IF NOT EXISTS "idx_quality_feedback_status" ON "process_quality_feedback" ("status") WHERE "is_deleted" = FALSE;
-- 索引：按类型查询
CREATE INDEX IF NOT EXISTS "idx_quality_feedback_type" ON "process_quality_feedback" ("feedback_type") WHERE "is_deleted" = FALSE;
-- 索引：按严重等级查询
CREATE INDEX IF NOT EXISTS "idx_quality_feedback_severity" ON "process_quality_feedback" ("severity") WHERE "is_deleted" = FALSE;

-- 外键约束
ALTER TABLE "process_quality_feedback" ADD CONSTRAINT "fk_quality_feedback_flow_card"
    FOREIGN KEY ("flow_card_id") REFERENCES "production_flow_card" ("id") ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE "process_quality_feedback" ADD CONSTRAINT "fk_quality_feedback_step_record"
    FOREIGN KEY ("step_record_id") REFERENCES "process_step_record" ("id") ON UPDATE CASCADE ON DELETE SET NULL;

-- ============================================================================
-- 初始化默认工序路线（前处理→染色→印花→后整理→验布）
-- 真实业务：标准染整工艺流程，可根据车间布局调整
-- ============================================================================
INSERT INTO "process_route" ("route_code", "route_name", "seq", "process_type", "default_duration_minutes", "require_scan", "is_active", "remarks") VALUES
    ('PRE_TREAT', '前处理', 1, 'pretreat', 120, TRUE, TRUE, '烧毛/退浆/煮练/漂白/丝光'),
    ('DYE', '染色', 2, 'dye', 240, TRUE, TRUE, '进缸染色/保温/出缸'),
    ('PRINT', '印花', 3, 'print', 180, FALSE, TRUE, '制版/调浆/印花/汽蒸/水洗（非必经工序）'),
    ('FINISH', '后整理', 4, 'finish', 120, TRUE, TRUE, '定型/预缩/抗皱/防水等功能整理'),
    ('INSPECT', '验布', 5, 'inspect', 90, TRUE, TRUE, '验布/打卷/分级/入库')
ON CONFLICT DO NOTHING;
