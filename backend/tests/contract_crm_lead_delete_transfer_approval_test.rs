//! CRM 线索删除的转移审批引用校验锁（本波修复。`delete_lead` 引用预校验腿二）
//!
//! 数据层事实（只读核对真实 DDL 与实体映射，本文件不自建 DDL）。
//! - `customer_transfer_approvals.lead_id` 是 INTEGER NOT NULL 且没有外键，
//!   列型与 `backend/src/models/customer_transfer_approval.rs` 的 `lead_id: i32` 实体映射一致。
//!   结论。DB 侧不会拦删除，漏拦只能靠服务层预校验止血。
//! - 对照的腿一 `crm_opportunity.lead_id` 有外键 `fk_crm_opportunity_lead`，
//!   已被 `tests/contract_wave1_crm_lead_delete_reference_test.rs` 锁死，本文件不重复那条文案。
//!
//! 锁定的契约（`backend/src/services/crm/lead.rs` 的 `delete_lead`）。
//! 1. 名下存在任意 `customer_transfer_approvals` 引用即拒删，含终态单
//!   （approved、rejected、cancelled 同样拒。审批历史要留痕，不能靠删线索抹掉）。
//!   错误族为 `AppError::business_displayable`
//!   （HTTP 400 + code=BUSINESS_ERROR + 真实文案外显，非脱敏族，更不是裸 500 DATABASE_ERROR）。
//! 2. 拒绝路径零漂移。线索行仍在、审批行仍在且状态未变（拒绝早于任何写操作）。
//! 3. 对照腿。无审批引用的线索仍可正常删除（证明不是无条件拒删）。
//! 4. 对照腿。别的线索有审批不影响本线索删除（证明过滤按 lead_id 收窄，不是全表计数）。
//!
//! 源码退化自证。腿一文案与本腿文案不同，若把腿二整块删掉，本文件 4 例拒删全部变红
//! （删除会成功，线索行消失，断言 is_some 与审批行数即失败）；
//! 若把腿二改成带 approval_status='pending' 过滤，终态单那一条用例变红；
//! 若把腿二改成全表计数（不带 lead_id 过滤），对照腿 4 变红；
//! 若把 business_displayable 退化成 business（脱敏族）或 DatabaseError，
//! 出参 code 与 message 两条断言同时变红。
//!
//! 活库口径。经 `test_common::setup_test_db()` 打已迁移 PostgreSQL（缺 TEST_DATABASE_URL
//! 直接 panic，不回退 sqlite）。本车道不跑 cargo，编译与活测交 CI 的分片 job 验。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::delete,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::crm_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::{crm_lead, customer_transfer_approval};
use bingxi_backend::services::crm::cust::CrmService;
use bingxi_backend::utils::error::AppError;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
};
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;

/// 拒删文案原文（与 services/crm/lead.rs delete_lead 的 business_displayable 逐字一致）
const REFUSED_MSG: &str = "该线索存在客户转移审批记录，不可删除，请先处理关联审批";

async fn live_db() -> sea_orm::DatabaseConnection {
    test_common::setup_test_db().await
}

