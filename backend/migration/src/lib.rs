pub use sea_orm_migration::prelude::*;

pub mod m0001_initial_schema;
pub mod m0002_add_crm_and_greige_tables;
pub mod m0003_add_dye_tables;
pub mod m0004_add_field_permissions;
pub mod m0005_add_basic_data_and_system_tables;
pub mod m0006_add_general_ledger_and_finance_base;
pub mod m0007_add_mrp_production_bom;
pub mod m0008_add_supplier_and_product_extensions;
pub mod m0009_add_purchase_extensions;
pub mod m0010_add_inventory_extensions;
pub mod m0011_add_sales_and_logistics_extensions;
pub mod m0012_add_ap_ar_finance_analysis;
pub mod m0013_add_business_process_and_traceability;
pub mod m0014_add_saas_notification_report_email_oa;
pub mod m0015_add_opportunity_id_to_sales_orders;
pub mod m0016_add_version_to_inventory_stocks;
pub mod m0017_add_crm_supplier_tables;
pub mod m0018_add_finance_tables;
pub mod m0019_add_missing_columns;
pub mod m0020_fix_schema_model_sync;
pub mod m0021_create_sales_quotations;
pub mod m0022_create_sales_quotation_items;
pub mod m0023_create_sales_quotation_terms;
pub mod m0024_create_product_color_prices;
pub mod m0025_p4_1_perf_indexes;
pub mod m0026_extend_audit_log;
pub mod m0027_enable_pg_stat_statements;
pub mod m0028_create_slow_query_log;
pub mod m0029_drop_tenant_columns;
// 批次 23 v5 P0-4：CRM 公海回收规则持久化迁移
pub mod m0030_create_crm_recycle_rules;
// P0 8-2（批次 53）：omni_audit_logs 添加 HMAC-SHA256 防篡改签名列
pub mod m0031_add_signature_to_omni_audit_logs;
// 批次 88 PH-1：custom_orders 添加 notes 列（占位符实现）
pub mod m0032_add_notes_to_custom_orders;
// 批次 88 PH-3：fixed_asset_disposals 添加 gain_loss 列（占位符实现）
pub mod m0033_add_gain_loss_to_fixed_asset_disposals;
// 批次 88 PH-2：固定资产折旧期间记录表（占位符实现）
pub mod m0034_create_fixed_asset_depreciation_records;
// 批次 90b P2-12：客户联系人表（占位符实现）
pub mod m0035_create_customer_contacts;
// 批次 91 P0-1：API 端点管理表
pub mod m0036_create_api_endpoints;
// 批次 92 P3-12/P3-13：fixed_asset_depreciation_records 外键 RESTRICT + 冗余索引清理
pub mod m0037_alter_fa_depreciation_records_fk;
// 批次 109 P1-1：ar_reconciliations 添加 notes 列（v7 复审修复）
pub mod m0038_add_notes_to_ar_reconciliations;

// 批次 112 P1-9：api_keys 添加 created_by 列（v7 复审修复）
pub mod m0039_add_created_by_to_api_keys;

// 批次 122 v8 复审 P1：CRM 标签字典表（替代 list_tags 硬编码 + create_tag/delete_tag 假实现）
pub mod m0040_create_crm_tags;

// 批次 127 v8 复审 P2：导入任务记录表（替代 list_import_tasks 空列表占位 + import_csv/import_excel 不落库）
pub mod m0041_create_import_tasks;

// 批次 131 v9 复审 P0：采购质检明细表（替代 4 个明细 CRUD 端点占位）
pub mod m0042_create_purchase_inspection_items;
// v11 批次 153 P2-A：inventory_piece.scan_type + crm_lead.industry 列迁移
pub mod m0043_add_scan_type_and_industry_columns;
// 批次 190 迁移整合：执行所有未被 Rust 模块引用的 SQL 迁移（31 个目录）
pub mod m0044_integrate_unreferenced_migrations;
// 批次 198 P0-2：users 表添加 password_changed_at 列（密码过期策略锚点）
pub mod m0045_add_password_changed_at_to_users;

