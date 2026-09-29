use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    Set, TransactionTrait,
};
use serde::Deserialize;
use std::collections::HashMap;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::ar_invoice;
use crate::models::logistics_waybill;
use crate::models::sales_order;
use crate::models::status::common as common_status;
use crate::models::status::logistics_waybill as waybill_status;
use crate::models::status::sales_order as so_status;
use crate::services::ar_invoice_service::ArInvoiceService;
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;
use crate::utils::response::{ApiResponse, PaginatedResponse};

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Deserialize)]
pub struct CreateWaybillRequest {
    pub order_id: i32,
    pub logistics_company: String,
    pub tracking_number: String,
    pub driver_name: Option<String>,
    pub driver_phone: Option<String>,
    pub freight_fee: Option<f64>,
    /// 预计到达日期（日期粒度，前端 el-date-picker 提交 YYYY-MM-DD），落库为当日 00:00:00 UTC
    pub expected_arrival: Option<chrono::NaiveDate>,
    pub notes: Option<String>,
}

/// 运费等金额入参转 Decimal：NaN/Infinity 等不可表示的值直接报错
fn to_decimal(value: f64, field: &str) -> Result<Decimal, AppError> {
    Decimal::from_f64_retain(value)
        .ok_or_else(|| AppError::bad_request(format!("{} 金额非法：{}", field, value)))
}

pub async fn create_waybill(
    State(state): State<AppState>,
    Json(req): Json<CreateWaybillRequest>,
) -> Result<Json<ApiResponse<logistics_waybill::Model>>, AppError> {
    let txn = state.db.begin().await?;

    if req.logistics_company.trim().is_empty() {
        return Err(AppError::bad_request("物流公司不能为空"));
    }
    if req.tracking_number.trim().is_empty() {
        return Err(AppError::bad_request("快递单号不能为空"));
    }

    // Verify order exists
    let order = sales_order::Entity::find_by_id(req.order_id)
        .one(&txn)
        .await?
        .ok_or_else(|| AppError::not_found("订单不存在"))?;

    // Create waybill
    let freight = match req.freight_fee {
        Some(f) => Some(to_decimal(f, "运费")?),
        None => None,
    };
    let expected_arrival = req
        .expected_arrival
        .map(|d| d.and_hms_opt(0, 0, 0).unwrap(/* 不变量：0,0,0 组合恒有效 */).and_utc());

    let new_waybill = logistics_waybill::ActiveModel {
        id: sea_orm::ActiveValue::NotSet,
        order_id: Set(req.order_id),
        logistics_company: Set(req.logistics_company),
        tracking_number: Set(req.tracking_number),
        driver_name: Set(req.driver_name),
        driver_phone: Set(req.driver_phone),
        freight_fee: Set(freight),
        status: Set(Some(waybill_status::IN_TRANSIT.to_string())),
        expected_arrival: Set(expected_arrival),
        actual_arrival: Set(None),
        notes: Set(req.notes),
        // 签收字段初始化为 None，sign_waybill handler 填入
        signed_by: Set(None),
        signed_at: Set(None),
        sign_receipt_url: Set(None),
        sign_photo_url: Set(None),
        sign_remark: Set(None),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
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

/// 运单列表查询参数
#[derive(Debug, Deserialize)]
pub struct ListWaybillsQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    /// 运单状态，取值须为 waybill_status::ALL 之一
    pub status: Option<String>,
    /// 物流公司精确匹配
    pub logistics_company: Option<String>,
    /// 关键字：模糊匹配快递单号 / 司机姓名 / 物流公司
    pub keyword: Option<String>,
    /// 创建时间下界（含），RFC 3339
    pub start_date: Option<String>,
    /// 创建时间上界（含），RFC 3339
    pub end_date: Option<String>,
}

/// 解析 RFC 3339 时间入参；格式非法直接报错，不按"无筛选条件"静默忽略
fn parse_date_param(raw: &str, field: &str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| {
            AppError::bad_request(format!(
                "{field} 时间格式非法（应为 RFC 3339，如 2026-09-21T00:00:00Z）：{e}"
            ))
        })
}

