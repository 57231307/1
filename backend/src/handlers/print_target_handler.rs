//! 打印目标管理 Handler
//!
//! 对外端点（资源键 `print-targets`，URL 段 `/api/v1/erp/print-targets` 派生）：
//! - `GET    /print-targets`     [`list_print_targets`]   打印目标全量列表（按 id 升序）
//! - `POST   /print-targets`     [`create_print_target`]  新建打印机/打印机池
//! - `GET    /print-targets/{id}` [`get_print_target`]    单个目标详情
//! - `PUT    /print-targets/{id}` [`update_print_target`] 更新目标
//! - `DELETE /print-targets/{id}` [`delete_print_target`] 删除目标
//!
//! 落库真做：全部读写走 `print_targets` 表（SeaORM），不经任何内存态存储；新建/编辑后刷新
//! 必然读回同值。校验严格对齐建表迁移 `chk_print_targets_kind` / `chk_print_targets_default_copies`
//! / `chk_print_targets_kind_required` / `chk_print_targets_pool_members` 与 `uq_print_targets_code`，
//! 非法入参一律 fail-visible 400 点名，绝不让其撞 DB CHECK 变成 500，也绝不回落掩盖。
//!
//! 目标数据无归属列、无创建人列，故本 handler 不做行级 scope 过滤，仅记录审计日志。

use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, Order, PaginatorTrait,
    QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use tracing::info;
use validator::Validate;

