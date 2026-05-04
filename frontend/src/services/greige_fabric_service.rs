//! 坯布管理服务

use crate::models::api_response::ApiResponse;
use crate::models::greige_fabric::{
    CreateGreigeFabricRequest, GreigeFabric, GreigeFabricListResponse, GreigeFabricQuery,
    StockInRequest, StockOutRequest, UpdateGreigeFabricRequest,
};
use crate::services::api::ApiService;
use crate::services::crud_service::CrudService;

pub struct GreigeFabricService;

impl CrudService for GreigeFabricService {
    type Model = GreigeFabric;
    type ListResponse = GreigeFabricListResponse;
    type CreateRequest = CreateGreigeFabricRequest;
    type UpdateRequest = UpdateGreigeFabricRequest;

    fn base_path() -> &'static str {
        "/greige-fabric"
    }
}


impl GreigeFabricService {
    pub async fn list(query: GreigeFabricQuery) -> Result<GreigeFabricListResponse, String> {
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

        let url = if params.is_empty() {
            String::from("/greige-fabric")
        } else {
            format!("/greige-fabric?{}", params.join("&"))
        };
        let response: GreigeFabricListResponse = ApiService::get(&url).await?;
        Ok(response)
    }

    pub async fn stock_in(id: i32, req: StockInRequest) -> Result<GreigeFabric, String> {
        let response: GreigeFabric = ApiService::post(&format!("/greige-fabric/{}/stock-in", id), &req).await?;
        Ok(response)
    }

    pub async fn stock_out(id: i32, req: StockOutRequest) -> Result<GreigeFabric, String> {
        let response: GreigeFabric = ApiService::post(&format!("/greige-fabric/{}/stock-out", id), &req).await?;
        Ok(response)
    }
}
