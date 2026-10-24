use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
use serde::Deserialize;
use validator::Validate;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::location::Entity as LocationEntity;
use crate::models::location::{self as location_model};
use crate::services::warehouse_service::WarehouseService;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};
// V15 P0-S12/P0-S15 修复（Batch 475c）：导出端点使用水印版 xlsx 工具
use crate::utils::xlsx_export::{WatermarkConfig, XlsxTable, build_xlsx_response_with_watermark};
// V15 P0-S11：导出审计日志写入所需依赖
use crate::models::audit_log::{OperationType, Severity};
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use std::sync::Arc;

/// 查询参数 - 仓库列表
// V15 P0-S12 修复（Batch 475c）：派生 Clone，export_warehouses 需要 clone 后覆盖分页参数用于全量导出
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct WarehouseListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub search: Option<String>,
    /// 仓库类型（greige/raw/finished/semi/return），列表页的类型下拉按此值精确筛选
    pub warehouse_type: Option<String>,
}

/// 创建仓库请求
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct CreateWarehouseRequest {
    // alias 对齐前端契约：前端 Warehouse 类型字段为 warehouse_name/warehouse_code
    #[serde(alias = "warehouse_name")]
    #[validate(length(min = 1, max = 100, message = "仓库名称不能为空"))]
    pub name: Option<String>,
    #[serde(alias = "warehouse_code")]
    #[validate(length(min = 1, max = 50, message = "仓库编码不能为空"))]
    pub code: Option<String>,
    pub address: Option<String>,
    pub manager: Option<String>,
    pub phone: Option<String>,
    /// 联系人（契约对齐：前端创建表单 contact_person）
    #[validate(length(max = 100, message = "联系人最长100字符"))]
    pub contact_person: Option<String>,
    /// 默认仓库标志（契约对齐：前端创建表单 is_default 开关）
    pub is_default: Option<bool>,
    /// 仓库容量（批次 158 v11 真实接入：扩展 schema 持久化，原 #[allow(dead_code)] 移除）
    pub capacity: Option<i32>,
    // 批次 93 P1 扩展：description 已接入 WarehouseService::create（写入 notes 列）
    pub description: Option<String>,
    /// 仓库类型：greige=胚布仓（只存未染色/未做工艺的胚布）；finished=成品仓
    /// （只存染色/工艺后成品）；不传=不校验（匹号领域规则）
    #[validate(length(max = 20, message = "仓库类型长度不能超过20个字符"))]
    pub warehouse_type: Option<String>,
}

/// JSON 三态反序列化适配器（RFC 7386 JSON Merge Patch 的"键缺席 ≠ 显式 null"语义所需）。
///
/// 为何需要：serde_json 对 `Option<Option<T>>` 的默认反序列化在遇到 JSON null 时
/// 直接调 visit_none()，把"显式 null"塌成外层 `None`，与"键缺席"不可区分。
/// 本适配器把字段先按内层 `Option<T>` 反序列化再包一层：
/// 键缺席（配合 `#[serde(default)]`）= `None`、显式 null = `Some(None)`、有值 = `Some(Some(v))`。
/// 与 handlers/department_handler.rs 中同名适配器形状一致（跨域合并到共享工具需动
/// utils，超出本批授权范围，各域 handler 内私有定义）。
fn double_option<'de, T, D>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Deserialize::deserialize(de).map(Some)
}

