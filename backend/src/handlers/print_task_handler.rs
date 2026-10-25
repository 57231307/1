//! 打印任务受理与查询 Handler
//!
//! 对外端点：
//! - `POST /print-tasks`   [`create_print_task`]   受理一次打印请求（落 `print_tasks` pending 行并后台派发）
//! - `GET  /print-tasks`   [`list_print_tasks`]    分页任务列表
//! - `GET  /print-tasks/{id}` [`get_print_task`]   单任务详情（前端轮询 status/external_job_id/error_message）
//!
//! 受理链路严格复用既有渲染与派发能力，绝不另起第二套：文档字节由
//! [`PrintService::get_print_data`] + [`PrintService::generate_docx`] 生成，页面布局统一由本模块
//! [`docx_layout_from_template`] 从 `print_templates` 行构造（`print_handler` 的 docx 渲染路径也经
//! [`resolve_print_docx_layout`] 走同一函数，二者共享单一来源）；派发一律交
//! [`crate::services::print_dispatch::dispatch_task`]，其在任意终态落库后返回 `Ok(())`，故受理侧
//! 以回读 `print_tasks` 行为事实来源判定结果，不以返回值区分成败。
//!
//! 失败可见性硬约束：受理时同步校验单据存在性（经 `get_print_data`），不静默接受无效 doc_id；
//! 后台任何前派发错误（取数/生成 docx）或未捕获 panic 都会把任务转 `failed` 并写非空
//! `error_message`，绝不让任务停留在 in-flight（pending/queued/dispatched）。打印机地址一律不
//! 出现在代码里：目标以 `print_targets.target_code` 符号名解析，`backend_kind` 取解析出目标行的 `kind`。

use std::panic::AssertUnwindSafe;
use std::sync::Arc;

use chrono::{DateTime, FixedOffset, Utc};
use futures::FutureExt;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait, QueryFilter, QueryOrder,
    Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use crate::config::settings::global_print_config;
use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::print_target::{
    Column as PrintTargetColumn, Entity as PrintTargetEntity, Model as PrintTargetModel,
};
use crate::models::print_task::{
    ActiveModel as PrintTaskActiveModel, Column as PrintTaskColumn, Entity as PrintTaskEntity,
    Model as PrintTaskModel,
};
use crate::models::print_template::{self, Model as PrintTemplateModel};
use crate::models::status::print::{print_task_status, print_template_status};
use crate::services::print_dispatch::dispatch_task;
use crate::services::print_service::PrintService;
use crate::utils::docx_export::DocxPageLayout;
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;
use crate::utils::pagination::paginate_with_total;
use crate::utils::response::{ApiResponse, PaginatedResponse};
use axum::{
    Json,
    extract::{Path, Query, State},
};

/// 打印任务号前缀：`print_tasks.job_code` 列 `VARCHAR(50) NOT NULL UNIQUE`（DDL
/// `uq_print_tasks_job_code`），保留 `PT-` 前缀与生成器默认流水宽度，收口到
/// [`DocumentNumberGenerator`] 统一取号（并发受理不产生重复号撞 UNIQUE）。
const PRINT_TASK_JOB_CODE_PREFIX: &str = "PT-";

/// 受理成功后返回给前端的最小引用：轮询详情所需的主键与任务号。
#[derive(Debug, Clone, Serialize)]
pub struct PrintTaskJobRef {
    pub id: i32,
    pub job_code: String,
}

/// `POST /print-tasks` 请求体。
///
/// 必填 `doc_type` + `doc_id` 定位单据实例；其余为可选的逐任务覆盖维度（份数/纸张/方向/四边边距/
/// 条码开关/目标符号名/模板引用）。缺省维度取所选（或 doc_type 默认）模板的持久化缺省值，
/// 再缺省回落 `print_tasks` 列默认。`doc_id` 为多态单据引用，列为 BIGINT，取数按渲染路径口径转 i32。
#[derive(Debug, Deserialize)]
pub struct CreatePrintTaskRequest {
    pub doc_type: String,
    pub doc_id: i64,
    #[serde(default)]
    pub target_name: Option<String>,
    #[serde(default)]
    pub template_id: Option<i32>,
    #[serde(default)]
    pub copies: Option<i32>,
    #[serde(default)]
    pub paper_size: Option<String>,
    #[serde(default)]
    pub orientation: Option<String>,
    #[serde(default)]
    pub margin_top_mm: Option<i16>,
    #[serde(default)]
    pub margin_bottom_mm: Option<i16>,
    #[serde(default)]
    pub margin_left_mm: Option<i16>,
    #[serde(default)]
    pub margin_right_mm: Option<i16>,
    #[serde(default)]
    pub barcode_enabled: Option<bool>,
}

