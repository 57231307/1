//! 合同 wave2 收口：合同编号不可经 update 改写 + 合同 remark 落库回读 + 供应商联系人越权门控
//!
//! 锁定的真实行为（对应本轮三项修复）：
//! 1. update 链路不再携带 contract_no（单据号由系统生成，禁手打禁改写）：
//!    - DTO 层：请求 JSON 里的 contract_no 反序列化后无处传播（编译期即无该字段）；
//!    - 活库层：PUT 携带"已存在的他人编号"不再触发生成 23505 裸 500，编号保持原值。
//!    file: `handlers/sales_contract_handler.rs` UpdateSalesContractDto、
//!          `handlers/purchase_contract_handler.rs` UpdateContractDto、
//!          `services/sales_contract_service.rs`/`services/purchase_contract_service.rs` update。
//! 2. remark 列（m0016 迁移补列）：create 落库、update 覆写、详情/列表回读不为 NULL。
//!    file: `models/sales_contract.rs`/`models/purchase_contract.rs`、
//!          `migration/src/domain/business/m0016_add_contract_remark.rs`。
//! 3. 供应商联系人四端点与资质同形门控：list/create/update/delete 先经父供应商
//!    data_scope 归属门控；update/delete 的 service 双键校验错配
//!    `(supplier_id, contact_id)` → business_displayable（400 + 用户可见文案，不脱敏）。
//!    file: `handlers/supplier_handler.rs:204-296`、`services/supplier_service.rs`
//!          update/delete_supplier_contact。
//!
//! 覆盖策略（对齐 contract_wave1_data_scope_idor_test.rs 范式）：
//! - 真 PostgreSQL（TEST_DATABASE_URL + 迁移建表，夹具清空业务表）：DTO 形状锁（serde 无 DB）、
//!   create remark 落库回读、联系人门控 403/错配 400（mismatch 路径在 lock/audit 之前返回）。
//!   suppliers 属迁移播种参照表**不清空**（m0015 已占 id 1/2），用例自建专属供应商行
//!   （serial 发号 + 唯一编码）并按返回 id 断言；supplier_contacts 为业务表，TRUNCATE
//!   RESTART IDENTITY 后显式 id=10 稳定。
//! - `#[ignore]` 活库（TEST_DATABASE_URL→PG，CI job ci-test-rust-ignored 执行）：
//!   update 全链路（lock_exclusive + update_with_audit）——
//!   编号重复请求被忽略且零 500、remark update 覆写回读、联系人删除错配 400/owner 200。

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
    purchase_contract_handler, sales_contract_handler, supplier_handler,
};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::{purchase_contract, sales_contract, supplier, supplier_contact};
use bingxi_backend::services::purchase_contract_service::{
    CreateContractRequest, PurchaseContractService,
};
use bingxi_backend::services::sales_contract_service::{
    CreateSalesContractRequest, SalesContractService,
};
use bingxi_backend::services::supplier_service::{
    CreateContactRequest, CreateSupplierRequest, SupplierService, UpdateContactRequest,
};
use bingxi_backend::utils::error::AppError;
use chrono::{TimeZone, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection};
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

async fn seeded_db() -> DatabaseConnection {
    test_common::setup_test_db().await
}

