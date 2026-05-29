//! 通用打印服务

use crate::utils::error::AppError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 打印数据类型
#[derive(Debug, Serialize, Deserialize)]
pub struct PrintData {
    pub template: String,
    pub data: HashMap<String, serde_json::Value>,
    pub items: Vec<HashMap<String, serde_json::Value>>,
}

/// 打印服务
pub struct PrintService;

impl Default for PrintService {
    fn default() -> Self {
        Self::new()
    }
}

impl PrintService {
    pub fn new() -> Self {
        Self
    }

    /// 获取打印数据
    pub async fn get_print_data(&self, doc_type: &str, doc_id: i32) -> Result<PrintData, AppError> {
        match doc_type {
            "sales_order" => self.get_sales_order_print_data(doc_id).await,
            "sales_contract" => self.get_sales_contract_print_data(doc_id).await,
            "purchase_order" => self.get_purchase_order_print_data(doc_id).await,
            "purchase_receipt" => self.get_purchase_receipt_print_data(doc_id).await,
            "inventory_transfer" => self.get_inventory_transfer_print_data(doc_id).await,
            "inventory_count" => self.get_inventory_count_print_data(doc_id).await,
            "voucher" => self.get_voucher_print_data(doc_id).await,
            _ => Err(AppError::NotFound(format!(
                "Unknown document type: {}",
                doc_type
            ))),
        }
    }

    async fn get_sales_order_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        let mut data = HashMap::new();
        data.insert(
            "order_no".to_string(),
            serde_json::json!(format!("SO-{:06}", id)),
        );
        data.insert("customer_name".to_string(), serde_json::json!("客户名称"));

        Ok(PrintData {
            template: "sales_order".to_string(),
            data,
            items: Vec::new(),
        })
    }

    async fn get_sales_contract_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        let mut data = HashMap::new();
        data.insert(
            "contract_no".to_string(),
            serde_json::json!(format!("SC-{:06}", id)),
        );

        Ok(PrintData {
            template: "sales_contract".to_string(),
            data,
            items: Vec::new(),
        })
    }

    async fn get_purchase_order_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        let mut data = HashMap::new();
        data.insert(
            "order_no".to_string(),
            serde_json::json!(format!("PO-{:06}", id)),
        );

        Ok(PrintData {
            template: "purchase_order".to_string(),
            data,
            items: Vec::new(),
        })
    }

    async fn get_purchase_receipt_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        let mut data = HashMap::new();
        data.insert(
            "receipt_no".to_string(),
            serde_json::json!(format!("RC-{:06}", id)),
        );

        Ok(PrintData {
            template: "purchase_receipt".to_string(),
            data,
            items: Vec::new(),
        })
    }

    async fn get_inventory_transfer_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        let mut data = HashMap::new();
        data.insert(
            "transfer_no".to_string(),
            serde_json::json!(format!("TR-{:06}", id)),
        );

        Ok(PrintData {
            template: "inventory_transfer".to_string(),
            data,
            items: Vec::new(),
        })
    }

    async fn get_inventory_count_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        let mut data = HashMap::new();
        data.insert(
            "count_no".to_string(),
            serde_json::json!(format!("CT-{:06}", id)),
        );

        Ok(PrintData {
            template: "inventory_count".to_string(),
            data,
            items: Vec::new(),
        })
    }

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
                .map(|(k, v)| format!("<tr><td>{}</td><td>{}</td></tr>", k, v))
                .collect::<Vec<_>>()
                .join("\n"),
            print_data.items.len()
        );
        Ok(html)
    }
}
