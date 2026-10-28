//! 纺织品安全主数据处理器（检测项目、限值判定表维护通道、出口市场要求登记）
//!
//! 判据与词表校验在 `services/textile_safety_master_service.rs`；本文件只拆装请求。
//! 报价与订单侧消费出口要求走 GET /textile-export-requirements 只读键。
use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::textile_safety_master_service::{
    ExportRequirementReq, LimitRuleReq, TestItemReq, TextileSafetyMasterService,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    Json,
    extract::{Path, Query, State},
};
use rust_decimal::Decimal;
use serde::Deserialize;

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Deserialize)]
pub struct TestItemQuery {
    basis: Option<String>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Deserialize)]
pub struct LimitRuleQuery {
    item_id: Option<i32>,
    safety_category: Option<String>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Deserialize)]
pub struct RequirementQuery {
    market: Option<String>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Deserialize)]
pub struct TestItemBody {
    item_code: String,
    item_name: String,
    item_kind: String,
    basis_standard: String,
    #[serde(default = "default_check_kind")]
    check_kind: String,
    method_standard: Option<String>,
    unit: Option<String>,
    #[serde(default)]
    description: String,
}

fn default_check_kind() -> String {
    crate::models::status::textile_check_kind::MEASURABLE.to_string()
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Deserialize)]
pub struct LimitRuleBody {
    pub item_code: String,
    pub safety_category: String,
    pub product_form: String,
    #[serde(default = "default_tag")]
    pub condition_tag: String,
    pub comparator: String,
    #[serde(default)]
    pub limit_low: Option<Decimal>,
    #[serde(default)]
    pub limit_high: Option<Decimal>,
    #[serde(default)]
    pub clause_ref: String,
    #[serde(default)]
    pub remark: String,
}

fn default_tag() -> String {
    crate::models::status::textile_limit_scope::TAG_NONE.to_string()
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Deserialize)]
pub struct RequirementBody {
    pub requirement_code: String,
    pub market: String,
    pub requirement_type: String,
    pub oeko_tex_class: Option<String>,
    #[serde(default)]
    pub standard_ref: String,
    pub detail: String,
    #[serde(default = "default_mandatory")]
    pub mandatory: bool,
}

fn default_mandatory() -> bool {
    true
}

// ===== 检测项目主数据 =====

pub async fn list_test_items(
    State(state): State<AppState>,
    Query(q): Query<TestItemQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = TextileSafetyMasterService::new(state.db.clone());
    let items = service.list_test_items(q.basis.as_deref()).await?;
    Ok(Json(ApiResponse::success(
        serde_json::json!({ "items": items }),
    )))
}

pub async fn create_test_item(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(body): Json<TestItemBody>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = TextileSafetyMasterService::new(state.db.clone());
    let saved = service
        .create_test_item(TestItemReq {
            item_code: body.item_code,
            item_name: body.item_name,
            item_kind: body.item_kind,
            basis_standard: body.basis_standard,
            check_kind: body.check_kind,
            method_standard: body.method_standard,
            unit: body.unit,
            description: body.description,
        })
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(saved)?)))
}

pub async fn update_test_item(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(body): Json<TestItemBody>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = TextileSafetyMasterService::new(state.db.clone());
    let saved = service
        .update_test_item(
            id,
            TestItemReq {
                item_code: body.item_code,
                item_name: body.item_name,
                item_kind: body.item_kind,
                basis_standard: body.basis_standard,
                check_kind: body.check_kind,
                method_standard: body.method_standard,
                unit: body.unit,
                description: body.description,
            },
        )
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(saved)?)))
}

pub async fn delete_test_item(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = TextileSafetyMasterService::new(state.db.clone());
    service.delete_test_item(id).await?;
    tracing::info!(
        item_id = id,
        operator = auth.user_id,
        "检测项目主数据已删除"
    );
    Ok(Json(ApiResponse::success(
        serde_json::json!({ "deleted": id }),
    )))
}

// ===== 限值判定表 =====

pub async fn list_limit_rules(
    State(state): State<AppState>,
    Query(q): Query<LimitRuleQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = TextileSafetyMasterService::new(state.db.clone());
    let rows = service
        .list_limit_rules(q.item_id, q.safety_category.as_deref())
        .await?;
    Ok(Json(ApiResponse::success(
        serde_json::json!({ "rules": rows }),
    )))
}

pub async fn create_limit_rule(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(body): Json<LimitRuleBody>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = TextileSafetyMasterService::new(state.db.clone());
    let saved = service
        .create_limit_rule(LimitRuleReq {
            item_code: body.item_code,
            safety_category: body.safety_category,
            product_form: body.product_form,
            condition_tag: body.condition_tag,
            comparator: body.comparator,
            limit_low: body.limit_low,
            limit_high: body.limit_high,
            clause_ref: body.clause_ref,
            remark: body.remark,
        })
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(saved)?)))
}

pub async fn update_limit_rule(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(body): Json<LimitRuleBody>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = TextileSafetyMasterService::new(state.db.clone());
    let saved = service
        .update_limit_rule(
            id,
            LimitRuleReq {
                item_code: body.item_code,
                safety_category: body.safety_category,
                product_form: body.product_form,
                condition_tag: body.condition_tag,
                comparator: body.comparator,
                limit_low: body.limit_low,
                limit_high: body.limit_high,
                clause_ref: body.clause_ref,
                remark: body.remark,
            },
        )
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(saved)?)))
}

pub async fn delete_limit_rule(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = TextileSafetyMasterService::new(state.db.clone());
    service.delete_limit_rule(id).await?;
    tracing::info!(rule_id = id, operator = auth.user_id, "限值判定表行已删除");
    Ok(Json(ApiResponse::success(
        serde_json::json!({ "deleted": id }),
    )))
}

// ===== 出口市场要求登记 =====

pub async fn list_export_requirements(
    State(state): State<AppState>,
    Query(q): Query<RequirementQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = TextileSafetyMasterService::new(state.db.clone());
    let rows = service
        .list_export_requirements(q.market.as_deref())
        .await?;
    Ok(Json(ApiResponse::success(
        serde_json::json!({ "requirements": rows }),
    )))
}

pub async fn create_export_requirement(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(body): Json<RequirementBody>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = TextileSafetyMasterService::new(state.db.clone());
    let saved = service
        .create_export_requirement(ExportRequirementReq {
            requirement_code: body.requirement_code,
            market: body.market,
            requirement_type: body.requirement_type,
            oeko_tex_class: body.oeko_tex_class,
            standard_ref: body.standard_ref,
            detail: body.detail,
            mandatory: body.mandatory,
        })
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(saved)?)))
}

pub async fn update_export_requirement(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(body): Json<RequirementBody>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = TextileSafetyMasterService::new(state.db.clone());
    let saved = service
        .update_export_requirement(
            id,
            ExportRequirementReq {
                requirement_code: body.requirement_code,
                market: body.market,
                requirement_type: body.requirement_type,
                oeko_tex_class: body.oeko_tex_class,
                standard_ref: body.standard_ref,
                detail: body.detail,
                mandatory: body.mandatory,
            },
        )
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(saved)?)))
}

pub async fn delete_export_requirement(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = TextileSafetyMasterService::new(state.db.clone());
    service.delete_export_requirement(id).await?;
    tracing::info!(
        requirement_id = id,
        operator = auth.user_id,
        "出口市场要求登记已删除"
    );
    Ok(Json(ApiResponse::success(
        serde_json::json!({ "deleted": id }),
    )))
}