/// 更新仓库请求
///
/// 字段语义 = 显式三态部分更新（对齐 RFC 7386 JSON Merge Patch）：
/// 键缺席=保持原值、显式 `null`=清空为 NULL（仅 DB 可空列）、有值=覆盖。
/// NOT NULL 列（warehouses.name，m0001 DDL；is_default/is_active→status 实体模型
/// 为非 Option bool）不开 null 清空，显式 null 由 service 拒绝。
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateWarehouseRequest {
    /// 仓库名称：NOT NULL 列（m0001 DDL）——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    #[validate(length(min = 1, max = 100, message = "仓库名称不能为空"))]
    pub name: Option<Option<String>>,
    /// DB 可空列 address（warehouses.address，m0001 DDL）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub address: Option<Option<String>>,
    /// DB 可空列 manager_id（字符串形态的仓库经理 ID）——显式 null 清除经理
    #[serde(default, deserialize_with = "double_option")]
    pub manager: Option<Option<String>>,
    /// DB 可空列 phone——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub phone: Option<Option<String>>,
    /// DB 可空列 contact_person——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    #[validate(length(max = 100, message = "联系人最长100字符"))]
    pub contact_person: Option<Option<String>>,
    /// 默认仓库标志：非 Option bool 列——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub is_default: Option<Option<bool>>,
    /// DB 可空列 capacity——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub capacity: Option<Option<i32>>,
    /// 启用状态（映射非 Option 列 is_active）——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub status: Option<Option<String>>,
    /// DB 可空列 warehouse_type——显式 null 清空（=不校验仓型）
    #[serde(default, deserialize_with = "double_option")]
    #[validate(length(max = 20, message = "仓库类型长度不能超过20个字符"))]
    pub warehouse_type: Option<Option<String>>,
}

// define_crud_handlers! 出参实参 = service 的真实返回类型（契约收紧）
// list -> PaginatedResponse<warehouse::Model>，get/create/update -> warehouse::Model
// （证据 services/warehouse_service.rs:29/75/83/242）
crate::define_crud_handlers!(
    WarehouseService,
    CreateWarehouseRequest,
    UpdateWarehouseRequest,
    WarehouseListQuery,
    i32,
    crate::utils::response::PaginatedResponse<crate::models::warehouse::Model>,
    crate::models::warehouse::Model
);

/// 查询参数 - 库位列表
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct LocationListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub warehouse_id: Option<i32>,
}

/// 创建库位请求
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct CreateLocationRequest {
    #[validate(range(min = 1, message = "仓库ID必须大于0"))]
    pub warehouse_id: i32,
    #[validate(length(min = 1, max = 50, message = "库位编码不能为空且最长50字符"))]
    pub location_code: String,
    #[validate(length(max = 20, message = "库位类型最长20字符"))]
    pub location_type: Option<String>,
    pub max_weight: Option<f64>,
    pub max_height: Option<f64>,
}

/// 更新库位请求（字段对齐 warehouse_locations 表结构）
///
/// 字段语义 = 显式三态部分更新（对齐 RFC 7386 JSON Merge Patch）：
/// 键缺席=保持原值、显式 `null`=清空为 NULL（仅 DB 可空列）、有值=覆盖。
/// NOT NULL 列 location_code（m0010 DDL）不开 null 清空，显式 null 在入口拒绝。
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateLocationRequest {
    /// 库位编码：NOT NULL 列（m0010 DDL）——显式 null 被入口拒绝
    #[serde(default, deserialize_with = "double_option")]
    #[validate(length(min = 1, max = 50, message = "库位编码不能为空且最长50字符"))]
    pub location_code: Option<Option<String>>,
    /// 库位类型：DB 可空列 location_type——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    #[validate(length(max = 20, message = "库位类型最长20字符"))]
    pub location_type: Option<Option<String>>,
    /// 最大承重：DB 可空列 max_weight——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub max_weight: Option<Option<f64>>,
    /// 最大高度：DB 可空列 max_height——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub max_height: Option<Option<f64>>,
    /// 是否启用批次管理：DB 可空列 is_batch_managed——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub is_batch_managed: Option<Option<bool>>,
    /// 是否启用色号管理：DB 可空列 is_color_managed——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub is_color_managed: Option<Option<bool>>,
}

