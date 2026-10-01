//! 契约波次 6 · 客户转移审批 `to_user_name` 必须透传 transfer_lead 已解析的真实姓名
//!
//! 根因（已裁定口径）：`customer_transfer_approval_service.rs` 的
//! `manager_approve`/`director_approve` 在此前写的是 `to_user_name = Set(Some(format!("用户{to_user_id}")))`
//! ——该列语义是**被转移人（第三方 to_user_id）**的展示名，`format!` 属编造展示名，
//! 违反"禁止 format! 造假名"硬规则；用审批人姓名顶替是另一种造假，同样禁止。
//! 正解（零额外查询）：`execute_transfer` 下游 `CrmAssignService::transfer_lead` 的
//! `fetch_and_validate_new_owner(to_user_id)`（assign.rs:351）已查得一次新归属人，并由
//! `build_transfer_result`（assign.rs:405-421）以 `to_user_name = new_owner.username`
//! 返回 `TransferLeadResult`；此前 `execute_transfer` 把结果丢弃。现透传回来在转移成功后
//! 回写审批行（+0 次 SELECT，审批行多 1 次 UPDATE——单行操作，非 N+1）。
//! 转移失败则 `?` 沿 `AppError` 信封显式上抛（不静默、不落空串、不造名）。
//!
//! 本文件为服务层 sqlite 真实链路锁（审批动作非"出参掩码出口"，不走 HTTP 信封断言）：
//! 1. manager_approve（普通客户）：审批行 to_user_name == 新归属人 users.username 真实值；
//! 2. director_approve（大客户）：同上；
//! 3. 同源一致性：审批行 to_user_name 与 crm_lead.owner_name、assignment_histories
//!    .to_user_name 三处落库值逐字相同（同一个 new_owner.username 单次查询来源）；
//! 4. 造名零命中棘轮：源文件不得再出现 `format!("用户{}")` 与"待用户裁定"占位。

use bingxi_backend::models::{assignment_history, crm_lead, customer_transfer_approval};
use bingxi_backend::services::crm::customer_transfer_approval_service::{
    ApproveRequest, CustomerTransferApprovalService,
};
use sea_orm::{ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter, Statement};
use std::sync::Arc;

const NEW_OWNER_LOGIN: &str = "transfer_target_wangwu";
const MANAGER: i32 = 55;
const DIRECTOR: i32 = 56;

async fn exec(db: &sea_orm::DatabaseConnection, sql: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        sql,
        Vec::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("DDL/种子 执行失败: {e}\nSQL: {sql}"));
}

const CREATE_CRM_LEAD: &str = r#"CREATE TABLE crm_lead (
    id INTEGER PRIMARY KEY,
    lead_no TEXT NOT NULL UNIQUE, lead_source TEXT NOT NULL,
    lead_status TEXT, company_name TEXT,
    contact_name TEXT NOT NULL, contact_title TEXT,
    mobile_phone TEXT, tel_phone TEXT, email TEXT, wechat TEXT, qq TEXT,
    address TEXT, product_interest TEXT,
    estimated_quantity TEXT, estimated_amount TEXT,
    expected_delivery_date TEXT, requirement_desc TEXT,
    owner_id INTEGER NOT NULL, department_id INTEGER, owner_name TEXT NOT NULL,
    last_follow_up_date TEXT, next_follow_up_date TEXT, follow_up_plan TEXT,
    converted_at TEXT, converted_customer_id INTEGER, converted_opportunity_id INTEGER,
    lost_reason TEXT, priority TEXT, rating INTEGER, tags TEXT, industry TEXT,
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
    created_by INTEGER, updated_by INTEGER, custom_fields TEXT
)"#;

const CREATE_USERS: &str = r#"CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    username TEXT NOT NULL, password_hash TEXT NOT NULL,
    real_name TEXT, avatar TEXT, email TEXT, phone TEXT,
    role_id INTEGER, department_id INTEGER, is_active INTEGER NOT NULL,
    totp_secret TEXT, is_totp_enabled INTEGER NOT NULL, totp_recovery_codes TEXT,
    last_login_at TEXT, password_changed_at TEXT, agreed_to_terms_at TEXT,
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
    gender TEXT, birth_date TEXT
)"#;

