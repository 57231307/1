//! 后端安全批「审计身份只认服务端会话」行为级活体证明（对应测试锁用例）
//!
//! 锁定的契约（每一域均为真库真 handler 行为证明，不是源码文本比对）：
//! - `handlers/chemical_handler.rs:458-478` 领用单 approve/issue：handler **无 body 提取器**，
//!   审批人/发料人取 `auth.user_id`（`services/chemical_ops/requisition.rs:203-235` 签名必填 i32）；
//! - `handlers/dye_batch_state_machine_handler.rs:314-321` 回修单 approve：同上；
//! - `handlers/production_recipe_handler.rs:158-168` 大货处方 approve、`:272-282` 加料处方 approve；
//!   `handlers/dye_recipe_handler.rs:125-135` 染色配方 approve：同上；
//! - `handlers/ai_model_management_handler.rs:40-101` create/approve/change_status：
//!   DTO 仍带业务字段（approval_status/new_status 等），但 `approved_by`/`changed_by`
//!   由 handler 传 `auth.user_id`，请求体伪造的同名键被 serde 直接忽略；
//! - `handlers/custom_order_handler.rs:407-439` advance：DTO 只剩 `notes`
//!   （`OptionalJson<AdvanceRequest>`），推进产生的流程日志 `process_logs.operator_id`
//!   与下一节点 `process_nodes.operator_id` 由会话派生（该两列在库里有真外键 →users，
//!   故本域夹具自种 users/customers/products 父行，口径同 contract_wave5_custom_order_status_unity）。
//!
//! 证明形态（每条锁缺一不可）：
//! ① 会话注入用户 A（`from_fn_with_state(make_auth(A), inject_auth)`，与真实权限中间件
//!   同走 extensions 注入）；② 请求体**故意携带另一用户 B** 的身份键
//!   （approved_by/operator_id/issued_by/changed_by/user_id/approver_id 全量伪造，
//!   模拟攻击者或旧客户端）；③ 动作必须成功（证明该身份键不再是必填、缺它合法）；
//! ④ **直读实体行**（`Entity::find_by_id(...).one(db)`，不看 HTTP 出参）断言审计列 == A
//!   且显式断言 != B；⑤ 「带 JSON 头空体 / 无 Content-Type 空体」两种缺体形态在领用单
//!   approve 站点上必须仍命中服务层真实状态门（400 `BUSINESS_ERROR`，非解码层
//!   `VALIDATION_ERROR`），且被拒绝后行审计列不得被半途写入——形态判据复用
//!   contract_wave11_optional_json_body 的 ReqShape 双断口径，不另造第二套判定器。
//!
//! 检测力（改坏源码即红的点位，逐条对应）：
//! - 若 chemical/dye_batch_rework/production_recipe/dye_recipe handler 退回
//!   `Json<ApproveReq>` 并从 body 取 `approved_by`/`operator_id` → 本文件直读断言
//!   `row.approved_by == Some(SESSION_A)` 当场红（落库值变为 Some(FORGED_B) 或 NULL）；
//! - 若 service 层退回 `approved_by: Option<i32>` + `unwrap_or_default`/`req.approved_by`
//!   兜底 → `== Some(SESSION_A)` 断言红（NULL 或 B）；
//! - 若 custom_order handler 退回从 AdvanceRequest 取操作人 → `log.operator_id`/
//!   `next_node.operator_id` 两条断言红；
//! - 若 approve 端点重新绑回必填 body 提取器 → ②③ 的缺体双断（400 判据）与
//!   伪造体 200 判据至少一条红；
//! - 若身份键改成 DTO 必填字段（旧契约），②③ 中「仅带伪造键」的请求会 400，
//!   `StatusCode::OK` 断言红。
//!
//! 夹具形态：真 PostgreSQL `setup_test_db`（已迁移库 + TRUNCATE 业务表，roles 等种子
//! 参照表不清空）；tower `oneshot` 打真实 handler；同测试二进制由 CI 以
//! `--test-threads=1` 串行执行，逐用例 setup 不互踩。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::post,
};
use bingxi_backend::constants::customer_type;
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::{
    ai_model_management_handler, chemical_handler, custom_order_handler,
    dye_batch_state_machine_handler, dye_recipe_handler, production_recipe_handler,
};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::status::{
    chemical_requisition_status, custom_order as co_status, dye_batch_rework_status,
    dye_recipe as dye_recipe_status, master_data, process_node as node_status,
    production_recipe as pr_status, production_recipe_addition as addition_status,
};
use bingxi_backend::models::{
    ai_model_version, chemical_requisition, custom_order, customer, dye_batch_rework, dye_recipe,
    process_log, process_node, product, production_recipe, production_recipe_addition, user,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};
