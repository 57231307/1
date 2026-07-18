//! 通用打印服务
//!
//! V15 P0-S17 修复（Batch 476）：6 个 get_*_print_data 方法从硬编码占位数据
//! 改为真实查询数据库（主表 + 关联客户/供应商/仓库 + 明细项）。

use crate::utils::error::AppError;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, LoaderTrait, ModelTrait, Order, QueryFilter, QueryOrder, RelationTrait};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// HTML 实体转义：将用户提供的数据安全嵌入 HTML，
/// 防止 XSS（跨站脚本攻击）。
/// 转义字符参考 OWASP 推荐：& < > " '
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// 打印数据类型
#[derive(Debug, Serialize, Deserialize)]
pub struct PrintData {
    pub template: String,
    pub data: HashMap<String, serde_json::Value>,
    pub items: Vec<HashMap<String, serde_json::Value>>,
}

/// 打印服务
///
/// V15 P0-S17：持有数据库连接，6 个 get_*_print_data 方法真实查询数据库
pub struct PrintService {
    db: Arc<DatabaseConnection>,
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
            _ => Err(AppError::not_found(format!(
                "Unknown document type: {}",
                doc_type
            ))),
        }
    }

    /// 销售订单打印数据：订单主表 + 客户 + 明细项（含产品）
    async fn get_sales_order_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        use crate::models::{customer, product, sales_order, sales_order_item};

        let order = sales_order::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售订单 {} 未找到", id)))?;

        let customer = order
            .find_related(customer::Entity)
            .one(&*self.db)
            .await?;

        let items = order
            .find_related(sales_order_item::Entity)
            .order_by(sales_order_item::Column::Id, Order::Asc)
            .all(&*self.db)
            .await?;

        let products = items.load_one(product::Entity, &*self.db).await?;

        let mut data = HashMap::new();
        data.insert("order_no".to_string(), serde_json::json!(order.order_no));
        data.insert(
            "customer_name".to_string(),
            serde_json::json!(customer.as_ref().map(|c| c.customer_name.clone()).unwrap_or_default()),
        );
        data.insert(
            "customer_code".to_string(),
            serde_json::json!(customer.as_ref().map(|c| c.customer_code.clone()).unwrap_or_default()),
        );
        data.insert("order_date".to_string(), serde_json::json!(order.order_date.format("%Y-%m-%d").to_string()));
        data.insert(
            "required_date".to_string(),
            serde_json::json!(order.required_date.format("%Y-%m-%d").to_string()),
        );
        data.insert(
            "ship_date".to_string(),
            serde_json::json!(order.ship_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()),
        );
        data.insert("status".to_string(), serde_json::json!(order.status));
        data.insert("subtotal".to_string(), serde_json::json!(order.subtotal.to_string()));
        data.insert("tax_amount".to_string(), serde_json::json!(order.tax_amount.to_string()));
        data.insert("discount_amount".to_string(), serde_json::json!(order.discount_amount.to_string()));
        data.insert("shipping_cost".to_string(), serde_json::json!(order.shipping_cost.to_string()));
        data.insert("total_amount".to_string(), serde_json::json!(order.total_amount.to_string()));
        data.insert("paid_amount".to_string(), serde_json::json!(order.paid_amount.to_string()));
        data.insert("balance_amount".to_string(), serde_json::json!(order.balance_amount.to_string()));
        data.insert(
            "shipping_address".to_string(),
            serde_json::json!(order.shipping_address.unwrap_or_default()),
        );
        data.insert(
            "billing_address".to_string(),
            serde_json::json!(order.billing_address.unwrap_or_default()),
        );
        data.insert("notes".to_string(), serde_json::json!(order.notes.unwrap_or_default()));

        let mut item_list = Vec::with_capacity(items.len());
        for (i, item) in items.into_iter().enumerate() {
            let product = products[i].as_ref();
            let mut row = HashMap::new();
            row.insert("line_no".to_string(), serde_json::json!((i + 1).to_string()));
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
            row.insert("quantity".to_string(), serde_json::json!(item.quantity.to_string()));
            row.insert("unit_price".to_string(), serde_json::json!(item.unit_price.to_string()));
            row.insert(
                "final_price".to_string(),
                serde_json::json!(item.final_price.map(|p| p.to_string()).unwrap_or_default()),
            );
            row.insert("subtotal".to_string(), serde_json::json!(item.subtotal.to_string()));
            row.insert("tax_amount".to_string(), serde_json::json!(item.tax_amount.to_string()));
            row.insert("total_amount".to_string(), serde_json::json!(item.total_amount.to_string()));
            row.insert(
                "quantity_meters".to_string(),
                serde_json::json!(item.quantity_meters.to_string()),
            );
            row.insert("quantity_kg".to_string(), serde_json::json!(item.quantity_kg.to_string()));
            item_list.push(row);
        }

        Ok(PrintData {
            template: "sales_order".to_string(),
            data,
            items: item_list,
        })
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
        data.insert("contract_no".to_string(), serde_json::json!(contract.contract_no));
        data.insert("contract_name".to_string(), serde_json::json!(contract.contract_name));
        data.insert(
            "contract_type".to_string(),
            serde_json::json!(contract.contract_type.unwrap_or_default()),
        );
        data.insert(
            "customer_name".to_string(),
            serde_json::json!(contract.customer_name.clone().unwrap_or_else(|| {
                customer.as_ref().map(|c| c.customer_name.clone()).unwrap_or_default()
            })),
        );
        data.insert(
            "customer_code".to_string(),
            serde_json::json!(customer.as_ref().map(|c| c.customer_code.clone()).unwrap_or_default()),
        );
        data.insert(
            "total_amount".to_string(),
            serde_json::json!(contract.total_amount.map(|a| a.to_string()).unwrap_or_default()),
        );
        data.insert(
            "signed_date".to_string(),
            serde_json::json!(contract.signed_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()),
        );
        data.insert(
            "effective_date".to_string(),
            serde_json::json!(contract.effective_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()),
        );
        data.insert(
            "expiry_date".to_string(),
            serde_json::json!(contract.expiry_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()),
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
            serde_json::json!(contract.delivery_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()),
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
        use crate::models::{product, purchase_order, purchase_order_item, supplier, warehouse};

        let order = purchase_order::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购订单 {} 未找到", id)))?;

        let supplier = supplier::Entity::find_by_id(order.supplier_id)
            .one(&*self.db)
            .await?;
        let warehouse = warehouse::Entity::find_by_id(order.warehouse_id)
            .one(&*self.db)
            .await?;

        let items = purchase_order_item::Entity::find()
            .filter(purchase_order_item::Column::OrderId.eq(id))
            .order_by(purchase_order_item::Column::LineNo, Order::Asc)
            .all(&*self.db)
            .await?;

        let product_ids: Vec<i32> = items.iter().map(|i| i.product_id).collect();
        let products = product::Entity::find()
            .filter(product::Column::Id.is_in(product_ids))
            .all(&*self.db)
            .await?;
        let product_map: HashMap<i32, product::Model> =
            products.into_iter().map(|p| (p.id, p)).collect();

        let mut data = HashMap::new();
        data.insert("order_no".to_string(), serde_json::json!(order.order_no));
        data.insert(
            "supplier_name".to_string(),
            serde_json::json!(supplier.as_ref().map(|s| s.supplier_name.clone()).unwrap_or_default()),
        );
        data.insert("order_date".to_string(), serde_json::json!(order.order_date.format("%Y-%m-%d").to_string()));
        data.insert(
            "expected_delivery_date".to_string(),
            serde_json::json!(order.expected_delivery_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()),
        );
        data.insert(
            "actual_delivery_date".to_string(),
            serde_json::json!(order.actual_delivery_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()),
        );
        data.insert(
            "warehouse_name".to_string(),
            serde_json::json!(warehouse.as_ref().map(|w| w.name.clone()).unwrap_or_default()),
        );
        data.insert("currency".to_string(), serde_json::json!(order.currency));
        data.insert("total_amount".to_string(), serde_json::json!(order.total_amount.to_string()));
        data.insert("total_quantity".to_string(), serde_json::json!(order.total_quantity.to_string()));
        data.insert("notes".to_string(), serde_json::json!(order.notes.unwrap_or_default()));

        let mut item_list = Vec::with_capacity(items.len());
        for item in items {
            let product = product_map.get(&item.product_id);
            let mut row = HashMap::new();
            row.insert("line_no".to_string(), serde_json::json!(item.line_no.to_string()));
            row.insert(
                "product_code".to_string(),
                serde_json::json!(product.map(|p| p.code.clone()).unwrap_or_default()),
            );
            row.insert(
                "product_name".to_string(),
                serde_json::json!(product.map(|p| p.name.clone()).unwrap_or_default()),
            );
            row.insert("quantity".to_string(), serde_json::json!(item.quantity.to_string()));
            row.insert("unit_price".to_string(), serde_json::json!(item.unit_price.to_string()));
            row.insert("tax_percent".to_string(), serde_json::json!(item.tax_percent.to_string()));
            row.insert("total_amount".to_string(), serde_json::json!(item.total_amount.to_string()));
            row.insert(
                "received_quantity".to_string(),
                serde_json::json!(item.received_quantity.to_string()),
            );
            item_list.push(row);
        }

        Ok(PrintData {
            template: "purchase_order".to_string(),
            data,
            items: item_list,
        })
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
        data.insert("receipt_no".to_string(), serde_json::json!(receipt.receipt_no));
        data.insert(
            "supplier_name".to_string(),
            serde_json::json!(supplier.as_ref().map(|s| s.supplier_name.clone()).unwrap_or_default()),
        );
        data.insert(
            "warehouse_name".to_string(),
            serde_json::json!(warehouse.as_ref().map(|w| w.name.clone()).unwrap_or_default()),
        );
        data.insert("receipt_date".to_string(), serde_json::json!(receipt.receipt_date.format("%Y-%m-%d").to_string()));
        data.insert("inspection_status".to_string(), serde_json::json!(receipt.inspection_status));
        data.insert("receipt_status".to_string(), serde_json::json!(receipt.receipt_status));
        data.insert("total_quantity".to_string(), serde_json::json!(receipt.total_quantity.to_string()));
        data.insert("total_amount".to_string(), serde_json::json!(receipt.total_amount.to_string()));
        data.insert("notes".to_string(), serde_json::json!(receipt.notes.unwrap_or_default()));

        let mut item_list = Vec::with_capacity(items.len());
        for item in items {
            let mut row = HashMap::new();
            row.insert("line_no".to_string(), serde_json::json!(item.line_no.to_string()));
            row.insert("material_code".to_string(), serde_json::json!(item.material_code));
            row.insert("material_name".to_string(), serde_json::json!(item.material_name));
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
            row.insert("quantity".to_string(), serde_json::json!(item.quantity.to_string()));
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

        let mut data = HashMap::new();
        data.insert("transfer_no".to_string(), serde_json::json!(transfer.transfer_no));
        data.insert(
            "from_warehouse_name".to_string(),
            serde_json::json!(from_warehouse.as_ref().map(|w| w.name.clone()).unwrap_or_default()),
        );
        data.insert(
            "to_warehouse_name".to_string(),
            serde_json::json!(to_warehouse.as_ref().map(|w| w.name.clone()).unwrap_or_default()),
        );
        data.insert(
            "transfer_date".to_string(),
            serde_json::json!(transfer.transfer_date.format("%Y-%m-%d %H:%M").to_string()),
        );
        data.insert("status".to_string(), serde_json::json!(transfer.status));
        data.insert("total_quantity".to_string(), serde_json::json!(transfer.total_quantity.to_string()));
        data.insert("notes".to_string(), serde_json::json!(transfer.notes.unwrap_or_default()));

        let mut item_list = Vec::with_capacity(items.len());
        for (i, item) in items.into_iter().enumerate() {
            let product = product_map.get(&item.product_id);
            let mut row = HashMap::new();
            row.insert("line_no".to_string(), serde_json::json!((i + 1).to_string()));
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
                "dye_lot_no".to_string(),
                serde_json::json!(item.dye_lot_no.unwrap_or_default()),
            );
            row.insert("batch_no".to_string(), serde_json::json!(item.batch_no));
            row.insert("quantity".to_string(), serde_json::json!(item.quantity.to_string()));
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

        Ok(PrintData {
            template: "inventory_transfer".to_string(),
            data,
            items: item_list,
        })
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
        data.insert("voucher_no".to_string(), serde_json::json!(voucher.voucher_no));
        data.insert("voucher_type".to_string(), serde_json::json!(voucher.voucher_type));
        data.insert("voucher_date".to_string(), serde_json::json!(voucher.voucher_date.format("%Y-%m-%d").to_string()));
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
            row.insert("line_no".to_string(), serde_json::json!(item.line_no.to_string()));
            row.insert("subject_code".to_string(), serde_json::json!(item.subject_code));
            row.insert("subject_name".to_string(), serde_json::json!(item.subject_name));
            row.insert("debit".to_string(), serde_json::json!(item.debit.to_string()));
            row.insert("credit".to_string(), serde_json::json!(item.credit.to_string()));
            row.insert(
                "summary".to_string(),
                serde_json::json!(item.summary.unwrap_or_default()),
            );
            item_list.push(row);
        }
        data.insert("total_debit".to_string(), serde_json::json!(total_debit.to_string()));
        data.insert("total_credit".to_string(), serde_json::json!(total_credit.to_string()));

        Ok(PrintData {
            template: "voucher".to_string(),
            data,
            items: item_list,
        })
    }

    /// 生成 HTML
    pub fn generate_pdf(&self, print_data: &PrintData) -> Result<String, AppError> {
        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head><title>打印单据 - {}</title>
<style>body{{font-family:Arial,sans-serif;padding:20px}}h1{{color:#333}}table{{border-collapse:collapse;width:100%}}th,td{{border:1px solid #ddd;padding:8px}}</style>
</head>
<body>
<h1>{}</h1>
<p>打印时间：{}</p>
<table>
<tr><th>字段</th><th>值</th></tr>
{}
</table>
<p>明细数量：{}</p>
</body>
</html>"#,
            print_data.template,
            print_data.template,
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            print_data
                .data
                .iter()
                .map(|(k, v)| {
                    // 将 JSON Value 转为字符串后做 HTML 转义，防止 XSS
                    let v_str = match v {
                        serde_json::Value::String(s) => s.clone(),
                        _ => v.to_string(),
                    };
                    format!(
                        "<tr><td>{}</td><td>{}</td></tr>",
                        escape_html(k),
                        escape_html(&v_str)
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"),
            print_data.items.len()
        );
        Ok(html)
    }
}
