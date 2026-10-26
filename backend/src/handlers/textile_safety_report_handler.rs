//! 第三方检测报告处理器（登记、要素修正、完整性门、逐项目结果录入、报告级判定）
//!
//! 完整性门与判定链路的判据在 `services/textile_safety_report_service.rs` 与
//! `services/textile_safety_limit_judge.rs`，本文件不复制校验逻辑；
//! 受理态（accepted/rejected）不可能由请求体直填，只能由门产出。
use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::textile_safety_report_service::{
    RegisterReportReq, ResultRowReq, TextileSafetyReportService,
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
pub struct ListReportsQuery {
    pub submission_id: i32,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Deserialize)]
pub struct RegisterReportBody {
    pub report_no: String,
    pub submission_id: i32,
    pub inspection_agency: String,
    #[serde(default)]
    pub has_cma: bool,
    #[serde(default)]
    pub has_cnas: bool,
    pub sample_name: Option<String>,
    pub material_composition: Option<String>,
    pub color_no: Option<String>,
    pub process_no: Option<String>,
    pub test_basis: Option<String>,
    pub conclusion: Option<String>,
    pub oeko_tex_class: Option<String>,
    /// 签发日期 YYYY-MM-DD
    pub issue_date: chrono::NaiveDate,
    /// 有效期至 YYYY-MM-DD（缺失时完整性门按无法核验点名 expired，宁拒不猜）
    pub valid_until: Option<chrono::NaiveDate>,
    pub report_file_url: Option<String>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Deserialize)]
pub struct ResultRowBody {
    pub item_code: String,
    #[serde(default)]
    pub measured_value: Option<Decimal>,
    /// 等级型 token（"3-4" 等），与 measured_value 互斥
    #[serde(default)]
    pub measured_token: Option<String>,
    #[serde(default)]
    pub measured_text: Option<String>,
    #[serde(default)]
    pub measured_unit: Option<String>,
    #[serde(default)]
    pub test_method: Option<String>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Deserialize)]
pub struct UpsertResultsBody {
    pub rows: Vec<ResultRowBody>,
}

fn register_req(body: RegisterReportBody, created_by: i32) -> RegisterReportReq {
    RegisterReportReq {
        report_no: body.report_no,
        submission_id: body.submission_id,
        inspection_agency: body.inspection_agency,
        has_cma: body.has_cma,
        has_cnas: body.has_cnas,
        sample_name: body.sample_name,
        material_composition: body.material_composition,
        color_no: body.color_no,
        process_no: body.process_no,
        test_basis: body.test_basis,
        conclusion: body.conclusion,
        oeko_tex_class: body.oeko_tex_class,
        issue_date: body.issue_date,
        valid_until: body.valid_until,
        report_file_url: body.report_file_url,
        created_by,
    }
}

/// 按送检单列报告（GET /textile-reports?submission_id=）。
pub async fn list_reports(
    State(state): State<AppState>,
    Query(q): Query<ListReportsQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = TextileSafetyReportService::new(state.db.clone());
    let items = service.list_by_submission(q.submission_id).await?;
    Ok(Json(ApiResponse::success(serde_json::json!({ "reports": items }))))
}

/// 报告详情（报告本体加逐项目结果行）。
pub async fn get_report(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = TextileSafetyReportService::new(state.db.clone());
    let report = service.get(id).await?;
    let results = service.list_results(id).await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "report": report,
        "results": results,
    }))))
}

/// 登记报告：受理态恒 pending，送检单 sent 自动推进 reported。
pub async fn create_report(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(body): Json<RegisterReportBody>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = TextileSafetyReportService::new(state.db.clone());
    let saved = service.register(register_req(body, auth.user_id)).await?;
    tracing::info!(report_id = saved.id, operator = auth.user_id, "第三方报告已登记");
    Ok(Json(ApiResponse::success(serde_json::to_value(saved)?)))
}

/// 修正报告要素（改后复位 pending，必须重新过完整性门）。
pub async fn update_report(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(body): Json<RegisterReportBody>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = TextileSafetyReportService::new(state.db.clone());
    let saved = service.update(id, register_req(body, auth.user_id)).await?;
    tracing::info!(report_id = id, operator = auth.user_id, "第三方报告要素已修正，受理态已复位待核");
    Ok(Json(ApiResponse::success(serde_json::to_value(saved)?)))
}

/// 完整性门：资质位、六项要素、有效期、按批次逐项核对；缺项逐条点名并落 rejected。
pub async fn accept_report(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = TextileSafetyReportService::new(state.db.clone());
    let (saved, missing) = service.run_acceptance_gate(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "report": saved,
        "missing_elements": missing,
    }))))
}

/// 逐项目实测结果 upsert（存值、单位、检测方法结构化承载；项目必须在送检清单内）。
pub async fn upsert_results(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(body): Json<UpsertResultsBody>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = TextileSafetyReportService::new(state.db.clone());
    let rows: Vec<ResultRowReq> = body
        .rows
        .into_iter()
        .map(|r| ResultRowReq {
            item_code: r.item_code,
            measured_value: r.measured_value,
            measured_token: r.measured_token,
            measured_text: r.measured_text,
            measured_unit: r.measured_unit,
            test_method: r.test_method,
        })
        .collect();
    let saved = service.upsert_results(id, rows, auth.user_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(saved)?)))
}

/// 报告级判定：报告全部结果行按限值矩阵出三态（报告必须已受理）。
pub async fn judge_report(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = TextileSafetyReportService::new(state.db.clone());
    let saved = service.judge_report(id, auth.user_id).await?;
    tracing::info!(report_id = id, judged = saved.len(), operator = auth.user_id, "第三方报告逐项目判定完成");
    Ok(Json(ApiResponse::success(serde_json::to_value(saved)?)))
}
