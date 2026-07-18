//! 通用打印服务

use crate::search::SearchClient;
use crate::services::inv::InventoryTransferService;
use crate::services::po::PurchaseOrderService;
use crate::services::purchase_receipt_service::PurchaseReceiptService;
use crate::services::sales_contract_service::SalesContractService;
use crate::services::so::SalesService;
use crate::utils::error::AppError;
use sea_orm::DatabaseConnection;
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

/// 将任意 serde_json::Value 转为可读字符串（用于 HTML 渲染）
fn value_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        _ => v.to_string(),
    }
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
/// V15 P0-S17（Batch 476）：从硬编码占位数据改为真实查询数据库。
/// 持有 `db` 与 `search_client` 字段以构造业务 service 实例。
pub struct PrintService {
    db: Arc<DatabaseConnection>,
    search_client: Arc<dyn SearchClient>,
}

impl PrintService {
    pub fn new(db: Arc<DatabaseConnection>, search_client: Arc<dyn SearchClient>) -> Self {
        Self { db, search_client }
    }

    /// 获取打印数据
    pub async fn get_print_data(&self, doc_type: &str, doc_id: i32) -> Result<PrintData, AppError> {
        match doc_type {
            "sales_order" => self.get_sales_order_print_data(doc_id).await,
            "sales_contract" => self.get_sales_contract_print_data(doc_id).await,
            "purchase_order" => self.get_purchase_order_print_data(doc_id).await,
            "purchase_receipt" => self.get_purchase_receipt_print_data(doc_id).await,
            "inventory_transfer" => self.get_inventory_transfer_print_data(doc_id).await,
            // v10 P1-3 修复：删除 inventory_count 分支（inventory_count_service 已在 v9 P1-F 删除）
            "voucher" => self.get_voucher_print_data(doc_id).await,
            _ => Err(AppError::not_found(format!(
                "Unknown document type: {}",
                doc_type
            ))),
        }
    }

    /// 销售订单打印数据（真实查询）
    async fn get_sales_order_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        let svc = SalesService::new(self.db.clone(), self.search_client.clone());
        let detail = svc.get_order_detail(id, None).await?;

        let mut data = HashMap::new();
        data.insert("order_no".to_string(), serde_json::json!(detail.order_no));
        data.insert(
            "customer_name".to_string(),
            serde_json::json!(detail.customer_name.unwrap_or_default()),
        );
        data.insert(
            "order_date".to_string(),
            serde_json::json!(detail.order_date.format("%Y-%m-%d").to_string()),
        );
        data.insert(
            "required_date".to_string(),
            serde_json::json!(detail.required_date.format("%Y-%m-%d").to_string()),
        );
        data.insert("status".to_string(), serde_json::json!(detail.status));
        data.insert(
            "total_amount".to_string(),
            serde_json::json!(detail.total_amount.to_string()),
        );
        data.insert(
            "paid_amount".to_string(),
            serde_json::json!(detail.paid_amount.to_string()),
        );
        data.insert(
            "balance_amount".to_string(),
            serde_json::json!(detail.balance_amount.to_string()),
        );
        data.insert(
            "shipping_address".to_string(),
            serde_json::json!(detail.shipping_address.unwrap_or_default()),
        );
        data.insert(
            "notes".to_string(),
            serde_json::json!(detail.notes.unwrap_or_default()),
        );

        let items: Vec<HashMap<String, serde_json::Value>> = detail
            .items
            .iter()
            .map(|item| {
                let mut m = HashMap::new();
                m.insert(
                    "product_code".to_string(),
                    serde_json::json!(item.product_code.clone().unwrap_or_default()),
                );
                m.insert(
                    "product_name".to_string(),
                    serde_json::json!(item.product_name.clone().unwrap_or_default()),
                );
                m.insert(
                    "quantity".to_string(),
                    serde_json::json!(item.quantity.to_string()),
                );
                m.insert(
                    "unit_price".to_string(),
                    serde_json::json!(item.unit_price.to_string()),
                );
                m.insert(
                    "total_amount".to_string(),
                    serde_json::json!(item.total_amount.to_string()),
                );
                m.insert("color_no".to_string(), serde_json::json!(item.color_no.clone()));
                m.insert(
                    "color_name".to_string(),
                    serde_json::json!(item.color_name.clone().unwrap_or_default()),
                );
                m
            })
            .collect();

