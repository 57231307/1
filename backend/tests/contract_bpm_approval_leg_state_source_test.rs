//! 契约锁：同一单据经任一入口（流程审批中心 / 单据快捷审批）办理后，
//! 落库状态、审批人记录、时间戳、审批历史轨迹完全同源
//!
//! 钉住的机理（正向与反向两侧都要，实现者不自审，负例造境即为变异自证）：
//! 1. 单一引擎办理腿：流程审批中心 execute_approval 与每条快捷审批回写最终都调用
//!    `BpmService::approve_task` 这一个 service 函数；任务态、处理人、时间戳、
//!    `approval_opinion` 全部由 `bpm_ops/task.rs` 的 `update_task_status` 一处写入，
//!    状态门（仅 pending 可办理）与词表（小写 pending/completed/rejected）同源。
//! 2. 单一业务写腿：中心经事件回写调用 `approve_via_bpm`，快捷经 `approve` 调用，
//!    两者共用 `build_approved_quotation_active_model`，落库业务状态取值逐字符一致。
//! 3. 历史轨迹同源（本锁新钉的缺陷）：快捷审批「通过」侧曾把采集到的意见在回写
//!    `approve_task` 时丢成 `None`，而流程审批中心把意见写入 `bpm_task.approval_opinion`，
//!    导致同一单据两入口办理后审批轨迹不一致。本锁静态负例钉死「APPROVE 侧回写不得
//!    传字面量 None」，活体正例断言两入口对同一意见产出逐字段一致的轨迹，
//!    二者构成变异自证：一旦回潮传 None，静态负例命中、活体轨迹等式判红。
//!
//! 静态段不依赖活库（CI 常规可跑）；活体段需 `TEST_DATABASE_URL` 指向已迁移
//! PostgreSQL（夹具语义见 tests/test_common）。

mod test_common;

use std::fs;
use std::path::PathBuf;

use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::str::FromStr;
use std::sync::Arc;

use bingxi_backend::models::dto::bpm_dto::ApproveTaskRequest;
use bingxi_backend::models::status::bpm_instance as instance_status;
use bingxi_backend::models::status::bpm_task as task_status;
use bingxi_backend::models::status::{quotation, quotation_ext};
use bingxi_backend::models::{
    bpm_process_definition, bpm_process_instance, bpm_task, customer, sales_quotation, user,
};
use bingxi_backend::services::bpm_ops::task::{APPROVE_ACTION, REJECT_ACTION};
use bingxi_backend::services::bpm_service::BpmService;
use bingxi_backend::services::quotation_approval_service::QuotationApprovalService;

fn read_rel(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("读取 {:?} 失败: {}", path, e))
        .replace('\r', "")
}

/// 剔除以 `//`（含 `///`）起始的行：守卫对象是代码；文档注释里点名被禁形态
/// （说明缺陷机理）是正当用途，不应误报。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 取函数体：从签名处起，到下一个同级 fn 标记前为止（与同族契约锁同口径，按函数体收窄判据）。
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

