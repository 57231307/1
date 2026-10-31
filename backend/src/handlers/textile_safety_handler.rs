//! 纺织品安全送检单处理器（建单、要素修改、状态机、项目清单推导、童装核查登记、整单判定）
//!
//! 判据全部在 `services/textile_safety_submission_service.rs` 与
//! `services/textile_safety_limit_judge.rs`，本文件只做请求拆装与会话注入：
//! 词表白名单校验、状态机守卫、批次口径校验都不在 handler 复制第二套。
use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::status::export_market;
use crate::models::textile_export_market_requirement::{
    Column as ReqColumn, Entity as ExportRequirement,
};
use crate::services::textile_safety_report_service::{
    ReportCandidate, TextileSafetyReportService, select_adopted_reports,
};
use crate::services::textile_safety_submission_service::{
    ChildCheckUpsertReq, CreateSubmissionReq, SubmissionListParams, TextileSafetySubmissionService,
};
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Deserialize)]
pub struct ListQuery {
    product_id: Option<i32>,
    status: Option<String>,
    target_market: Option<String>,
    page: Option<u64>,
    page_size: Option<u64>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Deserialize)]
pub struct CreateSubmissionBody {
    submission_no: String,
    product_id: i32,
    batch_no: Option<String>,
    color_no: String,
    process_no: String,
    end_user_group: String,
    contact_type: String,
    safety_category: String,
    product_form: String,
    target_market: String,
    #[serde(default)]
    color_role: Option<String>,
    #[serde(default)]
    exemption_tags: Vec<String>,
    remarks: Option<String>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Deserialize)]
pub struct StatusBody {
    /// 目标状态，取值域由 service 按词表与迁移白名单双重校验
    pub status: String,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Deserialize)]
pub struct GenerateItemsBody {
    /// 客户点名加验项目编码（入列归因 CUSTOMER，重生成存活）
    #[serde(default)]
    pub extra_item_codes: Vec<String>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Deserialize)]
pub struct ChildCheckRowBody {
    pub item_code: String,
    #[serde(default)]
    pub measured_value: Option<Decimal>,
    #[serde(default)]
    pub measured_unit: Option<String>,
    #[serde(default)]
    pub measured_flag: Option<String>,
    #[serde(default)]
    pub check_desc: Option<String>,
    #[serde(default)]
    pub manual_conclusion: Option<String>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Deserialize)]
pub struct ChildCheckUpsertBody {
    pub rows: Vec<ChildCheckRowBody>,
}

fn submission_req(body: CreateSubmissionBody, created_by: i32) -> CreateSubmissionReq {
    CreateSubmissionReq {
        submission_no: body.submission_no,
        product_id: body.product_id,
        batch_no: body.batch_no,
        color_no: body.color_no,
        process_no: body.process_no,
        // 色号角色缺省为全检（full 是词表内的中性缺省，不代表"浅色系列以深色代表"口径）
        color_role: body
            .color_role
            .unwrap_or_else(|| crate::models::status::sampling_color_role::FULL.to_string()),
        end_user_group: body.end_user_group,
        contact_type: body.contact_type,
        safety_category: body.safety_category,
        product_form: body.product_form,
        target_market: body.target_market,
        exemption_tags: body.exemption_tags,
        remarks: body.remarks,
        created_by,
    }
}

/// 送检单列表（分页，可按品种、状态、目标市场过滤）。
///
/// 出参遵循全站统一成功信封 ApiResponse，分页载荷 PaginatedResponse{items,total,page,page_size}。
pub async fn list_submissions(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = TextileSafetySubmissionService::new(state.db.clone());
    let page = q.page.unwrap_or(1);
    let page_size = q.page_size.unwrap_or(20);
    let (items, total) = service
        .list(SubmissionListParams {
            product_id: q.product_id,
            status: q.status,
            target_market: q.target_market,
            page: q.page,
            page_size: q.page_size,
        })
        .await?;
    let data = serde_json::to_value(PaginatedResponse::new(items, total, page, page_size))?;
    Ok(Json(ApiResponse::success(data)))
}

/// 送检单详情（单据本体加清单、童装核查、报告三张子集，一次给齐展示面）。
pub async fn get_submission(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = TextileSafetySubmissionService::new(state.db.clone());
    let submission = service.get(id).await?;
    let items = service.list_items(id).await?;
    let child_checks = service.list_child_checks(id).await?;
    let reports = TextileSafetyReportService::new(state.db.clone())
        .list_by_submission(id)
        .await?;
    let items_json: Vec<serde_json::Value> = items
        .iter()
        .map(|(row, item)| {
            serde_json::json!({
                "id": row.id,
                "source": row.source,
                "item_id": item.id,
                "item_code": item.item_code,
                "item_name": item.item_name,
                "item_kind": item.item_kind,
                "basis_standard": item.basis_standard,
                "check_kind": item.check_kind,
                "unit": item.unit,
            })
        })
        .collect();
    let data = serde_json::json!({
        "submission": submission,
        "items": items_json,
        "child_checks": child_checks,
        "reports": reports,
        // 采信决策预览（同一套单源规则，未判定前也能看清"哪份会被采信、其余为什么被排除"）
        "adoption": select_adopted_reports(
            &reports.iter().map(ReportCandidate::from).collect::<Vec<_>>(),
            chrono::Utc::now().date_naive(),
        ),
        // 目标市场合规要求参考（内销不挂出口要求返回空集；只推荐不硬判，不做判定依据）
        "market_requirements": if submission.target_market == export_market::DOM {
            Vec::<crate::models::textile_export_market_requirement::Model>::new()
        } else {
            ExportRequirement::find()
                .filter(ReqColumn::Market.eq(submission.target_market.as_str()))
                .all(state.db.as_ref())
                .await?
        },
    });
    Ok(Json(ApiResponse::success(data)))
}

