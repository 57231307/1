//! 缺失的 Handler 补充
//!
//! 本文件内的所有 API 端点均调用真实数据库 / 业务服务，**不再返回占位数据**。
//!
//! 涵盖 4 个子模块：
//! 1. 会计期间（CRUD + 关闭）— 复用 `accounting_period` 模型
//! 2. MRP 计算历史 — 复用 `MrpEngineService`
//! 3. 销售用户列表 — 复用 `user` 模型 + role 过滤
//! 4. CRM 公海回收规则 — 数据库持久化（批次 23 v5 P0-4：迁移至 `crm_recycle_rules` 表）

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{DateTime, TimeZone, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::middleware::auth_context::AuthContext;
use crate::models::accounting_period;
use crate::models::user;
use crate::services::crm::recycle_rule::{
    CreateRecycleRulePayload, RecycleRule, RecycleRuleService, UpdateRecycleRulePayload,
};
use crate::services::mrp_engine_service::MrpEngineService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::messages::biz_msg;
use crate::utils::response::ApiResponse;

// ============================================================================
// 1. 会计期间（Accounting Period）
// ============================================================================

/// 会计期间列表响应 DTO
#[derive(Debug, Serialize)]
pub struct AccountingPeriodDto {
    pub id: i32,
    pub year: i32,
    pub period: i32,
    pub period_name: String,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub status: String,
    pub closed_at: Option<DateTime<Utc>>,
    pub closed_by: Option<i32>,
    pub created_at: DateTime<Utc>,
}

impl From<accounting_period::Model> for AccountingPeriodDto {
    fn from(m: accounting_period::Model) -> Self {
        Self {
            id: m.id,
            year: m.year,
            period: m.period,
            period_name: m.period_name,
            start_date: m.start_date,
            end_date: m.end_date,
            status: m.status,
            closed_at: m.closed_at,
            closed_by: m.closed_by,
            created_at: m.created_at,
        }
    }
}

/// 获取会计期间列表（按年-月倒序）
pub async fn get_accounting_periods(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<AccountingPeriodDto>>>, AppError> {
    let periods = accounting_period::Entity::find()
        .order_by_desc(accounting_period::Column::Year)
        .order_by_desc(accounting_period::Column::Period)
        .all(state.db.as_ref())
        .await?;
    let dtos: Vec<AccountingPeriodDto> = periods.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(dtos)))
}

