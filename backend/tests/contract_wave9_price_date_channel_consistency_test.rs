//! 价目域（销售/采购）日期解析拒绝「同形」契约锁
//!
//! 钉死事实（本文件唯一职责，只断状态码 + 信封机器码 + 是否脱敏常量，不断文案字面量）：
//! - 销售价目 `sales_price_service::create_price`/`update_price` 与采购价目
//!   `purchase_price_service::create_price`/`update_price` 的日期解析拒绝（入参校验族）
//!   两侧必须**同通道**：HTTP 400 + `code=VALIDATION_ERROR`，且出参 `message` 携带真实
//!   拒绝原因（`AppError::validation_displayable`），**不得**是 `err_msg::VALIDATION_PUBLIC`
//!   脱敏常量——日期串是用户自己提交的值，回显不触碰脱敏红线；一旦被脱敏成固定常量，
//!   用户就无法定位"哪个日期不合法"。
//! - "同形"取最硬口径：同一非法日期串在销售侧与采购侧产生的出参 message 逐字符相等
//!   （两文件用同一条 `format!("日期格式错误：{}", e)` 构造，chrono 对同一输入确定性输出），
//!   任何一侧换回脱敏 `validation` 通道或改走其它族，本锁即红。
//! - 判"是否脱敏常量"比较 `utils/messages.rs` 的权威常量 `err_msg::VALIDATION_PUBLIC`，
//!   不硬编码中文文案（文案会改，常量是唯一事实来源）。
//! - 夹具为真库（`setup_test_db`，缺 TEST_DATABASE_URL 直接 panic，禁静默回退）；
//!   种子自建自清，无 `#[ignore]`。

mod test_common;

use axum::response::IntoResponse as _;
use bingxi_backend::models::{customer, product, supplier, user};
use bingxi_backend::services::purchase_price_service::{
    CreatePurchasePriceInput, PurchasePriceService,
};
use bingxi_backend::services::sales_price_service::{
    CreateSalesPriceInput, SalesPriceService, UpdateSalesPriceInput,
};
use bingxi_backend::utils::error::AppError;
use bingxi_backend::utils::messages::err_msg;
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use std::sync::Arc;
use test_common::setup_test_db;

const OPERATOR_ID: i32 = 9601;
const SEED_PRODUCT_ID: i32 = 9602;
const SEED_CUSTOMER_ID: i32 = 9603;

fn now() -> DateTime<Utc> {
    Utc::now()
}

