//! 销售订单交付日期「日粒度门控 + 可外显拒绝文案」契约锁
//!
//! ## 锁定对象（修复后形态）
//! - `backend/src/services/so/order_crud.rs::validate_create_preconditions`
//!   交付日期与今天比较按 `.date_naive()` 取日粒度（非时间戳），允许等于今天。
//! - 拒绝文案使用 `AppError::business_displayable`（非 `business`），出参 message 可外显。
//! - 同函数族的四处拒绝均从 `business`（脱敏为"业务处理失败"）翻正为 `business_displayable`。
//!
//! ## 判据
//! - required_date 等于今天（UTC midnight） → 必须通过，不报 400。
//! - required_date 等于昨天（UTC midnight） → 必须被拒：HTTP 400 + code=BUSINESS_ERROR
//!   + message 携带可外显文案（非"业务处理失败"）。
//! - 文案必须提到"今天"（用户可据此自助修正），不得含记录 ID 或金额。
//!
//! ## 夹具形态
//! 真库 `setup_test_db()`（已迁移 PostgreSQL、nextest db-integration 组），
//! users/customers/products/sales_orders 属逐例 TRUNCATE 的业务表。
//! 直连 `SalesService::create_order`，与 handler 通道同构。

mod test_common;

use bingxi_backend::models::product;
use bingxi_backend::models::status::master_data;
use bingxi_backend::models::{customer, user};
use bingxi_backend::search::{ElasticClient, SearchClient};
use bingxi_backend::services::so::order::SalesService;
use bingxi_backend::services::so::{CreateSalesOrderRequest, SalesOrderItemRequest};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use std::sync::Arc;
use test_common::setup_test_db;

const OPERATOR_ID: i32 = 997_001;
const CUSTOMER_ID: i32 = 997_011;
const PRODUCT_ID: i32 = 997_021;

fn now() -> DateTime<Utc> {
    Utc::now()
}

fn today_utc_midnight() -> DateTime<Utc> {
    Utc::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .expect("00:00:00 永远合法")
        .and_utc()
}

fn yesterday_utc_midnight() -> DateTime<Utc> {
    let d = Utc::now().date_naive() - chrono::Duration::days(1);
    d.and_hms_opt(0, 0, 0).expect("00:00:00 永远合法").and_utc()
}

async fn seed_operator(db: &DatabaseConnection) {
    user::ActiveModel {
        id: Set(OPERATOR_ID),
        username: Set("sodate_gate_lock".to_string()),
        password_hash: Set("not-a-real-hash".to_string()),
        real_name: Set(Some("日粒度交付日期门控锁操作人".to_string())),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("seed operator 失败: {e}"));
}

async fn seed_customer(db: &DatabaseConnection) {
    customer::ActiveModel {
        id: Set(CUSTOMER_ID),
        customer_code: Set("CUS-SODATE-001".to_string()),
        customer_name: Set("日粒度交付日期门控锁客户".to_string()),
        credit_limit: Set(Decimal::new(999_999, 0)),
        payment_terms: Set(30),
        status: Set(master_data::ACTIVE.to_string()),
        customer_type: Set("retail".to_string()),
        owner_id: Set(OPERATOR_ID),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("seed customer 失败: {e}"));
}

