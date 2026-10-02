use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::quality_inspection;
use crate::models::quality_inspection_record;
use crate::models::unqualified_product;
use crate::models::user;
use crate::services::quality_inspection_service::{
    CreateInspectionRecordRequest, CreateQualityInspectionStandardRequest,
    ProcessUnqualifiedRequest, QualityInspectionService,
};
use crate::utils::ApiResponse;
use crate::utils::data_scope::{DataScope, DataScopeContext, check_resource_owner};
use crate::utils::error::AppError;
// V15 P0-S12/P0-S15 修复（Batch 475d）：导出端点使用水印版 xlsx 工具
use crate::utils::xlsx_export::{WatermarkConfig, XlsxTable, build_xlsx_response_with_watermark};
// V15 P0-S11：导出审计日志写入所需依赖
use crate::models::audit_log::{OperationType, Severity};
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;
use validator::Validate;

/// 校验检验结论取值，越界值直接拒绝并报出允许值清单。
///
/// 该列此前是自由文本：界面自造过 pass/fail/pending，而唯一的自动写入方落的是中文结论，
/// 两套写法混在同一列会让按结论筛选与合格率统计静默失真，因此在入口处一次拦清。
pub fn validate_inspection_result(raw: &str) -> Result<(), AppError> {
    use crate::models::status::quality_inspection_result;

    if quality_inspection_result::ALL.contains(&raw) {
        Ok(())
    } else {
        Err(AppError::validation_displayable(format!(
            "检验结果 {raw} 不是合法取值，允许值：{}",
            quality_inspection_result::ALL.join("/")
        )))
    }
}

/// 校验检验类型取值，越界值拒绝并报出允许值清单。
///
/// 四个码由质检记录弹窗写入，`outsourcing_receipt` 是委外回仓自动建记录的来源标识；
/// 与 `ai_quality_predictions.inspection_type`（另一套 CHECK 词表）不可互抄。
pub fn validate_inspection_type(raw: &str) -> Result<(), AppError> {
    use crate::models::status::quality_inspection_type;

    for allowed in quality_inspection_type::ALL {
        if *allowed == raw {
            return Ok(());
        }
    }
    Err(AppError::validation_displayable(format!(
        "检验类型 {raw} 不是合法取值，允许值：{}",
        quality_inspection_type::ALL.join("/")
    )))
}