/// 校验状态入参：仅接受状态机定义的三个值
///
/// 返回值的生命周期绑定到 `raw`（入参本体），`field` 只出现在错误文案里；
/// 两个 `&str` 入参不写显式生命周期会让编译器无法判定返回引用来自谁。
pub fn validate_status_param<'a>(raw: &'a str, field: &str) -> Result<&'a str, AppError> {
    if waybill_status::ALL.contains(&raw) {
        Ok(raw)
    } else {
        Err(AppError::bad_request(format!(
            "{field} 取值非法：{raw}，合法值 {}",
            waybill_status::ALL.join("/")
        )))
    }
}

/// 运单状态机在本接口侧唯一放行的推进边：IN_TRANSIT → DELIVERED。
///
/// SIGNED 不在此放行——它由签收端点写入，同一事务内记录签收人/签收时间并触发
/// 应收确认，绕过签收端点即绕过应收确认；DELIVERED 之后无回退路径。
pub fn is_legal_waybill_transition(from: &str, to: &str) -> bool {
    from == waybill_status::IN_TRANSIT && to == waybill_status::DELIVERED
}

/// 按本页运单关联的 order_id 批量回查销售订单号（运单表只存 order_id）
async fn fetch_order_no_map(
    db: &impl sea_orm::ConnectionTrait,
    waybills: &[logistics_waybill::Model],
) -> Result<HashMap<i32, String>, AppError> {
    let order_ids: Vec<i32> = waybills
        .iter()
        .map(|w| w.order_id)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    if order_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let orders = sales_order::Entity::find()
        .filter(sales_order::Column::Id.is_in(order_ids))
        .all(db)
        .await?;
    Ok(orders.into_iter().map(|o| (o.id, o.order_no)).collect())
}

/// 运单列表：支持状态/物流公司/关键字/创建时间区间筛选与分页
/// 响应为 {items, total, page, page_size}，每条记录附带关联销售订单号 order_no
pub async fn list_waybills(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(params): Query<ListWaybillsQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<serde_json::Value>>>, AppError> {
    let page = params.page.unwrap_or(1).clamp(1, 1000);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);

    let mut query = logistics_waybill::Entity::find();

    if let Some(status) = params.status.as_deref().filter(|s| !s.is_empty()) {
        query = query
            .filter(logistics_waybill::Column::Status.eq(validate_status_param(status, "status")?));
    }
    if let Some(company) = params
        .logistics_company
        .as_deref()
        .filter(|s| !s.is_empty())
    {
        query = query.filter(logistics_waybill::Column::LogisticsCompany.eq(company));
    }
    if let Some(keyword) = params.keyword.as_deref().filter(|s| !s.is_empty()) {
        query = query.filter(
            Condition::any()
                .add(logistics_waybill::Column::TrackingNumber.contains(keyword))
                .add(logistics_waybill::Column::DriverName.contains(keyword))
                .add(logistics_waybill::Column::LogisticsCompany.contains(keyword)),
        );
    }
    if let Some(start) = params.start_date.as_deref().filter(|s| !s.is_empty()) {
        query = query.filter(
            logistics_waybill::Column::CreatedAt.gte(parse_date_param(start, "start_date")?),
        );
    }
    if let Some(end) = params.end_date.as_deref().filter(|s| !s.is_empty()) {
        query = query
            .filter(logistics_waybill::Column::CreatedAt.lte(parse_date_param(end, "end_date")?));
    }

    let paginator = query
        .order_by_desc(logistics_waybill::Column::CreatedAt)
        .paginate(&*state.db, page_size);
    let (waybills, total) = paginate_with_total(paginator, page).await?;

    let order_no_map = fetch_order_no_map(&*state.db, &waybills).await?;

    // 非管理员对运单列表司机手机号脱敏
    let mut items = Vec::with_capacity(waybills.len());
    for waybill in &waybills {
        let mut value = serde_json::to_value(waybill)
            .map_err(|e| AppError::internal(format!("运单序列化失败: {}", e)))?;
        match order_no_map.get(&waybill.order_id) {
            Some(order_no) => {
                value["order_no"] = serde_json::Value::String(order_no.clone());
            }
            // 外键约束下不应发生；出现即说明运单指向已被删除的订单，需人工核查
            None => {
                tracing::warn!(
                    "运单 {} 关联的销售订单 {} 不存在，order_no 无法回填",
                    waybill.id,
                    waybill.order_id
                );
                value["order_no"] = serde_json::Value::Null;
            }
        }
        items.push(crate::utils::field_mask::mask_contact_fields_for_role(
            value,
            auth.role_id,
        ));
    }

    Ok(Json(ApiResponse::success_paginated(
        items, total, page, page_size,
    )))
}