async fn seed_product(db: &DatabaseConnection) {
    product::ActiveModel {
        id: Set(PRODUCT_ID),
        name: Set("日粒度门控锁面料".to_string()),
        code: Set("PRD-SODATE-001".to_string()),
        unit: Set("米".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("fabric".to_string()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("seed product 失败: {e}"));
}

fn make_item() -> SalesOrderItemRequest {
    SalesOrderItemRequest {
        product_id: PRODUCT_ID,
        quantity: Decimal::new(100, 0),
        unit_price: Decimal::new(50, 0),
        discount_percent: None,
        tax_percent: None,
        notes: None,
        color_no: None,
        color_name: None,
        pantone_code: None,
        grade_required: None,
        quantity_meters: None,
        quantity_kg: None,
        gram_weight: None,
        width: None,
        paper_tube_weight: None,
        is_net_weight: None,
        batch_requirement: None,
        dye_lot_requirement: None,
        base_price: None,
        color_extra_cost: None,
        grade_price_diff: None,
        final_price: None,
        piece_no: None,
        quantity_tolerance_pct: None,
        fulfillment_source: None,
    }
}

fn make_request(required_date: Option<DateTime<Utc>>) -> CreateSalesOrderRequest {
    CreateSalesOrderRequest {
        customer_id: CUSTOMER_ID,
        opportunity_id: None,
        order_date: Some(now()),
        contact_person: Some("测试联系人".to_string()),
        contact_phone: Some("13800000001".to_string()),
        required_date,
        status: None,
        shipping_address: Some("测试地址".to_string()),
        billing_address: None,
        notes: None,
        items: vec![make_item()],
        payment_terms: None,
        remarks: None,
        batch_no: None,
        color_no: None,
        dye_lot_no: None,
        grade: None,
        packaging_requirement: None,
        quality_standard: None,
    }
}

/// required_date 等于今天（UTC midnight）：门控必须放行（订单创建成功）。
/// 若回潮为时间戳比较，则今天 UTC midnight < Utc::now()（当前时刻），
/// 门控错误拒绝 → 本用例断言通过即钉死"日粒度"口径。
#[tokio::test]
async fn required_date_today_must_pass() {
    let db = setup_test_db().await;
    seed_operator(&db).await;
    seed_customer(&db).await;
    seed_product(&db).await;

    let search_client: Arc<dyn SearchClient> = Arc::new(ElasticClient::mock());
    let service = SalesService::new(Arc::new(db), search_client);

    let req = make_request(Some(today_utc_midnight()));
    let result = service.create_order(req, OPERATOR_ID).await;

    assert!(
        result.is_ok(),
        "required_date 等于今天必须通过，实际拒绝: {:?}",
        result.err()
    );
}

/// required_date 等于昨天（UTC midnight）：门控必须拒绝，
/// 且错误码为 BUSINESS_ERROR、文案可外显（非"业务处理失败"脱敏常量）。
#[tokio::test]
async fn required_date_yesterday_must_be_rejected_with_displayable_message() {
    let db = setup_test_db().await;
    seed_operator(&db).await;
    seed_customer(&db).await;
    seed_product(&db).await;

    let search_client: Arc<dyn SearchClient> = Arc::new(ElasticClient::mock());
    let service = SalesService::new(Arc::new(db), search_client);

    let req = make_request(Some(yesterday_utc_midnight()));
    let result = service.create_order(req, OPERATOR_ID).await;

    let err = result.expect_err("required_date 昨天必须被拒绝");

    // 验证错误码
    assert_eq!(
        err.error_code(),
        "BUSINESS_ERROR",
        "交付日期门控拒绝应返回 BUSINESS_ERROR，实际: {}",
        err.error_code()
    );

    // 验证文案可外显（非脱敏常量"业务处理失败"）
    let public_msg = err.to_response().message;
    assert_ne!(
        public_msg, "业务处理失败",
        "拒绝文案必须可外显（business_displayable），不得是脱敏常量"
    );

    // 验证文案指向用户可执行的修正动作（提到"今天"）
    assert!(
        public_msg.contains("今天"),
        "拒绝文案必须包含\"今天\"让用户知道如何修正，实际: {public_msg}"
    );

    // 验证文案不含内部 ID（不泄露实现细节）
    assert!(
        !public_msg.contains(&CUSTOMER_ID.to_string()),
        "拒绝文案不得含记录 ID，实际: {public_msg}"
    );
}

/// 验证四处门控全部使用 business_displayable（非 business）：
/// 客户不存在、信用额度不足、产品不存在——均不在脱敏后返回固定"业务处理失败"。
#[tokio::test]
async fn non_existent_customer_rejected_with_displayable_message() {
    let db = setup_test_db().await;
    seed_operator(&db).await;
    seed_product(&db).await;

    let search_client: Arc<dyn SearchClient> = Arc::new(ElasticClient::mock());
    let service = SalesService::new(Arc::new(db), search_client);

    // 不种 customer，请求使用 CUSTOMER_ID → 客户不存在
    let req = make_request(Some(today_utc_midnight()));
    let result = service.create_order(req, OPERATOR_ID).await;

    let err = result.expect_err("客户不存在必须被拒绝");
    assert_eq!(err.error_code(), "BUSINESS_ERROR");

    let public_msg = err.to_response().message;
    assert_ne!(
        public_msg, "业务处理失败",
        "客户不存在拒绝文案必须可外显，不得是脱敏常量"
    );
    assert!(
        public_msg.contains("客户"),
        "文案必须指出是客户问题让用户知道该做什么，实际: {public_msg}"
    );
    assert!(
        !public_msg.contains(&CUSTOMER_ID.to_string()),
        "拒绝文案不得含内部记录 ID，实际: {public_msg}"
    );
}

/// 产品不存在：拒绝文案可外显且指向"产品"。
#[tokio::test]
async fn non_existent_product_rejected_with_displayable_message() {
    let db = setup_test_db().await;
    seed_operator(&db).await;
    seed_customer(&db).await;
    // 不种 product → 请求中 product_id 引用不存在的行

    let search_client: Arc<dyn SearchClient> = Arc::new(ElasticClient::mock());
    let service = SalesService::new(Arc::new(db), search_client);

    let req = make_request(Some(today_utc_midnight()));
    let result = service.create_order(req, OPERATOR_ID).await;

    let err = result.expect_err("产品不存在必须被拒绝");
    assert_eq!(err.error_code(), "BUSINESS_ERROR");

    let public_msg = err.to_response().message;
    assert_ne!(
        public_msg, "业务处理失败",
        "产品不存在拒绝文案必须可外显，不得是脱敏常量"
    );
    assert!(
        public_msg.contains("产品"),
        "文案必须指出是产品问题，实际: {public_msg}"
    );
    assert!(
        !public_msg.contains(&PRODUCT_ID.to_string()),
        "拒绝文案不得含内部记录 ID，实际: {public_msg}"
    );
}