/// `GET /print-tasks` 查询参数（分页 + 可选 doc_type / status 过滤）。
#[derive(Debug, Deserialize)]
pub struct ListPrintTasksQuery {
    #[serde(default)]
    pub page: Option<u64>,
    #[serde(default)]
    pub page_size: Option<u64>,
    #[serde(default)]
    pub doc_type: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
}

/// 打印任务详情出参：逐列镜像 `print_tasks` 行（序列化键名即模型 snake_case 字段名），
/// 时间列以 RFC3339 字符串输出，供前端轮询读取 `status`/`external_job_id`/`error_message`。
#[derive(Debug, Clone, Serialize)]
pub struct PrintTaskRecord {
    pub id: i32,
    pub job_code: String,
    pub requested_by: i32,
    pub requested_by_name: String,
    pub doc_type: String,
    pub doc_id: i64,
    pub template_id: Option<i32>,
    pub target_kind: String,
    pub target_name: String,
    pub copies: i32,
    pub paper_size: String,
    pub orientation: String,
    pub margin_top_mm: i16,
    pub margin_bottom_mm: i16,
    pub margin_left_mm: i16,
    pub margin_right_mm: i16,
    pub barcode_enabled: bool,
    pub source_module: String,
    pub status: String,
    pub backend_kind: String,
    pub external_job_id: Option<String>,
    pub error_message: Option<String>,
    pub dispatched_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<PrintTaskModel> for PrintTaskRecord {
    fn from(m: PrintTaskModel) -> Self {
        Self {
            id: m.id,
            job_code: m.job_code,
            requested_by: m.requested_by,
            requested_by_name: m.requested_by_name,
            doc_type: m.doc_type,
            doc_id: m.doc_id,
            template_id: m.template_id,
            target_kind: m.target_kind,
            target_name: m.target_name,
            copies: m.copies,
            paper_size: m.paper_size,
            orientation: m.orientation,
            margin_top_mm: m.margin_top_mm,
            margin_bottom_mm: m.margin_bottom_mm,
            margin_left_mm: m.margin_left_mm,
            margin_right_mm: m.margin_right_mm,
            barcode_enabled: m.barcode_enabled,
            source_module: m.source_module,
            status: m.status,
            backend_kind: m.backend_kind,
            external_job_id: m.external_job_id,
            error_message: m.error_message,
            dispatched_at: m.dispatched_at.map(|t| t.to_rfc3339()),
            completed_at: m.completed_at.map(|t| t.to_rfc3339()),
            created_at: m.created_at.to_rfc3339(),
            updated_at: m.updated_at.to_rfc3339(),
        }
    }
}

/// 当前时刻（列类型 `DateTimeWithTimeZone` = `DateTime<FixedOffset>`）。
fn now() -> DateTime<FixedOffset> {
    Utc::now().fixed_offset()
}

/// 由 `print_templates` 行构造渲染用页面布局（纸张/方向/四边边距/条码）。
///
/// 单一来源：`print_handler` 的 docx 渲染与本受理链路都经此函数从模板行取布局，不再各写一套
/// 字段映射。边距/条码列即 DDL 缺省所约束的取值域。
pub fn docx_layout_from_template(m: &PrintTemplateModel) -> DocxPageLayout {
    DocxPageLayout {
        paper_size: m.paper_size.clone(),
        orientation: m.orientation.clone(),
        margin_top_mm: m.margin_top_mm,
        margin_bottom_mm: m.margin_bottom_mm,
        margin_left_mm: m.margin_left_mm,
        margin_right_mm: m.margin_right_mm,
        barcode_enabled: m.barcode_default,
    }
}

/// 取 `doc_type` 下「启用 + 默认」的模板行（按 id 升序取首行）；查不到即 fail-visible 404
/// （不以内置默认值代渲染，与既有渲染路径口径一致）。
///
/// 启用位是硬解析条件：status 不等于启用 token 的行（含默认位为真的行）永不被
/// 打印出口选中，停用即断供、重新启用即恢复。默认位由写入侧按 doc_type 维持至多
/// 一行（与解析查找键同口径，见 print_handler 的建/改/设默认清理逻辑）；存量库
/// 遗留的同 doc_type 多默认行按 id 升序取首行，判定结果保持确定。
pub async fn load_active_default_template(
    db: &DatabaseConnection,
    doc_type: &str,
) -> Result<PrintTemplateModel, AppError> {
    print_template::Entity::find()
        .filter(print_template::Column::DocType.eq(doc_type))
        .filter(print_template::Column::Status.eq(print_template_status::ACTIVE))
        .filter(print_template::Column::IsDefault.eq(true))
        .order_by(print_template::Column::Id, Order::Asc)
        .one(db)
        .await?
        .ok_or_else(|| {
            AppError::not_found(format!(
                "单据类型 {doc_type} 没有可用的（启用且默认）打印模板，模板停用后打印口即断供"
            ))
        })
}

/// 按显式模板 id 取「启用」模板行：出件侧只接受启用位为启用的行。
///
/// 行不存在与行已停用都返回 404 not_found，消息点名模板 id 与成因（不存在还是
/// 已停用），使用者能直接看懂为什么打不出。与管理面读取函数
/// print_handler::load_template 口径有意不同：管理面（列表/详情/预览/编辑/复制）
/// 必须能读到停用行供运营核对待启用；本函数只用于真正出件与派发的链路。
/// 内置模板被用户的自建默认行降位后，仍可在模板管理页经显式 id 出件，前提是其启用位
/// 为 active（默认位只约束「按单据类型自动解析」的缺省出件，不约束显式指定）。
pub async fn load_printable_template(
    db: &DatabaseConnection,
    id: i32,
) -> Result<PrintTemplateModel, AppError> {
    let template = print_template::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::not_found(format!("打印模板 {id} 不存在")))?;
    if template.status != print_template_status::ACTIVE {
        return Err(AppError::not_found(format!(
            "打印模板 {id}（{}）已停用，不能作为打印出口，请先在模板管理中启用该模板",
            template.template_name
        )));
    }
    Ok(template)
}

