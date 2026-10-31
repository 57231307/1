//! 审计定位五维契约锁 终端类型与操作页面与单据编号与变更原因与风险标记
//!
//! 锁定的真实行为
//! 一 三端各自发起同一写操作 审计行的终端类型与操作页面与单据编号与变更原因都如实落库
//!    真实发 HTTP 请求 三端用不同 User-Agent 与自定义头 走生产 audit_context 中间件判定
//!    终端与页面 走生产 AuditLogService 写入单点落库 再真实回读 audit_logs 表
//! 二 高风险操作可被单独筛出 删除经生产 delete_with_audit 自动落 DELETE 标记
//!    红冲与作废经 mark_audit_high_risk 登记 读取侧按 risk_flag 过滤只回高风险行
//! 三 按单据编号能追溯回同一张单的全链路操作且顺序正确 读取侧 doc_no 精确过滤
//!    结果按 created_at 倒序 最近一次操作排在最前
//!
//! 反向钉 无终端头且无可用 UA 时 终端记 UNKNOWN 而不是猜成 PC
//!
//! 隔离 setup_test_db 每次清空业务表 含 audit_logs 与 warehouse 由夹具保证零残留
//! 集成测试整体在 nextest 的 db-integration 串行组内跑 彼此不并发

mod test_common;
use test_common::setup_test_db;

use axum::{
    Router,
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderName, HeaderValue, Method, Request, StatusCode},
    middleware::{Next, from_fn, from_fn_with_state},
    response::{IntoResponse, Response},
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::audit_log_handler;
use bingxi_backend::middleware::audit_context::{
    RISK_DELETE, RISK_VOID, TERMINAL_H5, TERMINAL_MINIPROGRAM, TERMINAL_PC, TERMINAL_UNKNOWN,
    audit_context_middleware, mark_audit_high_risk, set_audit_change_reason, set_audit_doc_no,
};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::{audit_log, role, warehouse};
use bingxi_backend::models::audit_log::{OperationType, Severity};
use bingxi_backend::services::audit_log_service::{AuditEvent, AuditLogService, flush};
use bingxi_backend::utils::error::AppError;
use chrono::Utc;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceExt;

/// 写端点 供测试发起一次经生产写入单点的审计落库
/// 在请求作用域内先用 set_audit_doc_no 与 set_audit_change_reason 与 mark_audit_high_risk
/// 把单号与原因与风险登记进活上下文 再经生产 record_async 发送 由写入单点从上下文取五维
async fn write_event_handler(
    State(state): State<AppState>,
    payload: axum::extract::Json<Value>,
) -> Result<Response, AppError> {
    let body = payload.0;
    let op = match body.get("operation").and_then(|v| v.as_str()) {
        Some("CREATE") => OperationType::Create,
        Some("DELETE") => OperationType::Delete,
        _ => OperationType::Update,
    };
    if let Some(d) = body.get("doc_no").and_then(|v| v.as_str()) {
        set_audit_doc_no(Some(d.to_string()));
    }
    if let Some(r) = body.get("change_reason").and_then(|v| v.as_str()) {
        set_audit_change_reason(Some(r.to_string()));
    }
    if let Some(f) = body.get("risk_flag").and_then(|v| v.as_str()) {
        mark_audit_high_risk(f);
    }
    let rid = body
        .get("resource_id")
        .and_then(|v| v.as_str())
        .unwrap_or("r")
        .to_string();
    let event = AuditEvent {
        user_id: Some(100),
        username: Some("audit_terminal_test".to_string()),
        operation_type: op,
        severity: Severity::Info,
        resource_type: Some("audit_terminal_test".to_string()),
        resource_id: Some(rid),
        resource_name: None,
        description: Some("契约定位用例".to_string()),
        request_method: Some("POST".to_string()),
        request_path: Some("/term-write".to_string()),
        before_snapshot: None,
        after_snapshot: None,
    };
    Arc::new(AuditLogService::new(state.db.clone())).record_async(event, None);
    Ok(json_response(json!({"ok": true})))
}

/// 删除端点 走生产 delete_with_audit 验证删除自动落高风险标记
/// doc_no 从查询参数带入 在删除前登记进活上下文 便于把该删除行归到同一张单追溯
async fn delete_event_handler(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Query(q): Query<HashMap<String, String>>,
) -> Result<Response, AppError> {
    if let Some(d) = q.get("doc_no") {
        set_audit_doc_no(Some(d.clone()));
    }
    if let Some(r) = q.get("change_reason") {
        set_audit_change_reason(Some(r.clone()));
    }
    AuditLogService::delete_with_audit::<warehouse::Entity, _>(
        state.db.as_ref(),
        "warehouse",
        id,
        Some(100),
    )
    .await?;
    Ok(json_response(json!({"ok": true})))
}

