//! 契约锁：报价/生产工单「业务侧裁决后反向回写 BPM 任务」的状态比较点必须与写入方同源
//!
//! 根因（两条锁各自钉住的机理）：
//! 1. `bpm_task.status` 的写入方是 `bpm_ops/instance.rs`（建首任务）与
//!    `bpm_ops/task.rs`（建下一节点任务），均写 `models/status::bpm_task::PENDING`
//!    （值 `pending`，小写）；DB CHECK `chk_bpm_task_status` 钉死小写集
//!    （migration/src/domain/crm_vocab_check/mod.rs）。而两条反向回写链路此前分别用
//!    大写 `"PENDING"` 字面量与别域借来的 `common::STATUS_PENDING`（值 `PENDING`）
//!    作 `Column::Status.eq(...)` 精确过滤值 ⇒ 过滤恒不命中 ⇒ 回写循环恒空转 ⇒
//!    BPM 实例永滞留 PROCESSING、拒绝理由永远进不了 `bpm_task.approval_opinion`。
//!    本文件静态锁：回写函数体内禁现大写形态、必须引 `task_status::PENDING`，
//!    且必须按 `instance_id` 收窄（防「能匹配但误伤别的单据」的粗放回潮）。
//! 2. 行为锁（活库）：走真实创建路径（start_process）造实例+待办后，先负例断言
//!    大写过滤在该库上确实查不到行（钉住「大小写不同源=静默空转」这个机理本身），
//!    再执行业务侧审批/拒绝，断言任务态真推进、`approval_opinion` 真落值。
//!    活库用例负例里的 `"PENDING"` 字面量是被锁的「修前形态」，属本锁的合法造境，
//!    静态扫描只作用于 src 侧回写函数体，不作用于本测试文件。

mod test_common;

use std::fs;
use std::path::PathBuf;

use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use std::str::FromStr;
use std::sync::Arc;

use bingxi_backend::models::status::bpm_instance as instance_status;
use bingxi_backend::models::status::bpm_task as task_status;
use bingxi_backend::models::status::{common, master_data, production, quotation, quotation_ext};
use bingxi_backend::models::{
    bpm_process_definition, bpm_process_instance, bpm_task, customer, production_order,
    sales_quotation, user,
};
use bingxi_backend::services::bpm_service::BpmService;
use bingxi_backend::services::production_order_service::ProductionOrderService;
use bingxi_backend::services::quotation_approval_service::QuotationApprovalService;
use sea_orm::DatabaseConnection;

// ---------------------------------------------------------------------------
// 1) 静态形态锁（不依赖活库，CI 常规可跑）
// ---------------------------------------------------------------------------

