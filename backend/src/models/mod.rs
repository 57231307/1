#![allow(dead_code, unused_imports, unused_variables)]
pub mod customer;
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
pub mod sales_order_item;
pub mod user;
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
pub mod business_trace_view;
// 供应商管理模块
pub mod product_supplier_mapping;
pub mod supplier;
pub mod supplier_blacklist;
pub mod supplier_category;
pub mod supplier_contact;
pub mod supplier_evaluation;
pub mod supplier_evaluation_record;
pub mod supplier_grade;
pub mod supplier_product;
pub mod supplier_product_color;
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
pub mod ar_mod;
// 成本管理模块
pub mod cost_analysis;
pub mod cost_collection;
pub mod cost_mod;
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
// 采购检验模块
pub mod purchase_inspection;
// 采购合同执行模块
pub mod purchase_contract_execution;
// 销售交货模块
pub mod sales_delivery;
pub mod sales_delivery_item;
// 固定资产处置模块
pub mod fixed_asset_disposal;
// 资金账户模块
pub mod fund_account;
// 资金转账记录模块
pub mod fund_transfer_record;
// 批次相关模块
pub mod batch_dye_lot;
pub mod batch_trace_log;
// 面料行业核心模块
pub mod dye_batch;
pub mod dye_lot_mapping;
pub mod dye_recipe;
pub mod greige_fabric;
// 产品编码映射模块
pub mod product_code_mapping;
// 色号映射模块
pub mod color_code_mapping;
// 匹号映射模块
pub mod piece_mapping;
// 匹数管理模块
pub mod inventory_piece;
// _LOCATION模块
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
pub mod business_trace_assist_link;
// CRM模块
pub mod crm_lead;
pub mod crm_opportunity;
// OA模块
pub mod oa_announcement;
// 日志模块
pub mod log_api_access;
pub mod log_login;
pub mod log_system;
// 报表模块
pub mod report_definition;
// AR模块（应收账款模块）- 在第62行已定义
pub mod logistics_waybill;
pub mod sales_return;
pub mod sales_return_item;