/// 获取库位列表
pub async fn list_locations(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(query): Query<LocationListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<serde_json::Value>>>, AppError> {
    let page = query.page.unwrap_or(1).clamp(1, 1000); // 批次 95 P3-3~8：分页 clamp 防 DoS
    let page_size = query.page_size.unwrap_or(10).clamp(1, 100);

    let mut query_builder = LocationEntity::find();

    if let Some(warehouse_id) = query.warehouse_id {
        query_builder = query_builder.filter(location_model::Column::WarehouseId.eq(warehouse_id));
    }

    let paginator = query_builder.paginate(&*state.db, page_size);
    // 批次 98 P2-A 修复（v5 复审）：page clamp 防 DoS
    let locations = paginator
        .fetch_page(page.clamp(1, 1000).saturating_sub(1))
        .await?;
    let total = paginator.num_items().await?;

    let locations_json: Vec<serde_json::Value> = locations
        .into_iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        locations_json,
        total,
        page,
        page_size,
    ))))
}

/// 创建库位
pub async fn create_location(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<CreateLocationRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // v8 P1-C 修复：调用 DTO 验证，激活 Validate 注解
    req.validate()?;
    let active_location = location_model::ActiveModel {
        id: Default::default(),
        warehouse_id: sea_orm::ActiveValue::Set(req.warehouse_id),
        location_code: sea_orm::ActiveValue::Set(req.location_code),
        location_type: sea_orm::ActiveValue::Set(req.location_type),
        max_weight: sea_orm::ActiveValue::Set(
            req.max_weight
                .and_then(rust_decimal::Decimal::from_f64_retain),
        ),
        max_height: sea_orm::ActiveValue::Set(
            req.max_height
                .and_then(rust_decimal::Decimal::from_f64_retain),
        ),
        is_batch_managed: sea_orm::ActiveValue::Set(Some(true)),
        is_color_managed: sea_orm::ActiveValue::Set(Some(true)),
        created_at: Default::default(),
        updated_at: Default::default(),
    };

    let location = active_location.insert(&*state.db).await?;
    let location_json = serde_json::to_value(location)?;
    Ok(Json(ApiResponse::success_with_message(
        location_json,
        "库位创建成功",
    )))
}

/// 获取库位详情
pub async fn get_location(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let location = LocationEntity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("库位不存在"))?;
    let location_json = serde_json::to_value(location)?;
    Ok(Json(ApiResponse::success(location_json)))
}

/// 更新库位（批次 197 P0-1：真实接入字段更新逻辑，原 stub 仅返回原记录）
pub async fn update_location(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpdateLocationRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // NOT NULL 列门控（location_code，m0010 DDL）：显式 null 在任何 DB 访问与
    // 通用校验之前拒绝（错误外显不脱敏，不得塌成"保持原值"）
    if matches!(req.location_code, Some(None)) {
        return Err(AppError::business_displayable(
            "库位编码不能清空：该字段为必填项",
        ));
    }
    req.validate()?;
    let location = LocationEntity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("库位不存在"))?;
    let mut active: location_model::ActiveModel = location.into();

    // 三态写入规则：None=不 Set（UPDATE 不含该列，原值不动）；
    // Some(None)=Set(None) 置 NULL；Some(Some(v))=Set(v) 覆盖
    if let Some(c) = req.location_code.flatten() {
        active.location_code = sea_orm::ActiveValue::Set(c);
    }
    if let Some(t) = req.location_type {
        active.location_type = sea_orm::ActiveValue::Set(t);
    }
    if let Some(w) = req.max_weight {
        active.max_weight = sea_orm::ActiveValue::Set(match w {
            Some(x) => Some(rust_decimal::Decimal::from_f64_retain(x).ok_or_else(|| {
                AppError::validation_displayable(format!(
                    "最大承重数值无法转换（NaN/Inf/溢出）：{x}"
                ))
            })?),
            None => None,
        });
    }
    if let Some(h) = req.max_height {
        active.max_height = sea_orm::ActiveValue::Set(match h {
            Some(x) => Some(rust_decimal::Decimal::from_f64_retain(x).ok_or_else(|| {
                AppError::validation_displayable(format!(
                    "最大高度数值无法转换（NaN/Inf/溢出）：{x}"
                ))
            })?),
            None => None,
        });
    }
    if let Some(b) = req.is_batch_managed {
        active.is_batch_managed = sea_orm::ActiveValue::Set(b);
    }
    if let Some(c) = req.is_color_managed {
        active.is_color_managed = sea_orm::ActiveValue::Set(c);
    }
    active.updated_at = sea_orm::ActiveValue::Set(Utc::now());

    let updated = active.update(&*state.db).await?;
    let location_json = serde_json::to_value(updated)?;
    Ok(Json(ApiResponse::success_with_message(
        location_json,
        "库位更新成功",
    )))
}