/// 按 `doc_type` 解析渲染所用的默认模板布局（启用且默认行）。
///
/// 这是 `print_handler::render_print_docx` 与打印任务派发共用的同一入口：受理时若未显式指定
/// 模板引用，即回落到本函数的 doc_type 默认模板；显式指定模板 id 时经
/// [`load_printable_template`] 同样强制启用位条件。
pub async fn resolve_print_docx_layout(
    db: &DatabaseConnection,
    doc_type: &str,
) -> Result<DocxPageLayout, AppError> {
    let template = load_active_default_template(db, doc_type).await?;
    Ok(docx_layout_from_template(&template))
}

/// 按 `target_code` 取「存在且启用」的打印目标行；不存在或禁用返回 `None`
/// （由受理侧转成 fail-visible 400，与派发层「打印目标未配置或未启用」同口径，不在代码里出现
/// 任何设备地址或回退默认目标）。
async fn find_enabled_target(
    db: &DatabaseConnection,
    code: &str,
) -> Result<Option<PrintTargetModel>, AppError> {
    let row = PrintTargetEntity::find()
        .filter(PrintTargetColumn::TargetCode.eq(code))
        .one(db)
        .await?;
    Ok(row.filter(|r| r.enabled))
}

/// 校验并归一化逐任务覆盖维度到 `print_tasks` 列约束域。
///
/// 取值域与建表 CHECK 逐条一致（orientation ∈ portrait/landscape、copies ∈ [1,99]、
/// 四边边距 ∈ [0,50]）；非法入参以 fail-visible 400 拒绝，不截断、不回落到模板值掩盖。
fn validate_overrides(req: &CreatePrintTaskRequest) -> Result<(), AppError> {
    if let Some(o) = req.orientation.as_deref() {
        if o != "portrait" && o != "landscape" {
            return Err(AppError::bad_request(format!(
                "打印方向非法（仅支持 portrait/landscape）：{o}"
            )));
        }
    }
    if let Some(c) = req.copies {
        if !(1..=99).contains(&c) {
            return Err(AppError::bad_request(format!(
                "打印份数须在 1-99 之间：{c}"
            )));
        }
    }
    for (name, v) in [
        ("上边距", req.margin_top_mm),
        ("下边距", req.margin_bottom_mm),
        ("左边距", req.margin_left_mm),
        ("右边距", req.margin_right_mm),
    ] {
        if let Some(m) = v {
            if !(0i16..=50i16).contains(&m) {
                return Err(AppError::bad_request(format!(
                    "{name}须在 0-50 毫米之间：{m}"
                )));
            }
        }
    }
    Ok(())
}

