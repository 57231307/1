//! 缸号管理Handler（染色批次管理）

use axum::{
    Json,
    extract::{Path, Query, State},
};
use rust_decimal::Decimal;
use sea_orm::prelude::DateTimeWithTimeZone;
use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, ColumnTrait, ConnectionTrait, EntityTrait,
    FromQueryResult, JoinType, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, RelationTrait,
    Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
// V15 P0-S11：导出审计日志写入所需依赖
use crate::models::audit_log::{OperationType, Severity};
use crate::models::color_card_item;
use crate::models::dye_batch;
use crate::models::greige_fabric;
use crate::models::status::quality_dyeing::dye_batch_lifecycle_status as batch_status;
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;
use crate::utils::response::{ApiResponse, PaginatedResponse};
use crate::utils::xlsx_export::{XlsxTable, build_xlsx_response};
use std::sync::Arc;

use crate::services::dye_batch_state_machine_validation;

/// 缸号（dye_batch.batch_no）自动编码前缀：沿用原手写格式 "DB-{时间戳}-{随机}"
/// 的业务前缀 DB，新格式统一为 {DB}{YYYYMMDD}{3位流水}（前缀集中定义，禁止散落）。
pub const DYE_BATCH_NO_PREFIX: &str = "DB";

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct DyeBatchListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub status: Option<String>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct CreateDyeBatchRequest {
    pub batch_no: Option<String>,
    pub greige_fabric_id: Option<i32>,
    // 色号：空（None/空白）= 白坯；非空 = 染色布，必须能在色卡档案反查到。
    pub color_no: Option<String>,
    // 染色批号：染色布（color_no 非空）必填，缺失显式拒绝；白坯归一为空串。禁止占位假值。
    pub dye_lot_no: Option<String>,
    pub planned_quantity: Option<f64>,
    pub status: Option<String>,
    // 备注：与实体列同名，落库 dye_batch.remarks。
    pub remarks: Option<String>,
    // 染色日期：新建表单采集，落库映射到既有 started_at 时间戳列（无独立日期列）。
    pub dye_date: Option<chrono::NaiveDate>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct UpdateDyeBatchRequest {
    pub greige_fabric_id: Option<i32>,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub planned_quantity: Option<f64>,
    pub status: Option<String>,
    // 备注：与实体列同名，落库 dye_batch.remarks。
    pub remarks: Option<String>,
}

/// 完工登记请求（任务 #168）：完工时强制登记实际产出三值，粒度 kg + 米 + 坯布投料量。
/// 依据：.monkeycode/docs/research/fabric-industry-research.md:149-158 缸号承载"最终落布重量"；印染完工申报必登记
/// 实际产量（单位成本/单位能耗均以产量为分母）。三值必填、为正、≤10 亿、最多 2 位小数
/// （DECIMAL(12,2) 列精度），复用全仓统一范围校验 `utils::validator::validate_amount_range`
/// （先例：ar_payment_handler.rs:33）。产出/投料比不做拒绝门：仓内唯一权威失重率口径是
/// 委外域"正常/异常损耗"核算分类标准（outsourcing_service.rs:76-97 染色 5%，§5.7 行业中值），
/// 语义为损耗分类而非完工拒绝边界，硬套会误伤真实发生的异常损耗完工；
/// 完工拒绝阈值待产品给口径（见交付报告登记项）。
#[derive(Debug, Deserialize, Validate)]
pub struct CompleteDyeBatchRequest {
    #[validate(custom(function = "crate::utils::validator::validate_amount_range"))]
    pub actual_output_kg: Decimal,
    #[validate(custom(function = "crate::utils::validator::validate_amount_range"))]
    pub actual_output_m: Decimal,
    #[validate(custom(function = "crate::utils::validator::validate_amount_range"))]
    pub greige_input_kg: Decimal,
}

/// 缸号列表出参 DTO：实体 `dye_batch::Model` 全字段 + 经 LEFT JOIN 富化的坯布名称。
/// 严格对齐前端 `api/dye-batch.ts::DyeBatch` 键集；可空列以 `Option` 表达，NOT NULL 列不用 `Option`。
/// 唯一富化方式：`column_as(greige_fabric.fabric_name, "greige_fabric_name")` + `LeftJoin`
/// + `into_model`（单次查询、无 N+1，见 §5 范式），禁止 `format!` 造假名或逐行再查。
#[derive(Debug, Clone, Serialize, FromQueryResult)]
pub struct DyeBatchDto {
    pub id: i32,
    pub batch_no: String,
    pub greige_fabric_id: Option<i32>,
    pub color_code: String,
    pub color_name: String,
    pub color_no: Option<String>,
    pub dye_lot_no: String,
    pub planned_quantity: Option<Decimal>,
    pub actual_output_kg: Option<Decimal>,
    pub actual_output_m: Option<Decimal>,
    pub greige_input_kg: Option<Decimal>,
    pub status: Option<String>,
    pub started_at: Option<DateTimeWithTimeZone>,
    pub completed_at: Option<DateTimeWithTimeZone>,
    pub is_deleted: Option<bool>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub remarks: Option<String>,
    pub greige_fabric_name: Option<String>,
}

pub async fn list_dye_batches(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<DyeBatchListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<DyeBatchDto>>>, AppError> {
    let _data_scope = auth.to_data_scope_context();
    let page = query.page.unwrap_or(1).clamp(1, 1000); // 批次 95 P3-3~8：分页 clamp 防 DoS
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

    // §5 范式：LEFT JOIN 坯布表，把 fabric_name 以列别名 greige_fabric_name 富化进同一查询，
    // 单次查询、无 N+1；禁止逐行再查或 format! 造假名。
    let mut q = dye_batch::Entity::find()
        .column_as(greige_fabric::Column::FabricName, "greige_fabric_name")
        .join(JoinType::LeftJoin, dye_batch::Relation::GreigeFabric.def())
        .filter(dye_batch::Column::IsDeleted.eq(false));

    if let Some(batch_no) = &query.batch_no {
        q = q.filter(dye_batch::Column::BatchNo.contains(batch_no));
    }
    if let Some(color_no) = &query.color_no {
        q = q.filter(dye_batch::Column::ColorNo.contains(color_no));
    }
    if let Some(dye_lot_no) = &query.dye_lot_no {
        q = q.filter(dye_batch::Column::DyeLotNo.contains(dye_lot_no));
    }
    if let Some(status) = &query.status {
        q = q.filter(dye_batch::Column::Status.eq(status));
    }

    q = q.order_by_desc(dye_batch::Column::CreatedAt);

    let paginator = q
        .into_model::<DyeBatchDto>()
        .paginate(&*state.db, page_size);
    let total = paginator.num_items().await?;
    // 批次 98 P2-A 修复（v5 复审）：page clamp 防 DoS
    let batches = paginator
        .fetch_page(page.clamp(1, 1000).saturating_sub(1))
        .await?;
    Ok(Json(ApiResponse::success_paginated(
        batches, total, page, page_size,
    )))
}

pub async fn get_dye_batch(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<dye_batch::Model>>, AppError> {
    let batch = dye_batch::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("缸号不存在"))?;
    Ok(Json(ApiResponse::success(batch)))
}

/// 缸号染色身份解析结果（color_no / color_code / color_name / dye_lot_no 四列的唯一落库来源）。
#[derive(Debug, Clone)]
pub struct ResolvedDyeIdentity {
    pub color_no: Option<String>,
    pub color_code: String,
    pub color_name: String,
    pub dye_lot_no: String,
}

/// 新建链路白坯/染色身份归一（唯一入口，禁止任何造假默认值：值只能来自用户提交或主数据派生）。
///
/// 口径与 `services/inv/fabric_class.rs:25-65`（白坯/染色判定全仓唯一实现）同型：
/// - `color_no` 为空（None/纯空白，trim 归一）⇒ 白坯：色号的真实表示就是"没有颜色"，
///   `color_code`/`color_name` 为 NOT NULL 列，白坯以空串落库表达（同源收口先例：
///   `handlers/inventory_stock_handler_dto.rs:19` "白坯空值以空串表达，落库列 NOT NULL"、
///   `services/purchase_return_service.rs` 三维归一空串口径）；白坯免缸号 ⇒ `dye_lot_no`
///   归一为空串（`handlers/inventory_stock_handler_fabric.rs:137-160`：白坯携带染缸料将
///   永久提不出，故主动归一，不得回填占位值）。
/// - `color_no` 非空 ⇒ 染色布：`dye_lot_no` 必填，缺失即显式 400（fabric_class:54-58 同文案
///   "染色布必须提供缸号"）；`color_code`/`color_name` 由色号主数据派生——按 color_code
///   全局反查色卡明细（查询形态同 `services/color_card_scan_service.rs:71-82`，本仓唯一
///   不带 product_id 的色号→名称权威查找；`product_colors` 是产品维度档案，dye_batch 无
///   product_id，不可用作反查源）。档案无此色 ⇒ 显式 400（文案只回显用户提交的色号）；
///   同色号多条记录无法唯一定位 ⇒ 显式业务错，不任选不兜底（四维歧义报业务错同源口径）。
pub async fn resolve_dye_color_identity<C: ConnectionTrait>(
    db: &C,
    color_no: Option<String>,
    dye_lot_no: Option<String>,
) -> Result<ResolvedDyeIdentity, AppError> {
    let color_no = color_no
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let dye_lot_no = dye_lot_no
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let Some(color_no) = color_no else {
        return Ok(ResolvedDyeIdentity {
            color_no: None,
            color_code: String::new(),
            color_name: String::new(),
            dye_lot_no: String::new(),
        });
    };

    let dye_lot_no = dye_lot_no.ok_or_else(|| {
        AppError::validation_displayable(format!(
            "染色布必须提供缸号（color_no={color_no} 但 dye_lot_no 为空）"
        ))
    })?;

    let items = color_card_item::Entity::find()
        .filter(color_card_item::Column::ColorCode.eq(&color_no))
        .all(db)
        .await?;
    let item = match items.as_slice() {
        [] => {
            return Err(AppError::validation_displayable(format!(
                "色号 {color_no} 在色卡档案中不存在"
            )));
        }
        [only] => only.clone(),
        _ => {
            return Err(AppError::business_displayable(format!(
                "色号 {color_no} 在色卡档案中存在多条记录，无法唯一定位名称"
            )));
        }
    };

    Ok(ResolvedDyeIdentity {
        color_no: Some(color_no),
        color_code: item.color_code,
        color_name: item.color_name,
        dye_lot_no,
    })
}

pub async fn create_dye_batch(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<CreateDyeBatchRequest>,
) -> Result<Json<ApiResponse<dye_batch::Model>>, AppError> {
    // 验证状态值（14 态 lifecycle_status 英文 key）
    let status = match req.status {
        Some(s) => {
            if !dye_batch_state_machine_validation::is_valid_status(&s) {
                return Err(AppError::bad_request(format!("无效的缸号状态：{}", s)));
            }
            Some(s)
        }
        None => Some(batch_status::PENDING_SCHEDULE.to_string()),
    };

    // 染色身份归一：值只能来自用户提交或主数据派生，禁止造假默认值。旧形态在色号缺失时
    // 把 color_code/color_name/dye_lot_no 回退为占位假值（测试用字面量与写死的假色名），
    // 假数据会出现在列表、打印单据与成本归集四维标识
    //（services/dye_batch_cost_bridge_service.rs:129-175）上。
    // 术语：dye_lot_no（染色批号）与 batch_no（缸号）是两个概念、非同一值
    //（models/dye_batch.rs:20-21 与 dye_batch_cost_bridge_service.rs:142 术语注释），
    // 故新建时染色批号由表单真实采集，缺失即显式拒绝，不从 batch_no 派生、不写占位值。
    let identity =
        resolve_dye_color_identity(&*state.db, req.color_no.clone(), req.dye_lot_no.clone())
            .await?;

    // 染色日期：新建表单采集的 dye_date 落库映射到既有 started_at 时间戳列（00:00:00 UTC）。
    // 说明：dye_batch 无独立的"染色日期"DATE 列，此处按实体既有语义写入 started_at；
    // 若需要精确到日且与起止时间语义分离，应新增真实 DATE 列（见交付报告）。
    let started_at = req.dye_date.map(|d| {
        d.and_hms_opt(0, 0, 0)
            .unwrap_or_default()
            .and_utc()
            .with_timezone(&crate::utils::date_utils::utc_offset())
    });

    // 缸号落库形态集中构建：手工传入与自动生成两条插入路径共用同一 ActiveModel 组装，
    // 避免两条路径字段漂移。
    let build_active = |batch_no: String| dye_batch::ActiveModel {
        id: NotSet,
        batch_no: Set(batch_no),
        greige_fabric_id: Set(req.greige_fabric_id),
        color_code: Set(identity.color_code.clone()),
        color_name: Set(identity.color_name.clone()),
        color_no: Set(identity.color_no.clone()),
        dye_lot_no: Set(identity.dye_lot_no.clone()),
        planned_quantity: Set(req.planned_quantity.and_then(Decimal::from_f64_retain)),
        // 完工实际产出三列仅由 complete 端点登记，新建时为 NULL（真实空值，不占位）
        actual_output_kg: Set(None),
        actual_output_m: Set(None),
        greige_input_kg: Set(None),
        status: Set(status.clone()),
        started_at: Set(started_at),
        completed_at: Set(None),
        remarks: Set(req.remarks.clone()),
        is_deleted: Set(Some(false)),
        created_at: Set(crate::utils::date_utils::utc_now_fixed()),
        updated_at: Set(crate::utils::date_utils::utc_now_fixed()),
    };

    // 自动生成缸号：取号与主表 INSERT 收口到同一写入事务。
    // 为什么：dye_batch.batch_no 带 UNIQUE 约束（migration/src/domain/system/
    // m0003_add_dye_tables.rs:12 `"batch_no" VARCHAR(50) NOT NULL UNIQUE`），
    // 旧手写 "DB-{14位时间戳}-{4位随机}" 同秒并发碰撞概率非零，撞约束即 500。
    // 现走 DocumentNumberGenerator::insert_with_no_retry：pg_advisory_xact_lock
    // 事务内取号 + INSERT 撞 23505 时在保存点内重新取号重试
    //（用法参照 services/so/order_crud.rs:125）。
    let txn = (*state.db).begin().await?;
    let provided_no = req
        .batch_no
        .clone()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let created = match provided_no {
        // 调用方显式传入非空缸号时尊重手工值（既有契约不变）；若与存量重复，
        // UNIQUE 违规经 AppError 的 DbErr 映射显式上抛，不静默改写、不兜底。
        Some(batch_no) => build_active(batch_no).insert(&txn).await?,
        None => DocumentNumberGenerator::insert_with_no_retry(
            &txn,
            DYE_BATCH_NO_PREFIX,
            dye_batch::Entity,
            dye_batch::Column::BatchNo,
            build_active,
        )
        .await
        .map_err(|e| {
            // fail-visible：记录根因。生成器取号类失败自带 business_displayable
            // 出参；INSERT 的非唯一约束 SQL 错误（如 FK 缺行）责任在数据/引用参数，
            // 原样上抛避免被"缸号生成失败"误导。
            tracing::error!(
                error = %e,
                prefix = DYE_BATCH_NO_PREFIX,
                "缸号取号或插入失败（create_dye_batch）"
            );
            e
        })?,
    };
    txn.commit().await?;

    Ok(Json(ApiResponse::success_with_message(
        created,
        "缸号创建成功",
    )))
}

pub async fn update_dye_batch(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
    Json(req): Json<UpdateDyeBatchRequest>,
) -> Result<Json<ApiResponse<dye_batch::Model>>, AppError> {
    let model = dye_batch::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("缸号不存在"))?;
    // 从 Model 读取当前状态（Model -> ActiveModel 转换后值语义为 Unchanged，
    // 用 ActiveValue::Set 匹配会恒落到默认分支，导致状态流转前置校验失真）
    let current_status = model
        .status
        .clone()
        .unwrap_or_else(|| batch_status::PENDING_SCHEDULE.to_string());
    let mut batch: dye_batch::ActiveModel = model.into();

    if let Some(greige_fabric_id) = req.greige_fabric_id {
        batch.greige_fabric_id = Set(Some(greige_fabric_id));
    }
    if let Some(color_no) = req.color_no {
        batch.color_no = Set(Some(color_no));
    }
    if let Some(dye_lot_no) = req.dye_lot_no {
        batch.dye_lot_no = Set(dye_lot_no);
    }
    if let Some(planned_quantity) = req.planned_quantity {
        batch.planned_quantity = Set(Decimal::from_f64_retain(planned_quantity));
    }
    if let Some(remarks) = req.remarks {
        batch.remarks = Set(Some(remarks));
    }
    if let Some(status) = req.status {
        // 验证状态值合法性（14 态 lifecycle_status）
        if !dye_batch_state_machine_validation::is_valid_status(&status) {
            return Err(AppError::bad_request(format!("无效的状态：{}", status)));
        }
        // 验证状态流转合法性
        if !dye_batch_state_machine_validation::is_valid_status_transition(&current_status, &status)
        {
            return Err(AppError::business(format!(
                "状态流转不合法：{} -> {}",
                current_status, status
            )));
        }

        batch.status = Set(Some(status.clone()));

        // 自动设置时间戳：染色中及之后工序记录开始时间
        let in_production = matches!(
            status.as_str(),
            batch_status::PREPARING
                | batch_status::DYEING
                | batch_status::WASHING
                | batch_status::FIXING
                | batch_status::DEHYDRATING
                | batch_status::DRYING
                | batch_status::INSPECTING
        );
        if in_production {
            let needs_start_time = batch.started_at.as_ref().is_none();
            if needs_start_time {
                batch.started_at = Set(Some(crate::utils::date_utils::utc_now_fixed()));
            }
        }
        // 入库及之后状态记录完成时间
        let is_finished = matches!(
            status.as_str(),
            batch_status::STORED | batch_status::SHIPPED
        );
        if is_finished {
            batch.completed_at = Set(Some(crate::utils::date_utils::utc_now_fixed()));
        }
    }

    batch.updated_at = Set(crate::utils::date_utils::utc_now_fixed());

    let updated = batch.update(&*state.db).await?;
    Ok(Json(ApiResponse::success_with_message(
        updated,
        "缸号更新成功",
    )))
}

pub async fn delete_dye_batch(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // 检查缸号状态，生产中的缸号不允许删除
    let batch = dye_batch::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("缸号不存在"))?;

    if matches!(
        batch.status.as_deref(),
        Some(batch_status::PREPARING)
            | Some(batch_status::DYEING)
            | Some(batch_status::WASHING)
            | Some(batch_status::FIXING)
            | Some(batch_status::DEHYDRATING)
            | Some(batch_status::DRYING)
            | Some(batch_status::INSPECTING)
    ) {
        return Err(AppError::business_displayable(
            "生产中的缸号不允许删除，请先取消或完成",
        ));
    }

    // 软删除
    let mut active: dye_batch::ActiveModel = batch.into();
    active.is_deleted = Set(Some(true));
    active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());

    active.update(&*state.db).await?;
    Ok(Json(ApiResponse::success_with_message((), "缸号删除成功")))
}

