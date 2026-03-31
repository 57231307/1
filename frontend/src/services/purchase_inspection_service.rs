//! 采购检验服务 API 客户端
//! 提供采购检验相关的 API 调用方法

use crate::services::api::ApiService;
use serde::{Deserialize, Serialize};

/// 采购检验单
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseInspection {
    /// 检验单 ID
    pub id: i32,
    /// 检验单号
    pub inspection_no: String,
    /// 采购订单 ID
    pub purchase_order_id: Option<i32>,
    /// 供应商 ID
    pub supplier_id: i32,
    /// 供应商名称
    pub supplier_name: Option<String>,
    /// 检验日期
    pub inspection_date: String,
    /// 质检员 ID
    pub inspector_id: Option<i32>,
    /// 质检员名称
    pub inspector_name: Option<String>,
    /// 检验结果
    pub result: String,
    /// 合格数量
    pub qualified_quantity: String,
    /// 不合格数量
    pub unqualified_quantity: String,
    /// 不合格原因
    pub unqualified_reason: Option<String>,
    /// 备注
    pub remarks: Option<String>,
    /// 创建人
    pub created_by: i32,
    /// 创建时间
    pub created_at: Option<String>,
    /// 更新时间
    pub updated_at: Option<String>,
}

/// 创建采购检验单请求
#[derive(Debug, Clone, Serialize)]
pub struct CreatePurchaseInspectionRequest {
    /// 入库单 ID
    pub receipt_id: i32,
    /// 采购订单 ID
    pub order_id: Option<i32>,
    /// 供应商 ID
    pub supplier_id: i32,
    /// 检验日期
    pub inspection_date: String,
    /// 质检员 ID
    pub inspector_id: Option<i32>,
    /// 检验类型
    pub inspection_type: Option<String>,
    /// 备注
    pub notes: Option<String>,
}

/// 更新采购检验单请求
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct UpdatePurchaseInspectionRequest {
    /// 抽样数量
    pub sample_size: Option<i32>,
    /// 缺陷描述
    pub defect_description: Option<String>,
    /// 备注
    pub notes: Option<String>,
}

/// 完成检验请求
#[derive(Debug, Clone, Serialize)]
pub struct CompleteInspectionRequest {
    /// 合格数量
    pub pass_quantity: String,
    /// 不合格数量
    pub reject_quantity: String,
    /// 检验结果
    pub inspection_result: String,
}

/// 检验单查询参数
#[derive(Debug, Clone, Serialize)]
pub struct PurchaseInspectionQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub supplier_id: Option<i32>,
}

/// 检验单列表响应
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct InspectionListResponse {
    pub items: Vec<PurchaseInspection>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

pub struct PurchaseInspectionService;

impl PurchaseInspectionService {
    /// 获取检验单列表
    pub async fn list(query: PurchaseInspectionQuery) -> Result<Vec<PurchaseInspection>, String> {
        let mut params = Vec::new();
        if let Some(page) = query.page {
            params.push(format!("page={}", page));
        }
        if let Some(page_size) = query.page_size {
            params.push(format!("page_size={}", page_size));
        }
        if let Some(ref status) = query.status {
            params.push(format!("status={}", status));
        }
        if let Some(supplier_id) = query.supplier_id {
            params.push(format!("supplier_id={}", supplier_id));
        }

        let query_string = if params.is_empty() {
            String::new()
        } else {
            format!("?{}", params.join("&"))
        };

        let response: serde_json::Value = ApiService::get(&format!("/purchase-inspections{}", query_string)).await?;

        // 从响应中提取数据
        if let Some(data) = response.get("data") {
            // 尝试解析为包含 items 字段的响应
            if let Some(items) = data.get("items").and_then(|v| v.as_array()) {
                let inspections: Vec<PurchaseInspection> = items
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect();
                return Ok(inspections);
            }
            // 尝试直接解析为数组
            if let Some(arr) = data.as_array() {
                let inspections: Vec<PurchaseInspection> = arr
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect();
                return Ok(inspections);
            }
        }
        Ok(Vec::new())
    }

    /// 获取检验单详情
    pub async fn get(id: i32) -> Result<PurchaseInspection, String> {
        let response: serde_json::Value = ApiService::get(&format!("/purchase-inspections/{}", id)).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "获取采购检验单详情失败".to_string())
    }

    /// 创建采购检验单
    pub async fn create(req: CreatePurchaseInspectionRequest) -> Result<PurchaseInspection, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::post("/purchase-inspections", &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "创建采购检验单失败".to_string())
    }

    /// 更新采购检验单
    #[allow(dead_code)]
    pub async fn update(id: i32, req: UpdatePurchaseInspectionRequest) -> Result<PurchaseInspection, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::put(&format!("/purchase-inspections/{}", id), &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "更新采购检验单失败".to_string())
    }

    /// 完成采购检验单
    pub async fn complete(id: i32, req: CompleteInspectionRequest) -> Result<PurchaseInspection, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::post(&format!("/purchase-inspections/{}/complete", id), &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "完成采购检验单失败".to_string())
    }
}