/// 受理一次打印：同步校验（覆盖维度、目标可解析、单据存在）→ 落 pending 行 → 后台生成 docx 并派发。
///
/// 目标优先级：显式请求 target_name > 适用模板 default_target_name。本 schema 未提供“按模块路由
/// 目标”的映射列，`PrintConfig` 亦无全局默认目标字段，故不存在可回退的全局/模块默认——两级皆缺即
/// fail-visible 400 拒绝受理（不伪造目标、不让注定失败的行入库）。`backend_kind` 取解析出的目标行
/// `kind`，`target_kind` 由目标行 `is_pool` 判定（pool/printer）。
pub(crate) async fn submit_print_task(
    state: &AppState,
    auth: &AuthContext,
    req: CreatePrintTaskRequest,
) -> Result<PrintTaskJobRef, AppError> {
    validate_overrides(&req)?;

    let db: Arc<DatabaseConnection> = state.db.clone();

    // 适用模板：显式 template_id 优先（须存在且启用），否则取 doc_type 的启用默认
    // 模板行。两条分支都以启用位为硬条件：停用模板无论经由哪条路径都不能出件
    // （不产生 docx、不入库、不派发）。
    let template: PrintTemplateModel = match req.template_id {
        Some(tid) => load_printable_template(&*db, tid).await?,
        None => load_active_default_template(&*db, &req.doc_type).await?,
    };

    // 目标解析（符号名 → print_targets 行）：据此定 target_kind / backend_kind（均 NOT NULL 受约束）。
    let target_code = req
        .target_name
        .clone()
        .or_else(|| template.default_target_name.clone())
        .ok_or_else(|| {
            AppError::bad_request(
                "打印需指定打印目标：请求未提供 target_name 且适用模板未配置 default_target_name",
            )
        })?;
    let target = find_enabled_target(&*db, &target_code)
        .await?
        .ok_or_else(|| AppError::bad_request(format!("打印目标未配置或未启用: {target_code}")))?;
    let target_kind = if target.is_pool { "pool" } else { "printer" }.to_string();
    let backend_kind = target.kind.clone();

    // 单据存在性同步校验：经渲染路径同款 get_print_data，取数失败即拒绝受理（不静默接受无效 doc_id）。
    let service = PrintService::new(db.clone());
    service
        .get_print_data(&req.doc_type, req.doc_id as i32)
        .await?;

    // 生效打印参数：请求覆盖 > 模板缺省 > 列默认（DDL DEFAULT）。
    let copies = req
        .copies
        .unwrap_or(i32::from(template.copies_default))
        .clamp(1, 99);
    let paper_size = req
        .paper_size
        .clone()
        .unwrap_or_else(|| template.paper_size.clone());
    let orientation = req
        .orientation
        .clone()
        .unwrap_or_else(|| template.orientation.clone());
    let margin_top_mm = req
        .margin_top_mm
        .unwrap_or(template.margin_top_mm)
        .clamp(0, 50);
    let margin_bottom_mm = req
        .margin_bottom_mm
        .unwrap_or(template.margin_bottom_mm)
        .clamp(0, 50);
    let margin_left_mm = req
        .margin_left_mm
        .unwrap_or(template.margin_left_mm)
        .clamp(0, 50);
    let margin_right_mm = req
        .margin_right_mm
        .unwrap_or(template.margin_right_mm)
        .clamp(0, 50);
    let barcode_enabled = req.barcode_enabled.unwrap_or(template.barcode_default);
    let template_id = Some(template.id);

    // 受理入库：经取号器在事务内对 job_code UNIQUE 做保存点级重试，避免并发受理撞号。
    let txn = db.begin().await?;
    let built = PrintTaskActiveModel {
        job_code: Default::default(),
        requested_by: Set(auth.user_id),
        requested_by_name: Set(auth.username.clone()),
        doc_type: Set(req.doc_type.clone()),
        doc_id: Set(req.doc_id),
        template_id: Set(template_id),
        target_kind: Set(target_kind.clone()),
        target_name: Set(target_code.clone()),
        copies: Set(copies),
        paper_size: Set(paper_size.clone()),
        orientation: Set(orientation.clone()),
        margin_top_mm: Set(margin_top_mm),
        margin_bottom_mm: Set(margin_bottom_mm),
        margin_left_mm: Set(margin_left_mm),
        margin_right_mm: Set(margin_right_mm),
        barcode_enabled: Set(barcode_enabled),
        source_module: Set(template.module.clone()),
        payload_params: Set(serde_json::Value::Object(serde_json::Map::new())),
        status: Set(print_task_status::PENDING.to_string()),
        backend_kind: Set(backend_kind.clone()),
        ..Default::default()
    };
    let inserted = DocumentNumberGenerator::insert_with_no_retry(
        &txn,
        PRINT_TASK_JOB_CODE_PREFIX,
        PrintTaskEntity,
        PrintTaskColumn::JobCode,
        |job_code| {
            let mut am = built.clone();
            am.job_code = Set(job_code);
            am
        },
    )
    .await?;
    txn.commit().await?;

    let job_id = inserted.id;
    let job_code = inserted.job_code.clone();
    info!(
        job_id,
        job_code = %job_code,
        user = %auth.username,
        doc_type = %req.doc_type,
        doc_id = req.doc_id,
        target = %target_code,
        backend_kind = %backend_kind,
        "打印任务已受理入库（pending），转入后台生成文档并派发"
    );

    // 后台：生成 docx → 派发 → 回读终态。所有前派发错误与未捕获 panic 都经
    // [`mark_failed`] 落 failed（写非空 error_message），保证行不会停留 in-flight。
    let spawn_db = db.clone();
    let doc_type_for_spawn = req.doc_type.clone();
    let doc_id_for_spawn = req.doc_id;
    tokio::spawn(async move {
        let outcome = AssertUnwindSafe(run_print_job(
            spawn_db.clone(),
            job_id,
            doc_type_for_spawn,
            doc_id_for_spawn,
        ))
        .catch_unwind()
        .await;
        match outcome {
            // 正常返回：run_print_job 内部已把终态落库（dispatch_task 已保证），无需再处理。
            Ok(Ok(())) => {}
            // 前派发错误（取数/生成 docx/回读失败）：把仍处 in-flight 的行转 failed。
            Ok(Err(e)) => {
                mark_failed(&spawn_db, job_id, &format!("打印任务后台处理失败: {e}")).await;
            }
            // 未捕获 panic：dispatch_task 的乐观锁不识别 panic，行可能停留 pending/queued，兜底转 failed。
            Err(panic_payload) => {
                let detail = panic_detail(&panic_payload);
                error!(job_id, panic = %detail, "打印任务后台任务 panic，兜底转入 failed");
                mark_failed(
                    &spawn_db,
                    job_id,
                    &format!("打印任务后台处理 panic: {detail}"),
                )
                .await;
            }
        }
    });

    Ok(PrintTaskJobRef {
        id: job_id,
        job_code,
    })
}

