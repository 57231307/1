//! 核心表结构：角色/部门/用户/权限
//!
//! 合并自: 6 个迁移文件

use sea_orm_migration::prelude::*;

// 导入所有迁移模块
mod m0001_initial_schema;
mod m0002_add_crm_and_greige_tables;
mod m0003_add_dye_tables;
mod m0004_add_field_permissions;
mod m0005_add_basic_data_and_system_tables;
mod m0006_add_general_ledger_and_finance_base;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_system_domain"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 依次执行所有迁移
        m0001_initial_schema::Migration.up(manager).await?;
        m0002_add_crm_and_greige_tables::Migration
            .up(manager)
            .await?;
        m0003_add_dye_tables::Migration.up(manager).await?;
        m0004_add_field_permissions::Migration.up(manager).await?;
        m0005_add_basic_data_and_system_tables::Migration
            .up(manager)
            .await?;
        m0006_add_general_ledger_and_finance_base::Migration
            .up(manager)
            .await?;
        let sql = r#"ALTER TABLE "accounting_periods" ADD COLUMN IF NOT EXISTS "close_ip" VARCHAR(255);
ALTER TABLE "accounting_periods" ADD COLUMN IF NOT EXISTS "close_remark" VARCHAR(255);
ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "created_by" INTEGER;
ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "description" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "after_snapshot" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "before_snapshot" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "condition" TEXT;
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "description" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "duration_ms" INTEGER;
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "export_approval_token" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "export_file_format" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "export_query_filter" TEXT;
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "export_record_count" INTEGER;
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "export_watermark_user" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "ip_address" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "new_value" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "old_value" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "operation_type" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "request_body" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "request_id" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "request_method" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "request_path" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "resource_id" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "resource_name" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "resource_type" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "response_status" INTEGER;
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "severity" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "user_agent" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "username" VARCHAR(255);
ALTER TABLE "bpm_process_instance" ADD COLUMN IF NOT EXISTS "completed_at" TIMESTAMPTZ;
ALTER TABLE "bpm_process_instance" ADD COLUMN IF NOT EXISTS "current_handler_ids" JSONB;
ALTER TABLE "bpm_process_instance" ADD COLUMN IF NOT EXISTS "current_handler_names" JSONB;
ALTER TABLE "bpm_process_instance" ADD COLUMN IF NOT EXISTS "current_node_id" VARCHAR(255);
ALTER TABLE "bpm_process_instance" ADD COLUMN IF NOT EXISTS "current_node_name" VARCHAR(255);
ALTER TABLE "bpm_process_instance" ADD COLUMN IF NOT EXISTS "duration_seconds" BIGINT;
ALTER TABLE "bpm_process_instance" ADD COLUMN IF NOT EXISTS "form_data" JSONB;
ALTER TABLE "bpm_process_instance" ADD COLUMN IF NOT EXISTS "initiator_department_id" INTEGER;
ALTER TABLE "bpm_process_instance" ADD COLUMN IF NOT EXISTS "initiator_id" INTEGER;
ALTER TABLE "bpm_process_instance" ADD COLUMN IF NOT EXISTS "initiator_name" VARCHAR(255);
ALTER TABLE "bpm_process_instance" ADD COLUMN IF NOT EXISTS "priority" VARCHAR(255);
ALTER TABLE "bpm_process_instance" ADD COLUMN IF NOT EXISTS "remarks" VARCHAR(255);
ALTER TABLE "bpm_process_instance" ADD COLUMN IF NOT EXISTS "started_at" TIMESTAMPTZ;
ALTER TABLE "bpm_process_instance" ADD COLUMN IF NOT EXISTS "title" VARCHAR(255);
ALTER TABLE "bpm_task" ADD COLUMN IF NOT EXISTS "action" VARCHAR(255);
ALTER TABLE "bpm_task" ADD COLUMN IF NOT EXISTS "actual_handler_id" INTEGER;
ALTER TABLE "bpm_task" ADD COLUMN IF NOT EXISTS "actual_handler_name" VARCHAR(255);
ALTER TABLE "bpm_task" ADD COLUMN IF NOT EXISTS "approval_opinion" VARCHAR(255);
ALTER TABLE "bpm_task" ADD COLUMN IF NOT EXISTS "assignee_ids" JSONB;
ALTER TABLE "bpm_task" ADD COLUMN IF NOT EXISTS "assignee_names" JSONB;
ALTER TABLE "bpm_task" ADD COLUMN IF NOT EXISTS "attachment_urls" JSONB;
ALTER TABLE "bpm_task" ADD COLUMN IF NOT EXISTS "candidate_role_ids" JSONB;
ALTER TABLE "bpm_task" ADD COLUMN IF NOT EXISTS "candidate_user_ids" JSONB;
ALTER TABLE "bpm_task" ADD COLUMN IF NOT EXISTS "due_date" TIMESTAMPTZ;
ALTER TABLE "bpm_task" ADD COLUMN IF NOT EXISTS "duration_seconds" BIGINT;
ALTER TABLE "bpm_task" ADD COLUMN IF NOT EXISTS "form_data" JSONB;
ALTER TABLE "bpm_task" ADD COLUMN IF NOT EXISTS "handled_at" TIMESTAMPTZ;
ALTER TABLE "bpm_task" ADD COLUMN IF NOT EXISTS "instance_id" INTEGER;
ALTER TABLE "bpm_task" ADD COLUMN IF NOT EXISTS "is_overdue" BOOLEAN;
ALTER TABLE "bpm_task" ADD COLUMN IF NOT EXISTS "node_type" VARCHAR(255);
ALTER TABLE "bpm_task" ADD COLUMN IF NOT EXISTS "overdue_days" INTEGER;
ALTER TABLE "bpm_task" ADD COLUMN IF NOT EXISTS "priority" VARCHAR(255);
ALTER TABLE "bpm_task" ADD COLUMN IF NOT EXISTS "process_definition_id" INTEGER;
ALTER TABLE "bpm_task" ADD COLUMN IF NOT EXISTS "remarks" VARCHAR(255);
ALTER TABLE "bpm_task" ADD COLUMN IF NOT EXISTS "task_variables" JSONB;
ALTER TABLE "currencies" ADD COLUMN IF NOT EXISTS "is_active" BOOLEAN;
ALTER TABLE "currencies" ADD COLUMN IF NOT EXISTS "is_deleted" BOOLEAN;
ALTER TABLE "currencies" ADD COLUMN IF NOT EXISTS "precision" INTEGER;
ALTER TABLE "dye_batch" ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(255);
ALTER TABLE "dye_batch" ADD COLUMN IF NOT EXISTS "completed_at" TIMESTAMPTZ;
ALTER TABLE "dye_batch" ADD COLUMN IF NOT EXISTS "dye_lot_no" VARCHAR(255);
ALTER TABLE "dye_batch" ADD COLUMN IF NOT EXISTS "greige_fabric_id" INTEGER;
ALTER TABLE "dye_batch" ADD COLUMN IF NOT EXISTS "is_deleted" BOOLEAN;
ALTER TABLE "dye_batch" ADD COLUMN IF NOT EXISTS "planned_quantity" DECIMAL(12,2);
ALTER TABLE "dye_batch" ADD COLUMN IF NOT EXISTS "started_at" TIMESTAMPTZ;
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "approved_at" TIMESTAMPTZ;
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "approved_by" INTEGER;
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "auxiliaries" JSONB;
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "chemical_formula" VARCHAR(255);
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "color_name" VARCHAR(255);
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(255);
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "dye_type" VARCHAR(255);
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "fabric_type" VARCHAR(255);
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "formula" VARCHAR(255);
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "is_deleted" BOOLEAN;
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "liquor_ratio" DECIMAL(10,2);
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "parent_recipe_id" INTEGER;
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "ph_value" DECIMAL(5,2);
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "recipe_no" VARCHAR(255);
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "remarks" VARCHAR(255);
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "status" VARCHAR(255);
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "temperature" DECIMAL(5,2);
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "time_minutes" INTEGER;
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "version" INTEGER;
ALTER TABLE "exchange_rates" ADD COLUMN IF NOT EXISTS "is_deleted" BOOLEAN;
ALTER TABLE "exchange_rates" ADD COLUMN IF NOT EXISTS "source" VARCHAR(255);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(255);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "composition" VARCHAR(255);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "density" VARCHAR(255);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "dye_lot_no" VARCHAR(255);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "gram_weight" DECIMAL(10,2);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "max_stock_point" DECIMAL(12,2);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "product_id" INTEGER;
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "production_date" DATE;
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "purchase_order_id" INTEGER;
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "purchase_receipt_id" INTEGER;
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "quantity_kg" DECIMAL(12,2);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "quantity_meters" DECIMAL(12,2);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "reorder_point" DECIMAL(12,2);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "reorder_quantity" DECIMAL(12,2);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "safety_stock" DECIMAL(12,2);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "structure" VARCHAR(255);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "width" DECIMAL(10,2);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "yarn_count" VARCHAR(255);
ALTER TABLE "log_system" ADD COLUMN IF NOT EXISTS "error_message" VARCHAR(255);
ALTER TABLE "log_system" ADD COLUMN IF NOT EXISTS "execution_time" BIGINT;
ALTER TABLE "log_system" ADD COLUMN IF NOT EXISTS "ip_address" VARCHAR(255);
ALTER TABLE "log_system" ADD COLUMN IF NOT EXISTS "method" VARCHAR(255);
ALTER TABLE "log_system" ADD COLUMN IF NOT EXISTS "operation" VARCHAR(255);
ALTER TABLE "log_system" ADD COLUMN IF NOT EXISTS "params" VARCHAR(255);
ALTER TABLE "log_system" ADD COLUMN IF NOT EXISTS "path" VARCHAR(255);
ALTER TABLE "log_system" ADD COLUMN IF NOT EXISTS "status_code" INTEGER;
ALTER TABLE "log_system" ADD COLUMN IF NOT EXISTS "user_agent" VARCHAR(255);
ALTER TABLE "log_system" ADD COLUMN IF NOT EXISTS "user_id" INTEGER;
ALTER TABLE "log_system" ADD COLUMN IF NOT EXISTS "username" VARCHAR(255);
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "condition" TEXT;
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "description" VARCHAR(255);
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "export_approval_token" VARCHAR(255);
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "export_record_count" INTEGER;
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "ip_address" VARCHAR(255);
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "new_value" JSONB;
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "old_value" JSONB;
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "operation_category" VARCHAR(255);
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "parent_span_id" VARCHAR(255);
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "request_body" VARCHAR(255);
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "request_method" VARCHAR(255);
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "request_path" VARCHAR(255);
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "resource_id" VARCHAR(255);
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "resource_name" VARCHAR(255);
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "resource_type" VARCHAR(255);
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "signature" VARCHAR(255);
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "span_id" VARCHAR(255);
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "user_agent" VARCHAR(255);
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "username" VARCHAR(255);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "auxiliary_type" VARCHAR(255);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(255);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "customer_id" INTEGER;
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "defect_type" VARCHAR(255);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "dye_lot_no" VARCHAR(255);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "dye_type" VARCHAR(255);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "fabric_source" VARCHAR(255);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "grade" VARCHAR(255);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "inspected_qty" DECIMAL(18,4);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "inspection_result" VARCHAR(255);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "qualification_rate" DECIMAL(18,4);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "related_id" INTEGER;
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "related_type" VARCHAR(255);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "remark" VARCHAR(255);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "temperature" DECIMAL(18,4);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "total_qty" DECIMAL(18,4);
ALTER TABLE "role_permissions" ADD COLUMN IF NOT EXISTS "permission_code" VARCHAR(255);
ALTER TABLE "webhooks" ADD COLUMN IF NOT EXISTS "last_event" VARCHAR(255);
ALTER TABLE "webhooks" ADD COLUMN IF NOT EXISTS "last_payload" VARCHAR(255);
ALTER TABLE "webhooks" ADD COLUMN IF NOT EXISTS "user_id" INTEGER;
"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        let sql = r#"
ALTER TABLE "customers" ADD COLUMN IF NOT EXISTS "annual_purchase" DECIMAL(14,2);
ALTER TABLE "customers" ADD COLUMN IF NOT EXISTS "bank_account" VARCHAR(255);
ALTER TABLE "customers" ADD COLUMN IF NOT EXISTS "bank_name" VARCHAR(255);
ALTER TABLE "customers" ADD COLUMN IF NOT EXISTS "city" VARCHAR(255);
ALTER TABLE "customers" ADD COLUMN IF NOT EXISTS "contact_email" VARCHAR(255);
ALTER TABLE "customers" ADD COLUMN IF NOT EXISTS "contact_person" VARCHAR(255);
ALTER TABLE "customers" ADD COLUMN IF NOT EXISTS "contact_phone" VARCHAR(255);
ALTER TABLE "customers" ADD COLUMN IF NOT EXISTS "country" VARCHAR(255);
ALTER TABLE "customers" ADD COLUMN IF NOT EXISTS "created_by" INTEGER;
ALTER TABLE "customers" ADD COLUMN IF NOT EXISTS "customer_code" VARCHAR(255);
ALTER TABLE "customers" ADD COLUMN IF NOT EXISTS "customer_industry" VARCHAR(255);
ALTER TABLE "customers" ADD COLUMN IF NOT EXISTS "customer_name" VARCHAR(255);
ALTER TABLE "customers" ADD COLUMN IF NOT EXISTS "inspection_standard" VARCHAR(255);
ALTER TABLE "customers" ADD COLUMN IF NOT EXISTS "main_products" VARCHAR(255);
ALTER TABLE "customers" ADD COLUMN IF NOT EXISTS "notes" VARCHAR(255);
ALTER TABLE "customers" ADD COLUMN IF NOT EXISTS "owner_assigned_at" TIMESTAMPTZ;
ALTER TABLE "customers" ADD COLUMN IF NOT EXISTS "owner_id" INTEGER;
ALTER TABLE "customers" ADD COLUMN IF NOT EXISTS "payment_terms" INTEGER;
ALTER TABLE "customers" ADD COLUMN IF NOT EXISTS "pool_recycle_reason" VARCHAR(255);
ALTER TABLE "customers" ADD COLUMN IF NOT EXISTS "postal_code" VARCHAR(255);
ALTER TABLE "customers" ADD COLUMN IF NOT EXISTS "province" VARCHAR(255);
ALTER TABLE "customers" ADD COLUMN IF NOT EXISTS "quality_requirement" VARCHAR(255);
ALTER TABLE "customers" ADD COLUMN IF NOT EXISTS "source" VARCHAR(255);
ALTER TABLE "customers" ADD COLUMN IF NOT EXISTS "special_process" VARCHAR(255);
ALTER TABLE "customers" ADD COLUMN IF NOT EXISTS "status" VARCHAR(255);
ALTER TABLE "customers" ADD COLUMN IF NOT EXISTS "tax_id" VARCHAR(255);
ALTER TABLE "finance_payments" ADD COLUMN IF NOT EXISTS "invoice_id" INTEGER;
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(255);
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "dye_lot_no" VARCHAR(255);
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "notes" VARCHAR(255);
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "quantity_actual" DECIMAL(10,2);
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "quantity_before" DECIMAL(10,2);
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "quantity_difference" DECIMAL(10,2);
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "stock_id" INTEGER;
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "total_cost" DECIMAL(12,2);
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "warehouse_id" INTEGER;
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "approved_at" TIMESTAMPTZ;
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "completed_at" TIMESTAMPTZ;
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "counted_items" INTEGER;
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "total_items" INTEGER;
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "variance_items" INTEGER;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "bin_location" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "dye_lot_no" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "expiry_date" TIMESTAMPTZ;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "grade" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "gram_weight" DECIMAL(10,2);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "last_count_date" TIMESTAMPTZ;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "last_movement_date" TIMESTAMPTZ;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "layer_no" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "location_id" INTEGER;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "max_stock_point" DECIMAL(12,2);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "production_date" TIMESTAMPTZ;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "quality_status" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "quantity_available" DECIMAL(18,4);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "quantity_incoming" DECIMAL(18,4);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "quantity_kg" DECIMAL(12,2);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "quantity_meters" DECIMAL(12,2);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "quantity_on_hand" DECIMAL(18,4);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "quantity_reserved" DECIMAL(18,4);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "quantity_shipped" DECIMAL(18,4);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "reorder_point" DECIMAL(18,4);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "reorder_quantity" DECIMAL(18,4);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "replenishment_strategy" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "shelf_no" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "stock_status" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "version" INTEGER;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "width" DECIMAL(10,2);
ALTER TABLE "inventory_transfer_items" ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(255);
ALTER TABLE "inventory_transfer_items" ADD COLUMN IF NOT EXISTS "dye_lot_no" VARCHAR(255);
ALTER TABLE "inventory_transfer_items" ADD COLUMN IF NOT EXISTS "notes" VARCHAR(255);
ALTER TABLE "inventory_transfer_items" ADD COLUMN IF NOT EXISTS "received_quantity" DECIMAL(18,4);
ALTER TABLE "inventory_transfer_items" ADD COLUMN IF NOT EXISTS "shipped_quantity" DECIMAL(18,4);
ALTER TABLE "inventory_transfer_items" ADD COLUMN IF NOT EXISTS "unit_cost" DECIMAL(18,4);
ALTER TABLE "inventory_transfer_items" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "inventory_transfers" ADD COLUMN IF NOT EXISTS "approval_level" VARCHAR(255);
ALTER TABLE "inventory_transfers" ADD COLUMN IF NOT EXISTS "approved_at" TIMESTAMPTZ;
ALTER TABLE "inventory_transfers" ADD COLUMN IF NOT EXISTS "approved_by_role" VARCHAR(255);
ALTER TABLE "inventory_transfers" ADD COLUMN IF NOT EXISTS "received_at" TIMESTAMPTZ;
ALTER TABLE "inventory_transfers" ADD COLUMN IF NOT EXISTS "shipped_at" TIMESTAMPTZ;
ALTER TABLE "inventory_transfers" ADD COLUMN IF NOT EXISTS "total_amount" DECIMAL(18,4);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "batch_level" VARCHAR(255);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "code" VARCHAR(255);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "cost_price" DECIMAL(18,4);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "density" VARCHAR(255);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "description" VARCHAR(255);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "execution_standard" VARCHAR(255);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "fabric_composition" VARCHAR(255);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "factory_address" VARCHAR(255);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "factory_name" VARCHAR(255);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "finish" VARCHAR(255);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "gram_weight" DECIMAL(10,2);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "is_batch_managed" BOOLEAN;
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "lead_time" INTEGER;
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "min_order_quantity" DECIMAL(12,2);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "product_grade" VARCHAR(255);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "product_type" VARCHAR(255);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "specification" VARCHAR(255);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "standard_price" DECIMAL(18,4);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "status" VARCHAR(255);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "structure" VARCHAR(255);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "supplier_product_code" VARCHAR(255);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "yarn_count" VARCHAR(255);
ALTER TABLE "purchase_orders" ADD COLUMN IF NOT EXISTS "actual_delivery_date" DATE;
ALTER TABLE "purchase_orders" ADD COLUMN IF NOT EXISTS "approved_at" TIMESTAMPTZ;
ALTER TABLE "purchase_orders" ADD COLUMN IF NOT EXISTS "attachment_urls" JSONB;
ALTER TABLE "purchase_orders" ADD COLUMN IF NOT EXISTS "currency" VARCHAR(10);
ALTER TABLE "purchase_orders" ADD COLUMN IF NOT EXISTS "department_id" INTEGER;
ALTER TABLE "purchase_orders" ADD COLUMN IF NOT EXISTS "exchange_rate" DECIMAL(18,6);
ALTER TABLE "purchase_orders" ADD COLUMN IF NOT EXISTS "expected_delivery_date" DATE;
ALTER TABLE "purchase_orders" ADD COLUMN IF NOT EXISTS "order_status" VARCHAR(20);
ALTER TABLE "purchase_orders" ADD COLUMN IF NOT EXISTS "payment_terms" VARCHAR(255);
ALTER TABLE "purchase_orders" ADD COLUMN IF NOT EXISTS "purchaser_id" INTEGER;
ALTER TABLE "purchase_orders" ADD COLUMN IF NOT EXISTS "rejected_reason" VARCHAR(255);
ALTER TABLE "purchase_orders" ADD COLUMN IF NOT EXISTS "shipping_terms" VARCHAR(255);
ALTER TABLE "purchase_orders" ADD COLUMN IF NOT EXISTS "total_amount_foreign" DECIMAL(18,2);
ALTER TABLE "purchase_orders" ADD COLUMN IF NOT EXISTS "total_quantity" DECIMAL(18,4);
ALTER TABLE "purchase_orders" ADD COLUMN IF NOT EXISTS "total_quantity_alt" DECIMAL(18,4);
ALTER TABLE "purchase_orders" ADD COLUMN IF NOT EXISTS "updated_by" INTEGER;
ALTER TABLE "purchase_orders" ADD COLUMN IF NOT EXISTS "warehouse_id" INTEGER;
ALTER TABLE "sales_order_items" ADD COLUMN IF NOT EXISTS "base_price" DECIMAL(18,4);
ALTER TABLE "sales_order_items" ADD COLUMN IF NOT EXISTS "batch_requirement" VARCHAR(255);
ALTER TABLE "sales_order_items" ADD COLUMN IF NOT EXISTS "color_extra_cost" DECIMAL(18,4);
ALTER TABLE "sales_order_items" ADD COLUMN IF NOT EXISTS "color_name" VARCHAR(255);
ALTER TABLE "sales_order_items" ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(255);
ALTER TABLE "sales_order_items" ADD COLUMN IF NOT EXISTS "discount_amount" DECIMAL(18,4);
ALTER TABLE "sales_order_items" ADD COLUMN IF NOT EXISTS "discount_percent" DECIMAL(18,4);
ALTER TABLE "sales_order_items" ADD COLUMN IF NOT EXISTS "dye_lot_requirement" VARCHAR(255);
ALTER TABLE "sales_order_items" ADD COLUMN IF NOT EXISTS "final_price" DECIMAL(18,4);
ALTER TABLE "sales_order_items" ADD COLUMN IF NOT EXISTS "grade_price_diff" DECIMAL(18,4);
ALTER TABLE "sales_order_items" ADD COLUMN IF NOT EXISTS "grade_required" VARCHAR(255);
ALTER TABLE "sales_order_items" ADD COLUMN IF NOT EXISTS "gram_weight" DECIMAL(18,4);
ALTER TABLE "sales_order_items" ADD COLUMN IF NOT EXISTS "is_net_weight" BOOLEAN;
ALTER TABLE "sales_order_items" ADD COLUMN IF NOT EXISTS "notes" VARCHAR(255);
ALTER TABLE "sales_order_items" ADD COLUMN IF NOT EXISTS "pantone_code" VARCHAR(255);
ALTER TABLE "sales_order_items" ADD COLUMN IF NOT EXISTS "paper_tube_weight" DECIMAL(18,4);
ALTER TABLE "sales_order_items" ADD COLUMN IF NOT EXISTS "quantity_kg" DECIMAL(18,4);
ALTER TABLE "sales_order_items" ADD COLUMN IF NOT EXISTS "quantity_meters" DECIMAL(18,4);
ALTER TABLE "sales_order_items" ADD COLUMN IF NOT EXISTS "shipped_quantity" DECIMAL(18,4);
ALTER TABLE "sales_order_items" ADD COLUMN IF NOT EXISTS "shipped_quantity_kg" DECIMAL(18,4);
ALTER TABLE "sales_order_items" ADD COLUMN IF NOT EXISTS "shipped_quantity_meters" DECIMAL(18,4);
ALTER TABLE "sales_order_items" ADD COLUMN IF NOT EXISTS "tax_amount" DECIMAL(18,4);
ALTER TABLE "sales_order_items" ADD COLUMN IF NOT EXISTS "tax_percent" DECIMAL(18,4);
ALTER TABLE "sales_order_items" ADD COLUMN IF NOT EXISTS "total_amount" DECIMAL(18,4);
ALTER TABLE "sales_order_items" ADD COLUMN IF NOT EXISTS "width" DECIMAL(18,4);
ALTER TABLE "sales_orders" ADD COLUMN IF NOT EXISTS "approved_at" TIMESTAMPTZ;
ALTER TABLE "sales_orders" ADD COLUMN IF NOT EXISTS "balance_amount" DECIMAL(18,4);
ALTER TABLE "sales_orders" ADD COLUMN IF NOT EXISTS "billing_address" VARCHAR(255);
ALTER TABLE "sales_orders" ADD COLUMN IF NOT EXISTS "opportunity_id" INTEGER;
ALTER TABLE "sales_orders" ADD COLUMN IF NOT EXISTS "required_date" TIMESTAMPTZ;
ALTER TABLE "sales_orders" ADD COLUMN IF NOT EXISTS "ship_date" TIMESTAMPTZ;
ALTER TABLE "sales_orders" ADD COLUMN IF NOT EXISTS "shipping_address" VARCHAR(255);
ALTER TABLE "sales_orders" ADD COLUMN IF NOT EXISTS "shipping_cost" DECIMAL(18,4);
ALTER TABLE "sales_orders" ADD COLUMN IF NOT EXISTS "subtotal" DECIMAL(18,4);
ALTER TABLE "sales_orders" ADD COLUMN IF NOT EXISTS "tax_amount" DECIMAL(18,4);
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "annual_revenue" DECIMAL(15,2);
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "assist_batch" BOOLEAN;
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "assist_supplier" BOOLEAN;
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "bank_account" VARCHAR(255);
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "bank_name" VARCHAR(255);
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "business_address" VARCHAR(255);
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "business_scope" VARCHAR(255);
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "business_term" VARCHAR(255);
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "category_id" INTEGER;
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "contact_phone" VARCHAR(255);
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "created_by" INTEGER;
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "credit_code" VARCHAR(255);
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "employee_count" INTEGER;
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "establishment_date" DATE;
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "fax" VARCHAR(255);
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "grade" VARCHAR(255);
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "grade_score" DECIMAL(5,2);
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "is_enabled" BOOLEAN;
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "is_processor" BOOLEAN;
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "last_evaluation_date" DATE;
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "legal_representative" VARCHAR(255);
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "main_business" VARCHAR(255);
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "main_market" VARCHAR(255);
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "processor_type" VARCHAR(255);
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "registered_address" VARCHAR(255);
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "registered_capital" DECIMAL(15,2);
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "remarks" VARCHAR(255);
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "status" VARCHAR(255);
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "supplier_code" VARCHAR(255);
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "supplier_name" VARCHAR(255);
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "supplier_short_name" VARCHAR(255);
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "supplier_type" VARCHAR(255);
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "taxpayer_type" VARCHAR(255);
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "updated_by" INTEGER;
ALTER TABLE "suppliers" ADD COLUMN IF NOT EXISTS "website" VARCHAR(255);
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "capacity" INTEGER;
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "city" VARCHAR(255);
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "country" VARCHAR(255);
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "email" VARCHAR(255);
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "notes" VARCHAR(255);
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "phone" VARCHAR(255);
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "postal_code" VARCHAR(255);
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "province" VARCHAR(255);
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "warehouse_code" VARCHAR(255);
"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 依次回滚所有迁移（逆序）
        m0006_add_general_ledger_and_finance_base::Migration
            .down(manager)
            .await?;
        m0005_add_basic_data_and_system_tables::Migration
            .down(manager)
            .await?;
        m0004_add_field_permissions::Migration.down(manager).await?;
        m0003_add_dye_tables::Migration.down(manager).await?;
        m0002_add_crm_and_greige_tables::Migration
            .down(manager)
            .await?;
        m0001_initial_schema::Migration.down(manager).await?;
        Ok(())
    }
}
