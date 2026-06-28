pub mod account_subject_handler;
pub mod accounting_period_handler;
pub mod assist_accounting_handler;
pub mod auth_handler;
// P9-2 D 拆分：认证相关辅助 handler 独立成文件
pub mod auth_handler_misc;
pub mod auth_handler_session;
pub mod bulk_product_handler;
pub mod business_trace_handler;
pub mod crm_handler;
pub mod customer_handler;
pub mod dashboard_handler;
pub mod department_handler;
pub mod dual_unit_converter_handler;
pub mod finance_invoice_handler;
pub mod finance_payment_handler;
pub mod finance_report_handler;
pub mod five_dimension_handler;
pub mod health_handler;
pub mod init_handler;
pub mod inventory_adjustment_handler;
pub mod inventory_batch_handler;
pub mod inventory_count_handler;
pub mod inventory_reservation_handler;
pub mod inventory_stock_handler;
pub mod inventory_stock_handler_dto;
pub mod inventory_stock_handler_fabric;
pub mod inventory_stock_handler_query;
pub mod inventory_transfer_handler;
pub mod omni_audit_handler;
pub mod product_category_handler;
pub mod product_handler;
pub mod role_handler;
pub mod sales_fabric_order_handler;
pub mod sales_order_handler;
// 销售报价单 handler（Week 1）
pub mod quotation_handler;
pub mod tracking_handler;
// 定制订单全流程跟踪 handler（P0-3）
pub mod custom_order_handler;
pub mod user_handler;
pub mod warehouse_handler;
// 供应商管理模块
pub mod supplier_handler;
// 采购管理模块
pub mod purchase_inspection_handler;
pub mod purchase_order_handler;
pub mod purchase_receipt_handler;
pub mod purchase_return_handler;
// 应付管理模块
pub mod ap_invoice_handler;
pub mod ap_payment_handler;
pub mod ap_payment_request_handler;
pub mod ap_reconciliation_handler;
pub mod ap_report_handler;
pub mod ap_verification_handler;
// 应收管理模块
pub mod ar_invoice_handler;
pub mod ar_payment_handler;
pub mod ar_report_handler;
pub mod ar_verification_handler;
// 总账管理模块
pub mod voucher_handler;
// 成本管理模块
pub mod cost_collection_handler;
// P1 模块
pub mod budget_management_handler;
pub mod customer_credit_handler;
pub mod fixed_asset_handler;
pub mod fund_management_handler;
pub mod purchase_contract_handler;
pub mod quality_standard_handler;
pub mod sales_contract_handler;
// P2 模块
pub mod financial_analysis_handler;
pub mod purchase_price_handler;
pub mod quality_inspection_handler;
pub mod sales_analysis_handler;
pub mod sales_price_handler;
pub mod supplier_evaluation_handler;
// 面料行业核心模块
pub mod bpm_handler;
pub mod dye_batch_handler;
pub mod dye_recipe_handler;
pub mod greige_fabric_handler;
pub mod sales_return_handler;
pub mod system_update_handler;
// MRP生产计划模块
pub mod barcode_scanner_handler;
pub mod bom_handler;
pub mod logistics_handler;
pub mod mrp_handler;
pub mod piece_split_handler;
pub mod production_order_handler;
// 多租户SaaS模块
pub mod api_gateway_handler;
pub mod api_key_handler;
pub mod webhook_handler;
// Phase 2-3 补充Handler
pub mod ai_analysis_handler;
pub mod ar_reconciliation_enhanced_handler;
pub mod ar_reconciliation_handler;
pub mod currency_handler;
pub mod report_engine_handler;
// P2-4 AI 分析深化（工艺优化 + 质量预测）
pub mod ai_extend_handler;
// 消息通知模块
pub mod notification_handler;
// 用户通知偏好设置模块
pub mod user_notification_setting_handler;
// 数据权限模块
pub mod data_permission_handler;
// AI 高级分析模块
pub mod advanced;
// 多币种增强模块
pub mod currency_enhanced_handler;
// MRP产能和缺料模块
pub mod capacity_handler;
pub mod material_shortage_handler;
// 报表和导入导出模块
pub mod import_export_handler;
pub mod report_enhanced_handler;
// BPM定义模块
pub mod bpm_definition_handler;
// CRM增强模块
pub mod crm_assignment_handler;
pub mod crm_customer_handler;
pub mod crm_pool_handler;
// 系统级功能模块
pub mod audit_enhanced_handler;
pub mod audit_log_handler;
pub mod email_handler;
// P3-4 BI 多维分析 handler
pub mod bi_handler;
pub mod login_security_handler;
pub mod slow_query_handler;
pub mod webhook_integration_handler;
// 生产排程模块
pub mod scheduling_handler;
// 字段权限模块
pub mod field_permission_handler;
// 通用打印模块
pub mod print_handler;
// 缺失的 handler 补充
pub mod missing_handlers;
// P0-2 主备隔离 handler
pub mod failover_handler;
// P0-4 色卡仓储管理 handler
pub mod color_card;
// 面料多色号定价扩展 handler（P0-5）
pub mod color_price_handler;
