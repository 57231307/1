-- 修复模型与数据库 schema 不一致的问题
-- 添加代码中使用但数据库中缺失的列

-- ============================================
-- 1. inventory_stocks: 添加 quantity_on_hand 及其他缺失列
-- ============================================

-- 删除旧的生成列（依赖 quantity 和 reserved_quantity）
ALTER TABLE "inventory_stocks" DROP COLUMN IF EXISTS "available_quantity";

-- 重命名旧列以匹配新模型
ALTER TABLE "inventory_stocks" RENAME COLUMN "quantity" TO "quantity_on_hand";
ALTER TABLE "inventory_stocks" RENAME COLUMN "reserved_quantity" TO "quantity_reserved";

-- 添加新列
ALTER TABLE "inventory_stocks" ADD COLUMN "quantity_available" DECIMAL(12, 2) NOT NULL DEFAULT 0;
ALTER TABLE "inventory_stocks" ADD COLUMN "quantity_shipped" DECIMAL(12, 2) NOT NULL DEFAULT 0;
ALTER TABLE "inventory_stocks" ADD COLUMN "quantity_incoming" DECIMAL(12, 2) NOT NULL DEFAULT 0;
ALTER TABLE "inventory_stocks" ADD COLUMN "reorder_point" DECIMAL(12, 2) NOT NULL DEFAULT 0;
ALTER TABLE "inventory_stocks" ADD COLUMN "reorder_quantity" DECIMAL(12, 2) NOT NULL DEFAULT 0;
ALTER TABLE "inventory_stocks" ADD COLUMN "bin_location" VARCHAR(100);
ALTER TABLE "inventory_stocks" ADD COLUMN "last_count_date" TIMESTAMPTZ;
ALTER TABLE "inventory_stocks" ADD COLUMN "last_movement_date" TIMESTAMPTZ;

-- 面料行业特色字段
ALTER TABLE "inventory_stocks" ADD COLUMN "color_no" VARCHAR(50) NOT NULL DEFAULT '';
ALTER TABLE "inventory_stocks" ADD COLUMN "dye_lot_no" VARCHAR(50);
ALTER TABLE "inventory_stocks" ADD COLUMN "grade" VARCHAR(20) NOT NULL DEFAULT '一等品';
ALTER TABLE "inventory_stocks" ADD COLUMN "production_date" TIMESTAMPTZ;
ALTER TABLE "inventory_stocks" ADD COLUMN "expiry_date" TIMESTAMPTZ;
ALTER TABLE "inventory_stocks" ADD COLUMN "quantity_meters" DECIMAL(12, 2) NOT NULL DEFAULT 0;
ALTER TABLE "inventory_stocks" ADD COLUMN "quantity_kg" DECIMAL(12, 2) NOT NULL DEFAULT 0;
ALTER TABLE "inventory_stocks" ADD COLUMN "gram_weight" DECIMAL(10, 2);
ALTER TABLE "inventory_stocks" ADD COLUMN "width" DECIMAL(10, 2);
ALTER TABLE "inventory_stocks" ADD COLUMN "location_id" INTEGER;
ALTER TABLE "inventory_stocks" ADD COLUMN "shelf_no" VARCHAR(50);
ALTER TABLE "inventory_stocks" ADD COLUMN "layer_no" VARCHAR(50);
ALTER TABLE "inventory_stocks" ADD COLUMN "stock_status" VARCHAR(20) NOT NULL DEFAULT '正常';
ALTER TABLE "inventory_stocks" ADD COLUMN "quality_status" VARCHAR(20) NOT NULL DEFAULT '合格';

