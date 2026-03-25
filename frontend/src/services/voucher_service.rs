//! 凭证管理服务
//!
//! 与后端凭证API交互

use crate::services::api::ApiService;
use serde::{Deserialize, Serialize};
use serde_json::Number;

/// 凭证数据模型
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Voucher {
    pub id: i32,
    pub voucher_no: String,
    pub voucher_type: String,
    pub voucher_date: String,
    pub status: String,
    pub source_type: Option<String>,
    pub source_module: Option<String>,
    pub source_bill_id: Option<i32>,
    pub source_bill_no: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub total_debit: String,
    pub total_credit: String,
    pub creator_id: i32,
    pub creator_name: Option<String>,
    pub submitter_id: Option<i32>,
    pub submitter_name: Option<String>,
    pub reviewer_id: Option<i32>,
    pub reviewer_name: Option<String>,
    pub poster_id: Option<i32>,
    pub poster_name: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 凭证明细项
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VoucherItem {
    pub id: i32,
    pub voucher_id: i32,
    pub line_no: i32,
    pub subject_code: String,
    pub subject_name: String,
    pub debit: String,
    pub credit: String,
    pub summary: Option<String>,
    pub assist_batch_id: Option<i32>,
    pub assist_color_no_id: Option<i32>,
    pub quantity_meters: Option<String>,
    pub quantity_kg: Option<String>,
}

/// 凭证列表响应
#[derive(Debug, Clone, Deserialize)]
pub struct VoucherListResponse {
    pub data: Vec<Voucher>,
    pub total: u64,
}

/// 凭证查询参数
#[derive(Debug, Clone, Serialize)]
pub struct VoucherQueryParams {
    pub voucher_type: Option<String>,
    pub status: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 创建凭证请求
#[derive(Debug, Clone, Serialize)]
pub struct CreateVoucherRequest {
    pub voucher_type: String,
    pub voucher_date: String,
    pub source_type: Option<String>,
    pub source_module: Option<String>,
    pub source_bill_id: Option<i32>,
    pub source_bill_no: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub items: Vec<VoucherItemRequest>,
}

/// 凭证明细请求
#[derive(Debug, Clone, Serialize)]
pub struct VoucherItemRequest {
    pub line_no: i32,
    pub subject_code: String,
    pub subject_name: String,
    pub debit: Number,
    pub credit: Number,
    pub summary: Option<String>,
    pub assist_batch_id: Option<i32>,
    pub assist_color_no_id: Option<i32>,
    pub quantity_meters: Option<Number>,
    pub quantity_kg: Option<Number>,
}

/// 凭证服务
pub struct VoucherService;

impl VoucherService {
    /// 查询凭证列表
    pub async fn list_vouchers(params: VoucherQueryParams) -> Result<VoucherListResponse, String> {
        let mut query_parts = vec![];

        if let Some(ref vt) = params.voucher_type {
            query_parts.push(format!("voucher_type={}", vt));
        }
        if let Some(ref s) = params.status {
            query_parts.push(format!("status={}", s));
        }
        if let Some(ref sd) = params.start_date {
            query_parts.push(format!("start_date={}", sd));
        }
        if let Some(ref ed) = params.end_date {
            query_parts.push(format!("end_date={}", ed));
        }
        if let Some(ref bn) = params.batch_no {
            query_parts.push(format!("batch_no={}", bn));
        }
        if let Some(ref cn) = params.color_no {
            query_parts.push(format!("color_no={}", cn));
        }
        if let Some(p) = params.page {
            query_parts.push(format!("page={}", p));
        }
        if let Some(ps) = params.page_size {
            query_parts.push(format!("page_size={}", ps));
        }

        let query_string = if query_parts.is_empty() {
            String::new()
        } else {
            format!("?{}", query_parts.join("&"))
        };

        let url = format!("/api/v1/erp/vouchers{}", query_string);
        ApiService::get::<VoucherListResponse>(&url).await
    }

    /// 获取凭证详情
    pub async fn get_voucher(id: i32) -> Result<Voucher, String> {
        ApiService::get::<Voucher>(&format!("/api/v1/erp/vouchers/{}", id)).await
    }

    /// 创建凭证
    pub async fn create_voucher(req: CreateVoucherRequest) -> Result<Voucher, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/api/v1/erp/vouchers", &payload).await
    }

    /// 提交凭证
    pub async fn submit_voucher(id: i32) -> Result<serde_json::Value, String> {
        ApiService::post(&format!("/api/v1/erp/vouchers/{}/submit", id), &serde_json::json!({})).await
    }

    /// 审核凭证
    pub async fn review_voucher(id: i32) -> Result<serde_json::Value, String> {
        ApiService::post(&format!("/api/v1/erp/vouchers/{}/review", id), &serde_json::json!({})).await
    }

    /// 过账凭证
    pub async fn post_voucher(id: i32) -> Result<serde_json::Value, String> {
        ApiService::post(&format!("/api/v1/erp/vouchers/{}/post", id), &serde_json::json!({})).await
    }
}