async fn seed_operator(db: &Arc<DatabaseConnection>) {
    user::ActiveModel {
        id: Set(OPERATOR_ID),
        username: Set("w9_price_date_channel".to_string()),
        password_hash: Set("test-only-not-a-real-hash".to_string()),
        real_name: Set(Some("价目日期通道契约操作人".to_string())),
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

async fn seed_product(db: &Arc<DatabaseConnection>) {
    product::ActiveModel {
        id: Set(SEED_PRODUCT_ID),
        name: Set("波次日期通道面料甲".to_string()),
        code: Set("PRD-PDC-0001".to_string()),
        unit: Set("米".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("fabric".to_string()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子产品插入失败: {e}"));
}

async fn seed_customer(db: &Arc<DatabaseConnection>) {
    customer::ActiveModel {
        id: Set(SEED_CUSTOMER_ID),
        customer_code: Set(format!("CUS-PDC-{SEED_CUSTOMER_ID}")),
        customer_name: Set("波次日期通道客户丙".to_string()),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
        customer_type: Set("retail".to_string()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子客户插入失败: {e}"));
}

/// 种子供应商：`suppliers` 属迁移种子参照表不被夹具 TRUNCATE ⇒ 不指定显式 id，
/// 自增插入后回读真实主键（列清单照 contract_wave8_price_ref_existence_test.rs 同款种子）。
async fn seed_supplier(db: &Arc<DatabaseConnection>, code: &str) -> i32 {
    let ts = supplier::ActiveModel {
        supplier_code: Set(code.to_string()),
        supplier_name: Set("价目日期通道供应商".to_string()),
        supplier_short_name: Set("日供".to_string()),
        supplier_type: Set("面料供应商".to_string()),
        credit_code: Set("91330000TEST00001X".to_string()),
        registered_address: Set("测试注册地址".to_string()),
        legal_representative: Set("测试法人".to_string()),
        registered_capital: Set(Decimal::ZERO),
        establishment_date: Set(NaiveDate::from_ymd_opt(2020, 1, 1).expect("夹具日期常量")),
        taxpayer_type: Set("一般纳税人".to_string()),
        bank_name: Set("测试银行".to_string()),
        bank_account: Set("6222000000000001".to_string()),
        contact_phone: Set("13800000001".to_string()),
        created_at: Set(now().into()),
        updated_at: Set(now().into()),
        is_processor: Set(false),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子供应商插入失败: {e}"));
    ts.id
}

/// 同形锁的三件套：400 + VALIDATION_ERROR + 出参 message **非**脱敏常量。
/// 返回出参 message 供跨通道逐字符比对。
fn expect_displayable_validation(err: &AppError, site: &str) -> String {
    assert_eq!(
        err.error_code(),
        "VALIDATION_ERROR",
        "{site}：日期解析拒绝必须归 VALIDATION_ERROR，实得 {err:?}"
    );
    let resp = err.to_response();
    assert_eq!(
        resp.code, "VALIDATION_ERROR",
        "{site}：出参信封 code 必须是 VALIDATION_ERROR，实得: {resp:?}"
    );
    assert_eq!(
        err.clone().into_response().status(),
        axum::http::StatusCode::BAD_REQUEST,
        "{site}：日期解析拒绝必须 400，禁止退化为 500/其它状态"
    );
    assert_ne!(
        resp.message,
        err_msg::VALIDATION_PUBLIC,
        "{site}：用户自己提交的日期串的解析拒绝原因被脱敏成固定常量，用户看不到\
         哪个日期不合法（#298 收口残留回归位）"
    );
    assert!(
        !resp.message.trim().is_empty(),
        "{site}：外显原因不得为空（空文案与脱敏等价，都是盲猜）"
    );
    resp.message
}

fn sales_create_input(
    effective_date: Option<&str>,
    expiry_date: Option<&str>,
) -> CreateSalesPriceInput {
    CreateSalesPriceInput {
        product_id: SEED_PRODUCT_ID,
        customer_id: Some(SEED_CUSTOMER_ID),
        customer_type: Some("direct".to_string()),
        price: Decimal::new(10000, 2),
        currency: Some("CNY".to_string()),
        unit: "米".to_string(),
        price_type: "standard".to_string(),
        price_level: None,
        min_order_qty: None,
        effective_date: effective_date.map(str::to_string),
        expiry_date: expiry_date.map(str::to_string),
    }
}

fn purchase_create_input(
    supplier_id: i32,
    effective_date: Option<&str>,
    expiry_date: Option<&str>,
) -> CreatePurchasePriceInput {
    CreatePurchasePriceInput {
        product_id: SEED_PRODUCT_ID,
        supplier_id,
        price: Decimal::new(8800, 2),
        currency: Some("CNY".to_string()),
        unit: "米".to_string(),
        price_type: "standard".to_string(),
        min_order_qty: None,
        effective_date: effective_date.map(str::to_string),
        expiry_date: expiry_date.map(str::to_string),
    }
}

// ---------------------------------------------------------------------------
// 1. create 双侧同形：同一非法到期日串，销售侧与采购侧的拒绝必须同 400、同机器码，
//    出参 message 非脱敏常量且逐字符相等；非法生效日同口径。
//    改坏什么必红：任一侧回退脱敏 `AppError::validation` → 该侧 message 命中脱敏
//    常量，assert_ne 红；任一侧换族（BUSINESS/INTERNAL）→ code/status 红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn create_invalid_dates_sales_and_purchase_are_same_shape_displayable() {
    let db = Arc::new(setup_test_db().await);
    seed_operator(&db).await;
    seed_product(&db).await;
    seed_customer(&db).await;
    let supplier_id = seed_supplier(&db, "SUP-PDC-0001").await;
    let sales = SalesPriceService::new(db.clone());
    let purchase = PurchasePriceService::new(db.clone());

    for illegal in ["2027-13-45", "not-a-date", "31/12/2027"] {
        let sales_err = sales
            .create_price(sales_create_input(None, Some(illegal)), OPERATOR_ID)
            .await
            .expect_err(&format!(
                "销售侧非法到期日 {illegal} 必须 fail-visible 拒绝"
            ));
        let purchase_err = purchase
            .create_price(
                purchase_create_input(supplier_id, None, Some(illegal)),
                OPERATOR_ID,
            )
            .await
            .expect_err(&format!(
                "采购侧非法到期日 {illegal} 必须 fail-visible 拒绝"
            ));

        let sales_msg = expect_displayable_validation(
            &sales_err,
            &format!("create.expiry_date({illegal}) 销售侧"),
        );
        let purchase_msg = expect_displayable_validation(
            &purchase_err,
            &format!("create.expiry_date({illegal}) 采购侧"),
        );
        assert_eq!(
            sales_msg, purchase_msg,
            "同一非法日期串在两侧的拒绝原因必须逐字符同形（通道/构造不同源即回归），\
             销售: {sales_msg} / 采购: {purchase_msg}"
        );
    }

    for illegal in ["2027-02-30", "0000-00-00"] {
        let sales_err = sales
            .create_price(sales_create_input(Some(illegal), None), OPERATOR_ID)
            .await
            .expect_err(&format!(
                "销售侧非法生效日 {illegal} 必须 fail-visible 拒绝"
            ));
        let purchase_err = purchase
            .create_price(
                purchase_create_input(supplier_id, Some(illegal), None),
                OPERATOR_ID,
            )
            .await
            .expect_err(&format!(
                "采购侧非法生效日 {illegal} 必须 fail-visible 拒绝"
            ));

        let sales_msg = expect_displayable_validation(
            &sales_err,
            &format!("create.effective_date({illegal}) 销售侧"),
        );
        let purchase_msg = expect_displayable_validation(
            &purchase_err,
            &format!("create.effective_date({illegal}) 采购侧"),
        );
        assert_eq!(
            sales_msg, purchase_msg,
            "生效日两侧同形锁（销售侧本批统一为 displayable 通道，回退即红）"
        );
    }
}

// ---------------------------------------------------------------------------
// 2. update 双侧同形：销售 update（effective/expiry 两分支）与采购 update（expiry 分支）
//    的非法日期拒绝同样必须走可外显通道且与采购 create 同形。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn update_invalid_dates_sales_and_purchase_are_same_shape_displayable() {
    let db = Arc::new(setup_test_db().await);
    seed_operator(&db).await;
    seed_product(&db).await;
    seed_customer(&db).await;
    let supplier_id = seed_supplier(&db, "SUP-PDC-0002").await;
    let sales = SalesPriceService::new(db.clone());
    let purchase = PurchasePriceService::new(db.clone());

    let sales_row = sales
        .create_price(sales_create_input(None, None), OPERATOR_ID)
        .await
        .expect("update 前置建行必须成功");
    let purchase_row = purchase
        .create_price(purchase_create_input(supplier_id, None, None), OPERATOR_ID)
        .await
        .expect("采购 update 前置建行必须成功");

    let illegal = "31/12/2099";

    // 销售 update：effective_date 分支（Some(Some(illegal))，NOT NULL 列给非法值）。
    // UpdateSalesPriceInput 未 derive Default ⇒ 逐字段显式给 None（键缺席=不改）。
    let sales_eff_err = sales
        .update_price(
            sales_row.id,
            UpdateSalesPriceInput {
                product_id: None,
                customer_id: None,
                customer_type: None,
                price: None,
                currency: None,
                price_level: None,
                min_order_qty: None,
                effective_date: Some(Some(illegal.to_string())),
                expiry_date: None,
            },
        )
        .await
        .expect_err("销售 update 非法生效日必须 fail-visible 拒绝");
    // 销售 update：expiry_date 分支
    let sales_exp_err = sales
        .update_price(
            sales_row.id,
            UpdateSalesPriceInput {
                product_id: None,
                customer_id: None,
                customer_type: None,
                price: None,
                currency: None,
                price_level: None,
                min_order_qty: None,
                effective_date: None,
                expiry_date: Some(Some(illegal.to_string())),
            },
        )
        .await
        .expect_err("销售 update 非法到期日必须 fail-visible 拒绝");
    // 采购 update：expiry_date 分支（本文件收口的最后一条残留通道）
    let purchase_exp_err = purchase
        .update_price(
            purchase_row.id,
            Decimal::new(1, 0),
            Some(illegal.to_string()),
            None,
        )
        .await
        .expect_err("采购 update 非法到期日必须 fail-visible 拒绝");

    let m1 = expect_displayable_validation(&sales_eff_err, "update.effective_date 销售侧");
    let m2 = expect_displayable_validation(&sales_exp_err, "update.expiry_date 销售侧");
    let m3 = expect_displayable_validation(&purchase_exp_err, "update.expiry_date 采购侧");
    assert_eq!(
        (m1.as_str(), m2.as_str()),
        (m3.as_str(), m3.as_str()),
        "同一非法日期串在价目域四个解析拒绝点必须逐字符同形，实得 销售eff={m1} \
         销售exp={m2} 采购exp={m3}"
    );
}