/// 建单：新建送检单（草稿态）。
///
/// 判据：created_by 取会话不可伪造；维度字段白名单与"人群最低类别"校验在 service。
pub async fn create_submission(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(body): Json<CreateSubmissionBody>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = TextileSafetySubmissionService::new(state.db.clone());
    let saved = service.create(submission_req(body, auth.user_id)).await?;
    tracing::info!(
        submission_id = saved.id,
        operator = auth.user_id,
        "纺织品安全送检单已创建"
    );
    Ok(Json(ApiResponse::success(serde_json::to_value(saved)?)))
}

/// 修改送检要素（仅可编辑态；全量字段重校验，与建单同判据）。
pub async fn update_submission(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(body): Json<CreateSubmissionBody>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = TextileSafetySubmissionService::new(state.db.clone());
    let saved = service
        .update(id, submission_req(body, auth.user_id))
        .await?;
    tracing::info!(
        submission_id = id,
        operator = auth.user_id,
        "纺织品安全送检单要素已修改"
    );
    Ok(Json(ApiResponse::success(serde_json::to_value(saved)?)))
}

/// 删除草稿送检单（仅 draft 且无报告挂单）。
pub async fn delete_submission(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = TextileSafetySubmissionService::new(state.db.clone());
    service.delete_draft(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success(
        serde_json::json!({ "deleted": id }),
    )))
}

/// 状态迁移（白名单推进与撤回；reported 与 judged 由登记与判定动作自动产出，不走本入口）。
pub async fn set_submission_status(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(body): Json<StatusBody>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = TextileSafetySubmissionService::new(state.db.clone());
    let (saved, stock_link) = service.set_status(id, &body.status, auth.user_id).await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "submission": saved,
        "stock_link": stock_link,
    }))))
}

/// 生成（重生成）送检项目清单：按人群与接触方式与市场推项目集，可带客户点名项。
///
/// 判据：项目集一律来自检测项目主数据的 basis 分组（单源），本 handler 不写任何项目编码；
/// 出口市场在单源里无登记加验项时由 service 的 scope_note 点名回显，不静默。
pub async fn generate_items(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(body): Json<GenerateItemsBody>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = TextileSafetySubmissionService::new(state.db.clone());
    let generated = service
        .generate_items(id, body.extra_item_codes, auth.user_id)
        .await?;
    let items_json: Vec<serde_json::Value> = generated
        .items
        .iter()
        .map(|(row, item)| {
            serde_json::json!({
                "id": row.id,
                "source": row.source,
                "item_id": item.id,
                "item_code": item.item_code,
                "item_name": item.item_name,
                "basis_standard": item.basis_standard,
            })
        })
        .collect();
    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": items_json,
        "scope_note": generated.scope_note,
        "needs_attention": generated.needs_attention,
    }))))
}

/// 童装加考核查登记（GB 31701 条款逐行 upsert；人工项只给条目与结论位）。
pub async fn upsert_child_checks(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(body): Json<ChildCheckUpsertBody>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = TextileSafetySubmissionService::new(state.db.clone());
    let rows: Vec<ChildCheckUpsertReq> = body
        .rows
        .into_iter()
        .map(|r| ChildCheckUpsertReq {
            item_code: r.item_code,
            measured_value: r.measured_value,
            measured_unit: r.measured_unit,
            measured_flag: r.measured_flag,
            check_desc: r.check_desc,
            manual_conclusion: r.manual_conclusion,
        })
        .collect();
    let saved = service.upsert_child_checks(id, rows, auth.user_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(saved)?)))
}

/// 整单判定：童装 measurable 行走判定器、manual 行搬运结论位、按**采信的那一份报告**汇总三态。
///
/// 判据：缺报告受理、无可靠采信对象、缺人工结论、残留待判定行一律拒绝完成并点名，
/// 绝不产出无依据结论；被排除的报告逐枚带原因回显（取值域 report_adoption_reason）。
pub async fn judge_submission(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = TextileSafetySubmissionService::new(state.db.clone());
    let outcome = service.judge_submission(id, auth.user_id).await?;
    tracing::info!(
        submission_id = id,
        overall = outcome.submission.overall_verdict.as_deref().unwrap_or(""),
        operator = auth.user_id,
        "纺织品安全送检单整单判定完成"
    );
    Ok(Json(ApiResponse::success(serde_json::json!({
        "submission": outcome.submission,
        "stock_link": outcome.stock_link,
        "adoption": outcome.adoption,
        "expired_reports_named": outcome.expired_reports_named,
    }))))
}
