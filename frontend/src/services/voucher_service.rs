//! 凭证管理服务
//!
//! 与后端凭证API交互

use crate::models::voucher::{CreateVoucherRequest, Voucher, VoucherItemRequest, VoucherListResponse, VoucherQueryParams};
use crate::services::api::ApiService;

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

        let url = format!("/vouchers{}", query_string);
        ApiService::get::<VoucherListResponse>(&url).await
    }

    /// 获取凭证详情
    pub async fn get_voucher(id: i32) -> Result<Voucher, String> {
        ApiService::get::<Voucher>(&format!("/vouchers/{}", id)).await
    }

    /// 创建凭证
    pub async fn create_voucher(req: CreateVoucherRequest) -> Result<Voucher, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/vouchers", &payload).await
    }

    /// 提交凭证
    pub async fn submit_voucher(id: i32) -> Result<serde_json::Value, String> {
        ApiService::post(&format!("/vouchers/{}/submit", id), &serde_json::json!({})).await
    }

    /// 审核凭证
    pub async fn review_voucher(id: i32) -> Result<serde_json::Value, String> {
        ApiService::post(&format!("/vouchers/{}/review", id), &serde_json::json!({})).await
    }

    /// 过账凭证
    pub async fn post_voucher(id: i32) -> Result<serde_json::Value, String> {
        ApiService::post(&format!("/vouchers/{}/post", id), &serde_json::json!({})).await
    }
}