use axum::{
    Json,
    extract::{Path, State},
};

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::print_target::{
    ActiveModel as PrintTargetActiveModel, Column as PrintTargetColumn,
    Entity as PrintTargetEntity, Model as PrintTargetModel,
};
use crate::models::print_template;
use crate::models::status::print::print_template_status;
use crate::models::status::print_target_kind;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 打印目标出参：逐列镜像 `print_targets` 行（序列化键即模型 snake_case 字段名），
/// 时间列以 RFC3339 字符串输出。`pool_members` 原样透出 JSON 数组。
#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, Clone, Serialize)]
pub struct PrintTargetRecord {
    pub id: i32,
    pub target_code: String,
    pub display_name: String,
    pub kind: String,
    pub queue_name: Option<String>,
    pub host: Option<String>,
    pub port: Option<i32>,
    pub endpoint_url: Option<String>,
    pub command_template: Option<String>,
    pub is_pool: bool,
    pub pool_members: serde_json::Value,
    pub default_copies: i16,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// 将 `print_targets` 行映射为出参结构，时间列转 RFC3339 字符串。
fn to_record(m: PrintTargetModel) -> PrintTargetRecord {
    PrintTargetRecord {
        id: m.id,
        target_code: m.target_code,
        display_name: m.display_name,
        kind: m.kind,
        queue_name: m.queue_name,
        host: m.host,
        port: m.port,
        endpoint_url: m.endpoint_url,
        command_template: m.command_template,
        is_pool: m.is_pool,
        pool_members: m.pool_members,
        default_copies: m.default_copies,
        enabled: m.enabled,
        created_at: m.created_at.to_rfc3339(),
        updated_at: m.updated_at.to_rfc3339(),
    }
}

/// `POST /print-targets` 请求体。
///
/// 必填 `target_code`（业务唯一符号编码）、`display_name`、`kind`。连接字段按 `kind` 选用，
/// 池行以 `pool_members`（成员 target_code 数组）路由。缺省 `is_pool=false`、
/// `default_copies=1`、`enabled=true`、`pool_members=[]`。
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct CreatePrintTargetRequest {
    #[validate(length(min = 1, max = 100, message = "目标编码长度须为 1-100 字符"))]
    pub target_code: String,
    #[validate(length(min = 1, max = 200, message = "目标显示名长度须为 1-200 字符"))]
    pub display_name: String,
    pub kind: String,
    pub queue_name: Option<String>,
    pub host: Option<String>,
    pub port: Option<i32>,
    pub endpoint_url: Option<String>,
    pub command_template: Option<String>,
    #[serde(default)]
    pub is_pool: Option<bool>,
    #[serde(default)]
    pub pool_members: Option<Vec<String>>,
    #[serde(default)]
    pub default_copies: Option<i16>,
    #[serde(default)]
    pub enabled: Option<bool>,
}

/// `PUT /print-targets/{id}` 请求体：所有字段可选，仅提交需要修改的列（差异更新）。
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct UpdatePrintTargetRequest {
    #[validate(length(min = 1, max = 100, message = "目标编码长度须为 1-100 字符"))]
    pub target_code: Option<String>,
    #[validate(length(min = 1, max = 200, message = "目标显示名长度须为 1-200 字符"))]
    pub display_name: Option<String>,
    pub kind: Option<String>,
    pub queue_name: Option<String>,
    pub host: Option<String>,
    pub port: Option<i32>,
    pub endpoint_url: Option<String>,
    pub command_template: Option<String>,
    pub is_pool: Option<bool>,
    pub pool_members: Option<Vec<String>>,
    pub default_copies: Option<i16>,
    pub enabled: Option<bool>,
}

/// 校验后的归一值：把 create/update 请求收敛到同一组有效值，供共享校验与入库复用。
struct TargetDraft {
    target_code: String,
    display_name: String,
    kind: String,
    queue_name: Option<String>,
    host: Option<String>,
    port: Option<i32>,
    endpoint_url: Option<String>,
    command_template: Option<String>,
    is_pool: bool,
    pool_members: Vec<String>,
    default_copies: i16,
    enabled: bool,
}

/// 取某连接字段 trim 后的非空 Some；空白串视为缺失（None），供必需字段判定。
fn non_blank(v: Option<String>) -> Option<String> {
    v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

/// 校验并归一目标参数，使取值域与 `print_targets` 建表 CHECK 逐条一致。
///
/// 覆盖 `chk_print_targets_kind`（kind 白名单）、`chk_print_targets_default_copies`（1-99）、
/// `chk_print_targets_kind_required`（非池行按 kind 携带必需连接字段）、
/// `chk_print_targets_pool_members`（池行成员非空数组）。非法即 fail-visible 400 点名，
/// 不回落、不截断、不掩盖。
fn validate_and_normalize(draft: &mut TargetDraft) -> Result<(), AppError> {
    draft.target_code = draft.target_code.trim().to_string();
    draft.display_name = draft.display_name.trim().to_string();
    let kind = draft.kind.as_str();

    if !print_target_kind::is_valid(kind) {
        return Err(AppError::bad_request(format!(
            "打印目标种类非法（仅支持 {}）：{kind}",
            print_target_kind::ALL.join("/")
        )));
    }
    if !(1..=99).contains(&draft.default_copies) {
        return Err(AppError::bad_request(format!(
            "缺省份数越界（须为 1-99）：{}",
            draft.default_copies
        )));
    }
    if let Some(p) = draft.port {
        if !(1..=65535).contains(&p) {
            return Err(AppError::bad_request(format!(
                "端口越界（须为 1-65535）：{p}"
            )));
        }
    }

    if draft.is_pool {
        // 池行：成员必为非空 target_code 数组（对齐 chk_print_targets_pool_members）；
        // 连接字段由成员行承载，池行本身不要求携带，清空避免误配。
        let members: Vec<String> = draft
            .pool_members
            .iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if members.is_empty() {
            return Err(AppError::bad_request(
                "打印机池须至少配置一个成员目标编码（pool_members 为空）",
            ));
        }
        draft.pool_members = members;
        draft.queue_name = None;
        draft.host = None;
        draft.port = None;
        draft.endpoint_url = None;
        draft.command_template = None;
        return Ok(());
    }

    // 非池行：按 kind 校验必需连接字段（对齐 chk_print_targets_kind_required）。
    let required_present = match kind {
        print_target_kind::CUPS => {
            draft.queue_name = non_blank(draft.queue_name.take());
            draft.queue_name.is_some()
        }
        print_target_kind::NETWORK => {
            draft.host = non_blank(draft.host.take());
            draft.host.is_some()
        }
        print_target_kind::IPP => {
            draft.endpoint_url = non_blank(draft.endpoint_url.take());
            draft.endpoint_url.is_some()
        }
        print_target_kind::SYSTEM_COMMAND => {
            draft.command_template = non_blank(draft.command_template.take());
            draft.command_template.is_some()
        }
        _ => false,
    };
    if !required_present {
        return Err(AppError::bad_request(format!(
            "种类 {kind} 缺少必需的连接字段，请补全后重试"
        )));
    }
    // 仅保留当前 kind 适用的连接列，其余显式置 None：kind 切换（编辑时前端只提交新 kind 字段）
    // 后不残留旧列，行内容与 chk_print_targets_kind_required 的语义严格一致。port 仅 network 适用。
    match kind {
        print_target_kind::CUPS => {
            draft.host = None;
            draft.port = None;
            draft.endpoint_url = None;
            draft.command_template = None;
        }
        print_target_kind::NETWORK => {
            draft.queue_name = None;
            draft.endpoint_url = None;
            draft.command_template = None;
        }
        print_target_kind::IPP => {
            draft.queue_name = None;
            draft.host = None;
            draft.port = None;
            draft.command_template = None;
        }
        print_target_kind::SYSTEM_COMMAND => {
            draft.queue_name = None;
            draft.host = None;
            draft.port = None;
            draft.endpoint_url = None;
        }
        _ => {}
    }
    // 非池行不消费 pool_members，恒置空数组（DDL 非池行为 '[]'）。
    draft.pool_members = Vec::new();
    Ok(())
}

/// `GET /print-targets`：读取全部打印目标行（按 id 升序），供管理页列表。
pub async fn list_print_targets(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<PrintTargetRecord>>>, AppError> {
    let models = PrintTargetEntity::find()
        .order_by(PrintTargetColumn::Id, Order::Asc)
        .all(&*state.db)
        .await?;
    let records: Vec<PrintTargetRecord> = models.into_iter().map(to_record).collect();
    Ok(Json(ApiResponse::success(records)))
}

/// `GET /print-targets/{id}`：单个目标详情，找不到返回 404。
pub async fn get_print_target(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<PrintTargetRecord>>, AppError> {
    let model = load_target(&*state.db, id).await?;
    Ok(Json(ApiResponse::success(to_record(model))))
}

/// 按 id 取目标行，缺失时返回 404（供详情/更新/删除复用）。
async fn load_target(
    db: &sea_orm::DatabaseConnection,
    id: i32,
) -> Result<PrintTargetModel, AppError> {
    PrintTargetEntity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::not_found(format!("打印目标 {id} 不存在")))
}

/// 校验 target_code 未被其它行占用（排除 `exclude_id`），命中即 400 点名重复。
async fn assert_code_free(
    db: &sea_orm::DatabaseConnection,
    code: &str,
    exclude_id: Option<i32>,
) -> Result<(), AppError> {
    let existing = PrintTargetEntity::find()
        .filter(PrintTargetColumn::TargetCode.eq(code))
        .one(db)
        .await?;
    if let Some(row) = existing {
        if Some(row.id) != exclude_id {
            return Err(AppError::bad_request("目标编码已存在，请更换一个唯一编码"));
        }
    }
    Ok(())
}

/// `POST /print-targets`：新建打印机/打印机池，落库后返回新行。
pub async fn create_print_target(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreatePrintTargetRequest>,
) -> Result<Json<ApiResponse<PrintTargetRecord>>, AppError> {
    req.validate()?;
    let mut draft = TargetDraft {
        target_code: req.target_code,
        display_name: req.display_name,
        kind: req.kind,
        queue_name: req.queue_name,
        host: req.host,
        port: req.port,
        endpoint_url: req.endpoint_url,
        command_template: req.command_template,
        is_pool: req.is_pool.unwrap_or(false),
        pool_members: req.pool_members.unwrap_or_default(),
        default_copies: req.default_copies.unwrap_or(1),
        enabled: req.enabled.unwrap_or(true),
    };
    validate_and_normalize(&mut draft)?;
    assert_code_free(&*state.db, &draft.target_code, None).await?;

    info!(
        user = %auth.username,
        target_code = %draft.target_code,
        kind = %draft.kind,
        is_pool = draft.is_pool,
        "创建打印目标"
    );

    let active = PrintTargetActiveModel {
        target_code: Set(draft.target_code),
        display_name: Set(draft.display_name),
        kind: Set(draft.kind),
        queue_name: Set(draft.queue_name),
        host: Set(draft.host),
        port: Set(draft.port),
        endpoint_url: Set(draft.endpoint_url),
        command_template: Set(draft.command_template),
        is_pool: Set(draft.is_pool),
        pool_members: Set(serde_json::Value::Array(
            draft
                .pool_members
                .into_iter()
                .map(serde_json::Value::String)
                .collect(),
        )),
        default_copies: Set(draft.default_copies),
        enabled: Set(draft.enabled),
        ..Default::default()
    };
    let model = active.insert(&*state.db).await?;
    Ok(Json(ApiResponse::success_with_message(
        to_record(model),
        "打印目标创建成功",
    )))
}

/// `PUT /print-targets/{id}`：差异更新目标列；kind/is_pool 变更后重新校验必需连接字段。
pub async fn update_print_target(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdatePrintTargetRequest>,
) -> Result<Json<ApiResponse<PrintTargetRecord>>, AppError> {
    req.validate()?;
    let existing = load_target(&*state.db, id).await?;

    // 以现有行为基，逐列覆盖被提交的字段，再统一校验（避免部分更新绕过 CHECK）。
    let mut draft = TargetDraft {
        target_code: req.target_code.unwrap_or(existing.target_code.clone()),
        display_name: req.display_name.unwrap_or(existing.display_name.clone()),
        kind: req.kind.unwrap_or(existing.kind.clone()),
        queue_name: req.queue_name.or(existing.queue_name.clone()),
        host: req.host.or(existing.host.clone()),
        port: if req.port.is_some() {
            req.port
        } else {
            existing.port
        },
        endpoint_url: req.endpoint_url.or(existing.endpoint_url.clone()),
        command_template: req.command_template.or(existing.command_template.clone()),
        is_pool: req.is_pool.unwrap_or(existing.is_pool),
        pool_members: req.pool_members.unwrap_or_else(|| {
            existing
                .pool_members
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default()
        }),
        default_copies: req.default_copies.unwrap_or(existing.default_copies),
        enabled: req.enabled.unwrap_or(existing.enabled),
    };
    validate_and_normalize(&mut draft)?;
    if draft.target_code != existing.target_code {
        assert_code_free(&*state.db, &draft.target_code, Some(id)).await?;
    }

    info!(
        user = %auth.username,
        target_id = id,
        target_code = %draft.target_code,
        "更新打印目标"
    );

    let mut active: PrintTargetActiveModel = existing.into_active_model();
    active.target_code = Set(draft.target_code);
    active.display_name = Set(draft.display_name);
    active.kind = Set(draft.kind);
    active.queue_name = Set(draft.queue_name);
    active.host = Set(draft.host);
    active.port = Set(draft.port);
    active.endpoint_url = Set(draft.endpoint_url);
    active.command_template = Set(draft.command_template);
    active.is_pool = Set(draft.is_pool);
    active.pool_members = Set(serde_json::Value::Array(
        draft
            .pool_members
            .into_iter()
            .map(serde_json::Value::String)
            .collect(),
    ));
    active.default_copies = Set(draft.default_copies);
    active.enabled = Set(draft.enabled);
    active.updated_at = Set(chrono::Utc::now().into());
    let model = active.update(&*state.db).await?;
    Ok(Json(ApiResponse::success_with_message(
        to_record(model),
        "打印目标更新成功",
    )))
}

/// `DELETE /print-targets/{id}`：删除目标行；不存在返回 404。
///
/// 仍被启用模板列为缺省目标的目标不许删：删掉后该符号在模板里悬挂，下一次打印只会得到
/// "打印目标未配置或未启用"，用户在模板页看不出成因。已归档任务里的目标符号是历史快照，
/// 不参与本判断。
pub async fn delete_print_target(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    info!(user = %auth.username, target_id = id, "删除打印目标");
    let target = load_target(&*state.db, id).await?;

    let referenced_by_templates = print_template::Entity::find()
        .filter(print_template::Column::DefaultTargetName.eq(target.target_code.clone()))
        .filter(print_template::Column::Status.eq(print_template_status::ACTIVE))
        .count(&*state.db)
        .await?;
    if referenced_by_templates > 0 {
        return Err(AppError::business_displayable(format!(
            "打印目标 {} 仍是 {} 个启用中打印模板的缺省目标，请先改模板缺省目标或停用模板",
            target.target_code, referenced_by_templates
        )));
    }

    PrintTargetEntity::delete_by_id(id).exec(&*state.db).await?;
    Ok(Json(ApiResponse::success_with_message(
        (),
        "打印目标删除成功",
    )))
}
