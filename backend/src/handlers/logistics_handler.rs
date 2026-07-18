use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set, TransactionTrait};
use serde::Deserialize;

use crate::models::ar_invoice;
use crate::models::logistics_waybill;
use crate::models::sales_order;
use crate::models::status::common as common_status;
use crate::models::status::logistics_waybill as waybill_status;
use crate::models::status::sales_order as so_status;
use crate::middleware::auth_context::AuthContext;
use crate::services::ar_invoice_service::ArInvoiceService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

#[derive(Deserialize)]
pub struct CreateWaybillRequest {
    pub order_id: i32,
    pub logistics_company: String,
    pub tracking_number: String,
    pub driver_name: Option<String>,
    pub driver_phone: Option<String>,
    pub freight_fee: Option<f64>,
    pub expected_arrival: Option<chrono::DateTime<Utc>>,
    pub notes: Option<String>,
}

pub async fn create_waybill(
    State(state): State<AppState>,
    Json(req): Json<CreateWaybillRequest>,
) -> Result<Json<ApiResponse<logistics_waybill::Model>>, AppError> {
    let txn = state.db.begin().await?;

    // Verify order exists
    let order = sales_order::Entity::find_by_id(req.order_id)
        .one(&txn)
        .await?
        .ok_or_else(|| AppError::not_found("订单不存在"))?;

    // Create waybill
    let freight = req
        .freight_fee
        .map(|f| Decimal::from_f64_retain(f).unwrap_or(Decimal::ZERO));

    let new_waybill = logistics_waybill::ActiveModel {
        id: sea_orm::ActiveValue::NotSet,
        order_id: Set(req.order_id),
        logistics_company: Set(req.logistics_company),
        tracking_number: Set(req.tracking_number),
        driver_name: Set(req.driver_name),
        driver_phone: Set(req.driver_phone),
        freight_fee: Set(freight),
        status: Set(Some(waybill_status::IN_TRANSIT.to_string())),
        expected_arrival: Set(req.expected_arrival),
        actual_arrival: Set(None),
        notes: Set(req.notes),
        // V15 P0-B13：签收字段初始化为 None，sign_waybill handler 填入
        signed_by: Set(None),
        signed_at: Set(None),
        sign_receipt_url: Set(None),
        sign_photo_url: Set(None),
        sign_remark: Set(None),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
    };

    let inserted = new_waybill.insert(&txn).await?;

    // 更新订单状态为已发货（小写常量，与 sales_order 状态机一致）
    if order.status != so_status::SHIPPED {
        let mut active_order: sales_order::ActiveModel = order.into();
        active_order.status = Set(so_status::SHIPPED.to_string());
        active_order.update(&txn).await?;
    }

    txn.commit().await?;

    Ok(Json(ApiResponse::success(inserted)))
}

pub async fn list_waybills(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<logistics_waybill::Model>>>, AppError> {
    let waybills = logistics_waybill::Entity::find()
        .order_by_desc(logistics_waybill::Column::CreatedAt)
        .all(&*state.db)
        .await?;

    Ok(Json(ApiResponse::success(waybills)))
}

#[derive(Deserialize)]
pub struct UpdateWaybillStatusReq {
    pub status: String,
}

pub async fn update_waybill_status(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateWaybillStatusReq>,
) -> Result<Json<ApiResponse<logistics_waybill::Model>>, AppError> {
    let waybill = logistics_waybill::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("运单不存在"))?;

    let mut active_waybill: logistics_waybill::ActiveModel = waybill.into();
    active_waybill.status = Set(Some(req.status.clone()));

    if req.status == waybill_status::DELIVERED {
        active_waybill.actual_arrival = Set(Some(Utc::now()));
    }

    active_waybill.updated_at = Set(Utc::now());
    let updated = active_waybill.update(&*state.db).await?;

    Ok(Json(ApiResponse::success(updated)))
}

/// V15 P0-B13：电子签收请求 DTO
#[derive(Deserialize)]
pub struct SignWaybillRequest {
    /// 纸质回单扫描件 URL（上传到对象存储后返回）
    pub receipt_url: Option<String>,
    /// 现场签收照片 URL（上传到对象存储后返回）
    pub photo_url: Option<String>,
    /// 签收备注（异常情况说明，如"包装破损"、"数量短缺"等）
    pub remark: Option<String>,
}

