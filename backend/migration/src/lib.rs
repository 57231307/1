//! 数据库迁移模块（m0001-m0074）。
//!
//! ## V15 P1 25.4-J 迁移兼容性规范（蓝绿部署保障）
//!
//! 蓝绿部署时新旧版本同时运行，schema 不兼容会导致旧版本写入失败。
//! 所有新增迁移必须遵守以下规则：
//!
//! 1. **新增字段**：必须 `NULLABLE` 或带 `DEFAULT` 值。
//!    - ✅ `ALTER TABLE t ADD COLUMN c VARCHAR NULL;`
//!    - ✅ `ALTER TABLE t ADD COLUMN c BOOLEAN NOT NULL DEFAULT FALSE;`
//!    - ❌ `ALTER TABLE t ADD COLUMN c VARCHAR NOT NULL;`（旧版本 INSERT 无此字段会失败）
//!
//! 2. **删除字段**：必须先废弃一个版本（先标记 deprecated，下一个版本再删除）。
//!    - 版本 N：应用层停止读写该字段
//!    - 版本 N+1：迁移删除该字段
//!
//! 3. **重命名字段**：必须分两步执行（跨两个版本）。
//!    - 版本 N：新增字段 + 应用层双写（old + new）
//!    - 版本 N+1：数据迁移 + 应用层切换读 new
//!    - 版本 N+2：删除 old 字段
//!
//! 4. **修改字段类型**：必须分步执行（兼容中间态）。
//!    - 版本 N：新增兼容类型字段 + 双写
//!    - 版本 N+1：数据迁移
//!    - 版本 N+2：应用层切换 + 删除旧字段
//!
//! 5. **新增约束（NOT NULL / CHECK / FK）**：必须先确保现有数据满足约束，
//!    且 `ALTER TABLE ADD CONSTRAINT` 在事务中执行（失败可回滚）。
//!
//! 启动时 `bootstrap::service_bootstrap::check_migration_compatibility` 会查询
//! `information_schema.columns` 检测违反规则 1 的 NOT NULL 无 DEFAULT 字段并 warn。

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
// Batch 484 P0-B15：缺料预警持久化（material_shortage_alerts + threshold_configs 两表）
pub mod m0068_create_material_shortage_tables;
// V15 批次 15 P1：补齐 supplier_evaluation_records 评估记录表迁移
pub mod m0069_create_supplier_evaluation_records;
// V15 P1 Batch-10 12.1：user_role 关联表（多对多，支持一个用户多角色）
pub mod m0070_create_user_role;
// V15 P1 Batch-09 10.3-1：color_card_issues 增加 sales_order_id 字段（订单驱动发放色卡场景）
pub mod m0071_add_sales_order_id_to_color_card_issues;
// V15 P1 Batch-10 12.6：permission_delegations 表（权限委托时限化 + 审计）
pub mod m0072_create_permission_delegations;
// V15 P1 Batch-10 12.2：role_relations 表（角色继承与互斥校验）
pub mod m0073_create_role_relations;
// V15 P1 迁移整合：执行 V15 P1 批次的 SQL 迁移文件（051-055）
pub mod m0074_v15_p1_integrate_sql_migrations;
// V15 P1 batch-16 缺陷 6.1/6.2/6.3：邮件异步队列 + 重试 + 附件（email_logs 新增 next_retry_at/attachments/html_content/text_content）
pub mod m0075_add_email_queue_fields;
// V15 P1 batch-11 缺陷 3-3：audit_logs 表导出专属字段（export_record_count/export_query_filter/export_file_format/export_approval_token/export_watermark_user）
pub mod m0076_add_export_audit_fields;
// V15 P1 batch-16 缺陷 7.2/7.3/8.3/8.4：OA 公告可见性 + 用户隐私同意 + 行为日志归档
pub mod m0077_add_oa_visibility_consent_retention;
// V15 P1 batch-18 缺陷 1.1/1.2/2.1/4.2/4.3/6.1/7.1/10.1/11.1/3.3：胚布采购关联 + 安全库存 + 委外胚布 + 8D根因 + 分级审批 + 补货策略 + 产能模型 + 工作中心实体
pub mod m0078_batch18_greige_outsourcing_quality_scheduling;
// V15 P1 batch-15 17.3-D5：催收模板表（话术标准化）
pub mod m0080_create_collection_templates;
// V15 P1 batch-15 17.8-D4：固定资产盘点表（盘点计划-执行-差异闭环）
pub mod m0081_create_fixed_asset_counts;
// V15 P1 batch-15 18.4-D2/D3：CRM 团队协作 + 数据共享时效
pub mod m0082_create_customer_team_and_share;
// V15 P1 batch-16 缺陷 1.1/4.1：报表模板版本管理 + 仪表板自定义卡片持久化
pub mod m0083_create_report_template_versions;
// V15 P1 batch-08 法律合规修复（环保/劳动/财税法律合规）：
// 缺陷 10/13/14/15/18/19/21/23/24 统一迁移
pub mod m0079_batch08_compliance_legal_env_tax_labor;
// V15 P1 batch-09 缺陷 10.4-1/10.4-2：补齐非 admin 角色的 color_card_issue:export 权限
// 为 sales_manager / warehouse_manager / cost_accountant 授予导出+成本字段查看权限
pub mod m0084_add_color_card_issue_export_permissions;
// V15 P1-10 batch-10：大货批色状态变更历史表（每次状态变更全量快照，支持追溯/责任/合规审计）
pub mod m0085_create_bulk_color_approval_history;
// V15 P1-21 缺陷 2.2：委外收回单关联质检记录（inspection_id 字段）
pub mod m0086_add_inspection_id_to_outsourcing_receipt;
// V15 P1 batch-19：组织定制物流 11 项 P1 修复（一人多部门/定制订单签字审批/售后闭环/物流运费/Incoterms）
pub mod m0087_batch19_custom_order_aftersales_logistics_incoterms;
// V15 缺陷 10-4：审计日志导出二次审计表（防篡改，独立于 audit_logs）
pub mod m0088_audit_log_export_log;
// V15 P2 B05-P2-2：dye_batch_rework 表新增 rework_cost 字段（配合 re_dye/replenish_dye 枚举扩展）
pub mod m0089_add_rework_cost_to_dye_batch_rework;
// V15 P2 B05-P2-6：染缸设备占用/释放记录表（缸号进入 dyeing 占用 / 离开 dyeing 释放）
pub mod m0090_create_dye_vat_occupation;
// V15 P2 B05-P2-7：PDA / 工控终端连接资源管理表（注册 / 心跳 / 下线 / 超时清理）
pub mod m0091_create_device_connection;
// V15 P2 B05-P2-10：期末调整记录表（暂估 / 摊销 / 预提）
pub mod m0092_create_period_adjustment_record;
pub mod m0093_add_category_id_to_suppliers;
pub mod m0094_add_processor_fields_to_suppliers;
pub mod m0095_create_sales_contract_items;
pub mod m0096_create_period_report_snapshot;
pub mod m0097_create_aging_alert_rules;

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
            Box::new(m0068_create_material_shortage_tables::Migration),
            Box::new(m0069_create_supplier_evaluation_records::Migration),
            Box::new(m0070_create_user_role::Migration),
            Box::new(m0071_add_sales_order_id_to_color_card_issues::Migration),
            Box::new(m0072_create_permission_delegations::Migration),
            Box::new(m0073_create_role_relations::Migration),
            Box::new(m0074_v15_p1_integrate_sql_migrations::Migration),
            Box::new(m0075_add_email_queue_fields::Migration),
            Box::new(m0076_add_export_audit_fields::Migration),
            Box::new(m0077_add_oa_visibility_consent_retention::Migration),
            Box::new(m0078_batch18_greige_outsourcing_quality_scheduling::Migration),
            Box::new(m0079_batch08_compliance_legal_env_tax_labor::Migration),
            Box::new(m0080_create_collection_templates::Migration),
            Box::new(m0081_create_fixed_asset_counts::Migration),
            Box::new(m0082_create_customer_team_and_share::Migration),
            Box::new(m0083_create_report_template_versions::Migration),
            Box::new(m0084_add_color_card_issue_export_permissions::Migration),
            Box::new(m0085_create_bulk_color_approval_history::Migration),
            Box::new(m0086_add_inspection_id_to_outsourcing_receipt::Migration),
            Box::new(m0087_batch19_custom_order_aftersales_logistics_incoterms::Migration),
            Box::new(m0088_audit_log_export_log::Migration),
            Box::new(m0089_add_rework_cost_to_dye_batch_rework::Migration),
            Box::new(m0090_create_dye_vat_occupation::Migration),
            Box::new(m0091_create_device_connection::Migration),
            Box::new(m0092_create_period_adjustment_record::Migration),
            Box::new(m0093_add_category_id_to_suppliers::Migration),
            Box::new(m0094_add_processor_fields_to_suppliers::Migration),
            Box::new(m0095_create_sales_contract_items::Migration),
            Box::new(m0096_create_period_report_snapshot::Migration),
            Box::new(m0097_create_aging_alert_rules::Migration),
        ]
    }
}