/// 与 models/customer_transfer_approval.rs::Model 逐列对应
const CREATE_TRANSFER_APPROVALS: &str = r#"CREATE TABLE customer_transfer_approvals (
    id INTEGER PRIMARY KEY,
    approval_no TEXT NOT NULL UNIQUE, lead_id INTEGER NOT NULL,
    company_name TEXT,
    from_user_id INTEGER NOT NULL, from_user_name TEXT,
    to_user_id INTEGER NOT NULL, to_user_name TEXT,
    applicant_id INTEGER NOT NULL, reason TEXT NOT NULL,
    is_large_customer INTEGER NOT NULL, approval_status TEXT NOT NULL,
    current_level INTEGER NOT NULL, max_level INTEGER NOT NULL,
    manager_approver_id INTEGER, manager_comment TEXT, manager_approved_at TEXT,
    director_approver_id INTEGER, director_comment TEXT, director_approved_at TEXT,
    completed_at TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL
)"#;

/// 与 models/assignment_history.rs::Model 逐列对应
const CREATE_ASSIGNMENT_HISTORIES: &str = r#"CREATE TABLE assignment_histories (
    id INTEGER PRIMARY KEY,
    lead_id INTEGER NOT NULL, lead_no TEXT NOT NULL, company_name TEXT,
    from_user_id INTEGER, from_user_name TEXT,
    to_user_id INTEGER, to_user_name TEXT,
    action TEXT NOT NULL, reason TEXT, notes TEXT,
    operated_by INTEGER NOT NULL, operated_by_name TEXT NOT NULL,
    created_at TEXT NOT NULL
)"#;

