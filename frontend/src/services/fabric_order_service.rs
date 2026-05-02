//! 面料订单服务 API 客户端
//! 提供面料订单相关的 API 调用方法

use crate::models::fabric_order::*;
use crate::services::api::ApiService;
use crate::services::crud_service::CrudService;

pub struct FabricOrderService;

impl CrudService for FabricOrderService {
    type Model = FabricOrder;
    type ListResponse = Vec<FabricOrder>;
    type CreateRequest = CreateFabricOrderRequest;
    type UpdateRequest = UpdateFabricOrderRequest;

    fn base_path() -> &'static str {
        "/sales/fabric-orders"
    }
}


impl FabricOrderService {
    pub async fn list(query: FabricOrderQuery) -> Result<Vec<FabricOrder>, String> {
        let mut params = Vec::new();
        if let Some(page) = query.page {
            params.push(format!("page={}", page));
        }
        if let Some(page_size) = query.page_size {
            params.push(format!("page_size={}", page_size));
        }
        if let Some(customer_id) = query.customer_id {
            params.push(format!("customer_id={}", customer_id));
        }
        if let Some(ref order_no) = query.order_no {
            params.push(format!("order_no={}", order_no));
        }
        if let Some(ref status) = query.status {
            params.push(format!("status={}", status));
        }
        if let Some(ref batch_no) = query.batch_no {
            params.push(format!("batch_no={}", batch_no));
        }
        if let Some(ref color_no) = query.color_no {
            params.push(format!("color_no={}", color_no));
        }

        let query_string = if params.is_empty() {
            String::new()
        } else {
            format!("?{}", params.join("&"))
        };

        ApiService::get(&format!("/sales/fabric-orders{}", query_string)).await
    }

    pub async fn approve(id: i32) -> Result<FabricOrder, String> {
        ApiService::post(&format!("/sales/fabric-orders/{}/approve", id), &serde_json::json!({})).await
    }
}