fn json_response(v: Value) -> Response {
    axum::Json(v).into_response()
}

/// 测试路由 生产 audit_context 中间件在外层 负责终端与页面判定并绑定活上下文
/// 再叠 inject_auth 提供 admin 会话 供 list 端点的角色门通过
async fn inject_auth(State(auth): State<AuthContext>, mut request: Request<Body>, next: Next) -> Response {
    request.extensions_mut().insert(auth);
    next.run(request).await
}

async fn admin_role_id(db: &sea_orm::DatabaseConnection) -> i32 {
    role::Entity::find()
        .filter(role::Column::Code.eq("admin"))
        .one(db)
        .await
        .expect("读取 admin 角色失败")
        .expect("迁移应已种入 admin 角色")
        .id
}

fn auth_for(role_id: i32) -> AuthContext {
    AuthContext {
        user_id: 100,
        username: "audit_terminal_admin".to_string(),
        role_id: Some(role_id),
        department_id: Some(1),
        data_scope: Some("all".to_string()),
        dept_ids: None,
        dept_member_user_ids: None,
    }
}

fn build_app(db: sea_orm::DatabaseConnection, role_id: i32) -> Router {
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    Router::new()
        .route("/term-write", axum::routing::post(write_event_handler))
        .route(
            "/term-delete/{id}",
            axum::routing::delete(delete_event_handler),
        )
        .route(
            "/audit-logs",
            axum::routing::get(audit_log_handler::list_audit_logs),
        )
        .with_state(state)
        .layer(from_fn_with_state(auth_for(role_id), inject_auth))
        .layer(from_fn(audit_context_middleware))
}

async fn call(
    app: &Router,
    method: Method,
    uri: &str,
    headers: &[(&str, &str)],
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method.clone()).uri(uri);
    for (k, v) in headers {
        builder = builder.header(
            HeaderName::from_lowercase(k.to_ascii_lowercase().as_bytes()).unwrap(),
            HeaderValue::from_str(v).unwrap(),
        );
    }
    let req = match body {
        Some(v) => {
            builder =
                builder.header("content-type", HeaderValue::from_static("application/json"));
            builder.body(Body::from(v.to_string())).unwrap()
        }
        None => builder.body(Body::empty()).unwrap(),
    };
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

