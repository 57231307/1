//! 质量预测 handler
//!
//! 端点：`POST /advanced/ai/quality-prediction`
//!
//! 请求体：
//! ```json
//! {
//!   "product_id": 1,           // 可选：限定产品
//!   "inspection_type": "成品", // 可选：进货/过程/成品/出货
//!   "window_days": 90          // 可选：默认 90
//! }
//! ```
//!
//! 响应：基于历史 `quality_inspection_records` 的合格率 / 趋势 /
//! 风险评分 / 主要问题归因 / 建议措施 / 月度分段统计；
//! 当历史 < 5 条时回退到保守默认值（合格率 95% / 置信度 0.3）。

use axum::{extract::State, Json};

use crate::services::ai::quality_pred::{QualityPredRequest, QualityPredResponse};
use crate::services::ai::AiAnalysisService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 质量预测 API 响应（直接复用 service 层 DTO）
pub type QualityPredictionResponse = QualityPredResponse;

/// 处理质量预测请求
///
/// 标准化入参（去除空白、限制 window_days 范围）后调用
/// `AiAnalysisService::predict_quality` 完成历史聚合 + 趋势判定 +
/// 风险评分 + 退化路径处理。
pub async fn quality_prediction(
    State(state): State<AppState>,
    Json(payload): Json<QualityPredRequest>,
) -> Result<Json<ApiResponse<QualityPredictionResponse>>, AppError> {
    // 1. 入参标准化
    let window_days = payload.window_days.unwrap_or(90);
    if !(1..=365).contains(&window_days) {
        return Err(AppError::validation("时间窗口 window_days 必须在 1-365 之间"));
    }

    let inspection_type = payload
        .inspection_type
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    if let Some(ref t) = inspection_type {
        if t.chars().count() > 32 {
            return Err(AppError::validation("检验类型 inspection_type 长度不能超过 32"));
        }
    }

    // 2. 调用 service
    let service = AiAnalysisService::new(state.db);
    let request = QualityPredRequest {
        product_id: payload.product_id,
        inspection_type,
        window_days: Some(window_days),
    };

    let response = service.predict_quality(request).await?;
    Ok(Json(ApiResponse::success(response)))
}

/// 单元占位：校验请求体结构正确构造
#[cfg(test)]
mod tests {
    use super::*;

    /// 测试：请求体结构正确构造
    #[test]
    fn test_request_struct_construction() {
        let req = QualityPredRequest {
            product_id: Some(1),
            inspection_type: Some("成品".to_string()),
            window_days: Some(90),
        };
        assert_eq!(req.product_id, Some(1));
        assert_eq!(req.inspection_type.as_deref(), Some("成品"));
        assert_eq!(req.window_days, Some(90));
    }
}