fn read_rel(manifest_rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(manifest_rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {:?} 失败: {}", path, e))
}

/// 剔除以 `//`（含 `///` 文档注释）起始的行：守卫对象是「代码」里的大写字面量，
/// 文档注释中以反引号引用被禁形态（说明缺陷机理）是正当用途，不应被误报。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 取函数体：从签名处开始，到下一个同级条目（4 空格缩进的 fn）出现之前为止。
/// 判据按函数体作用域收窄（本仓有全文件 contains 误伤注释/他域字面量的前科）。
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

/// 基准：写入方词表为小写（本文件其余断言以它的常量值为源，词表变则一并暴露）
#[test]
fn bpm_task_write_side_word_list_is_lowercase() {
    assert_eq!(task_status::PENDING, "pending");
    assert_eq!(task_status::COMPLETED, "completed");
    assert_eq!(task_status::REJECTED, "rejected");
    // 对照：别域借来的大写 PENDING（曾被供应商用在两处回写过滤点）
    assert_eq!(common::STATUS_PENDING, "PENDING");
    assert_ne!(
        common::STATUS_PENDING,
        task_status::PENDING,
        "前置事实：两套词表大小写不同源，混用即恒不命中"
    );
}

fn assert_writeback_body_locked(label: &str, src_rel: &str, signature: &str) {
    let src = read_rel(src_rel).replace('\r', "");
    let body = fn_body(&src, signature);
    let code = code_only(&body);
    for forbidden in [
        r#""PENDING""#,
        "common::STATUS_PENDING",
        "status::common::STATUS_PENDING",
    ] {
        assert!(
            !code.contains(forbidden),
            "{label}（{src_rel} :: {signature}）禁止再出现大写 PENDING 过滤形态 {forbidden}；\
             bpm_task 待办状态必须引写入方同源常量 bpm_task::PENDING。函数体:\n{body}"
        );
    }
    // 「按单据定位」的必要条件必须齐：实例收窄 + 写入方常量过滤（缺一即误伤或空转）
    assert!(
        code.contains("bpm_task::Column::InstanceId.eq"),
        "{label} 必须按 instance_id 收窄任务查询（防回写波及其他单据），函数体:\n{body}"
    );
    assert!(
        code.contains("bpm_task::Column::Status.eq(task_status::PENDING)"),
        "{label} 待办过滤值必须逐字符引写入方常量 task_status::PENDING，函数体:\n{body}"
    );
    assert!(
        src.contains("use crate::models::status::bpm_task as task_status"),
        "{label} 所在文件必须引用 bpm_task 词表模块 {src_rel}"
    );
    assert!(
        src.contains("use crate::models::bpm_task;"),
        "{label} 所在文件必须引用 bpm_task 实体模块 {src_rel}"
    );
}

#[test]
fn quotation_approval_writeback_filters_with_writer_constant() {
    assert_writeback_body_locked(
        "报价审批通过回写",
        "src/services/quotation_approval_service.rs",
        "async fn handle_bpm_quotation_approval_after_commit(",
    );
    assert_writeback_body_locked(
        "报价拒绝回写",
        "src/services/quotation_approval_service.rs",
        "async fn complete_rejection_bpm(",
    );
}

#[test]
fn production_order_writeback_filters_with_writer_constant() {
    assert_writeback_body_locked(
        "生产工单审批回写",
        "src/services/production_order_ops/approval.rs",
        "async fn handle_bpm_approval_after_commit(",
    );
}

// ---------------------------------------------------------------------------
// 2) 活库行为锁（TEST_DATABASE_URL 指向已迁移 PostgreSQL；CI 串行组执行）
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
    .unwrap()
    .id
}