/// 删除库位
pub async fn delete_location(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // P0 8-3 修复：delete 操作补审计日志
    // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
    crate::services::audit_log_service::AuditLogService::delete_with_audit::<LocationEntity, _>(
        &*state.db,
        "warehouse_location",
        id,
        Some(auth.user_id),
    )
    .await?;
    Ok(Json(ApiResponse::success_with_message((), "库位删除成功")))
}

// ========== 数据导出接口 ==========

/// 仓库导出表头（15 列）
fn warehouse_export_headers() -> Vec<String> {
    vec![
        "ID".to_string(),
        "仓库编码".to_string(),
        "仓库名称".to_string(),
        "地址".to_string(),
        "城市".to_string(),
        "省份".to_string(),
        "国家".to_string(),
        "电话".to_string(),
        "邮箱".to_string(),
        "经理ID".to_string(),
        "是否启用".to_string(),
        "备注".to_string(),
        "容量".to_string(),
        "创建时间".to_string(),
        "更新时间".to_string(),
    ]
}

/// 从仓库 JSON 对象构建 xlsx 行
fn build_warehouse_row(obj: &serde_json::Map<String, serde_json::Value>) -> Vec<String> {
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
        get_str("warehouse_code"),
        get_str("warehouse_name"),
        get_str("address"),
        get_str("city"),
        get_str("province"),
        get_str("country"),
        get_str("phone"),
        get_str("email"),
        get_str("manager_id"),
        get_str("is_active"),
        get_str("notes"),
        get_str("capacity"),
        get_str("created_at"),
        get_str("updated_at"),
    ]
}

/// 构造仓库列表 xlsx 表格
fn build_warehouses_table(warehouses_json: Vec<serde_json::Value>) -> Result<XlsxTable, AppError> {
    let mut rows: Vec<Vec<String>> = Vec::with_capacity(warehouses_json.len());
    for w in warehouses_json {
        let obj = w
            .as_object()
            .ok_or_else(|| AppError::internal("仓库序列化失败：期望 JSON 对象"))?;
        rows.push(build_warehouse_row(obj));
    }
    Ok(XlsxTable {
        sheet_name: "仓库列表".to_string(),
        headers: warehouse_export_headers(),
        rows,
    })
}

/// 异步记录仓库导出操作（审计自身）
///
/// 功能：把一次仓库导出记为导出类审计事件
/// 调用方：本文件的仓库导出端点
/// 入参：应用状态、会话上下文、实际导出行数、服务端总数、是否取全、当次筛选与文件名
/// 传给谁：转交 AuditLogService::record_async 异步落库
/// 存什么：存操作人、导出条数、服务端总数、是否取全与当次筛选条件
/// 存哪里：审计日志表
fn record_warehouses_export_audit(
    state: &AppState,
    auth: &AuthContext,
    row_count: usize,
    total: u64,
    export_complete: bool,
    query: &WarehouseListQuery,
    filename: &str,
) {
    let completeness = if export_complete {
        "已取全"
    } else {
        "未取全"
    };
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Export,
        severity: Severity::Info,
        resource_type: Some("warehouse".to_string()),
        resource_id: None,
        resource_name: Some(format!("{}.xlsx", filename)),
        description: Some(format!(
            "用户 {} 导出仓库列表（共 {} 条），服务端总数 {} 条，{}",
            auth.username, row_count, total, completeness
        )),
        request_method: Some("GET".to_string()),
        request_path: Some("/api/v1/erp/warehouses/export".to_string()),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "format": "xlsx",
            "total": row_count,
            "server_total": total,
            "complete": export_complete,
            "status_filter": query.status,
            "search_filter": query.search,
            "warehouse_type_filter": query.warehouse_type,
        })),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, None);
}

