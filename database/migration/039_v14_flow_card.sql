-- v14 批次 425：流转卡工序流转模块
-- 依据：面料行业真实业务调研文档 §14.1 流转卡工序流转（基于同凯印染 ERP/KESHTECH 真实开卡字段）
-- 真实业务流程：
--   流转卡定义：流转卡=生产流程卡/工序流转卡/缸卡，一卡对应一缸布的生产任务，
--              承载从开卡到成品入库的全部工序信息。流转卡=缸号+工单信息+工序路线+计划配布数量+条码。
--   扫码签入签出（PDA/工控终端）：
--     签入：扫码→识别工单号/缸号/工序路线→工人刷卡登记→记录工号、设备编号、开始时间→状态：待加工→加工中
--     签出：扫码→记录结束时间、实际产量→状态：加工中→完工
--     流转：完工→转入下道（系统自动触发下一道工序开工准备）
--     入库：完工→完工入库（PDA 扫描卷唛条码）
--   分卡/合卡/拆卡（KESHTECH 真实业务）：
--     分卡(split_card)：机缸容量不足，将坯布分成两部分分别染色，生成新卡号
--     合缸(merge_card)：多张小卡合并为一缸染色，共享缸号但保留各自卡号
--     拆卡(split_piece)：一匹布过长拆分为多匹，生成子卡号关联母卡号
--     缸终止(terminate_card)：因质量/工艺问题终止该缸生产
--   内修卡（KESHTECH 真实业务）：
--     内修卡号 = 原始卡号 + A/B/C 后缀（一次回修+A，二次回修+B，以此类推）
--     开内修卡前必须先在"质量异常登记"里登记
-- 来源：印染厂车间流转卡操作规程 / KESHTECH 染整 ERP 流转卡模块（2026-07-15 真实调研）