/// 把连续空白压成单空格，便于跨行匹配「APPROVE_ACTION, None」这类实参序形态。
fn collapse_ws(src: &str) -> String {
    src.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ---------------------------------------------------------------------------
// 1) 静态形态锁（不依赖活库，CI 常规可跑）
// ---------------------------------------------------------------------------

/// 单一引擎办理腿：流程审批中心 execute_approval 必须调用 approve_task；
/// 任务态、处理人、时间戳、意见的写入与状态门、词表校验全部收敛在 task.rs 一处。
#[test]
fn center_entry_routes_through_single_engine_leg() {
    let handler = code_only(&read_rel("src/handlers/bpm_handler.rs"));
    let exec_body = fn_body(&handler, "pub async fn execute_approval(");
    assert!(
        exec_body.contains(".approve_task(approve_req"),
        "流程审批中心 execute_approval 必须经 approve_task 单点办理，否则中心与快捷各写一套引擎状态"
    );

    let task = code_only(&read_rel("src/services/bpm_ops/task.rs"));
    // 状态门与词表同源：门判定用写入方小写常量，完成/拒绝态同源
    assert!(
        task.contains("task_status::PENDING")
            && task.contains("task_status::COMPLETED")
            && task.contains("task_status::REJECTED"),
        "approve_task 的状态门与落库态必须引 bpm_task 写入方词表常量，禁止别处另立一套大小写"
    );
    assert!(
        task.contains("validate_approve_action")
            && task.contains("ALL_APPROVE_ACTIONS"),
        "审批动作取值域必须由 validate_approve_action 单点校验，禁止各处裸比较 action 字符串"
    );
    // 快捷审批回写的引擎写口唯一：close_pending_tasks_for_business 内部必须调 approve_task
    let close_body = fn_body(&task, "pub async fn close_pending_tasks_for_business(");
    assert!(
        close_body.contains(".approve_task(req"),
        "快捷审批关闭待办必须复用 approve_task 引擎写口，禁止在 close 里另写一套任务状态更新"
    );
    assert!(
        close_body.contains("approval_opinion: reason.clone()"),
        "close_pending_tasks_for_business 必须把调用方转达的意见作为 approval_opinion 传给 approve_task"
    );
}

/// 变异自证负例（静态）：四族快捷审批「通过」回写不得把意见丢成字面量 None。
///
/// 回潮形态是 `...APPROVE_ACTION, None, user_id)` 或 `action: APPROVE_ACTION ... approval_opinion: None`；
/// 一旦任一文件重新出现，本锁判红（这就是对「历史轨迹两入口不同源」缺陷的变异自证）。
#[test]
fn quick_approve_passes_must_not_drop_opinion_to_none() {
    // quotation 通过链：approve 采集 approval_reason，必须转达 handle_bpm_..._after_commit 再进 approve_task
    let q = code_only(&read_rel("src/services/quotation_approval_service.rs"));
    let q_approve = fn_body(&q, "pub async fn approve(");
    assert!(
        q_approve.contains("bpm_opinion") && q_approve.contains("handle_bpm_quotation_approval_after_commit"),
        "报价快捷通过必须把采集的通过意见转达 BPM 回写（bpm_opinion），不得丢弃"
    );
    let q_writeback = fn_body(
        &q,
        "async fn handle_bpm_quotation_approval_after_commit(",
    );
    assert!(
        q_writeback.contains("approval_opinion: approval_opinion.clone()"),
        "报价回写 approve_task 的 approval_opinion 必须取转达值，禁止硬编码 None"
    );
    assert!(
        !collapse_ws(&q_writeback).contains("approval_opinion: None"),
        "变异自证：报价回写 approve_task 出现 approval_opinion: None 即两入口轨迹分裂回潮"
    );

    // PO / SO / 采购退货 approve 侧：close_pending_tasks_for_business 的 APPROVE 实参不得为 None
    let cases = [
        ("src/services/po/contract.rs", "APPROVE_ACTION"),
        (
            "src/services/so/order_workflow.rs",
            "APPROVE_ACTION",
        ),
        (
            "src/services/purchase_return_service.rs",
            "APPROVE_ACTION",
        ),
    ];
    for (file, action) in cases {
        let body = collapse_ws(&code_only(&read_rel(file)));
        // 定位 close_pending_tasks_for_business 调用点：APPROVE_ACTION 之后的意见实参
        let mut idx = 0usize;
        let pattern = format!("{}, ", action);
        let mut found_call = false;
        while let Some(pos) = body[idx..].find(&pattern) {
            found_call = true;
            let after = &body[idx + pos + pattern.len()..];
            let opinion_arg = after.split(',').next().unwrap_or("").trim();
            assert!(
                opinion_arg != "None",
                "{file}：快捷审批通过经 close_pending_tasks_for_business 回写时把意见丢成 None，两入口历史轨迹不同源（变异自证命中）"
            );
            idx += pos + pattern.len();
        }
        assert!(
            found_call,
            "{file}：未找到 APPROVE 侧 close_pending_tasks_for_business 调用点，锁对象缺失"
        );
    }
}

// ---------------------------------------------------------------------------
// 2) 活体真值锁（TEST_DATABASE_URL 指向已迁移 PostgreSQL；CI 串行组执行）
// ---------------------------------------------------------------------------

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn unique_tag() -> i64 {
    Utc::now()
        .timestamp_nanos_opt()
        .expect("测试造数取当前时刻纳秒：Utc::now 必落在 chrono 纳秒可表示区间，None 不可达")
}

fn today() -> NaiveDate {
    Utc::now().date_naive()
}

async fn seed_user(db: &DatabaseConnection, name: &str) -> i32 {
    let now = Utc::now();
    user::ActiveModel {
        username: Set(name.to_string()),
        password_hash: Set("x".repeat(60)),
        real_name: Set(Some(name.to_string())),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("种子用户失败")
    .id
}

async fn seed_customer(db: &DatabaseConnection, owner_id: i32) -> i32 {
    let now = Utc::now();
    customer::ActiveModel {
        customer_code: Set(format!("CUS-SRC-{}", unique_tag())),
        customer_name: Set("状态同源锁客户".to_string()),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
        customer_type: Set("retail".to_string()),
        owner_id: Set(owner_id),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("种子客户失败")
    .id
}

/// 激活单节点审批流程定义：start → user_task(办理人=assignee_id) → end。
async fn seed_active_definition(db: &DatabaseConnection, code: &str, assignee_id: i32) {
    let now = Utc::now();
    let config = serde_json::json!({
        "nodes": [
            { "id": "start", "type": "start_event", "name": "提交审批" },
            { "id": "approve_task", "type": "user_task", "name": "人工审批",
              "assignee_type": "user", "assignee_value": assignee_id },
            { "id": "end", "type": "end_event", "name": "完成" }
        ],
        "edges": [
            { "source": "start", "target": "approve_task" },
            { "source": "approve_task", "target": "end" }
        ]
    });
    bpm_process_definition::ActiveModel {
        name: Set(format!("{code}-状态同源锁流程")),
        code: Set(code.to_string()),
        config: Set(Some(config)),
        status: Set("ACTIVE".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("种子流程定义失败");
}

async fn seed_quotation(db: &DatabaseConnection, customer_id: i32, user_id: i32) -> i64 {
    let now = Utc::now();
    sales_quotation::ActiveModel {
        quotation_no: Set(format!("QT-SRC-{}", unique_tag())),
        customer_id: Set(customer_id),
        sales_user_id: Set(user_id as i64),
        quotation_date: Set(today()),
        valid_until: Set(today()),
        price_terms: Set("FOB".to_string()),
        subtotal: Set(dec("300000")),
        tax_amount: Set(dec("0")),
        total_amount: Set(dec("300000")),
        status: Set(quotation::DRAFT.to_string()),
        created_by: Set(user_id as i64),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("种子报价单失败")
    .id
}

/// 取实例下唯一待办任务（写入方同源小写常量过滤）。
async fn pending_task(db: &DatabaseConnection, instance_id: i32) -> bpm_task::Model {
    bpm_task::Entity::find()
        .filter(bpm_task::Column::InstanceId.eq(instance_id))
        .filter(bpm_task::Column::Status.eq(task_status::PENDING))
        .one(db)
        .await
        .unwrap()
        .expect("实例下必须有待办任务")
}

/// 双向往等价（正向通过 + 反向驳回）合并成单用例：
/// 同一测试二进制并行两 tokio 会撞 `bpm_process_definition_code_key` 唯一约束
/// （两 test 都要 seed 同 code 定义，cargo 并行线程 setup_test_db 各自 TRUNCATE 会互相拆台）；
/// 合入一条 test 内顺序执行即可完全避锋，正向与反向断言逻辑不变。
#[tokio::test]
async fn center_and_quick_are_state_identical_live() {
    let db = Arc::new(test_common::setup_test_db().await);
    let initiator = seed_user(&db, "src_initiator").await;
    let approver = seed_user(&db, "src_approver").await;
    let customer_id = seed_customer(&db, initiator).await;
    seed_active_definition(&db, "quotation_approval", approver).await;
    let svc = QuotationApprovalService::new(db.clone());

    // ---------- 正向（通过）----------
    let opinion = "同意：单价符合协议区间与报价口径";
    let a_id = seed_quotation(&db, customer_id, initiator).await;
    let a_sub = svc
        .submit(a_id, initiator)
        .await
        .expect("A 提交应启动 BPM");
    assert_eq!(a_sub.status, quotation_ext::PENDING_APPROVAL);
    let a_instance_id = a_sub.approval_instance_id.expect("A 应关联实例") as i32;
    let a_task = pending_task(&db, a_instance_id).await;
    BpmService::new(db.clone())
        .approve_task(
            ApproveTaskRequest {
                task_id: a_task.id,
                handler_id: approver,
                handler_name: String::new(),
                action: APPROVE_ACTION.to_string(),
                approval_opinion: Some(opinion.to_string()),
                attachment_urls: None,
            },
            Some(approver),
        )
        .await
        .expect("中心办理通过失败");
    svc.approve_via_bpm(a_id, approver)
        .await
        .expect("中心侧业务回写通过失败");

    let b_id = seed_quotation(&db, customer_id, initiator).await;
    let b_sub = svc
        .submit(b_id, initiator)
        .await
        .expect("B 提交应启动 BPM");
    let b_instance_id = b_sub.approval_instance_id.expect("B 应关联实例") as i32;
    let _ = pending_task(&db, b_instance_id).await;
    svc.approve(b_id, approver, opinion.to_string())
        .await
        .expect("快捷审批通过失败");

    let a_task = bpm_task::Entity::find_by_id(a_task.id)
        .one(&*db)
        .await
        .unwrap()
        .expect("A 任务行必须仍在");
    let b_task = handled_task_of_instance(&db, b_instance_id).await;

    assert_eq!(a_task.status, b_task.status, "两入口办理后任务态必须同源");
    assert_eq!(a_task.status.as_deref(), Some(task_status::COMPLETED));
    assert_eq!(
        b_task.status.as_deref(),
        Some(task_status::COMPLETED),
        "快捷通过也必须把任务推进为 completed"
    );
    assert_eq!(
        a_task.actual_handler_id, b_task.actual_handler_id,
        "两入口审批人记录必须同源（均取已认证操作人）"
    );
    assert_eq!(a_task.actual_handler_id, Some(approver));
    assert_eq!(
        a_task.approval_opinion, b_task.approval_opinion,
        "两入口审批历史轨迹（approval_opinion）必须逐字同源"
    );
    assert_eq!(
        a_task.approval_opinion.as_deref(),
        Some(opinion),
        "中心侧意见必须落 bpm_task.approval_opinion"
    );
    assert!(
        a_task.handled_at.is_some() && b_task.handled_at.is_some(),
        "两入口都必须落处理时间戳"
    );
    let a_row = sales_quotation::Entity::find_by_id(a_id)
        .one(&*db)
        .await
        .unwrap()
        .expect("A 报价行必须仍在");
    let b_row = sales_quotation::Entity::find_by_id(b_id)
        .one(&*db)
        .await
        .unwrap()
        .expect("B 报价行必须仍在");
    assert_eq!(a_row.status, b_row.status, "两入口业务落库状态必须同源");
    assert_eq!(a_row.status, quotation::APPROVED);
    assert_eq!(a_row.approved_by, b_row.approved_by);
    let a_inst = bpm_process_instance::Entity::find_by_id(a_instance_id)
        .one(&*db)
        .await
        .unwrap()
        .expect("A 实例必须仍在");
    let b_inst = bpm_process_instance::Entity::find_by_id(b_instance_id)
        .one(&*db)
        .await
        .unwrap()
        .expect("B 实例必须仍在");
    assert_eq!(a_inst.status, b_inst.status);
    assert_eq!(a_inst.status.as_deref(), Some(instance_status::COMPLETED));

    // ---------- 反向（驳回）----------
    let reason = "单价超出客户协议上限，拒绝";
    let c_id = seed_quotation(&db, customer_id, initiator).await;
    let c_sub = svc
        .submit(c_id, initiator)
        .await
        .expect("C 提交失败");
    let c_instance_id = c_sub.approval_instance_id.expect("C 实例") as i32;
    let c_task = pending_task(&db, c_instance_id).await;
    BpmService::new(db.clone())
        .approve_task(
            ApproveTaskRequest {
                task_id: c_task.id,
                handler_id: approver,
                handler_name: String::new(),
                action: REJECT_ACTION.to_string(),
                approval_opinion: Some(reason.to_string()),
                attachment_urls: None,
            },
            Some(approver),
        )
        .await
        .expect("中心驳回失败");
    svc.reject_via_bpm(c_id, reason.to_string(), approver)
        .await
        .expect("中心侧业务回写驳回失败");

    let d_id = seed_quotation(&db, customer_id, initiator).await;
    let d_sub = svc
        .submit(d_id, initiator)
        .await
        .expect("D 提交失败");
    let d_instance_id = d_sub.approval_instance_id.expect("D 实例") as i32;
    svc.reject(d_id, approver, reason.to_string())
        .await
        .expect("快捷驳回失败");

    let c_task = bpm_task::Entity::find_by_id(c_task.id)
        .one(&*db)
        .await
        .unwrap()
        .expect("C 任务必须仍在");
    let d_task = handled_task_of_instance(&db, d_instance_id).await;

    assert_eq!(c_task.status, d_task.status);
    assert_eq!(c_task.status.as_deref(), Some(task_status::REJECTED));
    assert_eq!(c_task.actual_handler_id, d_task.actual_handler_id);
    assert_eq!(c_task.approval_opinion, d_task.approval_opinion);
    assert_eq!(c_task.approval_opinion.as_deref(), Some(reason));
    assert!(c_task.handled_at.is_some() && d_task.handled_at.is_some());

    let c_inst = bpm_process_instance::Entity::find_by_id(c_instance_id)
        .one(&*db)
        .await
        .unwrap()
        .expect("C 实例必须仍在");
    let d_inst = bpm_process_instance::Entity::find_by_id(d_instance_id)
        .one(&*db)
        .await
        .unwrap()
        .expect("D 实例必须仍在");
    assert_eq!(c_inst.status, d_inst.status);
    assert_eq!(c_inst.status.as_deref(), Some(instance_status::TERMINATED));

    let c_row = sales_quotation::Entity::find_by_id(c_id)
        .one(&*db)
        .await
        .unwrap()
        .expect("C 行必须仍在");
    let d_row = sales_quotation::Entity::find_by_id(d_id)
        .one(&*db)
        .await
        .unwrap()
        .expect("D 行必须仍在");
    assert_eq!(c_row.status, d_row.status);
    assert_eq!(c_row.status, quotation::REJECTED);
}

/// 实例下已办理任务（快捷回写走完后 PENDING 已推进，只按实例定位）。
async fn handled_task_of_instance(db: &DatabaseConnection, instance_id: i32) -> bpm_task::Model {
    bpm_task::Entity::find()
        .filter(bpm_task::Column::InstanceId.eq(instance_id))
        .one(db)
        .await
        .unwrap()
        .expect("实例下必须存在已办理任务")
}