use serde_json::{Value, json};
use test_common::setup_test_db;
use tower::ServiceExt;

/// 会话用户 A：AuthContext.user_id 注入的合法操作人（私有 id 段 951x，
/// custom_order 域会把它插成真实 users 行以满足 operator 外键）
const SESSION_A: i32 = 9511;
/// 伪造用户 B：只出现在请求体里，永远不允许落进任何审计列（私有 id 段 947x）
const FORGED_B: i32 = 9472;

fn now() -> DateTime<Utc> {
    Utc::now()
}

fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("w11_identity_{user_id}"),
        role_id: Some(2),
        department_id: Some(1),
        data_scope: Some("all".to_string()),
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

/// 攻击者/旧客户端在 body 里全量伪造身份键（handler 不认的键 serde 直接忽略，
/// 照发不误——这正是本锁要证明的「发了也无效」）
fn forged_body(extra: Option<Value>) -> String {
    let mut map = serde_json::Map::new();
    for key in [
        "approved_by",
        "operator_id",
        "issued_by",
        "changed_by",
        "user_id",
        "approver_id",
    ] {
        map.insert(key.to_string(), Value::from(FORGED_B));
    }
    if let Some(obj) = extra.as_ref().and_then(Value::as_object) {
        for (k, v) in obj {
            map.insert(k.clone(), v.clone());
        }
    }
    serde_json::to_string(&Value::Object(map)).expect("伪造身份体必须是合法 JSON（夹具自证）")
}

/// 请求形态（口径照 contract_wave11_optional_json_body 的 ReqShape 双断，不另造判定器）
enum ReqShape<'a> {
    /// Content-Type: application/json + 完全无体
    JsonCtEmpty,
    /// 无 Content-Type 且无体
    NoCtEmpty,
    /// Content-Type: application/json + JSON 文本体
    Json(&'a str),
}

