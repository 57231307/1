//! 批次管理服务 API 客户端
//! 提供批次管理相关的 API 调用方法

use crate::services::api::ApiService;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Batch {
    pub id: i32,
    pub batch_no: String,
    pub product_id: i32,
    pub product_name: Option<String>,
    pub warehouse_id: i32,
    pub warehouse_name: Option<String>,
    pub color_no: String,
    pub color_name: Option<String>,
    pub dye_lot_no: Option<String>,
    pub grade: String,
    pub quantity_meters: f64,
    pub quantity_kg: f64,
    pub gram_weight: Option<f64>,
    pub width: Option<f64>,
    pub stock_status: String,
    pub quality_status: String,
    pub production_date: Option<String>,
    pub expiry_date: Option<String>,
    pub supplier_id: Option<i32>,
    pub supplier_name: Option<String>,
    pub purchase_order_no: Option<String>,
    pub remarks: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub product_id: Option<i32>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub grade: Option<String>,
    pub warehouse_id: Option<i32>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateBatchRequest {
    pub batch_no: String,
    pub product_id: i32,
    pub warehouse_id: i32,
    pub color_no: String,
    pub color_name: Option<String>,
    pub dye_lot_no: Option<String>,
    pub grade: String,
    pub quantity_meters: f64,
    pub quantity_kg: f64,
    pub gram_weight: Option<f64>,
    pub width: Option<f64>,
    pub production_date: Option<String>,
    pub expiry_date: Option<String>,
    pub supplier_id: Option<i32>,
    pub purchase_order_no: Option<String>,
    pub remarks: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateBatchRequest {
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub grade: Option<String>,
    pub gram_weight: Option<f64>,
    pub width: Option<f64>,
    pub expiry_date: Option<String>,
    pub remarks: Option<String>,
    pub stock_status: Option<String>,
    pub quality_status: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TransferBatchRequest {
    pub from_warehouse_id: i32,
    pub to_warehouse_id: i32,
    pub quantity_meters: f64,
    pub quantity_kg: f64,
    pub remarks: Option<String>,
}

pub struct BatchService;

impl BatchService {
    pub async fn list(query: BatchQuery) -> Result<Vec<Batch>, String> {
        let mut params = Vec::new();
        if let Some(page) = query.page {
            params.push(format!("page={}", page));
        }
        if let Some(page_size) = query.page_size {
            params.push(format!("page_size={}", page_size));
        }
        if let Some(product_id) = query.product_id {
            params.push(format!("product_id={}", product_id));
        }
        if let Some(ref batch_no) = query.batch_no {
            params.push(format!("batch_no={}", batch_no));
        }
        if let Some(ref color_no) = query.color_no {
            params.push(format!("color_no={}", color_no));
        }
        if let Some(ref grade) = query.grade {
            params.push(format!("grade={}", grade));
        }
        if let Some(warehouse_id) = query.warehouse_id {
            params.push(format!("warehouse_id={}", warehouse_id));
        }
        if let Some(ref start_date) = query.start_date {
            params.push(format!("start_date={}", start_date));
        }
        if let Some(ref end_date) = query.end_date {
            params.push(format!("end_date={}", end_date));
        }

        let query_string = if params.is_empty() {
            String::new()
        } else {
            format!("?{}", params.join("&"))
        };

        let response: serde_json::Value = ApiService::get(&format!("/api/v1/erp/batches{}", query_string)).await?;

        if let Some(data) = response.get("data").and_then(|v| v.as_array()) {
            let batches: Vec<Batch> = data
                .iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect();
            Ok(batches)
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn get(id: i32) -> Result<Batch, String> {
        let response: serde_json::Value = ApiService::get(&format!("/api/v1/erp/batches/{}", id)).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "获取批次详情失败".to_string())
    }

    pub async fn create(req: CreateBatchRequest) -> Result<Batch, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::post("/api/v1/erp/batches", &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "创建批次失败".to_string())
    }

    pub async fn update(id: i32, req: UpdateBatchRequest) -> Result<Batch, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::put(&format!("/api/v1/erp/batches/{}", id), &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "更新批次失败".to_string())
    }

    pub async fn delete(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/api/v1/erp/batches/{}", id)).await
    }

    pub async fn transfer(id: i32, req: TransferBatchRequest) -> Result<Batch, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::post(&format!("/api/v1/erp/batches/{}/transfer", id), &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "转移批次失败".to_string())
    }
}