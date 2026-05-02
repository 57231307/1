//! 批次管理服务 API 客户端
//! 提供批次管理相关的 API 调用方法

use crate::models::batch::*;
use crate::services::api::ApiService;
use crate::services::crud_service::CrudService;

pub struct BatchService;

impl CrudService for BatchService {
    type Model = Batch;
    type ListResponse = Vec<Batch>;
    type CreateRequest = CreateBatchRequest;
    type UpdateRequest = UpdateBatchRequest;

    fn base_path() -> &'static str {
        "/batches"
    }
}


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

        ApiService::get(&format!("/batches{}", query_string)).await
    }

    pub async fn transfer(id: i32, req: TransferBatchRequest) -> Result<Batch, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        ApiService::post(&format!("/batches/{}/transfer", id), &body).await
    }
}