/// 列表与导出共用的记录筛选条件构造：空串按「未选择」处理，两个枚举入参越界一律拒绝。
fn build_record_list_params(
    query: &RecordQuery,
    page: i64,
    page_size: i64,
) -> Result<crate::services::quality_inspection_service::RecordListParams, AppError> {
    use crate::services::quality_inspection_service::RecordListParams;

    let normalized = |raw: &Option<String>| {
        raw.as_deref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    };
    let inspection_type = normalized(&query.inspection_type);
    if let Some(value) = &inspection_type {
        validate_inspection_type(value)?;
    }
    let inspection_result = normalized(&query.inspection_result);
    if let Some(value) = &inspection_result {
        validate_inspection_result(value)?;
    }

    Ok(RecordListParams {
        inspection_type,
        inspection_result,
        product_id: query.product_id,
        batch_no: normalized(&query.batch_no),
        page,
        page_size,
    })
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct QualityInspectionQuery {
    pub inspection_type: Option<String>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

// 记录列表与导出共用该查询参数；字段名与 sales 侧契约一致（batch_no 而非 batch_number），
// 每个字段都真正参与筛选，因此不再需要 dead_code 豁免
#[derive(Debug, Clone, Deserialize)]
pub struct RecordQuery {
    pub product_id: Option<i32>,
    pub batch_no: Option<String>,
    pub inspection_type: Option<String>,
    pub inspection_result: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct DefectQuery {
    pub record_id: Option<i32>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

pub async fn list_standards(
    Query(params): Query<QualityInspectionQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<quality_inspection::Model>>>, AppError> {
    info!("用户 {} 正在查询质量检验标准列表", auth.user_id);

    let service = QualityInspectionService::new(state.db.clone());
    let query_params = crate::services::quality_inspection_service::QualityInspectionQueryParams {
        inspection_type: params.inspection_type,
        status: params.status,
        page: params.page.unwrap_or(1).clamp(1, 1000),
        page_size: params.page_size.unwrap_or(10).clamp(1, 100),
    };

    let (standards, _total) = service.get_standards_list(query_params).await?;
    info!("质量检验标准列表查询成功，共 {} 条记录", standards.len());

    Ok(Json(ApiResponse::success(standards)))
}

pub async fn create_standard(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateQualityInspectionStandardRequest>,
) -> Result<Json<ApiResponse<quality_inspection::Model>>, AppError> {
    info!("用户 {} 正在创建质量检验标准", auth.user_id);

    let service = QualityInspectionService::new(state.db.clone());
    let standard = service.create_standard(req, auth.user_id).await?;
    info!("质量检验标准创建成功，ID：{}", standard.id);

    Ok(Json(ApiResponse::success(standard)))
}

pub async fn list_records(
    Query(params): Query<RecordQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<
    Json<ApiResponse<crate::utils::response::PaginatedResponse<quality_inspection_record::Model>>>,
    AppError,
> {
    info!("用户 {} 正在查询质量检验记录列表", auth.user_id);

    let page = params.page.unwrap_or(1).clamp(1, 1000) as u64;
    let page_size = params.page_size.unwrap_or(10).clamp(1, 100) as u64;
    let service = QualityInspectionService::new(state.db.clone());
    let query_params = build_record_list_params(&params, page as i64, page_size as i64)?;

    let (records, total) = service.get_records_list(query_params).await?;
    info!("质量检验记录列表查询成功，共 {} 条记录", records.len());

    // v11 批次 161 P2-5 修复：返回 PaginatedResponse（含 total），替代原先丢弃 _total 的 Vec 返回
    Ok(Json(ApiResponse::success_paginated(
        records, total, page, page_size,
    )))
}

#[axum::debug_handler]
pub async fn create_record(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateInspectionRecordRequest>,
) -> Result<Json<ApiResponse<quality_inspection_record::Model>>, AppError> {
    info!("用户 {} 正在创建质量检验记录", auth.user_id);
    validate_inspection_result(&req.inspection_result)?;

    let service = QualityInspectionService::new(state.db.clone());
    let record = service.create_record(req, auth.user_id).await?;
    info!("质量检验记录创建成功，ID：{}", record.id);

    Ok(Json(ApiResponse::success(record)))
}

pub async fn get_record(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<quality_inspection_record::Model>>, AppError> {
    info!("用户 {} 正在查询质量检验记录，ID: {}", auth.user_id, id);

    let service = QualityInspectionService::new(state.db.clone());
    let record = service.get_record_by_id(id).await?;
    info!("质量检验记录查询成功，ID：{}", record.id);

    Ok(Json(ApiResponse::success(record)))
}

/// 更新质检记录请求（对应前端 api/quality.ts updateQualityRecord 传 Partial<QualityRecord>，全字段可选）
///
/// JSON 三态反序列化适配器与字段语义见 handlers/department_handler.rs 同名实现/注释：
/// 键缺席=保持原值、显式 null=清空为 NULL（仅 DB 可空列）、有值=覆盖。
/// NOT NULL/模型非 Option 列（inspection_type/inspection_date：m0005:164/166 建表 NOT NULL；
/// total_qty/inspected_qty/inspection_result：system/mod.rs:197/190/191 ALTER 补列虽可空，
/// 但实体 Model 列为非 Option Decimal/String，置 NULL 该行按模型不可读）显式 null 一律拒绝。
fn double_option<'de, T, D>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Deserialize::deserialize(de).map(Some)
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateInspectionRecordRequest {
    /// 检验类型：DB NOT NULL（m0005:164）——显式 null 被拒绝
    #[serde(default, deserialize_with = "double_option")]
    #[validate(length(max = 50, message = "检验类型长度不得超过 50 字符"))]
    pub inspection_type: Option<Option<String>>,
    /// 批次号：DB 可空 VARCHAR（m0005:162）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    #[validate(length(max = 100, message = "批次号长度不得超过 100 字符"))]
    pub batch_no: Option<Option<String>>,
    /// 检验日期：DB NOT NULL（m0005:166）——显式 null 被拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub inspection_date: Option<Option<chrono::NaiveDate>>,
    /// 检验员 ID：DB 可空 INTEGER（m0005:167）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub inspector_id: Option<Option<i32>>,
    /// 送检总数：实体 Model 非 Option Decimal（system/mod.rs:197 补列）——显式 null 被拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub total_qty: Option<Option<rust_decimal::Decimal>>,
    /// 已检数：实体 Model 非 Option Decimal（system/mod.rs:190 补列）——显式 null 被拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub inspected_qty: Option<Option<rust_decimal::Decimal>>,
    /// 合格数：DB 可空 DECIMAL（m0005:169）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub qualified_qty: Option<Option<rust_decimal::Decimal>>,
    /// 不合格数：DB 可空 DECIMAL（m0005:170）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub unqualified_qty: Option<Option<rust_decimal::Decimal>>,
    /// 合格率：DB 可空 DECIMAL（system/mod.rs:192 补列）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub qualification_rate: Option<Option<rust_decimal::Decimal>>,
    /// 检验结论：实体 Model 非 Option String（system/mod.rs:191 补列；词表 quality_inspection_result）——显式 null 被拒绝
    #[serde(default, deserialize_with = "double_option")]
    #[validate(length(max = 20, message = "检验结果长度不得超过 20 字符"))]
    pub inspection_result: Option<Option<String>>,
    /// 备注：DB 可空 VARCHAR（system/mod.rs:195 补列）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub remark: Option<Option<String>>,
    /// 缺陷类型：DB 可空 VARCHAR（system/mod.rs:185 补列）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    #[validate(length(max = 50, message = "缺陷类型长度不得超过 50 字符"))]
    pub defect_type: Option<Option<String>>,
    /// 等级：DB 可空 VARCHAR（system/mod.rs:189 补列）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    #[validate(length(max = 10, message = "等级长度不得超过 10 字符"))]
    pub grade: Option<Option<String>>,
    /// 色号：DB 可空 VARCHAR（system/mod.rs:183 补列）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    #[validate(length(max = 100, message = "色号长度不得超过 100 字符"))]
    pub color_no: Option<Option<String>>,
    /// 缸号：DB 可空 VARCHAR（system/mod.rs:186 补列）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    #[validate(length(max = 100, message = "缸号长度不得超过 100 字符"))]
    pub dye_lot_no: Option<Option<String>>,
}

/// PUT /api/v1/erp/production/quality-inspection/records/{id} - 编辑质检记录
/// （对应前端 api/quality.ts updateQualityRecord；参照 create_record 的字段集，仅更新请求中显式提供的字段）
pub async fn update_record(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
    Json(req): Json<UpdateInspectionRecordRequest>,
) -> Result<Json<ApiResponse<quality_inspection_record::Model>>, AppError> {
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    info!("用户 {} 正在更新质量检验记录，ID: {}", auth.user_id, id);
    req.validate().map_err(AppError::from)?;

    // NOT NULL/模型非 Option 列门控（在任何 DB 访问之前拒绝，外显不脱敏）：
    if matches!(req.inspection_type, Some(None)) {
        return Err(AppError::business_displayable(
            "检验类型不能清空：该字段为必填项",
        ));
    }
    if matches!(req.inspection_date, Some(None)) {
        return Err(AppError::business_displayable(
            "检验日期不能清空：该字段为必填项",
        ));
    }
    if matches!(req.total_qty, Some(None)) {
        return Err(AppError::business_displayable(
            "送检总数不能清空：该字段为必填项",
        ));
    }
    if matches!(req.inspected_qty, Some(None)) {
        return Err(AppError::business_displayable(
            "已检数不能清空：该字段为必填项",
        ));
    }
    if matches!(req.inspection_result, Some(None)) {
        return Err(AppError::business_displayable(
            "检验结论不能清空：该字段为必填项",
        ));
    }

    // 词表校验仅对覆盖值执行（借用检查，不移动字段——后续写入仍需消费 req）
    if let Some(Some(v)) = req.inspection_result.as_ref() {
        validate_inspection_result(v)?;
    }

    let existing = quality_inspection_record::Entity::find_by_id(id)
        .one(state.db.as_ref())
        .await?
        .ok_or_else(|| AppError::not_found(format!("质量检验记录不存在：{}", id)))?;

    let mut active: quality_inspection_record::ActiveModel = existing.into();
    // NOT NULL 列（Some(None) 已在入口拒绝）：仅覆盖/保持
    if let Some(v) = req.inspection_type.flatten() {
        active.inspection_type = Set(v);
    }
    if let Some(v) = req.inspection_date.flatten() {
        active.inspection_date = Set(v);
    }
    if let Some(v) = req.total_qty.flatten() {
        active.total_qty = Set(v);
    }
    if let Some(v) = req.inspected_qty.flatten() {
        active.inspected_qty = Set(v);
    }
    if let Some(v) = req.inspection_result.flatten() {
        active.inspection_result = Set(v);
    }
    // DB 可空列：Some(None)=Set(None) 清空、Some(Some(v))=Set(Some(v)) 覆盖、None=不 Set
    if let Some(v) = req.batch_no {
        active.batch_no = Set(v);
    }
    if let Some(v) = req.inspector_id {
        active.inspector_id = Set(v);
    }
    if let Some(v) = req.qualified_qty {
        active.qualified_qty = Set(v);
    }
    if let Some(v) = req.unqualified_qty {
        active.unqualified_qty = Set(v);
    }
    if let Some(v) = req.qualification_rate {
        active.qualification_rate = Set(v);
    }
    if let Some(v) = req.remark {
        active.remark = Set(v);
    }
    if let Some(v) = req.defect_type {
        active.defect_type = Set(v);
    }
    if let Some(v) = req.grade {
        active.grade = Set(v);
    }
    if let Some(v) = req.color_no {
        active.color_no = Set(v);
    }
    if let Some(v) = req.dye_lot_no {
        active.dye_lot_no = Set(v);
    }
    active.updated_at = Set(chrono::Utc::now());

    let updated = active.update(state.db.as_ref()).await?;
    info!("质量检验记录更新成功，ID：{}", updated.id);

    Ok(Json(ApiResponse::success_with_message(
        updated,
        "质量检验记录更新成功",
    )))
}

pub async fn list_defects(
    Query(params): Query<DefectQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<unqualified_product::Model>>>, AppError> {
    info!("用户 {} 正在查询质量缺陷列表", auth.user_id);

    let service = QualityInspectionService::new(state.db.clone());
    let query_params = crate::services::quality_inspection_service::QualityInspectionQueryParams {
        inspection_type: None,
        status: params.status,
        page: params.page.unwrap_or(1).clamp(1, 1000),
        page_size: params.page_size.unwrap_or(10).clamp(1, 100),
    };

    let (defects, _total) = service.get_defects_list(query_params).await?;
    info!("质量缺陷列表查询成功，共 {} 条记录", defects.len());

    Ok(Json(ApiResponse::success(defects)))
}

#[axum::debug_handler]
pub async fn process_defect(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
    Json(req): Json<ProcessUnqualifiedRequest>,
) -> Result<Json<ApiResponse<unqualified_product::Model>>, AppError> {
    info!("用户 {} 正在处理质量缺陷，记录ID: {}", auth.user_id, id);

    let service = QualityInspectionService::new(state.db.clone());
    let result = service.process_unqualified(id, req, auth.user_id).await?;
    info!("质量缺陷处理成功，ID：{}", result.id);

    Ok(Json(ApiResponse::success(result)))
}

/// 报废一级（财务）审批请求。
///
/// 审批人身份**不在请求契约内**：approver_id 一律取服务端会话（`AuthContext.user_id`），
/// 从结构上杜绝 body 伪造 `approved_by`（对照 dye rework 审批端点由前端传审批人的反面教材）。
#[derive(Debug, Deserialize)]
pub struct ScrapFinancialApprovalRequest {
    /// true=通过（pending_fin → pending_gm）；false=拒绝（→ rejected，处理状态同步 rejected）
    pub approved: bool,
}

/// 报废二级（总经理）审批请求（最终审批，通过后写入报废损失金额供成本核算）。
#[derive(Debug, Deserialize)]
pub struct ScrapGmApprovalRequest {
    /// true=通过（pending_gm → approved）；false=拒绝（→ rejected）
    pub approved: bool,
    /// 报废损失金额：仅 approved=true 时有意义；负数拒绝（金额校验属于用户自提字段的公开规则，可外显）
    pub scrap_loss_amount: Option<rust_decimal::Decimal>,
}

/// 报废审批行级归属预检（IDOR/数据范围防护）。
///
/// `unqualified_products` 全表不存在任何归属人/部门列（m0013 建表 + business/v15 域全部 ALTER
/// 逐列核实），因此行归属只能沿真实外键链推导：不合格品 → 关联质检记录（inspection_id）→
/// 检验员（inspector_id，归属人）→ 检验员部门（users.department_id，仅 dept 范围需要）。
/// 判定复用全域唯一实现 `utils/data_scope::check_resource_owner`（All 放行 / Dept 需检验员
/// 部门 ∈ 可见部门集合 / Self 仅本人检验的报废单），查不到归属链一律 fail-closed 拒绝。
/// 越权拒绝出参永久脱敏（PermissionDenied → 403 `FORBIDDEN` + 固定文案，真实依据只进日志）。
async fn assert_scrap_approval_access(
    db: &sea_orm::DatabaseConnection,
    unqualified_id: i32,
    ctx: &DataScopeContext,
) -> Result<(), AppError> {
    // All（管理员/总经理等全部数据范围）无行级限制；存在性检查仍由服务层执行（不在此放松）
    if matches!(ctx.scope, DataScope::All) {
        return Ok(());
    }
    use sea_orm::EntityTrait;
    let record = unqualified_product::Entity::find_by_id(unqualified_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::not_found(format!("不合格品记录不存在：{}", unqualified_id)))?;
    let (owner, dept) = match record.inspection_id {
        Some(inspection_id) => {
            let inspector_id = quality_inspection_record::Entity::find_by_id(inspection_id)
                .one(db)
                .await?
                .and_then(|rec| rec.inspector_id);
            let dept_id = match inspector_id {
                Some(uid) => user::Entity::find_by_id(uid)
                    .one(db)
                    .await?
                    .and_then(|u| u.department_id),
                None => None,
            };
            (inspector_id, dept_id)
        }
        None => (None, None),
    };
    if !check_resource_owner(ctx, owner, dept) {
        tracing::warn!(
            unqualified_id,
            user_id = ctx.user_id,
            scope = ctx.scope.as_str(),
            inspector_id = ?owner,
            inspector_dept = ?dept,
            "报废审批行级归属校验未通过（数据范围限制，出参脱敏 403）"
        );
        return Err(AppError::permission_denied(
            "无权对该不合格品记录执行报废审批",
        ));
    }
    Ok(())
}

/// POST /api/v1/erp/production/quality-inspection/defects/{id}/scrap-approval/financial
/// —— 报废财务（一级）审批。
///
/// path `{id}` = 不合格品记录 `unqualified_products.id`（与 defects 列表同一实体）。
/// 权限键由 URL 段推导（seg3=production 模块前缀 → resource=quality-inspection，POST→create），
/// 与本区块 defects/{id}/process 同源自动统一，不引入新权限常量。
/// 状态门（handling_method=scrap、scrap_approval_status=pending_fin）由服务层既有判定执行，
/// 拒绝族 = BUSINESS_ERROR（出参默认脱敏，真实原因只进 tracing::warn）。
pub async fn approve_scrap_financial(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
    Json(req): Json<ScrapFinancialApprovalRequest>,
) -> Result<Json<ApiResponse<unqualified_product::Model>>, AppError> {
    let ctx = auth.to_data_scope_context();
    assert_scrap_approval_access(state.db.as_ref(), id, &ctx).await?;

    let service = QualityInspectionService::new(state.db.clone());
    let result = service
        .approve_scrap_financial(id, auth.user_id, req.approved)
        .await;
    match &result {
        Ok(updated) => info!(
            unqualified_id = updated.id,
            approver_id = auth.user_id,
            approved = req.approved,
            scrap_approval_status = %updated.scrap_approval_status,
            "报废财务审批完成"
        ),
        Err(e) => tracing::warn!(
            unqualified_id = id,
            approver_id = auth.user_id,
            error = %e,
            "报废财务审批被拒（存在性/状态门；出参保持脱敏 BUSINESS_ERROR）"
        ),
    }
    Ok(Json(ApiResponse::success(result?)))
}

/// POST /api/v1/erp/production/quality-inspection/defects/{id}/scrap-approval/gm
/// —— 报废总经理（二级/最终）审批。
///
/// 前置门（必须先完成财务审批，即 scrap_approval_status=pending_gm）由服务层
/// `approve_scrap_gm` 既有判定执行，本 handler 不放松也不重复实现；拒绝族与脱敏口径
/// 与一级审批一致（BUSINESS_ERROR + 403 权限族永久脱敏）。
pub async fn approve_scrap_gm(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
    Json(req): Json<ScrapGmApprovalRequest>,
) -> Result<Json<ApiResponse<unqualified_product::Model>>, AppError> {
    // 入参值门：损失金额只允许随「同意」提交且非负——描述用户自提字段的公开规则，可外显；
    // 拒绝路径不产生任何写入（校验先于任何 DB 访问）
    if !req.approved && req.scrap_loss_amount.is_some() {
        return Err(AppError::validation_displayable(
            "仅同意的报废审批可携带报废损失金额",
        ));
    }
    if let Some(amount) = req.scrap_loss_amount {
        if amount < rust_decimal::Decimal::ZERO {
            return Err(AppError::validation_displayable("报废损失金额不得为负数"));
        }
    }

    let ctx = auth.to_data_scope_context();
    assert_scrap_approval_access(state.db.as_ref(), id, &ctx).await?;

    let service = QualityInspectionService::new(state.db.clone());
    let result = service
        .approve_scrap_gm(id, auth.user_id, req.approved, req.scrap_loss_amount)
        .await;
    match &result {
        Ok(updated) => info!(
            unqualified_id = updated.id,
            approver_id = auth.user_id,
            approved = req.approved,
            scrap_approval_status = %updated.scrap_approval_status,
            "报废总经理审批完成"
        ),
        Err(e) => tracing::warn!(
            unqualified_id = id,
            approver_id = auth.user_id,
            error = %e,
            "报废总经理审批被拒（存在性/状态门/跳级；出参保持脱敏 BUSINESS_ERROR）"
        ),
    }
    Ok(Json(ApiResponse::success(result?)))
}

/// 质量检验记录导出表头（13 列）
fn record_export_headers() -> Vec<String> {
    vec![
        "ID".to_string(),
        "检验编号".to_string(),
        "检验类型".to_string(),
        "产品ID".to_string(),
        "批次号".to_string(),
        "检验日期".to_string(),
        "检验员ID".to_string(),
        "总数量".to_string(),
        "已检数量".to_string(),
        "合格数量".to_string(),
        "不合格数量".to_string(),
        "检验结果".to_string(),
        "等级".to_string(),
    ]
}

/// 从质量检验记录 JSON 对象构建 xlsx 行
fn build_record_row(obj: &serde_json::Map<String, serde_json::Value>) -> Vec<String> {
    let get_str = |key: &str| -> String {
        obj.get(key)
            .map(|v| {
                if v.is_null() {
                    String::new()
                } else if v.is_string() {
                    v.as_str().unwrap_or("").to_string()
                } else {
                    v.to_string()
                }
            })
            .unwrap_or_default()
    };
    vec![
        get_str("id"),
        get_str("inspection_no"),
        get_str("inspection_type"),
        get_str("product_id"),
        get_str("batch_no"),
        get_str("inspection_date"),
        get_str("inspector_id"),
        get_str("total_qty"),
        get_str("inspected_qty"),
        get_str("qualified_qty"),
        get_str("unqualified_qty"),
        get_str("inspection_result"),
        get_str("grade"),
    ]
}

/// 构造质量检验记录列表 xlsx 表格
fn build_records_table(records_json: Vec<serde_json::Value>) -> Result<XlsxTable, AppError> {
    let mut rows: Vec<Vec<String>> = Vec::with_capacity(records_json.len());
    for r in records_json {
        let obj = r
            .as_object()
            .ok_or_else(|| AppError::internal("质量检验记录序列化失败：期望 JSON 对象"))?;
        rows.push(build_record_row(obj));
    }
    Ok(XlsxTable {
        sheet_name: "质量检验记录".to_string(),
        headers: record_export_headers(),
        rows,
    })
}

/// 异步记录质量检验记录导出操作（审计自身）
fn record_records_export_audit(
    state: &AppState,
    auth: &AuthContext,
    row_count: usize,
    filename: &str,
) {
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Export,
        severity: Severity::Info,
        resource_type: Some("quality_inspection_record".to_string()),
        resource_id: None,
        resource_name: Some(format!("{}.xlsx", filename)),
        description: Some(format!(
            "用户 {} 导出质量检验记录列表（共 {} 条）",
            auth.username, row_count
        )),
        request_method: Some("GET".to_string()),
        request_path: Some("/api/v1/erp/quality-inspection/records/export".to_string()),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "format": "xlsx",
            "total": row_count,
        })),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, None);
}

/// GET /api/v1/erp/quality-inspection/records/export - 导出质量检验记录列表（带水印 + 异步审计日志）；V15 P0-S12 修复（Batch 475d）：导出接入后端 - 注入水印（operator/exported_at/extra 含条数） - 异步审计日志（OperationType::Export） - 直接调
/// service.get_records_list 取全量数据（page=1/page_size=10000） - 筛选条件与 list_records 走同一个
/// `build_record_list_params`（同一套取值域校验），保证导出与列表口径一致
pub async fn export_records(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<RecordQuery>,
) -> Result<axum::response::Response, AppError> {
    let service = QualityInspectionService::new(state.db.clone());

    // V15 P0-S12 修复（Batch 475d）：导出全量数据；筛选条件与 list_records 共用同一构造函数，
    // 保证导出数据与列表口径一致（含取值域校验，越界同样拒绝）
    let query_params = build_record_list_params(&query, 1, 10000)?;

    let (records, _total) = service.get_records_list(query_params).await?;
    let row_count = records.len();

    // 序列化为 JSON 以统一字段处理
    let records_json: Vec<serde_json::Value> = records
        .into_iter()
        .map(|r| serde_json::to_value(r).map_err(AppError::from))
        .collect::<Result<Vec<_>, _>>()?;

    let table = build_records_table(records_json)?;
    let filename = format!(
        "quality_inspection_records_export_{}",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );
    record_records_export_audit(&state, &auth, row_count, &filename);

    // V15 P0-S15 修复（Batch 475d）：注入水印（操作员/导出时间/导出条数）
    let watermark = WatermarkConfig {
        operator: Some(auth.username.clone()),
        ip_address: None,
        exported_at: Some(chrono::Utc::now().to_rfc3339()),
        extra: Some(format!("质量检验记录导出（共 {} 条）", row_count)),
    };

    build_xlsx_response_with_watermark(&table, &filename, &watermark)
}