async fn seed_customer(db: &DatabaseConnection) -> i32 {
    let c = bingxi_backend::models::customer::ActiveModel {
        customer_code: Set("CUS-W2-001".to_string()),
        customer_name: Set("合同波次客户".to_string()),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
        customer_type: Set("retail".to_string()),
        owner_id: Set(9101),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    c.id
}

/// suppliers 为迁移播种参照表（不清空、m0015 已占 id 1/2）：用例自建专属行，
/// 编码带纳秒后缀避开 supplier_code UNIQUE 与存量/其他用例冲突。
/// 返回 (owner=9101 的供应商 id, owner=9102 的供应商 id)
async fn seed_two_suppliers(db: &DatabaseConnection) -> (i32, i32) {
    let suffix = Utc::now().timestamp_nanos_opt().expect(
        "测试造数取当前时刻纳秒：Utc::now 必落在 chrono 纳秒可表示区间（约1678-2262 年），None 不可达",
    );
    let mut ids = Vec::new();
    for (n, owner) in [(1i32, 9101i32), (2, 9102)] {
        let s = supplier::ActiveModel {
            supplier_code: Set(format!("SUP-W2-{suffix}-{n}")),
            supplier_name: Set(format!("合同波次供应商{suffix}-{n}")),
            supplier_short_name: Set(String::new()),
            supplier_type: Set("普通供应商".to_string()),
            credit_code: Set(String::new()),
            registered_address: Set(String::new()),
            legal_representative: Set(String::new()),
            registered_capital: Set(Decimal::ZERO),
            establishment_date: Set(Utc
                .with_ymd_and_hms(2020, 1, 1, 0, 0, 0)
                .unwrap()
                .date_naive()),
            taxpayer_type: Set("一般纳税人".to_string()),
            bank_name: Set(String::new()),
            bank_account: Set(String::new()),
            contact_phone: Set(String::new()),
            created_at: Set(Utc::now().into()),
            updated_at: Set(Utc::now().into()),
            created_by: Set(Some(owner)),
            is_processor: Set(false),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
        ids.push(s.id);
    }
    (ids[0], ids[1])
}

/// seed 联系人 id=10（supplier_contacts 是业务表，TRUNCATE RESTART IDENTITY 后显式 id 稳定），
/// 归属指定供应商
async fn seed_contact_under_supplier(db: &DatabaseConnection, supplier_id: i32) {
    let now = Utc::now();
    supplier_contact::ActiveModel {
        id: Set(10),
        supplier_id: Set(supplier_id),
        contact_name: Set("主联系人".to_string()),
        mobile_phone: Set("13800000000".to_string()),
        is_primary: Set(true),
        created_at: Set(now.into()),
        updated_at: Set(now.into()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

fn supplier_contacts_router(db: DatabaseConnection, auth: AuthContext) -> Router {
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    Router::new()
        .route(
            "/suppliers/{supplier_id}/contacts",
            axum::routing::get(supplier_handler::list_supplier_contacts)
                .post(supplier_handler::create_supplier_contact),
        )
        .route(
            "/suppliers/{supplier_id}/contacts/{contact_id}",
            axum::routing::put(supplier_handler::update_supplier_contact)
                .delete(supplier_handler::delete_supplier_contact),
        )
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth))
}

// =========================================================
// A) DTO 形状锁：update 请求不再有 contract_no 通道
// =========================================================

/// 销售合同 update：JSON 里携带的 contract_no 反序列化后无落点（DTO 已移除该字段），
/// 序列化回传也只含 remark 等真实可更新字段——编号改写通道被编译期封死。
#[test]
fn sales_update_dto_has_no_contract_no_channel() {
    let dto: sales_contract_handler::UpdateSalesContractDto = serde_json::from_value(json!({
        "contract_no": "SC-EXISTING-DUP",
        "contract_name": "改名后的合同",
        "remark": "波次备注"
    }))
    .expect("update DTO 反序列化失败");
    let v = serde_json::to_value(&dto).unwrap();
    assert!(
        v.get("contract_no").is_none(),
        "update DTO 不应存在 contract_no 字段"
    );
    assert_eq!(v["contract_name"], "改名后的合同");
    assert_eq!(v["remark"], "波次备注");
}

/// 采购合同 update：同上
#[test]
fn purchase_update_dto_has_no_contract_no_channel() {
    let dto: purchase_contract_handler::UpdateContractDto = serde_json::from_value(json!({
        "contract_no": "PC-EXISTING-DUP",
        "contract_name": "改名后的合同",
        "remark": "波次备注"
    }))
    .expect("update DTO 反序列化失败");
    let v = serde_json::to_value(&dto).unwrap();
    assert!(
        v.get("contract_no").is_none(),
        "update DTO 不应存在 contract_no 字段"
    );
    assert_eq!(v["contract_name"], "改名后的合同");
    assert_eq!(v["remark"], "波次备注");
}

/// create DTO 仍必须收 contract_no 与 remark（只收紧 update，不放开 create）
#[test]
fn create_dtos_still_carry_contract_no_and_remark() {
    let dto: sales_contract_handler::CreateSalesContractRequestDto =
        serde_json::from_value(json!({
            "contract_no": "SC20260920001",
            "contract_name": "销售合同",
            "customer_id": 1,
            "total_amount": "1000.00",
            "remark": "创建备注"
        }))
        .expect("sales create DTO 反序列化失败");
    assert_eq!(dto.contract_no, "SC20260920001");
    assert_eq!(dto.remark.as_deref(), Some("创建备注"));
}

// =========================================================
// B) 真 PG：create 的 remark 真实落库并回读非 NULL
// =========================================================

#[tokio::test]
async fn sales_create_persists_remark_and_reads_back() {
    let db = seeded_db().await;
    let cid = seed_customer(&db).await;

    let service = SalesContractService::new(Arc::new(db.clone()));
    let created = service
        .create(
            CreateSalesContractRequest {
                contract_no: "SC-W2-PG-001".to_string(),
                contract_name: "备注落库验证".to_string(),
                customer_id: cid,
                total_amount: Decimal::from(100),
                contract_type: None,
                payment_terms: None,
                delivery_date: None,
                signed_date: None,
                effective_date: None,
                expiry_date: None,
                payment_method: None,
                delivery_location: None,
                remark: Some("这是备注，不得被静默丢弃".to_string()),
                items: None,
            },
            9101,
        )
        .await
        .expect("sales create 失败");
    assert_eq!(created.remark.as_deref(), Some("这是备注，不得被静默丢弃"));

    // 独立回读（不复用 insert 返回值）：行内存的 remark 必须非 NULL
    let reread = sales_contract::Entity::find_by_id(created.id)
        .one(&db)
        .await
        .unwrap()
        .expect("回读销售合同失败");
    assert_eq!(reread.remark.as_deref(), Some("这是备注，不得被静默丢弃"));
}

#[tokio::test]
async fn purchase_create_persists_remark_and_reads_back() {
    let db = seeded_db().await;
    let (sup1, _sup2) = seed_two_suppliers(&db).await;

    let service = PurchaseContractService::new(Arc::new(db.clone()));
    let created = service
        .create(
            CreateContractRequest {
                contract_no: "PC-W2-PG-001".to_string(),
                contract_name: "采购备注落库验证".to_string(),
                supplier_id: sup1,
                total_amount: Decimal::from(200),
                contract_type: None,
                payment_terms: None,
                delivery_date: None,
                signed_date: None,
                effective_date: None,
                expiry_date: None,
                payment_method: None,
                delivery_location: None,
                remark: Some("采购备注文本".to_string()),
            },
            9101,
        )
        .await
        .expect("purchase create 失败");
    assert_eq!(created.remark.as_deref(), Some("采购备注文本"));

    let reread = purchase_contract::Entity::find_by_id(created.id)
        .one(&db)
        .await
        .unwrap()
        .expect("回读采购合同失败");
    assert_eq!(reread.remark.as_deref(), Some("采购备注文本"));
}

// =========================================================
// C) 真 PG：供应商联系人门控（父归属 403 + 双键错配可见业务错误）
// =========================================================

/// 播种并组装联系人路由；返回 (router, 供应商1[owner 9101], 供应商2[owner 9102])，
/// 供应商 id 来自真表发号（suppliers 参照表不清空，不能假设固定 id）
async fn seeded_contacts_app(viewer: i32) -> (Router, i32, i32) {
    let db = seeded_db().await;
    let (sup1, sup2) = seed_two_suppliers(&db).await;
    seed_contact_under_supplier(&db, sup1).await;
    (
        supplier_contacts_router(db, make_scope_auth(viewer, Some("self"))),
        sup1,
        sup2,
    )
}

/// 非父供应商 owner（self 范围）列他人联系人 → 403（修复前：无 AuthContext，任意枚举）
#[tokio::test]
async fn contacts_list_non_owner_403() {
    let (app, sup1, _sup2) = seeded_contacts_app(9102).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/suppliers/{sup1}/contacts"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "实际: {v}");
    assert_eq!(v["code"], "FORBIDDEN");
}

/// owner 列自家联系人 → 200 且真实返回行（防止把门控修成一律 403）
#[tokio::test]
async fn contacts_list_owner_200_real_rows() {
    let (app, sup1, _sup2) = seeded_contacts_app(9101).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/suppliers/{sup1}/contacts"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "实际: {v}");
    assert_eq!(v["data"].as_array().unwrap().len(), 1);
    assert_eq!(v["data"][0]["id"], 10);
}

/// 父供应商不存在 → 404（门控经 get_supplier 如实上抛）
#[tokio::test]
async fn contacts_list_missing_supplier_404() {
    let (app, _sup1, _sup2) = seeded_contacts_app(9101).await;
    let (status, v) = call(&app, Method::GET, "/suppliers/999/contacts", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "实际: {v}");
    assert_eq!(v["code"], "NOT_FOUND");
}

/// 双键错配：以供应商 2 的 owner 身份改供应商 1 的联系人 →
/// 父门控通过、service 归属校验拒绝，400 + 用户可见文案（不得脱敏成"业务处理失败"、不得 500）
#[tokio::test]
async fn contacts_update_mismatched_pair_400_visible_message() {
    let (app, _sup1, sup2) = seeded_contacts_app(9102).await;
    let (status, v) = call(
        &app,
        Method::PUT,
        &format!("/suppliers/{sup2}/contacts/10"),
        Some(json!({ "contact_name": "越权改名" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "实际: {v}");
    assert_eq!(v["code"], "BUSINESS_ERROR");
    assert_eq!(
        v["message"],
        "该联系人不属于此供应商，无法更新，请刷新后重试"
    );
    assert_ne!(
        v["message"], "业务处理失败",
        "business_displayable 不得被脱敏"
    );
}

/// service 单键直调也必须拒绝错配（门控在 handler 之外的第二道防线）
#[tokio::test]
async fn contact_update_service_level_mismatch_rejected() {
    let db = seeded_db().await;
    let (sup1, sup2) = seed_two_suppliers(&db).await;
    seed_contact_under_supplier(&db, sup1).await;

    let service = SupplierService::new(Arc::new(db.clone()));
    let err = service
        .update_supplier_contact(
            sup2,
            10,
            UpdateContactRequest {
                contact_name: Some("越权改名".to_string()),
                department: None,
                position: None,
                mobile_phone: None,
                tel_phone: None,
                email: None,
                wechat: None,
                qq: None,
                is_primary: None,
                remarks: None,
            },
            9102,
        )
        .await
        .expect_err("错配 (supplier_id, contact_id) 必须被拒绝");
    assert!(
        matches!(&err, AppError::BusinessErrorDisplayable(m) if m.contains("该联系人不属于此供应商")),
        "期望可见业务错误，实际: {err:?}"
    );
}

// =========================================================
// D) 活库（PG）：update 全链路（lock_exclusive + audit）——
//    编号重复请求被忽略零 500 + remark 覆写回读 + 联系人删除门控
// =========================================================

#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL（service.update 走 lock_exclusive + update_with_audit）"]
async fn live_sales_update_ignores_duplicate_contract_no_and_persists_remark() {
    let db = test_common::setup_test_db().await;
    let suffix = Utc::now().timestamp_nanos_opt().unwrap();
    let cust = bingxi_backend::models::customer::ActiveModel {
        customer_code: Set(format!("CUS-W2L-{suffix}")),
        customer_name: Set(format!("活库波次客户{suffix}")),
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
    let mk = |no: &str| CreateSalesContractRequest {
        contract_no: no.to_string(),
        contract_name: "活库合同".to_string(),
        customer_id: cust.id,
        total_amount: Decimal::from(100),
        contract_type: None,
        payment_terms: None,
        delivery_date: None,
        signed_date: None,
        effective_date: None,
        expiry_date: None,
        payment_method: None,
        delivery_location: None,
        remark: None,
        items: None,
    };
    let a = service
        .create(mk(&format!("SC-W2L-A-{suffix}")), 9101)
        .await
        .unwrap();
    let _b = service
        .create(mk(&format!("SC-W2L-B-{suffix}")), 9101)
        .await
        .unwrap();

    // PUT：携带已存在的他人编号 + remark——修复前该形态要么撞 UNIQUE 裸 500，要么 remark 丢失
    let dup_no = format!("SC-W2L-B-{suffix}");
    let app = {
        let state = AppState {
            db: Arc::new(db.clone()),
            ..Default::default()
        };
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
    };
    let uri = format!("/sales-contracts/{}", a.id);
    let (status, v) = call(
        &app,
        Method::PUT,
        &uri,
        Some(json!({
            "contract_no": dup_no,
            "contract_name": "改名后的活库合同",
            "remark": "活库备注不得丢失"
        })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "update 不应因编号撞 UNIQUE 裸 500: {v}"
    );
    assert_eq!(
        v["data"]["contract_no"],
        format!("SC-W2L-A-{suffix}"),
        "编号必须保持原值"
    );
    assert_eq!(v["data"]["contract_name"], "改名后的活库合同");
    assert_eq!(v["data"]["remark"], "活库备注不得丢失");

    // DB 回读：编号未被改写、remark 非 NULL
    let reread = sales_contract::Entity::find_by_id(a.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(reread.contract_no, format!("SC-W2L-A-{suffix}"));
    assert_eq!(reread.remark.as_deref(), Some("活库备注不得丢失"));
}

#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL（service.update 走 lock_exclusive + update_with_audit）"]
async fn live_purchase_update_ignores_duplicate_contract_no_and_persists_remark() {
    let db = test_common::setup_test_db().await;
    let suffix = Utc::now().timestamp_nanos_opt().unwrap();
    let supplier_service = SupplierService::new(Arc::new(db.clone()));
    let sup = supplier_service
        .create_supplier(
            CreateSupplierRequest {
                supplier_name: format!("活库波次供应商{suffix}"),
                supplier_short_name: None,
                supplier_type: None,
                credit_code: None,
                registered_address: None,
                business_address: None,
                legal_representative: None,
                registered_capital: None,
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
                annual_revenue: None,
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
    let mk = |no: &str| CreateContractRequest {
        contract_no: no.to_string(),
        contract_name: "活库采购合同".to_string(),
        supplier_id: sup.id,
        total_amount: Decimal::from(100),
        contract_type: None,
        payment_terms: None,
        delivery_date: None,
        signed_date: None,
        effective_date: None,
        expiry_date: None,
        payment_method: None,
        delivery_location: None,
        remark: None,
    };
    let a = service
        .create(mk(&format!("PC-W2L-A-{suffix}")), 9101)
        .await
        .unwrap();
    let _b = service
        .create(mk(&format!("PC-W2L-B-{suffix}")), 9101)
        .await
        .unwrap();

    let app = {
        let state = AppState {
            db: Arc::new(db.clone()),
            ..Default::default()
        };
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
    };
    let uri = format!("/purchase-contracts/{}", a.id);
    let (status, v) = call(
        &app,
        Method::PUT,
        &uri,
        Some(json!({
            "contract_no": format!("PC-W2L-B-{suffix}"),
            "remark": "采购活库备注"
        })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "update 不应因编号撞 UNIQUE 裸 500: {v}"
    );
    assert_eq!(v["data"]["contract_no"], format!("PC-W2L-A-{suffix}"));
    assert_eq!(v["data"]["remark"], "采购活库备注");
}

/// 联系人删除：错配 (supplier_id, contact_id) → 400 可见文案；owner 正确配对 → 200
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL（service.delete 走 lock_exclusive + delete_with_audit）"]
async fn live_contact_delete_mismatch_400_owner_200() {
    let db = test_common::setup_test_db().await;
    let suffix = Utc::now().timestamp_nanos_opt().unwrap();
    let service = SupplierService::new(Arc::new(db.clone()));
    let s1 = service
        .create_supplier(
            CreateSupplierRequest {
                supplier_name: format!("联系人门控供应商A{suffix}"),
                supplier_short_name: None,
                supplier_type: None,
                credit_code: None,
                registered_address: None,
                business_address: None,
                legal_representative: None,
                registered_capital: None,
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
                annual_revenue: None,
                category_id: None,
                is_processor: None,
                processor_type: None,
                contacts: None,
                qualifications: None,
            },
            9601,
        )
        .await
        .unwrap();
    let s2 = service
        .create_supplier(
            CreateSupplierRequest {
                supplier_name: format!("联系人门控供应商B{suffix}"),
                supplier_short_name: None,
                supplier_type: None,
                credit_code: None,
                registered_address: None,
                business_address: None,
                legal_representative: None,
                registered_capital: None,
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
                annual_revenue: None,
                category_id: None,
                is_processor: None,
                processor_type: None,
                contacts: None,
                qualifications: None,
            },
            9602,
        )
        .await
        .unwrap();
    let contact = service
        .create_supplier_contact(
            s1.id,
            CreateContactRequest {
                contact_name: "待删联系人".to_string(),
                department: None,
                position: None,
                mobile_phone: "13900000000".to_string(),
                tel_phone: None,
                email: None,
                wechat: None,
                qq: None,
                is_primary: false,
                remarks: None,
            },
            9601,
        )
        .await
        .unwrap();

    // 以 s2 owner 身份删 s1 的联系人：父门控通过、双键错配 → 400 可见业务错误
    let app = supplier_contacts_router(db.clone(), make_scope_auth(9602, Some("self")));
    let uri = format!("/suppliers/{}/contacts/{}", s2.id, contact.id);
    let (status, v) = call(&app, Method::DELETE, &uri, None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "错配删除应 400: {v}");
    assert_eq!(v["code"], "BUSINESS_ERROR");
    assert_eq!(
        v["message"],
        "该联系人不属于此供应商，无法删除，请刷新后重试"
    );

    // s1 owner 正确配对 → 200，且行确实消失
    let app_owner = supplier_contacts_router(db.clone(), make_scope_auth(9601, Some("self")));
    let uri_owner = format!("/suppliers/{}/contacts/{}", s1.id, contact.id);
    let (status, v) = call(&app_owner, Method::DELETE, &uri_owner, None).await;
    assert_eq!(status, StatusCode::OK, "owner 删除应 200: {v}");
    let gone = supplier_contact::Entity::find_by_id(contact.id)
        .one(&db)
        .await
        .unwrap();
    assert!(gone.is_none(), "删除后行应不存在");
}