async fn call(app: &Router, path: &str, shape: ReqShape<'_>) -> (StatusCode, Value) {
    let builder = Request::builder().method(Method::POST).uri(path);
    let req = match shape {
        ReqShape::JsonCtEmpty => builder
            .header("content-type", "application/json")
            .body(Body::empty())
            .unwrap(),
        ReqShape::NoCtEmpty => builder.body(Body::empty()).unwrap(),
        ReqShape::Json(text) => builder
            .header("content-type", "application/json")
            .body(Body::from(text.to_string()))
            .unwrap(),
    };
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// 以 SESSION_A 为会话用户组装带 auth 注入层的 Router（路由按占位符注册，同真实 routes 形态）
fn layered_app(
    state: AppState,
    routes: Vec<(&'static str, axum::routing::MethodRouter<AppState>)>,
) -> Router {
    let mut router: Router<AppState> = Router::new();
    for (path, method_route) in routes {
        router = router.route(path, method_route);
    }
    router
        .with_state(state)
        .layer(from_fn_with_state(make_auth(SESSION_A), inject_auth))
}

// =========================================================
// 域 1：染化料领用单（chemical_handler approve / issue）
// =========================================================

async fn seed_requisition(
    db: &DatabaseConnection,
    tag: &str,
    status: &str,
) -> chemical_requisition::Model {
    chemical_requisition::ActiveModel {
        requisition_no: Set(format!("CR-W11ID-{tag}")),
        requisition_type: Set("production".to_string()),
        requisition_date: Set(Utc::now().date_naive()),
        status: Set(status.to_string()),
        total_amount: Set(Decimal::ZERO),
        is_deleted: Set(false),
        created_at: Set(now().into()),
        updated_at: Set(now().into()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子领用单 {tag} 失败: {e}"))
}

async fn read_requisition(db: &DatabaseConnection, id: i32) -> chemical_requisition::Model {
    chemical_requisition::Entity::find_by_id(id)
        .one(db)
        .await
        .unwrap_or_else(|e| panic!("领用单 {id} 直读失败: {e}"))
        .unwrap_or_else(|| panic!("领用单 {id} 必须存在"))
}

#[tokio::test]
async fn chemical_requisition_approve_stamps_session_user_not_body_identity() {
    let db = setup_test_db().await;
    let seeded = seed_requisition(&db, "APPR", chemical_requisition_status::DRAFT).await;
    assert_eq!(
        seeded.approved_by, None,
        "前提自检：种子 draft 行的审批人必须为空"
    );
    let read_db = db.clone();
    let state = AppState {
        db: std::sync::Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        vec![(
            "/chemical-requisitions/{id}/approve",
            post(chemical_handler::approve_requisition),
        )],
    );
    let body = forged_body(None);
    let (status, v) = call(
        &app,
        &format!("/chemical-requisitions/{}/approve", seeded.id),
        ReqShape::Json(&body),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "伪造身份键不得使请求失败（该键非必填，缺它合法）, 实际: {status} {v}"
    );
    assert_eq!(v["code"], 200, "成功信封 code 必须为 200, 实际: {v}");

    let row = read_requisition(&read_db, seeded.id).await;
    assert_eq!(
        row.status,
        chemical_requisition_status::APPROVED,
        "审批动作必须真实推进状态机（而不是静默放过）"
    );
    assert_eq!(
        row.approved_by,
        Some(SESSION_A),
        "直读实体：审批人必须等于会话用户 A，实际 {:?}",
        row.approved_by
    );
    assert_ne!(
        row.approved_by,
        Some(FORGED_B),
        "直读实体：body 伪造的 B 绝不许落进审批人列"
    );
}

#[tokio::test]
async fn chemical_requisition_issue_stamps_session_issuer_not_body_identity() {
    let db = setup_test_db().await;
    let seeded = seed_requisition(&db, "ISSUE", chemical_requisition_status::APPROVED).await;
    let read_db = db.clone();
    let state = AppState {
        db: std::sync::Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        vec![(
            "/chemical-requisitions/{id}/issue",
            post(chemical_handler::issue_requisition),
        )],
    );
    let body = forged_body(None);
    let (status, v) = call(
        &app,
        &format!("/chemical-requisitions/{}/issue", seeded.id),
        ReqShape::Json(&body),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "发料必须成功, 实际: {status} {v}");

    let row = read_requisition(&read_db, seeded.id).await;
    assert_eq!(row.status, chemical_requisition_status::ISSUED);
    assert_eq!(
        row.issued_by,
        Some(SESSION_A),
        "直读实体：发料人必须等于会话用户 A，实际 {:?}",
        row.issued_by
    );
    assert_ne!(
        row.issued_by,
        Some(FORGED_B),
        "直读实体：body 伪造的 B 绝不许落进发料人列"
    );
}

#[tokio::test]
async fn chemical_requisition_approve_absent_body_forms_still_hit_real_state_gate() {
    // 缺体双断（ReqShape 口径同 contract_wave11_optional_json_body）：
    // 已审批行重复 approve，「带 JSON 头空体」与「无头空体」都必须命中
    // services/chemical_ops/requisition.rs:203-210 的「仅 draft 可审批」状态门
    // （400 BUSINESS_ERROR），且被拒后行审计列不得被半途写入。
    let db = setup_test_db().await;
    let seeded = seed_requisition(&db, "GATE", chemical_requisition_status::APPROVED).await;
    let read_db = db.clone();
    let state = AppState {
        db: std::sync::Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        vec![(
            "/chemical-requisitions/{id}/approve",
            post(chemical_handler::approve_requisition),
        )],
    );
    let path = format!("/chemical-requisitions/{}/approve", seeded.id);
    for shape in [ReqShape::JsonCtEmpty, ReqShape::NoCtEmpty] {
        let desc = match shape {
            ReqShape::JsonCtEmpty => "带 JSON 头空体",
            ReqShape::NoCtEmpty => "无 Content-Type 空体",
            ReqShape::Json(_) => unreachable!("本用例只打两种缺体形态"),
        };
        let (status, v) = call(&app, &path, shape).await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "{desc} 必须进真实状态门（已审批行重复审批 → 400），不得停在解码层, 实际: {status} {v}"
        );
        assert_eq!(
            v["code"], "BUSINESS_ERROR",
            "{desc} 命中的必须是服务层业务门而非 VALIDATION_ERROR, 实际: {v}"
        );
    }
    let row = read_requisition(&read_db, seeded.id).await;
    assert_eq!(
        row.status,
        chemical_requisition_status::APPROVED,
        "状态门拒绝后行状态必须保持原值"
    );
    assert_eq!(
        row.approved_by, None,
        "被拒绝的 approve 绝不允许半途写入审批人（无痕断言），实际 {:?}",
        row.approved_by
    );
}

// =========================================================
// 域 2：缸号回修单（dye_batch_state_machine_handler approve）
// =========================================================

#[tokio::test]
async fn dye_batch_rework_approve_stamps_session_approver_not_body_identity() {
    let db = setup_test_db().await;
    let seeded = dye_batch_rework::ActiveModel {
        original_batch_id: Set(1),
        original_batch_no: Set("BATCH-W11ID-REWORK".to_string()),
        rework_type: Set("color_difference".to_string()),
        rework_reason: Set("色差超差需回修".to_string()),
        original_status: Set("inspecting".to_string()),
        status: Set(dye_batch_rework_status::DRAFT.to_string()),
        is_deleted: Set(false),
        created_at: Set(now().into()),
        updated_at: Set(now().into()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("种子回修单失败");
    let read_db = db.clone();
    let state = AppState {
        db: std::sync::Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        vec![(
            "/dye-batch-reworks/{id}/approve",
            post(dye_batch_state_machine_handler::approve_rework),
        )],
    );
    let body = forged_body(None);
    let (status, v) = call(
        &app,
        &format!("/dye-batch-reworks/{}/approve", seeded.id),
        ReqShape::Json(&body),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "回修审批必须成功, 实际: {status} {v}"
    );

    let row = dye_batch_rework::Entity::find_by_id(seeded.id)
        .one(&read_db)
        .await
        .expect("回修单直读失败")
        .expect("回修单必须存在");
    assert_eq!(
        row.status,
        dye_batch_rework_status::APPROVED,
        "审批动作必须真实推进状态机"
    );
    assert_eq!(
        row.approved_by,
        Some(SESSION_A),
        "直读实体：回修审批人必须等于会话用户 A，实际 {:?}",
        row.approved_by
    );
    assert_ne!(
        row.approved_by,
        Some(FORGED_B),
        "直读实体：body 伪造的 B 绝不许落进回修审批人列"
    );
    assert!(
        row.approved_at.is_some(),
        "审批时间必须与审批人一同写入（无痕断言）"
    );
}

// =========================================================
// 域 3：大货处方 + 加料处方（production_recipe_handler approve / approve_addition）
// =========================================================

#[tokio::test]
async fn production_recipe_approve_stamps_session_approver_not_body_identity() {
    let db = setup_test_db().await;
    let seeded = production_recipe::ActiveModel {
        recipe_no: Set("PR-W11ID-001".to_string()),
        fabric_weight: Set(Decimal::from(100_i64)),
        liquor_ratio: Set("1:8".to_string()),
        recipe_detail: Set(Some(vec![production_recipe::RecipeMaterialItem {
            material_code: "DYE-001".to_string(),
            material_name: "活性染料红".to_string(),
            concentration: Some(Decimal::from(2_i64)),
            unit: "kg".to_string(),
            amount: Decimal::from(2_i64),
            category: "dye".to_string(),
        }])),
        status: Set(pr_status::DRAFT.to_string()),
        is_deleted: Set(false),
        created_at: Set(now().into()),
        updated_at: Set(now().into()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("种子大货处方失败");
    let read_db = db.clone();
    let state = AppState {
        db: std::sync::Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        vec![(
            "/production-recipes/{id}/approve",
            post(production_recipe_handler::approve),
        )],
    );
    let body = forged_body(None);
    let (status, v) = call(
        &app,
        &format!("/production-recipes/{}/approve", seeded.id),
        ReqShape::Json(&body),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "大货处方审批必须成功, 实际: {status} {v}"
    );

    let row = production_recipe::Entity::find_by_id(seeded.id)
        .one(&read_db)
        .await
        .expect("大货处方直读失败")
        .expect("大货处方必须存在");
    assert_eq!(row.status, pr_status::APPROVED, "状态机必须真实推进");
    assert_eq!(
        row.approved_by,
        Some(SESSION_A),
        "直读实体：大货处方审核人必须等于会话用户 A，实际 {:?}",
        row.approved_by
    );
    assert_ne!(
        row.approved_by,
        Some(FORGED_B),
        "直读实体：body 伪造的 B 绝不许落进审核人列"
    );
}

#[tokio::test]
async fn production_recipe_addition_approve_stamps_session_approver_not_body_identity() {
    let db = setup_test_db().await;
    let seeded = production_recipe_addition::ActiveModel {
        addition_no: Set("PA-W11ID-001".to_string()),
        production_recipe_id: Set(1),
        addition_reason: Set(Some("助剂不足".to_string())),
        addition_detail: Set(Some(vec![
            production_recipe_addition::AdditionMaterialItem {
                material_code: "AUX-001".to_string(),
                material_name: "匀染剂".to_string(),
                amount: Decimal::from(1_i64),
                unit: "kg".to_string(),
                category: "auxiliary".to_string(),
            },
        ])),
        status: Set(addition_status::DRAFT.to_string()),
        is_deleted: Set(false),
        created_at: Set(now().into()),
        updated_at: Set(now().into()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("种子加料处方失败");
    let read_db = db.clone();
    let state = AppState {
        db: std::sync::Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        vec![(
            "/production-recipes/additions/{id}/approve",
            post(production_recipe_handler::approve_addition),
        )],
    );
    let body = forged_body(None);
    let (status, v) = call(
        &app,
        &format!("/production-recipes/additions/{}/approve", seeded.id),
        ReqShape::Json(&body),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "加料处方审批必须成功, 实际: {status} {v}"
    );

    let row = production_recipe_addition::Entity::find_by_id(seeded.id)
        .one(&read_db)
        .await
        .expect("加料处方直读失败")
        .expect("加料处方必须存在");
    assert_eq!(row.status, addition_status::APPROVED, "状态机必须真实推进");
    assert_eq!(
        row.approved_by,
        Some(SESSION_A),
        "直读实体：加料处方审核人必须等于会话用户 A，实际 {:?}",
        row.approved_by
    );
    assert_ne!(
        row.approved_by,
        Some(FORGED_B),
        "直读实体：body 伪造的 B 绝不许落进加料审核人列"
    );
}

// =========================================================
// 域 4：染色配方（dye_recipe_handler approve_recipe）
// =========================================================

#[tokio::test]
async fn dye_recipe_approve_stamps_session_approver_not_body_identity() {
    let db = setup_test_db().await;
    let seeded = dye_recipe::ActiveModel {
        recipe_no: Set("DR-W11ID-001".to_string()),
        recipe_name: Set(Some("_identity_lock_配方".to_string())),
        color_code: Set(Some("C-W11ID".to_string())),
        status: Set(Some(dye_recipe_status::DRAFT.to_string())),
        is_deleted: Set(Some(false)),
        created_at: Set(now().into()),
        updated_at: Set(now().into()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("种子染色配方失败");
    let read_db = db.clone();
    let state = AppState {
        db: std::sync::Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        vec![(
            "/dye-recipes/{id}/approve",
            post(dye_recipe_handler::approve_recipe),
        )],
    );
    let body = forged_body(None);
    let (status, v) = call(
        &app,
        &format!("/dye-recipes/{}/approve", seeded.id),
        ReqShape::Json(&body),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "染色配方审批必须成功, 实际: {status} {v}"
    );

    let row = dye_recipe::Entity::find_by_id(seeded.id)
        .one(&read_db)
        .await
        .expect("染色配方直读失败")
        .expect("染色配方必须存在");
    assert_eq!(
        row.status.as_deref(),
        Some(dye_recipe_status::APPROVED),
        "状态机必须真实推进"
    );
    assert_eq!(
        row.approved_by,
        Some(SESSION_A),
        "直读实体：染色配方审核人必须等于会话用户 A，实际 {:?}",
        row.approved_by
    );
    assert_ne!(
        row.approved_by,
        Some(FORGED_B),
        "直读实体：body 伪造的 B 绝不许落进配方审核人列"
    );
}

// =========================================================
// 域 5：AI 模型版本（create 登记人 / approve 审批人 / change_status 变更人）
// =========================================================

async fn seed_model_version(db: &DatabaseConnection, name: &str, approval_status: &str) -> i32 {
    let row = ai_model_version::ActiveModel {
        model_name: Set(name.to_string()),
        version: Set("v1".to_string()),
        algorithm: Set("xgboost".to_string()),
        status: Set(master_data::DRAFT.to_string()),
        approval_status: Set(approval_status.to_string()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子模型版本 {name} 失败: {e}"));
    row.id
}

#[tokio::test]
async fn ai_model_version_create_stamps_session_changer_not_body_identity() {
    let db = setup_test_db().await;
    let read_db = db.clone();
    let state = AppState {
        db: std::sync::Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        vec![(
            "/ai/model-versions",
            post(ai_model_management_handler::create_model_version),
        )],
    );
    let body = forged_body(Some(json!({
        "model_name": "w11-id-create",
        "version": "v1",
        "algorithm": "xgboost",
    })));
    let (status, v) = call(&app, "/ai/model-versions", ReqShape::Json(&body)).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "登记模型版本必须成功, 实际: {status} {v}"
    );

    let row = ai_model_version::Entity::find()
        .filter(ai_model_version::Column::ModelName.eq("w11-id-create"))
        .one(&read_db)
        .await
        .expect("模型版本直读失败")
        .expect("模型版本必须存在");
    assert_eq!(
        row.changed_by,
        Some(SESSION_A),
        "直读实体：登记人(changed_by)必须等于会话用户 A，实际 {:?}",
        row.changed_by
    );
    assert_ne!(
        row.changed_by,
        Some(FORGED_B),
        "直读实体：body 伪造的 B 绝不许落进登记人列"
    );
    assert_eq!(
        row.approved_by, None,
        "新建行审批人必须为空（不许由 body 身份键预填）"
    );
}

#[tokio::test]
async fn ai_model_version_approve_stamps_session_approver_not_body_identity() {
    let db = setup_test_db().await;
    let vid = seed_model_version(&db, "w11-id-approve", master_data::PENDING).await;
    let read_db = db.clone();
    let state = AppState {
        db: std::sync::Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        vec![(
            "/ai/model-versions/{id}/approve",
            post(ai_model_management_handler::approve_model_version),
        )],
    );
    // approval_status 是 DTO 真实业务字段（仍必填），伪造的是 approved_by
    let body = forged_body(Some(json!({ "approval_status": master_data::APPROVED })));
    let (status, v) = call(
        &app,
        &format!("/ai/model-versions/{vid}/approve"),
        ReqShape::Json(&body),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "模型版本审批必须成功, 实际: {status} {v}"
    );

    let row = ai_model_version::Entity::find_by_id(vid)
        .one(&read_db)
        .await
        .expect("模型版本直读失败")
        .expect("模型版本必须存在");
    assert_eq!(
        row.approval_status,
        master_data::APPROVED,
        "业务字段 approval_status 必须照常生效（可选身份键不得吞合法业务字段）"
    );
    assert_eq!(
        row.approved_by,
        Some(SESSION_A),
        "直读实体：审批人必须等于会话用户 A，实际 {:?}",
        row.approved_by
    );
    assert_ne!(
        row.approved_by,
        Some(FORGED_B),
        "直读实体：body 伪造的 B 绝不许落进审批人列"
    );
}

#[tokio::test]
async fn ai_model_version_change_status_stamps_session_changer_not_body_identity() {
    let db = setup_test_db().await;
    // active 门：仅 approval_status=approved 的版本可激活（ai_model_management_service.rs:242-246）
    let vid = seed_model_version(&db, "w11-id-change", master_data::APPROVED).await;
    let read_db = db.clone();
    let state = AppState {
        db: std::sync::Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        vec![(
            "/ai/model-versions/{id}/status",
            post(ai_model_management_handler::change_model_status),
        )],
    );
    let body = forged_body(Some(json!({ "new_status": master_data::ACTIVE })));
    let (status, v) = call(
        &app,
        &format!("/ai/model-versions/{vid}/status"),
        ReqShape::Json(&body),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "状态变更必须成功, 实际: {status} {v}"
    );

    let row = ai_model_version::Entity::find_by_id(vid)
        .one(&read_db)
        .await
        .expect("模型版本直读失败")
        .expect("模型版本必须存在");
    assert_eq!(
        row.status,
        master_data::ACTIVE,
        "业务状态必须真实推进（状态门未被伪造键绕过）"
    );
    assert_eq!(
        row.changed_by,
        Some(SESSION_A),
        "直读实体：变更人必须等于会话用户 A，实际 {:?}",
        row.changed_by
    );
    assert_ne!(
        row.changed_by,
        Some(FORGED_B),
        "直读实体：body 伪造的 B 绝不许落进变更人列"
    );
}

// =========================================================
// 域 6：定制订单推进（custom_order_handler advance → 流程日志/节点操作人）
// =========================================================

/// FK 前置自种子（口径同 contract_wave5_custom_order_status_unity）：
/// process_logs.operator_id / process_nodes.operator_id 是真外键 →users，
/// 会话用户 A 与伪造用户 B 都必须有真实 users 行（显式主键，id 即常量本身）。
async fn seed_identity_users(db: &DatabaseConnection) {
    for uid in [SESSION_A, FORGED_B] {
        user::ActiveModel {
            id: Set(uid),
            username: Set(format!("w11id_u{uid}")),
            password_hash: Set("x".repeat(60)),
            is_active: Set(true),
            is_totp_enabled: Set(false),
            created_at: Set(now()),
            updated_at: Set(now()),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap_or_else(|e| panic!("种子 users({uid}) 失败: {e}"));
    }
}

#[tokio::test]
async fn custom_order_advance_stamps_session_operator_in_node_and_log() {
    let db = setup_test_db().await;
    seed_identity_users(&db).await;
    let ts = Utc::now().timestamp_nanos_opt().expect("nanos");
    let owner = user::Entity::find_by_id(SESSION_A)
        .one(&db)
        .await
        .expect("users 直读失败")
        .expect("会话用户 A 必须已种入");

    let cust = customer::ActiveModel {
        customer_code: Set(format!("CUST-W11ID-{ts}")),
        customer_name: Set("身份会话契约客户".to_string()),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
        customer_type: Set(customer_type::OTHER.to_string()),
        owner_id: Set(owner.id),
        created_by: Set(Some(owner.id)),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("种子 customers 失败");
    let prod = product::ActiveModel {
        name: Set(format!("身份会话契约产品-{ts}")),
        code: Set(format!("PRD-W11ID-{ts}")),
        unit: Set("m".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("fabric".to_string()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("种子 products 失败");

    // dyeing → finishing：非门控转换（advance 的 gate 只拦 quotation/yarn_purchasing），
    // 两个节点类型都在 chk_node_type 允许集内
    let order = custom_order::ActiveModel {
        order_no: Set(format!("CO-W11ID-{ts}")),
        customer_id: Set(cust.id as i64),
        product_id: Set(prod.id as i64),
        spec: Set("身份会话契约规格".to_string()),
        quantity: Set(Decimal::from(10_i64)),
        unit: Set("m".to_string()),
        custom_requirements: Set(json!({})),
        status: Set(co_status::DYEING.to_string()),
        currency: Set("CNY".to_string()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("种子 custom_orders 失败");

    let cur_node = process_node::ActiveModel {
        custom_order_id: Set(order.id),
        node_type: Set(co_status::DYEING.to_string()),
        node_name: Set("染色".to_string()),
        sequence: Set(1),
        status: Set(node_status::IN_PROGRESS.to_string()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("种子进行中节点失败");
    let next_node = process_node::ActiveModel {
        custom_order_id: Set(order.id),
        node_type: Set(co_status::FINISHING.to_string()),
        node_name: Set("后整理".to_string()),
        sequence: Set(2),
        status: Set(node_status::PENDING.to_string()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("种子待启动节点失败");

    let read_db = db.clone();
    let state = AppState {
        db: std::sync::Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        vec![(
            "/custom-orders/{id}/advance",
            post(custom_order_handler::advance_custom_order),
        )],
    );
    // DTO 只剩 notes；operator_id/user_id 是纯伪造键
    let body = forged_body(Some(json!({ "notes": "身份会话推进" })));
    let (status, v) = call(
        &app,
        &format!("/custom-orders/{}/advance", order.id),
        ReqShape::Json(&body),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "推进必须成功, 实际: {status} {v}");

    let fresh_order = custom_order::Entity::find_by_id(order.id)
        .one(&read_db)
        .await
        .expect("定制订单直读失败")
        .expect("定制订单必须存在");
    assert_eq!(
        fresh_order.status,
        co_status::FINISHING,
        "状态机必须真实推进 dyeing → finishing"
    );

    let done_node = process_node::Entity::find_by_id(cur_node.id)
        .one(&read_db)
        .await
        .expect("已完成节点直读失败")
        .expect("已完成节点必须存在");
    assert_eq!(
        done_node.status,
        node_status::COMPLETED,
        "当前节点必须置为 completed"
    );
    let started_node = process_node::Entity::find_by_id(next_node.id)
        .one(&read_db)
        .await
        .expect("下一节点直读失败")
        .expect("下一节点必须存在");
    assert_eq!(
        started_node.status,
        node_status::IN_PROGRESS,
        "下一节点必须置为 in_progress"
    );
    assert_eq!(
        started_node.operator_id,
        Some(SESSION_A),
        "直读实体：下一节点操作人必须等于会话用户 A，实际 {:?}",
        started_node.operator_id
    );
    assert_ne!(
        started_node.operator_id,
        Some(FORGED_B),
        "直读实体：body 伪造的 B 绝不许落进节点操作人列"
    );

    let log = process_log::Entity::find()
        .filter(process_log::Column::ProcessNodeId.eq(cur_node.id))
        .filter(process_log::Column::Action.eq("complete"))
        .one(&read_db)
        .await
        .expect("流程日志直读失败")
        .expect("推进必须产生 complete 流程日志");
    assert_eq!(
        log.operator_id,
        Some(SESSION_A),
        "直读实体：流程日志操作人必须等于会话用户 A，实际 {:?}",
        log.operator_id
    );
    assert_ne!(
        log.operator_id,
        Some(FORGED_B),
        "直读实体：body 伪造的 B 绝不许落进流程日志操作人列"
    );
}
