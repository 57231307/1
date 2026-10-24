//! 契约锁：BPM 发起与审批推进不得因"没有可执行的审批节点"而放行（双重 fail-closed）
//!
//! 本文件钉死的不变量：
//! 1. 发起路径（创建首个任务的函数体）绝不出现把实例置成已完成、也绝不发布
//!    流程结束事件，且必须同时具备两道拒绝：
//!    ① 解析结果为 None（找不到首个任务节点）→ 拒绝；
//!    ② 解析结果非 None 但节点类型不是审批节点（起始连线指向的目标不经类型校验，
//!    可能带回引擎不建单的词表项）或审批人非法 → 同样拒绝。
//!    任一命中都返回可外显校验错误、事务回滚、不留半写实例。
//! 2. 审批推进路径（尝试推进下一节点）里，匹配边的目标节点类型不在引擎词表
//!    （user_task / end_event）或连线指向不存在的节点时，拒绝本次审批并回滚，
//!    不得按"流程正常结束且已批准"收尾。
//! 3. 正确形状的流程定义（start_event / user_task / end_event + edges）发起
//!    不受影响：恰好生成一条 pending 工单，实例停在 PROCESSING。
//!
//! 检测力自证：反向锁 A/B 钉守卫②（首边目标类型不校验的错形状——只补了 None 分支
//! 的"半修"抓不住它，因为此时解析结果非 None、节点还带合法 assignee_value 时旧形态
//! 会直接建单返回 Ok），反向锁 C 钉守卫①（解析结果为 None 的分支——只补了错形状守卫
//! 的半修在这里红），反向锁 D 钉守卫③（审批人缺失不建孤儿单）。静态锁同时要求函数体
//! 存在 None 拒绝（ok_or_else）与类型拒绝（!= "user_task" 比较）两个要件，缺一即红；
//! 若整体退回旧形态（找不到任务节点就把实例置 completed 并发批准事件），反向锁会因
//! 收到 Ok 在 expect_err 处红，函数体也会因重新出现批准事件/完成状态字样被静态锁抓到。
//! 推进用例钉守卫②的推进侧：退回"识别不了就按完成批准"时因收到 Ok 而红。
//!
//! 静态锁不依赖活库，CI 常规测试阶段执行；活库行为锁按本仓 test_common 夹具
//! 口径（TEST_DATABASE_URL 指向已迁移 PostgreSQL），由 CI 侧执行且整档以
//! --test-threads=1 串行跑，逐用例 setup 互不串库。

mod test_common;

use std::fs;
use std::path::PathBuf;

use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde_json::json;
use std::sync::Arc;

use bingxi_backend::models::dto::bpm_dto::{ApproveTaskRequest, StartProcessRequest};
use bingxi_backend::models::status::bpm_instance as instance_status;
use bingxi_backend::models::status::bpm_task as task_status;
use bingxi_backend::models::{bpm_process_definition, bpm_process_instance, bpm_task, user};
use bingxi_backend::services::bpm_service::BpmService;
use bingxi_backend::utils::error::AppError;
use sea_orm::DatabaseConnection;

// ---------------------------------------------------------------------------
// 1) 静态形态锁（不依赖活库，CI 常规可跑）
// ---------------------------------------------------------------------------

