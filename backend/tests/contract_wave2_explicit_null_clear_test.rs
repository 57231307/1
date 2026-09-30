//! 合同 wave2 收口（二）：更新链路显式 null 三态清空语义（对齐 RFC 7386 JSON Merge Patch）
//!
//! 锁定的真实行为（对应本轮修复）：
//! 1. 三态语义：键缺席=保持原值、显式 `null`=清空为 NULL、有值=覆盖。
//!    - DTO 层：`Option<Option<T>>` + `double_option` 适配器区分缺席与显式 null；
//!    - service 层：Some(None) → `Set(None)` 真实落 NULL；None 不 Set（列不进 UPDATE）；
//!    - NOT NULL 列（contract_name/supplier_id/customer_id；department 的 name/code/
//!      sort_order/is_active）不开 null 清空：显式 null 在任何 DB 访问前被
//!      `AppError::business_displayable` 拒绝（400 + 外显文案，不脱敏、无副作用）。
//!    file: `handlers/purchase_contract_handler.rs` UpdateContractDto、
//!          `handlers/sales_contract_handler.rs` UpdateSalesContractDto、
//!          `handlers/department_handler.rs` UpdateDepartmentRequest、
//!          `services/purchase_contract_service.rs`/`services/sales_contract_service.rs`/
//!          `services/department_service.rs` update。
//! 2. 审计：update_with_audit 以更新后回读模型生成 after_snapshot，清空列的快照为真实 NULL。
//!
//! 覆盖策略（对齐 contract_wave2_contract_no_and_remark_test.rs 范式）：
//! - 非活库（sqlite::memory: / 纯 serde）：DTO 三态形状锁、NOT NULL 显式 null 的
//!   service 级拒绝（拒绝在任何 DB 访问前返回，sqlite 可达）、handler 400 信封与文案外显。
//! - `#[ignore]` 活库（TEST_DATABASE_URL→PG，CI job ci-test-rust-ignored 执行）：
//!   update 全链路（lock_exclusive + update_with_audit 仅 PG 方言可跑）——
//!   发 null 后 DB 回读为 NULL、不发键原值保持、审计快照反映真实值。
//!   活库用例首先断言后端为 PostgreSQL，禁止 sqlite 回退下假绿。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::{
    department_handler, purchase_contract_handler, sales_contract_handler,
};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::{audit_log, department, purchase_contract, sales_contract};
use bingxi_backend::services::department_service::DepartmentService;
use bingxi_backend::services::purchase_contract_service::{
    CreateContractRequest, PurchaseContractService, UpdateContractRequest,
};
use bingxi_backend::services::sales_contract_service::{
    CreateSalesContractRequest, SalesContractService, UpdateSalesContractRequest,
};
use bingxi_backend::services::supplier_service::{CreateSupplierRequest, SupplierService};
use bingxi_backend::utils::error::AppError;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbBackend, EntityTrait, Order, QueryFilter,
    QueryOrder, Set,
};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

// =========================================================
// 公共夹具
// =========================================================

fn make_scope_auth(user_id: i32, scope: Option<&str>) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("e2e_user_{user_id}"),
        role_id: Some(2),
        department_id: Some(1),
        data_scope: scope.map(|s| s.to_string()),
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

