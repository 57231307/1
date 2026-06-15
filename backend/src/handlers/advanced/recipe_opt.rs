//! 染色工艺参数智能推荐 handler
//!
//! 端点：`POST /advanced/ai/recipe-optimization`
//!
//! 请求体：
//! ```json
//! {
//!   "color_no": "BL-301",
//!   "fabric_type": "棉",
//!   "dye_type": "活性染料",
//!   "color_name": "宝蓝",
//!   "k": 5
//! }
//! ```
//!
//! 响应：基于历史染配的 k-NN 相似度匹配结果；当历史不足 3 条时退化到
//! 内置典型参数表（80°C / 45min / pH 6.0 / 浴比 1:8）。

use axum::{extract::State, Json};
use serde::Deserialize;

use crate::services::ai::recipe_opt::{RecipeOptRequest, RecipeOptResponse};
use crate::services::ai::AiAnalysisService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 工艺优化请求体
#[derive(Debug, Clone, Deserialize)]
pub struct RecipeOptimizationRequest {
    /// 色号（如 "BL-301"），必填
    pub color_no: String,
    /// 布类（棉 / 涤纶 / 丝绸 / 羊毛 等），必填
    pub fabric_type: String,
    /// 染料类型（可选）
    pub dye_type: Option<String>,
    /// 颜色名称（可选）
    pub color_name: Option<String>,
    /// k-NN 近邻数（可选，默认 5；传 0 时强制走退化路径）
    pub k: Option<usize>,
}

/// 工艺优化 API 响应（直接复用 service 层 DTO）
pub type RecipeOptimizationResponse = RecipeOptResponse;

/// 处理工艺优化推荐请求
///
/// 校验色号与布类非空后，调用 `AiAnalysisService::optimize_recipe`
/// 完成 k-NN 匹配 + 加权聚合 + 退化路径处理。
pub async fn optimize_recipe(
    State(state): State<AppState>,
    Json(payload): Json<RecipeOptimizationRequest>,
) -> Result<Json<ApiResponse<RecipeOptimizationResponse>>, AppError> {
    // 1. 基础校验
    if payload.color_no.trim().is_empty() {
        return Err(AppError::validation("色号 color_no 不能为空"));
    }
    if payload.fabric_type.trim().is_empty() {
        return Err(AppError::validation("布类 fabric_type 不能为空"));
    }

    // 2. 调用 service
    let service = AiAnalysisService::new(state.db);
    let request = RecipeOptRequest {
        color_no: payload.color_no.trim().to_string(),
        fabric_type: payload.fabric_type.trim().to_string(),
        dye_type: payload
            .dye_type
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        color_name: payload
            .color_name
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        k: payload.k,
    };

    let response = service.optimize_recipe(request).await?;
    Ok(Json(ApiResponse::success(response)))
}

/// 单元占位，确保文件被视为模块；实际逻辑由 axum 路由调度
#[cfg(test)]
mod tests {
    use super::*;

    /// 校验：请求体结构正确构造
    #[test]
    fn test_request_struct_construction() {
        let req = RecipeOptimizationRequest {
            color_no: "BL-301".to_string(),
            fabric_type: "棉".to_string(),
            dye_type: Some("活性染料".to_string()),
            color_name: Some("宝蓝".to_string()),
            k: Some(5),
        };
        assert!(!req.color_no.is_empty());
        assert!(!req.fabric_type.is_empty());
        assert_eq!(req.k, Some(5));
    }
}