/// 完工登记：强制采集实际产出三值（kg/米/坯布投料量）→ 落库三列 → 走既有 14 态状态机
/// 流转至 stored。校验顺序：先输入校验（validate_amount_range，失败 400 且不推进状态），
/// 再状态机门控（沿用本 handler 既有 business 族，不改错误族选择）。
/// 落库后的实际产量是成本归集/能耗分摊分母的唯一来源
/// （services/dye_batch_cost_bridge_service.rs 回填 cost_collection.output_quantity_*）。
pub async fn complete_dye_batch(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
    Json(req): Json<CompleteDyeBatchRequest>,
) -> Result<Json<ApiResponse<dye_batch::Model>>, AppError> {
    // 输入校验先于状态推进：非法产出值一律 validation_displayable（只回显用户提交数值），
    // 绝不静默推进状态。From<ValidationErrors> 已统一映射为可外显 400（utils/error.rs:449）。
    req.validate().map_err(AppError::from)?;

    let model = dye_batch::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("缸号不存在"))?;
    // 与 update_dye_batch 一致：从 Model 读当前状态，Model -> ActiveModel 后值语义非 Set
    let current_status = model
        .status
        .clone()
        .unwrap_or_else(|| batch_status::PENDING_SCHEDULE.to_string());
    let greige_input_kg = req.greige_input_kg;
    let actual_output_kg = req.actual_output_kg;
    let mut batch: dye_batch::ActiveModel = model.into();

    // 检查当前状态是否允许完成（流转到 stored 终态前态）。状态推进唯一经
    // dye_batch_state_machine_validation 权威流转表判定，禁止旁路直改状态列；
    // 本调用点保持单行形态（源码扫描锁 wave5 按 `is_valid_status_transition(&current_status, "stored")`
    // 逐字锚定"确实走流转函数"，链路断行重排会造成漏检假死码判红）。
    // 第二参目标态字面量与写入方常量的同源性由下行 debug_assert 钉死，漂移即炸。
    debug_assert_eq!("stored", batch_status::STORED);
    if !dye_batch_state_machine_validation::is_valid_status_transition(&current_status, "stored") {
        return Err(AppError::business(format!(
            "状态流转不合法：{} -> {}",
            current_status,
            batch_status::STORED
        )));
    }

    // 染色完成时发布 DyeBatchCompleted 业务事件，供质检单生成、染缸产能统计、
    // 成本结转、BI 生产报表等下游被动感知
    batch.actual_output_kg = Set(Some(actual_output_kg));
    batch.actual_output_m = Set(Some(req.actual_output_m));
    batch.greige_input_kg = Set(Some(greige_input_kg));
    batch.status = Set(Some(batch_status::STORED.to_string()));
    batch.completed_at = Set(Some(crate::utils::date_utils::utc_now_fixed()));
    batch.updated_at = Set(crate::utils::date_utils::utc_now_fixed());

    let updated = batch.update(&*state.db).await?;

    // 失重率参考日志（非拒绝门）：仓内权威染色损耗标准 5%（§5.7 行业中值，
    // outsourcing_service.rs::compute_standard_loss_rate，与委外域同源）。超过标准仅
    // fail-visible 记 warn 供追查，不阻断完工——完工拒绝阈值待产品给口径（见交付报告）。
    let standard_loss = crate::services::outsourcing_service::compute_standard_loss_rate(
        crate::models::status::wage_energy_chemical_business::outsourcing_order_type::DYEING,
    );
    if actual_output_kg <= greige_input_kg {
        let loss_rate = (greige_input_kg - actual_output_kg) / greige_input_kg;
        if loss_rate > standard_loss {
            tracing::warn!(
                batch_id = updated.id,
                batch_no = %updated.batch_no,
                actual_output_kg = %actual_output_kg,
                greige_input_kg = %greige_input_kg,
                loss_rate = %loss_rate.round_dp(4),
                standard_loss_rate = %standard_loss,
                "完工失重率超过染色标准损耗率（仅记录不拒绝；完工阈值待产品口径）"
            );
        }
    }

    // 落库成功后发布 DyeBatchCompleted 事件
    crate::services::event_bus::EVENT_BUS.publish(
        crate::services::event_bus::BusinessEvent::DyeBatchCompleted {
            batch_id: updated.id,
            batch_no: updated.batch_no.clone(),
            color_no: updated.color_no.clone(),
            greige_fabric_id: updated.greige_fabric_id,
            planned_quantity: updated.planned_quantity,
            completed_by: None,
        },
    );
    tracing::info!(
        batch_id = updated.id,
        batch_no = %updated.batch_no,
        actual_output_kg = %updated.actual_output_kg.unwrap_or(rust_decimal::Decimal::ZERO),
        actual_output_m = %updated.actual_output_m.unwrap_or(rust_decimal::Decimal::ZERO),
        greige_input_kg = %updated.greige_input_kg.unwrap_or(rust_decimal::Decimal::ZERO),
        "染色完成，实际产出已登记，已发布 DyeBatchCompleted 事件"
    );

    Ok(Json(ApiResponse::success_with_message(
        updated,
        "缸号完成成功",
    )))
}

