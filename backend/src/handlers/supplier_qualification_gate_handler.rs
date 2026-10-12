//! 供应商资质过期门控域 handler（到期预警扫描端点）
//!
//! 分层对齐劳动合同 `labor_contract_handler::scan_expiry_warnings`：
//! handler 只做装配与显式日志（成功/失败都打），业务判定全部在
//! `services::supplier_qualification_gate`。例外放行没有独立端点——
//! 放行原因随采购订单创建请求（`CreatePurchaseOrderRequest.qualification_waiver_reason`）
//! 在同一事务内校验权限并落审计，避免「先批条后下单」的两段式 TOCTOU。

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::supplier_qualification_gate::SupplierQualificationGate;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{Json, extract::State};

/// 扫描供应商资质到期预警（已过期 + 进入预警档位的即将过期）
pub async fn scan_expiry_warnings(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let gate = SupplierQualificationGate::new(state.db.clone());
    match gate.scan_expiry_warnings().await {
        Ok(warnings) => {
            tracing::info!(
                warning_count = warnings.len(),
                "资质到期预警扫描成功（POST /supplier-qualifications/scan-expiry-warnings）"
            );
            Ok(Json(ApiResponse::success(serde_json::to_value(warnings)?)))
        }
        Err(e) => {
            tracing::error!(error = %e, "资质到期预警扫描失败");
            Err(e)
        }
    }
}