/// 后台执行体：取打印数据 → 解析布局（与受理入库同源的生效维度）→ 生成 docx → 派发。
///
/// 前派发的取数/生成错误以 `Err` 上抛给调用（由调用转 failed）；进入 `dispatch_task` 后其自身
/// 已保证任意终态落库并返回 `Ok(())`，故此处仅在无法把结果落库时才继续报错。
async fn run_print_job(
    db: Arc<DatabaseConnection>,
    job_id: i32,
    doc_type: String,
    doc_id: i64,
) -> Result<(), AppError> {
    // 回读已入库的受理行，据其快照列（含生效维度）构造布局，确保 docx 版面与受理参数一致。
    let task = PrintTaskEntity::find_by_id(job_id)
        .one(&*db)
        .await?
        .ok_or_else(|| AppError::not_found(format!("打印任务不存在（id={job_id}）")))?;

    let layout = DocxPageLayout {
        paper_size: task.paper_size.clone(),
        orientation: task.orientation.clone(),
        margin_top_mm: task.margin_top_mm,
        margin_bottom_mm: task.margin_bottom_mm,
        margin_left_mm: task.margin_left_mm,
        margin_right_mm: task.margin_right_mm,
        barcode_enabled: task.barcode_enabled,
    };

    let service = PrintService::new(db.clone());
    let print_data = service.get_print_data(&doc_type, doc_id as i32).await?;
    let bytes = service.generate_docx(&print_data, &layout).await?;

    let config = global_print_config();
    // dispatch_task 无论成功/失败均驱动到终态并返回 Ok（仅落库失败才 Err）；此处透传其结果。
    dispatch_task(&db, job_id, &bytes, &config).await
}

