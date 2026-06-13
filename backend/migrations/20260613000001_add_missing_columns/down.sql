-- 回滚：移除添加的缺失列

-- ============================================
-- 4. omni_audit_logs: 移除添加的列
-- ============================================
DROP INDEX IF EXISTS "idx_omni_audit_logs_tenant";
ALTER TABLE "omni_audit_logs" DROP COLUMN IF EXISTS "new_value";
ALTER TABLE "omni_audit_logs" DROP COLUMN IF EXISTS "old_value";
ALTER TABLE "omni_audit_logs" DROP COLUMN IF EXISTS "request_body";
ALTER TABLE "omni_audit_logs" DROP COLUMN IF EXISTS "request_path";
ALTER TABLE "omni_audit_logs" DROP COLUMN IF EXISTS "request_method";
ALTER TABLE "omni_audit_logs" DROP COLUMN IF EXISTS "user_agent";
ALTER TABLE "omni_audit_logs" DROP COLUMN IF EXISTS "ip_address";
ALTER TABLE "omni_audit_logs" DROP COLUMN IF EXISTS "description";
ALTER TABLE "omni_audit_logs" DROP COLUMN IF EXISTS "resource_name";
ALTER TABLE "omni_audit_logs" DROP COLUMN IF EXISTS "resource_id";
ALTER TABLE "omni_audit_logs" DROP COLUMN IF EXISTS "resource_type";
ALTER TABLE "omni_audit_logs" DROP COLUMN IF EXISTS "username";
ALTER TABLE "omni_audit_logs" DROP COLUMN IF EXISTS "parent_span_id";
ALTER TABLE "omni_audit_logs" DROP COLUMN IF EXISTS "span_id";
ALTER TABLE "omni_audit_logs" DROP COLUMN IF EXISTS "tenant_id";

-- ============================================
-- 3. currencies: 移除添加的列
-- ============================================
ALTER TABLE "currencies" DROP COLUMN IF EXISTS "is_deleted";
ALTER TABLE "currencies" DROP COLUMN IF EXISTS "is_active";
ALTER TABLE "currencies" DROP COLUMN IF EXISTS "precision";

-- ============================================
-- 2. bpm_task: 移除添加的列
-- ============================================
ALTER TABLE "bpm_task" DROP COLUMN IF EXISTS "remarks";
ALTER TABLE "bpm_task" DROP COLUMN IF EXISTS "task_variables";
ALTER TABLE "bpm_task" DROP COLUMN IF EXISTS "form_data";
ALTER TABLE "bpm_task" DROP COLUMN IF EXISTS "overdue_days";
ALTER TABLE "bpm_task" DROP COLUMN IF EXISTS "is_overdue";
ALTER TABLE "bpm_task" DROP COLUMN IF EXISTS "due_date";
ALTER TABLE "bpm_task" DROP COLUMN IF EXISTS "duration_seconds";
ALTER TABLE "bpm_task" DROP COLUMN IF EXISTS "handled_at";
ALTER TABLE "bpm_task" DROP COLUMN IF EXISTS "attachment_urls";
ALTER TABLE "bpm_task" DROP COLUMN IF EXISTS "approval_opinion";
ALTER TABLE "bpm_task" DROP COLUMN IF EXISTS "action";
ALTER TABLE "bpm_task" DROP COLUMN IF EXISTS "actual_handler_name";
ALTER TABLE "bpm_task" DROP COLUMN IF EXISTS "actual_handler_id";
ALTER TABLE "bpm_task" DROP COLUMN IF EXISTS "candidate_user_ids";
ALTER TABLE "bpm_task" DROP COLUMN IF EXISTS "candidate_role_ids";
ALTER TABLE "bpm_task" DROP COLUMN IF EXISTS "assignee_names";
ALTER TABLE "bpm_task" DROP COLUMN IF EXISTS "assignee_ids";
ALTER TABLE "bpm_task" DROP COLUMN IF EXISTS "priority";
ALTER TABLE "bpm_task" DROP COLUMN IF EXISTS "node_type";
ALTER TABLE "bpm_task" DROP COLUMN IF EXISTS "instance_id";

-- ============================================
-- 1. inventory_stocks: 还原列结构
-- ============================================

-- 移除面料行业特色字段
ALTER TABLE "inventory_stocks" DROP COLUMN IF EXISTS "quality_status";
ALTER TABLE "inventory_stocks" DROP COLUMN IF EXISTS "stock_status";
ALTER TABLE "inventory_stocks" DROP COLUMN IF EXISTS "layer_no";
ALTER TABLE "inventory_stocks" DROP COLUMN IF EXISTS "shelf_no";
ALTER TABLE "inventory_stocks" DROP COLUMN IF EXISTS "location_id";
ALTER TABLE "inventory_stocks" DROP COLUMN IF EXISTS "width";
ALTER TABLE "inventory_stocks" DROP COLUMN IF EXISTS "gram_weight";
ALTER TABLE "inventory_stocks" DROP COLUMN IF EXISTS "quantity_kg";
ALTER TABLE "inventory_stocks" DROP COLUMN IF EXISTS "quantity_meters";
ALTER TABLE "inventory_stocks" DROP COLUMN IF EXISTS "expiry_date";
ALTER TABLE "inventory_stocks" DROP COLUMN IF EXISTS "production_date";
ALTER TABLE "inventory_stocks" DROP COLUMN IF EXISTS "grade";
ALTER TABLE "inventory_stocks" DROP COLUMN IF EXISTS "dye_lot_no";
ALTER TABLE "inventory_stocks" DROP COLUMN IF EXISTS "color_no";

-- 移除其他新增列
ALTER TABLE "inventory_stocks" DROP COLUMN IF EXISTS "last_movement_date";
ALTER TABLE "inventory_stocks" DROP COLUMN IF EXISTS "last_count_date";
ALTER TABLE "inventory_stocks" DROP COLUMN IF EXISTS "bin_location";
ALTER TABLE "inventory_stocks" DROP COLUMN IF EXISTS "reorder_quantity";
ALTER TABLE "inventory_stocks" DROP COLUMN IF EXISTS "reorder_point";
ALTER TABLE "inventory_stocks" DROP COLUMN IF EXISTS "quantity_incoming";
ALTER TABLE "inventory_stocks" DROP COLUMN IF EXISTS "quantity_shipped";
ALTER TABLE "inventory_stocks" DROP COLUMN IF EXISTS "quantity_available";

-- 还原列名
ALTER TABLE "inventory_stocks" RENAME COLUMN "quantity_reserved" TO "reserved_quantity";
ALTER TABLE "inventory_stocks" RENAME COLUMN "quantity_on_hand" TO "quantity";

-- 恢复生成列
ALTER TABLE "inventory_stocks" ADD COLUMN "available_quantity" INTEGER GENERATED ALWAYS AS (quantity - reserved_quantity) STORED;