async fn seed_db() -> sea_orm::DatabaseConnection {
    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败");
    for ddl in [
        CREATE_CRM_LEAD,
        CREATE_USERS,
        CREATE_TRANSFER_APPROVALS,
        CREATE_ASSIGNMENT_HISTORIES,
    ] {
        exec(&db, ddl).await;
    }
    exec(
        &db,
        "INSERT INTO users (id,username,password_hash,is_active,is_totp_enabled,created_at,updated_at) VALUES
         (50,'sales_a','x',1,0,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (51,'transfer_target_wangwu','x',1,0,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (55,'manager_li','x',1,0,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (56,'director_zhao','x',1,0,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;
    // 两条未转化线索（分别服务普通/大客户审批用例），归属 OWNER
    exec(
        &db,
        "INSERT INTO crm_lead (id,lead_no,lead_source,lead_status,company_name,
         contact_name,owner_id,owner_name,department_id,created_at,updated_at) VALUES
         (1,'LD001','website','new','甲公司','张三',50,'sales_a',1,
          '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (2,'LD002','website','new','乙公司','李四',50,'sales_a',1,
          '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;
    // 两条 pending 审批行（to_user_name 建档即为 NULL，等待转移成功后回写真实姓名）
    exec(
        &db,
        "INSERT INTO customer_transfer_approvals (id,approval_no,lead_id,company_name,
         from_user_id,from_user_name,to_user_id,to_user_name,applicant_id,reason,
         is_large_customer,approval_status,current_level,max_level,created_at,updated_at) VALUES
         (1,'TA001',1,'甲公司',50,'sales_a',51,NULL,50,'客户跟进职责调整',
          0,'pending',1,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (2,'TA002',2,'乙公司',50,'sales_a',51,NULL,50,'大客户转移',
          1,'pending',2,2,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;
    db
}

/// 三处落库值逐字相同（同一个 new_owner.username 单次查询来源），且不得是造名
fn assert_real_name_everywhere(
    approval_to: Option<&str>,
    lead_owner: &str,
    history_to: Option<&str>,
    label: &str,
) {
    for (name, value) in [
        ("to_user_name（审批行）", approval_to),
        ("to_user_name（转移历史）", history_to),
    ] {
        let value =
            value.unwrap_or_else(|| panic!("{label}：{name} 必须回写真实姓名（不落空/不静默）"));
        assert_eq!(
            value, NEW_OWNER_LOGIN,
            "{label}：{name} 必须等于新归属人 users.username，实际 {value}"
        );
        assert!(
            !value.starts_with("用户"),
            "{label}：{name} 不得为 format! 造出的展示名，实际 {value}"
        );
    }
    assert_eq!(
        lead_owner, NEW_OWNER_LOGIN,
        "{label}：crm_lead.owner_name 与审批行必须同源（同一次 fetch_and_validate_new_owner）"
    );
}

#[tokio::test]
async fn manager_approve_lands_real_to_user_name_transferred_from_result() {
    let db = Arc::new(seed_db().await);
    let read_db = db.clone();
    let svc = CustomerTransferApprovalService::new(db);

    let dto = svc
        .manager_approve(
            ApproveRequest {
                approval_id: 1,
                comment: "同意转移".to_string(),
                approved: true,
            },
            MANAGER,
            "manager_li",
        )
        .await
        .expect("经理审批通过应成功");

    assert_eq!(
        dto.approval_status,
        customer_transfer_approval::STATUS_APPROVED,
        "普通客户经理通过应直接进入 approved"
    );
    assert_eq!(
        dto.to_user_name.as_deref(),
        Some(NEW_OWNER_LOGIN),
        "manager_approve DTO 必须回传透传后的真实姓名"
    );
    assert!(
        !dto.to_user_name
            .clone()
            .unwrap_or_default()
            .starts_with("用户"),
        "manager_approve 负向锁：不得出现 format! 造名"
    );

    // 回读落库（不信 DTO 自说自话）：审批行 + 线索归属 + 转移历史三处同源
    let row = customer_transfer_approval::Entity::find_by_id(1)
        .one(&*read_db)
        .await
        .expect("查询审批行失败")
        .expect("审批行必须存在");
    let lead = crm_lead::Entity::find_by_id(1)
        .one(&*read_db)
        .await
        .expect("查询线索失败")
        .expect("线索必须存在");
    let history = assignment_history::Entity::find()
        .filter(assignment_history::Column::LeadId.eq(1))
        .one(&*read_db)
        .await
        .expect("查询转移历史失败");
    assert_real_name_everywhere(
        row.to_user_name.as_deref(),
        &lead.owner_name,
        history.as_ref().and_then(|h| h.to_user_name.as_deref()),
        "manager_approve 落库回读",
    );
}

#[tokio::test]
async fn director_approve_lands_real_to_user_name_transferred_from_result() {
    let db = Arc::new(seed_db().await);
    let svc = CustomerTransferApprovalService::new(db);

    let dto = svc
        .director_approve(
            ApproveRequest {
                approval_id: 2,
                comment: "总监同意".to_string(),
                approved: true,
            },
            DIRECTOR,
            "director_zhao",
        )
        .await
        .expect("总监审批通过应成功");

    assert_eq!(
        dto.approval_status,
        customer_transfer_approval::STATUS_APPROVED
    );
    assert_eq!(
        dto.to_user_name.as_deref(),
        Some(NEW_OWNER_LOGIN),
        "总监路径同样必须透传真实姓名"
    );
    assert!(
        !dto.to_user_name
            .clone()
            .unwrap_or_default()
            .starts_with("用户"),
        "总监路径负向锁：不得出现 format! 造名"
    );
}

#[tokio::test]
async fn manager_approve_reject_does_not_touch_to_user_name() {
    // 拒绝分支不执行转移：to_user_name 保持建档 NULL（不造名、也不假装回写）
    let db = Arc::new(seed_db().await);
    let svc = CustomerTransferApprovalService::new(db);
    let dto = svc
        .manager_approve(
            ApproveRequest {
                approval_id: 1,
                comment: "不同意".to_string(),
                approved: false,
            },
            MANAGER,
            "manager_li",
        )
        .await
        .expect("拒绝应成功");
    assert_eq!(
        dto.approval_status,
        customer_transfer_approval::STATUS_REJECTED
    );
    assert_eq!(
        dto.to_user_name, None,
        "未发生转移时不得写入任何被转移人姓名"
    );
}

/// 造名零命中棘轮 + 透传正向锁（源码扫描，shrink-only）
#[test]
fn transfer_approval_must_not_fabricate_to_user_name() {
    let src = include_str!("../src/services/crm/customer_transfer_approval_service.rs");
    assert!(
        !src.contains("format!(\"用户{}\")"),
        "回潮棘轮：customer_transfer_approval_service.rs 又用 format! 伪造被转移人展示名"
    );
    assert!(
        !src.contains("待用户裁定"),
        "回潮棘轮：该点已按裁定落地，不得遗留占位标注（占位=未收口的静默）"
    );
    let occurrences = src
        .matches("Set(Some(transfer_result.to_user_name))")
        .count();
    assert_eq!(
        occurrences, 2,
        "manager_approve/director_approve 两处必须各自透传 transfer_lead 结果的真实姓名，实际命中 {occurrences}"
    );
    // 零额外查询锁：审批服务不得为取新归属人姓名再次直查 users
    assert!(
        !src.contains("user::Entity::find_by_id(to_user_id)"),
        "回潮棘轮：又出现为取被转移人姓名额外查 users（姓名应由 TransferLeadResult 透传）"
    );
    assert!(
        src.contains("-> Result<TransferLeadResult, AppError>"),
        "execute_transfer 必须回传 TransferLeadResult（真实姓名的透传载体）"
    );
}