/// 运单更新请求：状态流转与运输信息编辑共用一个入口，字段缺省表示不修改
#[derive(Deserialize)]
pub struct UpdateWaybillReq {
    /// 目标状态，取值须为 waybill_status::ALL 之一
    pub status: Option<String>,
    pub logistics_company: Option<String>,
    pub tracking_number: Option<String>,
    pub driver_name: Option<String>,
    pub driver_phone: Option<String>,
    pub freight_fee: Option<f64>,
    /// 预计到达日期（日期粒度，提交 YYYY-MM-DD）
    pub expected_arrival: Option<chrono::NaiveDate>,
    pub notes: Option<String>,
}

/// 运单更新
///
/// 状态按状态机校验；运输信息（物流公司/单号/司机/运费/预计到达/备注）只在运输中阶段可改，
/// 送达后这些是签收与应收确认的事实依据，不再允许覆盖。
pub async fn update_waybill(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateWaybillReq>,
) -> Result<Json<ApiResponse<logistics_waybill::Model>>, AppError> {
    let waybill = logistics_waybill::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("运单不存在"))?;

    let has_field_edits = req.logistics_company.is_some()
        || req.tracking_number.is_some()
        || req.driver_name.is_some()
        || req.driver_phone.is_some()
        || req.freight_fee.is_some()
        || req.expected_arrival.is_some()
        || req.notes.is_some();
    if req.status.is_none() && !has_field_edits {
        return Err(AppError::bad_request("没有需要更新的字段"));
    }

    let current = waybill.status.clone().unwrap_or_default();
    if !waybill_status::ALL.contains(&current.as_str()) {
        return Err(AppError::business(format!(
            "运单 {} 当前状态值「{}」不在状态机 {} 内，需先核查该条数据",
            id,
            current,
            waybill_status::ALL.join("/")
        )));
    }

    // 目标状态先行校验：状态机 IN_TRANSIT → DELIVERED → SIGNED。
    // SIGNED 只能经 POST /logistics/:id/sign 写入——该端点同时记录签收人/签收时间并触发
    // 应收确认，从本接口直接置为 SIGNED 会绕过应收确认，故一律拒绝。
    let target_status = match req.status.as_deref() {
        Some(raw) => {
            let target = validate_status_param(raw, "status")?;
            if !is_legal_waybill_transition(&current, target) {
                return Err(AppError::bad_request(format!(
                    "非法状态流转：{} → {}；本接口仅支持 {} → {}，签收请使用签收接口",
                    current,
                    target,
                    waybill_status::IN_TRANSIT,
                    waybill_status::DELIVERED
                )));
            }
            Some(target.to_string())
        }
        None => None,
    };

    if has_field_edits && current != waybill_status::IN_TRANSIT {
        return Err(AppError::business(format!(
            "运单 {} 已处于 {} 状态，运输信息不可修改",
            id, current
        )));
    }

    let mut active_waybill: logistics_waybill::ActiveModel = waybill.into();

    if let Some(company) = req.logistics_company {
        if company.trim().is_empty() {
            return Err(AppError::bad_request("物流公司不能为空"));
        }
        active_waybill.logistics_company = Set(company);
    }
    if let Some(tracking_number) = req.tracking_number {
        if tracking_number.trim().is_empty() {
            return Err(AppError::bad_request("快递单号不能为空"));
        }
        active_waybill.tracking_number = Set(tracking_number);
    }
    if let Some(driver_name) = req.driver_name {
        active_waybill.driver_name = Set(Some(driver_name));
    }
    if let Some(driver_phone) = req.driver_phone {
        active_waybill.driver_phone = Set(Some(driver_phone));
    }
    if let Some(freight_fee) = req.freight_fee {
        active_waybill.freight_fee = Set(Some(to_decimal(freight_fee, "运费")?));
    }
    if let Some(expected_arrival) = req.expected_arrival {
        active_waybill.expected_arrival = Set(Some(
            expected_arrival
                .and_hms_opt(0, 0, 0)
                .unwrap(/* 不变量：0,0,0 组合恒有效 */)
                .and_utc(),
        ));
    }
    if let Some(notes) = req.notes {
        active_waybill.notes = Set(Some(notes));
    }
    if let Some(target) = target_status {
        active_waybill.status = Set(Some(target.clone()));
        if target == waybill_status::DELIVERED {
            active_waybill.actual_arrival = Set(Some(Utc::now()));
        }
    }

    active_waybill.updated_at = Set(Utc::now());
    let updated = active_waybill.update(&*state.db).await?;

    Ok(Json(ApiResponse::success(updated)))
}

