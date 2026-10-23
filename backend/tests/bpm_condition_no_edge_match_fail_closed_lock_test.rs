//! 契约锁：条件分支全部不匹配（无边命中）不得自动批准——发起即拒，推进侧第二道防线
//!
//! 背景缺陷（第三处 fail-open，用户裁定分支语义变更）：审批节点存在出边但
//! condition 求值全部不通过时，旧实现落到"完成收尾"（实例置 COMPLETED 并发布
//! approved=true 事件），等于绕过人工审批自动批准。裁定：改为显式失败，并在
//! 发起路径就给结论（实例变量发起后不可变、条件求值确定，故可干跑预演），
//! 不允许用户看到实例已创建却在节点上卡死或静默放行；存量定义若不覆盖
//! 某些条件组合，会在下一次发起时当场暴露，属预期。
//!
//! 本文件钉死的不变量：
//! 1. 发起侧（核心）：条件分支与发起数据全部不匹配的定义 → start_process 拒绝，
//!    可外显校验错误点名"补全条件覆盖或增加默认分支"，实例与工单零残留。
//! 2. 发起侧：审批节点无出边（链不闭合）同样当场拒绝——推进侧对该形状保留
//!    现行完成收尾只服务于本规则生效前的历史在途实例，新实例不得再进入。
//! 3. 正向锁：补一条无条件默认分支后发起成功，首节点批准后推进到默认分支目标，
//!    绝不因"曾有不匹配的条件边"而提前完成。
//! 4. 推进侧防线：规则生效前已创建的历史在途实例（直接种实例与工单绕过发起校验），
//!    批准时条件分支全部不匹配 → 拒绝本次审批并整体回滚，任务保持 pending、
//!    实例保持 PROCESSING，不得置 COMPLETED、不得发布批准事件。
//! 5. 静态锁：推进函数体必须存在"出边为空判断 + 全不匹配显式拒绝"两个要件；
//!    发起函数体必须调用链干跑校验。缺任一即"半修"，本锁直接红。
//!
//! 检测力自证：把发起侧干跑删除或退回自动完成时，用例 1/2 在 expect_err 处红
//! （发起会意外 Ok）；把推进侧退回 Ok(false) 完成收尾时，用例 4 在 expect_err
//! 处红且实例状态断言红；把默认分支语义改坏（空串条件当缺键省略）时用例 3 的
//! 推进目标断言红；静态锁在守卫要件被整体抹除时红。
//!
//! 静态锁不依赖活库，CI 常规测试阶段执行；活库行为锁按本仓 test_common 夹具
//! 口径（TEST_DATABASE_URL 指向已迁移 PostgreSQL），由 CI 侧执行且整档以
//! --test-threads=1 串行跑，逐用例 setup 互不串库。

mod test_common;

use std::fs;
use std::path::PathBuf;

use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set};
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

/// 取函数体：从签名处开始，到下一个同级条目（4 空格缩进的 fn）出现之前为止。
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

/// 推进函数体两要件：出边为空判断（仅历史在途实例的完成收尾通道）与
/// "条件分支全部不匹配显式拒绝"，缺一即退回自动批准形态。
#[test]
fn advance_path_rejects_when_no_edge_matches() {
    let src = read_rel("src/services/bpm_ops/task.rs").replace('\r', "");
    let body = code_only(&fn_body(&src, "async fn try_advance_to_next_node("));
    assert!(
        body.contains("outgoing.is_empty()"),
        "推进函数必须先区分完全没有出边与有出边不匹配两种形状，缺失即退回旧形态。函数体:\n{body}"
    );
    assert!(
        body.contains("条件分支") && body.contains("AppError::validation_displayable"),
        "有出边但条件分支全部不匹配时必须以可外显校验错误拒绝本次审批，禁止落回完成收尾。函数体:\n{body}"
    );
}