/// 种子线索（列集与同域 contract_wave1_crm_lead_delete_reference_test.rs 同形）
async fn seed_lead(db: &sea_orm::DatabaseConnection, id: i32, owner_id: i32) {
    crm_lead::ActiveModel {
        id: Set(id),
        lead_no: Set(format!("LD-APPR-{id:03}")),
        lead_source: Set("OTHER".to_string()),
        lead_status: Set(Some(
            bingxi_backend::models::status::crm_lead::NEW.to_string(),
        )),
        company_name: Set(Some(format!("审批引用测试公司{id}"))),
        contact_name: Set("张三".to_string()),
        owner_id: Set(owner_id),
        owner_name: Set(format!("用户{owner_id}")),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

/// 种子转移审批引用行。`lead_id` 列无外键，故只需给齐 NOT NULL 列即可插入
/// （这恰恰是缺陷能成立的根因。DB 不会拦，删除后审批行就成孤儿）
async fn seed_approval(
    db: &sea_orm::DatabaseConnection,
    approval_no: &str,
    lead_id: i32,
    status: &str,
) {
    let now = chrono::Utc::now();
    customer_transfer_approval::ActiveModel {
        approval_no: Set(approval_no.to_string()),
        lead_id: Set(lead_id),
        company_name: Set(Some(format!("审批引用线索{lead_id}"))),
        from_user_id: Set(100),
        from_user_name: Set(Some("用户100".to_string())),
        to_user_id: Set(200),
        to_user_name: Set(None),
        applicant_id: Set(100),
        reason: Set("E2E 集成测试造数".to_string()),
        is_large_customer: Set(false),
        approval_status: Set(status.to_string()),
        current_level: Set(1),
        max_level: Set(1),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("种子转移审批行插入失败（列集不齐或约束不符）");
}

/// 零漂移回读。线索仍在 且 该线索名下审批行数与状态原样未动
async fn assert_untouched(db: &sea_orm::DatabaseConnection, lead_id: i32, expect_rows: u64) {
    let lead = crm_lead::Entity::find_by_id(lead_id)
        .one(db)
        .await
        .unwrap();
    assert!(lead.is_some(), "拒删路径不得删掉线索行（lead_id={lead_id}）");
    let kept = customer_transfer_approval::Entity::find()
        .filter(customer_transfer_approval::Column::LeadId.eq(lead_id))
        .count(db)
        .await
        .unwrap();
    assert_eq!(
        kept, expect_rows,
        "拒删路径不得改动审批引用行（lead_id={lead_id}）"
    );
}

// =========================================================
// service 层（真实 PostgreSQL）
// =========================================================

/// pending 审批引用 → 拒删，错误族与文案逐字锁死，线索与审批行零漂移
#[tokio::test]
async fn delete_lead_with_pending_approval_rejected_and_rows_untouched() {
    let db = live_db().await;
    seed_lead(&db, 30, 100).await;
    seed_approval(&db, "TA-E2E-PENDING-30", 30, customer_transfer_approval::STATUS_PENDING).await;

    let svc = CrmService::new(Arc::new(db.clone()));
    let err = svc
        .delete_lead(30, 100)
        .await
        .expect_err("存在转移审批引用的线索不可删除（修复前此处放行，审批行变孤儿）");

    match &err {
        AppError::BusinessErrorDisplayable(_) => {}
        other => panic!("必须用 business_displayable（文案须可外显），实际: {other:?}"),
    }
    assert_eq!(err.error_code(), "BUSINESS_ERROR");
    assert_eq!(err.to_string(), format!("业务错误：{REFUSED_MSG}"));

    assert_untouched(&db, 30, 1).await;
}

/// 终态审批（approved、rejected、cancelled）同样拒删。审批历史要留痕，
/// 不能因为单子已终结就放线索删除过去
#[tokio::test]
async fn delete_lead_with_terminal_state_approval_still_rejected() {
    let db = live_db().await;
    let cases = [
        (41, customer_transfer_approval::STATUS_APPROVED),
        (42, customer_transfer_approval::STATUS_REJECTED),
        (43, customer_transfer_approval::STATUS_CANCELLED),
    ];
    for (lead_id, status) in cases {
        seed_lead(&db, lead_id, 100).await;
        seed_approval(&db, &format!("TA-E2E-{status}-{lead_id}"), lead_id, status).await;
    }

    let svc = CrmService::new(Arc::new(db.clone()));
    for (lead_id, status) in cases {
        let err = svc
            .delete_lead(lead_id, 100)
            .await
            .expect_err(format!("终态审批 {status} 引用仍须拒删线索 {lead_id}"));
        assert!(
            matches!(err, AppError::BusinessErrorDisplayable(_)),
            "终态审批拒删必须走 displayable 族，实际: {err:?}"
        );
        assert_eq!(err.error_code(), "BUSINESS_ERROR");
        assert_eq!(err.to_string(), format!("业务错误：{REFUSED_MSG}"));
        assert_untouched(&db, lead_id, 1).await;
    }
}

/// 对照腿。无审批引用的线索仍可删除（证明不是无条件拒删）
#[tokio::test]
async fn delete_lead_without_any_approval_reference_succeeds() {
    let db = live_db().await;
    seed_lead(&db, 50, 100).await;

    let svc = CrmService::new(Arc::new(db.clone()));
    svc.delete_lead(50, 100)
        .await
        .expect("无审批引用必须删除成功，本腿不得过紧");

    assert!(
        crm_lead::Entity::find_by_id(50).one(&db).await.unwrap().is_none(),
        "无引用线索应被真实硬删"
    );
}

/// 对照腿。引用过滤按 lead_id 收窄。别的线索挂着审批不影响本线索删除
#[tokio::test]
async fn delete_lead_is_scoped_by_lead_id_not_whole_table() {
    let db = live_db().await;
    seed_lead(&db, 60, 100).await;
    seed_lead(&db, 61, 100).await;
    seed_approval(&db, "TA-E2E-PENDING-61", 61, customer_transfer_approval::STATUS_PENDING).await;

    let svc = CrmService::new(Arc::new(db.clone()));
    svc.delete_lead(60, 100)
        .await
        .expect("审批引用挂在另一条线索上，不得拦住本线索删除");

    assert!(
        crm_lead::Entity::find_by_id(60).one(&db).await.unwrap().is_none(),
        "无直接引用的线索应被真实硬删"
    );
    assert_untouched(&db, 61, 1).await;
}

// =========================================================
// HTTP handler 层（真实 PostgreSQL）
// 失败信封必须是 400 BUSINESS_ERROR 加可外显文案，不得退化成 500 或脱敏文案
// =========================================================

fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("e2e_user_{user_id}"),
        role_id: Some(2),
        department_id: Some(1),
        data_scope: Some("self".to_string()),
        dept_ids: None,
        dept_member_user_ids: None,
    }
}

async fn inject_auth(
    State(auth): State<AuthContext>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    request.extensions_mut().insert(auth);
    next.run(request).await
}

fn build_app(state: AppState, auth: AuthContext) -> Router {
    Router::new()
        .route("/crm/leads/{id}", delete(crm_handler::delete_lead))
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth))
}