COMMENT ON COLUMN "inventory_stocks"."quantity_on_hand" IS '在手数量';
COMMENT ON COLUMN "inventory_stocks"."quantity_available" IS '可用数量';
COMMENT ON COLUMN "inventory_stocks"."quantity_reserved" IS '预留数量';
COMMENT ON COLUMN "inventory_stocks"."quantity_shipped" IS '已发货数量';
COMMENT ON COLUMN "inventory_stocks"."quantity_incoming" IS '在途数量';
COMMENT ON COLUMN "inventory_stocks"."reorder_point" IS '再订货点';
COMMENT ON COLUMN "inventory_stocks"."reorder_quantity" IS '再订货量';
COMMENT ON COLUMN "inventory_stocks"."bin_location" IS '仓位';
COMMENT ON COLUMN "inventory_stocks"."color_no" IS '色号';
COMMENT ON COLUMN "inventory_stocks"."dye_lot_no" IS '缸号';
COMMENT ON COLUMN "inventory_stocks"."grade" IS '等级';
COMMENT ON COLUMN "inventory_stocks"."quantity_meters" IS '数量（米）- 主计量单位';
COMMENT ON COLUMN "inventory_stocks"."quantity_kg" IS '数量（公斤）- 辅计量单位';
COMMENT ON COLUMN "inventory_stocks"."gram_weight" IS '克重（g/m²）';
COMMENT ON COLUMN "inventory_stocks"."width" IS '幅宽（cm）';
COMMENT ON COLUMN "inventory_stocks"."location_id" IS '库位 ID';
COMMENT ON COLUMN "inventory_stocks"."shelf_no" IS '货架号';
COMMENT ON COLUMN "inventory_stocks"."layer_no" IS '层号';
COMMENT ON COLUMN "inventory_stocks"."stock_status" IS '库存状态：正常/冻结/待检';
COMMENT ON COLUMN "inventory_stocks"."quality_status" IS '质量状态：合格/不合格/待检';

-- ============================================
-- 2. bpm_task: 添加 instance_id 列
-- ============================================
ALTER TABLE "bpm_task" ADD COLUMN "instance_id" INTEGER;
ALTER TABLE "bpm_task" ADD COLUMN "node_type" VARCHAR(50) NOT NULL DEFAULT 'APPROVAL';
ALTER TABLE "bpm_task" ADD COLUMN "priority" VARCHAR(20);
ALTER TABLE "bpm_task" ADD COLUMN "assignee_ids" INTEGER[];
ALTER TABLE "bpm_task" ADD COLUMN "assignee_names" TEXT[];
ALTER TABLE "bpm_task" ADD COLUMN "candidate_role_ids" INTEGER[];
ALTER TABLE "bpm_task" ADD COLUMN "candidate_user_ids" INTEGER[];
ALTER TABLE "bpm_task" ADD COLUMN "actual_handler_id" INTEGER;
ALTER TABLE "bpm_task" ADD COLUMN "actual_handler_name" VARCHAR(100);
ALTER TABLE "bpm_task" ADD COLUMN "action" VARCHAR(50);
ALTER TABLE "bpm_task" ADD COLUMN "approval_opinion" TEXT;
ALTER TABLE "bpm_task" ADD COLUMN "attachment_urls" TEXT[];
ALTER TABLE "bpm_task" ADD COLUMN "handled_at" TIMESTAMPTZ;
ALTER TABLE "bpm_task" ADD COLUMN "duration_seconds" BIGINT;
ALTER TABLE "bpm_task" ADD COLUMN "due_date" TIMESTAMPTZ;
ALTER TABLE "bpm_task" ADD COLUMN "is_overdue" BOOLEAN;
ALTER TABLE "bpm_task" ADD COLUMN "overdue_days" INTEGER;
ALTER TABLE "bpm_task" ADD COLUMN "form_data" JSONB;
ALTER TABLE "bpm_task" ADD COLUMN "task_variables" JSONB;
ALTER TABLE "bpm_task" ADD COLUMN "remarks" TEXT;

COMMENT ON COLUMN "bpm_task"."instance_id" IS '流程实例ID';
COMMENT ON COLUMN "bpm_task"."node_type" IS '节点类型';
COMMENT ON COLUMN "bpm_task"."priority" IS '优先级';
COMMENT ON COLUMN "bpm_task"."assignee_ids" IS '处理人ID列表';
COMMENT ON COLUMN "bpm_task"."assignee_names" IS '处理人姓名列表';
COMMENT ON COLUMN "bpm_task"."actual_handler_id" IS '实际处理人ID';
COMMENT ON COLUMN "bpm_task"."actual_handler_name" IS '实际处理人姓名';
COMMENT ON COLUMN "bpm_task"."action" IS '审批动作';
COMMENT ON COLUMN "bpm_task"."approval_opinion" IS '审批意见';
COMMENT ON COLUMN "bpm_task"."form_data" IS '表单数据';
COMMENT ON COLUMN "bpm_task"."task_variables" IS '任务变量';
COMMENT ON COLUMN "bpm_task"."remarks" IS '备注';

