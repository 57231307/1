//! 契约锁：库存预留引用存在性预检 + 凭证借贷不平衡装配族一致性
//! + 委外订单 DbErr 重包装防回潮（先例：contract_wave2_reservation_error_mapping_test.rs /
//! contract_wave3_state_gate_family_consistency_test.rs）
//!
//! 锁定三条契约：
//! 1. `inventory_reservation_service.rs::create_reservation` 写库前显式预检
//!    order_id/product_id/warehouse_id 三个引用（照 purchase_inspection_service.rs 的
//!    receipt_id 预检、sku_mapping_service.rs 的 validate_refs 范式）：
//!    - 引用行不存在 → 404 NOT_FOUND（含 ID 的真实原因走脱敏 not_found，出参恒「资源未找到」）；
//!      预检先于触库，坏引用不得直达 DB 外键经 From<DbErr> Exec 分支落 DATABASE_ERROR/500；
//!    - 软删/停用视同不存在：products.is_deleted=true 或 status≠active、warehouses.is_active=false
//!      （判定依据逐列对照 models/product.rs、models/warehouse.rs；sales_orders 无软删/停用列，
//!      存在性=行存在，cancelled/rejected 是终态而非"不存在"，不并入本预检）；
//!    - 拒绝必须零脏行；正向合法引用创建成功并可回读。
//! 表结构唯一来源 = backend/migration，
//!    inventory_reservations 的三个真外键（fk_inventory_reservations_order/product/
//!    warehouse，m0010:63-65）同样存在：本用例锁的是「应用层预检先行」——
//!    坏引用在触库前即被 404 拒绝（若预检被删，才会以 23503 FK 落 DATABASE_ERROR/500）；
//!    契约出参口径恒为 404 + 零脏行。
//! 2. `voucher_ops/workflow.rs` 借贷不平衡拒绝必须复用 `crud.rs` 的唯一装配点
//!    `balance_error`（VALIDATION 族 + 脱敏出参「请求参数验证失败」），不得另装配
//!    BAD_REQUEST——同语义与 crud.rs create/update 分属两族会让前端按 code 分支不一致。
//!    workflow 状态门带 lock_exclusive（PG 行锁），活库行为用例在真 PG 直接真跑
//!    （不带 `#[ignore]`）；族一致性另配源码扫描锁双保险。
//! 3. `outsourcing_ops/order.rs` 不得再把 SeaORM DbErr 重包装成 DatabaseError 并拼
//!    错误原文（{e} 含约束名/列名）——一律经 From<DbErr> 统一分类，真实原因只进
//!    tracing::error，出参脱敏。源码扫描锁固化。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::{IntoResponse, Response},
    routing::post,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::inventory_reservation_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::{customer, product, sales_order, user, warehouse};
use bingxi_backend::services::inventory_reservation_service::InventoryReservationService;
use bingxi_backend::utils::error::AppError;
use bingxi_backend::utils::messages::err_msg;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, EntityTrait, PaginatorTrait, Set};
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;

// =========================================================
// 1) 库存预留引用预检（真 PG 行为锁；表结构唯一来源 = backend/migration）
// =========================================================

