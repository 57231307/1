//! 单据号查重
//!
//! 前端自动生成单据号后调用本接口确认唯一性，仅当不存在时才使用；
//! 存在时前端重新生成。数据层由各表单据号列的 UNIQUE 约束兜底。

use axum::extract::{Query, State};
use axum::Json;
use sea_orm::ColumnTrait;
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;
use serde::Deserialize;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::utils::error::AppError;
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
    let db = &*state.db;
    let no = q.no.trim().to_string();
    if no.is_empty() {
        return Err(AppError::bad_request("单据号不能为空"));
    }

    let exists = match q.doc_type.as_str() {
        "outsourcing_order" => crate::models::outsourcing_order::Entity::find()
            .filter(crate::models::outsourcing_order::Column::OrderNo.eq(&no))
            .one(db)
            .await?
            .is_some(),
        "dye_batch" => crate::models::dye_batch::Entity::find()
            .filter(crate::models::dye_batch::Column::BatchNo.eq(&no))
            .one(db)
            .await?
            .is_some(),
        "dye_recipe" => crate::models::dye_recipe::Entity::find()
            .filter(crate::models::dye_recipe::Column::RecipeNo.eq(&no))
            .one(db)
            .await?
            .is_some(),
        "sales_contract" => crate::models::sales_contract::Entity::find()
            .filter(crate::models::sales_contract::Column::ContractNo.eq(&no))
            .one(db)
            .await?
            .is_some(),
        "purchase_contract" => crate::models::purchase_contract::Entity::find()
            .filter(crate::models::purchase_contract::Column::ContractNo.eq(&no))
            .one(db)
            .await?
            .is_some(),
        "labor_contract" => crate::models::labor_contract::Entity::find()
            .filter(crate::models::labor_contract::Column::ContractNo.eq(&no))
            .one(db)
            .await?
            .is_some(),
        "finance_invoice" => crate::models::finance_invoice::Entity::find()
            .filter(crate::models::finance_invoice::Column::InvoiceNo.eq(&no))
            .one(db)
            .await?
            .is_some(),
        other => {
            return Err(AppError::bad_request(format!("未知的单据类型：{}", other)));
        }
    };

    Ok(Json(ApiResponse::success(exists)))
}
