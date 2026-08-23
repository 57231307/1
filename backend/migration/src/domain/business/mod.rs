//! 业务表：生产/采购/库存/销售/财务
//!
//! 合并自: 8 个迁移文件

use sea_orm_migration::prelude::*;

// 导入所有迁移模块
mod m0007_add_mrp_production_bom;
mod m0008_add_supplier_and_product_extensions;
mod m0009_add_purchase_extensions;
mod m0010_add_inventory_extensions;
mod m0011_add_sales_and_logistics_extensions;
mod m0012_add_ap_ar_finance_analysis;
mod m0013_add_business_process_and_traceability;
mod m0014_add_saas_notification_report_email_oa;

pub struct Migration;

impl MigrationName for Migration {
    fn name() -> &'static str {
        "m_business_domain"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 依次执行所有迁移
        m0007_add_mrp_production_bom::Migration.up(manager).await?;
        m0008_add_supplier_and_product_extensions::Migration
            .up(manager)
            .await?;
        m0009_add_purchase_extensions::Migration.up(manager).await?;
        m0010_add_inventory_extensions::Migration
            .up(manager)
            .await?;
        m0011_add_sales_and_logistics_extensions::Migration
            .up(manager)
            .await?;
        m0012_add_ap_ar_finance_analysis::Migration
            .up(manager)
            .await?;
        m0013_add_business_process_and_traceability::Migration
            .up(manager)
            .await?;
        m0014_add_saas_notification_report_email_oa::Migration
            .up(manager)
            .await?;
        let sql = r#"ALTER TABLE "ar_invoices" ADD COLUMN IF NOT EXISTS "salesperson_id" INTEGER;
ALTER TABLE "ar_reconciliations" ADD COLUMN IF NOT EXISTS "notes" VARCHAR(255);
ALTER TABLE "batch_trace_log" ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(255);
ALTER TABLE "batch_trace_log" ADD COLUMN IF NOT EXISTS "dye_lot_no" VARCHAR(255);
ALTER TABLE "batch_trace_log" ADD COLUMN IF NOT EXISTS "from_status" VARCHAR(255);
ALTER TABLE "batch_trace_log" ADD COLUMN IF NOT EXISTS "product_id" INTEGER;
ALTER TABLE "batch_trace_log" ADD COLUMN IF NOT EXISTS "to_status" VARCHAR(255);
ALTER TABLE "bom_items" ADD COLUMN IF NOT EXISTS "is_deleted" BOOLEAN;
ALTER TABLE "boms" ADD COLUMN IF NOT EXISTS "is_deleted" BOOLEAN;
ALTER TABLE "budget_items" ADD COLUMN IF NOT EXISTS "account_subject_id" INTEGER;
ALTER TABLE "budget_items" ADD COLUMN IF NOT EXISTS "budget_year" INTEGER;
ALTER TABLE "budget_items" ADD COLUMN IF NOT EXISTS "planned_amount" DECIMAL(14,2);
ALTER TABLE "budget_items" ADD COLUMN IF NOT EXISTS "remark" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "custom_fields" JSONB;
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "industry" VARCHAR(255);
ALTER TABLE "dye_lot_mapping" ADD COLUMN IF NOT EXISTS "batch_dye_lot_id" INTEGER;
ALTER TABLE "dye_lot_mapping" ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(255);
ALTER TABLE "dye_lot_mapping" ADD COLUMN IF NOT EXISTS "created_by" INTEGER;
ALTER TABLE "dye_lot_mapping" ADD COLUMN IF NOT EXISTS "internal_dye_lot_no" VARCHAR(255);
ALTER TABLE "dye_lot_mapping" ADD COLUMN IF NOT EXISTS "is_active" BOOLEAN;
ALTER TABLE "dye_lot_mapping" ADD COLUMN IF NOT EXISTS "mapping_date" DATE;
ALTER TABLE "dye_lot_mapping" ADD COLUMN IF NOT EXISTS "product_code" VARCHAR(255);
ALTER TABLE "dye_lot_mapping" ADD COLUMN IF NOT EXISTS "remarks" VARCHAR(255);
ALTER TABLE "dye_lot_mapping" ADD COLUMN IF NOT EXISTS "supplier_dye_lot_no" VARCHAR(255);
ALTER TABLE "dye_lot_mapping" ADD COLUMN IF NOT EXISTS "supplier_id" INTEGER;
ALTER TABLE "dye_lot_mapping" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "dye_lot_mapping" ADD COLUMN IF NOT EXISTS "updated_by" INTEGER;
ALTER TABLE "dye_lot_mapping" ADD COLUMN IF NOT EXISTS "validated_at" TIMESTAMPTZ;
ALTER TABLE "dye_lot_mapping" ADD COLUMN IF NOT EXISTS "validated_by" INTEGER;
ALTER TABLE "dye_lot_mapping" ADD COLUMN IF NOT EXISTS "validation_status" VARCHAR(255);
ALTER TABLE "email_logs" ADD COLUMN IF NOT EXISTS "attachments" JSONB;
ALTER TABLE "email_logs" ADD COLUMN IF NOT EXISTS "html_content" VARCHAR(255);
ALTER TABLE "email_logs" ADD COLUMN IF NOT EXISTS "next_retry_at" TIMESTAMPTZ;
ALTER TABLE "email_logs" ADD COLUMN IF NOT EXISTS "text_content" VARCHAR(255);
ALTER TABLE "fixed_asset_disposals" ADD COLUMN IF NOT EXISTS "gain_loss" DECIMAL(18,4);
ALTER TABLE "fixed_assets" ADD COLUMN IF NOT EXISTS "asset_category_id" INTEGER;
ALTER TABLE "fixed_assets" ADD COLUMN IF NOT EXISTS "depreciation_start_date" DATE;
ALTER TABLE "fund_accounts" ADD COLUMN IF NOT EXISTS "available_balance" DECIMAL(18,4);
ALTER TABLE "fund_accounts" ADD COLUMN IF NOT EXISTS "frozen_balance" DECIMAL(18,4);
ALTER TABLE "fund_accounts" ADD COLUMN IF NOT EXISTS "opened_date" DATE;
ALTER TABLE "fund_accounts" ADD COLUMN IF NOT EXISTS "remark" VARCHAR(255);
ALTER TABLE "fund_accounts" ADD COLUMN IF NOT EXISTS "status" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "created_by" INTEGER;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "dye_lot_id" INTEGER;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "dye_lot_no" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "gram_weight" DECIMAL(18,4);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "inspection_id" INTEGER;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "inventory_status" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "original_length" DECIMAL(18,4);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "original_weight" DECIMAL(18,4);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "package_no" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "piece_seq" INTEGER;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "position_no" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "production_date" DATE;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "quality_status" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "scan_type" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "shelf_life" INTEGER;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "supplier_piece_no" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "updated_by" INTEGER;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "width" DECIMAL(18,4);
ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "distance_km" DECIMAL(18,4);
ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "freight_bearer" VARCHAR(255);
ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "freight_rate" DECIMAL(18,4);
ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "order_type" VARCHAR(255);
ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "sign_photo_url" VARCHAR(255);
ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "sign_receipt_url" VARCHAR(255);
ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "sign_remark" VARCHAR(255);
ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "signed_at" TIMESTAMPTZ;
ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "signed_by" INTEGER;
ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "total_volume" DECIMAL(18,4);
ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "total_weight" DECIMAL(18,4);
ALTER TABLE "notification_settings" ADD COLUMN IF NOT EXISTS "enable_webhook" BOOLEAN;
ALTER TABLE "notifications" ADD COLUMN IF NOT EXISTS "dedup_key" VARCHAR(255);
ALTER TABLE "oa_announcement" ADD COLUMN IF NOT EXISTS "visibility_scope" VARCHAR(255);
ALTER TABLE "oa_announcement" ADD COLUMN IF NOT EXISTS "visible_scope_config" JSONB;
ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "batch_no" VARCHAR(255);
ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(255);
ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "dye_lot_no" VARCHAR(255);
ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "order_type" VARCHAR(255);
ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "original_batch_id" INTEGER;
ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "schedule_batch_key" VARCHAR(255);
ALTER TABLE "purchase_return_item" ADD COLUMN IF NOT EXISTS "batch_no" VARCHAR(255);
ALTER TABLE "purchase_return_item" ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(255);
ALTER TABLE "purchase_return_item" ADD COLUMN IF NOT EXISTS "dye_lot_no" VARCHAR(255);
ALTER TABLE "report_subscriptions" ADD COLUMN IF NOT EXISTS "max_retries" INTEGER;
ALTER TABLE "report_subscriptions" ADD COLUMN IF NOT EXISTS "next_retry_at" TIMESTAMPTZ;
ALTER TABLE "report_subscriptions" ADD COLUMN IF NOT EXISTS "parameters" JSONB;
ALTER TABLE "report_subscriptions" ADD COLUMN IF NOT EXISTS "retry_count" INTEGER;
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "cache_ttl_seconds" INTEGER;
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "category" VARCHAR(255);
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "data_source" VARCHAR(255);
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "parameters" JSONB;
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "refresh_strategy" VARCHAR(255);
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "required_permission" VARCHAR(255);
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "supported_formats" JSONB;
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "template_id" VARCHAR(255);
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "version" INTEGER;
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "breach_liability" VARCHAR(255);
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "dispute_resolution" VARCHAR(255);
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "performance_period" VARCHAR(255);
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "quality_terms" VARCHAR(255);
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "signature_certificate" VARCHAR(255);
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "signature_hash" VARCHAR(255);
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "signature_image_url" VARCHAR(255);
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "signed_at" TIMESTAMPTZ;
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "signed_by_user_id" INTEGER;
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "stamp_tax_amount" DECIMAL(18,4);
ALTER TABLE "sales_delivery_item" ADD COLUMN IF NOT EXISTS "dye_lot_id" INTEGER;
ALTER TABLE "sales_delivery_item" ADD COLUMN IF NOT EXISTS "dye_lot_no" VARCHAR(255);
ALTER TABLE "sales_return_item" ADD COLUMN IF NOT EXISTS "batch_no" VARCHAR(255);
ALTER TABLE "sales_return_item" ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(255);
ALTER TABLE "sales_return_item" ADD COLUMN IF NOT EXISTS "dye_lot_no" VARCHAR(255);
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "approved_at_fin" TIMESTAMPTZ;
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "approved_at_gm" TIMESTAMPTZ;
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "approver_id_fin" INTEGER;
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "approver_id_gm" INTEGER;
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "grade" VARCHAR(255);
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "handling_result" VARCHAR(255);
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "scrap_approval_status" VARCHAR(255);
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "scrap_loss_amount" DECIMAL(18,4);
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "stock_grade_synced" BOOLEAN;
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "stock_id" INTEGER;
ALTER TABLE "work_centers" ADD COLUMN IF NOT EXISTS "auto_reschedule_enabled" BOOLEAN;
ALTER TABLE "work_centers" ADD COLUMN IF NOT EXISTS "equipment_count" INTEGER;
ALTER TABLE "work_centers" ADD COLUMN IF NOT EXISTS "shift_hours" DECIMAL(6,2);
ALTER TABLE "work_centers" ADD COLUMN IF NOT EXISTS "standard_hours_per_unit" DECIMAL(10,2);
ALTER TABLE "work_centers" ADD COLUMN IF NOT EXISTS "worker_count" INTEGER;
"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 依次回滚所有迁移（逆序）
        m0014_add_saas_notification_report_email_oa::Migration
            .down(manager)
            .await?;
        m0013_add_business_process_and_traceability::Migration
            .down(manager)
            .await?;
        m0012_add_ap_ar_finance_analysis::Migration
            .down(manager)
            .await?;
        m0011_add_sales_and_logistics_extensions::Migration
            .down(manager)
            .await?;
        m0010_add_inventory_extensions::Migration
            .down(manager)
            .await?;
        m0009_add_purchase_extensions::Migration
            .down(manager)
            .await?;
        m0008_add_supplier_and_product_extensions::Migration
            .down(manager)
            .await?;
        m0007_add_mrp_production_bom::Migration
            .down(manager)
            .await?;
        Ok(())
    }
}
