//! 批量 RFM 档位查询契约锁（连真库 PostgreSQL + 生产词表/机器码同源断言）。
//!
//! 钉死 `GET /crm/rfm/segments` 的三条契约：
//! - ①批量返回的档位与后端中文四桶权威词表逐字一致（词表来源 = 生产
//!   `services::crm::RfmSegment::as_str`，非 Debug 形态、非本文件另抄的常量）；
//! - ②请求里混入不存在 / 越权的 customer_id 时的**确定行为**（逐行回 not_found /
//!   no_permission，不静默丢弃、不为其泄露档位、不裸 500）；
//! - ③入参超单次规模上界时的拒绝机器码（HTTP 400 + `VALIDATION_ERROR` + 出参走
//!   生产固定脱敏常量 `err_msg::VALIDATION_PUBLIC`，文案不含任何记录 ID）。
//!
//! 权限键派生另有一条纯函数 #[test]（直接调生产 `extract_resource_info` /
//! `extract_action_from_path` / `method_to_action` 与 `resolve_module_prefixed_resource`，
//! 不复刻规则）钉住 `GET /crm/rfm/segments` 经 `crm/rfm → customers` 消歧后派生
//! `customers:read`（与行级端点 `/crm/customers/{id}/rfm` 同一枚已登记键、同源同权）。
//!
//! 通道：`setup_test_db` 连已迁移 PostgreSQL 真跑；表结构唯一来源 = backend/migration，
//! 缺 `TEST_DATABASE_URL` 直接 panic，不回退 sqlite、不 `#[ignore]`、不 skip。FK 父行自种子
//!（users→customers→sales_orders，口径同 `contract_wave11_rfm_score_shape_test`）。

mod test_common;

use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::{Method, Request, StatusCode};
use axum::middleware::{Next, from_fn_with_state};
use axum::response::Response;
use axum::routing::get;
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::crm_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::middleware::permission::{
    extract_action_from_path, extract_resource_info, method_to_action,
};
use bingxi_backend::models::{customer, sales_order, user};
use bingxi_backend::services::crm::RfmSegment;
use bingxi_backend::services::crm::cust::CrmService;
use bingxi_backend::utils::data_scope::{DataScope, DataScopeContext};
use bingxi_backend::utils::messages::err_msg;
use bingxi_backend::utils::path_utils::{
    is_nested_module_prefix, resolve_module_prefixed_resource,
};
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection};
use serde_json::Value;
use std::sync::Arc;
use test_common::setup_test_db;

// ---------------------------------------------------------------------------
// 一、权限键派生（纯函数，真调生产提取器，不复刻 URL→键规则）
// ---------------------------------------------------------------------------

/// `GET /crm/rfm/segments` 的运行时权限键必须是 `customers:read`：
/// - seg3=`crm` 是模块前缀（白名单放行），非双层子模块（`is_nested_module_prefix("crm","rfm")`
///   为 false），故资源名走 `resolve_module_prefixed_resource("crm","rfm")`；该消歧表已把
///   `crm/rfm` 对齐到注册表权威名 `customers`（与行级端点 `/crm/customers/{id}/rfm` 同源）；
/// - 末段 `segments` 非动作关键字、非数字 → 无 resource_id；GET 无 `?action` → 动作 `read`。
/// 键名以真调生产函数为准（不猜）：一旦消歧被改动，本锁红会强制重新审视权限落点。
#[test]
fn get_crm_rfm_segments_derives_customers_read_key() {
    // 佐证消歧前提：crm/rfm 不是双层子模块、且被消歧到 customers。
    assert!(
        !is_nested_module_prefix("crm", "rfm"),
        "crm/rfm 不应被判为双层模块前缀（否则资源名会漂到 seg5 维度段）"
    );
    assert_eq!(
        resolve_module_prefixed_resource("crm", "rfm"),
        "customers",
        "crm/rfm 必须消歧到注册表权威名 customers（与单客户 rfm 端点同键）"
    );
    let path = "/api/v1/erp/crm/rfm/segments";
    let (resource, id) = extract_resource_info(path);
    assert_eq!(
        (resource.as_str(), id),
        ("customers", None),
        "批量档位端点经消歧后资源段必须是 customers、且无记录级 resource_id（否则退化成按记录授权、批量读永不命中）"
    );
    assert_eq!(
        extract_action_from_path(path).as_deref(),
        None,
        "末段 segments 不是动作关键字，不得派生成动作段（否则产生注册表外的伪动作键）"
    );
    assert_eq!(method_to_action(&Method::GET).as_str(), "read");
    assert_eq!(
        format!("{}:{}", resource, method_to_action(&Method::GET)),
        "customers:read",
        "GET /crm/rfm/segments 运行时权限键必须是 customers:read（复用已登记键，无需新增授权）"
    );
}

