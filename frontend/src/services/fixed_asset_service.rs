//! 固定资产服务模块

use crate::services::api::ApiService;

/// 固定资产数据模型
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct FixedAsset {
    pub id: i32,
    pub asset_no: String,
    pub asset_name: String,
    pub asset_category: Option<String>,
    pub specification: Option<String>,
    pub location: Option<String>,
    pub original_value: f64,
    pub current_value: Option<f64>,
    pub useful_life: i32,
    pub depreciation_method: Option<String>,
    pub purchase_date: String,
    pub put_in_date: String,
    pub status: Option<String>,
    pub supplier_id: Option<i32>,
    pub remark: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 资产列表响应
#[derive(Debug, Clone, serde::Deserialize)]
pub struct AssetListResponse {
    pub data: Vec<FixedAsset>,
}

/// 资产查询参数
#[derive(Debug, Clone, serde::Serialize)]
pub struct AssetQueryParams {
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub asset_category: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// 创建资产请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct CreateAssetRequest {
    pub asset_no: String,
    pub asset_name: String,
    pub asset_category: Option<String>,
    pub specification: Option<String>,
    pub location: Option<String>,
    pub original_value: f64,
    pub useful_life: i32,
    pub depreciation_method: Option<String>,
    pub purchase_date: String,
    pub put_in_date: String,
    pub supplier_id: Option<i32>,
    pub remark: Option<String>,
}

/// 计提折旧请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct DepreciateRequest {
    pub period: String,
}

/// 资产处置请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct DisposalRequest {
    pub disposal_type: String,
    pub disposal_value: f64,
    pub disposal_date: String,
    pub reason: String,
    pub buyer_info: Option<String>,
}

/// 固定资产服务
pub struct FixedAssetService;

impl FixedAssetService {
    /// 获取资产列表
    pub async fn list_assets(params: AssetQueryParams) -> Result<Vec<FixedAsset>, String> {
        let mut url = String::from("/assets");
        let mut query_params = Vec::new();

        if let Some(keyword) = &params.keyword {
            query_params.push(format!("keyword={}", keyword));
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

        ApiService::get::<AssetListResponse>(&url).await.map(|r| r.data)
    }

    /// 获取资产详情
    pub async fn get_asset(id: i32) -> Result<FixedAsset, String> {
        ApiService::get::<FixedAsset>(&format!("/assets/{}", id)).await
    }

    /// 创建资产
    pub async fn create_asset(req: CreateAssetRequest) -> Result<FixedAsset, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/assets", &payload).await
    }

    /// 计提折旧
    pub async fn depreciate_asset(id: i32, req: DepreciateRequest) -> Result<String, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        let _: serde_json::Value = ApiService::post(&format!("/assets/{}/depreciate", id), &payload).await?;
        Ok(format!("资产 {} 折旧计提成功", id))
    }

    /// 处置资产
    pub async fn dispose_asset(id: i32, req: DisposalRequest) -> Result<String, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        let _: serde_json::Value = ApiService::post(&format!("/assets/{}/dispose", id), &payload).await?;
        Ok(format!("资产 {} 处置成功", id))
    }

    /// 删除资产
    pub async fn delete_asset(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/assets/{}", id)).await
    }
}