-- ============================================================================
-- 1. 流转卡主表（flow_card）
-- 业务来源：业务员/计划员接到生产订单后开具流转卡，承载一缸布从开卡到入库的全部工序信息
-- 真实必填字段：flow_card_no(卡号)、status(卡状态)
-- 关键约束：同一 dye_lot_no 缸号仅允许一张未删除的主卡（业务校验在 Service 层实现）
-- ============================================================================
CREATE TABLE IF NOT EXISTS "flow_card" (
    "id" SERIAL PRIMARY KEY,
    -- 卡号：FC-YYYYMMDDHHMMSS-NNN（唯一）
    "flow_card_no" VARCHAR(64) NOT NULL,
    -- 条码（Code128 格式字符串，扫码用）
    "barcode" VARCHAR(128),
    -- 缸号（一缸布的标识，合缸时多卡共享）
    "dye_lot_no" VARCHAR(64),
    -- 关联工单 ID
    "work_order_id" INTEGER,
    -- 关联生产订单 ID
    "production_order_id" INTEGER,
    -- 客户 ID
    "customer_id" INTEGER,
    -- 业务员 ID
    "salesman_id" INTEGER,
    -- 坯布 ID
    "greige_fabric_id" INTEGER,
    -- 布种
    "fabric_type" VARCHAR(64),
    -- 纱支
    "yarn_count" VARCHAR(32),
    -- 成分
    "composition" VARCHAR(128),
    -- 克重 g/m²
    "gram_weight" DECIMAL(10,2),
    -- 门幅 cm
    "fabric_width" DECIMAL(10,2),
    -- 色号
    "color_no" VARCHAR(64),
    -- 色名
    "color_name" VARCHAR(128),
    -- 对色光源（D65/TL84/U3000/CWF/A 等）
    "light_source" VARCHAR(32),
    -- 开卡匹数
    "planned_pieces" INTEGER,
    -- 计划总重 kg
    "planned_weight_kg" DECIMAL(12,2),
    -- 配布数量
    "planned_quantity" DECIMAL(12,2),
    -- 实际匹数
    "actual_pieces" INTEGER DEFAULT 0,
    -- 实际总重
    "actual_weight_kg" DECIMAL(12,2),
    -- 工序路线 JSON：[{sequence, name, status}]
    "process_route" JSONB,
    -- 当前工序
    "current_process" VARCHAR(64),
    -- 交货期
    "delivery_date" DATE,
    -- 仓位
    "warehouse_position" VARCHAR(64),
    -- 卡状态机：opened → waiting_dyeing → scheduled → preparing → dyeing → dyed
    --         → inspecting → stored → shipped；分支：paused / rework / terminated / cancelled
    "status" VARCHAR(32) NOT NULL DEFAULT 'opened',
    -- 原始卡号（回修卡关联）
    "original_card_id" INTEGER,
    -- 回修次数
    "rework_count" INTEGER DEFAULT 0,
    -- 母卡号（拆卡关联）
    "parent_card_id" INTEGER,
    -- 是否回修卡
    "is_rework" BOOLEAN NOT NULL DEFAULT false,
    -- 软删除与审计
    "is_deleted" BOOLEAN NOT NULL DEFAULT false,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 卡号唯一
CREATE UNIQUE INDEX IF NOT EXISTS "idx_flow_card_no" ON "flow_card" ("flow_card_no");
-- 缸号索引（按缸号查询流转卡）
CREATE INDEX IF NOT EXISTS "idx_flow_card_dye_lot_no" ON "flow_card" ("dye_lot_no");
-- 工单外键索引
CREATE INDEX IF NOT EXISTS "idx_flow_card_work_order" ON "flow_card" ("work_order_id");
-- 生产订单外键索引
CREATE INDEX IF NOT EXISTS "idx_flow_card_production_order" ON "flow_card" ("production_order_id");
-- 客户外键索引
CREATE INDEX IF NOT EXISTS "idx_flow_card_customer" ON "flow_card" ("customer_id");
-- 坯布外键索引
CREATE INDEX IF NOT EXISTS "idx_flow_card_greige_fabric" ON "flow_card" ("greige_fabric_id");
-- 状态索引
CREATE INDEX IF NOT EXISTS "idx_flow_card_status" ON "flow_card" ("status");
-- 原始卡号索引（回修卡关联查询）
CREATE INDEX IF NOT EXISTS "idx_flow_card_original_card" ON "flow_card" ("original_card_id");
-- 母卡号索引（拆卡关联查询）
CREATE INDEX IF NOT EXISTS "idx_flow_card_parent_card" ON "flow_card" ("parent_card_id");
-- 交货期索引（按交期排产）
CREATE INDEX IF NOT EXISTS "idx_flow_card_delivery_date" ON "flow_card" ("delivery_date");
-- 软删除过滤索引
CREATE INDEX IF NOT EXISTS "idx_flow_card_not_deleted" ON "flow_card" ("is_deleted") WHERE "is_deleted" = false;
-- 条码索引（扫码查询）
CREATE INDEX IF NOT EXISTS "idx_flow_card_barcode" ON "flow_card" ("barcode");

-- ============================================================================
-- 2. 工序操作记录表（flow_card_operation）
-- 业务来源：PDA/工控终端扫码签入签出，记录每道工序的操作员/设备/产量/疵点
-- 关键约束：同一流转卡同一工序序号仅允许一条记录（业务校验在 Service 层实现）
-- ============================================================================
CREATE TABLE IF NOT EXISTS "flow_card_operation" (
    "id" SERIAL PRIMARY KEY,
    -- 流转卡 ID
    "flow_card_id" INTEGER NOT NULL,
    -- 工序序号
    "process_sequence" INTEGER NOT NULL,
    -- 工序名称
    "process_name" VARCHAR(64) NOT NULL,
    -- 操作员 ID
    "operator_id" INTEGER,
    -- 设备编号
    "equipment_id" VARCHAR(64),
    -- 工序状态机：pending → in_progress → completed → transferred → stored
    --           分支：paused / rework
    "status" VARCHAR(32) NOT NULL DEFAULT 'pending',
    -- 签入时间
    "sign_in_at" TIMESTAMPTZ,
    -- 签出时间
    "sign_out_at" TIMESTAMPTZ,
    -- 实际产量
    "actual_quantity" DECIMAL(12,2),
    -- 实际匹数
    "actual_pieces" INTEGER,
    -- 疵点数
    "defect_count" INTEGER DEFAULT 0,
    -- 备注
    "remarks" VARCHAR(512),
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 流转卡外键索引
CREATE INDEX IF NOT EXISTS "idx_flow_card_op_card" ON "flow_card_operation" ("flow_card_id");
-- 操作员索引
CREATE INDEX IF NOT EXISTS "idx_flow_card_op_operator" ON "flow_card_operation" ("operator_id");
-- 工序状态索引
CREATE INDEX IF NOT EXISTS "idx_flow_card_op_status" ON "flow_card_operation" ("status");
-- 签入时间索引（按时间统计产能）
CREATE INDEX IF NOT EXISTS "idx_flow_card_op_sign_in" ON "flow_card_operation" ("sign_in_at");
-- 同卡同工序唯一约束（一卡一道工序仅一条记录）
CREATE UNIQUE INDEX IF NOT EXISTS "idx_flow_card_op_card_seq" ON "flow_card_operation" ("flow_card_id", "process_sequence");

-- ============================================================================
-- 3. 外键约束（DO 块幂等）
-- ============================================================================
DO $$
BEGIN
    -- flow_card.work_order_id → production_orders.id
    IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_flow_card_work_order') THEN
        ALTER TABLE "flow_card" ADD CONSTRAINT "fk_flow_card_work_order"
            FOREIGN KEY ("work_order_id") REFERENCES "production_orders" ("id") ON DELETE SET NULL;
    END IF;
    -- flow_card.production_order_id → production_orders.id
    IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_flow_card_production_order') THEN
        ALTER TABLE "flow_card" ADD CONSTRAINT "fk_flow_card_production_order"
            FOREIGN KEY ("production_order_id") REFERENCES "production_orders" ("id") ON DELETE SET NULL;
    END IF;
    -- flow_card.customer_id → customers.id
    IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_flow_card_customer') THEN
        ALTER TABLE "flow_card" ADD CONSTRAINT "fk_flow_card_customer"
            FOREIGN KEY ("customer_id") REFERENCES "customers" ("id") ON DELETE SET NULL;
    END IF;
    -- flow_card.greige_fabric_id → greige_fabric.id
    IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_flow_card_greige_fabric') THEN
        ALTER TABLE "flow_card" ADD CONSTRAINT "fk_flow_card_greige_fabric"
            FOREIGN KEY ("greige_fabric_id") REFERENCES "greige_fabric" ("id") ON DELETE SET NULL;
    END IF;

    -- flow_card_operation.flow_card_id → flow_card.id
    IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_flow_card_op_flow_card') THEN
        ALTER TABLE "flow_card_operation" ADD CONSTRAINT "fk_flow_card_op_flow_card"
            FOREIGN KEY ("flow_card_id") REFERENCES "flow_card" ("id") ON DELETE CASCADE;
    END IF;
END $$;
