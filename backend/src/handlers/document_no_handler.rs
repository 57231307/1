//! 单据号查重
//!
//! 前端自动生成单据号后调用本接口确认唯一性，仅当不存在时才使用；
//! 存在时前端重新生成。数据层由各表单据号列的 UNIQUE 约束兜底。
//! doc_type 白名单注册表在 `utils::number_generator::is_document_no_taken`，
//! 新增需要编码的单据时必须在那里登记，本 handler 不再各自维护映射。

use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::utils::error::AppError;
use crate::utils::number_generator::is_document_no_taken;
use crate::utils::response::ApiResponse;

/// 单据号查重请求参数
#[derive(Debug, Deserialize)]
pub struct CheckDocNoQuery {
    /// 单据类型（映射到具体表与编号列）
    pub doc_type: String,
    /// 待检查的单据号
    pub no: String,
}

/// GET /api/v1/erp/document-no/check - 检查单据号是否已存在（true=已占用）
pub async fn check_doc_no(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(q): Query<CheckDocNoQuery>,
) -> Result<Json<ApiResponse<bool>>, AppError> {
    let no = q.no.trim().to_string();
    if no.is_empty() {
        return Err(AppError::bad_request("单据号不能为空"));
    }

    let exists = is_document_no_taken(&*state.db, &q.doc_type, &no).await?;

    Ok(Json(ApiResponse::success(exists)))
}
