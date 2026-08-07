//! 通用打印服务
//!
//! V15 P0-S17 修复（Batch 476）：6 个 get_*_print_data 方法从硬编码占位数据
//! 改为真实查询数据库（主表 + 关联客户/供应商/仓库 + 明细项）。

use crate::utils::error::AppError;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, LoaderTrait, ModelTrait, Order, QueryFilter,
    QueryOrder,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// 打印数据类型
#[derive(Debug, Serialize, Deserialize)]
pub struct PrintData {
    pub template: String,
    pub data: HashMap<String, serde_json::Value>,
    pub items: Vec<HashMap<String, serde_json::Value>>,
}

/// 打印服务（V15 P0-S17：持有数据库连接，6 个 get_*_print_data 方法真实查询数据库）
pub struct PrintService {
    db: Arc<DatabaseConnection>,
}

/// 库存调拨打印上下文：聚合主表、调出/调入仓库、明细及产品映射
struct TransferPrintContext {
    transfer: crate::models::inventory_transfer::Model,
    from_warehouse: Option<crate::models::warehouse::Model>,
    to_warehouse: Option<crate::models::warehouse::Model>,
    items: Vec<crate::models::inventory_transfer_item::Model>,
    product_map: HashMap<i32, crate::models::product::Model>,
}