/// 真库 FK 父行自插：`sales_orders.customer_id NOT NULL` 有
/// fk_sales_orders_customer→customers、fk_sales_orders_created_by→users（m0001:632-633），
/// 而 customers/users 属会被清空且不播种的业务表 ⇒ 用例必须自带合法父行，
/// 不指望环境已有数据。链：users(7) → customers(1) → sales_orders(1)。
async fn seed_users_and_customer(db: &sea_orm::DatabaseConnection) {
    let now = Utc::now();
    user::ActiveModel {
        id: Set(7),
        username: Set("w5_precheck_user".to_string()),
        password_hash: Set("test-only-not-a-real-hash".to_string()),
        real_name: Set(Some("波5预检夹具用户".to_string())),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("seed 归属用户失败（users 无迁移播种，必须自插）");
    customer::ActiveModel {
        id: Set(1),
        customer_code: Set("W5C-0001".to_string()),
        customer_name: Set("波5预检夹具客户".to_string()),
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
    .expect("seed 客户失败（customers 无迁移播种，必须自插）");
}

async fn seed_order(db: &sea_orm::DatabaseConnection, id: i32) {
    let now = Utc::now();
    sales_order::ActiveModel {
        id: Set(id),
        order_no: Set(format!("SO-W5-{id}")),
        // 真实父行：customers.id=1 / users.id=7（见 seed_users_and_customer）
        customer_id: Set(1),
        order_date: Set(now),
        required_date: Set(Some(now)),
        status: Set("approved".to_string()),
        subtotal: Set(Decimal::ZERO),
        tax_amount: Set(Decimal::ZERO),
        discount_amount: Set(Decimal::ZERO),
        shipping_cost: Set(Decimal::ZERO),
        total_amount: Set(Decimal::ZERO),
        paid_amount: Set(Decimal::ZERO),
        balance_amount: Set(Decimal::ZERO),
        created_by: Set(Some(7)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("seed 销售订单失败");
}

async fn seed_product(db: &sea_orm::DatabaseConnection, id: i32, status: &str, is_deleted: bool) {
    let now = Utc::now();
    product::ActiveModel {
        id: Set(id),
        name: Set(format!("W5 产品 {id}")),
        code: Set(format!("W5P{id}")),
        unit: Set("米".to_string()),
        status: Set(status.to_string()),
        is_deleted: Set(is_deleted),
        product_type: Set("成品布".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("seed 产品失败");
}

async fn seed_warehouse(db: &sea_orm::DatabaseConnection, id: i32, is_active: bool) {
    let now = Utc::now();
    warehouse::ActiveModel {
        id: Set(id),
        warehouse_code: Set(format!("W5WH{id}")),
        name: Set(format!("W5 仓库 {id}")),
        is_default: Set(false),
        is_active: Set(is_active),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("seed 仓库失败");
}

/// 全合法引用底：users(7)/customers(1)/order=1 / product=2(active,未删) / warehouse=3(active)
async fn seeded_db() -> sea_orm::DatabaseConnection {
    let db = test_common::setup_test_db().await;
    seed_users_and_customer(&db).await;
    seed_order(&db, 1).await;
    seed_product(&db, 2, "active", false).await;
    seed_warehouse(&db, 3, true).await;
    db
}

async fn reservation_count(db: &sea_orm::DatabaseConnection) -> u64 {
    use bingxi_backend::models::inventory_reservation::Entity as RE;
    RE::find().count(db).await.expect("统计预留行数失败")
}

/// 断言一次拒绝的完整出参形态：404 + NOT_FOUND + 脱敏常量，显式排除 500/DATABASE_ERROR
async fn assert_not_found_rejection(err: AppError, ctx: &str) {
    assert_eq!(
        err.error_code(),
        "NOT_FOUND",
        "{ctx}：引用预检必须归 NOT_FOUND 族，实际={err:?}"
    );
    assert_ne!(
        err.error_code(),
        "DATABASE_ERROR",
        "{ctx}：禁止回潮为裸 FK 500 的 DATABASE_ERROR"
    );
    let resp = err.clone().into_response();
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "{ctx}：HTTP 状态必须 404（修复前为 500），禁止 DATABASE_ERROR 兜底"
    );
    assert_ne!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        err.to_response().message,
        err_msg::NOT_FOUND_PUBLIC,
        "{ctx}：出参走脱敏常量"
    );
}

/// 不存在的 order_id → 404 NOT_FOUND（不得直达 DB 外键成 DATABASE_ERROR/500）
#[tokio::test]
async fn create_reservation_missing_order_is_404_not_500() {
    let db = seeded_db().await;
    let svc = InventoryReservationService::new(Arc::new(db.clone()));
    let err = svc
        .create_reservation(999, 2, 3, Decimal::from(5), Some(7), None)
        .await
        .expect_err("order 引用不存在必须被预检拒绝");
    assert_not_found_rejection(err, "缺失 order 引用").await;
    assert_eq!(reservation_count(&db).await, 0, "拒绝后必须零脏行");
}

/// 不存在的 product_id → 404 NOT_FOUND + 零脏行
#[tokio::test]
async fn create_reservation_missing_product_is_404_not_500() {
    let db = seeded_db().await;
    let svc = InventoryReservationService::new(Arc::new(db.clone()));
    let err = svc
        .create_reservation(1, 999, 3, Decimal::from(5), Some(7), None)
        .await
        .expect_err("product 引用不存在必须被预检拒绝");
    assert_not_found_rejection(err, "缺失 product 引用").await;
    assert_eq!(reservation_count(&db).await, 0, "拒绝后必须零脏行");
}

/// 不存在的 warehouse_id → 404 NOT_FOUND + 零脏行
#[tokio::test]
async fn create_reservation_missing_warehouse_is_404_not_500() {
    let db = seeded_db().await;
    let svc = InventoryReservationService::new(Arc::new(db.clone()));
    let err = svc
        .create_reservation(1, 2, 999, Decimal::from(5), Some(7), None)
        .await
        .expect_err("warehouse 引用不存在必须被预检拒绝");
    assert_not_found_rejection(err, "缺失 warehouse 引用").await;
    assert_eq!(reservation_count(&db).await, 0, "拒绝后必须零脏行");
}

/// 软删产品（is_deleted=true）视同不存在 → 404（判定依据：models/product.rs is_deleted 列）
#[tokio::test]
async fn create_reservation_soft_deleted_product_treated_as_missing() {
    let db = seeded_db().await;
    seed_product(&db, 20, "active", true).await;
    let svc = InventoryReservationService::new(Arc::new(db.clone()));
    let err = svc
        .create_reservation(1, 20, 3, Decimal::from(5), Some(7), None)
        .await
        .expect_err("已软删产品必须视同不存在");
    assert_not_found_rejection(err, "软删产品引用").await;
    assert_eq!(reservation_count(&db).await, 0, "拒绝后必须零脏行");
}

/// 停用产品（status=inactive）视同不存在 → 404（判定依据：models/product.rs 注释
/// 「active-启用，inactive-停用」，token 对齐 models::status::master_data::INACTIVE）
#[tokio::test]
async fn create_reservation_inactive_product_treated_as_missing() {
    let db = seeded_db().await;
    seed_product(&db, 21, "inactive", false).await;
    let svc = InventoryReservationService::new(Arc::new(db.clone()));
    let err = svc
        .create_reservation(1, 21, 3, Decimal::from(5), Some(7), None)
        .await
        .expect_err("已停用产品必须视同不存在");
    assert_not_found_rejection(err, "停用产品引用").await;
    assert_eq!(reservation_count(&db).await, 0, "拒绝后必须零脏行");
}

/// 停用仓库（is_active=false）视同不存在 → 404（判定依据：models/warehouse.rs 无软删列，
/// is_active 即停用标志）
#[tokio::test]
async fn create_reservation_inactive_warehouse_treated_as_missing() {
    let db = seeded_db().await;
    seed_warehouse(&db, 30, false).await;
    let svc = InventoryReservationService::new(Arc::new(db.clone()));
    let err = svc
        .create_reservation(1, 2, 30, Decimal::from(5), Some(7), None)
        .await
        .expect_err("已停用仓库必须视同不存在");
    assert_not_found_rejection(err, "停用仓库引用").await;
    assert_eq!(reservation_count(&db).await, 0, "拒绝后必须零脏行");
}

/// 正向：三个引用均合法 → 创建成功并可回读（预检不得误杀合法路径）
#[tokio::test]
async fn create_reservation_with_valid_refs_succeeds_and_readable() {
    let db = seeded_db().await;
    let svc = InventoryReservationService::new(Arc::new(db.clone()));
    let created = svc
        .create_reservation(
            1,
            2,
            3,
            Decimal::from(7),
            Some(7),
            Some("wave5 正向".to_string()),
        )
        .await
        .expect("合法引用创建预留必须成功（预检不得误杀）");
    assert_eq!(created.order_id, 1);
    assert_eq!(created.product_id, 2);
    assert_eq!(created.warehouse_id, 3);
    let reread = bingxi_backend::models::inventory_reservation::Entity::find_by_id(created.id)
        .one(&db)
        .await
        .expect("回读失败")
        .expect("回读为空：创建未真实落库？");
    assert_eq!(reread.quantity, Decimal::from(7));
    assert_eq!(reread.status, "pending");
    assert_eq!(reservation_count(&db).await, 1);
}

/// handler 级端到端（POST 真实 JSON 出参）：坏 order 引用 → 404 NOT_FOUND，
/// 锁不得成 500 "数据库错误"（23503 不得经 From<DbErr> 裸落 DATABASE_ERROR）
#[tokio::test]
async fn create_reservation_handler_returns_404_envelope_for_missing_refs() {
    fn make_auth(user_id: i32) -> AuthContext {
        AuthContext {
            user_id,
            username: format!("w5_user_{user_id}"),
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

    let db = seeded_db().await;
    let state = AppState {
        db: Arc::new(db.clone()),
        ..Default::default()
    };
    let app = Router::new()
        .route(
            "/inventory/reservations",
            post(inventory_reservation_handler::create_reservation),
        )
        .with_state(state)
        .layer(from_fn_with_state(make_auth(7), inject_auth));

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/inventory/reservations")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({
                        "order_id": 999, "product_id": 2, "warehouse_id": 3, "quantity": "5.00"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let v: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(status, StatusCode::NOT_FOUND, "实际出参: {v}");
    assert_eq!(v["code"], "NOT_FOUND");
    assert_ne!(v["code"], "DATABASE_ERROR");
    assert_ne!(
        v["message"],
        err_msg::DB_ERROR_PUBLIC,
        "不得再出裸「数据库错误」"
    );
    assert_eq!(v["message"], err_msg::NOT_FOUND_PUBLIC);
    assert_eq!(reservation_count(&db).await, 0, "拒绝后必须零脏行");
}

// =========================================================
// 2) 凭证 workflow 借贷不平衡：VALIDATION 族 + 脱敏（真库用例直接真跑）
// =========================================================

/// 借贷不平衡凭证种子（draft 或指定状态；分录借 100 / 贷 99，绕开 create 校验直插）
async fn seed_unbalanced_voucher(
    db: &sea_orm::DatabaseConnection,
    status: &str,
) -> (i32, Vec<i32>) {
    use bingxi_backend::models::{voucher, voucher_item};
    let now = Utc::now();
    let v = voucher::ActiveModel {
        voucher_no: Set(format!(
            "W5VB-{}",
            now.timestamp_nanos_opt().unwrap_or_default()
        )),
        voucher_type: Set("记".to_string()),
        voucher_date: Set(NaiveDate::from_ymd_opt(2026, 1, 15).expect("静态日期")),
        status: Set(status.to_string()),
        attachment_count: Set(0),
        created_by: Set(1),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("seed 凭证失败（需已跑迁移的活库）");
    let mut ids = Vec::new();
    for (line, debit, credit) in [
        (1, Decimal::from(100), Decimal::from(99)),
        (2, Decimal::ZERO, Decimal::ZERO),
    ] {
        let item = voucher_item::ActiveModel {
            voucher_id: Set(v.id),
            line_no: Set(line),
            subject_code: Set(if line == 1 { "1001" } else { "1002" }.to_string()),
            subject_name: Set(if line == 1 {
                "库存现金"
            } else {
                "银行存款"
            }
            .to_string()),
            debit: Set(debit),
            credit: Set(credit),
            summary: Set(Some("wave5 不平种子".to_string())),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .expect("seed 凭证分录失败（需已跑迁移的活库）");
        ids.push(item.id);
    }
    (v.id, ids)
}

async fn cleanup_voucher(db: &sea_orm::DatabaseConnection, voucher_id: i32, item_ids: &[i32]) {
    use bingxi_backend::models::{voucher, voucher_item};
    for iid in item_ids {
        voucher_item::Entity::delete_by_id(*iid)
            .exec(db)
            .await
            .expect("清理凭证分录失败");
    }
    voucher::Entity::delete_by_id(voucher_id)
        .exec(db)
        .await
        .expect("清理凭证主表失败");
}

fn assert_balance_validation_family(err: AppError) {
    assert!(
        matches!(err, AppError::ValidationError(_)),
        "借贷不平衡必须归脱敏 VALIDATION 族（不是 bad_request/BAD_REQUEST，也不是 Displayable 外显），实际={err:?}"
    );
    assert_eq!(err.error_code(), "VALIDATION_ERROR");
    assert_ne!(
        err.error_code(),
        "BAD_REQUEST",
        "禁止回潮 workflow 独有的 BAD_REQUEST 族"
    );
    assert_eq!(
        err.to_response().message,
        err_msg::VALIDATION_PUBLIC,
        "出参必须等于脱敏常量「请求参数验证失败」（金额不外泄）"
    );
    assert_ne!(err.to_response().message, err_msg::BAD_REQUEST_PUBLIC);
    // 日志侧信息不丢：Display（进 tracing 的文本）仍含「借贷不平衡」与真实金额
    let display = err.to_string();
    assert!(
        display.contains("借贷不平衡"),
        "Display 侧拒绝原因不得丢失，实际: {display}"
    );
    assert!(
        display.contains("100"),
        "Display 侧须保留借方金额供排查，实际: {display}"
    );
    let resp = err.clone().into_response();
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "VALIDATION 族 HTTP 恒 400"
    );
}

/// 真库：draft 借贷不平凭证 submit → 400 VALIDATION_ERROR + 脱敏出参。
/// workflow 状态门 lock_exclusive 是 PG 行锁 ⇒ 本用例在真 PG 直接真跑（不带
/// `#[ignore]`）；连接走公共夹具 setup_test_db()（清业务表后自插种子，无环境耦合）。
#[tokio::test]
async fn voucher_submit_unbalanced_is_validation_error_redacted() {
    let db = test_common::setup_test_db().await;
    let svc = bingxi_backend::services::voucher_service::VoucherService::new(Arc::new(db.clone()));
    let (id, items) = seed_unbalanced_voucher(&db, "draft").await;
    let err = svc
        .submit(id, 1)
        .await
        .expect_err("借贷不平凭证提交必须被拒绝");
    assert_balance_validation_family(err);
    cleanup_voucher(&db, id, &items).await;
}

/// 真库：submitted 借贷不平凭证 review → 同上族/出参口径（submit 与 review 两站点同锁）
#[tokio::test]
async fn voucher_review_unbalanced_is_validation_error_redacted() {
    let db = test_common::setup_test_db().await;
    let svc = bingxi_backend::services::voucher_service::VoucherService::new(Arc::new(db.clone()));
    let (id, items) = seed_unbalanced_voucher(&db, "submitted").await;
    let err = svc
        .review(id, 1)
        .await
        .expect_err("借贷不平凭证审核必须被拒绝");
    assert_balance_validation_family(err);
    cleanup_voucher(&db, id, &items).await;
}

// =========================================================
// 3) 源码扫描锁（无 DB）
// =========================================================

/// workflow.rs 的两个借贷不平衡站点必须复用 crud.rs 唯一装配点 balance_error，
/// 且装配族两边一致（都只能是 validation，禁止一边 business/bad_request 一边 validation）
#[test]
fn source_scan_balance_error_single_validation_family_across_files() {
    let crud = include_str!("../src/services/voucher_ops/crud.rs").replace('\r', "");
    let workflow = include_str!("../src/services/voucher_ops/workflow.rs").replace('\r', "");

    // crud：balance_error 仍是唯一装配点，且只构造 validation 族
    let anchor = crud
        .find("fn balance_error")
        .expect("crud.rs 借贷不平衡唯一装配点 balance_error 不得消失");
    // 取签名起 220 字符（chars 计数避免 CJK 字节切片越界 panic），覆盖函数体即止
    let body: String = crud[anchor..].chars().take(220).collect();
    assert!(
        body.contains("AppError::validation("),
        "balance_error 必须装配 VALIDATION 族（脱敏），实际片段: {body}"
    );
    for forbidden in [
        "AppError::business",
        "AppError::bad_request",
        "AppError::not_found",
    ] {
        assert!(
            !body.contains(forbidden),
            "balance_error 装配点禁止混入 {forbidden} 族"
        );
    }

    // workflow：两个校验函数都经 Self::balance_error 复用同族同措辞，
    // 全文件不得再出现 bad_request（出现即 BAD_REQUEST 族回潮）
    assert_eq!(
        workflow.matches("Self::balance_error(").count(),
        2,
        "workflow.rs 的 validate_voucher / validate_voucher_in_transaction 两处必须复用 balance_error 唯一装配点"
    );
    assert!(
        !workflow.contains("AppError::bad_request("),
        "workflow.rs 禁止回潮 BAD_REQUEST 族装配（同语义站点与 crud 不同族 = 前端按 code 分支不一致）"
    );
    assert!(
        !workflow.contains("AppError::business("),
        "借贷不平衡不得被改成 business 族（两边族必须同为 validation）"
    );
    assert!(
        !crud.contains("AppError::bad_request(format!(\"凭证借贷不平衡"),
        "crud.rs 不得绕开 balance_error 另起 BAD_REQUEST 措辞（禁止第三套文案）"
    );
}

/// 委外订单：DbErr 不得再被重包装成 DatabaseError 并拼错误原文；
/// 全文件 AppError::database 零命中（真实原因只进 From<DbErr> 的 tracing::error）
#[test]
fn source_scan_outsourcing_order_has_no_database_rewrap() {
    let src = include_str!("../src/services/outsourcing_ops/order.rs").replace('\r', "");
    assert!(
        !src.contains("AppError::database(format!("),
        "outsourcing_ops/order.rs 禁止 AppError::database(format!(…)) 重包装 DbErr（会把含约束名/列名的错误原文拼进出参）"
    );
    assert!(
        !src.contains("AppError::database("),
        "outsourcing_ops/order.rs 禁止任何 AppError::database 手工构造，DbErr 一律走 From<DbErr> 统一分类"
    );
    assert!(
        !src.contains("AppError::internal("),
        "outsourcing_ops/order.rs 禁止把已返回 AppError 的调用再包 internal 强转 500"
    );
}

/// 预留 service：引用预检必须先于 insert（防回潮把预检挪到写库之后或删掉）
#[test]
fn source_scan_reservation_precheck_precedes_insert() {
    let src = include_str!("../src/services/inventory_reservation_service.rs").replace('\r', "");
    let insert_at = src
        .find("reservation.insert(&*self.db)")
        .expect("create_reservation 的 insert 锚点不得消失");
    for pre in [
        "sales_order::Entity::find_by_id(order_id)",
        "product::Entity::find_by_id(product_id)",
        "warehouse::Entity::find_by_id(warehouse_id)",
    ] {
        let at = src
            .find(pre)
            .unwrap_or_else(|| panic!("引用存在性预检丢失: {pre}"));
        assert!(at < insert_at, "预检 {pre} 必须位于写库 insert 之前");
    }
    // 软删/停用判定依据必须保留（视同不存在）
    assert!(
        src.contains("product_row.is_deleted || product_row.status != master_data::ACTIVE"),
        "产品软删/停用视同不存在的判定不得丢失"
    );
    assert!(
        src.contains("!warehouse_row.is_active"),
        "仓库停用视同不存在的判定不得丢失"
    );
}
