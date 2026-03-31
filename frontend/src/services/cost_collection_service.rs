//! 成本归集服务
//! 提供成本归集数据的查询、创建等功能

use crate::models::cost_collection::*;
use crate::services::api::ApiService;

pub struct CostCollectionService;

impl CostCollectionService {
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

        ApiService::get(&format!("/cost-collections{}", query_string)).await
    }

    pub async fn create(req: CreateCostCollectionRequest) -> Result<CostCollection, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        ApiService::post("/cost-collections", &body).await
    }
}
