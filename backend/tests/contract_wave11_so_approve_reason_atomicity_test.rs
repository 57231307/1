//! 销售订单批准「理由与状态同一事务」真库契约锁（后端线）
//!
//! 已定案口径（本文件逐条钉死）：
//! - approve 端点的状态三件套（status/approved_by/approved_at）与选填通过理由
//!   `approval_reason` 必须在**同一事务、同一行锁下单条 UPDATE** 写入同一行——
//!   不允许存在"状态已提交、理由另起第二事务补写"的两步写形态。
//! - 原子性判据（行为级）：`AuditLogService::update_with_audit` 在同一事务内
//!   无条件插入 UPDATE 审计行（业务写成功则审计行必落、业务写回滚则审计行同
//!   回滚），因此「一次批准只产生一条 UPDATE 审计行，且该行 after 快照同时含
//!   approved 状态与审批理由」即可钉死两步写已消失：若回潮为第二事务补写理由，
//!   必然出现第二条 UPDATE 审计行（计数红），或用裸 SQL 绕审计写理由——则唯一
//!   审计行的 after 快照不含 approval_reason（快照断言红），两条防线互为兜底，
//!   无法静默绕过。
//! - 无理由门面形态（BPM 回调等走 `approve_order`）：`approval_reason` 保持
//!   NULL（未采集），不得报错、不得落空串。
//! - 业务门拦截与状态词表由既有锁覆盖（contract_wave11_order_return_approval_test
//!   等），本文件不重复钉；只钉"同一行同时可见 + 单写原子性 + 门面 NULL"三件事。
//! - 断言口径：只断 HTTP 码 + 机器 code + 列值/快照值，不断错误文案原文
//!   （本仓业务拒绝文案永久脱敏）。
//!
//! 夹具形态照 `contract_wave11_order_return_approval_test.rs`：真库
//! `setup_test_db`（已迁移 PostgreSQL、nextest 串行 db-integration 组），
//! users/customers/sales_orders/audit_logs 均属逐例 TRUNCATE 的业务表，操作人
//! 与行主键可显式固定；路由仅在测试 Router 内注册，路径形态与真实路由
//! （routes/sales.rs）逐字符一致。

mod test_common;

use bingxi_backend::container::AppState;
use bingxi_backend::handlers::sales_order_handler;
use bingxi_backend::models::status::master_data;
use bingxi_backend::models::status::sales_order as so_status;
use bingxi_backend::models::{audit_log, customer, sales_order, user};
use bingxi_backend::search::{ElasticClient, SearchClient};
use bingxi_backend::services::event_notification_service::EventNotificationService;
use bingxi_backend::services::so::order::SalesService;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde_json::{Value, json};
use std::sync::Arc;
use test_common::setup_test_db;

/// 注入到 AuthContext 并落 approved_by/审计的操作人主键（users 逐例被夹具清空）
const OPERATOR_ID: i32 = 9741;

fn now() -> DateTime<Utc> {
    Utc::now()
}

