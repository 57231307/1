pub mod assist_accounting_service;
pub mod auth_service;
pub mod batch_service;
pub mod business_trace_service;
pub mod crm_service;
pub mod customer_service;
pub mod dashboard_service;
pub mod department_service;
pub mod finance_invoice_service;
pub mod finance_payment_service;
pub mod finance_report_service;
pub mod five_dimension_query_service;
pub mod five_dimension_service;
pub mod init_service;
pub mod inventory_adjustment_service;
pub mod inventory_count_service;
pub mod inventory_finance_bridge_service;
pub mod inventory_reservation_service;
pub mod inventory_stock_service;
pub mod inventory_transfer_service;
pub mod product_category_service;
pub mod product_service;
pub mod role_permission_service;
pub mod sales_service;
pub mod user_service;
pub mod warehouse_service;
// 供应商管理模块
pub mod supplier_evaluation_service;
pub mod supplier_service;
// 采购管理模块
pub mod purchase_inspection_service;
pub mod purchase_order_service;
pub mod purchase_receipt_service;
pub mod purchase_return_service;
// 应付管理模块
pub mod ap_invoice_service;
pub mod ap_payment_request_service;
pub mod ap_payment_service;
pub mod ap_reconciliation_service;
pub mod ap_report_service;
pub mod ap_verification_service;
// 应收管理模块
pub mod ar_invoice_service;
// 总账管理模块
pub mod account_subject_service;
pub mod accounting_period_service;
pub mod voucher_service;
// 成本管理模块
pub mod cost_collection_service;
// P1 模块
pub mod budget_management_service;
pub mod customer_credit_service;
pub mod fixed_asset_service;
pub mod fund_management_service;
pub mod purchase_contract_service;
pub mod quality_standard_service;
pub mod sales_contract_service;
// P2 模块
pub mod audit_log_service;
pub mod bpm_service;
pub mod event_bus;
pub mod financial_analysis_service;
pub mod metrics_service;
pub mod omni_audit_query_service;
pub mod omni_audit_service;
pub mod operation_log_service;
pub mod order_change_history_service;
pub mod purchase_delivery_calculator;
pub mod purchase_price_service;
pub mod quality_inspection_service;
pub mod sales_analysis_service;
pub mod sales_price_service;
pub mod sales_return_service;
pub mod system_update_service;
pub mod totp_service;
pub mod transaction_helper;
// MRP生产计划模块
pub mod bom_service;
pub mod mrp_engine_service;
pub mod production_order_service;
// 应收对账与多币种模块
pub mod ar_reconciliation_service;
pub mod currency_service;
// AI智能分析与报表模块
pub mod ai_analysis_service;
pub mod report_engine_service;
// 多租户SaaS模块
pub mod api_key_service;
pub mod tenant_isolation_service;
pub mod tenant_service;
pub mod webhook_service;
// 消息通知模块
pub mod data_permission_service;
pub mod email_service;
pub mod event_notification_service;
pub mod notification_service;
pub mod user_notification_setting_service;
// 产能分析模块
pub mod capacity_service;
// 缺料预警模块
pub mod material_shortage_service;
// 生产排程模块
pub mod scheduling_service;
// 字段权限模块
pub mod field_permission_service;
// 租户计费模块
pub mod tenant_billing_service;
// 导入导出模块
pub mod import_export_service;
// 报表模板模块
pub mod report_template_service;
// 邮件模板模块
pub mod email_template_service;
// 邮件发送记录模块
pub mod email_log_service;
// 分配历史模块
pub mod assignment_history_service;
// 报表订阅模块
pub mod report_subscription_service;
// 定时任务调度器模块
pub mod scheduler_service;
// 导出服务模块
pub mod export_service;
// 通用打印服务
pub mod print_service;
