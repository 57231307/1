use serde::{de::DeserializeOwned, Serialize};
use crate::services::api::ApiService;

/// 通用的 CRUD 服务 Trait
pub trait CrudService {
    type Model: DeserializeOwned;
    type ListResponse: DeserializeOwned;
    type CreateRequest: Serialize;
    type UpdateRequest: Serialize;

    /// 资源的 API 基础路径，例如 "/departments"
    fn base_path() -> &'static str;

    async fn list() -> Result<Self::ListResponse, String> {
        ApiService::get::<Self::ListResponse>(Self::base_path()).await
    }

    /// 带查询参数的列表请求
    async fn list_with_query<Q: Serialize>(query: &Q) -> Result<Self::ListResponse, String> {
        // 由于 Yew/reqwest 没有内置非常方便的 query 序列化，我们可以简单地将 Q 序列化为 urlencoded
        let qs = serde_urlencoded::to_string(query).map_err(|e| format!("查询参数序列化失败: {}", e))?;
        let url = if qs.is_empty() {
            Self::base_path().to_string()
        } else {
            format!("{}?{}", Self::base_path(), qs)
        };
        ApiService::get::<Self::ListResponse>(&url).await
    }

    async fn get(id: i32) -> Result<Self::Model, String> {
        ApiService::get::<Self::Model>(&format!("{}/{}", Self::base_path(), id)).await
    }

    async fn create(req: Self::CreateRequest) -> Result<Self::Model, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post(Self::base_path(), &payload).await
    }

    async fn update(id: i32, req: Self::UpdateRequest) -> Result<Self::Model, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::put(&format!("{}/{}", Self::base_path(), id), &payload).await
    }

    async fn delete(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("{}/{}", Self::base_path(), id)).await
    }
}