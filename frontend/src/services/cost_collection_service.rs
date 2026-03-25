//! 成本归集服务
//! 提供成本归集数据的查询、创建等功能

use crate::services::api::ApiService;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 成本归集模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostCollection {
    pub id: i32,
    pub collection_no: String,
    pub collection_date: String,
    pub cost_object_type: Option<String>,
    pub cost_object_id: Option<i32>,
    pub cost_object_no: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub workshop: Option<String>,
    pub direct_material: Value,
    pub direct_labor: Value,
    pub manufacturing_overhead: Value,
    pub processing_fee: Value,
    pub dyeing_fee: Value,
    pub output_quantity_meters: Option<Value>,
    pub output_quantity_kg: Option<Value>,
    pub created_by: i32,
    pub created_at: String,
    pub updated_at: Option<String>,
}

/// 成本归集查询参数
#[derive(Debug, Clone, Serialize)]
pub struct CostCollectionQuery {
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 创建成本归集请求
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCostCollectionRequest {
    pub collection_date: String,
    pub cost_object_type: Option<String>,
    pub cost_object_id: Option<i32>,
    pub cost_object_no: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub workshop: Option<String>,
    pub direct_material: String,
    pub direct_labor: String,
    pub manufacturing_overhead: String,
    pub processing_fee: String,
    pub dyeing_fee: String,
    pub output_quantity_meters: Option<String>,
    pub output_quantity_kg: Option<String>,
}

/// 成本归集服务
pub struct CostCollectionService;

impl CostCollectionService {
    /// 查询成本归集列表
    pub async fn list(query: CostCollectionQuery) -> Result<Vec<CostCollection>, String> {
        let mut params = Vec::new();
        if let Some(ref batch_no) = query.batch_no {
            if !batch_no.is_empty() {
                params.push(format!("batch_no={}", batch_no));
            }
        }
        if let Some(ref color_no) = query.color_no {
            if !color_no.is_empty() {
                params.push(format!("color_no={}", color_no));
            }
        }
        if let Some(page) = query.page {
            params.push(format!("page={}", page));
        }
        if let Some(page_size) = query.page_size {
            params.push(format!("page_size={}", page_size));
        }

        let query_string = if params.is_empty() {
            String::new()
        } else {
            format!("?{}", params.join("&"))
        };

        let response: Value = ApiService::get(&format!("/cost-collections{}", query_string)).await?;

        if let Some(data) = response.get("data").and_then(|v| v.as_array()) {
            let collections: Vec<CostCollection> = data
                .iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect();
            Ok(collections)
        } else {
            Ok(Vec::new())
        }
    }

    /// 创建成本归集
    pub async fn create(req: CreateCostCollectionRequest) -> Result<CostCollection, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: Value = ApiService::post("/cost-collections", &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "创建成本归集失败".to_string())
    }
}