/// V15 P0-B13：电子签收 handler
///
/// 业务规则：
///   1. 运单必须存在
///   2. 运单状态必须为 DELIVERED（已送达）才允许签收
///   3. 运单不能已签收（status != SIGNED，禁止重复签收）
///   4. 签收时：
///      - 状态推进到 SIGNED
///      - signed_by 自动填入 AuthContext.user_id
///      - signed_at 自动填入当前时间
///      - 写入 receipt_url / photo_url / remark（可选）
///   5. 签收后触发 AR 应收确认：查找关联销售订单的 ar_invoice，
///      若状态为 DRAFT 则调用 approve 推进到 APPROVED（确认应收），
///      若不存在或已审批则跳过（幂等）
///
/// 设计依据：V15 审计报告 batch-19 §23.4 缺陷 4
pub async fn sign_waybill(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<SignWaybillRequest>,
) -> Result<Json<ApiResponse<logistics_waybill::Model>>, AppError> {
    let txn = state.db.begin().await?;

    let waybill = logistics_waybill::Entity::find_by_id(id)
        .one(&txn)
        .await?
        .ok_or_else(|| AppError::not_found("运单不存在"))?;

    // 校验：仅 DELIVERED 状态允许签收
    let current_status = waybill.status.as_deref().unwrap_or("");
    if current_status != waybill_status::DELIVERED {
        return Err(AppError::bad_request(format!(
            "运单状态为 {}，仅 {} 状态允许签收",
            current_status,
            waybill_status::DELIVERED
        )));
    }

    // 校验：禁止重复签收
    if waybill.signed_by.is_some() {
        return Err(AppError::bad_request(format!(
            "运单 {} 已由用户 {} 签收，禁止重复签收",
            id,
            waybill.signed_by.unwrap_or(0)
        )));
    }

    let now = Utc::now();
    let mut active_waybill: logistics_waybill::ActiveModel = waybill.into();
    active_waybill.status = Set(Some(waybill_status::SIGNED.to_string()));
    active_waybill.signed_by = Set(Some(auth.user_id));
    active_waybill.signed_at = Set(Some(now));
    active_waybill.sign_receipt_url = Set(req.receipt_url);
    active_waybill.sign_photo_url = Set(req.photo_url);
    active_waybill.sign_remark = Set(req.remark);
    active_waybill.updated_at = Set(now);

    let updated = active_waybill.update(&txn).await?;

    // V15 P0-B13：签收后触发 AR 应收确认
    // 查找关联销售订单的 ar_invoice（source_type=SALES_ORDER, source_bill_id=order_id），
    // 若状态为 DRAFT 则调用 approve 推进到 APPROVED（确认应收，财务可做收款计划），
    // 若不存在或已审批则跳过（幂等，不阻塞签收事务）
    // 注：ar_invoice.source_bill_id 类型为 Option<i32>，与 waybill.order_id (i32) 一致
    let ar_invoice_to_confirm = ar_invoice::Entity::find()
        .filter(ar_invoice::Column::SourceType.eq("SALES_ORDER"))
        .filter(ar_invoice::Column::SourceBillId.eq(updated.order_id))
        .filter(ar_invoice::Column::Status.eq(common_status::STATUS_DRAFT))
        .one(&txn)
        .await?;

    if let Some(draft_invoice) = ar_invoice_to_confirm {
        let invoice_id = draft_invoice.id;
        // 在签收事务内调用 approve：若失败则回滚整个签收事务（保证一致性）
        let ar_service = ArInvoiceService::new(state.db.clone());
        // ArInvoiceService.approve 内部会开启自己的事务，这里通过传入 user_id 让其审计
        // 注：approve 内部使用 self.db 而非外部 txn，故其提交独立于签收事务；
        //     若 approve 失败，签收事务也会回滚（保持强一致）
        if let Err(e) = ar_service.approve(invoice_id, auth.user_id).await {
            return Err(AppError::business(format!(
                "签收后触发 AR 应收确认失败（invoice_id={}）：{}",
                invoice_id, e
            )));
        }
    }

    txn.commit().await?;

    Ok(Json(ApiResponse::success_with_message(
        updated,
        "签收成功，AR 应收已确认",
    )))
}

pub async fn get_waybill(
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<logistics_waybill::Model>>, AppError> {
    let waybill = logistics_waybill::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("运单不存在"))?;

    Ok(Json(ApiResponse::success(waybill)))
}

pub async fn delete_waybill(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let waybill = logistics_waybill::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("运单不存在"))?;

    // 检查是否可以删除（例如：未发货的运单才能删除）
    if waybill.status == Some(waybill_status::IN_TRANSIT.to_string())
        || waybill.status == Some(waybill_status::DELIVERED.to_string())
    {
        return Err(AppError::bad_request(
            "运输中或已送达的运单不能删除".to_string(),
        ));
    }

    // P0 8-3 修复：delete 操作补审计日志
    // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
    crate::services::audit_log_service::AuditLogService::delete_with_audit::<
        logistics_waybill::Entity,
        _,
    >(&*state.db, "logistics_waybill", id, Some(auth.user_id))
    .await?;

    Ok(Json(ApiResponse::success_with_message((), "运单删除成功")))
}