impl PrintService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取打印数据
    pub async fn get_print_data(&self, doc_type: &str, doc_id: i32) -> Result<PrintData, AppError> {
        match doc_type {
            "sales_order" => self.get_sales_order_print_data(doc_id).await,
            "sales_contract" => self.get_sales_contract_print_data(doc_id).await,
            "purchase_order" => self.get_purchase_order_print_data(doc_id).await,
            "purchase_receipt" => self.get_purchase_receipt_print_data(doc_id).await,
            "inventory_transfer" => self.get_inventory_transfer_print_data(doc_id).await,
            "voucher" => self.get_voucher_print_data(doc_id).await,
            "production_flow_card" => self.get_production_flow_card_print_data(doc_id).await,
            "fabric_inspection" => self.get_fabric_inspection_print_data(doc_id).await,
            "dye_batch_card" => self.get_dye_batch_card_print_data(doc_id).await,
            "color_card_issue" => self.get_color_card_issue_print_data(doc_id).await,
            "bulk_color_approval" => self.get_bulk_color_approval_print_data(doc_id).await,
            "lab_dip_request" => self.get_lab_dip_request_print_data(doc_id).await,
            "production_order" => self.get_production_order_print_data(doc_id).await,
            "production_recipe" => self.get_production_recipe_print_data(doc_id).await,
            "quality_inspection_record" => self.get_quality_inspection_record_print_data(doc_id).await,
            "sales_delivery" => self.get_sales_delivery_print_data(doc_id).await,
            "ar_collection" => self.get_ar_collection_print_data(doc_id).await,
            "ap_payment" => self.get_ap_payment_print_data(doc_id).await,
            "ap_invoice" => self.get_ap_invoice_print_data(doc_id).await,
            "sales_quotation" => self.get_sales_quotation_print_data(doc_id).await,
            "sales_return" => self.get_sales_return_print_data(doc_id).await,
            "purchase_return" => self.get_purchase_return_print_data(doc_id).await,
            "outsourcing_order" => self.get_outsourcing_order_print_data(doc_id).await,
            "outsourcing_receipt" => self.get_outsourcing_receipt_print_data(doc_id).await,
            "logistics_waybill" => self.get_logistics_waybill_print_data(doc_id).await,
            "certificate_of_origin" => self.get_certificate_of_origin_print_data(doc_id).await,
            "export_customs_declaration" => self.get_export_customs_declaration_print_data(doc_id).await,
            "solid_waste_disposal" => self.get_solid_waste_disposal_print_data(doc_id).await,
            "unqualified_product" => self.get_unqualified_product_print_data(doc_id).await,
            "chemical_requisition" => self.get_chemical_requisition_print_data(doc_id).await,
            "export_inspection" => self.get_export_inspection_print_data(doc_id).await,
            "ap_payment_request" => self.get_ap_payment_request_print_data(doc_id).await,
            "ap_reconciliation" => self.get_ap_reconciliation_print_data(doc_id).await,
            "purchase_inspection" => self.get_purchase_inspection_print_data(doc_id).await,
            "inventory_adjustment" => self.get_inventory_adjustment_print_data(doc_id).await,
            "bom" => self.get_bom_print_data(doc_id).await,
            "material_shortage" => self.get_material_shortage_print_data(doc_id).await,
            "quality_8d_report" => self.get_quality_8d_report_print_data(doc_id).await,
            "labor_contract" => self.get_labor_contract_print_data(doc_id).await,
            "wage_record" => self.get_wage_record_print_data(doc_id).await,
            "energy_consumption_record" => self.get_energy_consumption_record_print_data(doc_id).await,
            "purchase_contract" => self.get_purchase_contract_print_data(doc_id).await,
            "supplier_evaluation_record" => self.get_supplier_evaluation_record_print_data(doc_id).await,
            "safety_accident_report" => self.get_safety_accident_report_print_data(doc_id).await,
            "occupational_hazard_monitoring" => self.get_occupational_hazard_monitoring_print_data(doc_id).await,
            "pollution_permit" => self.get_pollution_permit_print_data(doc_id).await,
            "process_route" => self.get_process_route_print_data(doc_id).await,
            "foreign_exchange_verification" => self.get_foreign_exchange_verification_print_data(doc_id).await,
            "export_refund_declaration" => self.get_export_refund_declaration_print_data(doc_id).await,
            "fixed_asset" => self.get_fixed_asset_print_data(doc_id).await,
            "scheduling_result" => self.get_scheduling_result_print_data(doc_id).await,
            "inventory_write_down" => self.get_inventory_write_down_print_data(doc_id).await,
            "fixed_asset_count" => self.get_fixed_asset_count_print_data(doc_id).await,
            "social_insurance_record" => self.get_social_insurance_record_print_data(doc_id).await,
            "occupational_health_exam" => self.get_occupational_health_exam_print_data(doc_id).await,
            "dye_batch_rework" => self.get_dye_batch_rework_print_data(doc_id).await,
            "bad_debt_writeoff" => self.get_bad_debt_writeoff_print_data(doc_id).await,
            "custom_order" => self.get_custom_order_print_data(doc_id).await,
            "ppe_distribution" => self.get_ppe_distribution_print_data(doc_id).await,
            "customer_credit" => self.get_customer_credit_print_data(doc_id).await,
            "after_sales" => self.get_after_sales_print_data(doc_id).await,
            "quality_issue" => self.get_quality_issue_print_data(doc_id).await,
            "ar_reconciliation" => self.get_ar_reconciliation_print_data(doc_id).await,
            _ => Err(AppError::not_found(format!(
                "Unknown document type: {}",
                doc_type
            ))),
        }
    }

    /// 销售订单打印数据：订单主表 + 客户 + 明细项（含产品）
    async fn get_sales_order_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        let (order, customer, items, products) =
            Self::fetch_sales_order_relations(&*self.db, id).await?;
        let data = Self::build_sales_order_main_data(&order, customer.as_ref());
        let item_list = Self::build_sales_order_item_list(items, &products);
        Ok(PrintData {
            template: "sales_order".to_string(),
            data,
            items: item_list,
        })
    }

    /// 销售订单打印：批量查询订单 + 客户 + 明细 + 产品
    async fn fetch_sales_order_relations(
        db: &DatabaseConnection,
        id: i32,
    ) -> Result<
        (
            crate::models::sales_order::Model,
            Option<crate::models::customer::Model>,
            Vec<crate::models::sales_order_item::Model>,
            Vec<Option<crate::models::product::Model>>,
        ),
        AppError,
    > {
        use crate::models::{customer, product, sales_order, sales_order_item};
        let order = sales_order::Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售订单 {} 未找到", id)))?;
        let customer = order.find_related(customer::Entity).one(db).await?;
        let items = order
            .find_related(sales_order_item::Entity)
            .order_by(sales_order_item::Column::Id, Order::Asc)
            .all(db)
            .await?;
        let products = items.load_one(product::Entity, db).await?;
        Ok((order, customer, items, products))
    }

    /// 销售订单打印：构造主表 data
    fn build_sales_order_main_data(
        order: &crate::models::sales_order::Model,
        customer: Option<&crate::models::customer::Model>,
    ) -> HashMap<String, serde_json::Value> {
        let mut data = HashMap::new();
        data.insert(
            "order_no".to_string(),
            serde_json::json!(order.order_no.clone()),
        );
        data.insert(
            "customer_name".to_string(),
            serde_json::json!(customer
                .map(|c| c.customer_name.clone())
                .unwrap_or_default()),
        );
        data.insert(
            "customer_code".to_string(),
            serde_json::json!(customer
                .map(|c| c.customer_code.clone())
                .unwrap_or_default()),
        );
        data.insert(
            "order_date".to_string(),
            serde_json::json!(order.order_date.format("%Y-%m-%d").to_string()),
        );
        data.insert(
            "required_date".to_string(),
            serde_json::json!(order.required_date.format("%Y-%m-%d").to_string()),
        );
        data.insert(
            "ship_date".to_string(),
            serde_json::json!(order
                .ship_date
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_default()),
        );
        data.insert(
            "status".to_string(),
            serde_json::json!(order.status.clone()),
        );
        data.insert(
            "subtotal".to_string(),
            serde_json::json!(order.subtotal.to_string()),
        );
        data.insert(
            "tax_amount".to_string(),
            serde_json::json!(order.tax_amount.to_string()),
        );
        data.insert(
            "discount_amount".to_string(),
            serde_json::json!(order.discount_amount.to_string()),
        );
        data.insert(
            "shipping_cost".to_string(),
            serde_json::json!(order.shipping_cost.to_string()),
        );
        data.insert(
            "total_amount".to_string(),
            serde_json::json!(order.total_amount.to_string()),
        );
        data.insert(
            "paid_amount".to_string(),
            serde_json::json!(order.paid_amount.to_string()),
        );
        data.insert(
            "balance_amount".to_string(),
            serde_json::json!(order.balance_amount.to_string()),
        );
        data.insert(
            "shipping_address".to_string(),
            serde_json::json!(order.shipping_address.clone().unwrap_or_default()),
        );
        data.insert(
            "billing_address".to_string(),
            serde_json::json!(order.billing_address.clone().unwrap_or_default()),
        );
        data.insert(
            "notes".to_string(),
            serde_json::json!(order.notes.clone().unwrap_or_default()),
        );
        data
    }

    /// 销售订单打印：构造明细行列表
    fn build_sales_order_item_list(
        items: Vec<crate::models::sales_order_item::Model>,
        products: &[Option<crate::models::product::Model>],
    ) -> Vec<HashMap<String, serde_json::Value>> {
        let mut item_list = Vec::with_capacity(items.len());
        for (i, item) in items.into_iter().enumerate() {
            let product = products[i].as_ref();
            let mut row = HashMap::new();
            row.insert(
                "line_no".to_string(),
                serde_json::json!((i + 1).to_string()),
            );
            row.insert(
                "product_code".to_string(),
                serde_json::json!(product.map(|p| p.code.clone()).unwrap_or_default()),
            );
            row.insert(
                "product_name".to_string(),
                serde_json::json!(product.map(|p| p.name.clone()).unwrap_or_default()),
            );
            row.insert("color_no".to_string(), serde_json::json!(item.color_no));
            row.insert(
                "color_name".to_string(),
                serde_json::json!(item.color_name.unwrap_or_default()),
            );
            row.insert(
                "quantity".to_string(),
                serde_json::json!(item.quantity.to_string()),
            );
            row.insert(
                "unit_price".to_string(),
                serde_json::json!(item.unit_price.to_string()),
            );
            row.insert(
                "final_price".to_string(),
                serde_json::json!(item.final_price.map(|p| p.to_string()).unwrap_or_default()),
            );
            row.insert(
                "subtotal".to_string(),
                serde_json::json!(item.subtotal.to_string()),
            );
            row.insert(
                "tax_amount".to_string(),
                serde_json::json!(item.tax_amount.to_string()),
            );
            row.insert(
                "total_amount".to_string(),
                serde_json::json!(item.total_amount.to_string()),
            );
            row.insert(
                "quantity_meters".to_string(),
                serde_json::json!(item.quantity_meters.to_string()),
            );
            row.insert(
                "quantity_kg".to_string(),
                serde_json::json!(item.quantity_kg.to_string()),
            );
            item_list.push(row);
        }
        item_list
    }

    /// 销售合同打印数据：合同主表 + 客户
    async fn get_sales_contract_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::{customer, sales_contract};

        let contract = sales_contract::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售合同 {} 未找到", id)))?;

        let customer = customer::Entity::find_by_id(contract.customer_id)
            .one(&*self.db)
            .await?;

        let mut data = HashMap::new();
        data.insert(
            "contract_no".to_string(),
            serde_json::json!(contract.contract_no),
        );
        data.insert(
            "contract_name".to_string(),
            serde_json::json!(contract.contract_name),
        );
        data.insert(
            "contract_type".to_string(),
            serde_json::json!(contract.contract_type.unwrap_or_default()),
        );
        data.insert(
            "customer_name".to_string(),
            serde_json::json!(contract.customer_name.clone().unwrap_or_else(|| {
                customer
                    .as_ref()
                    .map(|c| c.customer_name.clone())
                    .unwrap_or_default()
            })),
        );
        data.insert(
            "customer_code".to_string(),
            serde_json::json!(customer
                .as_ref()
                .map(|c| c.customer_code.clone())
                .unwrap_or_default()),
        );
        data.insert(
            "total_amount".to_string(),
            serde_json::json!(contract
                .total_amount
                .map(|a| a.to_string())
                .unwrap_or_default()),
        );
        data.insert(
            "signed_date".to_string(),
            serde_json::json!(contract
                .signed_date
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_default()),
        );
        data.insert(
            "effective_date".to_string(),
            serde_json::json!(contract
                .effective_date
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_default()),
        );
        data.insert(
            "expiry_date".to_string(),
            serde_json::json!(contract
                .expiry_date
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_default()),
        );
        data.insert(
            "payment_terms".to_string(),
            serde_json::json!(contract.payment_terms.unwrap_or_default()),
        );
        data.insert(
            "payment_method".to_string(),
            serde_json::json!(contract.payment_method.unwrap_or_default()),
        );
        data.insert(
            "delivery_date".to_string(),
            serde_json::json!(contract
                .delivery_date
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_default()),
        );
        data.insert(
            "delivery_location".to_string(),
            serde_json::json!(contract.delivery_location.unwrap_or_default()),
        );
        data.insert("status".to_string(), serde_json::json!(contract.status));

        Ok(PrintData {
            template: "sales_contract".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 采购订单打印数据：订单主表 + 供应商 + 仓库 + 明细项（含产品）
    async fn get_purchase_order_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        let (order, supplier, warehouse, items, product_map) =
            Self::fetch_purchase_order_relations(&*self.db, id).await?;
        let data =
            Self::build_purchase_order_main_data(&order, supplier.as_ref(), warehouse.as_ref());
        let item_list = Self::build_purchase_order_item_list(items, &product_map);
        Ok(PrintData {
            template: "purchase_order".to_string(),
            data,
            items: item_list,
        })
    }

    /// 采购订单打印：批量查询订单 + 供应商 + 仓库 + 明细 + 产品
    async fn fetch_purchase_order_relations(
        db: &DatabaseConnection,
        id: i32,
    ) -> Result<
        (
            crate::models::purchase_order::Model,
            Option<crate::models::supplier::Model>,
            Option<crate::models::warehouse::Model>,
            Vec<crate::models::purchase_order_item::Model>,
            HashMap<i32, crate::models::product::Model>,
        ),
        AppError,
    > {
        use crate::models::{product, purchase_order, purchase_order_item, supplier, warehouse};
        let order = purchase_order::Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购订单 {} 未找到", id)))?;
        let supplier = supplier::Entity::find_by_id(order.supplier_id)
            .one(db)
            .await?;
        let warehouse = warehouse::Entity::find_by_id(order.warehouse_id)
            .one(db)
            .await?;
        let items = purchase_order_item::Entity::find()
            .filter(purchase_order_item::Column::OrderId.eq(id))
            .order_by(purchase_order_item::Column::LineNo, Order::Asc)
            .all(db)
            .await?;
        let product_ids: Vec<i32> = items.iter().map(|i| i.product_id).collect();
        let products = product::Entity::find()
            .filter(product::Column::Id.is_in(product_ids))
            .all(db)
            .await?;
        let product_map: HashMap<i32, product::Model> =
            products.into_iter().map(|p| (p.id, p)).collect();
        Ok((order, supplier, warehouse, items, product_map))
    }

    /// 采购订单打印：构造主表 data
    fn build_purchase_order_main_data(
        order: &crate::models::purchase_order::Model,
        supplier: Option<&crate::models::supplier::Model>,
        warehouse: Option<&crate::models::warehouse::Model>,
    ) -> HashMap<String, serde_json::Value> {
        let mut data = HashMap::new();
        data.insert(
            "order_no".to_string(),
            serde_json::json!(order.order_no.clone()),
        );
        data.insert(
            "supplier_name".to_string(),
            serde_json::json!(supplier
                .map(|s| s.supplier_name.clone())
                .unwrap_or_default()),
        );
        data.insert(
            "order_date".to_string(),
            serde_json::json!(order.order_date.format("%Y-%m-%d").to_string()),
        );
        data.insert(
            "expected_delivery_date".to_string(),
            serde_json::json!(order
                .expected_delivery_date
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_default()),
        );
        data.insert(
            "actual_delivery_date".to_string(),
            serde_json::json!(order
                .actual_delivery_date
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_default()),
        );
        data.insert(
            "warehouse_name".to_string(),
            serde_json::json!(warehouse.map(|w| w.name.clone()).unwrap_or_default()),
        );
        data.insert(
            "currency".to_string(),
            serde_json::json!(order.currency.clone()),
        );
        data.insert(
            "total_amount".to_string(),
            serde_json::json!(order.total_amount.to_string()),
        );
        data.insert(
            "total_quantity".to_string(),
            serde_json::json!(order.total_quantity.to_string()),
        );
        data.insert(
            "notes".to_string(),
            serde_json::json!(order.notes.clone().unwrap_or_default()),
        );
        data
    }

    /// 采购订单打印：构造明细行列表
    fn build_purchase_order_item_list(
        items: Vec<crate::models::purchase_order_item::Model>,
        product_map: &HashMap<i32, crate::models::product::Model>,
    ) -> Vec<HashMap<String, serde_json::Value>> {
        let mut item_list = Vec::with_capacity(items.len());
        for item in items {
            let product = product_map.get(&item.product_id);
            let mut row = HashMap::new();
            row.insert(
                "line_no".to_string(),
                serde_json::json!(item.line_no.to_string()),
            );
            row.insert(
                "product_code".to_string(),
                serde_json::json!(product.map(|p| p.code.clone()).unwrap_or_default()),
            );
            row.insert(
                "product_name".to_string(),
                serde_json::json!(product.map(|p| p.name.clone()).unwrap_or_default()),
            );
            row.insert(
                "quantity".to_string(),
                serde_json::json!(item.quantity.to_string()),
            );
            row.insert(
                "unit_price".to_string(),
                serde_json::json!(item.unit_price.to_string()),
            );
            row.insert(
                "tax_percent".to_string(),
                serde_json::json!(item.tax_percent.to_string()),
            );
            row.insert(
                "total_amount".to_string(),
                serde_json::json!(item.total_amount.to_string()),
            );
            row.insert(
                "received_quantity".to_string(),
                serde_json::json!(item.received_quantity.to_string()),
            );
            item_list.push(row);
        }
        item_list
    }

    /// 采购收货单打印数据：收货主表 + 供应商 + 仓库 + 明细项
    async fn get_purchase_receipt_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::{purchase_receipt, purchase_receipt_item, supplier, warehouse};

        let receipt = purchase_receipt::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购收货单 {} 未找到", id)))?;

        let supplier = supplier::Entity::find_by_id(receipt.supplier_id)
            .one(&*self.db)
            .await?;
        let warehouse = warehouse::Entity::find_by_id(receipt.warehouse_id)
            .one(&*self.db)
            .await?;

        let items = purchase_receipt_item::Entity::find()
            .filter(purchase_receipt_item::Column::ReceiptId.eq(id))
            .order_by(purchase_receipt_item::Column::LineNo, Order::Asc)
            .all(&*self.db)
            .await?;

        let mut data = HashMap::new();
        data.insert(
            "receipt_no".to_string(),
            serde_json::json!(receipt.receipt_no),
        );
        data.insert(
            "supplier_name".to_string(),
            serde_json::json!(supplier
                .as_ref()
                .map(|s| s.supplier_name.clone())
                .unwrap_or_default()),
        );
        data.insert(
            "warehouse_name".to_string(),
            serde_json::json!(warehouse
                .as_ref()
                .map(|w| w.name.clone())
                .unwrap_or_default()),
        );
        data.insert(
            "receipt_date".to_string(),
            serde_json::json!(receipt.receipt_date.format("%Y-%m-%d").to_string()),
        );
        data.insert(
            "inspection_status".to_string(),
            serde_json::json!(receipt.inspection_status),
        );
        data.insert(
            "receipt_status".to_string(),
            serde_json::json!(receipt.receipt_status),
        );
        data.insert(
            "total_quantity".to_string(),
            serde_json::json!(receipt.total_quantity.to_string()),
        );
        data.insert(
            "total_amount".to_string(),
            serde_json::json!(receipt.total_amount.to_string()),
        );
        data.insert(
            "notes".to_string(),
            serde_json::json!(receipt.notes.unwrap_or_default()),
        );

        let mut item_list = Vec::with_capacity(items.len());
        for item in items {
            let mut row = HashMap::new();
            row.insert(
                "line_no".to_string(),
                serde_json::json!(item.line_no.to_string()),
            );
            row.insert(
                "material_code".to_string(),
                serde_json::json!(item.material_code),
            );
            row.insert(
                "material_name".to_string(),
                serde_json::json!(item.material_name),
            );
            row.insert(
                "batch_no".to_string(),
                serde_json::json!(item.batch_no.unwrap_or_default()),
            );
            row.insert(
                "color_code".to_string(),
                serde_json::json!(item.color_code.unwrap_or_default()),
            );
            row.insert(
                "lot_no".to_string(),
                serde_json::json!(item.lot_no.unwrap_or_default()),
            );
            row.insert(
                "quantity".to_string(),
                serde_json::json!(item.quantity.to_string()),
            );
            row.insert(
                "unit_master".to_string(),
                serde_json::json!(item.unit_master),
            );
            row.insert(
                "unit_price".to_string(),
                serde_json::json!(item.unit_price.map(|p| p.to_string()).unwrap_or_default()),
            );
            row.insert(
                "amount".to_string(),
                serde_json::json!(item.amount.map(|a| a.to_string()).unwrap_or_default()),
            );
            item_list.push(row);
        }

        Ok(PrintData {
            template: "purchase_receipt".to_string(),
            data,
            items: item_list,
        })
    }

    /// 库存调拨单打印数据：调拨主表 + 调出/调入仓库 + 明细项（含产品）
    async fn get_inventory_transfer_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        let ctx = self.fetch_transfer_print_context(id).await?;
        let data = Self::build_transfer_main_data(&ctx);
        let item_list = Self::build_transfer_item_list(&ctx);
        Ok(PrintData {
            template: "inventory_transfer".to_string(),
            data,
            items: item_list,
        })
    }

    /// 查询调拨单所有关联数据：主表、调出/调入仓库、明细、产品映射
    async fn fetch_transfer_print_context(
        &self,
        id: i32,
    ) -> Result<TransferPrintContext, AppError> {
        use crate::models::{inventory_transfer, inventory_transfer_item, product, warehouse};

        let transfer = inventory_transfer::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("库存调拨单 {} 未找到", id)))?;

        let from_warehouse = warehouse::Entity::find_by_id(transfer.from_warehouse_id)
            .one(&*self.db)
            .await?;
        let to_warehouse = warehouse::Entity::find_by_id(transfer.to_warehouse_id)
            .one(&*self.db)
            .await?;

        let items = inventory_transfer_item::Entity::find()
            .filter(inventory_transfer_item::Column::TransferId.eq(id))
            .order_by(inventory_transfer_item::Column::Id, Order::Asc)
            .all(&*self.db)
            .await?;

        let product_ids: Vec<i32> = items.iter().map(|i| i.product_id).collect();
        let products = product::Entity::find()
            .filter(product::Column::Id.is_in(product_ids))
            .all(&*self.db)
            .await?;
        let product_map: HashMap<i32, product::Model> =
            products.into_iter().map(|p| (p.id, p)).collect();

        Ok(TransferPrintContext {
            transfer,
            from_warehouse,
            to_warehouse,
            items,
            product_map,
        })
    }

    /// 构建调拨单主表数据 HashMap
    fn build_transfer_main_data(ctx: &TransferPrintContext) -> HashMap<String, serde_json::Value> {
        let mut data = HashMap::new();
        data.insert(
            "transfer_no".to_string(),
            serde_json::json!(ctx.transfer.transfer_no.clone()),
        );
        data.insert(
            "from_warehouse_name".to_string(),
            serde_json::json!(ctx
                .from_warehouse
                .as_ref()
                .map(|w| w.name.clone())
                .unwrap_or_default()),
        );
        data.insert(
            "to_warehouse_name".to_string(),
            serde_json::json!(ctx
                .to_warehouse
                .as_ref()
                .map(|w| w.name.clone())
                .unwrap_or_default()),
        );
        data.insert(
            "transfer_date".to_string(),
            serde_json::json!(ctx
                .transfer
                .transfer_date
                .format("%Y-%m-%d %H:%M")
                .to_string()),
        );
        data.insert(
            "status".to_string(),
            serde_json::json!(ctx.transfer.status.clone()),
        );
        data.insert(
            "total_quantity".to_string(),
            serde_json::json!(ctx.transfer.total_quantity.to_string()),
        );
        data.insert(
            "notes".to_string(),
            serde_json::json!(ctx.transfer.notes.clone().unwrap_or_default()),
        );
        data
    }

    /// 构建调拨单明细项列表
    fn build_transfer_item_list(
        ctx: &TransferPrintContext,
    ) -> Vec<HashMap<String, serde_json::Value>> {
        let mut item_list = Vec::with_capacity(ctx.items.len());
        for (i, item) in ctx.items.iter().enumerate() {
            let product = ctx.product_map.get(&item.product_id);
            let mut row = HashMap::new();
            row.insert(
                "line_no".to_string(),
                serde_json::json!((i + 1).to_string()),
            );
            row.insert(
                "product_code".to_string(),
                serde_json::json!(product.map(|p| p.code.clone()).unwrap_or_default()),
            );
            row.insert(
                "product_name".to_string(),
                serde_json::json!(product.map(|p| p.name.clone()).unwrap_or_default()),
            );
            row.insert(
                "color_no".to_string(),
                serde_json::json!(item.color_no.clone()),
            );
            row.insert(
                "dye_lot_no".to_string(),
                serde_json::json!(item.dye_lot_no.clone()),
            );
            row.insert(
                "batch_no".to_string(),
                serde_json::json!(item.batch_no.clone()),
            );
            row.insert(
                "quantity".to_string(),
                serde_json::json!(item.quantity.to_string()),
            );
            row.insert(
                "shipped_quantity".to_string(),
                serde_json::json!(item.shipped_quantity.to_string()),
            );
            row.insert(
                "received_quantity".to_string(),
                serde_json::json!(item.received_quantity.to_string()),
            );
            row.insert(
                "unit_cost".to_string(),
                serde_json::json!(item.unit_cost.map(|c| c.to_string()).unwrap_or_default()),
            );
            item_list.push(row);
        }
        item_list
    }

    /// 会计凭证打印数据：凭证主表 + 分录明细
    async fn get_voucher_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::{voucher, voucher_item};

        let voucher = voucher::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("凭证 {} 未找到", id)))?;

        let items = voucher_item::Entity::find()
            .filter(voucher_item::Column::VoucherId.eq(id))
            .order_by(voucher_item::Column::LineNo, Order::Asc)
            .all(&*self.db)
            .await?;

        let mut data = HashMap::new();
        data.insert(
            "voucher_no".to_string(),
            serde_json::json!(voucher.voucher_no),
        );
        data.insert(
            "voucher_type".to_string(),
            serde_json::json!(voucher.voucher_type),
        );
        data.insert(
            "voucher_date".to_string(),
            serde_json::json!(voucher.voucher_date.format("%Y-%m-%d").to_string()),
        );
        data.insert(
            "source_module".to_string(),
            serde_json::json!(voucher.source_module.unwrap_or_default()),
        );
        data.insert(
            "source_bill_no".to_string(),
            serde_json::json!(voucher.source_bill_no.unwrap_or_default()),
        );
        data.insert("status".to_string(), serde_json::json!(voucher.status));
        data.insert(
            "workshop".to_string(),
            serde_json::json!(voucher.workshop.unwrap_or_default()),
        );

        let mut total_debit = rust_decimal::Decimal::ZERO;
        let mut total_credit = rust_decimal::Decimal::ZERO;
        let mut item_list = Vec::with_capacity(items.len());
        for item in items {
            total_debit += item.debit;
            total_credit += item.credit;
            let mut row = HashMap::new();
            row.insert(
                "line_no".to_string(),
                serde_json::json!(item.line_no.to_string()),
            );
            row.insert(
                "subject_code".to_string(),
                serde_json::json!(item.subject_code),
            );
            row.insert(
                "subject_name".to_string(),
                serde_json::json!(item.subject_name),
            );
            row.insert(
                "debit".to_string(),
                serde_json::json!(item.debit.to_string()),
            );
            row.insert(
                "credit".to_string(),
                serde_json::json!(item.credit.to_string()),
            );
            row.insert(
                "summary".to_string(),
                serde_json::json!(item.summary.unwrap_or_default()),
            );
            item_list.push(row);
        }
        data.insert(
            "total_debit".to_string(),
            serde_json::json!(total_debit.to_string()),
        );
        data.insert(
            "total_credit".to_string(),
            serde_json::json!(total_credit.to_string()),
        );

        Ok(PrintData {
            template: "voucher".to_string(),
            data,
            items: item_list,
        })
    }

    /// V15 P1 batch-08 缺陷 8：生成 docx 字节流（规则 3 强制要求合同/发票/报表支持 .docx）
    /// 将 PrintData 转为 Word 文档（标题 + 主表键值对 + 明细表格）。


    /// 生产流转卡打印数据
    async fn get_production_flow_card_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::production_flow_card;

        let record = production_flow_card::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("生产流转卡 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("card_no".to_string(), serde_json::json!(record.card_no.clone()));
        data.insert("barcode".to_string(), serde_json::json!(record.barcode.clone()));
        data.insert("production_order_id".to_string(), serde_json::json!(record.production_order_id));
        data.insert("dye_lot_no".to_string(), serde_json::json!(record.dye_lot_no.clone()));
        data.insert("color_no".to_string(), serde_json::json!(record.color_no.clone()));
        data.insert("dyeing_requirements".to_string(), serde_json::json!(record.dyeing_requirements.clone().unwrap_or_default()));
        data.insert("planned_fabric_weight".to_string(), serde_json::json!(record.planned_fabric_weight.map(|v| v.to_string()).unwrap_or_default()));
        data.insert("actual_fabric_weight".to_string(), serde_json::json!(record.actual_fabric_weight.map(|v| v.to_string()).unwrap_or_default()));
        data.insert("current_step_seq".to_string(), serde_json::json!(record.current_step_seq));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));

        Ok(PrintData {
            template: "production_flow_card".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 验布记录打印数据
    async fn get_fabric_inspection_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::fabric_inspection_record;

        let record = fabric_inspection_record::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("验布记录 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("inspection_no".to_string(), serde_json::json!(record.inspection_no.clone()));
        data.insert("flow_card_id".to_string(), serde_json::json!(record.flow_card_id));
        data.insert("dye_lot_no".to_string(), serde_json::json!(record.dye_lot_no.clone()));
        data.insert("product_name".to_string(), serde_json::json!(record.product_name.clone().unwrap_or_default()));
        data.insert("color_no".to_string(), serde_json::json!(record.color_no.clone()));
        data.insert("inspection_date".to_string(), serde_json::json!(record.inspection_date.format("%Y-%m-%d").to_string()));
        data.insert("inspector_name".to_string(), serde_json::json!(record.inspector_name.clone().unwrap_or_default()));
        data.insert("machine_no".to_string(), serde_json::json!(record.machine_no.clone().unwrap_or_default()));
        data.insert("scoring_system".to_string(), serde_json::json!(record.scoring_system.clone()));
        data.insert("inspected_yards".to_string(), serde_json::json!(record.inspected_yards.to_string()));
        data.insert("total_defect_points".to_string(), serde_json::json!(record.total_defect_points));
        data.insert("grade".to_string(), serde_json::json!(record.grade.clone().unwrap_or_default()));
        data.insert("abc_grade".to_string(), serde_json::json!(record.abc_grade.clone().unwrap_or_default()));
        data.insert("total_rolls".to_string(), serde_json::json!(record.total_rolls));

        Ok(PrintData {
            template: "fabric_inspection".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 染色批次卡打印数据
    async fn get_dye_batch_card_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::dye_batch;

        let record = dye_batch::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("染色批次卡 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("batch_no".to_string(), serde_json::json!(record.batch_no.clone()));
        data.insert("dye_lot_no".to_string(), serde_json::json!(record.dye_lot_no.clone()));
        data.insert("color_no".to_string(), serde_json::json!(record.color_no.clone()));
        data.insert("planned_quantity".to_string(), serde_json::json!(record.planned_quantity.map(|v| v.to_string()).unwrap_or_default()));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));
        data.insert("started_at".to_string(), serde_json::json!(record.started_at.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()));
        data.insert("completed_at".to_string(), serde_json::json!(record.completed_at.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()));

        Ok(PrintData {
            template: "dye_batch_card".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 色卡发放单打印数据
    async fn get_color_card_issue_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::color_card_issue;

        let record = color_card_issue::Entity::find_by_id(id as i32)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("色卡发放单 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("color_card_id".to_string(), serde_json::json!(record.color_card_id));
        data.insert("customer_id".to_string(), serde_json::json!(record.customer_id));
        data.insert("issue_qty".to_string(), serde_json::json!(record.issue_qty));
        data.insert("issued_at".to_string(), serde_json::json!(record.issued_at.format("%Y-%m-%d").to_string()));
        data.insert("expected_return_date".to_string(), serde_json::json!(record.expected_return_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));
        data.insert("purpose".to_string(), serde_json::json!(record.purpose.clone()));
        data.insert("dye_lot_no".to_string(), serde_json::json!(record.dye_lot_no.clone()));

        Ok(PrintData {
            template: "color_card_issue".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 大货色审批单打印数据
    async fn get_bulk_color_approval_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::bulk_color_approval;

        let record = bulk_color_approval::Entity::find_by_id(id as i32)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("大货色审批单 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("sales_order_id".to_string(), serde_json::json!(record.sales_order_id));
        data.insert("dye_batch_id".to_string(), serde_json::json!(record.dye_batch_id));
        data.insert("customer_id".to_string(), serde_json::json!(record.customer_id));
        data.insert("color_no".to_string(), serde_json::json!(record.color_no.clone()));
        data.insert("dye_lot_no".to_string(), serde_json::json!(record.dye_lot_no.clone()));
        data.insert("batch_no".to_string(), serde_json::json!(record.batch_no.clone()));
        data.insert("sample_type".to_string(), serde_json::json!(record.sample_type.clone()));
        data.insert("approval_status".to_string(), serde_json::json!(record.approval_status.clone()));
        data.insert("approval_date".to_string(), serde_json::json!(record.approval_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()));
        data.insert("customer_feedback".to_string(), serde_json::json!(record.customer_feedback.clone().unwrap_or_default()));
        data.insert("delta_e_value".to_string(), serde_json::json!(record.delta_e_value.map(|v| v.to_string()).unwrap_or_default()));
        data.insert("reject_reason".to_string(), serde_json::json!(record.reject_reason.clone().unwrap_or_default()));

        Ok(PrintData {
            template: "bulk_color_approval".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 打样申请单打印数据
    async fn get_lab_dip_request_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::lab_dip_request;

        use crate::models::lab_dip_sample;

        let record = lab_dip_request::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("打样申请单 {} 未找到", id)))?;

        let items = lab_dip_sample::Entity::find()
            .filter(lab_dip_sample::Column::RequestId.eq(id))
            .order_by(lab_dip_sample::Column::Id, Order::Asc)
            .all(&*self.db)
            .await?;

        let mut data = HashMap::new();
        data.insert("request_no".to_string(), serde_json::json!(record.request_no.clone()));
        data.insert("customer_id".to_string(), serde_json::json!(record.customer_id));
        data.insert("customer_color_no".to_string(), serde_json::json!(record.customer_color_no.clone()));
        data.insert("customer_color_name".to_string(), serde_json::json!(record.customer_color_name.clone()));
        data.insert("fabric_spec".to_string(), serde_json::json!(record.fabric_spec.clone()));
        data.insert("fabric_component".to_string(), serde_json::json!(record.fabric_component.clone()));
        data.insert("light_source".to_string(), serde_json::json!(record.light_source.clone()));
        data.insert("dye_category".to_string(), serde_json::json!(record.dye_category.clone()));
        data.insert("required_date".to_string(), serde_json::json!(record.required_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));

        let mut item_list = Vec::with_capacity(items.len());
        for item in items {
            let mut row = HashMap::new();
            row.insert("version_label".to_string(), serde_json::json!(item.version_label.clone()));
            row.insert("recipe_no".to_string(), serde_json::json!(item.recipe_no.clone()));
            row.insert("formula".to_string(), serde_json::json!(item.formula.clone().unwrap_or_default()));
            row.insert("temperature".to_string(), serde_json::json!(item.temperature.map(|v| v.to_string()).unwrap_or_default()));
            row.insert("time_minutes".to_string(), serde_json::json!(item.time_minutes));
            row.insert("liquor_ratio".to_string(), serde_json::json!(item.liquor_ratio.clone()));
            row.insert("dyeing_method".to_string(), serde_json::json!(item.dyeing_method.clone().unwrap_or_default()));
            row.insert("result".to_string(), serde_json::json!(item.matching_result.clone()));
            item_list.push(row);
        }

        Ok(PrintData {
            template: "lab_dip_request".to_string(),
            data,
            items: item_list,
        })
    }

    /// 生产工单打印数据
    async fn get_production_order_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::production_order;

        let record = production_order::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("生产工单 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("order_no".to_string(), serde_json::json!(record.order_no.clone()));
        data.insert("sales_order_id".to_string(), serde_json::json!(record.sales_order_id));
        data.insert("product_id".to_string(), serde_json::json!(record.product_id));
        data.insert("planned_quantity".to_string(), serde_json::json!(record.planned_quantity.to_string()));
        data.insert("actual_quantity".to_string(), serde_json::json!(record.actual_quantity.map(|v| v.to_string()).unwrap_or_default()));
        data.insert("planned_start_date".to_string(), serde_json::json!(record.planned_start_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()));
        data.insert("planned_end_date".to_string(), serde_json::json!(record.planned_end_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));
        data.insert("priority".to_string(), serde_json::json!(record.priority));
        data.insert("work_center_id".to_string(), serde_json::json!(record.work_center_id));
        data.insert("color_no".to_string(), serde_json::json!(record.color_no.clone()));
        data.insert("dye_lot_no".to_string(), serde_json::json!(record.dye_lot_no.clone()));
        data.insert("batch_no".to_string(), serde_json::json!(record.batch_no.clone()));
        data.insert("order_type".to_string(), serde_json::json!(record.order_type.clone()));

        Ok(PrintData {
            template: "production_order".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 生产配方打印数据
    async fn get_production_recipe_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::production_recipe;

        use crate::models::production_recipe_addition;

        let record = production_recipe::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("生产配方 {} 未找到", id)))?;

        let addition = production_recipe_addition::Entity::find()
            .filter(production_recipe_addition::Column::ProductionRecipeId.eq(id))
            .order_by(production_recipe_addition::Column::Id, Order::Asc)
            .all(&*self.db)
            .await?;

        let mut data = HashMap::new();
        data.insert("recipe_no".to_string(), serde_json::json!(record.recipe_no.clone()));
        data.insert("work_order_id".to_string(), serde_json::json!(record.work_order_id));
        data.insert("dye_batch_id".to_string(), serde_json::json!(record.dye_batch_id));
        data.insert("color_no".to_string(), serde_json::json!(record.color_no.clone()));
        data.insert("fabric_name".to_string(), serde_json::json!(record.fabric_name.clone().unwrap_or_default()));
        data.insert("fabric_spec".to_string(), serde_json::json!(record.fabric_spec.clone()));
        data.insert("fabric_weight".to_string(), serde_json::json!(record.fabric_weight.to_string()));
        data.insert("equipment_no".to_string(), serde_json::json!(record.equipment_no.clone().unwrap_or_default()));

        let mut item_list = Vec::new();
        for addition_record in addition {
            if let Some(detail) = addition_record.addition_detail {
                for item in detail {
                    let mut row = HashMap::new();
                    row.insert("material_code".to_string(), serde_json::json!(item.material_code));
                    row.insert("material_name".to_string(), serde_json::json!(item.material_name));
            row.insert("amount".to_string(), serde_json::json!(item.total_amount.to_string()));
                    row.insert("unit".to_string(), serde_json::json!(item.unit));
                    row.insert("category".to_string(), serde_json::json!(item.category));
                    item_list.push(row);
                }
            }
        }

        Ok(PrintData {
            template: "production_recipe".to_string(),
            data,
            items: item_list,
        })
    }

    /// 质检记录打印数据
    async fn get_quality_inspection_record_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::quality_inspection_record;

        let record = quality_inspection_record::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("质检记录 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("inspection_no".to_string(), serde_json::json!(record.inspection_no.clone()));
        data.insert("inspection_type".to_string(), serde_json::json!(record.inspection_type.clone()));
        data.insert("product_id".to_string(), serde_json::json!(record.product_id));
        data.insert("batch_no".to_string(), serde_json::json!(record.batch_no.clone()));
        data.insert("inspection_date".to_string(), serde_json::json!(record.inspection_date.format("%Y-%m-%d").to_string()));
        data.insert("total_qty".to_string(), serde_json::json!(record.total_qty.to_string()));
        data.insert("inspected_qty".to_string(), serde_json::json!(record.inspected_qty.to_string()));
        data.insert("qualified_qty".to_string(), serde_json::json!(record.qualified_qty.map(|v| v.to_string()).unwrap_or_default()));
        data.insert("unqualified_qty".to_string(), serde_json::json!(record.unqualified_qty.map(|v| v.to_string()).unwrap_or_default()));
        data.insert("qualification_rate".to_string(), serde_json::json!(record.qualification_rate.map(|v| v.to_string()).unwrap_or_default()));
        data.insert("inspection_result".to_string(), serde_json::json!(record.inspection_result.clone()));
        data.insert("grade".to_string(), serde_json::json!(record.grade.clone().unwrap_or_default()));
        data.insert("color_no".to_string(), serde_json::json!(record.color_no.clone()));

        Ok(PrintData {
            template: "quality_inspection_record".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 销售发货单打印数据
    async fn get_sales_delivery_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::sales_delivery;

        use crate::models::sales_delivery_item;

        let record = sales_delivery::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售发货单 {} 未找到", id)))?;

        let items = sales_delivery_item::Entity::find()
            .filter(sales_delivery_item::Column::DeliveryId.eq(id))
            .order_by(sales_delivery_item::Column::Id, Order::Asc)
            .all(&*self.db)
            .await?;

        let mut data = HashMap::new();
        data.insert("delivery_no".to_string(), serde_json::json!(record.delivery_no.clone()));
        data.insert("order_id".to_string(), serde_json::json!(record.order_id));
        data.insert("customer_id".to_string(), serde_json::json!(record.customer_id));
        data.insert("delivery_date".to_string(), serde_json::json!(record.delivery_date.format("%Y-%m-%d").to_string()));
        data.insert("warehouse_id".to_string(), serde_json::json!(record.warehouse_id));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));
        data.insert("total_quantity".to_string(), serde_json::json!(record.total_quantity.to_string()));
        data.insert("total_amount".to_string(), serde_json::json!(record.total_amount.to_string()));

        let mut item_list = Vec::with_capacity(items.len());
        for item in items {
            let mut row = HashMap::new();
            row.insert("product_id".to_string(), serde_json::json!(item.product_id));
            row.insert("batch_no".to_string(), serde_json::json!(item.batch_no.clone()));
            row.insert("color_no".to_string(), serde_json::json!(item.color_no.clone()));
            row.insert("dye_lot_no".to_string(), serde_json::json!(item.dye_lot_no.clone()));
            row.insert("quantity".to_string(), serde_json::json!(item.quantity.to_string()));
            row.insert("unit_price".to_string(), serde_json::json!(item.unit_price.to_string()));
            row.insert("amount".to_string(), serde_json::json!(item.amount.to_string()));
            item_list.push(row);
        }

        Ok(PrintData {
            template: "sales_delivery".to_string(),
            data,
            items: item_list,
        })
    }

    /// 应收账款收款单打印数据
    async fn get_ar_collection_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::ar_collection;

        let record = ar_collection::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("应收账款收款单 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("collection_no".to_string(), serde_json::json!(record.collection_no.clone()));
        data.insert("collection_date".to_string(), serde_json::json!(record.collection_date.format("%Y-%m-%d").to_string()));
        data.insert("customer_id".to_string(), serde_json::json!(record.customer_id));
        data.insert("customer_name".to_string(), serde_json::json!(record.customer_name.clone().unwrap_or_default()));
        data.insert("collection_amount".to_string(), serde_json::json!(record.collection_amount.to_string()));
        data.insert("collection_method".to_string(), serde_json::json!(record.collection_method.clone().unwrap_or_default()));
        data.insert("bank_account".to_string(), serde_json::json!(record.bank_account.clone().unwrap_or_default()));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));

        Ok(PrintData {
            template: "ar_collection".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 应付账款付款单打印数据
    async fn get_ap_payment_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::ap_payment;

        let record = ap_payment::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("应付账款付款单 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("payment_no".to_string(), serde_json::json!(record.payment_no.clone()));
        data.insert("payment_date".to_string(), serde_json::json!(record.payment_date.format("%Y-%m-%d").to_string()));
        data.insert("supplier_id".to_string(), serde_json::json!(record.supplier_id));
        data.insert("payment_method".to_string(), serde_json::json!(record.payment_method.clone()));
        data.insert("payment_amount".to_string(), serde_json::json!(record.payment_amount.to_string()));
        data.insert("payment_status".to_string(), serde_json::json!(record.payment_status.clone()));
        data.insert("currency".to_string(), serde_json::json!(record.currency.clone()));
        data.insert("bank_name".to_string(), serde_json::json!(record.bank_name.clone().unwrap_or_default()));
        data.insert("bank_account".to_string(), serde_json::json!(record.bank_account.clone().unwrap_or_default()));
        data.insert("transaction_no".to_string(), serde_json::json!(record.transaction_no.clone().unwrap_or_default()));

        Ok(PrintData {
            template: "ap_payment".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 应付发票打印数据
    async fn get_ap_invoice_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::ap_invoice;

        let record = ap_invoice::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("应付发票 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("invoice_no".to_string(), serde_json::json!(record.invoice_no.clone()));
        data.insert("supplier_id".to_string(), serde_json::json!(record.supplier_id));
        data.insert("invoice_type".to_string(), serde_json::json!(record.invoice_type.clone()));
        data.insert("invoice_date".to_string(), serde_json::json!(record.invoice_date.format("%Y-%m-%d").to_string()));
        data.insert("due_date".to_string(), serde_json::json!(record.due_date.format("%Y-%m-%d").to_string()));
        data.insert("amount".to_string(), serde_json::json!(record.amount.to_string()));
        data.insert("paid_amount".to_string(), serde_json::json!(record.paid_amount.to_string()));
        data.insert("unpaid_amount".to_string(), serde_json::json!(record.unpaid_amount.to_string()));
        data.insert("invoice_status".to_string(), serde_json::json!(record.invoice_status.clone()));
        data.insert("currency".to_string(), serde_json::json!(record.currency.clone()));
        data.insert("tax_amount".to_string(), serde_json::json!(record.tax_amount.to_string()));

        Ok(PrintData {
            template: "ap_invoice".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 销售报价单打印数据
    async fn get_sales_quotation_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::sales_quotation;

        use crate::models::sales_quotation_item;

        let record = sales_quotation::Entity::find_by_id(id as i32)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售报价单 {} 未找到", id)))?;

        let items = sales_quotation_item::Entity::find()
            .filter(sales_quotation_item::Column::QuotationId.eq(id as i32))
            .order_by(sales_quotation_item::Column::Id, Order::Asc)
            .all(&*self.db)
            .await?;

        let mut data = HashMap::new();
        data.insert("quotation_no".to_string(), serde_json::json!(record.quotation_no.clone()));
        data.insert("customer_id".to_string(), serde_json::json!(record.customer_id));
        data.insert("quotation_date".to_string(), serde_json::json!(record.quotation_date.format("%Y-%m-%d").to_string()));
        data.insert("valid_until".to_string(), serde_json::json!(record.valid_until.format("%Y-%m-%d").to_string()));
        data.insert("currency".to_string(), serde_json::json!(record.currency.clone()));
        data.insert("price_terms".to_string(), serde_json::json!(record.price_terms.clone()));
        data.insert("subtotal".to_string(), serde_json::json!(record.subtotal.to_string()));
        data.insert("tax_amount".to_string(), serde_json::json!(record.tax_amount.to_string()));
        data.insert("total_amount".to_string(), serde_json::json!(record.total_amount.to_string()));

        let mut item_list = Vec::with_capacity(items.len());
        for item in items {
            let mut row = HashMap::new();
            row.insert("product_id".to_string(), serde_json::json!(item.product_id));
            row.insert("color_code".to_string(), serde_json::json!(item.color_code.clone().unwrap_or_default()));
            row.insert("specification".to_string(), serde_json::json!(item.specification.clone().unwrap_or_default()));
            row.insert("unit".to_string(), serde_json::json!(item.unit.clone()));
            row.insert("quantity".to_string(), serde_json::json!(item.quantity.to_string()));
            row.insert("unit_price".to_string(), serde_json::json!(item.unit_price.to_string()));
            row.insert("amount".to_string(), serde_json::json!(item.amount.to_string()));
            item_list.push(row);
        }

        Ok(PrintData {
            template: "sales_quotation".to_string(),
            data,
            items: item_list,
        })
    }

    /// 销售退货单打印数据
    async fn get_sales_return_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::sales_return;

        use crate::models::sales_return_item;

        let record = sales_return::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售退货单 {} 未找到", id)))?;

        let items = sales_return_item::Entity::find()
            .filter(sales_return_item::Column::ReturnId.eq(id))
            .order_by(sales_return_item::Column::Id, Order::Asc)
            .all(&*self.db)
            .await?;

        let mut data = HashMap::new();
        data.insert("return_no".to_string(), serde_json::json!(record.return_no.clone()));
        data.insert("sales_order_id".to_string(), serde_json::json!(record.sales_order_id));
        data.insert("customer_id".to_string(), serde_json::json!(record.customer_id));
        data.insert("return_date".to_string(), serde_json::json!(record.return_date.format("%Y-%m-%d").to_string()));
        data.insert("warehouse_id".to_string(), serde_json::json!(record.warehouse_id));
        data.insert("reason".to_string(), serde_json::json!(record.reason.clone()));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));
        data.insert("total_amount".to_string(), serde_json::json!(record.total_amount.to_string()));

        let mut item_list = Vec::with_capacity(items.len());
        for item in items {
            let mut row = HashMap::new();
            row.insert("product_id".to_string(), serde_json::json!(item.product_id));
            row.insert("quantity".to_string(), serde_json::json!(item.quantity.to_string()));
            row.insert("unit_price".to_string(), serde_json::json!(item.unit_price.to_string()));
            row.insert("amount".to_string(), serde_json::json!(item.amount.to_string()));
            item_list.push(row);
        }

        Ok(PrintData {
            template: "sales_return".to_string(),
            data,
            items: item_list,
        })
    }

    /// 采购退货单打印数据
    async fn get_purchase_return_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::purchase_return;

        use crate::models::purchase_return_item;

        let record = purchase_return::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购退货单 {} 未找到", id)))?;

        let items = purchase_return_item::Entity::find()
            .filter(purchase_return_item::Column::ReturnId.eq(id))
            .order_by(purchase_return_item::Column::Id, Order::Asc)
            .all(&*self.db)
            .await?;

        let mut data = HashMap::new();
        data.insert("return_no".to_string(), serde_json::json!(record.return_no.clone()));
        data.insert("receipt_id".to_string(), serde_json::json!(record.receipt_id));
        data.insert("order_id".to_string(), serde_json::json!(record.order_id));
        data.insert("supplier_id".to_string(), serde_json::json!(record.supplier_id));
        data.insert("return_date".to_string(), serde_json::json!(record.return_date.format("%Y-%m-%d").to_string()));
        data.insert("reason_type".to_string(), serde_json::json!(record.reason_type.clone().unwrap_or_default()));
        data.insert("return_status".to_string(), serde_json::json!(record.return_status.clone()));
        data.insert("total_quantity".to_string(), serde_json::json!(record.total_quantity.map(|v| v.to_string()).unwrap_or_default()));
        data.insert("total_amount".to_string(), serde_json::json!(record.total_amount.map(|v| v.to_string()).unwrap_or_default()));

        let mut item_list = Vec::with_capacity(items.len());
        for item in items {
            let mut row = HashMap::new();
            row.insert("product_id".to_string(), serde_json::json!(item.product_id));
            row.insert("quantity".to_string(), serde_json::json!(item.quantity.to_string()));
            row.insert("unit_price".to_string(), serde_json::json!(item.unit_price.to_string()));
            row.insert("total_amount".to_string(), serde_json::json!(item.total_amount.to_string()));
            row.insert("color_no".to_string(), serde_json::json!(item.color_no.clone()));
            row.insert("dye_lot_no".to_string(), serde_json::json!(item.dye_lot_no.clone()));
            row.insert("batch_no".to_string(), serde_json::json!(item.batch_no.clone()));
            item_list.push(row);
        }

        Ok(PrintData {
            template: "purchase_return".to_string(),
            data,
            items: item_list,
        })
    }

    /// 委外加工单打印数据
    async fn get_outsourcing_order_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::outsourcing_order;

        use crate::models::outsourcing_order_item;

        let record = outsourcing_order::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("委外加工单 {} 未找到", id)))?;

        let items = outsourcing_order_item::Entity::find()
            .filter(outsourcing_order_item::Column::OutsourcingOrderId.eq(id))
            .order_by(outsourcing_order_item::Column::Id, Order::Asc)
            .all(&*self.db)
            .await?;

        let mut data = HashMap::new();
        data.insert("order_no".to_string(), serde_json::json!(record.order_no.clone()));
        data.insert("order_type".to_string(), serde_json::json!(record.order_type.clone()));
        data.insert("supplier_id".to_string(), serde_json::json!(record.supplier_id));
        data.insert("dye_batch_id".to_string(), serde_json::json!(record.dye_batch_id));
        data.insert("color_no".to_string(), serde_json::json!(record.color_no.clone()));
        data.insert("dye_lot_no".to_string(), serde_json::json!(record.dye_lot_no.clone()));
        data.insert("issue_date".to_string(), serde_json::json!(record.issue_date.format("%Y-%m-%d").to_string()));
        data.insert("issue_quantity".to_string(), serde_json::json!(record.issue_quantity.to_string()));
        data.insert("return_quantity".to_string(), serde_json::json!(record.return_quantity.to_string()));
        data.insert("loss_quantity".to_string(), serde_json::json!(record.loss_quantity.to_string()));
        data.insert("material_cost".to_string(), serde_json::json!(record.material_cost.to_string()));
        data.insert("processing_fee".to_string(), serde_json::json!(record.processing_fee.to_string()));

        let mut item_list = Vec::with_capacity(items.len());
        for item in items {
            let mut row = HashMap::new();
            row.insert("product_id".to_string(), serde_json::json!(item.product_id));
            row.insert("color_no".to_string(), serde_json::json!(item.color_no.clone()));
            row.insert("quantity".to_string(), serde_json::json!(item.quantity.to_string()));
            row.insert("unit".to_string(), serde_json::json!(item.unit.clone()));
            row.insert("unit_cost".to_string(), serde_json::json!(item.unit_cost.to_string()));
            row.insert("total_cost".to_string(), serde_json::json!(item.total_cost.to_string()));
            item_list.push(row);
        }

        Ok(PrintData {
            template: "outsourcing_order".to_string(),
            data,
            items: item_list,
        })
    }

    /// 委外收货单打印数据
    async fn get_outsourcing_receipt_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::outsourcing_receipt;

        let record = outsourcing_receipt::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("委外收货单 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("receipt_no".to_string(), serde_json::json!(record.receipt_no.clone()));
        data.insert("outsourcing_order_id".to_string(), serde_json::json!(record.outsourcing_order_id));
        data.insert("receipt_date".to_string(), serde_json::json!(record.receipt_date.format("%Y-%m-%d").to_string()));
        data.insert("color_no".to_string(), serde_json::json!(record.color_no.clone()));
        data.insert("dye_lot_no".to_string(), serde_json::json!(record.dye_lot_no.clone()));
        data.insert("batch_no".to_string(), serde_json::json!(record.batch_no.clone()));
        data.insert("return_quantity".to_string(), serde_json::json!(record.return_quantity.to_string()));
        data.insert("loss_quantity".to_string(), serde_json::json!(record.loss_quantity.to_string()));
        data.insert("loss_rate".to_string(), serde_json::json!(record.loss_rate.map(|v| v.to_string()).unwrap_or_default()));
        data.insert("unit_cost".to_string(), serde_json::json!(record.unit_cost.to_string()));
        data.insert("total_cost".to_string(), serde_json::json!(record.total_cost.to_string()));
        data.insert("quality_status".to_string(), serde_json::json!(record.quality_status.clone().unwrap_or_default()));
        data.insert("grade".to_string(), serde_json::json!(record.grade.clone().unwrap_or_default()));

        Ok(PrintData {
            template: "outsourcing_receipt".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 物流运单打印数据
    async fn get_logistics_waybill_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::logistics_waybill;

        let record = logistics_waybill::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("物流运单 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("order_id".to_string(), serde_json::json!(record.order_id));
        data.insert("logistics_company".to_string(), serde_json::json!(record.logistics_company.clone()));
        data.insert("tracking_number".to_string(), serde_json::json!(record.tracking_number.clone()));
        data.insert("driver_name".to_string(), serde_json::json!(record.driver_name.clone().unwrap_or_default()));
        data.insert("freight_fee".to_string(), serde_json::json!(record.freight_fee.map(|v| v.to_string()).unwrap_or_default()));
        data.insert("total_weight".to_string(), serde_json::json!(record.total_weight.map(|v| v.to_string()).unwrap_or_default()));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));
        data.insert("expected_arrival".to_string(), serde_json::json!(record.expected_arrival.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()));
        data.insert("actual_arrival".to_string(), serde_json::json!(record.actual_arrival.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()));

        Ok(PrintData {
            template: "logistics_waybill".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 原产地证明打印数据
    async fn get_certificate_of_origin_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::certificate_of_origin;

        let record = certificate_of_origin::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("原产地证明 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("certificate_no".to_string(), serde_json::json!(record.certificate_no.clone()));
        data.insert("product_name".to_string(), serde_json::json!(record.product_name.clone()));
        data.insert("hs_code".to_string(), serde_json::json!(record.hs_code.clone()));
        data.insert("origin_country".to_string(), serde_json::json!(record.origin_country.clone()));
        data.insert("destination_country".to_string(), serde_json::json!(record.destination_country.clone()));
        data.insert("quantity".to_string(), serde_json::json!(record.quantity.to_string()));
        data.insert("unit".to_string(), serde_json::json!(record.unit.clone()));
        data.insert("certificate_type".to_string(), serde_json::json!(record.certificate_type.clone()));
        data.insert("issue_date".to_string(), serde_json::json!(record.issue_date.format("%Y-%m-%d").to_string()));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));

        Ok(PrintData {
            template: "certificate_of_origin".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 出口报关单打印数据
    async fn get_export_customs_declaration_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::export_customs_declaration;

        let record = export_customs_declaration::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("出口报关单 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("declaration_no".to_string(), serde_json::json!(record.declaration_no.clone()));
        data.insert("sales_order_id".to_string(), serde_json::json!(record.sales_order_id));
        data.insert("export_date".to_string(), serde_json::json!(record.export_date.format("%Y-%m-%d").to_string()));
        data.insert("destination_country".to_string(), serde_json::json!(record.destination_country.clone().unwrap_or_default()));
        data.insert("total_amount".to_string(), serde_json::json!(record.total_amount.to_string()));
        data.insert("customs_code".to_string(), serde_json::json!(record.customs_code.clone().unwrap_or_default()));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));

        Ok(PrintData {
            template: "export_customs_declaration".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 固废处置单打印数据
    async fn get_solid_waste_disposal_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::solid_waste_disposal_record;

        let record = solid_waste_disposal_record::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("固废处置单 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("manifest_no".to_string(), serde_json::json!(record.manifest_no.clone()));
        data.insert("waste_type".to_string(), serde_json::json!(record.waste_type.clone()));
        data.insert("waste_category".to_string(), serde_json::json!(record.waste_category.clone()));
        data.insert("waste_amount".to_string(), serde_json::json!(record.waste_amount.to_string()));
        data.insert("waste_unit".to_string(), serde_json::json!(record.waste_unit.clone()));
        data.insert("generation_date".to_string(), serde_json::json!(record.generation_date.format("%Y-%m-%d").to_string()));
        data.insert("disposal_date".to_string(), serde_json::json!(record.disposal_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()));
        data.insert("disposal_method".to_string(), serde_json::json!(record.disposal_method.clone()));
        data.insert("disposal_vendor_name".to_string(), serde_json::json!(record.disposal_vendor_name.clone().unwrap_or_default()));
        data.insert("transport_license_no".to_string(), serde_json::json!(record.transport_license_no.clone().unwrap_or_default()));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));

        Ok(PrintData {
            template: "solid_waste_disposal".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 不合格品处理单打印数据
    async fn get_unqualified_product_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::unqualified_product;

        let record = unqualified_product::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("不合格品处理单 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("unqualified_no".to_string(), serde_json::json!(record.unqualified_no.clone()));
        data.insert("product_id".to_string(), serde_json::json!(record.product_id));
        data.insert("batch_no".to_string(), serde_json::json!(record.batch_no.clone()));
        data.insert("unqualified_qty".to_string(), serde_json::json!(record.unqualified_qty.to_string()));
        data.insert("unqualified_reason".to_string(), serde_json::json!(record.unqualified_reason.clone()));
        data.insert("handling_method".to_string(), serde_json::json!(record.handling_method.clone()));
        data.insert("handling_status".to_string(), serde_json::json!(record.handling_status.clone()));
        data.insert("grade".to_string(), serde_json::json!(record.grade.clone().unwrap_or_default()));
        data.insert("handling_result".to_string(), serde_json::json!(record.handling_result.clone().unwrap_or_default()));

        Ok(PrintData {
            template: "unqualified_product".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 化工料领用单打印数据
    async fn get_chemical_requisition_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::chemical_requisition;

        let record = chemical_requisition::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("化工料领用单 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("requisition_no".to_string(), serde_json::json!(record.requisition_no.clone()));
        data.insert("requisition_type".to_string(), serde_json::json!(record.requisition_type.clone()));
        data.insert("department_id".to_string(), serde_json::json!(record.department_id));
        data.insert("requisition_date".to_string(), serde_json::json!(record.requisition_date.format("%Y-%m-%d").to_string()));
        data.insert("required_date".to_string(), serde_json::json!(record.required_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()));
        data.insert("dye_batch_id".to_string(), serde_json::json!(record.dye_batch_id));
        data.insert("production_order_id".to_string(), serde_json::json!(record.production_order_id));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));
        data.insert("total_amount".to_string(), serde_json::json!(record.total_amount.to_string()));

        Ok(PrintData {
            template: "chemical_requisition".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 出口检验单打印数据
    async fn get_export_inspection_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::export_inspection;

        let record = export_inspection::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("出口检验单 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("inspection_no".to_string(), serde_json::json!(record.inspection_no.clone()));
        data.insert("sales_order_id".to_string(), serde_json::json!(record.sales_order_id));
        data.insert("product_name".to_string(), serde_json::json!(record.product_name.clone()));
        data.insert("hs_code".to_string(), serde_json::json!(record.hs_code.clone()));
        data.insert("inspection_type".to_string(), serde_json::json!(record.inspection_type.clone()));
        data.insert("inspection_agency".to_string(), serde_json::json!(record.inspection_agency.clone()));
        data.insert("inspection_date".to_string(), serde_json::json!(record.inspection_date.format("%Y-%m-%d").to_string()));
        data.insert("result".to_string(), serde_json::json!(record.result.clone()));
        data.insert("certificate_no".to_string(), serde_json::json!(record.certificate_no.clone().unwrap_or_default()));

        Ok(PrintData {
            template: "export_inspection".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 付款申请单打印数据
    async fn get_ap_payment_request_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::ap_payment_request;

        use crate::models::ap_payment_request_item;

        let record = ap_payment_request::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("付款申请单 {} 未找到", id)))?;

        let items = ap_payment_request_item::Entity::find()
            .filter(ap_payment_request_item::Column::RequestId.eq(id))
            .order_by(ap_payment_request_item::Column::Id, Order::Asc)
            .all(&*self.db)
            .await?;

        let mut data = HashMap::new();
        data.insert("request_no".to_string(), serde_json::json!(record.request_no.clone()));
        data.insert("request_date".to_string(), serde_json::json!(record.request_date.format("%Y-%m-%d").to_string()));
        data.insert("supplier_id".to_string(), serde_json::json!(record.supplier_id));
        data.insert("payment_type".to_string(), serde_json::json!(record.payment_type.clone()));
        data.insert("payment_method".to_string(), serde_json::json!(record.payment_method.clone()));
        data.insert("request_amount".to_string(), serde_json::json!(record.request_amount.to_string()));
        data.insert("currency".to_string(), serde_json::json!(record.currency.clone()));
        data.insert("expected_payment_date".to_string(), serde_json::json!(record.expected_payment_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()));
        data.insert("bank_name".to_string(), serde_json::json!(record.bank_name.clone().unwrap_or_default()));

        let mut item_list = Vec::with_capacity(items.len());
        for item in items {
            let mut row = HashMap::new();
            row.insert("invoice_id".to_string(), serde_json::json!(item.invoice_id));
            row.insert("apply_amount".to_string(), serde_json::json!(item.apply_amount.to_string()));
            item_list.push(row);
        }

        Ok(PrintData {
            template: "ap_payment_request".to_string(),
            data,
            items: item_list,
        })
    }

    /// 应付对账单打印数据
    async fn get_ap_reconciliation_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::ap_reconciliation;

        let record = ap_reconciliation::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("应付对账单 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("reconciliation_no".to_string(), serde_json::json!(record.reconciliation_no.clone()));
        data.insert("supplier_id".to_string(), serde_json::json!(record.supplier_id));
        data.insert("start_date".to_string(), serde_json::json!(record.start_date.format("%Y-%m-%d").to_string()));
        data.insert("end_date".to_string(), serde_json::json!(record.end_date.format("%Y-%m-%d").to_string()));
        data.insert("opening_balance".to_string(), serde_json::json!(record.opening_balance.to_string()));
        data.insert("total_invoice".to_string(), serde_json::json!(record.total_invoice.to_string()));
        data.insert("total_payment".to_string(), serde_json::json!(record.total_payment.to_string()));
        data.insert("closing_balance".to_string(), serde_json::json!(record.closing_balance.to_string()));
        data.insert("reconciliation_status".to_string(), serde_json::json!(record.reconciliation_status.clone()));

        Ok(PrintData {
            template: "ap_reconciliation".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 采购质检单打印数据
    async fn get_purchase_inspection_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::purchase_inspection;

        use crate::models::purchase_inspection_item;

        let record = purchase_inspection::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购质检单 {} 未找到", id)))?;

        let items = purchase_inspection_item::Entity::find()
            .filter(purchase_inspection_item::Column::InspectionId.eq(id))
            .order_by(purchase_inspection_item::Column::Id, Order::Asc)
            .all(&*self.db)
            .await?;

        let mut data = HashMap::new();
        data.insert("inspection_no".to_string(), serde_json::json!(record.inspection_no.clone()));
        data.insert("receipt_id".to_string(), serde_json::json!(record.receipt_id));
        data.insert("supplier_id".to_string(), serde_json::json!(record.supplier_id));
        data.insert("inspection_date".to_string(), serde_json::json!(record.inspection_date.format("%Y-%m-%d").to_string()));
        data.insert("inspection_type".to_string(), serde_json::json!(record.inspection_type.clone()));
        data.insert("pass_quantity".to_string(), serde_json::json!(record.pass_quantity.map(|v| v.to_string()).unwrap_or_default()));
        data.insert("reject_quantity".to_string(), serde_json::json!(record.reject_quantity.map(|v| v.to_string()).unwrap_or_default()));
        data.insert("inspection_result".to_string(), serde_json::json!(record.inspection_result.clone()));
        data.insert("quality_score".to_string(), serde_json::json!(record.quality_score.map(|v| v.to_string()).unwrap_or_default()));

        let mut item_list = Vec::with_capacity(items.len());
        for item in items {
            let mut row = HashMap::new();
            row.insert("product_id".to_string(), serde_json::json!(item.product_id));
            row.insert("item_name".to_string(), serde_json::json!(item.item_name.clone()));
            row.insert("qualified_quantity".to_string(), serde_json::json!(item.qualified_quantity.to_string()));
            row.insert("unqualified_quantity".to_string(), serde_json::json!(item.unqualified_quantity.to_string()));
            item_list.push(row);
        }

        Ok(PrintData {
            template: "purchase_inspection".to_string(),
            data,
            items: item_list,
        })
    }

    /// 库存调整单打印数据
    async fn get_inventory_adjustment_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::inventory_adjustment;

        use crate::models::inventory_adjustment_item;

        let record = inventory_adjustment::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("库存调整单 {} 未找到", id)))?;

        let items = inventory_adjustment_item::Entity::find()
            .filter(inventory_adjustment_item::Column::AdjustmentId.eq(id))
            .order_by(inventory_adjustment_item::Column::Id, Order::Asc)
            .all(&*self.db)
            .await?;

        let mut data = HashMap::new();
        data.insert("adjustment_no".to_string(), serde_json::json!(record.adjustment_no.clone()));
        data.insert("warehouse_id".to_string(), serde_json::json!(record.warehouse_id));
        data.insert("adjustment_date".to_string(), serde_json::json!(record.adjustment_date.format("%Y-%m-%d").to_string()));
        data.insert("adjustment_type".to_string(), serde_json::json!(record.adjustment_type.clone()));
        data.insert("reason_type".to_string(), serde_json::json!(record.reason_type.clone()));
        data.insert("total_quantity".to_string(), serde_json::json!(record.total_quantity.to_string()));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));

        let mut item_list = Vec::with_capacity(items.len());
        for item in items {
            let mut row = HashMap::new();
            row.insert("stock_id".to_string(), serde_json::json!(item.stock_id));
            row.insert("quantity".to_string(), serde_json::json!(item.quantity.to_string()));
            row.insert("quantity_before".to_string(), serde_json::json!(item.quantity_before.to_string()));
            row.insert("quantity_after".to_string(), serde_json::json!(item.quantity_after.to_string()));
            row.insert("unit_cost".to_string(), serde_json::json!(item.unit_cost.map(|v| v.to_string()).unwrap_or_default()));
            row.insert("amount".to_string(), serde_json::json!(item.amount.map(|v| v.to_string()).unwrap_or_default()));
            item_list.push(row);
        }

        Ok(PrintData {
            template: "inventory_adjustment".to_string(),
            data,
            items: item_list,
        })
    }

    /// 物料清单打印数据
    async fn get_bom_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::bom;

        use crate::models::bom_item;

        let record = bom::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("物料清单 {} 未找到", id)))?;

        let items = bom_item::Entity::find()
            .filter(bom_item::Column::BomId.eq(id))
            .order_by(bom_item::Column::Id, Order::Asc)
            .all(&*self.db)
            .await?;

        let mut data = HashMap::new();
        data.insert("product_id".to_string(), serde_json::json!(record.product_id));
        data.insert("version".to_string(), serde_json::json!(record.version));
        data.insert("is_default".to_string(), serde_json::json!(record.is_default));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));

        let mut item_list = Vec::with_capacity(items.len());
        for item in items {
            let mut row = HashMap::new();
            row.insert("material_id".to_string(), serde_json::json!(item.material_id));
            row.insert("quantity".to_string(), serde_json::json!(item.quantity.to_string()));
            row.insert("unit".to_string(), serde_json::json!(item.unit.clone()));
            row.insert("scrap_rate".to_string(), serde_json::json!(item.scrap_rate.map(|v| v.to_string()).unwrap_or_default()));
            item_list.push(row);
        }

        Ok(PrintData {
            template: "bom".to_string(),
            data,
            items: item_list,
        })
    }

    /// 物料缺料预警打印数据
    async fn get_material_shortage_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::material_shortage;

        let record = material_shortage::Entity::find_by_id(id as i32)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("物料缺料预警 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("alert_no".to_string(), serde_json::json!(record.alert_no.clone()));
        data.insert("material_id".to_string(), serde_json::json!(record.material_id));
        data.insert("material_name".to_string(), serde_json::json!(record.material_name.clone()));
        data.insert("material_code".to_string(), serde_json::json!(record.material_code.clone()));
        data.insert("required_quantity".to_string(), serde_json::json!(record.required_quantity.to_string()));
        data.insert("available_quantity".to_string(), serde_json::json!(record.available_quantity.to_string()));
        data.insert("shortage_quantity".to_string(), serde_json::json!(record.shortage_quantity.to_string()));
        data.insert("deficit_rate".to_string(), serde_json::json!(record.deficit_rate.to_string()));
        data.insert("level".to_string(), serde_json::json!(record.level.clone()));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));
        data.insert("affected_orders_count".to_string(), serde_json::json!(record.affected_orders_count));
        data.insert("unit".to_string(), serde_json::json!(record.unit.clone()));

        Ok(PrintData {
            template: "material_shortage".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 8D质量报告打印数据
    async fn get_quality_8d_report_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::quality_8d_report;

        let record = quality_8d_report::Entity::find_by_id(id as i32)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("8D质量报告 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("quality_issue_id".to_string(), serde_json::json!(record.quality_issue_id));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));
        data.insert("d0_date".to_string(), serde_json::json!(record.d0_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()));
        data.insert("d0_plan".to_string(), serde_json::json!(record.d0_plan.clone().unwrap_or_default()));
        data.insert("d1_team_members".to_string(), serde_json::json!(record.d1_team_members.clone().unwrap_or_default()));
        data.insert("d2_problem_description".to_string(), serde_json::json!(record.d2_problem_description.clone().unwrap_or_default()));
        data.insert("d3_interim_action".to_string(), serde_json::json!(record.d3_interim_action.clone().unwrap_or_default()));
        data.insert("d4_root_cause_summary".to_string(), serde_json::json!(record.d4_root_cause_summary.clone().unwrap_or_default()));
        data.insert("d5_permanent_action".to_string(), serde_json::json!(record.d5_permanent_action.clone().unwrap_or_default()));
        data.insert("d6_verification_result".to_string(), serde_json::json!(record.d6_verification_result.clone().unwrap_or_default()));
        data.insert("d7_prevention_action".to_string(), serde_json::json!(record.d7_prevention_action.clone().unwrap_or_default()));

        Ok(PrintData {
            template: "quality_8d_report".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 劳动合同打印数据
    async fn get_labor_contract_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::labor_contract;

        let record = labor_contract::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("劳动合同 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("worker_id".to_string(), serde_json::json!(record.worker_id));
        data.insert("contract_no".to_string(), serde_json::json!(record.contract_no.clone()));
        data.insert("contract_type".to_string(), serde_json::json!(record.contract_type.clone()));
        data.insert("start_date".to_string(), serde_json::json!(record.start_date.format("%Y-%m-%d").to_string()));
        data.insert("end_date".to_string(), serde_json::json!(record.end_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()));
        data.insert("probation_salary".to_string(), serde_json::json!(record.probation_salary.to_string()));
        data.insert("regular_salary".to_string(), serde_json::json!(record.regular_salary.to_string()));
        data.insert("position".to_string(), serde_json::json!(record.position.clone().unwrap_or_default()));
        data.insert("department".to_string(), serde_json::json!(record.department.clone().unwrap_or_default()));
        data.insert("working_hours_system".to_string(), serde_json::json!(record.working_hours_system.clone()));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));

        Ok(PrintData {
            template: "labor_contract".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 工资记录打印数据
    async fn get_wage_record_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::wage_record;

        use crate::models::wage_record_detail;

        let record = wage_record::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("工资记录 {} 未找到", id)))?;

        let items = wage_record_detail::Entity::find()
            .filter(wage_record_detail::Column::WageRecordId.eq(id))
            .order_by(wage_record_detail::Column::Id, Order::Asc)
            .all(&*self.db)
            .await?;

        let mut data = HashMap::new();
        data.insert("record_no".to_string(), serde_json::json!(record.record_no.clone()));
        data.insert("period_start".to_string(), serde_json::json!(record.period_start.format("%Y-%m-%d").to_string()));
        data.insert("period_end".to_string(), serde_json::json!(record.period_end.format("%Y-%m-%d").to_string()));
        data.insert("workshop".to_string(), serde_json::json!(record.workshop.clone().unwrap_or_default()));
        data.insert("total_workers".to_string(), serde_json::json!(record.total_workers));
        data.insert("total_qualified_quantity".to_string(), serde_json::json!(record.total_qualified_quantity.to_string()));
        data.insert("total_duration_minutes".to_string(), serde_json::json!(record.total_duration_minutes));
        data.insert("total_amount".to_string(), serde_json::json!(record.total_amount.to_string()));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));

        let mut item_list = Vec::with_capacity(items.len());
        for item in items {
            let mut row = HashMap::new();
            row.insert("worker_name".to_string(), serde_json::json!(item.worker_name.clone().unwrap_or_default()));
            row.insert("equipment_name".to_string(), serde_json::json!(item.equipment_name.clone().unwrap_or_default()));
            row.insert("wage_type".to_string(), serde_json::json!(item.wage_type.clone()));
            row.insert("grade".to_string(), serde_json::json!(item.grade.clone()));
            row.insert("actual_quantity".to_string(), serde_json::json!(item.actual_quantity.to_string()));
            row.insert("qualified_quantity".to_string(), serde_json::json!(item.qualified_quantity.to_string()));
            row.insert("piece_price".to_string(), serde_json::json!(item.piece_price.to_string()));
            row.insert("duration_minutes".to_string(), serde_json::json!(item.duration_minutes));
            item_list.push(row);
        }

        Ok(PrintData {
            template: "wage_record".to_string(),
            data,
            items: item_list,
        })
    }

    /// 能耗记录打印数据
    async fn get_energy_consumption_record_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::energy_consumption_record;

        let record = energy_consumption_record::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("能耗记录 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("record_no".to_string(), serde_json::json!(record.record_no.clone()));
        data.insert("meter_type".to_string(), serde_json::json!(record.meter_type.clone()));
        data.insert("workshop".to_string(), serde_json::json!(record.workshop.clone().unwrap_or_default()));
        data.insert("unit".to_string(), serde_json::json!(record.unit.clone()));
        data.insert("previous_reading".to_string(), serde_json::json!(record.previous_reading.to_string()));
        data.insert("current_reading".to_string(), serde_json::json!(record.current_reading.to_string()));
        data.insert("consumption".to_string(), serde_json::json!(record.consumption.to_string()));
        data.insert("unit_price".to_string(), serde_json::json!(record.unit_price.to_string()));
        data.insert("total_cost".to_string(), serde_json::json!(record.total_cost.to_string()));
        data.insert("period_start".to_string(), serde_json::json!(record.period_start.format("%Y-%m-%d").to_string()));
        data.insert("period_end".to_string(), serde_json::json!(record.period_end.format("%Y-%m-%d").to_string()));
        data.insert("dye_lot_no".to_string(), serde_json::json!(record.dye_lot_no.clone()));
        data.insert("equipment_name".to_string(), serde_json::json!(record.equipment_name.clone().unwrap_or_default()));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));

        Ok(PrintData {
            template: "energy_consumption_record".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 采购合同打印数据
    async fn get_purchase_contract_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::purchase_contract;

        let record = purchase_contract::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购合同 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("contract_no".to_string(), serde_json::json!(record.contract_no.clone()));
        data.insert("contract_name".to_string(), serde_json::json!(record.contract_name.clone()));
        data.insert("supplier_id".to_string(), serde_json::json!(record.supplier_id));
        data.insert("total_amount".to_string(), serde_json::json!(record.total_amount.map(|v| v.to_string()).unwrap_or_default()));
        data.insert("signed_date".to_string(), serde_json::json!(record.signed_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()));
        data.insert("effective_date".to_string(), serde_json::json!(record.effective_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()));
        data.insert("expiry_date".to_string(), serde_json::json!(record.expiry_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()));
        data.insert("payment_terms".to_string(), serde_json::json!(record.payment_terms.clone().unwrap_or_default()));
        data.insert("delivery_date".to_string(), serde_json::json!(record.delivery_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));

        Ok(PrintData {
            template: "purchase_contract".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 供应商评价记录打印数据
    async fn get_supplier_evaluation_record_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::supplier_evaluation_record;

        let record = supplier_evaluation_record::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("供应商评价记录 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("supplier_id".to_string(), serde_json::json!(record.supplier_id));
        data.insert("evaluation_period".to_string(), serde_json::json!(record.evaluation_period.clone()));
        data.insert("indicator_id".to_string(), serde_json::json!(record.indicator_id));
        data.insert("score".to_string(), serde_json::json!(record.score.to_string()));
        data.insert("weighted_score".to_string(), serde_json::json!(record.weighted_score.map(|v| v.to_string()).unwrap_or_default()));
        data.insert("evaluation_date".to_string(), serde_json::json!(record.evaluation_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()));
        data.insert("remark".to_string(), serde_json::json!(record.remark.clone().unwrap_or_default()));

        Ok(PrintData {
            template: "supplier_evaluation_record".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 安全事故报告打印数据
    async fn get_safety_accident_report_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::safety_accident_report;

        let record = safety_accident_report::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("安全事故报告 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("accident_no".to_string(), serde_json::json!(record.accident_no.clone()));
        data.insert("accident_level".to_string(), serde_json::json!(record.accident_level.clone()));
        data.insert("accident_date".to_string(), serde_json::json!(record.accident_date.format("%Y-%m-%d").to_string()));
        data.insert("location".to_string(), serde_json::json!(record.location.clone().unwrap_or_default()));
        data.insert("description".to_string(), serde_json::json!(record.description.clone()));
        data.insert("casualties".to_string(), serde_json::json!(record.casualties));
        data.insert("direct_loss".to_string(), serde_json::json!(record.direct_loss.map(|v| v.to_string()).unwrap_or_default()));
        data.insert("cause".to_string(), serde_json::json!(record.cause.clone().unwrap_or_default()));
        data.insert("measures".to_string(), serde_json::json!(record.measures.clone().unwrap_or_default()));

        Ok(PrintData {
            template: "safety_accident_report".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 职业病危害监测打印数据
    async fn get_occupational_hazard_monitoring_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::occupational_hazard_monitoring;

        let record = occupational_hazard_monitoring::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("职业病危害监测 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("hazard_type".to_string(), serde_json::json!(record.hazard_type.clone()));
        data.insert("hazard_name".to_string(), serde_json::json!(record.hazard_name.clone()));
        data.insert("monitoring_point".to_string(), serde_json::json!(record.monitoring_point.clone()));
        data.insert("measured_value".to_string(), serde_json::json!(record.measured_value.to_string()));
        data.insert("unit".to_string(), serde_json::json!(record.unit.clone()));
        data.insert("limit_value".to_string(), serde_json::json!(record.limit_value.to_string()));
        data.insert("is_exceeding".to_string(), serde_json::json!(record.is_exceeding));
        data.insert("exceeding_ratio".to_string(), serde_json::json!(record.exceeding_ratio.map(|v| v.to_string()).unwrap_or_default()));
        data.insert("monitoring_date".to_string(), serde_json::json!(record.monitoring_date.format("%Y-%m-%d").to_string()));
        data.insert("monitoring_organization".to_string(), serde_json::json!(record.monitoring_organization.clone().unwrap_or_default()));

        Ok(PrintData {
            template: "occupational_hazard_monitoring".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 排污许可证打印数据
    async fn get_pollution_permit_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::pollution_permit;

        let record = pollution_permit::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("排污许可证 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("permit_no".to_string(), serde_json::json!(record.permit_no.clone()));
        data.insert("permit_type".to_string(), serde_json::json!(record.permit_type.clone()));
        data.insert("permit_category".to_string(), serde_json::json!(record.permit_category.clone().unwrap_or_default()));
        data.insert("issue_date".to_string(), serde_json::json!(record.issue_date.format("%Y-%m-%d").to_string()));
        data.insert("expiry_date".to_string(), serde_json::json!(record.expiry_date.format("%Y-%m-%d").to_string()));
        data.insert("issuing_authority".to_string(), serde_json::json!(record.issuing_authority.clone()));
        data.insert("permitted_capacity".to_string(), serde_json::json!(record.permitted_capacity.map(|v| v.to_string()).unwrap_or_default()));
        data.insert("capacity_unit".to_string(), serde_json::json!(record.capacity_unit.clone().unwrap_or_default()));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));

        Ok(PrintData {
            template: "pollution_permit".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 工艺路线打印数据
    async fn get_process_route_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::process_route;

        let record = process_route::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("工艺路线 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("route_code".to_string(), serde_json::json!(record.route_code.clone()));
        data.insert("route_name".to_string(), serde_json::json!(record.route_name.clone()));
        data.insert("seq".to_string(), serde_json::json!(record.seq));
        data.insert("process_type".to_string(), serde_json::json!(record.process_type.clone()));
        data.insert("default_duration_minutes".to_string(), serde_json::json!(record.default_duration_minutes));
        data.insert("require_scan".to_string(), serde_json::json!(record.require_scan));
        data.insert("is_active".to_string(), serde_json::json!(record.is_active));

        Ok(PrintData {
            template: "process_route".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 外汇核销单打印数据
    async fn get_foreign_exchange_verification_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::foreign_exchange_verification;

        let record = foreign_exchange_verification::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("外汇核销单 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("verification_no".to_string(), serde_json::json!(record.verification_no.clone()));
        data.insert("customs_declaration_id".to_string(), serde_json::json!(record.customs_declaration_id));
        data.insert("sales_order_id".to_string(), serde_json::json!(record.sales_order_id));
        data.insert("verification_date".to_string(), serde_json::json!(record.verification_date.format("%Y-%m-%d").to_string()));
        data.insert("foreign_currency_amount".to_string(), serde_json::json!(record.foreign_currency_amount.to_string()));
        data.insert("rmb_amount".to_string(), serde_json::json!(record.rmb_amount.to_string()));
        data.insert("exchange_rate".to_string(), serde_json::json!(record.exchange_rate.to_string()));
        data.insert("bank_code".to_string(), serde_json::json!(record.bank_code.clone().unwrap_or_default()));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));

        Ok(PrintData {
            template: "foreign_exchange_verification".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 出口退税申报单打印数据
    async fn get_export_refund_declaration_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::export_refund_declaration;

        let record = export_refund_declaration::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("出口退税申报单 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("declaration_no".to_string(), serde_json::json!(record.declaration_no.clone()));
        data.insert("period_year".to_string(), serde_json::json!(record.period_year));
        data.insert("period_month".to_string(), serde_json::json!(record.period_month));
        data.insert("declaration_date".to_string(), serde_json::json!(record.declaration_date.format("%Y-%m-%d").to_string()));
        data.insert("export_sales_amount".to_string(), serde_json::json!(record.export_sales_amount.to_string()));
        data.insert("refundable_vat_amount".to_string(), serde_json::json!(record.refundable_vat_amount.to_string()));
        data.insert("actual_refund_amount".to_string(), serde_json::json!(record.actual_refund_amount.to_string()));
        data.insert("refund_rate".to_string(), serde_json::json!(record.refund_rate.to_string()));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));

        Ok(PrintData {
            template: "export_refund_declaration".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 固定资产卡片打印数据
    async fn get_fixed_asset_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::fixed_asset;

        let record = fixed_asset::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("固定资产卡片 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("asset_no".to_string(), serde_json::json!(record.asset_no.clone()));
        data.insert("asset_name".to_string(), serde_json::json!(record.asset_name.clone()));
        data.insert("asset_category".to_string(), serde_json::json!(record.asset_category.clone().unwrap_or_default()));
        data.insert("specification".to_string(), serde_json::json!(record.specification.clone().unwrap_or_default()));
        data.insert("model".to_string(), serde_json::json!(record.model.clone().unwrap_or_default()));
        data.insert("original_value".to_string(), serde_json::json!(record.original_value.to_string()));
        data.insert("accumulated_depreciation".to_string(), serde_json::json!(record.accumulated_depreciation.to_string()));
        data.insert("net_value".to_string(), serde_json::json!(record.net_value.map(|v| v.to_string()).unwrap_or_default()));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));
        data.insert("purchase_date".to_string(), serde_json::json!(record.purchase_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()));
        data.insert("in_service_date".to_string(), serde_json::json!(record.in_service_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()));

        Ok(PrintData {
            template: "fixed_asset".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 排程结果打印数据
    async fn get_scheduling_result_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::scheduling_result;

        let record = scheduling_result::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("排程结果 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("batch_no".to_string(), serde_json::json!(record.batch_no.clone()));
        data.insert("strategy".to_string(), serde_json::json!(record.strategy.clone()));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));
        data.insert("total_orders".to_string(), serde_json::json!(record.total_orders));
        data.insert("scheduled_orders".to_string(), serde_json::json!(record.scheduled_orders));
        data.insert("unscheduled_orders".to_string(), serde_json::json!(record.unscheduled_orders));
        data.insert("conflict_count".to_string(), serde_json::json!(record.conflict_count));
        data.insert("schedule_start_date".to_string(), serde_json::json!(record.schedule_start_date.format("%Y-%m-%d").to_string()));
        data.insert("schedule_end_date".to_string(), serde_json::json!(record.schedule_end_date.format("%Y-%m-%d").to_string()));

        Ok(PrintData {
            template: "scheduling_result".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 存货跌价准备打印数据
    async fn get_inventory_write_down_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::inventory_write_down;

        let record = inventory_write_down::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("存货跌价准备 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("product_id".to_string(), serde_json::json!(record.product_id));
        data.insert("write_down_type".to_string(), serde_json::json!(record.write_down_type.clone()));
        data.insert("original_cost".to_string(), serde_json::json!(record.original_cost.to_string()));
        data.insert("net_realizable_value".to_string(), serde_json::json!(record.net_realizable_value.to_string()));
        data.insert("write_down_amount".to_string(), serde_json::json!(record.write_down_amount.to_string()));
        data.insert("reason".to_string(), serde_json::json!(record.reason.clone().unwrap_or_default()));
        data.insert("period".to_string(), serde_json::json!(record.period.format("%Y-%m-%d").to_string()));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));

        Ok(PrintData {
            template: "inventory_write_down".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 固定资产盘点单打印数据
    async fn get_fixed_asset_count_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::fixed_asset_count;

        use crate::models::fixed_asset_count_item;

        let record = fixed_asset_count::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("固定资产盘点单 {} 未找到", id)))?;

        let items = fixed_asset_count_item::Entity::find()
            .filter(fixed_asset_count_item::Column::CountId.eq(id))
            .order_by(fixed_asset_count_item::Column::Id, Order::Asc)
            .all(&*self.db)
            .await?;

        let mut data = HashMap::new();
        data.insert("count_no".to_string(), serde_json::json!(record.count_no.clone()));
        data.insert("plan_name".to_string(), serde_json::json!(record.plan_name.clone()));
        data.insert("count_date".to_string(), serde_json::json!(record.count_date.format("%Y-%m-%d").to_string()));
        data.insert("asset_category".to_string(), serde_json::json!(record.asset_category.clone().unwrap_or_default()));
        data.insert("use_location".to_string(), serde_json::json!(record.use_location.clone().unwrap_or_default()));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));
        data.insert("total_items".to_string(), serde_json::json!(record.total_items));
        data.insert("counted_items".to_string(), serde_json::json!(record.counted_items));
        data.insert("surplus_items".to_string(), serde_json::json!(record.surplus_items));
        data.insert("shortage_items".to_string(), serde_json::json!(record.shortage_items));

        let mut item_list = Vec::with_capacity(items.len());
        for item in items {
            let mut row = HashMap::new();
            row.insert("asset_no".to_string(), serde_json::json!(item.asset_no.clone()));
            row.insert("asset_name".to_string(), serde_json::json!(item.asset_name.clone()));
            row.insert("book_original_value".to_string(), serde_json::json!(item.book_original_value.to_string()));
            row.insert("actual_original_value".to_string(), serde_json::json!(item.actual_original_value.map(|v| v.to_string()).unwrap_or_default()));
            row.insert("count_result".to_string(), serde_json::json!(item.count_result.clone().unwrap_or_default()));
            row.insert("variance_type".to_string(), serde_json::json!(item.variance_type.clone().unwrap_or_default()));
            item_list.push(row);
        }

        Ok(PrintData {
            template: "fixed_asset_count".to_string(),
            data,
            items: item_list,
        })
    }

    /// 社保缴纳记录打印数据
    async fn get_social_insurance_record_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::social_insurance_record;

        let record = social_insurance_record::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("社保缴纳记录 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("worker_id".to_string(), serde_json::json!(record.worker_id));
        data.insert("period_year".to_string(), serde_json::json!(record.period_year));
        data.insert("period_month".to_string(), serde_json::json!(record.period_month));
        data.insert("base_amount".to_string(), serde_json::json!(record.base_amount.to_string()));
        data.insert("pension_employer".to_string(), serde_json::json!(record.pension_employer.to_string()));
        data.insert("pension_employee".to_string(), serde_json::json!(record.pension_employee.to_string()));
        data.insert("medical_employer".to_string(), serde_json::json!(record.medical_employer.to_string()));
        data.insert("medical_employee".to_string(), serde_json::json!(record.medical_employee.to_string()));
        data.insert("total_employer".to_string(), serde_json::json!(record.total_employer.to_string()));
        data.insert("total_employee".to_string(), serde_json::json!(record.total_employee.to_string()));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));

        Ok(PrintData {
            template: "social_insurance_record".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 职业健康体检打印数据
    async fn get_occupational_health_exam_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::occupational_health_exam;

        let record = occupational_health_exam::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("职业健康体检 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("worker_id".to_string(), serde_json::json!(record.worker_id));
        data.insert("exam_type".to_string(), serde_json::json!(record.exam_type.clone()));
        data.insert("exam_date".to_string(), serde_json::json!(record.exam_date.format("%Y-%m-%d").to_string()));
        data.insert("next_exam_date".to_string(), serde_json::json!(record.next_exam_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()));
        data.insert("exam_organization".to_string(), serde_json::json!(record.exam_organization.clone().unwrap_or_default()));
        data.insert("exam_result".to_string(), serde_json::json!(record.exam_result.clone()));
        data.insert("contraindications".to_string(), serde_json::json!(record.contraindications.clone().unwrap_or_default()));

        Ok(PrintData {
            template: "occupational_health_exam".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 染色返工单打印数据
    async fn get_dye_batch_rework_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::dye_batch_rework;

        let record = dye_batch_rework::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("染色返工单 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("original_batch_no".to_string(), serde_json::json!(record.original_batch_no.clone()));
        data.insert("rework_batch_no".to_string(), serde_json::json!(record.rework_batch_no.clone().unwrap_or_default()));
        data.insert("rework_type".to_string(), serde_json::json!(record.rework_type.clone()));
        data.insert("rework_reason".to_string(), serde_json::json!(record.rework_reason.clone()));
        data.insert("original_status".to_string(), serde_json::json!(record.original_status.clone()));
        data.insert("rework_cost".to_string(), serde_json::json!(record.rework_cost.map(|v| v.to_string()).unwrap_or_default()));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));
        data.insert("started_at".to_string(), serde_json::json!(record.started_at.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()));
        data.insert("completed_at".to_string(), serde_json::json!(record.completed_at.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()));

        Ok(PrintData {
            template: "dye_batch_rework".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 坏账核销单打印数据
    async fn get_bad_debt_writeoff_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::bad_debt_writeoff;

        let record = bad_debt_writeoff::Entity::find_by_id(id as i32)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("坏账核销单 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("customer_id".to_string(), serde_json::json!(record.customer_id));
        data.insert("ar_invoice_id".to_string(), serde_json::json!(record.ar_invoice_id));
        data.insert("writeoff_amount".to_string(), serde_json::json!(record.writeoff_amount.to_string()));
        data.insert("reason".to_string(), serde_json::json!(record.reason.clone()));
        data.insert("applicant_username".to_string(), serde_json::json!(record.applicant_username.clone()));
        data.insert("applicant_at".to_string(), serde_json::json!(record.applicant_at.format("%Y-%m-%d").to_string()));
        data.insert("approval_status".to_string(), serde_json::json!(record.approval_status.clone()));
        data.insert("finance_manager_comment".to_string(), serde_json::json!(record.finance_manager_comment.clone().unwrap_or_default()));
        data.insert("general_manager_comment".to_string(), serde_json::json!(record.general_manager_comment.clone().unwrap_or_default()));

        Ok(PrintData {
            template: "bad_debt_writeoff".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 定制订单打印数据
    async fn get_custom_order_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::custom_order;

        let record = custom_order::Entity::find_by_id(id as i32)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("定制订单 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("order_no".to_string(), serde_json::json!(record.order_no.clone()));
        data.insert("customer_id".to_string(), serde_json::json!(record.customer_id));
        data.insert("product_id".to_string(), serde_json::json!(record.product_id));
        data.insert("color_id".to_string(), serde_json::json!(record.color_id));
        data.insert("spec".to_string(), serde_json::json!(record.spec.clone()));
        data.insert("quantity".to_string(), serde_json::json!(record.quantity.to_string()));
        data.insert("unit".to_string(), serde_json::json!(record.unit.clone()));
        data.insert("custom_requirements".to_string(), serde_json::json!(record.custom_requirements.clone()));
        data.insert("yarn_spec".to_string(), serde_json::json!(record.yarn_spec.clone().unwrap_or_default()));
        data.insert("dye_method".to_string(), serde_json::json!(record.dye_method.clone().unwrap_or_default()));
        data.insert("finishing_method".to_string(), serde_json::json!(record.finishing_method.clone().unwrap_or_default()));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));
        data.insert("expected_delivery_date".to_string(), serde_json::json!(record.expected_delivery_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()));
        data.insert("total_amount".to_string(), serde_json::json!(record.total_amount.map(|v| v.to_string()).unwrap_or_default()));
        data.insert("currency".to_string(), serde_json::json!(record.currency.clone()));

        Ok(PrintData {
            template: "custom_order".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 劳保用品发放记录打印数据
    async fn get_ppe_distribution_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::ppe_distribution_record;

        let record = ppe_distribution_record::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("劳保用品发放记录 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("worker_id".to_string(), serde_json::json!(record.worker_id));
        data.insert("ppe_name".to_string(), serde_json::json!(record.ppe_name.clone()));
        data.insert("ppe_type".to_string(), serde_json::json!(record.ppe_type.clone()));
        data.insert("specification".to_string(), serde_json::json!(record.specification.clone().unwrap_or_default()));
        data.insert("quantity".to_string(), serde_json::json!(record.quantity.to_string()));
        data.insert("distribution_date".to_string(), serde_json::json!(record.distribution_date.format("%Y-%m-%d").to_string()));
        data.insert("expiry_date".to_string(), serde_json::json!(record.expiry_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()));
        data.insert("hazard_type".to_string(), serde_json::json!(record.hazard_type.clone().unwrap_or_default()));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));

        Ok(PrintData {
            template: "ppe_distribution".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 客户信用额度打印数据
    async fn get_customer_credit_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::customer_credit;

        let record = customer_credit::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("客户信用额度 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("customer_id".to_string(), serde_json::json!(record.customer_id));
        data.insert("customer_name".to_string(), serde_json::json!(record.customer_name.clone().unwrap_or_default()));
        data.insert("credit_level".to_string(), serde_json::json!(record.credit_level.clone().unwrap_or_default()));
        data.insert("credit_score".to_string(), serde_json::json!(record.credit_score.map(|v| v.to_string()).unwrap_or_default()));
        data.insert("credit_limit".to_string(), serde_json::json!(record.credit_limit.to_string()));
        data.insert("used_credit".to_string(), serde_json::json!(record.used_credit.to_string()));
        data.insert("available_credit".to_string(), serde_json::json!(record.available_credit.to_string()));
        data.insert("credit_days".to_string(), serde_json::json!(record.credit_days));
        data.insert("last_assessment_date".to_string(), serde_json::json!(record.last_assessment_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));

        Ok(PrintData {
            template: "customer_credit".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 售后服务单打印数据
    async fn get_after_sales_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::after_sales;

        let record = after_sales::Entity::find_by_id(id as i32)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("售后服务单 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("custom_order_id".to_string(), serde_json::json!(record.custom_order_id));
        data.insert("issue_type".to_string(), serde_json::json!(record.issue_type.clone()));
        data.insert("customer_id".to_string(), serde_json::json!(record.customer_id));
        data.insert("description".to_string(), serde_json::json!(record.description.clone()));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));
        data.insert("opened_at".to_string(), serde_json::json!(record.opened_at.format("%Y-%m-%d").to_string()));
        data.insert("closed_at".to_string(), serde_json::json!(record.closed_at.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()));
        data.insert("resolution".to_string(), serde_json::json!(record.resolution.clone().unwrap_or_default()));
        data.insert("refund_amount".to_string(), serde_json::json!(record.refund_amount.map(|v| v.to_string()).unwrap_or_default()));
        data.insert("evaluation_score".to_string(), serde_json::json!(record.evaluation_score.map(|v| v.to_string()).unwrap_or_default()));
        data.insert("evaluation_comment".to_string(), serde_json::json!(record.evaluation_comment.clone().unwrap_or_default()));

        Ok(PrintData {
            template: "after_sales".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 质量问题单打印数据
    async fn get_quality_issue_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::quality_issue;

        let record = quality_issue::Entity::find_by_id(id as i32)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("质量问题单 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("custom_order_id".to_string(), serde_json::json!(record.custom_order_id));
        data.insert("issue_type".to_string(), serde_json::json!(record.issue_type.clone()));
        data.insert("severity".to_string(), serde_json::json!(record.severity.clone()));
        data.insert("description".to_string(), serde_json::json!(record.description.clone()));
        data.insert("discovered_at".to_string(), serde_json::json!(record.discovered_at.format("%Y-%m-%d").to_string()));
        data.insert("resolved_at".to_string(), serde_json::json!(record.resolved_at.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()));
        data.insert("resolution".to_string(), serde_json::json!(record.resolution.clone().unwrap_or_default()));
        data.insert("status".to_string(), serde_json::json!(record.status.clone()));
        data.insert("root_cause_method".to_string(), serde_json::json!(record.root_cause_method.clone().unwrap_or_default()));
        data.insert("root_cause_detail".to_string(), serde_json::json!(record.root_cause_detail.clone().unwrap_or_default()));

        Ok(PrintData {
            template: "quality_issue".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 应收对账单打印数据
    async fn get_ar_reconciliation_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::ar_reconciliation;

        let record = ar_reconciliation::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("应收对账单 {} 未找到", id)))?;

        let mut data = HashMap::new();
        data.insert("reconciliation_no".to_string(), serde_json::json!(record.reconciliation_no.clone()));
        data.insert("reconciliation_date".to_string(), serde_json::json!(record.reconciliation_date.format("%Y-%m-%d").to_string()));
        data.insert("period_start".to_string(), serde_json::json!(record.period_start.format("%Y-%m-%d").to_string()));
        data.insert("period_end".to_string(), serde_json::json!(record.period_end.format("%Y-%m-%d").to_string()));
        data.insert("customer_id".to_string(), serde_json::json!(record.customer_id));
        data.insert("customer_name".to_string(), serde_json::json!(record.customer_name.clone().unwrap_or_default()));
        data.insert("opening_balance".to_string(), serde_json::json!(record.opening_balance.to_string()));
        data.insert("total_invoices".to_string(), serde_json::json!(record.total_invoices.to_string()));
        data.insert("total_collections".to_string(), serde_json::json!(record.total_collections.to_string()));
        data.insert("closing_balance".to_string(), serde_json::json!(record.closing_balance.to_string()));
        data.insert("reconciliation_status".to_string(), serde_json::json!(record.reconciliation_status.clone()));

        Ok(PrintData {
            template: "ar_reconciliation".to_string(),
            data,
            items: Vec::new(),
        })
    }

    pub fn generate_docx(&self, print_data: &PrintData) -> Result<Vec<u8>, AppError> {
        let title = match print_data.template.as_str() {
            "sales_order" => "销售订单",
            "sales_contract" => "销售合同",
            "purchase_order" => "采购订单",
            "purchase_receipt" => "采购收货单",
            "inventory_transfer" => "库存调拨单",
            "voucher" => "会计凭证",
            "production_flow_card" => "生产流转卡",
            "fabric_inspection" => "验布记录",
            "dye_batch_card" => "染色批次卡",
            "color_card_issue" => "色卡发放单",
            "bulk_color_approval" => "大货色审批单",
            "lab_dip_request" => "打样申请单",
            "production_order" => "生产工单",
            "production_recipe" => "生产配方",
            "quality_inspection_record" => "质检记录",
            "sales_delivery" => "销售发货单",
            "ar_collection" => "应收账款收款单",
            "ap_payment" => "应付账款付款单",
            "ap_invoice" => "应付发票",
            "sales_quotation" => "销售报价单",
            "sales_return" => "销售退货单",
            "purchase_return" => "采购退货单",
            "outsourcing_order" => "委外加工单",
            "outsourcing_receipt" => "委外收货单",
            "logistics_waybill" => "物流运单",
            "certificate_of_origin" => "原产地证明",
            "export_customs_declaration" => "出口报关单",
            "solid_waste_disposal" => "固废处置单",
            "unqualified_product" => "不合格品处理单",
            "chemical_requisition" => "化工料领用单",
            "export_inspection" => "出口检验单",
            "ap_payment_request" => "付款申请单",
            "ap_reconciliation" => "应付对账单",
            "purchase_inspection" => "采购质检单",
            "inventory_adjustment" => "库存调整单",
            "bom" => "物料清单",
            "material_shortage" => "物料缺料预警",
            "quality_8d_report" => "8D质量报告",
            "labor_contract" => "劳动合同",
            "wage_record" => "工资记录",
            "energy_consumption_record" => "能耗记录",
            "purchase_contract" => "采购合同",
            "supplier_evaluation_record" => "供应商评价记录",
            "safety_accident_report" => "安全事故报告",
            "occupational_hazard_monitoring" => "职业病危害监测",
            "pollution_permit" => "排污许可证",
            "process_route" => "工艺路线",
            "foreign_exchange_verification" => "外汇核销单",
            "export_refund_declaration" => "出口退税申报单",
            "fixed_asset" => "固定资产卡片",
            "scheduling_result" => "排程结果",
            "inventory_write_down" => "存货跌价准备",
            "fixed_asset_count" => "固定资产盘点单",
            "social_insurance_record" => "社保缴纳记录",
            "occupational_health_exam" => "职业健康体检",
            "dye_batch_rework" => "染色返工单",
            "bad_debt_writeoff" => "坏账核销单",
            "custom_order" => "定制订单",
            "ppe_distribution" => "劳保用品发放记录",
            "customer_credit" => "客户信用额度",
            "after_sales" => "售后服务单",
            "quality_issue" => "质量问题单",
            "ar_reconciliation" => "应收对账单",
            other => other,
        };

        // 主表键值对
        let mut keys = Vec::with_capacity(print_data.data.len());
        let mut values = Vec::with_capacity(print_data.data.len());
        for (k, v) in print_data.data.iter() {
            keys.push(k.clone());
            let v_str = match v {
                serde_json::Value::String(s) => s.clone(),
                _ => v.to_string(),
            };
            values.push(v_str);
        }
        let kv = crate::utils::docx_export::DocxKeyValue { keys, values };

        // 明细表头与行：若 items 为空，则不输出明细表
        let detail_headers: Vec<String> = if print_data.items.is_empty() {
            Vec::new()
        } else {
            print_data
                .items
                .first()
                .map(|first| first.keys().cloned().collect())
                .unwrap_or_default()
        };
        let detail_rows: Vec<Vec<String>> = print_data
            .items
            .iter()
            .map(|row| {
                detail_headers
                    .iter()
                    .map(|h| {
                        row.get(h)
                            .map(|v| match v {
                                serde_json::Value::String(s) => s.clone(),
                                _ => v.to_string(),
                            })
                            .unwrap_or_default()
                    })
                    .collect()
            })
            .collect();

        crate::utils::docx_export::build_docx_with_kv(title, &kv, &detail_headers, &detail_rows)
    }
}