async fn call(app: &Router, method: Method, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    let req = match body {
        Some(v) => {
            builder = builder.header("content-type", "application/json");
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

async fn sqlite_db() -> DatabaseConnection {
    sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败")
}

fn purchase_update_router(db: DatabaseConnection) -> Router {
    let mut state = AppState::default();
    state.db = Arc::new(db);
    Router::new()
        .route(
            "/purchase-contracts/{id}",
            axum::routing::put(purchase_contract_handler::update_contract),
        )
        .with_state(state)
        .layer(from_fn_with_state(
            make_scope_auth(9101, Some("all")),
            inject_auth,
        ))
}

fn sales_update_router(db: DatabaseConnection) -> Router {
    let mut state = AppState::default();
    state.db = Arc::new(db);
    Router::new()
        .route(
            "/sales-contracts/{id}",
            axum::routing::put(sales_contract_handler::update_contract),
        )
        .with_state(state)
        .layer(from_fn_with_state(
            make_scope_auth(9101, Some("all")),
            inject_auth,
        ))
}

// =========================================================
// A) DTO 三态形状锁（纯 serde，无 DB）：
//    缺席=None（保持）、null=Some(None)（清空）、有值=Some(Some(v))（覆盖）
// =========================================================

#[test]
fn purchase_update_dto_distinguishes_absent_null_and_value() {
    let dto: purchase_contract_handler::UpdateContractDto = serde_json::from_value(json!({
        "remark": null,
        "delivery_date": null,
        "payment_terms": "月结30天",
        "contract_name": null,
    }))
    .expect("采购 update DTO 三态反序列化失败");
    assert!(
        matches!(dto.remark, Some(None)),
        "显式 null 的 remark 必须反序列化为 Some(None)（清空），不得与缺席塌同"
    );
    assert!(matches!(dto.delivery_date, Some(None)));
    assert!(matches!(&dto.payment_terms, Some(Some(v)) if v == "月结30天"));
    assert!(dto.signed_date.is_none(), "缺席键必须是 None（保持原值）");
    assert!(dto.total_amount.is_none());
    assert!(
        matches!(dto.contract_name, Some(None)),
        "NOT NULL 列的显式 null 也必须可辨，才能交 service 判业务错误拒绝"
    );

    let with_date: purchase_contract_handler::UpdateContractDto = serde_json::from_value(json!({
        "delivery_date": "2026-10-01",
    }))
    .expect("采购 update DTO 日期值反序列化失败");
    assert!(matches!(
        with_date.delivery_date,
        Some(Some(d)) if d == chrono::NaiveDate::from_ymd_opt(2026, 10, 1).unwrap()
    ));
}

#[test]
fn sales_update_dto_distinguishes_absent_null_and_value() {
    let dto: sales_contract_handler::UpdateSalesContractDto = serde_json::from_value(json!({
        "expiry_date": null,
        "remark": null,
        "payment_terms": "货到付款",
        "customer_id": null,
    }))
    .expect("销售 update DTO 三态反序列化失败");
    assert!(matches!(dto.expiry_date, Some(None)));
    assert!(matches!(dto.remark, Some(None)));
    assert!(matches!(&dto.payment_terms, Some(Some(v)) if v == "货到付款"));
    assert!(dto.delivery_date.is_none(), "缺席键必须是 None（保持原值）");
    assert!(matches!(dto.customer_id, Some(None)));

    let with_amount: sales_contract_handler::UpdateSalesContractDto =
        serde_json::from_value(json!({ "total_amount": "1000.00" }))
            .expect("销售 update DTO 金额值反序列化失败");
    assert!(matches!(&with_amount.total_amount, Some(Some(v)) if *v == Decimal::from(1000)));
}

#[test]
fn department_update_dto_distinguishes_absent_null_and_value() {
    let req: department_handler::UpdateDepartmentRequest = serde_json::from_value(json!({
        "description": null,
        "parent_id": null,
        "manager_id": 5,
        "sort_order": 3,
    }))
    .expect("部门 update DTO 三态反序列化失败");
    assert!(matches!(req.description, Some(None)));
    assert!(matches!(req.parent_id, Some(None)));
    assert!(matches!(req.manager_id, Some(Some(5))));
    assert!(matches!(req.sort_order, Some(Some(3))));
    assert!(req.name.is_none(), "缺席键必须是 None（保持原值）");
    assert!(req.is_active.is_none());
    assert!(req.code.is_none());

    let keep: department_handler::UpdateDepartmentRequest =
        serde_json::from_value(json!({ "manager_id": null }))
            .expect("部门 update DTO 清空负责人形态反序列化失败");
    assert!(
        matches!(keep.manager_id, Some(None)),
        "manager_id 显式 null = 清除负责人，不得塌成缺席"
    );
}

// =========================================================
// B) service 层 NOT NULL 显式 null 拒绝（sqlite 即可：拒绝在任何 DB 访问前返回）
// =========================================================

#[tokio::test]
async fn purchase_update_explicit_null_on_not_null_columns_rejected_before_db() {
    let db = sqlite_db().await;
    let service = PurchaseContractService::new(Arc::new(db));

    let err = service
        .update(
            999_999,
            UpdateContractRequest {
                contract_name: Some(None),
                ..Default::default()
            },
            9101,
        )
        .await
        .expect_err("NOT NULL 列 contract_name 显式 null 必须被拒绝");
    assert!(
        matches!(&err, AppError::BusinessErrorDisplayable(m) if m.contains("合同名称不能清空")),
        "期望可见业务错误，实际: {err:?}"
    );

    let err2 = service
        .update(
            999_999,
            UpdateContractRequest {
                supplier_id: Some(None),
                ..Default::default()
            },
            9101,
        )
        .await
        .expect_err("NOT NULL 列 supplier_id 显式 null 必须被拒绝");
    assert!(
        matches!(&err2, AppError::BusinessErrorDisplayable(m) if m.contains("供应商不能清空")),
        "期望可见业务错误，实际: {err2:?}"
    );
}

#[tokio::test]
async fn sales_update_explicit_null_on_not_null_columns_rejected_before_db() {
    let db = sqlite_db().await;
    let service = SalesContractService::new(Arc::new(db));

    let err = service
        .update(
            999_999,
            UpdateSalesContractRequest {
                contract_name: Some(None),
                ..Default::default()
            },
            9101,
        )
        .await
        .expect_err("NOT NULL 列 contract_name 显式 null 必须被拒绝");
    assert!(
        matches!(&err, AppError::BusinessErrorDisplayable(m) if m.contains("合同名称不能清空")),
        "期望可见业务错误，实际: {err:?}"
    );

    let err2 = service
        .update(
            999_999,
            UpdateSalesContractRequest {
                customer_id: Some(None),
                ..Default::default()
            },
            9101,
        )
        .await
        .expect_err("NOT NULL 列 customer_id 显式 null 必须被拒绝");
    assert!(
        matches!(&err2, AppError::BusinessErrorDisplayable(m) if m.contains("客户不能清空")),
        "期望可见业务错误，实际: {err2:?}"
    );
}

#[tokio::test]
async fn department_update_explicit_null_on_not_null_columns_rejected_before_db() {
    let db = sqlite_db().await;
    let service = DepartmentService::new(Arc::new(db));

    for (field_desc, req) in [
        (
            "name",
            department_handler::UpdateDepartmentRequest {
                name: Some(None),
                code: None,
                description: None,
                parent_id: None,
                manager_id: None,
                sort_order: None,
                is_active: None,
            },
        ),
        (
            "code",
            department_handler::UpdateDepartmentRequest {
                name: None,
                code: Some(None),
                description: None,
                parent_id: None,
                manager_id: None,
                sort_order: None,
                is_active: None,
            },
        ),
        (
            "sort_order",
            department_handler::UpdateDepartmentRequest {
                name: None,
                code: None,
                description: None,
                parent_id: None,
                manager_id: None,
                sort_order: Some(None),
                is_active: None,
            },
        ),
        (
            "is_active",
            department_handler::UpdateDepartmentRequest {
                name: None,
                code: None,
                description: None,
                parent_id: None,
                manager_id: None,
                sort_order: None,
                is_active: Some(None),
            },
        ),
    ] {
        // 拒绝必须发生在任何 DB 访问前（sqlite 内存库无 departments 表也能得到业务错误，
        // 若返回的不是 BusinessErrorDisplayable 而是 404/500 形态，即说明拒绝点放错了位置）
        let err = match service.update(999_999, 9101, req).await {
            Err(e) => e,
            Ok(m) => panic!("字段 {field_desc} 显式 null 应被拒绝，却返回成功: {m:?}"),
        };
        assert!(
            matches!(&err, AppError::BusinessErrorDisplayable(m) if m.contains("不能清空")),
            "NOT NULL 列 {field_desc} 显式 null 应被业务错误拒绝，实际: {err:?}"
        );
    }
}

// =========================================================
// C) handler 400 信封：NOT NULL 显式 null → BUSINESS_ERROR + 文案外显（不脱敏）
// =========================================================

#[tokio::test]
async fn purchase_put_null_contract_name_400_visible_message() {
    let app = purchase_update_router(sqlite_db().await);
    let (status, v) = call(
        &app,
        Method::PUT,
        "/purchase-contracts/1",
        Some(json!({ "contract_name": null })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "实际: {v}");
    assert_eq!(v["code"], "BUSINESS_ERROR");
    assert_eq!(v["message"], "合同名称不能清空：该字段为必填项");
    assert_ne!(
        v["message"], "业务处理失败",
        "business_displayable 不得被脱敏"
    );
}

#[tokio::test]
async fn sales_put_null_customer_id_400_visible_message() {
    let app = sales_update_router(sqlite_db().await);
    let (status, v) = call(
        &app,
        Method::PUT,
        "/sales-contracts/1",
        Some(json!({ "customer_id": null })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "实际: {v}");
    assert_eq!(v["code"], "BUSINESS_ERROR");
    assert_eq!(v["message"], "客户不能清空：请选择有效客户");
    assert_ne!(
        v["message"], "业务处理失败",
        "business_displayable 不得被脱敏"
    );
}

// =========================================================
// D) 活库（PG）：三态全链路——发 null 后列回读为 NULL、不发键原值保持、审计反映真实值
// =========================================================

/// 活库用例的 PG 硬断言：TEST_DATABASE_URL 缺失时 setup_test_db 回退 sqlite，
/// 必须显式炸红（条件跳过/静默 pass 是本仓定性的假绿根因）。
async fn require_postgres(db: &DatabaseConnection) {
    assert_eq!(
        db.get_database_backend(),
        DbBackend::Postgres,
        "本用例必须跑在已迁移的 PostgreSQL（TEST_DATABASE_URL）上，由 ci-test-rust-ignored 执行；禁止 sqlite 回退假绿"
    );
}

#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL（service.update 走 lock_exclusive + update_with_audit）"]
async fn live_purchase_update_null_clears_nullable_and_absent_keeps_with_audit_truth() {
    let db = test_common::setup_test_db().await;
    require_postgres(&db).await;
    let suffix = Utc::now().timestamp_nanos_opt().unwrap();

    let sup = SupplierService::new(Arc::new(db.clone()))
        .create_supplier(
            CreateSupplierRequest {
                supplier_name: format!("三态清空供应商{suffix}"),
                supplier_short_name: None,
                supplier_type: None,
                credit_code: None,
                registered_address: None,
                business_address: None,
                legal_representative: None,
                registered_capital: None,
                annual_revenue: None,
                establishment_date: None,
                business_term: None,
                business_scope: None,
                taxpayer_type: None,
                bank_name: None,
                bank_account: None,
                contact_phone: None,
                fax: None,
                website: None,
                email: None,
                main_business: None,
                main_market: None,
                employee_count: None,
                category_id: None,
                is_processor: None,
                processor_type: None,
                contacts: None,
                qualifications: None,
            },
            9101,
        )
        .await
        .unwrap();

    let service = PurchaseContractService::new(Arc::new(db.clone()));
    let created = service
        .create(
            CreateContractRequest {
                contract_no: format!("PC-XNULL-{suffix}"),
                contract_name: "三态清空采购合同".to_string(),
                supplier_id: sup.id,
                total_amount: Decimal::from(500),
                contract_type: Some("PURCHASE".to_string()),
                payment_terms: Some("月结30天".to_string()),
                delivery_date: chrono::NaiveDate::from_ymd_opt(2026, 10, 1),
                signed_date: chrono::NaiveDate::from_ymd_opt(2026, 9, 1),
                effective_date: None,
                expiry_date: None,
                payment_method: Some("BANK_TRANSFER".to_string()),
                delivery_location: Some("杭州仓库".to_string()),
                remark: Some("原备注不得被无声保留".to_string()),
            },
            9101,
        )
        .await
        .unwrap();

    let app = purchase_update_router(db.clone());
    let uri = format!("/purchase-contracts/{}", created.id);

    // 1) NOT NULL 显式 null → 400 且行零改动（拒绝在任何 DB 访问前）
    let (status, v) = call(
        &app,
        Method::PUT,
        &uri,
        Some(json!({ "remark": null, "contract_name": null })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "实际: {v}");
    let untouched = purchase_contract::Entity::find_by_id(created.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        untouched.remark.as_deref(),
        Some("原备注不得被无声保留"),
        "被拒绝的请求不得留下任何写入痕迹"
    );

    // 2) 三态 PUT：remark/delivery_date 显式 null 清空；payment_terms/signed_date 键缺席保持
    let (status, v) = call(
        &app,
        Method::PUT,
        &uri,
        Some(json!({ "remark": null, "delivery_date": null })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "三态 update 失败: {v}");
    assert!(v["data"]["remark"].is_null(), "响应体 remark 必须为 null");

    let reread = purchase_contract::Entity::find_by_id(created.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert!(
        reread.remark.is_none(),
        "发 null 后 remark 列 DB 回读必须为 NULL"
    );
    assert!(
        reread.delivery_date.is_none(),
        "发 null 后 delivery_date 列 DB 回读必须为 NULL"
    );
    assert_eq!(
        reread.payment_terms.as_deref(),
        Some("月结30天"),
        "未发键的 payment_terms 必须保持原值"
    );
    assert_eq!(
        reread.signed_date,
        chrono::NaiveDate::from_ymd_opt(2026, 9, 1),
        "未发键的 signed_date 必须保持原值"
    );
    assert_eq!(reread.contract_name, "三态清空采购合同");

    // 3) 审计快照反映变更后真实值：after_snapshot.remark/delivery_date 为 null
    let log = audit_log::Entity::find()
        .filter(audit_log::Column::ResourceType.eq("auto_audit"))
        .filter(audit_log::Column::ResourceId.eq(created.id.to_string()))
        .order_by(audit_log::Column::Id, Order::Desc)
        .one(&db)
        .await
        .unwrap()
        .expect("update_with_audit 必须落审计行");
    let after = log
        .after_snapshot
        .as_ref()
        .expect("after_snapshot 必须存在");
    assert!(
        after.0.get("remark").map(|x| x.is_null()).unwrap_or(false),
        "审计 after_snapshot 必须反映清空后的真实 NULL: {}",
        after.0
    );
    let before = log
        .before_snapshot
        .as_ref()
        .expect("before_snapshot 必须存在");
    assert_eq!(before.0.get("remark"), Some(&json!("原备注不得被无声保留")));
}

#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL（service.update 走 lock_exclusive + update_with_audit）"]
async fn live_sales_update_null_clears_nullable_and_absent_keeps() {
    let db = test_common::setup_test_db().await;
    require_postgres(&db).await;
    let suffix = Utc::now().timestamp_nanos_opt().unwrap();

    let cust = bingxi_backend::models::customer::ActiveModel {
        customer_code: Set(format!("CUS-XNULL-{suffix}")),
        customer_name: Set(format!("三态清空客户{suffix}")),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
        customer_type: Set("retail".to_string()),
        owner_id: Set(9101),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let service = SalesContractService::new(Arc::new(db.clone()));
    let created = service
        .create(
            CreateSalesContractRequest {
                contract_no: format!("SC-XNULL-{suffix}"),
                contract_name: "三态清空销售合同".to_string(),
                customer_id: cust.id,
                total_amount: Decimal::from(800),
                contract_type: None,
                payment_terms: Some("货到付款".to_string()),
                delivery_date: None,
                signed_date: None,
                effective_date: chrono::NaiveDate::from_ymd_opt(2026, 9, 5),
                expiry_date: chrono::NaiveDate::from_ymd_opt(2026, 12, 31),
                payment_method: None,
                delivery_location: Some("上海港".to_string()),
                remark: Some("待清空的备注".to_string()),
                items: None,
            },
            9101,
        )
        .await
        .unwrap();

    let app = sales_update_router(db.clone());
    let uri = format!("/sales-contracts/{}", created.id);
    let (status, v) = call(
        &app,
        Method::PUT,
        &uri,
        Some(json!({ "remark": null, "expiry_date": null })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "三态 update 失败: {v}");

    let reread = sales_contract::Entity::find_by_id(created.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert!(
        reread.remark.is_none(),
        "发 null 后 remark 列 DB 回读必须为 NULL"
    );
    assert!(
        reread.expiry_date.is_none(),
        "发 null 后 expiry_date 列 DB 回读必须为 NULL"
    );
    assert_eq!(
        reread.effective_date,
        chrono::NaiveDate::from_ymd_opt(2026, 9, 5),
        "未发键的 effective_date 必须保持原值"
    );
    assert_eq!(reread.payment_terms.as_deref(), Some("货到付款"));
    assert_eq!(reread.delivery_location.as_deref(), Some("上海港"));
}

#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL（update_with_audit + fetch_username 走真实表）"]
async fn live_department_update_null_clears_description_and_detaches_parent() {
    let db = test_common::setup_test_db().await;
    require_postgres(&db).await;
    let suffix = Utc::now().timestamp_nanos_opt().unwrap();
    let service = DepartmentService::new(Arc::new(db.clone()));

    let parent = service
        .create(
            department_handler::CreateDepartmentRequest {
                name: format!("三态父部门{suffix}"),
                code: Some(format!("XN-P-{suffix}")),
                description: Some("父部门说明".to_string()),
                parent_id: None,
                manager_id: None,
                sort_order: None,
            },
            9101,
        )
        .await
        .unwrap();
    let child = service
        .create(
            department_handler::CreateDepartmentRequest {
                name: format!("三态子部门{suffix}"),
                code: Some(format!("XN-C-{suffix}")),
                description: Some("子部门说明".to_string()),
                parent_id: Some(parent.id),
                manager_id: None,
                sort_order: None,
            },
            9101,
        )
        .await
        .unwrap();

    // 三态：description/parent_id 显式 null 清空（脱离父级），name/code 缺席保持
    let cleared = service
        .update(
            child.id,
            9101,
            department_handler::UpdateDepartmentRequest {
                name: None,
                code: None,
                description: Some(None),
                parent_id: Some(None),
                manager_id: None,
                sort_order: None,
                is_active: None,
            },
        )
        .await
        .expect("部门三态 update 失败");
    assert!(cleared.description.is_none());
    assert!(cleared.parent_id.is_none());

    let reread = department::Entity::find_by_id(child.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert!(
        reread.description.is_none(),
        "发 null 后 description 列 DB 回读必须为 NULL"
    );
    assert!(
        reread.parent_id.is_none(),
        "发 null 后 parent_id 列必须为 NULL（脱离父级）"
    );
    assert_eq!(
        reread.name,
        format!("三态子部门{suffix}"),
        "缺席键必须保持原值"
    );
    assert_eq!(reread.code, format!("XN-C-{suffix}"));

    // 审计：after_snapshot 反映清空后的真实 NULL
    let log = audit_log::Entity::find()
        .filter(audit_log::Column::ResourceType.eq("departments"))
        .filter(audit_log::Column::ResourceId.eq(child.id.to_string()))
        .order_by(audit_log::Column::Id, Order::Desc)
        .one(&db)
        .await
        .unwrap()
        .expect("update_with_audit 必须落审计行");
    let after = log
        .after_snapshot
        .as_ref()
        .expect("after_snapshot 必须存在");
    assert!(
        after
            .0
            .get("description")
            .map(|x| x.is_null())
            .unwrap_or(false),
        "审计 after_snapshot 必须反映清空后的真实 NULL: {}",
        after.0
    );

    // 清空后再缺席更新：值保持 NULL（缺席 ≠ 恢复旧值）
    let kept = service
        .update(
            child.id,
            9101,
            department_handler::UpdateDepartmentRequest {
                name: Some(Some(format!("三态子部门改名{suffix}"))),
                code: None,
                description: None,
                parent_id: None,
                manager_id: None,
                sort_order: None,
                is_active: None,
            },
        )
        .await
        .unwrap();
    assert!(kept.description.is_none());
    assert!(kept.parent_id.is_none());
    assert_eq!(kept.name, format!("三态子部门改名{suffix}"));
}