async fn seed_customer(db: &DatabaseConnection, owner_id: i32) -> i32 {
    let now = Utc::now();
    customer::ActiveModel {
        customer_code: Set(format!("CUS-W11-{}", unique_tag())),
        customer_name: Set("回写用例锁客户".to_string()),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set(master_data::ACTIVE.to_string()),
        customer_type: Set("retail".to_string()),
        owner_id: Set(owner_id),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
    .id
}

/// 激活的单节点审批流程定义：start → user_task(办理人=assignee_id) → end。
/// 首任务由 `BpmService::start_process` 真实创建路径生成（状态由写入方落，非手写）。
async fn seed_active_definition(db: &DatabaseConnection, code: &str, assignee_id: i32) {
    let now = Utc::now();
    let config = serde_json::json!({
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
    });
    bpm_process_definition::ActiveModel {
        name: Set(format!("{code}-回写用例锁流程")),
        code: Set(code.to_string()),
        config: Set(Some(config)),
        status: Set("ACTIVE".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

async fn seed_quotation(db: &DatabaseConnection, customer_id: i32, user_id: i32) -> i64 {
    let now = Utc::now();
    sales_quotation::ActiveModel {
        quotation_no: Set(format!("QT-W11-{}", unique_tag())),
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
    .unwrap()
    .id
}

/// 实例下的待办任务（写入方同源常量过滤）+ 大写形态负例探针，一次锁定机理：
/// 同一行真数据，小写常量命中、大写 `"PENDING"` 字面量恒查不到——
/// 「修前形态」的回写循环在此库上就是零次执行（静默空转），并非偶发。
async fn assert_case_split_mechanism(
    db: &DatabaseConnection,
    instance_id: i32,
) -> Vec<bpm_task::Model> {
    let lower = bpm_task::Entity::find()
        .filter(bpm_task::Column::InstanceId.eq(instance_id))
        .filter(bpm_task::Column::Status.eq(task_status::PENDING))
        .all(db)
        .await
        .unwrap();
    assert_eq!(
        lower.len(),
        1,
        "真实创建路径应为实例生成恰好一条状态 {} 的待办任务",
        task_status::PENDING
    );
    let upper_hits = bpm_task::Entity::find()
        .filter(bpm_task::Column::InstanceId.eq(instance_id))
        // 这里是「修前形态」的字面量本身（负例造境），不是业务代码的比较点；
        // DB CHECK 钉小写集，任何行都不可能被大写等值命中。
        .filter(bpm_task::Column::Status.eq("PENDING"))
        .all(db)
        .await
        .unwrap();
    assert!(
        upper_hits.is_empty(),
        "机理钉：大写 PENDING 过滤在本库上必须恒查不到行（否则大小写混用的空转根因不成立）"
    );
    lower
}

/// 报价拒绝链路活库锁：业务拒绝 → 本单据 BPM 待办任务真推进为 rejected，
/// 拒绝理由真落 `bpm_task.approval_opinion`，实例真终止（不再滞留 PROCESSING）。
#[tokio::test]
async fn quotation_reject_writeback_advances_bpm_task_live() {
    let db = Arc::new(test_common::setup_test_db().await);
    let initiator = seed_user(&db, "quotation_initiator").await;
    let approver = seed_user(&db, "quotation_approver").await;
    let customer_id = seed_customer(&db, initiator).await;
    seed_active_definition(&db, "quotation_approval", approver).await;

    let quotation_id = seed_quotation(&db, customer_id, initiator).await;
    let svc = QuotationApprovalService::new(db.clone());
    let submitted = svc
        .submit(quotation_id, initiator)
        .await
        .expect("中大额报价提交应成功并启动 BPM");
    assert_eq!(submitted.status, quotation_ext::PENDING_APPROVAL);
    let instance_id = submitted
        .approval_instance_id
        .expect("提交成功后必须关联 BPM 实例") as i32;

    let tasks = assert_case_split_mechanism(&*db, instance_id).await;

    let reason = "单价超出客户协议上限，拒绝";
    let rejected = svc
        .reject(quotation_id, approver, reason.to_string())
        .await
        .expect("业务侧拒绝必须成功");
    assert_eq!(rejected.status, quotation::REJECTED);
    assert_eq!(rejected.rejection_reason.as_deref(), Some(reason));

    let task = bpm_task::Entity::find_by_id(tasks[0].id)
        .one(&*db)
        .await
        .unwrap()
        .expect("任务行必须仍在");
    assert_eq!(
        task.status.as_deref(),
        Some(task_status::REJECTED),
        "业务拒绝后 BPM 任务必须真实推进（修前形态此处恒为 pending 空转）"
    );
    assert_eq!(
        task.approval_opinion.as_deref(),
        Some(reason),
        "拒绝理由必须进 bpm_task.approval_opinion（BPM 接入域理由的唯一载体）"
    );
    assert_eq!(task.actual_handler_id, Some(approver));
    assert!(task.handled_at.is_some());

    let instance = bpm_process_instance::Entity::find_by_id(instance_id)
        .one(&*db)
        .await
        .unwrap()
        .expect("实例行必须仍在");
    assert_eq!(
        instance.status.as_deref(),
        Some(instance_status::TERMINATED),
        "拒绝回写后流程实例必须终止，不再滞留 PROCESSING"
    );
}

/// 报价批准链路活库锁：业务批准 → 任务 completed、实例 completed（同一比较点，
/// 批准侧此前同样空转，批准不回写的实例即孤儿 PROCESSING）。
#[tokio::test]
async fn quotation_approve_writeback_completes_bpm_live() {
    let db = Arc::new(test_common::setup_test_db().await);
    let initiator = seed_user(&db, "quotation_initiator2").await;
    let approver = seed_user(&db, "quotation_approver2").await;
    let customer_id = seed_customer(&db, initiator).await;
    seed_active_definition(&db, "quotation_approval", approver).await;

    let quotation_id = seed_quotation(&db, customer_id, initiator).await;
    let svc = QuotationApprovalService::new(db.clone());
    let submitted = svc.submit(quotation_id, initiator).await.unwrap();
    let instance_id = submitted.approval_instance_id.unwrap() as i32;

    let tasks = assert_case_split_mechanism(&*db, instance_id).await;

    let approved = svc
        .approve(
            quotation_id,
            approver,
            "同意：符合协议区间与报价口径".to_string(),
        )
        .await
        .expect("业务侧批准必须成功");
    assert_eq!(approved.status, quotation::APPROVED);

    let task = bpm_task::Entity::find_by_id(tasks[0].id)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(task.status.as_deref(), Some(task_status::COMPLETED));

    let instance = bpm_process_instance::Entity::find_by_id(instance_id)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(instance.status.as_deref(), Some(instance_status::COMPLETED));
}

/// 生产工单拒绝链路活库锁：业务拒绝（approved=false + opinion）→ 任务真推进、
/// 理由真落 `bpm_task.approval_opinion`（工单业务表无意见列，BPM 是唯一理由载体）。
#[tokio::test]
async fn production_order_reject_writeback_advances_bpm_task_live() {
    let db = Arc::new(test_common::setup_test_db().await);
    let initiator = seed_user(&db, "prd_initiator").await;
    let approver = seed_user(&db, "prd_approver").await;
    seed_active_definition(&db, "production_order_approval", approver).await;

    let now = Utc::now();
    let order = production_order::ActiveModel {
        order_no: Set(format!("MO-W11-{}", unique_tag())),
        product_id: Set(1),
        planned_quantity: Set(dec("100.00")),
        status: Set(common::STATUS_DRAFT.to_string()),
        priority: Set(5),
        color_no: Set(String::new()),
        dye_lot_no: Set(String::new()),
        batch_no: Set(String::new()),
        order_type: Set("normal".to_string()),
        created_by: Set(initiator),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&*db)
    .await
    .unwrap();

    let svc = ProductionOrderService::new(db.clone());
    svc.submit_for_approval(order.id, initiator, "prd_initiator")
        .await
        .expect("提交审批必须成功并启动 BPM");
    let instance = BpmService::new(db.clone())
        .get_process_by_business("production_order", order.id)
        .await
        .unwrap()
        .expect("提交后必须存在流程实例");

    let tasks = assert_case_split_mechanism(&*db, instance.id).await;

    let opinion = "产能不足，无法按期交付";
    let updated = svc
        .approve_order(
            order.id,
            approver,
            "prd_approver",
            false,
            Some(opinion.to_string()),
        )
        .await
        .expect("业务侧拒绝裁决必须成功");
    assert_eq!(updated.status, production::PRODUCTION_REJECTED);

    let task = bpm_task::Entity::find_by_id(tasks[0].id)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        task.status.as_deref(),
        Some(task_status::REJECTED),
        "工单拒绝后 BPM 任务必须真实推进（修前形态此处恒为 pending 空转）"
    );
    assert_eq!(
        task.approval_opinion.as_deref(),
        Some(opinion),
        "工单意见必须进 bpm_task.approval_opinion（业务表无列，此处是唯一落点）"
    );

    let instance = bpm_process_instance::Entity::find_by_id(instance.id)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        instance.status.as_deref(),
        Some(instance_status::TERMINATED)
    );
}