async fn seed_operator(db: &Arc<DatabaseConnection>) {
    user::ActiveModel {
        id: Set(OPERATOR_ID),
        username: Set("w11_so_approve_atomicity".to_string()),
        password_hash: Set("test-only-not-a-real-hash".to_string()),
        real_name: Set(Some("SO批准并单事务契约锁操作人".to_string())),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子操作人插入失败: {e}"));
}

async fn seed_customer(db: &Arc<DatabaseConnection>, tag: &str) -> i32 {
    let m = customer::ActiveModel {
        customer_code: Set(format!("CUS-W11ATA-{tag}")),
        customer_name: Set("SO批准并单事务契约锁客户".to_string()),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set(master_data::ACTIVE.to_string()),
        customer_type: Set("retail".to_string()),
        owner_id: Set(OPERATOR_ID),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子客户 {tag} 插入失败: {e}"));
    m.id
}

/// 直落一行待审（pending）销售订单——批准动作的唯一前置状态
async fn seed_pending_so(db: &Arc<DatabaseConnection>, tag: &str, customer_id: i32) -> i32 {
    let order = sales_order::ActiveModel {
        order_no: Set(format!("SO-W11ATA-{tag}")),
        customer_id: Set(customer_id),
        order_date: Set(now()),
        status: Set(so_status::PENDING.to_string()),
        subtotal: Set(Decimal::ZERO),
        tax_amount: Set(Decimal::ZERO),
        discount_amount: Set(Decimal::ZERO),
        shipping_cost: Set(Decimal::ZERO),
        total_amount: Set(Decimal::ZERO),
        paid_amount: Set(Decimal::ZERO),
        balance_amount: Set(Decimal::ZERO),
        created_by: Set(Some(OPERATOR_ID)),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子销售订单 {tag} 插入失败: {e}"));
    order.id
}

/// SO 行权威回读（列值断言一律直读 DB）
async fn read_so(db: &Arc<DatabaseConnection>, id: i32) -> sales_order::Model {
    sales_order::Entity::find_by_id(id)
        .one(db.as_ref())
        .await
        .unwrap_or_else(|e| panic!("销售订单 {id} 直读失败: {e}"))
        .unwrap_or_else(|| panic!("销售订单 {id} 必须存在"))
}

/// 该订单的全部 UPDATE 审计行（approve 服务侧经 update_with_audit 落
/// resource_type="auto_audit"；resource_id 为订单主键字符串）
async fn update_audit_rows(db: &Arc<DatabaseConnection>, order_id: i32) -> Vec<audit_log::Model> {
    audit_log::Entity::find()
        .filter(audit_log::Column::Action.eq("UPDATE"))
        .filter(audit_log::Column::ResourceType.eq("auto_audit"))
        .filter(audit_log::Column::ResourceId.eq(order_id.to_string()))
        .all(db.as_ref())
        .await
        .unwrap_or_else(|e| panic!("订单 {order_id} 审计行回读失败: {e}"))
}

fn snapshot_of(log: &audit_log::Model) -> Value {
    log.after_snapshot
        .as_ref()
        .map(|v| v.0.clone())
        .unwrap_or_else(|| {
            panic!("UPDATE 审计行必须携带 after_snapshot（update_with_audit 无条件写入）")
        })
}

fn before_snapshot_of(log: &audit_log::Model) -> Value {
    log.before_snapshot
        .as_ref()
        .map(|v| v.0.clone())
        .unwrap_or_else(|| {
            panic!("UPDATE 审计行必须携带 before_snapshot（update_with_audit 无条件写入）")
        })
}

/// HTTP 形态夹具：approve 端点；auth 注入与 AppState 照 wave11 审批锁先例
async fn approval_app() -> (Arc<DatabaseConnection>, axum::Router) {
    let db = Arc::new(setup_test_db().await);
    seed_operator(&db).await;
    let mut state = AppState {
        db: db.clone(),
        ..Default::default()
    };
    // `AppState::default()` 的全部子服务绑在 `DatabaseConnection::default()`
    // （= sea-orm 的 `Disconnected` 变体，`src/container/mod.rs:334`）上，只覆盖 `db`
    // 字段不会把它们搬到真库。本锁的用例 1/2 经真实 handler 成功批准 SO，落库后必走
    // 站内通知（`src/handlers/sales_order_handler.rs:433` → `notify_order_approved`
    // `:437`），通知服务对 `Disconnected` 连接取 backend 直接 panic
    // （sea-orm-2.0.2 `src/database/db_connection.rs:727`）。生产装配无条件构造该服务
    // 并与 `state.db` 同池（`src/container/mod.rs:343`），故此处按生产口径把它重建到
    // 真库上——禁止用 `None` 绕过真实链路（那会把已装配的通知通道变成不可达分支）。
    state.event_notification_service = Some(Arc::new(EventNotificationService::new(db.clone())));
    async fn inject_auth(
        auth: axum::extract::State<bingxi_backend::middleware::auth_context::AuthContext>,
        mut request: axum::http::Request<axum::body::Body>,
        next: axum::middleware::Next,
    ) -> axum::response::Response {
        request.extensions_mut().insert(auth.0);
        next.run(request).await
    }
    let auth = bingxi_backend::middleware::auth_context::AuthContext {
        user_id: OPERATOR_ID,
        username: "w11_so_approve_atomicity".to_string(),
        role_id: None,
        department_id: None,
        data_scope: Some("all".to_string()),
        dept_ids: None,
        dept_member_user_ids: None,
    };
    let app = axum::Router::new()
        .route(
            "/sales/orders/{id}/approve",
            axum::routing::post(sales_order_handler::approve_order),
        )
        .with_state(state)
        .layer(axum::middleware::from_fn_with_state(auth, inject_auth));
    (db, app)
}

async fn post_json(app: &axum::Router, uri: &str, body: Value) -> (axum::http::StatusCode, Value) {
    use tower::ServiceExt;
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method(axum::http::Method::POST)
                .uri(uri)
                .header(axum::http::header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8_lossy(&bytes).to_string();
    (
        status,
        serde_json::from_str(&text).unwrap_or(Value::String(text)),
    )
}

// ---------------------------------------------------------------------------
// 1. 带理由批准成功后，同一行里状态与 approval_reason 同时可见（双证）：
//    端点出参 data 与真库直读都必须同时读到 approved 态与 trim 后的理由原文。
//    改坏什么必红：理由退回"另事务补写"且补写失败 ⇒ 端点/直读理由断言红；
//    理由写错列或只进日志 ⇒ 直读 NULL 红；批准不落状态 ⇒ 词表常量断言红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn approve_with_reason_lands_status_and_reason_on_same_row() {
    let (db, app) = approval_app().await;
    let customer_id = seed_customer(&db, "T1").await;
    let order_id = seed_pending_so(&db, "T1", customer_id).await;
    let raw = "  客户信用额度与交期均已复核通过，同意执行  ";
    let expected = raw.trim();

    let (status, v) = post_json(
        &app,
        &format!("/sales/orders/{order_id}/approve"),
        json!({ "approval_reason": raw }),
    )
    .await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "带理由批准权威路径必须 200，实得 {v}"
    );
    assert_eq!(v["code"], 200, "成功信封 code 必须 200，实得 {v}");
    assert_eq!(
        v["data"]["status"],
        so_status::APPROVED,
        "批准端点出参必须同时刻可见 approved 态"
    );
    assert_eq!(
        v["data"]["approval_reason"].as_str(),
        Some(expected),
        "批准端点出参必须同时刻可见逐字理由（trim 后原样）"
    );

    let row = read_so(&db, order_id).await;
    assert_eq!(
        row.status,
        so_status::APPROVED,
        "真库直读：批准必须落 approved 态（值与写入方词表常量逐字符同源）"
    );
    assert_eq!(
        row.status, "approved",
        "approved 落值逐字钉死（词表漂移即此断言红）"
    );
    assert_eq!(
        row.approval_reason.as_deref(),
        Some(expected),
        "真库直读：approval_reason 必须与状态同一行同时可见"
    );
    assert_eq!(
        row.approved_by,
        Some(OPERATOR_ID),
        "批准必须记录审批人（与理由同事务写入）"
    );
    assert!(
        row.approved_at.is_some(),
        "批准必须记录审批时间（与理由同事务写入）"
    );
    assert_eq!(
        row.rejected_reason, None,
        "两动作两列：批准不得写 rejected_reason 列"
    );
}

// ---------------------------------------------------------------------------
// 2. 原子性（行为级单写证明，最关键）：一次带理由批准对该订单只产生**一条**
//    UPDATE 审计行，且该行的 after 快照同时携带 approved 状态与理由、before
//    快照仍为 pending——即"状态+理由"确经同一条 UPDATE、同一次 commit 落库。
//    判据与不可静默绕过原因：update_with_audit 在同一事务内无条件插审计行，
//    业务写与审计写同生共死。回潮两步写（第二事务补理由）⇒ 出现第二条 UPDATE
//    审计行，计数红；改用裸 SQL 绕审计补写 ⇒ 计数仍为 1 但唯一审计行的 after
//    快照缺 approval_reason，快照断言红（且该形态本身违反"业务更新必须走
//    update_with_audit"的全仓红线）。本例配合用例 1 的直读，把"半程态
//   （已批准但理由缺失）"在结构与行为两个层面同时钉死。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn single_approve_with_reason_produces_exactly_one_update_write() {
    let (db, app) = approval_app().await;
    let customer_id = seed_customer(&db, "T2").await;
    let order_id = seed_pending_so(&db, "T2", customer_id).await;
    let reason = "单价与账期均在协议区间内，同意批准";

    let (status, v) = post_json(
        &app,
        &format!("/sales/orders/{order_id}/approve"),
        json!({ "approval_reason": reason }),
    )
    .await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "前置：带理由批准必须成功，实得 {v}"
    );

    let rows = update_audit_rows(&db, order_id).await;
    assert_eq!(
        rows.len(),
        1,
        "一次批准对该订单必须只留下恰好一条 UPDATE 审计行（两条即两步写回潮），实得 {} 条",
        rows.len()
    );
    let after = snapshot_of(&rows[0]);
    assert_eq!(
        after["status"],
        so_status::APPROVED,
        "唯一 UPDATE 审计行的 after 快照必须携带 approved 状态"
    );
    assert_eq!(
        after["approval_reason"].as_str(),
        Some(reason),
        "唯一 UPDATE 审计行的 after 快照必须与状态同行携带理由（缺失即状态/理由分居两次写入）"
    );
    let before = before_snapshot_of(&rows[0]);
    assert_eq!(
        before["status"],
        so_status::PENDING,
        "before 快照必须仍是 pending（门与写同事务、批准前状态未被提前裸写）"
    );
    assert_eq!(
        before["approval_reason"],
        Value::Null,
        "before 快照理由必须为 NULL（种子行未采集理由，批准前不得有旁路写入）"
    );
}

