pub mod customer;
pub mod customer_followup;
pub mod department;
pub mod dto;
pub mod finance_invoice;
pub mod finance_payment;
pub mod inventory_adjustment;
pub mod inventory_adjustment_item;
pub mod inventory_count;
pub mod inventory_count_item;
pub mod inventory_reservation;
pub mod inventory_stock;
pub mod inventory_transaction;
pub mod inventory_transfer;
pub mod inventory_transfer_item;
pub mod operation_log;
pub mod product;
pub mod product_category;
pub mod product_color;
pub mod role;
pub mod role_permission;
pub mod sales_order;
pub mod sales_order_change_history;
pub mod sales_order_item;
pub mod status;
pub mod user;
// MRP生产计划模块
pub mod bom;
pub mod bom_item;
pub mod mrp_result;
pub mod production_order;
pub mod scheduling_result;
pub mod work_center;
// 应收对账与多币种模块
pub mod ar_reconciliation;
pub mod ar_reconciliation_item;
pub mod currency;
pub mod exchange_rate;
pub mod warehouse;
// 总账模块
pub mod account_balance;
pub mod account_subject;
pub mod accounting_period;
pub mod voucher;
pub mod voucher_item;
// 辅助核算模块
pub mod assist_accounting_dimension;
pub mod assist_accounting_record;
pub mod assist_accounting_summary;
// 业务追溯模块
pub mod business_trace_chain;
pub mod business_trace_snapshot;
// 供应商管理模块
pub mod supplier;
pub mod supplier_category;
pub mod supplier_contact;
pub mod supplier_evaluation;
pub mod supplier_evaluation_record;
pub mod supplier_product;
pub mod supplier_qualification;
// 采购管理模块
pub mod purchase_order;
pub mod purchase_order_item;
pub mod purchase_receipt;
pub mod purchase_receipt_item;
// 应付管理模块
pub mod ap_invoice;
pub mod ap_payment;
pub mod ap_payment_request;
pub mod ap_payment_request_item;
pub mod ap_reconciliation;
pub mod ap_verification;
pub mod ap_verification_item;
// 应收账款模块
pub mod ar_collection;
pub mod ar_invoice;
// 成本管理模块
pub mod cost_analysis;
pub mod cost_collection;
// P1 模块
pub mod budget_management;
pub mod budget_plan;
pub mod customer_credit;
pub mod fixed_asset;
pub mod fund_management;
pub mod purchase_contract;
pub mod quality_standard;
pub mod sales_contract;
// P2 模块
pub mod financial_analysis;
pub mod financial_analysis_result;
pub mod purchase_price;
pub mod quality_inspection;
pub mod sales_analysis;
pub mod sales_price;
// P2 模块 - 质量检验记录
pub mod quality_inspection_record;
pub mod unqualified_product;
// 采购退货模块
pub mod purchase_return;
pub mod purchase_return_item;
// 销售报价单模块（Week 1）
pub mod sales_quotation;
pub mod sales_quotation_item;
pub mod sales_quotation_term;
pub mod product_color_price;
pub mod quotation_create_dto;
pub mod quotation_update_dto;
pub mod quotation_response_dto;
pub mod quotation_convert_dto;
// 采购检验模块
pub mod purchase_inspection;
// 采购合同执行模块
pub mod purchase_contract_execution;
// 销售交货模块
pub mod sales_delivery;
pub mod sales_delivery_item;
// 固定资产处置模块
pub mod fixed_asset_disposal;
// 资金转账记录模块
pub mod fund_transfer_record;
// 面料行业核心模块
pub mod dye_batch;
pub mod dye_lot_mapping;
pub mod dye_recipe;
pub mod greige_fabric;
// 匹数管理模块
pub mod inventory_piece;
// 仓位/库位模块
pub mod location;
// BPM流程模块
pub mod bpm_process_definition;
pub mod bpm_process_instance;
pub mod bpm_task;
// 预算执行模块
pub mod budget_adjustment;
pub mod budget_execution;
// 业务追溯模块
pub mod business_trace;
// CRM模块
pub mod approval_instance;
pub mod approval_log;
pub mod approval_node;
pub mod approval_template;
pub mod audit_alert_rule;
pub mod audit_log;
pub mod crm_lead;
pub mod crm_opportunity;
pub mod logistics_waybill;
pub mod omni_audit_log;
pub mod sales_return;
pub mod sales_return_item;
// 多租户SaaS模块
pub mod api_key;
pub mod tenant;
pub mod tenant_config;
pub mod tenant_invoice;
pub mod tenant_plan;
pub mod tenant_subscription;
pub mod tenant_usage;
pub mod tenant_user;
pub mod webhook;
// 消息通知模块
pub mod notification;
pub mod notification_setting;
pub mod user_notification_setting;
// 数据权限模块
pub mod data_permission;
// 字段权限模块
pub mod field_permission;
// 报表模板模块
pub mod report_subscription;
pub mod report_template;
// 邮件模块
pub mod email_log;
pub mod email_template;
// CRM分配历史模块
pub mod assignment_history;
// 登录日志模块
pub mod log_login;
// 供应商产品颜色模块
pub mod supplier_product_color;
// 产品供应商映射模块
pub mod product_supplier_mapping;
// 产品编码映射模块
pub mod product_code_mapping;
// 资金账户模块
pub mod fund_account;
// API访问日志模块
pub mod log_api_access;
// 系统日志模块
pub mod log_system;
// 系统版本模块
pub mod system_version;
// 公告模块
pub mod oa_announcement;
// 颜色编码映射模块
pub mod color_code_mapping;
// 批次追溯日志模块
pub mod batch_trace_log;
// 批次染缸映射模块
pub mod batch_dye_lot;
// 匹数映射模块
pub mod piece_mapping;
// 成本模块
// P1P2模块
// 应收账龄分析模块
pub mod ar_aging_analysis;
// 供应商基础模块
pub mod supplier_blacklist;
pub mod supplier_grade;
// 业务追溯辅助链接模块
pub mod business_trace_assist_link;
pub mod business_trace_view;
// 字段权限模块
pub mod report_definition;
// 增强审计日志已废弃（无对应 migration + 0 业务引用）→ 整个 _legacy/ 目录已清理
// P0-2 主备隔离模块
pub mod failover_config;
pub mod failover_event;
pub mod failover_status;
