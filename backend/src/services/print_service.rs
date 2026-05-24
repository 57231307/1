//! 通用打印服务
//!
//! 支持所有单据类型的打印功能

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

impl PrintService {
    pub fn new() -> Self {
        Self
    }

    /// 获取打印数据
    pub async fn get_print_data(
        &self,
        doc_type: &str,
        doc_id: i32,
    ) -> Result<PrintData, AppError> {
        match doc_type {
            "sales_order" => self.get_sales_order_print_data(doc_id).await,
            "purchase_order" => self.get_purchase_order_print_data(doc_id).await,
            "purchase_receipt" => self.get_purchase_receipt_print_data(doc_id).await,
            "inventory_transfer" => self.get_inventory_transfer_print_data(doc_id).await,
            "inventory_count" => self.get_inventory_count_print_data(doc_id).await,
            _ => Err(AppError::NotFound(format!("Unknown document type: {}", doc_type))),
        }
    }

    /// 销售订单打印数据
    async fn get_sales_order_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        // TODO: 从数据库获取数据
        let mut data = HashMap::new();
        data.insert("order_no".to_string(), serde_json::json!(format!("SO-{:06}", id)));
        data.insert("customer_name".to_string(), serde_json::json!("客户名称"));
        data.insert("order_date".to_string(), serde_json::json!("2026-05-24"));
        
        let mut items = Vec::new();
        // TODO: 获取订单明细
        
        Ok(PrintData {
            template: "sales_order".to_string(),
            data,
            items,
        })
    }

    /// 采购订单打印数据
    async fn get_purchase_order_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        let mut data = HashMap::new();
        data.insert("order_no".to_string(), serde_json::json!(format!("PO-{:06}", id)));
        data.insert("supplier_name".to_string(), serde_json::json!("供应商名称"));
        
        Ok(PrintData {
            template: "purchase_order".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 采购入库单打印数据
    async fn get_purchase_receipt_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        let mut data = HashMap::new();
        data.insert("receipt_no".to_string(), serde_json::json!(format!("RC-{:06}", id)));
        
        Ok(PrintData {
            template: "purchase_receipt".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 库存调拨单打印数据
    async fn get_inventory_transfer_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        let mut data = HashMap::new();
        data.insert("transfer_no".to_string(), serde_json::json!(format!("TR-{:06}", id)));
        
        Ok(PrintData {
            template: "inventory_transfer".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 库存盘点单打印数据
    async fn get_inventory_count_print_data(&self, id: i32) -> Result<PrintData, AppError> {
        let mut data = HashMap::new();
        data.insert("count_no".to_string(), serde_json::json!(format!("CT-{:06}", id)));
        
        Ok(PrintData {
            template: "inventory_count".to_string(),
            data,
            items: Vec::new(),
        })
    }

    /// 生成 PDF（简化版本，返回 HTML）
    pub fn generate_pdf(&self, print_data: &PrintData) -> Result<String, AppError> {
        // TODO: 使用 PDF 生成库
        // 暂时返回简单的 HTML
        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head><title>打印单据</title></head>
<body>
<h1>{}</h1>
<p>单据数据：{:?}</p>
<p>明细数量：{}</p>
</body>
</html>"#,
            print_data.template,
            print_data.data,
            print_data.items.len()
        );
        Ok(html)
    }
}

// 添加 items 字段
PrintData {
    template: "sales_order".to_string(),
    data,
    items,
}
