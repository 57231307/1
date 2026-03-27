//! 坯布管理服务

use crate::services::api::ApiService;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GreigeFabric {
    pub id: i32,
    pub fabric_no: String,
    pub fabric_name: String,
    pub fabric_type: String,
    pub color_code: Option<String>,
    pub width_cm: Option<f64>,
    pub weight_kg: Option<f64>,
    pub length_m: Option<f64>,
    pub supplier_id: Option<i32>,
    pub batch_no: Option<String>,
    pub warehouse_id: Option<i32>,
    pub location: Option<String>,
    pub status: String,
    pub quality_grade: Option<String>,
    pub purchase_date: Option<String>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GreigeFabricQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub fabric_no: Option<String>,
    pub fabric_name: Option<String>,
    pub fabric_type: Option<String>,
    pub supplier_id: Option<i32>,
    pub warehouse_id: Option<i32>,
    pub status: Option<String>,
    pub quality_grade: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGreigeFabricRequest {
    pub fabric_no: String,
    pub fabric_name: String,
    pub fabric_type: String,
    pub color_code: Option<String>,
    pub width_cm: Option<f64>,
    pub weight_kg: Option<f64>,
    pub length_m: Option<f64>,
    pub supplier_id: Option<i32>,
    pub batch_no: Option<String>,
    pub warehouse_id: Option<i32>,
    pub location: Option<String>,
    pub status: Option<String>,
    pub quality_grade: Option<String>,
    pub purchase_date: Option<String>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateGreigeFabricRequest {
    pub fabric_name: Option<String>,
    pub fabric_type: Option<String>,
    pub color_code: Option<String>,
    pub width_cm: Option<f64>,
    pub weight_kg: Option<f64>,
    pub length_m: Option<f64>,
    pub supplier_id: Option<i32>,
    pub batch_no: Option<String>,
    pub warehouse_id: Option<i32>,
    pub location: Option<String>,
    pub status: Option<String>,
    pub quality_grade: Option<String>,
    pub remarks: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockInRequest {
    pub warehouse_id: i32,
    pub location: Option<String>,
    pub weight_kg: f64,
    pub length_m: f64,
    pub quality_grade: Option<String>,
    pub remarks: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockOutRequest {
    pub weight_kg: Option<f64>,
    pub length_m: Option<f64>,
    pub remarks: Option<String>,
}

pub struct GreigeFabricService;

impl GreigeFabricService {
    pub async fn list(query: GreigeFabricQuery) -> Result<Vec<GreigeFabric>, String> {
        let mut params = Vec::new();
        if let Some(page) = query.page {
            params.push(format!("page={}", page));
        }
        if let Some(page_size) = query.page_size {
            params.push(format!("page_size={}", page_size));
        }
        if let Some(fabric_no) = &query.fabric_no {
            params.push(format!("fabric_no={}", fabric_no));
        }
        if let Some(fabric_name) = &query.fabric_name {
            params.push(format!("fabric_name={}", fabric_name));
        }
        if let Some(fabric_type) = &query.fabric_type {
            params.push(format!("fabric_type={}", fabric_type));
        }
        if let Some(supplier_id) = query.supplier_id {
            params.push(format!("supplier_id={}", supplier_id));
        }
        if let Some(warehouse_id) = query.warehouse_id {
            params.push(format!("warehouse_id={}", warehouse_id));
        }
        if let Some(status) = &query.status {
            params.push(format!("status={}", status));
        }

        let url = format!("/api/v1/erp/greige-fabric?{}", params.join("&"));
        ApiService::get(&url).await
    }

    pub async fn get(id: i32) -> Result<GreigeFabric, String> {
        let url = format!("/api/v1/erp/greige-fabric/{}", id);
        ApiService::get(&url).await
    }

    pub async fn create(req: CreateGreigeFabricRequest) -> Result<GreigeFabric, String> {
        let url = "/api/v1/erp/greige-fabric";
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败: {}", e))?;
        ApiService::post(url, &body).await
    }

    pub async fn update(id: i32, req: UpdateGreigeFabricRequest) -> Result<GreigeFabric, String> {
        let url = format!("/api/v1/erp/greige-fabric/{}", id);
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败: {}", e))?;
        ApiService::put(&url, &body).await
    }

    pub async fn delete(id: i32) -> Result<(), String> {
        let url = format!("/api/v1/erp/greige-fabric/{}", id);
        ApiService::delete(&url).await
    }

    pub async fn stock_in(id: i32, req: StockInRequest) -> Result<GreigeFabric, String> {
        let url = format!("/api/v1/erp/greige-fabric/{}/stock-in", id);
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败: {}", e))?;
        ApiService::post(&url, &body).await
    }

    pub async fn stock_out(id: i32, req: StockOutRequest) -> Result<GreigeFabric, String> {
        let url = format!("/api/v1/erp/greige-fabric/{}/stock-out", id);
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败: {}", e))?;
        ApiService::post(&url, &body).await
    }

    pub async fn get_by_supplier(supplier_id: i32) -> Result<Vec<GreigeFabric>, String> {
        let url = format!("/api/v1/erp/greige-fabric/by-supplier/{}", supplier_id);
        ApiService::get(&url).await
    }
}
