//! 固定资产服务模块

use crate::models::api_response::ApiResponse;
use crate::models::fixed_asset::{
    AssetListResponse, AssetQueryParams, CreateAssetRequest, DepreciateRequest, DisposalRequest,
    FixedAsset,
};
use crate::services::api::ApiService;

/// 固定资产服务
pub struct FixedAssetService;

impl FixedAssetService {
    /// 获取资产列表
    pub async fn list_assets(params: AssetQueryParams) -> Result<AssetListResponse, String> {
        let mut url = String::from("/assets");
        let mut query_params = Vec::new();

        if let Some(keyword) = &params.keyword {
            query_params.push(format!("keyword={}", urlencoding::encode(keyword)));
        }
        if let Some(status) = &params.status {
            query_params.push(format!("status={}", status));
        }
        if let Some(category) = &params.asset_category {
            query_params.push(format!("asset_category={}", category));
        }
        if let Some(page) = params.page {
            query_params.push(format!("page={}", page));
        }
        if let Some(page_size) = params.page_size {
            query_params.push(format!("page_size={}", page_size));
        }

        if !query_params.is_empty() {
            url = format!("{}?{}", url, query_params.join("&"));
        }

        let response: ApiResponse<AssetListResponse> = ApiService::get(&url).await?;
        response.into_result()
    }

    /// 获取资产详情
    pub async fn get_asset(id: i32) -> Result<FixedAsset, String> {
        let response: ApiResponse<FixedAsset> = ApiService::get(&format!("/assets/{}", id)).await?;
        response.into_result()
    }

    /// 创建资产
    pub async fn create_asset(req: CreateAssetRequest) -> Result<FixedAsset, String> {
        let response: ApiResponse<FixedAsset> = ApiService::post("/assets", &req).await?;
        response.into_result()
    }

    /// 计提折旧
    pub async fn depreciate_asset(id: i32, req: DepreciateRequest) -> Result<FixedAsset, String> {
        let response: ApiResponse<FixedAsset> =
            ApiService::post(&format!("/assets/{}/depreciate", id), &req).await?;
        response.into_result()
    }

    /// 处置资产
    pub async fn dispose_asset(id: i32, req: DisposalRequest) -> Result<FixedAsset, String> {
        let response: ApiResponse<FixedAsset> =
            ApiService::post(&format!("/assets/{}/dispose", id), &req).await?;
        response.into_result()
    }

    /// 删除资产
    pub async fn delete_asset(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/assets/{}", id)).await
    }
}