fn read_rel(manifest_rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(manifest_rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {:?} 失败: {}", path, e))
}

/// 剔除注释行：守卫对象是代码里的形态，文档注释中说明禁语的上下文不算命中。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 取函数体：从签名处开始，到下一个同级条目（4 空格缩进的 fn）出现之前为止，
/// 按函数作用域收窄断言，避免全文件 contains 误伤其他合法路径。
fn fn_body(src: &str, signature: &str) -> String {
    let anchor = src
        .find(signature)
        .unwrap_or_else(|| panic!("待锁函数不存在: {signature}"));
    let tail = &src[anchor..];
    let next = [
        "\n    pub async fn ",
        "\n    pub fn ",
        "\n    async fn ",
        "\n    fn ",
    ]
    .iter()
    .filter_map(|marker| tail.find(marker))
    .min()
    .unwrap_or(tail.len());
    tail[..next].to_string()
}

/// 发起路径的建首任务函数体：禁止出现自动完成形态的任何要件
/// （置实例已完成、写完成时间、收集流程批准事件）；同时必须齐备两道拒绝要件——
/// None 分支的拒绝（ok_or_else）与错形状分支的类型拒绝（与 "user_task" 的显式比较），
/// 缺任意一道即"半修"，本静态锁直接红。
#[test]
fn start_path_never_autocompletes_without_approval_node() {
    let src = read_rel("src/services/bpm_ops/instance.rs").replace('\r', "");
    let body = code_only(&fn_body(&src, "async fn create_first_task("));
    for forbidden in [
        "BpmProcessFinished",
        "instance_status::COMPLETED",
        "completed_at",
    ] {
        assert!(
            !body.contains(forbidden),
            "发起路径的建首任务函数体禁止出现自动完成形态要件 {forbidden}；\
             没有可执行审批节点必须拒绝发起而不是批准收尾。函数体:\n{body}"
        );
    }
    assert!(
        body.contains("AppError::validation_displayable"),
        "拒绝发起必须以可外显的校验错误返回（文案要点名流程标识供管理员行动）。函数体:\n{body}"
    );
    assert!(
        body.contains("ok_or_else"),
        "守卫①：解析结果为 None 的分支必须以 ok_or_else 拒绝发起，缺失即半修。函数体:\n{body}"
    );
    assert!(
        body.contains(r#"!= "user_task""#),
        "守卫②：解析结果非 None 时必须校验节点类型与 \"user_task\" 逐字符比较，\
         缺失则起始连线指向错误词表项的形态会漏网（半修）。函数体:\n{body}"
    );
}

/// 发起入口函数体：首任务建不出来即整体失败回滚，发起路径不得发布任何流程事件
/// （撤回路径 cancel_instance 的事件发布不在本锁范围，故按函数作用域收窄）。
#[test]
fn start_process_publishes_no_business_event() {
    let src = read_rel("src/services/bpm_ops/instance.rs").replace('\r', "");
    let body = code_only(&fn_body(&src, "pub async fn start_process("));
    assert!(
        !body.contains("EVENT_BUS.publish"),
        "发起路径不得发布业务事件：批准事件只能来自真实人工审批完成的推进/撤回路径。函数体:\n{body}"
    );
}

// ---------------------------------------------------------------------------
// 2) 活库行为锁（TEST_DATABASE_URL 指向已迁移 PostgreSQL；CI 侧以
//    --test-threads=1 串行执行）
// ---------------------------------------------------------------------------

fn unique_tag() -> i64 {
    Utc::now()
        .timestamp_nanos_opt()
        .expect("测试造数取当前时刻纳秒：Utc::now 必落在 chrono 纳秒可表示区间，None 不可达")
}

async fn seed_user(db: &DatabaseConnection, role: &str) -> i32 {
    let now = Utc::now();
    let name = format!("bpm-lock-{role}-{}", unique_tag());
    user::ActiveModel {
        username: Set(name.clone()),
        password_hash: Set("x".repeat(60)),
        real_name: Set(Some(name)),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
    .id
}

async fn seed_definition(db: &DatabaseConnection, code: &str, config: serde_json::Value) -> i32 {
    let now = Utc::now();
    bpm_process_definition::ActiveModel {
        name: Set(format!("{code}-契约锁流程")),
        code: Set(code.to_string()),
        config: Set(Some(config)),
        status: Set("ACTIVE".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
    .id
}

fn start_req(code: &str, business_id: i64, initiator_id: i32) -> StartProcessRequest {
    StartProcessRequest {
        process_key: code.to_string(),
        business_type: "quotation".to_string(),
        business_id,
        title: "契约锁发起单".to_string(),
        initiator_id,
        initiator_name: String::new(),
        initiator_department_id: None,
        priority: None,
        form_data: None,
        variables: None,
    }
}

/// 引擎词表的正确形状：start → user_task(办理人=assignee) → end，含 edges。
fn engine_vocab_config(assignee_id: i32) -> serde_json::Value {
    json!({
        "nodes": [
            { "id": "start", "type": "start_event", "name": "开始" },
            { "id": "approve_node", "type": "user_task", "name": "审批",
              "assignee_value": assignee_id.to_string() },
            { "id": "end", "type": "end_event", "name": "结束" }
        ],
        "edges": [
            { "source": "start", "target": "approve_node" },
            { "source": "approve_node", "target": "end" }
        ]
    })
}

/// 守卫②形态一（前端旧表单存出的现实形状）：start_event 的首条出边指向
/// type=approval 的节点——解析器不校验目标类型、原样返回，且该节点带可解析的
/// assignee_value，只补 None 分支守卫的"半修"会让它成功建单，必须被类型守卫拒绝。
fn unknown_node_type_config() -> serde_json::Value {
    json!({
        "nodes": [
            { "id": "start", "type": "start_event", "name": "开始" },
            { "id": "approve_node", "type": "approval", "name": "审批",
              "assignee_value": "1" },
            { "id": "end", "type": "end_event", "name": "结束" }
        ],
        "edges": [
            { "source": "start", "target": "approve_node" },
            { "source": "approve_node", "target": "end" }
        ]
    })
}

/// 守卫②形态二：start→end 直连、无任何审批节点——首边目标 end_event 被解析器
/// 原样带回（非 None），同样必须按"不是可执行的审批节点"拒绝。
fn only_start_end_config() -> serde_json::Value {
    json!({
        "nodes": [
            { "id": "start", "type": "start_event", "name": "开始" },
            { "id": "end", "type": "end_event", "name": "结束" }
        ],
        "edges": [ { "source": "start", "target": "end" } ]
    })
}

/// 守卫①形态（解析结果为 None）：节点词表全不被引擎认识、既无 start_event 也
/// 无 user_task 可回退，回退与首边两条路径都取不到节点。
fn no_resolvable_node_config() -> serde_json::Value {
    json!({
        "nodes": [
            { "id": "n1", "type": "approval", "name": "审批节点",
              "assignee_value": "1" }
        ],
        "edges": []
    })
}

/// 守卫③形态：首节点类型正确（user_task）但未配置审批人——不得建出无人可审的
/// 孤儿工单，必须拒绝发起。
fn user_task_without_assignee_config() -> serde_json::Value {
    json!({
        "nodes": [
            { "id": "start", "type": "start_event", "name": "开始" },
            { "id": "approve_node", "type": "user_task", "name": "审批" },
            { "id": "end", "type": "end_event", "name": "结束" }
        ],
        "edges": [
            { "source": "start", "target": "approve_node" },
            { "source": "approve_node", "target": "end" }
        ]
    })
}

/// 推进路径的目标节点类型不受支持：t1 的出边指向一个 type=approval 的节点。
fn unknown_target_on_advance_config(assignee_id: i32) -> serde_json::Value {
    json!({
        "nodes": [
            { "id": "start", "type": "start_event", "name": "开始" },
            { "id": "t1", "type": "user_task", "name": "一级审批",
              "assignee_value": assignee_id.to_string() },
            { "id": "ghost", "type": "approval", "name": "二级审批" }
        ],
        "edges": [
            { "source": "start", "target": "t1" },
            { "source": "t1", "target": "ghost" }
        ]
    })
}

/// 拒绝路径的事务纪律：实例与任务一行都不许留下（半写实例=回滚失效）。
async fn assert_rolled_back(db: &DatabaseConnection, business_id: i64, definition_id: i32) {
    let instances = bpm_process_instance::Entity::find()
        .filter(bpm_process_instance::Column::BusinessType.eq("quotation"))
        .filter(bpm_process_instance::Column::BusinessId.eq(business_id))
        .all(db)
        .await
        .unwrap();
    assert!(
        instances.is_empty(),
        "拒绝发起必须整体回滚：不得留下任何状态的实例行，实得 {} 条",
        instances.len()
    );
    let tasks = bpm_task::Entity::find()
        .filter(bpm_task::Column::ProcessDefinitionId.eq(definition_id))
        .all(db)
        .await
        .unwrap();
    assert!(
        tasks.is_empty(),
        "拒绝发起不得留下工单行，实得 {} 条",
        tasks.len()
    );
}

/// 断言内层是可外显校验错误、点名"可执行的审批节点"并声明已拒绝发起
/// （守卫①文案"没有可执行的审批节点"、守卫②文案"不是可执行的审批节点"共同满足）。
fn assert_rejected_for_missing_approver(err: AppError) {
    match err {
        AppError::ValidationErrorDisplayable(msg) => {
            assert!(
                msg.contains("可执行的审批节点") && msg.contains("已拒绝发起审批"),
                "拒绝文案必须让使用者知道定义没有可执行的审批节点且本次发起已被拒绝，实得: {msg}"
            );
        }
        other => panic!("应返回可外显的校验错误，实得: {other:?}"),
    }
}

/// 正向锁：引擎词表的正确形状发起成功——恰好一条 pending 工单，实例不落已完成。
#[tokio::test]
async fn start_process_with_engine_vocab_creates_pending_task() {
    let db = Arc::new(test_common::setup_test_db().await);
    let initiator = seed_user(&db, "initiator").await;
    let approver = seed_user(&db, "approver").await;
    let code = format!("bpm_lock_ok_{}", unique_tag());
    seed_definition(&db, &code, engine_vocab_config(approver)).await;

    let business_id = unique_tag();
    let resp = BpmService::new(db.clone())
        .start_process(start_req(&code, business_id, initiator))
        .await
        .expect("词表正确且配了审批人的定义必须能正常发起");
    assert_eq!(resp.task_ids.len(), 1, "发起应恰好生成一条首任务");

    let task = bpm_task::Entity::find_by_id(resp.task_ids[0])
        .one(&*db)
        .await
        .unwrap()
        .expect("首任务行必须已落库");
    assert_eq!(task.status.as_deref(), Some(task_status::PENDING));
    assert_eq!(
        task.actual_handler_id,
        Some(approver),
        "工单必须落到真实审批人"
    );

    let instance = bpm_process_instance::Entity::find_by_id(resp.instance_id)
        .one(&*db)
        .await
        .unwrap()
        .expect("实例行必须已落库");
    assert_eq!(
        instance.status.as_deref(),
        Some(instance_status::PROCESSING)
    );
    assert!(
        instance.completed_at.is_none(),
        "正常发起的实例不得带完成时间（未审批即完成的形态即本锁要拦的旁路）"
    );
}

/// 反向锁 A（本修复核心·守卫②错形状分支）：首边指向 type=approval 且带可解析
/// assignee_value 的节点 → 必须拒绝发起、实例与任务零残留。
/// 只补 None 分支守卫的半修在此红：旧形态会解析出该节点、审批人也解析得出，
/// 直接建单返回 Ok（工单挂到 assignee_value 里的 1 号用户，单据照常进"审批"）。
#[tokio::test]
async fn start_process_rejects_definition_with_unknown_node_type() {
    let db = Arc::new(test_common::setup_test_db().await);
    let initiator = seed_user(&db, "initiator").await;
    let code = format!("bpm_lock_unknown_type_{}", unique_tag());
    let definition_id = seed_definition(&db, &code, unknown_node_type_config()).await;

    let business_id = unique_tag();
    let err = BpmService::new(db.clone())
        .start_process(start_req(&code, business_id, initiator))
        .await
        .expect_err("没有可执行审批节点的定义必须被拒绝发起，禁止自动批准");
    assert_rejected_for_missing_approver(err);
    assert_rolled_back(&db, business_id, definition_id).await;
}

/// 反向锁 B（守卫②错形状分支·end_event 直连形态）：start→end 的解析结果是非 None
/// 的 end_event 节点，旧形态会把它当首任务节点建单——必须同样拒绝、零残留。
#[tokio::test]
async fn start_process_rejects_definition_with_only_start_end() {
    let db = Arc::new(test_common::setup_test_db().await);
    let initiator = seed_user(&db, "initiator").await;
    let code = format!("bpm_lock_no_approver_{}", unique_tag());
    let definition_id = seed_definition(&db, &code, only_start_end_config()).await;

    let business_id = unique_tag();
    let err = BpmService::new(db.clone())
        .start_process(start_req(&code, business_id, initiator))
        .await
        .expect_err("只有起止节点的定义不得被判定为已完成且批准");
    assert_rejected_for_missing_approver(err);
    assert_rolled_back(&db, business_id, definition_id).await;
}

/// 反向锁 C（守卫①None 分支）：节点词表全不被认识、首边与回退两条路径都取不到
/// 节点（解析结果为 None）→ 拒绝发起、零残留。只补类型守卫而漏 None 分支的半修在此红。
#[tokio::test]
async fn start_process_rejects_definition_with_unresolvable_nodes() {
    let db = Arc::new(test_common::setup_test_db().await);
    let initiator = seed_user(&db, "initiator").await;
    let code = format!("bpm_lock_unresolvable_{}", unique_tag());
    let definition_id = seed_definition(&db, &code, no_resolvable_node_config()).await;

    let business_id = unique_tag();
    let err = BpmService::new(db.clone())
        .start_process(start_req(&code, business_id, initiator))
        .await
        .expect_err("解析不到任何首任务节点的定义必须被拒绝发起");
    assert_rejected_for_missing_approver(err);
    assert_rolled_back(&db, business_id, definition_id).await;
}

/// 反向锁 D（守卫③审批人缺失）：首节点类型正确但未配审批人 → 拒绝发起，
/// 不得建出无人可审的孤儿工单（该守卫在解析审批人的既有实现里，本锁钉其不回退）。
#[tokio::test]
async fn start_process_rejects_user_task_without_assignee() {
    let db = Arc::new(test_common::setup_test_db().await);
    let initiator = seed_user(&db, "initiator").await;
    let code = format!("bpm_lock_no_assignee_{}", unique_tag());
    let definition_id = seed_definition(&db, &code, user_task_without_assignee_config()).await;

    let business_id = unique_tag();
    let err = BpmService::new(db.clone())
        .start_process(start_req(&code, business_id, initiator))
        .await
        .expect_err("未配置审批人的审批节点不得建孤儿工单");
    match err {
        AppError::ValidationErrorDisplayable(msg) => {
            assert!(
                msg.contains("未配置审批人"),
                "拒绝文案必须点名节点未配置审批人，实得: {msg}"
            );
        }
        other => panic!("应返回可外显的校验错误，实得: {other:?}"),
    }
    assert_rolled_back(&db, business_id, definition_id).await;
}

/// 推进路径锁：审批通过后出边目标节点类型不受支持 → 拒绝本次审批并整体回滚，
/// 任务保持 pending、实例保持 PROCESSING，不得按"流程正常结束"批准。
#[tokio::test]
async fn approve_rejects_target_node_with_unsupported_type() {
    let db = Arc::new(test_common::setup_test_db().await);
    let initiator = seed_user(&db, "initiator").await;
    let approver = seed_user(&db, "approver").await;
    let code = format!("bpm_lock_advance_bad_target_{}", unique_tag());
    seed_definition(&db, &code, unknown_target_on_advance_config(approver)).await;

    let business_id = unique_tag();
    let svc = BpmService::new(db.clone());
    let resp = svc
        .start_process(start_req(&code, business_id, initiator))
        .await
        .expect("首节点形状正确，发起应成功");
    let task_id = resp.task_ids[0];

    let err = svc
        .approve_task(
            ApproveTaskRequest {
                task_id,
                handler_id: approver,
                handler_name: String::new(),
                action: "approve".to_string(),
                approval_opinion: None,
                attachment_urls: None,
            },
            Some(approver),
        )
        .await
        .expect_err("下一节点类型不受支持时必须拒绝本次审批，禁止静默收尾批准");
    match err {
        AppError::ValidationErrorDisplayable(msg) => {
            assert!(
                msg.contains("不受支持"),
                "拒绝文案必须点名节点类型不受支持，实得: {msg}"
            );
        }
        other => panic!("应返回可外显的校验错误，实得: {other:?}"),
    }

    let task = bpm_task::Entity::find_by_id(task_id)
        .one(&*db)
        .await
        .unwrap()
        .expect("审批被拒后原任务行仍在（回滚而非改写）");
    assert_eq!(
        task.status.as_deref(),
        Some(task_status::PENDING),
        "审批整体回滚后任务必须仍是 pending"
    );
    assert!(task.handled_at.is_none(), "被拒绝的审批不得留下办理时间");

    let instance = bpm_process_instance::Entity::find_by_id(resp.instance_id)
        .one(&*db)
        .await
        .unwrap()
        .expect("实例行仍在");
    assert_eq!(
        instance.status.as_deref(),
        Some(instance_status::PROCESSING),
        "定义配置错误时实例不得被判完成"
    );
}