        Ok(PrintData {
            template: "sales_order".to_string(),
            data,
            items,
        })
    }

    /// 销售合同打印数据（真实查询）
    async fn get_sales_contract_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        let svc = SalesContractService::new(self.db.clone());
        let contract = svc.get_by_id(id).await?;

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
            serde_json::json!(contract.customer_name.unwrap_or_default()),
        );
        data.insert(
            "total_amount".to_string(),
            serde_json::json!(contract
                .total_amount
                .map(|v| v.to_string())
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
            "status".to_string(),
            serde_json::json!(contract.status),
        );

        Ok(PrintData {
            template: "sales_contract".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 采购订单打印数据（真实查询）
    async fn get_purchase_order_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        let svc = PurchaseOrderService::new(self.db.clone());
        let order = svc.get_order(id, None).await?;
        let items_dto = svc.list_order_items(id).await?;

        let mut data = HashMap::new();
        data.insert("order_no".to_string(), serde_json::json!(order.order_no));
        data.insert(
            "supplier_name".to_string(),
            serde_json::json!(order.supplier_name.unwrap_or_default()),
        );
        data.insert(
            "warehouse_name".to_string(),
            serde_json::json!(order.warehouse_name.unwrap_or_default()),
        );
        data.insert(
            "department_name".to_string(),
            serde_json::json!(order.department_name.unwrap_or_default()),
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
            "currency".to_string(),
            serde_json::json!(order.currency),
        );
        data.insert(
            "total_amount".to_string(),
            serde_json::json!(order.total_amount.to_string()),
        );
        data.insert(
            "total_quantity".to_string(),
            serde_json::json!(order.total_quantity.to_string()),
        );
        data.insert("status".to_string(), serde_json::json!(order.order_status));
        data.insert(
            "payment_terms".to_string(),
            serde_json::json!(order.payment_terms.unwrap_or_default()),
        );
        data.insert(
            "notes".to_string(),
            serde_json::json!(order.notes.unwrap_or_default()),
        );

        let items: Vec<HashMap<String, serde_json::Value>> = items_dto
            .iter()
            .map(|item| {
                let mut m = HashMap::new();
                m.insert(
                    "line_no".to_string(),
                    serde_json::json!(item.line_no),
                );
                m.insert(
                    "material_code".to_string(),
                    serde_json::json!(item.material_code.clone().unwrap_or_default()),
                );
                m.insert(
                    "material_name".to_string(),
                    serde_json::json!(item.material_name.clone().unwrap_or_default()),
                );
                m.insert(
                    "quantity".to_string(),
                    serde_json::json!(item.quantity.to_string()),
                );
                m.insert(
                    "unit_price".to_string(),
                    serde_json::json!(item.unit_price.to_string()),
                );
                m.insert(
                    "total_amount".to_string(),
                    serde_json::json!(item.total_amount.to_string()),
                );
                m.insert(
                    "received_quantity".to_string(),
                    serde_json::json!(item.received_quantity.to_string()),
                );
                m
            })
            .collect();

        Ok(PrintData {
            template: "purchase_order".to_string(),
            data,
            items,
        })
    }

    /// 采购收货单打印数据（真实查询）
    async fn get_purchase_receipt_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        let svc = PurchaseReceiptService::new(self.db.clone());
        let receipt = svc.get_receipt(id).await?;
        let items_model = svc.list_receipt_items(id).await?;

        let mut data = HashMap::new();
        data.insert(
            "receipt_no".to_string(),
            serde_json::json!(receipt.receipt_no),
        );
        data.insert(
            "supplier_id".to_string(),
            serde_json::json!(receipt.supplier_id),
        );
        data.insert(
            "receipt_date".to_string(),
            serde_json::json!(receipt.receipt_date.format("%Y-%m-%d").to_string()),
        );
        data.insert(
            "warehouse_id".to_string(),
            serde_json::json!(receipt.warehouse_id),
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

        let items: Vec<HashMap<String, serde_json::Value>> = items_model
            .iter()
            .map(|item| {
                let mut m = HashMap::new();
                m.insert(
                    "line_no".to_string(),
                    serde_json::json!(item.line_no),
                );
                m.insert(
                    "material_code".to_string(),
                    serde_json::json!(item.material_code.clone()),
                );
                m.insert(
                    "material_name".to_string(),
                    serde_json::json!(item.material_name.clone()),
                );
                m.insert(
                    "quantity".to_string(),
                    serde_json::json!(item.quantity.to_string()),
                );
                m.insert(
                    "unit_price".to_string(),
                    serde_json::json!(item
                        .unit_price
                        .map(|v| v.to_string())
                        .unwrap_or_default()),
                );
                m.insert(
                    "amount".to_string(),
                    serde_json::json!(item
                        .amount
                        .map(|v| v.to_string())
                        .unwrap_or_default()),
                );
                m.insert(
                    "color_code".to_string(),
                    serde_json::json!(item.color_code.clone().unwrap_or_default()),
                );
                m.insert(
                    "batch_no".to_string(),
                    serde_json::json!(item.batch_no.clone().unwrap_or_default()),
                );
                m
            })
            .collect();

        Ok(PrintData {
            template: "purchase_receipt".to_string(),
            data,
            items,
        })
    }

    /// 库存调拨单打印数据（真实查询）
    async fn get_inventory_transfer_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        let svc = InventoryTransferService::new(self.db.clone());
        let detail = svc.get_transfer_detail(id, None).await?;

        let mut data = HashMap::new();
        data.insert(
            "transfer_no".to_string(),
            serde_json::json!(detail.transfer_no),
        );
        data.insert(
            "from_warehouse_id".to_string(),
            serde_json::json!(detail.from_warehouse_id),
        );
        data.insert(
            "to_warehouse_id".to_string(),
            serde_json::json!(detail.to_warehouse_id),
        );
        data.insert(
            "transfer_date".to_string(),
            serde_json::json!(detail.transfer_date.format("%Y-%m-%d %H:%M:%S").to_string()),
        );
        data.insert("status".to_string(), serde_json::json!(detail.status));
        data.insert(
            "total_quantity".to_string(),
            serde_json::json!(detail.total_quantity.to_string()),
        );
        data.insert(
            "notes".to_string(),
            serde_json::json!(detail.notes.unwrap_or_default()),
        );

        let items: Vec<HashMap<String, serde_json::Value>> = detail
            .items
            .iter()
            .map(|item| {
                let mut m = HashMap::new();
                m.insert(
                    "product_id".to_string(),
                    serde_json::json!(item.product_id),
                );
                m.insert(
                    "quantity".to_string(),
                    serde_json::json!(item.quantity.to_string()),
                );
                m.insert(
                    "shipped_quantity".to_string(),
                    serde_json::json!(item.shipped_quantity.to_string()),
                );
                m.insert(
                    "received_quantity".to_string(),
                    serde_json::json!(item.received_quantity.to_string()),
                );
                m.insert(
                    "unit_cost".to_string(),
                    serde_json::json!(item.unit_cost.map(|v| v.to_string()).unwrap_or_default()),
                );
                m.insert(
                    "notes".to_string(),
                    serde_json::json!(item.notes.clone().unwrap_or_default()),
                );
                m
            })
            .collect();

        Ok(PrintData {
            template: "inventory_transfer".to_string(),
            data,
            items,
        })
    }

    // v10 P1-3 修复：删除 get_inventory_count_print_data 死代码（inventory_count_service 已在 v9 P1-F 删除）

    /// 会计凭证打印数据（保留占位）
    ///
    /// 路由未注册 voucher print handler（仅 print_service 内部分支保留），
    /// 凭证打印功能暂未实现真实查询，待后续单独修复时扩展。
    async fn get_voucher_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        let mut data = HashMap::new();
        data.insert(
            "voucher_no".to_string(),
            serde_json::json!(format!("VCH-{:06}", id)),
        );

        Ok(PrintData {
            template: "voucher".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 生成 HTML
    ///
    /// V15 P0-S17（Batch 476）：增强模板渲染——主表字段渲染为键值表，
    /// 明细 items 渲染为完整表格（含所有字段列）。
    pub fn generate_pdf(&self, print_data: &PrintData) -> Result<String, AppError> {
        // 主表字段渲染
        let main_table_rows = print_data
            .data
            .iter()
            .map(|(k, v)| {
                let v_str = value_to_string(v);
                format!(
                    "<tr><td>{}</td><td>{}</td></tr>",
                    escape_html(k),
                    escape_html(&v_str)
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        // 明细表格渲染（仅在 items 非空时渲染）
        let items_section = if print_data.items.is_empty() {
            String::new()
        } else {
            // 收集所有 items 的字段名（保持插入顺序）
            let mut columns: Vec<String> = Vec::new();
            let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
            for item in &print_data.items {
                for key in item.keys() {
                    if seen.insert(key.as_str()) {
                        columns.push(key.clone());
                    }
                }
            }

            // 表头单元格
            let header_cells: String = columns
                .iter()
                .map(|c| format!("<th>{}</th>", escape_html(c)))
                .collect::<Vec<_>>()
                .join("");

            // 表体行
            let body_rows: String = print_data
                .items
                .iter()
                .enumerate()
                .map(|(idx, item)| {
                    let cells: String = columns
                        .iter()
                        .map(|col| {
                            let v = item.get(col).map(value_to_string).unwrap_or_default();
                            format!("<td>{}</td>", escape_html(&v))
                        })
                        .collect::<Vec<_>>()
                        .join("");
                    format!("<tr><td>{}</td>{}</tr>", idx + 1, cells)
                })
                .collect::<Vec<_>>()
                .join("\n");

            format!(
                r#"<h2>明细列表</h2>
<table>
<tr><th>序号</th>{}</tr>
{}
</table>"#,
                header_cells, body_rows
            )
        };

        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head><meta charset="utf-8"><title>打印单据 - {}</title>
<style>
body{{font-family:Arial,'Microsoft YaHei',sans-serif;padding:20px;color:#333}}
h1{{color:#2c3e50;border-bottom:2px solid #2c3e50;padding-bottom:10px}}
h2{{color:#34495e;margin-top:30px}}
table{{border-collapse:collapse;width:100%;margin:10px 0}}
th,td{{border:1px solid #ddd;padding:8px;text-align:left}}
th{{background-color:#f5f5f5;font-weight:bold}}
tr:nth-child(even){{background-color:#fafafa}}
.meta{{color:#666;font-size:12px;margin-bottom:20px}}
</style>
</head>
<body>
<h1>{}</h1>
<p class="meta">打印时间：{}</p>
<h2>单据信息</h2>
<table>
<tr><th>字段</th><th>值</th></tr>
{}
</table>
{}
</body>
</html>"#,
            escape_html(&print_data.template),
            escape_html(&print_data.template),
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            main_table_rows,
            items_section
        );
        Ok(html)
    }
}