-- ============================================
-- 3. currencies: 添加 precision 列
-- ============================================
ALTER TABLE "currencies" ADD COLUMN "precision" INTEGER DEFAULT 2;
ALTER TABLE "currencies" ADD COLUMN "is_active" BOOLEAN DEFAULT true;
ALTER TABLE "currencies" ADD COLUMN "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE;

COMMENT ON COLUMN "currencies"."precision" IS '小数精度';

-- ============================================
-- 4. omni_audit_logs: 添加 tenant_id 及其他缺失列
-- ============================================
ALTER TABLE "omni_audit_logs" ADD COLUMN "tenant_id" INTEGER;
ALTER TABLE "omni_audit_logs" ADD COLUMN "span_id" VARCHAR(50);
ALTER TABLE "omni_audit_logs" ADD COLUMN "parent_span_id" VARCHAR(50);
ALTER TABLE "omni_audit_logs" ADD COLUMN "username" VARCHAR(100);
ALTER TABLE "omni_audit_logs" ADD COLUMN "resource_type" VARCHAR(100);
ALTER TABLE "omni_audit_logs" ADD COLUMN "resource_id" VARCHAR(100);
ALTER TABLE "omni_audit_logs" ADD COLUMN "resource_name" VARCHAR(200);
ALTER TABLE "omni_audit_logs" ADD COLUMN "description" TEXT;
ALTER TABLE "omni_audit_logs" ADD COLUMN "ip_address" VARCHAR(50);
ALTER TABLE "omni_audit_logs" ADD COLUMN "user_agent" TEXT;
ALTER TABLE "omni_audit_logs" ADD COLUMN "request_method" VARCHAR(10);
ALTER TABLE "omni_audit_logs" ADD COLUMN "request_path" VARCHAR(500);
ALTER TABLE "omni_audit_logs" ADD COLUMN "request_body" TEXT;
ALTER TABLE "omni_audit_logs" ADD COLUMN "old_value" JSONB;
ALTER TABLE "omni_audit_logs" ADD COLUMN "new_value" JSONB;

CREATE INDEX IF NOT EXISTS "idx_omni_audit_logs_tenant" ON "omni_audit_logs" ("tenant_id");

COMMENT ON COLUMN "omni_audit_logs"."tenant_id" IS '租户ID';
COMMENT ON COLUMN "omni_audit_logs"."span_id" IS '链路追踪 span ID';
COMMENT ON COLUMN "omni_audit_logs"."parent_span_id" IS '父 span ID';
COMMENT ON COLUMN "omni_audit_logs"."username" IS '用户名';
COMMENT ON COLUMN "omni_audit_logs"."resource_type" IS '资源类型';
COMMENT ON COLUMN "omni_audit_logs"."resource_id" IS '资源ID';
COMMENT ON COLUMN "omni_audit_logs"."resource_name" IS '资源名称';
COMMENT ON COLUMN "omni_audit_logs"."description" IS '操作描述';
COMMENT ON COLUMN "omni_audit_logs"."ip_address" IS 'IP地址';
COMMENT ON COLUMN "omni_audit_logs"."user_agent" IS '用户代理';
COMMENT ON COLUMN "omni_audit_logs"."request_method" IS '请求方法';
COMMENT ON COLUMN "omni_audit_logs"."request_path" IS '请求路径';
COMMENT ON COLUMN "omni_audit_logs"."request_body" IS '请求体';
COMMENT ON COLUMN "omni_audit_logs"."old_value" IS '变更前的值';
COMMENT ON COLUMN "omni_audit_logs"."new_value" IS '变更后的值';