/// 把 panic payload 归一化为可读文本（`&str` / `String` 两类常见载荷）。
fn panic_detail(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "未知 panic".to_string()
    }
}

/// 兜底把仍处 in-flight 的任务转 `failed` 并写非空 `error_message`。
///
/// 用乐观锁 `WHERE status IN (pending/queued/dispatched)` 命中前置态才改写，绝不回退
/// dispatch_task 已落的终态（succeeded/failed）；改写失败（并发已推进/已终态）记 WARN 不静默。
async fn mark_failed(db: &Arc<DatabaseConnection>, job_id: i32, message: &str) {
    let am = PrintTaskActiveModel {
        status: Set(print_task_status::FAILED.to_string()),
        error_message: Set(Some(message.to_string())),
        completed_at: Set(Some(now())),
        updated_at: Set(now()),
        ..Default::default()
    };
    match PrintTaskEntity::update_many()
        .set(am)
        .filter(PrintTaskColumn::Id.eq(job_id))
        .filter(PrintTaskColumn::Status.is_in([
            print_task_status::PENDING,
            print_task_status::QUEUED,
            print_task_status::DISPATCHED,
        ]))
        .exec(&**db)
        .await
    {
        Ok(r) if r.rows_affected > 0 => {
            warn!(job_id, error = %message, "打印任务转入 failed（error_message 已写）");
        }
        Ok(_) => {
            warn!(
                job_id,
                "failed 兜底未命中 in-flight 前置态：任务已被并发推进或已终态，不重复改写"
            );
        }
        Err(e) => {
            error!(job_id, error = %e, "打印任务转 failed 兜底写库失败（该行可能停留 in-flight，需人工介入）");
        }
    }
}

/// `POST /print-tasks`：受理打印任务，返回 `{ id, job_code }`（前端据 id 轮询详情）。
pub async fn create_print_task(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreatePrintTaskRequest>,
) -> Result<Json<ApiResponse<PrintTaskJobRef>>, AppError> {
    let job_ref = submit_print_task(&state, &auth, req).await?;
    Ok(Json(ApiResponse::success(job_ref)))
}

/// `GET /print-tasks/{id}`：返回单个任务全列（含 status/external_job_id/error_message）。
pub async fn get_print_task(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<PrintTaskRecord>>, AppError> {
    let model = PrintTaskEntity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found(format!("打印任务不存在（id={id}）")))?;
    Ok(Json(ApiResponse::success(PrintTaskRecord::from(model))))
}

/// `GET /print-tasks`：分页任务列表（按创建时间倒序），可选 doc_type/status 过滤。
///
/// 列表返回全量任务台账：本轮仅 system_admin（read）与超管旁路可达，无业务岗授权，故不施加
/// 按发起人行级收敛（print_tasks 无部门归属列，requested_by 仅是发起人快照）；未来若向业务岗
/// 开放再补行级 scope。分页元数据置于 `data`（PaginatedResponse 单一口径）。
pub async fn list_print_tasks(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(q): Query<ListPrintTasksQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<PrintTaskRecord>>>, AppError> {
    let page = q.page.unwrap_or(1).clamp(1, 1000);
    let page_size = q.page_size.unwrap_or(20).clamp(1, 200);

    let mut select = PrintTaskEntity::find();
    if let Some(dt) = q
        .doc_type
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        select = select.filter(PrintTaskColumn::DocType.eq(dt));
    }
    if let Some(st) = q.status.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        if !print_task_status::is_valid(st) {
            return Err(AppError::bad_request(format!("打印任务状态非法: {st}")));
        }
        select = select.filter(PrintTaskColumn::Status.eq(st));
    }

    let paginator = select
        .order_by_desc(PrintTaskColumn::CreatedAt)
        .paginate(&*state.db, page_size);
    let (items, total) = paginate_with_total(paginator, page).await?;
    let records: Vec<PrintTaskRecord> = items.into_iter().map(PrintTaskRecord::from).collect();

    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        records, total, page, page_size,
    ))))
}