/// 获取单个会计期间详情
pub async fn get_accounting_period_detail(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<AccountingPeriodDto>>, AppError> {
    let period = accounting_period::Entity::find_by_id(id)
        .one(state.db.as_ref())
        .await?
        .ok_or_else(|| AppError::not_found(format!("会计期间 {} 不存在", id)))?;
    Ok(Json(ApiResponse::success(AccountingPeriodDto::from(
        period,
    ))))
}

/// 创建会计期间请求
#[derive(Debug, Deserialize, Validate)]
pub struct CreateAccountingPeriodPayload {
    pub year: i32,
    #[validate(range(min = 1, max = 12, message = "期间必须在 1-12 之间"))]
    pub period: i32,
}

/// 创建会计期间
pub async fn create_accounting_period(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(payload): Json<CreateAccountingPeriodPayload>,
) -> Result<Json<ApiResponse<AccountingPeriodDto>>, AppError> {
    payload.validate()?;

    // 防止重复创建
    if accounting_period::Entity::find()
        .filter(accounting_period::Column::Year.eq(payload.year))
        .filter(accounting_period::Column::Period.eq(payload.period))
        .one(state.db.as_ref())
        .await?
        .is_some()
    {
        return Err(AppError::business(format!(
            "{} 年 {:02} 月的会计期间已存在",
            payload.year, payload.period
        )));
    }

    // 计算起止时间
    let month = payload.period as u32;
    let start_date = chrono::Utc
        .with_ymd_and_hms(payload.year, month, 1, 0, 0, 0)
        .single()
        .ok_or_else(|| AppError::bad_request("非法的起始日期"))?;
    let (next_year, next_month) = if month == 12 {
        (payload.year + 1, 1u32)
    } else {
        (payload.year, month + 1)
    };
    let end_date = chrono::Utc
        .with_ymd_and_hms(next_year, next_month, 1, 0, 0, 0)
        .single()
        .ok_or_else(|| AppError::bad_request("非法的结束日期"))?
        - chrono::Duration::seconds(1);

    let active_model = accounting_period::ActiveModel {
        year: Set(payload.year),
        period: Set(payload.period),
        period_name: Set(format!("{} 年 {:02} 月", payload.year, payload.period)),
        start_date: Set(start_date),
        end_date: Set(end_date),
        status: Set("OPEN".to_string()),
        created_at: Set(Utc::now()),
        ..Default::default()
    };

    let period = active_model.insert(state.db.as_ref()).await?;
    Ok(Json(ApiResponse::success_with_message(
        AccountingPeriodDto::from(period),
        biz_msg::CREATE_OK,
    )))
}

/// 更新会计期间（暂只支持重命名）
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateAccountingPeriodPayload {
    #[validate(length(min = 1, max = 50, message = "期间名称不能为空"))]
    pub period_name: Option<String>,
    pub status: Option<String>,
}

/// 更新会计期间
pub async fn update_accounting_period(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
    Json(payload): Json<UpdateAccountingPeriodPayload>,
) -> Result<Json<ApiResponse<AccountingPeriodDto>>, AppError> {
    payload.validate()?;

    let period = accounting_period::Entity::find_by_id(id)
        .one(state.db.as_ref())
        .await?
        .ok_or_else(|| AppError::not_found(format!("会计期间 {} 不存在", id)))?;

    if period.status == "CLOSED" {
        return Err(AppError::business("已结账的会计期间不可修改".to_string()));
    }

    let mut active: accounting_period::ActiveModel = period.into();
    if let Some(name) = payload.period_name {
        active.period_name = Set(name);
    }
    if let Some(status) = payload.status {
        if status != "OPEN" && status != "CLOSING" {
            return Err(AppError::bad_request("status 仅支持 OPEN/CLOSING"));
        }
        active.status = Set(status);
    }
    let updated = active.update(state.db.as_ref()).await?;
    Ok(Json(ApiResponse::success_with_message(
        AccountingPeriodDto::from(updated),
        biz_msg::UPDATE_OK,
    )))
}

/// 删除会计期间
pub async fn delete_accounting_period(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let period = accounting_period::Entity::find_by_id(id)
        .one(state.db.as_ref())
        .await?
        .ok_or_else(|| AppError::not_found(format!("会计期间 {} 不存在", id)))?;

    if period.status == "CLOSED" {
        return Err(AppError::business("已结账的会计期间不可删除".to_string()));
    }

    // 不允许删除存在凭证的期间
    let voucher_count = crate::models::voucher::Entity::find()
        .filter(crate::models::voucher::Column::VoucherDate.gte(period.start_date))
        .filter(crate::models::voucher::Column::VoucherDate.lte(period.end_date))
        .count(state.db.as_ref())
        .await?;
    if voucher_count > 0 {
        return Err(AppError::business(format!(
            "该会计期间存在 {} 张凭证，不可删除",
            voucher_count
        )));
    }

    let active: accounting_period::ActiveModel = period.into();
    active.delete(state.db.as_ref()).await?;
    Ok(Json(ApiResponse::success_with_message((), biz_msg::DELETE_OK)))
}

// ============================================================================
// 2. MRP 历史
// ============================================================================

/// MRP 历史列表响应
#[derive(Debug, Serialize)]
pub struct MrpHistoryDto {
    pub calculation_no: String,
    pub product_id: i32,
    pub required_quantity: rust_decimal::Decimal,
    pub required_date: Option<chrono::NaiveDate>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

/// MRP 历史列表查询参数
#[derive(Debug, Deserialize)]
pub struct MrpHistoryListQuery {
    pub calculation_no: Option<String>,
    pub product_id: Option<i32>,
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 获取 MRP 计算历史列表
pub async fn get_mrp_history(
    State(state): State<AppState>,
    _auth: AuthContext,
    axum::extract::Query(params): axum::extract::Query<MrpHistoryListQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let page = params.page.unwrap_or(1).clamp(1, 1000);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);

    let mrp_service = MrpEngineService::new(state.db.clone());
    let (results, total) = mrp_service
        .get_results(
            params.calculation_no,
            params.product_id,
            params.status,
            page,
            page_size,
        )
        .await?;

    let dtos: Vec<MrpHistoryDto> = results
        .into_iter()
        .map(|r| MrpHistoryDto {
            calculation_no: r.calculation_no,
            product_id: r.product_id,
            required_quantity: r.required_quantity,
            required_date: r.required_date,
            status: r.status,
            created_at: r.created_at,
        })
        .collect();

    let response = serde_json::json!({
        "items": dtos,
        "total": total,
        "page": page,
        "page_size": page_size,
    });
    Ok(Json(ApiResponse::success(response)))
}

/// MRP 历史详情响应
#[derive(Debug, Serialize)]
pub struct MrpHistoryDetailDto {
    pub calculation_no: String,
    pub product_id: i32,
    pub required_quantity: rust_decimal::Decimal,
    pub required_date: Option<chrono::NaiveDate>,
    pub source_type: String,
    pub source_id: Option<i32>,
    pub planned_order_quantity: Option<rust_decimal::Decimal>,
    pub planned_order_date: Option<chrono::NaiveDate>,
    pub status: String,
    pub remarks: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// 获取 MRP 历史详情
pub async fn get_mrp_history_detail(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<MrpHistoryDetailDto>>, AppError> {
    // 直接查询 mrp_result 模型
    let result = crate::models::mrp_result::Entity::find_by_id(id)
        .one(state.db.as_ref())
        .await?
        .ok_or_else(|| AppError::not_found(format!("MRP 历史 {} 不存在", id)))?;

    let dto = MrpHistoryDetailDto {
        calculation_no: result.calculation_no,
        product_id: result.product_id,
        required_quantity: result.required_quantity,
        required_date: result.required_date,
        source_type: result.source_type,
        source_id: result.source_id,
        planned_order_quantity: result.planned_order_quantity,
        planned_order_date: result.planned_order_date,
        status: result.status,
        remarks: result.remarks,
        created_at: result.created_at,
    };
    Ok(Json(ApiResponse::success(dto)))
}

// ============================================================================
// 3. 销售用户列表
// ============================================================================

/// 销售用户 DTO
#[derive(Debug, Serialize)]
pub struct SalesUser {
    pub id: i32,
    pub username: String,
    pub real_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
}

/// 获取销售用户列表（role_id 对应"销售"角色的活跃用户）
pub async fn get_sales_users(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<SalesUser>>>, AppError> {
    // 查找角色名包含"销售"的 role
    let sales_role_ids: Vec<i32> = crate::models::role::Entity::find()
        .filter(crate::models::role::Column::Name.contains("销售"))
        .all(state.db.as_ref())
        .await?
        .into_iter()
        .map(|r| r.id)
        .collect();

    if sales_role_ids.is_empty() {
        return Ok(Json(ApiResponse::success(vec![])));
    }

    let users = user::Entity::find()
        .filter(user::Column::RoleId.is_in(sales_role_ids))
        .filter(user::Column::IsActive.eq(true))
        .order_by_asc(user::Column::Username)
        .all(state.db.as_ref())
        .await?;

    let dtos: Vec<SalesUser> = users
        .into_iter()
        .map(|u| SalesUser {
            id: u.id,
            username: u.username.clone(),
            // P2 1-9 修复：原 Some(u.username) 将登录账号误写入真实姓名字段
            // User 模型当前无 real_name 字段，置为 None 避免语义错误
            real_name: None,
            email: u.email,
            phone: u.phone,
        })
        .collect();
    Ok(Json(ApiResponse::success(dtos)))
}

// ============================================================================
// 4. CRM 公海回收规则（数据库持久化）
// ============================================================================
// 批次 23 v5 P0-4 修复：原 `static RECYCLE_RULES: LazyLock<RwLock<Vec<RecycleRule>>>`
// 内存存储会导致进程重启后丢失运行时修改，已迁移至数据库 `crm_recycle_rules` 表。
//
// 类型 `RecycleRule` / `CreateRecycleRulePayload` / `UpdateRecycleRulePayload` 已
// 下沉至 `services/crm/recycle_rule.rs`，本文件仅保留 handler 薄封装，
// 具体 CRUD 逻辑由 `RecycleRuleService` 承载。

/// 获取回收规则列表
pub async fn get_recycle_rules(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<RecycleRule>>>, AppError> {
    let service = RecycleRuleService::new(state.db.clone());
    let rules = service.list_rules().await?;
    Ok(Json(ApiResponse::success(rules)))
}

/// 创建回收规则
pub async fn create_recycle_rule(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(payload): Json<CreateRecycleRulePayload>,
) -> Result<Json<ApiResponse<RecycleRule>>, AppError> {
    payload.validate()?;
    let service = RecycleRuleService::new(state.db.clone());
    let new_rule = service.create_rule(payload).await?;
    Ok(Json(ApiResponse::success_with_message(
        new_rule,
        "回收规则创建成功",
    )))
}

/// 更新回收规则
pub async fn update_recycle_rule(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
    Json(payload): Json<UpdateRecycleRulePayload>,
) -> Result<Json<ApiResponse<RecycleRule>>, AppError> {
    payload.validate()?;
    let service = RecycleRuleService::new(state.db.clone());
    let rule = service.update_rule(id, payload).await?;
    Ok(Json(ApiResponse::success_with_message(
        rule,
        "回收规则更新成功",
    )))
}

/// 删除回收规则
pub async fn delete_recycle_rule(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = RecycleRuleService::new(state.db.clone());
    // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
    service.delete_rule(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success_with_message(
        (),
        "回收规则删除成功",
    )))
}