async fn seeded_router(viewer: i32, with_approval: bool) -> Router {
    let db = live_db().await;
    seed_lead(&db, 70, 100).await;
    if with_approval {
        seed_approval(
            &db,
            "TA-E2E-PENDING-70",
            70,
            customer_transfer_approval::STATUS_PENDING,
        )
        .await;
    }
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    build_app(state, make_auth(viewer))
}

async fn delete_json(app: &Router, id: i32) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/crm/leads/{id}"))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

/// DELETE 带审批引用的线索 → 400 BUSINESS_ERROR 且真实文案外显（非 500、非脱敏）
#[tokio::test]
async fn http_delete_lead_with_approval_400_business_error_with_displayable_msg() {
    let app = seeded_router(100, true).await;
    let (status, v) = delete_json(&app, 70).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(v["code"], "BUSINESS_ERROR");
    assert_eq!(v["message"], REFUSED_MSG);
    assert_ne!(v["code"], "DATABASE_ERROR", "回归锁。不得回到裸 500 映射");
    assert_ne!(v["message"], "业务处理失败", "displayable 族不得被脱敏吞掉");
}

/// DELETE 无审批引用的线索 → 200（本门不得过紧把正常删除一起拦掉）
#[tokio::test]
async fn http_delete_lead_without_approval_200() {
    let app = seeded_router(100, false).await;
    let (status, v) = delete_json(&app, 70).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(v["code"], 200);
}
