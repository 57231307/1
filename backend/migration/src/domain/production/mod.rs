//! 生产/质量/委外
//!
//! 合并自: 16 个迁移文件

use sea_orm_migration::prelude::*;

// 导入所有迁移模块
mod m0025_p4_1_perf_indexes;
mod m0026_extend_audit_log;
mod m0027_enable_pg_stat_statements;
mod m0028_create_slow_query_log;
mod m0029_drop_tenant_columns;
mod m0030_create_crm_recycle_rules;
mod m0041_create_import_tasks;
mod m0042_create_purchase_inspection_items;
mod m0043_add_scan_type_and_industry_columns;
mod m0044_integrate_unreferenced_migrations;
mod m0045_add_password_changed_at_to_users;
mod m0046_drop_audit_alert_rules;
mod m0047_add_last_payload_to_webhooks;
mod m0048_add_user_id_to_webhooks;
mod m0049_create_processed_events;
mod m0050_create_event_dead_letters;

pub struct Migration;

impl MigrationName for Migration {
    fn name() -> &'static str {
        "m_production_domain"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 依次执行所有迁移
        m0025_p4_1_perf_indexes::Migration.up(manager).await?;
        m0026_extend_audit_log::Migration.up(manager).await?;
        m0027_enable_pg_stat_statements::Migration
            .up(manager)
            .await?;
        m0028_create_slow_query_log::Migration.up(manager).await?;
        // 顺序修复：m0044 必须在 m0029 之前执行。
        // m0044 创建 custom_orders / color_cards / process_nodes / sales_facts 等 31 张表，
        // m0029 随后对这些表执行 ALTER TABLE ... DROP COLUMN IF EXISTS "tenant_id"。
        // PostgreSQL 的 DROP COLUMN IF EXISTS 仅保护列不存在，不保护表不存在；
        // 若 m0029 先于 m0044 执行，会因 "relation xxx does not exist" 中断整个迁移链。
        // 这也符合 m0044 文件头注释的设计意图："注册在 m0028 之后、m0029 之前"。
        m0044_integrate_unreferenced_migrations::Migration
            .up(manager)
            .await?;
        m0029_drop_tenant_columns::Migration.up(manager).await?;
        m0030_create_crm_recycle_rules::Migration
            .up(manager)
            .await?;
        m0041_create_import_tasks::Migration.up(manager).await?;
        m0042_create_purchase_inspection_items::Migration
            .up(manager)
            .await?;
        m0043_add_scan_type_and_industry_columns::Migration
            .up(manager)
            .await?;
        m0045_add_password_changed_at_to_users::Migration
            .up(manager)
            .await?;
        m0046_drop_audit_alert_rules::Migration.up(manager).await?;
        m0047_add_last_payload_to_webhooks::Migration
            .up(manager)
            .await?;
        m0048_add_user_id_to_webhooks::Migration.up(manager).await?;
        m0049_create_processed_events::Migration.up(manager).await?;
        m0050_create_event_dead_letters::Migration
            .up(manager)
            .await?;
        let sql = r#"ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "created_by" INTEGER;
ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "expires_at" TIMESTAMPTZ;
ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "is_active" BOOLEAN;
ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "key_hash" VARCHAR(255);
ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "key_prefix" VARCHAR(255);
ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "last_used_at" TIMESTAMPTZ;
ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "name" VARCHAR(255);
ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "permissions" VARCHAR(255);
ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "rate_limit_per_minute" INTEGER;
ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "action" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "condition" TEXT;
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "created_at" VARCHAR(255);
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
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "request_body" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "request_method" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "request_path" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "resource_id" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "resource_name" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "resource_type" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "response_status" INTEGER;
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "user_agent" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "user_id" INTEGER;
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "username" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "address" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "company_name" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "contact_name" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "contact_title" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "converted_at" TIMESTAMPTZ;
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "converted_customer_id" INTEGER;
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "converted_opportunity_id" INTEGER;
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "created_by" INTEGER;
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "custom_fields" JSONB;
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "email" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "estimated_amount" DECIMAL(18,4);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "estimated_quantity" DECIMAL(18,4);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "expected_delivery_date" DATE;
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "follow_up_plan" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "last_follow_up_date" DATE;
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "lead_no" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "lead_source" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "lead_status" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "lost_reason" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "mobile_phone" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "next_follow_up_date" DATE;
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "owner_id" INTEGER;
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "owner_name" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "priority" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "product_interest" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "qq" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "rating" INTEGER;
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "requirement_desc" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "tags" JSONB;
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "tel_phone" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "updated_by" INTEGER;
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "wechat" VARCHAR(255);
ALTER TABLE "event_dead_letters" ADD COLUMN IF NOT EXISTS "created_at" VARCHAR(255);
ALTER TABLE "event_dead_letters" ADD COLUMN IF NOT EXISTS "event_payload" JSONB;
ALTER TABLE "event_dead_letters" ADD COLUMN IF NOT EXISTS "event_type" VARCHAR(255);
ALTER TABLE "event_dead_letters" ADD COLUMN IF NOT EXISTS "failure_reason" VARCHAR(255);
ALTER TABLE "event_dead_letters" ADD COLUMN IF NOT EXISTS "first_failed_at" VARCHAR(255);
ALTER TABLE "event_dead_letters" ADD COLUMN IF NOT EXISTS "last_error" VARCHAR(255);
ALTER TABLE "event_dead_letters" ADD COLUMN IF NOT EXISTS "last_retry_at" VARCHAR(255);
ALTER TABLE "event_dead_letters" ADD COLUMN IF NOT EXISTS "max_retries" INTEGER;
ALTER TABLE "event_dead_letters" ADD COLUMN IF NOT EXISTS "resolved_at" VARCHAR(255);
ALTER TABLE "event_dead_letters" ADD COLUMN IF NOT EXISTS "resolved_by" INTEGER;
ALTER TABLE "event_dead_letters" ADD COLUMN IF NOT EXISTS "retry_count" INTEGER;
ALTER TABLE "event_dead_letters" ADD COLUMN IF NOT EXISTS "status" VARCHAR(255);
ALTER TABLE "event_dead_letters" ADD COLUMN IF NOT EXISTS "updated_at" VARCHAR(255);
ALTER TABLE "failover_config" ADD COLUMN IF NOT EXISTS "config_key" VARCHAR(255);
ALTER TABLE "failover_config" ADD COLUMN IF NOT EXISTS "config_value" VARCHAR(255);
ALTER TABLE "failover_config" ADD COLUMN IF NOT EXISTS "description" VARCHAR(255);
ALTER TABLE "failover_config" ADD COLUMN IF NOT EXISTS "function_name" VARCHAR(255);
ALTER TABLE "failover_config" ADD COLUMN IF NOT EXISTS "is_active" BOOLEAN;
ALTER TABLE "failover_config" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "failover_event" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "failover_event" ADD COLUMN IF NOT EXISTS "event_type" VARCHAR(255);
ALTER TABLE "failover_event" ADD COLUMN IF NOT EXISTS "from_state" VARCHAR(255);
ALTER TABLE "failover_event" ADD COLUMN IF NOT EXISTS "function_name" VARCHAR(255);
ALTER TABLE "failover_event" ADD COLUMN IF NOT EXISTS "latency_ms" INTEGER;
ALTER TABLE "failover_event" ADD COLUMN IF NOT EXISTS "reason" VARCHAR(255);
ALTER TABLE "failover_event" ADD COLUMN IF NOT EXISTS "to_state" VARCHAR(255);
ALTER TABLE "failover_status" ADD COLUMN IF NOT EXISTS "backup_type" VARCHAR(255);
ALTER TABLE "failover_status" ADD COLUMN IF NOT EXISTS "circuit_state" VARCHAR(255);
ALTER TABLE "failover_status" ADD COLUMN IF NOT EXISTS "consecutive_failures" INTEGER;
ALTER TABLE "failover_status" ADD COLUMN IF NOT EXISTS "current_state" VARCHAR(255);
ALTER TABLE "failover_status" ADD COLUMN IF NOT EXISTS "function_name" VARCHAR(255);
ALTER TABLE "failover_status" ADD COLUMN IF NOT EXISTS "last_success_at" TIMESTAMPTZ;
ALTER TABLE "failover_status" ADD COLUMN IF NOT EXISTS "last_switch_at" TIMESTAMPTZ;
ALTER TABLE "failover_status" ADD COLUMN IF NOT EXISTS "primary_url" VARCHAR(255);
ALTER TABLE "failover_status" ADD COLUMN IF NOT EXISTS "total_backup_calls" BIGINT;
ALTER TABLE "failover_status" ADD COLUMN IF NOT EXISTS "total_primary_calls" BIGINT;
ALTER TABLE "failover_status" ADD COLUMN IF NOT EXISTS "total_switches" BIGINT;
ALTER TABLE "failover_status" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "barcode" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "batch_no" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "created_by" INTEGER;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "dye_lot_id" INTEGER;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "dye_lot_no" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "gram_weight" DECIMAL(18,4);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "inspection_id" INTEGER;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "inventory_status" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "length" DECIMAL(18,4);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "location_id" INTEGER;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "original_length" DECIMAL(18,4);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "original_weight" DECIMAL(18,4);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "package_no" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "parent_piece_id" INTEGER;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "piece_no" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "piece_seq" INTEGER;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "position_no" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "product_id" INTEGER;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "production_date" DATE;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "quality_status" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "remarks" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "shelf_life" INTEGER;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "status" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "supplier_piece_no" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "updated_by" INTEGER;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "warehouse_id" INTEGER;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "weight" DECIMAL(18,4);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "width" DECIMAL(18,4);
ALTER TABLE "processed_events" ADD COLUMN IF NOT EXISTS "consumer_id" VARCHAR(255);
ALTER TABLE "processed_events" ADD COLUMN IF NOT EXISTS "event_key" VARCHAR(255);
ALTER TABLE "processed_events" ADD COLUMN IF NOT EXISTS "event_type" VARCHAR(255);
ALTER TABLE "processed_events" ADD COLUMN IF NOT EXISTS "processed_at" TIMESTAMPTZ;
ALTER TABLE "slow_query_log" ADD COLUMN IF NOT EXISTS "assigned_to" VARCHAR(255);
ALTER TABLE "slow_query_log" ADD COLUMN IF NOT EXISTS "jira_ticket" VARCHAR(255);
ALTER TABLE "slow_query_log" ADD COLUMN IF NOT EXISTS "optimization_status" VARCHAR(255);
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "agreed_to_terms_at" TIMESTAMPTZ;
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "birth_date" DATE;
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "department_id" INTEGER;
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "email" VARCHAR(255);
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "gender" VARCHAR(255);
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "is_active" BOOLEAN;
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "is_totp_enabled" BOOLEAN;
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "last_login_at" TIMESTAMPTZ;
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "password_hash" VARCHAR(255);
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "phone" VARCHAR(255);
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "role_id" INTEGER;
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "totp_recovery_codes" VARCHAR(255);
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "totp_secret" VARCHAR(255);
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "username" VARCHAR(255);
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "address" VARCHAR(255);
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "city" VARCHAR(255);
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "country" VARCHAR(255);
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "email" VARCHAR(255);
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "is_active" BOOLEAN;
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "manager_id" INTEGER;
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "name" VARCHAR(255);
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "notes" VARCHAR(255);
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "phone" VARCHAR(255);
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "postal_code" VARCHAR(255);
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "province" VARCHAR(255);
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "warehouse_code" VARCHAR(255);
ALTER TABLE "webhooks" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "webhooks" ADD COLUMN IF NOT EXISTS "events" VARCHAR(255);
ALTER TABLE "webhooks" ADD COLUMN IF NOT EXISTS "is_active" BOOLEAN;
ALTER TABLE "webhooks" ADD COLUMN IF NOT EXISTS "last_status" VARCHAR(255);
ALTER TABLE "webhooks" ADD COLUMN IF NOT EXISTS "last_triggered_at" TIMESTAMPTZ;
ALTER TABLE "webhooks" ADD COLUMN IF NOT EXISTS "name" VARCHAR(255);
ALTER TABLE "webhooks" ADD COLUMN IF NOT EXISTS "retry_count" INTEGER;
ALTER TABLE "webhooks" ADD COLUMN IF NOT EXISTS "secret" VARCHAR(255);
ALTER TABLE "webhooks" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "webhooks" ADD COLUMN IF NOT EXISTS "url" VARCHAR(255);
"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        let sql = r#"
ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "accepted_at" TIMESTAMPTZ;
ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "evaluated_at" TIMESTAMPTZ;
ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "evaluation_comment" VARCHAR(255);
ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "evaluation_score" INTEGER;
ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "quality_issue_id" BIGINT;
ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "reason_category" VARCHAR(255);
ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "reason_detail" VARCHAR(255);
ALTER TABLE "ai_process_optimizations" ADD COLUMN IF NOT EXISTS "inference_latency_ms" INTEGER;
ALTER TABLE "ai_process_optimizations" ADD COLUMN IF NOT EXISTS "model_version_id" INTEGER;
ALTER TABLE "ai_process_optimizations" ADD COLUMN IF NOT EXISTS "production_recipe_id" INTEGER;
ALTER TABLE "ai_quality_predictions" ADD COLUMN IF NOT EXISTS "actual_avg_qualification_rate" DECIMAL(18,4);
ALTER TABLE "ai_quality_predictions" ADD COLUMN IF NOT EXISTS "actual_grade" VARCHAR(255);
ALTER TABLE "ai_quality_predictions" ADD COLUMN IF NOT EXISTS "actual_recorded_at" TIMESTAMPTZ;
ALTER TABLE "ai_quality_predictions" ADD COLUMN IF NOT EXISTS "actual_risk_level" VARCHAR(255);
ALTER TABLE "ai_quality_predictions" ADD COLUMN IF NOT EXISTS "claim_amount" DECIMAL(18,4);
ALTER TABLE "ai_quality_predictions" ADD COLUMN IF NOT EXISTS "claim_recorded_at" TIMESTAMPTZ;
ALTER TABLE "ai_quality_predictions" ADD COLUMN IF NOT EXISTS "inference_latency_ms" INTEGER;
ALTER TABLE "ai_quality_predictions" ADD COLUMN IF NOT EXISTS "model_version_id" INTEGER;
ALTER TABLE "budget_items" ADD COLUMN IF NOT EXISTS "account_subject_id" INTEGER;
ALTER TABLE "budget_items" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "budget_items" ADD COLUMN IF NOT EXISTS "item_code" VARCHAR(255);
ALTER TABLE "budget_items" ADD COLUMN IF NOT EXISTS "item_name" VARCHAR(255);
ALTER TABLE "budget_items" ADD COLUMN IF NOT EXISTS "item_type" VARCHAR(255);
ALTER TABLE "budget_items" ADD COLUMN IF NOT EXISTS "level" INTEGER;
ALTER TABLE "budget_items" ADD COLUMN IF NOT EXISTS "parent_id" INTEGER;
ALTER TABLE "budget_items" ADD COLUMN IF NOT EXISTS "status" VARCHAR(255);
ALTER TABLE "budget_items" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "color_cards" ADD COLUMN IF NOT EXISTS "color_fastness_grade" VARCHAR(255);
ALTER TABLE "color_cards" ADD COLUMN IF NOT EXISTS "dyeing_capability" VARCHAR(255);
ALTER TABLE "color_cards" ADD COLUMN IF NOT EXISTS "issued_quantity" INTEGER;
ALTER TABLE "color_cards" ADD COLUMN IF NOT EXISTS "printing_capability" VARCHAR(255);
ALTER TABLE "color_cards" ADD COLUMN IF NOT EXISTS "stock_quantity" INTEGER;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "approval_instance_id" BIGINT;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "approved_at" TIMESTAMPTZ;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "approved_by" BIGINT;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "customer_approval_comment" VARCHAR(255);
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "customer_approved_at" TIMESTAMPTZ;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "lab_dip_request_id" INTEGER;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "quality_standard_id" INTEGER;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "quotation_id" BIGINT;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "rejection_reason" VARCHAR(255);
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "batch_no" VARCHAR(255);
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(255);
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "count_id" INTEGER;
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "dye_lot_no" VARCHAR(255);
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "product_id" INTEGER;
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "unit_cost" DECIMAL(12,2);
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "approved_by" INTEGER;
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "count_date" TIMESTAMPTZ;
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "count_no" VARCHAR(255);
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "created_by" INTEGER;
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "notes" VARCHAR(255);
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "status" VARCHAR(255);
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "warehouse_id" INTEGER;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "batch_no" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "bin_location" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "dye_lot_no" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "expiry_date" TIMESTAMPTZ;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "grade" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "gram_weight" DECIMAL(10,2);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "last_count_date" TIMESTAMPTZ;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "last_movement_date" TIMESTAMPTZ;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "layer_no" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "location_id" INTEGER;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "product_id" INTEGER;
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
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "version" INTEGER;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "warehouse_id" INTEGER;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "width" DECIMAL(10,2);
ALTER TABLE "product_color_prices" ADD COLUMN IF NOT EXISTS "base_price" DECIMAL(18,4);
ALTER TABLE "product_color_prices" ADD COLUMN IF NOT EXISTS "color_id" BIGINT;
ALTER TABLE "product_color_prices" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "product_color_prices" ADD COLUMN IF NOT EXISTS "currency" VARCHAR(255);
ALTER TABLE "product_color_prices" ADD COLUMN IF NOT EXISTS "customer_level" VARCHAR(255);
ALTER TABLE "product_color_prices" ADD COLUMN IF NOT EXISTS "effective_from" DATE;
ALTER TABLE "product_color_prices" ADD COLUMN IF NOT EXISTS "effective_to" DATE;
ALTER TABLE "product_color_prices" ADD COLUMN IF NOT EXISTS "min_quantity" DECIMAL(18,4);
ALTER TABLE "product_color_prices" ADD COLUMN IF NOT EXISTS "notes" VARCHAR(255);
ALTER TABLE "product_color_prices" ADD COLUMN IF NOT EXISTS "product_id" BIGINT;
ALTER TABLE "product_color_prices" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "quality_issues" ADD COLUMN IF NOT EXISTS "permanent_action_completed_at" TIMESTAMPTZ;
ALTER TABLE "quality_issues" ADD COLUMN IF NOT EXISTS "permanent_action_due_date" DATE;
ALTER TABLE "quality_issues" ADD COLUMN IF NOT EXISTS "permanent_action_owner" INTEGER;
ALTER TABLE "quality_issues" ADD COLUMN IF NOT EXISTS "root_cause_detail" JSONB;
ALTER TABLE "quality_issues" ADD COLUMN IF NOT EXISTS "root_cause_method" VARCHAR(255);
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "duty_cost" DECIMAL(18,4);
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "freight_cost" DECIMAL(18,4);
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "insurance_cost" DECIMAL(18,4);
"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 依次回滚所有迁移（逆序）
        m0050_create_event_dead_letters::Migration
            .down(manager)
            .await?;
        m0049_create_processed_events::Migration
            .down(manager)
            .await?;
        m0048_add_user_id_to_webhooks::Migration
            .down(manager)
            .await?;
        m0047_add_last_payload_to_webhooks::Migration
            .down(manager)
            .await?;
        m0046_drop_audit_alert_rules::Migration
            .down(manager)
            .await?;
        m0045_add_password_changed_at_to_users::Migration
            .down(manager)
            .await?;
        m0043_add_scan_type_and_industry_columns::Migration
            .down(manager)
            .await?;
        m0042_create_purchase_inspection_items::Migration
            .down(manager)
            .await?;
        m0041_create_import_tasks::Migration.down(manager).await?;
        m0030_create_crm_recycle_rules::Migration
            .down(manager)
            .await?;
        m0029_drop_tenant_columns::Migration.down(manager).await?;
        // 逆序：m0044 在 m0029 之后回滚（与 up 顺序相反）
        m0044_integrate_unreferenced_migrations::Migration
            .down(manager)
            .await?;
        m0028_create_slow_query_log::Migration.down(manager).await?;
        m0027_enable_pg_stat_statements::Migration
            .down(manager)
            .await?;
        m0026_extend_audit_log::Migration.down(manager).await?;
        m0025_p4_1_perf_indexes::Migration.down(manager).await?;
        Ok(())
    }
}