/// 发起函数体必须挂链干跑：条件不覆盖的形状要在发起时给结论，而不是实例创建后卡死。
#[test]
fn start_path_dry_runs_forward_chain_before_committing() {
    let src = read_rel("src/services/bpm_ops/instance.rs").replace('\r', "");
    let body = code_only(&fn_body(&src, "async fn create_first_task("));
    assert!(
        body.contains("validate_forward_path("),
        "发起路径必须调用审批链干跑校验，缺失即把分支语义缺陷留给推进时暴露。函数体:\n{body}"
    );
    let walker = src
        .find("fn validate_forward_path(")
        .expect("链干跑函数必须存在");
    let walker_body = code_only(&src[walker..]);
    assert!(
        walker_body.contains("没有可执行的分支") && walker_body.contains("默认分支"),
        "干跑必须以点名补默认分支的文案拒绝全不匹配形状"
    );
    assert!(
        walker_body.contains("visited"),
        "干跑必须有防环访问集，否则成环定义会让发起侧死循环"
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
    let name = format!("bpm-cond-{role}-{}", unique_tag());
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
        name: Set(format!("{code}-条件分支流程")),
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

fn start_req(code: &str, business_id: i64, initiator_id: i32, amount: i64) -> StartProcessRequest {
    StartProcessRequest {
        process_key: code.to_string(),
        business_type: "quotation".to_string(),
        business_id,
        title: "条件分支契约锁发起单".to_string(),
        initiator_id,
        initiator_name: String::new(),
        initiator_department_id: None,
        priority: None,
        form_data: None,
        variables: Some(json!({ "amount": amount })),
    }
}

/// 条件不覆盖形状：t1 两条出边分别要求 amount>10000 与 amount<=5000，
/// 发起数据 amount=8000 落在缝隙里，全部分支不匹配；big/small 之后各自闭链到
/// end（保证除条件覆盖缝隙外没有其他配置错误干扰断言）。
fn uncovered_condition_config(approver_id: i32) -> serde_json::Value {
    json!({
        "nodes": [
            { "id": "start", "type": "start_event", "name": "开始" },
            { "id": "t1", "type": "user_task", "name": "初审",
              "assignee_value": approver_id.to_string() },
            { "id": "big", "type": "user_task", "name": "大额审批",
              "assignee_value": approver_id.to_string() },
            { "id": "small", "type": "user_task", "name": "小额审批",
              "assignee_value": approver_id.to_string() },
            { "id": "end", "type": "end_event", "name": "结束" }
        ],
        "edges": [
            { "source": "start", "target": "t1" },
            { "source": "t1", "target": "big", "condition": "${amount} > 10000" },
            { "source": "t1", "target": "small", "condition": "${amount} <= 5000" },
            { "source": "big", "target": "end" },
            { "source": "small", "target": "end" }
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

fn assert_rejected_condition_shape(err: AppError) {
    match err {
        AppError::ValidationErrorDisplayable(msg) => {
            assert!(
                msg.contains("条件分支") && msg.contains("默认分支"),
                "拒绝文案必须点名条件分支不覆盖并指引补默认分支，实得: {msg}"
            );
        }
        other => panic!("应返回可外显的校验错误，实得: {other:?}"),
    }
}

/// 核心锁（裁定形态）：条件分支与发起数据全部不匹配 → 发起即拒、零残留，
/// 绝不出现"实例已创建却自动完成（等于自动批准）"或"创建后卡死"两种旧形态。
#[tokio::test]
async fn start_rejects_when_no_condition_edge_matches() {
    let db = Arc::new(test_common::setup_test_db().await);
    let initiator = seed_user(&db, "initiator").await;
    let approver = seed_user(&db, "approver").await;
    let code = format!("bpm_cond_uncovered_{}", unique_tag());
    let definition_id = seed_definition(&db, &code, uncovered_condition_config(approver)).await;

    let business_id = unique_tag();
    let err = BpmService::new(db.clone())
        .start_process(start_req(&code, business_id, initiator, 8000))
        .await
        .expect_err("条件分支全部不匹配的定义必须发起即拒，禁止自动批准");
    assert_rejected_condition_shape(err);
    assert_rolled_back(&db, business_id, definition_id).await;
}

/// 链不闭合形状：首任务节点没有任何出边 → 发起当场拒绝（推进侧保留的完成
/// 收尾通道对新实例不可达，此锁钉住发起侧拦截）。
#[tokio::test]
async fn start_rejects_when_first_task_has_no_outgoing_edge() {
    let db = Arc::new(test_common::setup_test_db().await);
    let initiator = seed_user(&db, "initiator").await;
    let approver = seed_user(&db, "approver").await;
    let code = format!("bpm_cond_open_chain_{}", unique_tag());
    let config = json!({
        "nodes": [
            { "id": "start", "type": "start_event", "name": "开始" },
            { "id": "t1", "type": "user_task", "name": "初审",
              "assignee_value": approver.to_string() }
        ],
        "edges": [ { "source": "start", "target": "t1" } ]
    });
    let definition_id = seed_definition(&db, &code, config).await;

    let business_id = unique_tag();
    let err = BpmService::new(db.clone())
        .start_process(start_req(&code, business_id, initiator, 8000))
        .await
        .expect_err("审批链不闭合（节点无出边）的定义必须发起即拒");
    match err {
        AppError::ValidationErrorDisplayable(msg) => {
            assert!(
                msg.contains("没有连线"),
                "拒绝文案必须点名节点缺后续连线并指引补线，实得: {msg}"
            );
        }
        other => panic!("应返回可外显的校验错误，实得: {other:?}"),
    }
    assert_rolled_back(&db, business_id, definition_id).await;
}

/// 正向锁：补无条件默认分支后发起成功；amount=8000 条件边均不匹配时命中
/// 默认分支并推进到 fallback 工单（处理人是该节点配置的人，绝不提前完成）。
#[tokio::test]
async fn default_edge_makes_flow_advance_instead_of_auto_complete() {
    let db = Arc::new(test_common::setup_test_db().await);
    let initiator = seed_user(&db, "initiator").await;
    let approver = seed_user(&db, "approver").await;
    let code = format!("bpm_cond_default_{}", unique_tag());

    // 直接在字面量上构造含默认分支的完整定义（uncovered_condition_config 的
    // default_edge 分支只演示边集合，节点与边需同源于本处断言）
    let config = json!({
        "nodes": [
            { "id": "start", "type": "start_event", "name": "开始" },
            { "id": "t1", "type": "user_task", "name": "初审",
              "assignee_value": approver.to_string() },
            { "id": "big", "type": "user_task", "name": "大额审批",
              "assignee_value": approver.to_string() },
            { "id": "fallback", "type": "user_task", "name": "默认分支审批",
              "assignee_value": approver.to_string() },
            { "id": "end", "type": "end_event", "name": "结束" }
        ],
        "edges": [
            { "source": "start", "target": "t1" },
            { "source": "t1", "target": "big", "condition": "${amount} > 10000" },
            { "source": "t1", "target": "fallback" },
            { "source": "fallback", "target": "end" }
        ]
    });
    seed_definition(&db, &code, config).await;

    let business_id = unique_tag();
    let svc = BpmService::new(db.clone());
    let resp = svc
        .start_process(start_req(&code, business_id, initiator, 8000))
        .await
        .expect("存在默认分支的定义必须可发起（条件不覆盖不再是自动批准，也不得误拒可闭合定义）");
    let instance = bpm_process_instance::Entity::find_by_id(resp.instance_id)
        .one(&*db)
        .await
        .unwrap()
        .expect("实例行必须已落库");
    assert_eq!(
        instance.status.as_deref(),
        Some(instance_status::PROCESSING)
    );

    svc.approve_task(
        ApproveTaskRequest {
            task_id: resp.task_ids[0],
            handler_id: approver,
            handler_name: String::new(),
            action: "approve".to_string(),
            approval_opinion: None,
            attachment_urls: None,
        },
        Some(approver),
    )
    .await
    .expect("初审通过后应沿默认分支推进");

    let fallback_task = bpm_task::Entity::find()
        .filter(bpm_task::Column::InstanceId.eq(resp.instance_id))
        .filter(bpm_task::Column::NodeId.eq("fallback"))
        .one(&*db)
        .await
        .unwrap()
        .expect("默认分支工单必须真实创建，而不是自动完成");
    assert_eq!(fallback_task.status.as_deref(), Some(task_status::PENDING));
    let instance = bpm_process_instance::Entity::find_by_id(resp.instance_id)
        .one(&*db)
        .await
        .unwrap()
        .expect("实例行仍在");
    assert_eq!(
        instance.status.as_deref(),
        Some(instance_status::PROCESSING),
        "还有真实审批节点未过，实例不得被判完成"
    );
}

/// 推进侧防线锁（历史在途实例形状）：绕过发起校验直接种实例+待办工单，
/// 批准时条件分支全部不匹配 → 拒绝本次审批并整体回滚，任务保持 pending、
/// 实例保持 PROCESSING，不得置 COMPLETED（第二道防线防定义在途被改）。
#[tokio::test]
async fn legacy_inflight_instance_approve_rejects_when_no_edge_matches() {
    let db = Arc::new(test_common::setup_test_db().await);
    let approver = seed_user(&db, "approver").await;
    let code = format!("bpm_cond_legacy_{}", unique_tag());
    let definition_id = seed_definition(&db, &code, uncovered_condition_config(approver)).await;

    let now = Utc::now();
    let instance_no = format!("BPM-LEGACY-{}", unique_tag());
    let instance = bpm_process_instance::ActiveModel {
        process_definition_id: Set(definition_id),
        instance_no: Set(instance_no),
        business_type: Set("quotation".to_string()),
        business_id: Set(Some(unique_tag())),
        applicant_id: Set(approver),
        title: Set("历史在途实例造数".to_string()),
        initiator_id: Set(approver),
        initiator_name: Set(String::new()),
        status: Set(Some(instance_status::PROCESSING.to_string())),
        variables: Set(Some(json!({ "amount": 8000 }))),
        started_at: Set(Some(now)),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&*db)
    .await
    .unwrap();

    let task_no = format!("TSK-LEGACY-{}", unique_tag());
    let task = bpm_task::ActiveModel {
        instance_id: Set(instance.id),
        process_definition_id: Set(definition_id),
        task_no: Set(task_no),
        node_id: Set("t1".to_string()),
        node_name: Set("初审".to_string()),
        node_type: Set("user_task".to_string()),
        task_type: Set(Some("user_task".to_string())),
        actual_handler_id: Set(Some(approver)),
        status: Set(Some(task_status::PENDING.to_string())),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&*db)
    .await
    .unwrap();

    let err = BpmService::new(db.clone())
        .approve_task(
            ApproveTaskRequest {
                task_id: task.id,
                handler_id: approver,
                handler_name: String::new(),
                action: "approve".to_string(),
                approval_opinion: None,
                attachment_urls: None,
            },
            Some(approver),
        )
        .await
        .expect_err("条件分支全部不匹配时推进侧必须拒绝本次审批，禁止完成收尾等于自动批准");
    assert_rejected_condition_shape(err);

    let task = bpm_task::Entity::find_by_id(task.id)
        .one(&*db)
        .await
        .unwrap()
        .expect("审批被拒后原任务行仍在（回滚而非改写）");
    assert_eq!(
        task.status.as_deref(),
        Some(task_status::PENDING),
        "被拒绝的审批整体回滚后任务必须仍是 pending"
    );
    let instance = bpm_process_instance::Entity::find_by_id(instance.id)
        .one(&*db)
        .await
        .unwrap()
        .expect("实例行仍在");
    assert_eq!(
        instance.status.as_deref(),
        Some(instance_status::PROCESSING),
        "条件全不匹配不得把实例置成已完成"
    );
    let follow_ups = bpm_task::Entity::find()
        .filter(bpm_task::Column::InstanceId.eq(instance.id))
        .filter(bpm_task::Column::NodeId.is_in(vec!["big", "small", "fallback"]))
        .count(&*db)
        .await
        .unwrap();
    assert_eq!(follow_ups, 0, "被拒绝的审批不得创建任何后续工单");
}