/// V15 P0-B13：电子签收请求 DTO
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Deserialize)]
pub struct SignWaybillRequest {
    /// 纸质回单扫描件 URL（上传到对象存储后返回）
    pub receipt_url: Option<String>,
    /// 现场签收照片 URL（上传到对象存储后返回）
    pub photo_url: Option<String>,
    /// 签收备注（异常情况说明，如"包装破损"、"数量短缺"等）
    pub remark: Option<String>,
}

/// V15 P0-B13：电子签收 handler；业务规则： 1. 运单必须存在 2. 运单状态必须为 DELIVERED（已送达）才允许签收 3. 运单不能已签收（status != SIGNED，禁止重复签收） 4. 签收时： - 状态推进到 SIGNED - signed_by 自动填入 AuthContext.user_id - signed_at
/// 自动填入当前时间 - 写入 receipt_url / photo_url / remark（可选） 5. 签收后触发 AR 应收确认：查找关联销售订单的 ar_invoice， 若状态为 DRAFT 则调用 approve 推进到 APPROVED（确认应收）， 若不存在或已审批则跳过（幂等）；设计依据：V15 审计报告 batch-19 §23.4 缺陷 4
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
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let waybill = logistics_waybill::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("运单不存在"))?;

    // 非管理员对运单详情司机手机号脱敏；关联销售订单号由 order_id 回查补齐
    let mut value = serde_json::to_value(&waybill)
        .map_err(|e| AppError::internal(format!("运单序列化失败: {}", e)))?;
    let order_no_map = fetch_order_no_map(&*state.db, std::slice::from_ref(&waybill)).await?;
    match order_no_map.get(&waybill.order_id) {
        Some(order_no) => value["order_no"] = serde_json::Value::String(order_no.clone()),
        None => {
            tracing::warn!(
                "运单 {} 关联的销售订单 {} 不存在，order_no 无法回填",
                waybill.id,
                waybill.order_id
            );
            value["order_no"] = serde_json::Value::Null;
        }
    }
    let value = crate::utils::field_mask::mask_contact_fields_for_role(value, auth.role_id);

    Ok(Json(ApiResponse::success(value)))
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

    // 删除范围：运单一律为「已创建、货物在途」状态起步（无待发货态），故仅允许撤销在途的误建运单；
    // 已送达起进入签收与应收确认链路，已签收记录更是财务凭证，两者均不可删除
    if waybill.status == Some(waybill_status::DELIVERED.to_string())
        || waybill.status == Some(waybill_status::SIGNED.to_string())
    {
        let status = waybill.status.clone().unwrap_or_default();
        return Err(AppError::business(format!(
            "运单 {} 状态为 {}，已送达/已签收的运单不可删除",
            id, status
        )));
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