/// 导出仓库列表为带水印的 xlsx，端点为 GET /api/v1/erp/warehouses/export
///
/// 功能：按当前筛选分批取满全部仓库并落成文件
/// 调用方：仓库路由的导出端点
/// 入参：应用状态、会话上下文、列表查询参数
/// 传给谁：取数交 WarehouseService::list_warehouses_for_export，表格构造交
/// build_warehouses_table，审计交 record_warehouses_export_audit
/// 存什么：不写库；水印行写进导出文件，审计行写审计日志表
/// 存哪里：不适用
///
/// 仓库表无行级数据权限列，导出与列表同口径不做归属过滤。
/// 未取全时把用户下一步该做什么写进水印行，因为水印行是导出文件里用户直接看得到的通道。
pub async fn export_warehouses(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<WarehouseListQuery>,
) -> Result<axum::response::Response, AppError> {
    let service = WarehouseService::new(state.db.clone());

    // 分批取满当前筛选下的全部行，不受对外分页上限约束
    let (warehouses, total) = service.list_warehouses_for_export(&query).await?;
    let row_count = warehouses.len();
    let export_complete = row_count as u64 == total;
    log_warehouses_export_completeness(row_count, total, export_complete);

    // 序列化为 JSON 以统一字段处理
    let warehouses_json: Vec<serde_json::Value> = warehouses
        .into_iter()
        .map(|w| serde_json::to_value(w).map_err(AppError::from))
        .collect::<Result<Vec<_>, _>>()?;

    let table = build_warehouses_table(warehouses_json)?;
    let filename = format!(
        "warehouses_export_{}",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );
    record_warehouses_export_audit(
        &state,
        &auth,
        row_count,
        total,
        export_complete,
        &query,
        &filename,
    );

    // 注入水印（操作员/导出时间/导出条数），未取全时追加下一步动作
    let completeness_note = if export_complete {
        String::new()
    } else {
        "；本次未取全，请缩小筛选范围后重新导出".to_string()
    };
    let watermark = WatermarkConfig {
        operator: Some(auth.username.clone()),
        ip_address: None,
        exported_at: Some(chrono::Utc::now().to_rfc3339()),
        extra: Some(format!(
            "仓库导出（共 {} 条）{}",
            row_count, completeness_note
        )),
    };

    build_xlsx_response_with_watermark(&table, &filename, &watermark)
}

/// 记录仓库导出取数是否取全，结论与实际行数一并落日志
///
/// 功能：把实际导出行数与服务端总数并列记录，取全与未取全分别走不同级别
/// 调用方：本文件的仓库导出端点
/// 入参：实际导出行数、服务端总数、是否取全
/// 传给谁：日志层
/// 存什么：不落库
/// 存哪里：不适用
fn log_warehouses_export_completeness(row_count: usize, total: u64, export_complete: bool) {
    if export_complete {
        tracing::info!(
            "仓库导出已取全: 实际 {} 条, 服务端总数 {} 条",
            row_count,
            total
        );
    } else {
        tracing::warn!(
            "仓库导出未取全: 实际 {} 条, 服务端总数 {} 条（取数期间数据发生变动，或达到分批上界）",
            row_count,
            total
        );
    }
}