// ---------------------------------------------------------------------------
// 3. 无理由门面形态（approve_order，BPM 回调/存量调用方签名）：委托
//    approve_order_with_reason(None) 成功落 approved 态，approval_reason 保持
//    NULL 且不得因 NULL 报错；同样只产生一条 UPDATE 审计行，after 快照理由为
//    NULL（None 分支不 Set，不得伪造成空串）。
//    改坏什么必红：门面签名漂移（击穿 listener.rs BPM 回写与单测矩阵）⇒ 编译
//    红；门面把 None 落成空串 ⇒ NULL 断言红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn approve_facade_without_reason_keeps_approval_reason_null() {
    let db = Arc::new(setup_test_db().await);
    seed_operator(&db).await;
    let customer_id = seed_customer(&db, "T3").await;
    let order_id = seed_pending_so(&db, "T3", customer_id).await;

    let search_client: Arc<dyn SearchClient> = Arc::new(ElasticClient::mock());
    let service = SalesService::new(db.clone(), search_client);
    let order = service
        .approve_order(order_id, OPERATOR_ID)
        .await
        .unwrap_or_else(|e| panic!("门面（无理由形态）批准不得失败：{e}"));
    assert_eq!(
        order.status,
        so_status::APPROVED,
        "门面批准必须落 approved 态"
    );

    let row = read_so(&db, order_id).await;
    assert_eq!(
        row.status,
        so_status::APPROVED,
        "真库直读：门面批准落 approved"
    );
    assert_eq!(
        row.approval_reason, None,
        "无理由形态 approval_reason 必须保持 NULL（未采集），不得落空串"
    );

    let rows = update_audit_rows(&db, order_id).await;
    assert_eq!(
        rows.len(),
        1,
        "门面批准同样必须只有一条 UPDATE 审计行，实得 {} 条",
        rows.len()
    );
    let after = snapshot_of(&rows[0]);
    assert_eq!(
        after["status"],
        so_status::APPROVED,
        "门面批准唯一审计行 after 快照必须携带 approved"
    );
    assert_eq!(
        after["approval_reason"],
        Value::Null,
        "门面批准 after 快照理由必须为 NULL（None 不 Set 列）"
    );
}