pub async fn get_dye_batches_by_color(
    State(state): State<AppState>,
    Path(color_no): Path<String>,
) -> Result<Json<ApiResponse<Vec<dye_batch::Model>>>, AppError> {
    let batches = dye_batch::Entity::find()
        .filter(dye_batch::Column::ColorNo.eq(color_no))
        .filter(dye_batch::Column::IsDeleted.eq(false))
        .order_by_desc(dye_batch::Column::CreatedAt)
        .all(&*state.db)
        .await?;
    Ok(Json(ApiResponse::success(batches)))
}

/// GET /api/v1/erp/dye-batches/export - 导出缸号列表（xlsx）
pub async fn export_dye_batches(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<DyeBatchListQuery>,
) -> Result<axum::response::Response, AppError> {
    let mut q = dye_batch::Entity::find().filter(dye_batch::Column::IsDeleted.eq(false));
    q = apply_dye_batch_filters(q, &query);
    q = q.order_by_desc(dye_batch::Column::CreatedAt);

    // V15 缺陷 9-2 修复：导出全量查询加 limit 防止滥用（最大 10000 条）
    let batches = q.limit(10000).all(&*state.db).await?;

    let table = build_dye_batch_xlsx_table(&batches);
    let row_count = batches.len();

    // V15 P0-S11：导出审计日志写入（best-effort，异步不阻塞响应）
    let event = build_dye_batch_audit_event(&auth, &query, row_count);
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, None);

    // 规则 3：导出统一使用 xlsx 格式，错误用 AppError 表达，成功返回 200 + xlsx 响应体
    build_xlsx_response(&table, "dye_batches_export")
}