// ---------------------------------------------------------------------------
// 二、真库夹具：users(7) → customers(101 有单/VIP 档、102 无单/低价值档)
// ---------------------------------------------------------------------------

async fn seeded_db() -> DatabaseConnection {
    let db = setup_test_db().await;
    let now = Utc::now();
    user::ActiveModel {
        id: Set(7),
        username: Set("w15_rfm_seg_owner".to_string()),
        password_hash: Set("test-only-not-a-real-hash".to_string()),
        real_name: Set(Some("批量档位夹具归属人".to_string())),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("夹具：users 迁移不播种，必须自插归属用户");

    seed_customer(&db, 101, "RFM-SEG-VIP").await;
    seed_customer(&db, 102, "RFM-SEG-LOW").await;

    // customer 101 铺 6 张近期订单，单价 200000 → R=5 F=4 M=5，合成分≈4.67 ∈ VIP 档。
    for oid in 5001..5007 {
        seed_order(&db, oid, 101, Decimal::new(200_000, 0)).await;
    }
    db
}

async fn seed_customer(db: &DatabaseConnection, id: i32, code: &str) {
    let now = Utc::now();
    customer::ActiveModel {
        id: Set(id),
        customer_code: Set(code.to_string()),
        customer_name: Set(format!("批量档位客户-{code}")),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
        customer_type: Set("retail".to_string()),
        owner_id: Set(7),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("夹具：自插客户行（owner_id=7 真 FK 父行已在场）");
}

async fn seed_order(db: &DatabaseConnection, id: i32, customer_id: i32, amount: Decimal) {
    let now = Utc::now();
    sales_order::ActiveModel {
        id: Set(id),
        order_no: Set(format!("SO-RFM-SEG-{id}")),
        customer_id: Set(customer_id),
        order_date: Set(now),
        required_date: Set(Some(now)),
        status: Set("approved".to_string()),
        subtotal: Set(amount),
        tax_amount: Set(Decimal::ZERO),
        discount_amount: Set(Decimal::ZERO),
        shipping_cost: Set(Decimal::ZERO),
        total_amount: Set(amount),
        paid_amount: Set(Decimal::ZERO),
        balance_amount: Set(amount),
        created_by: Set(Some(7)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("夹具：sales_orders 有 customers/users 外键，父行已自插");
}

fn svc(db: &DatabaseConnection) -> CrmService {
    CrmService::new(Arc::new(db.clone()))
}

fn ctx_all() -> DataScopeContext {
    DataScopeContext {
        scope: DataScope::All,
        user_id: 7,
        department_id: Some(1),
        dept_ids: vec![1],
        dept_member_user_ids: vec![7],
    }
}

/// Self_ 范围、user_id=42（非任何客户归属人）→ 归属门对 customer(101/102) 一律拒绝。
fn ctx_self_other() -> DataScopeContext {
    DataScopeContext {
        scope: DataScope::Self_,
        user_id: 42,
        department_id: None,
        dept_ids: vec![],
        dept_member_user_ids: vec![],
    }
}

/// 从返回 JSON 数组里按 customer_id 取一行对象。
fn row_of(items: &[Value], cid: i32) -> Value {
    items
        .iter()
        .find(|r| r["customer_id"].as_i64() == Some(cid as i64))
        .cloned()
        .unwrap_or_else(|| {
            panic!("被请求的 customer_id={cid} 必须原样出现在结果里（禁止静默过滤）")
        })
}

// ---------------------------------------------------------------------------
// 三、①档位与中文四桶词表逐字一致（词表来自生产枚举，禁 Debug 形态）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn batch_segments_are_verbatim_authoritative_four_buckets() {
    let db = seeded_db().await;
    let service = svc(&db);
    let items = service
        .get_rfm_segments(&[101, 102], &ctx_all())
        .await
        .expect("All 范围批量档位查询应成功");
    let json_items = serde_json::to_value(&items).expect("档位出参可序列化");
    let arr = json_items.as_array().expect("批量档位出参必须是数组");
    assert_eq!(arr.len(), 2, "两个被请求且可见的客户都必须有一行结果");

    let vip_row = row_of(arr, 101);
    let low_row = row_of(arr, 102);
    assert_eq!(vip_row["access"], "visible");
    assert_eq!(low_row["access"], "visible");

    // 期望档位 token 直接取生产枚举 as_str()（唯一词表来源），不写第二套字面量。
    // 若有人把出参改成 Debug 形态（变体名 "Vip"/"RfmSegment::Vip"），此处必红。
    assert_eq!(
        vip_row["segment"].as_str(),
        Some(RfmSegment::Vip.as_str()),
        "高分客户档位必须等于权威词表 VIP token（逐字同源，禁 Debug 变体名）"
    );
    assert_eq!(
        low_row["segment"].as_str(),
        Some(RfmSegment::LowValue.as_str()),
        "无单客户档位必须等于权威词表低价值 token（逐字同源，禁 Debug 变体名）"
    );

    // 交叉核对：批量档位词表 ⊆ 群体分布出参键集（两个出参共用同一枚举，不得漂移）。
    let dist = service
        .get_rfm_distribution(&ctx_all())
        .await
        .expect("群体分布查询应成功");
    let dist_keys: Vec<&str> = dist
        .as_object()
        .expect("分布出参必须是对象")
        .keys()
        .map(|k| k.as_str())
        .filter(|k| *k != "total_customers")
        .collect();
    for row in arr {
        if row["access"] == "visible" {
            let seg = row["segment"].as_str().expect("可见行必须有档位字符串");
            assert!(
                dist_keys.contains(&seg),
                "批量档位 token「{seg}」必须是群体分布键集（{dist_keys:?}）之一，否则两出参词表漂移"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 四、②不存在 / 越权 id 的确定行为（逐行结论，不静默丢弃、不泄露档位）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn not_found_and_out_of_scope_ids_yield_determinate_access() {
    let db = seeded_db().await;
    let service = svc(&db);
    // 请求含：101（存在但越权）、999999（不存在）、102（存在但越权）；Self_ 范围非归属人。
    let items = service
        .get_rfm_segments(&[101, 999999, 102], &ctx_self_other())
        .await
        .expect("越权/不存在不得抛错，必须逐行给确定结论");
    let json_items = serde_json::to_value(&items).expect("出参可序列化");
    let arr = json_items.as_array().expect("批量档位出参必须是数组");
    assert_eq!(
        arr.len(),
        3,
        "每个被请求 id 都必须有一行结果（不得静默过滤掉越权/不存在的项）"
    );

    let r101 = row_of(arr, 101);
    let r999 = row_of(arr, 999999);
    let r102 = row_of(arr, 102);
    // 存在但不在数据范围 → no_permission，且不泄露档位（segment=null）。
    assert_eq!(
        r101["access"], "no_permission",
        "越权可见行必须判 no_permission"
    );
    assert!(r101["segment"].is_null(), "越权行不得返回档位");
    assert_eq!(r102["access"], "no_permission");
    assert!(r102["segment"].is_null());
    // 不存在 → not_found。
    assert_eq!(r999["access"], "not_found", "不存在的 id 必须判 not_found");
    assert!(r999["segment"].is_null());

    // 对照：同一条 101 在 All 范围必须翻成 visible（证明差异源于归属门，而非数据缺失）。
    let visible = service
        .get_rfm_segments(&[101], &ctx_all())
        .await
        .expect("All 范围查询成功");
    let vjson = serde_json::to_value(&visible).expect("出参可序列化");
    assert_eq!(vjson[0]["access"], "visible");
    assert!(!vjson[0]["segment"].is_null());
}

// ---------------------------------------------------------------------------
// 五、③入参超上界的拒绝机器码（真走 handler→AppError→HTTP 信封）
// ---------------------------------------------------------------------------

fn make_auth() -> AuthContext {
    AuthContext {
        user_id: 7,
        username: "w15_rfm_seg_owner".to_string(),
        role_id: Some(1),
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

/// 只挂 handler + 注入 auth 的最小路由（上界拒绝发生在取数前，无需权限中间件）。
fn app_over_limit(db: Arc<DatabaseConnection>) -> Router {
    let state = AppState {
        db: db.clone(),
        ..Default::default()
    };
    Router::new()
        .route("/crm/rfm/segments", get(crm_handler::get_rfm_segments))
        .with_state(state)
        .layer(from_fn_with_state(make_auth(), inject_auth))
}

#[tokio::test]
async fn over_limit_customer_ids_rejected_as_validation_400() {
    use tower::ServiceExt;
    let db = Arc::new(seeded_db().await);
    let app = app_over_limit(db);

    // 501 个 id（超过生产上界 500）：拼 1..501。
    let ids = (1..=501)
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/crm/rfm/segments?customer_ids={ids}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "超上界属字段校验族，HTTP 必须是 400"
    );
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .expect("读取失败信封体");
    let body: Value = serde_json::from_slice(&bytes).expect("失败信封必须是 JSON");
    assert_eq!(
        body["code"].as_str(),
        Some("VALIDATION_ERROR"),
        "机器码必须是 VALIDATION_ERROR（字段校验族），不得映射为 BUSINESS/INTERNAL"
    );
    // 出参文案必须等于生产固定脱敏常量，且不含任何记录 ID（连提交的 1..501 数字都不得回显）。
    assert_eq!(
        body["message"].as_str(),
        Some(err_msg::VALIDATION_PUBLIC),
        "超上界出参走生产固定脱敏常量，禁止外显真实拒绝详情"
    );
    let msg = body["message"].as_str().unwrap_or_default();
    assert!(
        !msg.contains("501") && !msg.contains("500"),
        "用户可见文案禁止出现记录/数量等任何数字标识，实际: {msg}"
    );
}
