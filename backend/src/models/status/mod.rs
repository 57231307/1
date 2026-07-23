//! 业务状态常量模块
//!
//! 批次 100 P3-A 修复（v5 复审）：抽取 4 个 service 文件中的硬编码状态字符串为常量，
//! 提高可维护性，避免字符串拼写错误导致状态匹配失败。
//!
//! 按业务域分组：
//! - 通用状态：DRAFT/PENDING/APPROVED/CANCELLED/COMPLETED/ACTIVE（多业务共用）
//! - 生产订单专属：SCHEDULED/IN_PROGRESS/PENDING_APPROVAL/REJECTED
//! - 付款专属：REGISTERED/CONFIRMED/PAID/PARTIAL_PAID
//! - 采购订单状态：批次 158 v11 真实接入 po/ 子模块
//! - 销售订单状态：批次 158 v11 真实接入 so/ 子模块
//! - 通用审批状态：批次 158 v11 真实接入 color_price / budget_adjustment / ar_invoice
//! - 库存预留状态：批次 158 v11 真实接入 so/delivery
//! - 销售发货状态：批次 158 v11 真实接入 so/delivery
//!
//! 批次 490 D10-3b 拆分：本文件作为 facade，原 100 个 `pub mod` 块按业务域分组迁移到 8 个子文件。
//! 通过 `mod group_xxx; pub use group_xxx::*;` 模式保持外部引用路径
//! `crate::models::status::<sub>::<CONST>` 不变。
//! 分组文件：
//! - general：common/payment/master_data/import_task/login_log/email_log/active_status/audit_message/health_check/reconcile_result/failover
//! - production：production/scheduling/process_node/mrp/work_center/flow_card/step_record
//! - sales：sales_order/sales_delivery/sales_return/quotation/custom_order/quotation_ext/price_approval/custom_order_ext/sales_fabric_order
//! - purchase_inventory：purchase_order/purchase_receipt/inventory_reservation/inventory_transfer/inventory_count/purchase_return/purchase_inspection/inventory_adjustment/inventory_piece/purchase_receipt_inspection
//! - finance：ar/ap_invoice/ap_payment_request/voucher/accounting_period/finance_invoice/finance_payment/ap_reconciliation/ap_verification/fixed_asset/cost_collection/accounting_period_closing
//! - bpm_crm_contract：approval/budget/contract/logistics_waybill/bpm_instance/bpm_task/crm_lead/crm_opportunity/contract_status
//! - quality_dyeing：quality_standard/quality_handling/dye_recipe/lab_dip_request/lab_dip_sample/lab_dip_resample/production_recipe/production_recipe_addition/quality_feedback/fabric_inspection/fabric_scoring/fabric_grade/dye_batch_*
//! - wage_energy_chemical_business：wage_*/energy_*/color_card/chemical_*/outsourcing_*/business_*

mod general;
// production 需为 pub：调用方直接访问 status::production::PRODUCTION_*（文件名与原始内部模块名相同）
pub mod production;
mod sales;
mod purchase_inventory;
mod finance;
mod bpm_crm_contract;
mod quality_dyeing;
mod wage_energy_chemical_business;

pub use general::*;
pub use production::*;
pub use sales::*;
pub use purchase_inventory::*;
pub use finance::*;
pub use bpm_crm_contract::*;
pub use quality_dyeing::*;
pub use wage_energy_chemical_business::*;