/// 应用缸号列表查询过滤条件
fn apply_dye_batch_filters(
    mut q: sea_orm::Select<dye_batch::Entity>,
    query: &DyeBatchListQuery,
) -> sea_orm::Select<dye_batch::Entity> {
    if let Some(batch_no) = &query.batch_no {
        q = q.filter(dye_batch::Column::BatchNo.contains(batch_no));
    }
    if let Some(color_no) = &query.color_no {
        q = q.filter(dye_batch::Column::ColorNo.contains(color_no));
    }
    if let Some(dye_lot_no) = &query.dye_lot_no {
        q = q.filter(dye_batch::Column::DyeLotNo.contains(dye_lot_no));
    }
    if let Some(status) = &query.status {
        q = q.filter(dye_batch::Column::Status.eq(status));
    }
    q
}

/// 构造缸号列表导出表格
fn build_dye_batch_xlsx_table(batches: &[dye_batch::Model]) -> XlsxTable {
    XlsxTable {
        sheet_name: "缸号列表".to_string(),
        headers: vec![
            "ID".to_string(),
            "缸号".to_string(),
            "染色批号".to_string(),
            "色号".to_string(),
            "坯布ID".to_string(),
            "计划数量".to_string(),
            "状态".to_string(),
            "创建时间".to_string(),
        ],
        rows: batches
            .iter()
            .map(|b| {
                vec![
                    b.id.to_string(),
                    b.batch_no.clone(),
                    b.dye_lot_no.clone(),
                    b.color_no.clone().unwrap_or_default(),
                    b.greige_fabric_id
                        .map(|i| i.to_string())
                        .unwrap_or_default(),
                    b.planned_quantity
                        .map(|d| d.to_string())
                        .unwrap_or_default(),
                    b.status.clone().unwrap_or_default(),
                    b.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                ]
            })
            .collect(),
    }
}

/// 构造缸号导出审计事件
fn build_dye_batch_audit_event(
    auth: &AuthContext,
    query: &DyeBatchListQuery,
    row_count: usize,
) -> AuditEvent {
    AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Export,
        severity: Severity::Info,
        resource_type: Some("dye_batch".to_string()),
        resource_id: None,
        resource_name: Some("dye_batches_export.xlsx".to_string()),
        description: Some(format!(
            "用户 {} 导出染色缸号列表（共 {} 条）",
            auth.username, row_count
        )),
        request_method: Some("GET".to_string()),
        request_path: Some("/api/v1/erp/dye-batches/export".to_string()),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "format": "xlsx",
            "total": row_count,
            "batch_no_filter": query.batch_no,
            "color_no_filter": query.color_no,
            "status_filter": query.status,
        })),
    }
}