// 批次 202 P1-2：删除 audit_alert_rules 表（遗留死代码，模型无业务引用）
pub mod m0046_drop_audit_alert_rules;

// 批次 251 v14 中风险：webhooks 表添加 last_payload + last_event 列（retry 重投原始数据）
pub mod m0047_add_last_payload_to_webhooks;

// 批次 320 v9 中风险 M-4：webhooks 表添加 user_id 列（IDOR 防护）
pub mod m0048_add_user_id_to_webhooks;

// 批次 365 v13 复审 B-P1-8：processed_events 事件幂等去重表
pub mod m0049_create_processed_events;

// 批次 384 v13 复审 B-P1-7：事件死信队列表
pub mod m0050_create_event_dead_letters;

// V15 P0-S01：role 表新增 data_scope 字段（行级数据权限）
pub mod m0051_add_data_scope_to_roles;
pub mod m0052_create_role_conflicts;
// V15 P0-S06：权限变更审计表
pub mod m0053_create_permission_change_audit;
// Batch 464 P0-S25：行级数据权限 RLS 策略启用（5 张敏感表）
pub mod m0054_enable_rls_policies;
// Batch 473 P0-S14：敏感数据导出二级审批表（补齐缺失的 migration）
pub mod m0055_create_export_approval_request;
// Batch 473 P0-S19：审计日志补齐 condition 字段（audit_logs + omni_audit_logs）
pub mod m0056_add_condition_to_audit_logs;
// Batch 477 P0-F10：创建 color_card_issues 表（补齐 Batch 471 遗漏）+ color_cards 表新增库存字段
pub mod m0057_create_color_card_issues_and_stock_fields;
pub mod m0058_create_bulk_color_approval;
// Batch 479 P0-F18/F21：返工走生产订单 + 库存降级/报废（production_orders 加 order_type/original_batch_id, dye_batch_rework 加 production_order_id）
pub mod m0059_add_rework_order_fields;
// Batch 480 P0-F20：8D 质量管理流程（quality_8d_reports 表 + 11 态状态机 D0~D8 + closed）
pub mod m0060_create_quality_8d_reports;
// Batch 481 P0-B01：坏账准备计提表（账龄法：1y/2y/3y/over 5%/20%/50%/100%）
pub mod m0061_create_bad_debt_provisions;
// Batch 481 P0-B02：坏账核销审批表（二级审批流 pending→finance_approved→approved/rejected/cancelled）
pub mod m0062_create_bad_debt_writeoffs;
// Batch 481 P0-B03：催收任务表（自动生成 + 4 类型 phone/visit/email/letter + 优先级）
pub mod m0063_create_collection_tasks;
// Batch 481 P0-B04：财务预警表（4 类 ar_overdue/inventory_backlog/cash_flow_shortage/budget_overrun）
pub mod m0064_create_finance_alerts;
// Batch 483 P0-B11：定制订单补齐打样和报价环节（custom_orders 加 lab_dip_request_id + quotation_id）
pub mod m0065_add_custom_order_sample_quotation_fields;
// Batch 483 P0-B12：售后与质量集成（after_sales 加 quality_issue_id 关联 quality_issues）
pub mod m0066_add_after_sales_quality_issue_id;
// Batch 483 P0-B13：物流电子签收（logistics_waybills 加 signed_by/signed_at/sign_receipt_url/sign_photo_url/sign_remark）
pub mod m0067_add_logistics_waybill_sign_fields;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m0001_initial_schema::Migration),
            Box::new(m0002_add_crm_and_greige_tables::Migration),
            Box::new(m0003_add_dye_tables::Migration),
            Box::new(m0004_add_field_permissions::Migration),
            Box::new(m0005_add_basic_data_and_system_tables::Migration),
            Box::new(m0006_add_general_ledger_and_finance_base::Migration),
            Box::new(m0007_add_mrp_production_bom::Migration),
            Box::new(m0008_add_supplier_and_product_extensions::Migration),
            Box::new(m0009_add_purchase_extensions::Migration),
            Box::new(m0010_add_inventory_extensions::Migration),
            Box::new(m0011_add_sales_and_logistics_extensions::Migration),
            Box::new(m0012_add_ap_ar_finance_analysis::Migration),
            Box::new(m0013_add_business_process_and_traceability::Migration),
            Box::new(m0014_add_saas_notification_report_email_oa::Migration),
            Box::new(m0015_add_opportunity_id_to_sales_orders::Migration),
            Box::new(m0016_add_version_to_inventory_stocks::Migration),
            Box::new(m0017_add_crm_supplier_tables::Migration),
            Box::new(m0018_add_finance_tables::Migration),
            Box::new(m0019_add_missing_columns::Migration),
            Box::new(m0020_fix_schema_model_sync::Migration),
            Box::new(m0021_create_sales_quotations::Migration),
            Box::new(m0022_create_sales_quotation_items::Migration),
            Box::new(m0023_create_sales_quotation_terms::Migration),
            Box::new(m0024_create_product_color_prices::Migration),
            Box::new(m0025_p4_1_perf_indexes::Migration),
            Box::new(m0026_extend_audit_log::Migration),
            Box::new(m0027_enable_pg_stat_statements::Migration),
            Box::new(m0028_create_slow_query_log::Migration),
            // 批次 190：整合迁移必须在 m0029_drop_tenant_columns 之前执行，
            // 确保 custom_orders/process_nodes/color_cards 等表已创建
            Box::new(m0044_integrate_unreferenced_migrations::Migration),
            Box::new(m0029_drop_tenant_columns::Migration),
            Box::new(m0030_create_crm_recycle_rules::Migration),
            Box::new(m0031_add_signature_to_omni_audit_logs::Migration),
            Box::new(m0032_add_notes_to_custom_orders::Migration),
            Box::new(m0033_add_gain_loss_to_fixed_asset_disposals::Migration),
            Box::new(m0034_create_fixed_asset_depreciation_records::Migration),
            Box::new(m0035_create_customer_contacts::Migration),
            Box::new(m0036_create_api_endpoints::Migration),
            Box::new(m0037_alter_fa_depreciation_records_fk::Migration),
            Box::new(m0038_add_notes_to_ar_reconciliations::Migration),
            Box::new(m0039_add_created_by_to_api_keys::Migration),
            Box::new(m0040_create_crm_tags::Migration),
            Box::new(m0041_create_import_tasks::Migration),
            Box::new(m0042_create_purchase_inspection_items::Migration),
            Box::new(m0043_add_scan_type_and_industry_columns::Migration),
            Box::new(m0045_add_password_changed_at_to_users::Migration),
            Box::new(m0046_drop_audit_alert_rules::Migration),
            Box::new(m0047_add_last_payload_to_webhooks::Migration),
            Box::new(m0048_add_user_id_to_webhooks::Migration),
            Box::new(m0049_create_processed_events::Migration),
            Box::new(m0050_create_event_dead_letters::Migration),
            Box::new(m0051_add_data_scope_to_roles::Migration),
            Box::new(m0052_create_role_conflicts::Migration),
            Box::new(m0053_create_permission_change_audit::Migration),
            Box::new(m0054_enable_rls_policies::Migration),
            Box::new(m0055_create_export_approval_request::Migration),
            Box::new(m0056_add_condition_to_audit_logs::Migration),
            Box::new(m0057_create_color_card_issues_and_stock_fields::Migration),
            Box::new(m0058_create_bulk_color_approval::Migration),
            Box::new(m0059_add_rework_order_fields::Migration),
            Box::new(m0060_create_quality_8d_reports::Migration),
            Box::new(m0061_create_bad_debt_provisions::Migration),
            Box::new(m0062_create_bad_debt_writeoffs::Migration),
            Box::new(m0063_create_collection_tasks::Migration),
            Box::new(m0064_create_finance_alerts::Migration),
            Box::new(m0065_add_custom_order_sample_quotation_fields::Migration),
            Box::new(m0066_add_after_sales_quality_issue_id::Migration),
            Box::new(m0067_add_logistics_waybill_sign_fields::Migration),
        ]
    }
}
