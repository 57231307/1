//! 供应商资质到期预警域路由
//!
//! 路由注册形态对齐劳动合同（`routes/labor_contract.rs`）：只写相对路径，
//! 权限键由 URL 段推导（resource=`supplier-qualifications`，scan 端点为 POST，
//! 与 `/labor-contracts/scan-expiry-warnings` 同一口径）；按需触发、不新建 job。

use crate::container::AppState;
use crate::handlers::supplier_qualification_gate_handler;
use axum::{Router, routing::post};

/// 供应商资质到期预警路由（path 前缀 /supplier-qualifications）
pub fn supplier_qualifications() -> Router<AppState> {
    Router::new().route(
        "/supplier-qualifications/scan-expiry-warnings",
        post(supplier_qualification_gate_handler::scan_expiry_warnings),
    )
}

/// 供应商资质到期预警域统一入口
pub fn routes() -> Router<AppState> {
    Router::new().merge(supplier_qualifications())
}