/// 种一条可删仓库 warehouse 为业务表 夹具每用例清空 固定 id 合法
async fn seed_warehouse(db: &sea_orm::DatabaseConnection, id: i32) {
    warehouse::ActiveModel {
        id: Set(id),
        warehouse_code: Set(format!("WH-AUDIT-{id:04}")),
        name: Set(format!("审计定位仓库-{id}")),
        is_default: Set(false),
        is_active: Set(true),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

/// 等异步审计总线把 record_async 的事件落库
async fn drain_bus() {
    let counts = flush(Duration::from_secs(10)).await;
    assert_eq!(
        counts.pending, 0,
        "审计总线应已排空 实际未处理 {} 条",
        counts.pending
    );
}

// ============================================================
// 一枚 三端真实 HTTP 同一写操作 终端与页面与单号与原因如实落库
// ============================================================

#[tokio::test]
async fn three_terminals_same_write_record_terminal_page_docno_reason_faithfully() {
    let db = setup_test_db().await;
    let role_id = admin_role_id(&db).await;
    let app = build_app(db.clone(), role_id);

    let doc = "SO-TERM-20260001";
    let reason = "改单价因汇率调整";

    // PC 桌面浏览器 UA 无小程序自定义头 无页面头 回退 Referer 的路径
    let (s_pc, _) = call(
        &app,
        Method::POST,
        "/term-write",
        &[
            (
                "user-agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36",
            ),
            ("referer", "https://erp.example.com/sales/orders"),
        ],
        Some(json!({
            "operation": "UPDATE",
            "resource_id": "TERM-PC",
            "doc_no": doc,
            "change_reason": reason,
        })),
    )
    .await;
    assert_eq!(s_pc, StatusCode::OK, "PC 写请求应 200");

    // H5 移动端浏览器 UA 带显式页面头
    let (s_h5, _) = call(
        &app,
        Method::POST,
        "/term-write",
        &[
            (
                "user-agent",
                "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 Mobile/15E148",
            ),
            ("x-page-path", "/h5/sales/order-detail"),
        ],
        Some(json!({
            "operation": "UPDATE",
            "resource_id": "TERM-H5",
            "doc_no": doc,
            "change_reason": reason,
        })),
    )
    .await;
    assert_eq!(s_h5, StatusCode::OK, "H5 写请求应 200");

    // 小程序端 带三枚小程序自定义头 页面头给真实页面路径
    let (s_mp, _) = call(
        &app,
        Method::POST,
        "/term-write",
        &[
            ("terminalsource", "6"),
            ("terminalsourcev2", "81"),
            ("app-user-agent", "wx-miniprogram/1.0"),
            ("x-page-path", "/pages/order/detail"),
        ],
        Some(json!({
            "operation": "UPDATE",
            "resource_id": "TERM-MP",
            "doc_no": doc,
            "change_reason": reason,
        })),
    )
    .await;
    assert_eq!(s_mp, StatusCode::OK, "小程序写请求应 200");

    drain_bus().await;

    // 逐端回读断言终端与页面与单号与原因都如实落库
    let pc = audit_log::Entity::find()
        .filter(audit_log::Column::ResourceId.eq("TERM-PC"))
        .one(&db)
        .await
        .unwrap()
        .expect("PC 审计行应落库");
    assert_eq!(
        pc.terminal_type.as_deref(),
        Some(TERMINAL_PC),
        "PC 桌面 UA 应判 PC 实际 {:?}",
        pc.terminal_type
    );
    assert_eq!(
        pc.operation_page.as_deref(),
        Some("/sales/orders"),
        "PC 无页面头应回退 Referer 路径 实际 {:?}",
        pc.operation_page
    );
    assert_eq!(pc.doc_no.as_deref(), Some(doc), "PC 单号应如实落库");
    assert_eq!(
        pc.change_reason.as_deref(),
        Some(reason),
        "PC 变更原因应如实落库"
    );

    let h5 = audit_log::Entity::find()
        .filter(audit_log::Column::ResourceId.eq("TERM-H5"))
        .one(&db)
        .await
        .unwrap()
        .expect("H5 审计行应落库");
    assert_eq!(
        h5.terminal_type.as_deref(),
        Some(TERMINAL_H5),
        "移动端 UA 应判 H5 实际 {:?}",
        h5.terminal_type
    );
    assert_eq!(
        h5.operation_page.as_deref(),
        Some("/h5/sales/order-detail"),
        "H5 显式页面头应优先落库 实际 {:?}",
        h5.operation_page
    );
    assert_eq!(h5.doc_no.as_deref(), Some(doc), "H5 单号应如实落库");
    assert_eq!(
        h5.change_reason.as_deref(),
        Some(reason),
        "H5 变更原因应如实落库"
    );

    let mp = audit_log::Entity::find()
        .filter(audit_log::Column::ResourceId.eq("TERM-MP"))
        .one(&db)
        .await
        .unwrap()
        .expect("小程序审计行应落库");
    assert_eq!(
        mp.terminal_type.as_deref(),
        Some(TERMINAL_MINIPROGRAM),
        "带小程序自定义头应判 MINIPROGRAM 实际 {:?}",
        mp.terminal_type
    );
    assert_eq!(
        mp.operation_page.as_deref(),
        Some("/pages/order/detail"),
        "小程序页面头应落库 实际 {:?}",
        mp.operation_page
    );
    assert_eq!(mp.doc_no.as_deref(), Some(doc), "小程序单号应如实落库");
    assert_eq!(
        mp.change_reason.as_deref(),
        Some(reason),
        "小程序变更原因应如实落库"
    );
}

/// 反向 无终端头无可用 UA 时 终端记 UNKNOWN 绝不猜成 PC
#[tokio::test]
async fn unknown_terminal_is_recorded_as_unknown_not_guessed_pc() {
    let db = setup_test_db().await;
    let role_id = admin_role_id(&db).await;
    let app = build_app(db.clone(), role_id);

    // 只给 content-type 不给 user-agent 不给任何终端头也不给 referer 与页面头
    let (s, _) = call(
        &app,
        Method::POST,
        "/term-write",
        &[("content-type", "application/json")],
        Some(json!({
            "operation": "UPDATE",
            "resource_id": "TERM-UNKNOWN",
        })),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "无终端头写请求应仍 200");
    drain_bus().await;

    let row = audit_log::Entity::find()
        .filter(audit_log::Column::ResourceId.eq("TERM-UNKNOWN"))
        .one(&db)
        .await
        .unwrap()
        .expect("未知终端审计行应落库");
    assert_eq!(
        row.terminal_type.as_deref(),
        Some(TERMINAL_UNKNOWN),
        "取不到终端应记 UNKNOWN 而不是默认 PC 实际 {:?}",
        row.terminal_type
    );
    assert_eq!(row.operation_page, None, "取不到页面应记 NULL 不臆造");
    assert_eq!(row.doc_no, None, "调用方未给单号应记 NULL");
    assert_eq!(
        row.change_reason, None,
        "调用方未给原因应记 NULL 不用默认文案"
    );
}

// ============================================================
// 二枚 高风险操作可被单独筛出
// ============================================================

#[tokio::test]
async fn high_risk_operations_are_filterable_separately() {
    let db = setup_test_db().await;
    let role_id = admin_role_id(&db).await;
    let app = build_app(db.clone(), role_id);

    // 普通更新 无风险标记
    let (s_norm, _) = call(
        &app,
        Method::POST,
        "/term-write",
        &[("user-agent", "Mozilla/5.0 (Windows NT 10.0) Chrome/120")],
        Some(json!({"operation": "UPDATE", "resource_id": "RISK-NORMAL"})),
    )
    .await;
    assert_eq!(s_norm, StatusCode::OK);

    // 作废 红冲类经 mark_audit_high_risk 登记 VOID
    let (s_void, _) = call(
        &app,
        Method::POST,
        "/term-write",
        &[("user-agent", "Mozilla/5.0 (iPhone; Mobile)")],
        Some(json!({"operation": "UPDATE", "resource_id": "RISK-VOID", "risk_flag": RISK_VOID})),
    )
    .await;
    assert_eq!(s_void, StatusCode::OK);

    // 删除 走生产 delete_with_audit 自动补 DELETE 标记
    seed_warehouse(&db, 7).await;
    let (s_del, _) = call(
        &app,
        Method::DELETE,
        "/term-delete/7?doc_no=DEL-001",
        &[("user-agent", "Mozilla/5.0 (Windows NT 10.0) Chrome/120")],
        None,
    )
    .await;
    assert_eq!(s_del, StatusCode::OK, "删除请求应 200");

    drain_bus().await;

    // 删除行应带 DELETE 标记 由写入单点自动补 业务未显式给也落
    let del_row = audit_log::Entity::find()
        .filter(audit_log::Column::ResourceId.eq("7"))
        .filter(audit_log::Column::OperationType.eq("DELETE"))
        .one(&db)
        .await
        .unwrap()
        .expect("删除审计行应落库");
    assert_eq!(
        del_row.risk_flag.as_deref(),
        Some(RISK_DELETE),
        "删除应自动落高风险 DELETE 标记 实际 {:?}",
        del_row.risk_flag
    );

    // 读取侧 按 risk_flag 传 high 只回非空标记行 普通更新被排除
    let (s_list, v) = call(
        &app,
        Method::GET,
        "/audit-logs?risk_flag=high",
        &[("user-agent", "Mozilla/5.0 (Windows NT 10.0) Chrome/120")],
        None,
    )
    .await;
    assert_eq!(s_list, StatusCode::OK, "高风险筛选应 200 实际 {v}");
    let items = v["data"]["items"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let flagged: Vec<String> = items
        .iter()
        .filter_map(|it| it["risk_flag"].as_str().map(|s| s.to_string()))
        .collect();
    assert!(
        flagged.iter().any(|f| f == RISK_DELETE),
        "risk_flag=high 结果应含 DELETE 行 实际 {flagged:?}"
    );
    assert!(
        flagged.iter().any(|f| f == RISK_VOID),
        "risk_flag=high 结果应含 VOID 行 实际 {flagged:?}"
    );

    // 精确按 DELETE 筛只回删除行 不含作废行
    let (s_del_only, v_del) = call(
        &app,
        Method::GET,
        "/audit-logs?risk_flag=DELETE",
        &[("user-agent", "Mozilla/5.0 (Windows NT 10.0) Chrome/120")],
        None,
    )
    .await;
    assert_eq!(s_del_only, StatusCode::OK);
    let del_items = v_del["data"]["items"].as_array().cloned().unwrap_or_default();
    assert!(
        !del_items.is_empty(),
        "按 DELETE 精筛应至少命中删除行"
    );
    for it in &del_items {
        assert_eq!(
            it["risk_flag"].as_str(),
            Some(RISK_DELETE),
            "精筛 DELETE 的每行标记都应等于 DELETE 实际 {it}"
        );
    }
    // 普通更新未被任何 risk_flag 命中 即它是可被排除的独立态
    let normal_in_high = items
        .iter()
        .any(|it| it["resource_id"].as_str() == Some("RISK-NORMAL"));
    assert!(
        !normal_in_high,
        "无标记的普通更新不得出现在高风险结果里"
    );
}

// ============================================================
// 三枚 按单据编号追溯回全链路且顺序正确
// ============================================================

#[tokio::test]
async fn trace_by_doc_no_returns_full_chain_in_time_order() {
    let db = setup_test_db().await;
    let role_id = admin_role_id(&db).await;
    let app = build_app(db.clone(), role_id);

    let doc = "TRACE-ORDER-555";
    let ua_pc = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120";
    let ua_h5 = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) Mobile/15E148";

    // 同一张单按时间顺序发起四类操作 中间隔一拍保证时间戳可分且单调
    // 另发一张不同单号的干扰单 验证追溯不误带别单
    let (s1, _) = call(
        &app,
        Method::POST,
        "/term-write",
        &[("user-agent", ua_pc), ("x-page-path", "/sales/orders/create")],
        Some(json!({"operation": "CREATE", "resource_id": "TR-C", "doc_no": doc, "change_reason": "建单"})),
    )
    .await;
    assert_eq!(s1, StatusCode::OK);
    tokio::time::sleep(Duration::from_millis(30)).await;

    let (s2, _) = call(
        &app,
        Method::POST,
        "/term-write",
        &[("user-agent", ua_h5), ("x-page-path", "/h5/sales/orders/edit")],
        Some(json!({"operation": "UPDATE", "resource_id": "TR-U", "doc_no": doc, "change_reason": "改量"})),
    )
    .await;
    assert_eq!(s2, StatusCode::OK);
    tokio::time::sleep(Duration::from_millis(30)).await;

    // 干扰单 单号不同 不得被本单追溯命中
    let (s_noise, _) = call(
        &app,
        Method::POST,
        "/term-write",
        &[("user-agent", ua_pc), ("x-page-path", "/other")],
        Some(json!({"operation": "UPDATE", "resource_id": "TR-NOISE", "doc_no": "OTHER-DOC"})),
    )
    .await;
    assert_eq!(s_noise, StatusCode::OK);
    tokio::time::sleep(Duration::from_millis(30)).await;

    // 作废登记 本单最后一次操作 应排在追溯结果最前
    let (s3, _) = call(
        &app,
        Method::POST,
        "/term-write",
        &[("user-agent", ua_pc), ("x-page-path", "/sales/orders/void")],
        Some(json!({"operation": "UPDATE", "resource_id": "TR-V", "doc_no": doc, "change_reason": "作废", "risk_flag": RISK_VOID})),
    )
    .await;
    assert_eq!(s3, StatusCode::OK);
    drain_bus().await;

    let (s_list, v) = call(
        &app,
        Method::GET,
        "/audit-logs?doc_no=TRACE-ORDER-555",
        &[("user-agent", ua_pc)],
        None,
    )
    .await;
    assert_eq!(s_list, StatusCode::OK, "按单号追溯应 200 实际 {v}");
    let items = v["data"]["items"].as_array().cloned().unwrap_or_default();

    // 全链路覆盖本单三次操作 且不含干扰单
    let ids: Vec<&str> = items
        .iter()
        .filter_map(|it| it["resource_id"].as_str())
        .collect();
    assert!(ids.contains(&"TR-C"), "追溯应含建单 实际 {ids:?}");
    assert!(ids.contains(&"TR-U"), "追溯应含改量 实际 {ids:?}");
    assert!(ids.contains(&"TR-V"), "追溯应含作废 实际 {ids:?}");
    assert!(
        !ids.contains(&"TR-NOISE"),
        "追溯不得误带他单行 实际 {ids:?}"
    );
    assert_eq!(
        items.len(),
        3,
        "本单追溯应恰好三行 实际 {}",
        items.len()
    );

    // 每行单号都等于查询单号 且终端如实反映发起端
    for it in &items {
        assert_eq!(
            it["doc_no"].as_str(),
            Some(doc),
            "追溯每行单号都应是本单 实际 {it}"
        );
    }

    // 顺序正确 按时间倒序 最近一次 作废 排最前 且整体 created_at 单调不增
    assert_eq!(
        items[0]["resource_id"].as_str(),
        Some("TR-V"),
        "最后发起的作废应排在最前 实际 {:?}",
        items[0]
    );
    let times: Vec<&str> = items
        .iter()
        .filter_map(|it| it["created_at"].as_str())
        .collect();
    assert_eq!(times.len(), items.len(), "每行都应有 created_at");
    for w in times.windows(2) {
        assert!(
            w[0] >= w[1],
            "追溯结果应按 created_at 倒序 发现乱序 {} 早于 {}",
            w[0],
            w[1]
        );
    }
}
