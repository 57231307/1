//! 任务板 —— 价目写链「引用存在性预检」真库契约锁（后端线）
//!
//! 钉死缺陷（调查组结论）：`sales_price_service::create_price`/`update_price` 与
//! `purchase_price_service::create_price` 此前把 product_id/customer_id/supplier_id
//! 直接 `Set(...)` 落库、**完全不校验引用存在性**，而 `sales_prices`/`purchase_prices`
//! 两表当前没有指向 products/customers/suppliers 的外键（补 FK 属另一路任务板
//! 的迁移）⇒ 给不存在的产品/客户/供应商建价目行会静默成功，产生孤儿价目（列表侧
//! `SalesPriceView`/`PurchasePriceView` 的 JOIN 名列如实 NULL 即其下游形态）。
//!
//! 收紧后的口径（本文件逐条钉死）：
//! - 引用不存在归 **VALIDATION 族**（HTTP 400 + `code=VALIDATION_ERROR`，可外显变体
//!   `AppError::validation_displayable`，与销售侧价格等级白名单拒绝同族同通道，
//!   先例 `sku_mapping_service::validate_refs` / `contract_wave8_price_level_expiry_test.rs`）；
//!   **绝不能是 500/DATABASE_ERROR**（FK 违约经 `AppError::From<DbErr>` 的 Exec 分支
//!   才是裸 500 形态，见 `inventory_reservation_service::create_reservation` 头注缺陷族）。
//! - 拒绝路径零副作用（不落行 / 原行逐列不动）；合法引用正向必须放行（预检不得误杀）。
//! - update 侧 product_id/customer_id 为 Option：仅显式传值时校验，键缺席不校验不改值。
//! - 本仓拒绝文案永久脱敏/外显由构造点声明，本文件**只断 status 与信封机器码，
//!   绝不断错误文案原文**（脱敏红线）。
//! - 夹具为真库（`setup_test_db`，已迁移 PostgreSQL，缺 TEST_DATABASE_URL 直接 panic，
//!   禁静默回退 sqlite）；种子**真实**自建产品/客户/供应商（suppliers 属迁移种子参照表
//!   不被 TRUNCATE，自建行按回读 id 使用），坏引用用不可能存在的显式 ID，绝不依赖
//!   "库恰好为空"。

mod test_common;

use bingxi_backend::models::purchase_price;
use bingxi_backend::models::sales_price;
use bingxi_backend::models::{customer, product, supplier, user};
use bingxi_backend::services::purchase_price_service::{
    CreatePurchasePriceInput, PurchasePriceService,
};
use bingxi_backend::services::sales_price_service::{
    CreateSalesPriceInput, SalesPriceService, UpdateSalesPriceInput,
};
use bingxi_backend::utils::error::AppError;
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, PaginatorTrait, Set};
use serde_json::json;
use std::sync::Arc;
use test_common::setup_test_db;

const OPERATOR_ID: i32 = 9531;
const SEED_PRODUCT_ID: i32 = 9501;
const SEED_CUSTOMER_ID: i32 = 9502;
const OTHER_CUSTOMER_ID: i32 = 9503;
/// 不可能存在的引用 ID：products/customers 每例被夹具 TRUNCATE 后从 1 起，
/// suppliers 虽属不被清空的迁移种子参照表（`test_common` 的 SEALED_REFERENCE_TABLES），
/// 其目录种子远小于该值 ⇒ 三个域共用此探针对象，判定与"库恰好为空"无关。
const MISSING_REF_ID: i32 = 2_000_000_001;

fn now() -> DateTime<Utc> {
    Utc::now()
}

async fn seed_operator(db: &Arc<DatabaseConnection>) {
    user::ActiveModel {
        id: Set(OPERATOR_ID),
        username: Set("w8_price_ref".to_string()),
        password_hash: Set("test-only-not-a-real-hash".to_string()),
        real_name: Set(Some("价目引用预检操作人".to_string())),
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

async fn seed_product(db: &Arc<DatabaseConnection>, id: i32, name: &str, code: &str) {
    product::ActiveModel {
        id: Set(id),
        name: Set(name.to_string()),
        code: Set(code.to_string()),
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
    .unwrap_or_else(|e| panic!("种子产品 {id} 插入失败: {e}"));
}

async fn seed_customer(db: &Arc<DatabaseConnection>, id: i32, name: &str) {
    customer::ActiveModel {
        id: Set(id),
        customer_code: Set(format!("CUS-REF-{id}")),
        customer_name: Set(name.to_string()),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
        // 词表安全侧：constants::customer_type::ALLOWED 成员（见 models/customer.rs 列注）
        customer_type: Set("retail".to_string()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子客户 {id} 插入失败: {e}"));
}

/// 种子供应商：`suppliers` 是迁移种子参照表、夹具不清空 ⇒ 不指定显式 id，
/// 自增插入后回读真实主键给正向用例使用（列清单对 models/supplier.rs NOT NULL 列逐一核对，
/// 造数形态照 tests/contract_wave1_purchase_return_dimensions_test.rs 的 supplier 种子）。
async fn seed_supplier(db: &Arc<DatabaseConnection>, code: &str) -> i32 {
    let ts = supplier::ActiveModel {
        supplier_code: Set(code.to_string()),
        supplier_name: Set("价目引用预检供应商".to_string()),
        supplier_short_name: Set("引供".to_string()),
        supplier_type: Set("面料供应商".to_string()),
        credit_code: Set("91330000TEST00000X".to_string()),
        registered_address: Set("测试注册地址".to_string()),
        legal_representative: Set("测试法人".to_string()),
        registered_capital: Set(Decimal::ZERO),
        establishment_date: Set(NaiveDate::from_ymd_opt(2020, 1, 1).expect("夹具日期常量")),
        taxpayer_type: Set("一般纳税人".to_string()),
        bank_name: Set("测试银行".to_string()),
        bank_account: Set("6222000000000000".to_string()),
        contact_phone: Set("13800000000".to_string()),
        created_at: Set(now().into()),
        updated_at: Set(now().into()),
        // is_processor 为 NOT NULL bool 列（models/supplier.rs），照
        // tests/contract_wave1_purchase_return_dimensions_test.rs 种子显式给值，
        // 不赌库端默认值形态
        is_processor: Set(false),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子供应商插入失败: {e}"));
    ts.id
}

fn sales_create_input(product_id: i32, customer_id: Option<i32>) -> CreateSalesPriceInput {
    CreateSalesPriceInput {
        product_id,
        customer_id,
        customer_type: Some("retail".to_string()),
        price: Decimal::new(10000, 2),
        currency: Some("CNY".to_string()),
        unit: "米".to_string(),
        price_type: "standard".to_string(),
        price_level: None,
        min_order_qty: None,
        effective_date: None,
        expiry_date: None,
    }
}

fn purchase_create_input(product_id: i32, supplier_id: i32) -> CreatePurchasePriceInput {
    CreatePurchasePriceInput {
        product_id,
        supplier_id,
        price: Decimal::new(10000, 2),
        currency: Some("CNY".to_string()),
        unit: "米".to_string(),
        price_type: "standard".to_string(),
        min_order_qty: None,
        effective_date: None,
        expiry_date: None,
    }
}

/// 校验拒绝必须 400 + VALIDATION_ERROR（族与销售侧价格等级白名单拒绝同口径，
/// 照 `contract_wave8_price_level_expiry_test.rs::expect_validation_400`）；
/// 只钉机器码与状态码，绝不断错误文案原文（脱敏红线）；并显式排除 500/DATABASE_ERROR 回潮。
fn expect_validation_400(err: &AppError) {
    assert_eq!(
        err.error_code(),
        "VALIDATION_ERROR",
        "引用不存在拒绝必须归 VALIDATION_ERROR，实得 {err:?}"
    );
    let resp = err.to_response();
    assert_eq!(
        resp.code, "VALIDATION_ERROR",
        "出参信封 code 必须是 VALIDATION_ERROR，实得: {resp:?}"
    );
    use axum::response::IntoResponse;
    let http = err.clone().into_response();
    assert_eq!(
        http.status(),
        axum::http::StatusCode::BAD_REQUEST,
        "引用不存在拒绝必须 400，禁止退化为 500/其它状态"
    );
    assert_ne!(
        err.error_code(),
        "DATABASE_ERROR",
        "禁止裸 FK/数据库错误形态回潮"
    );
    assert_ne!(
        http.status(),
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        "禁止 500（含 map_err(internal) 拍平族，任务板 #175 口径）"
    );
}

// ---------------------------------------------------------------------------
// 1. ①销售建单：不存在 product_id → 400 + VALIDATION_ERROR + 零落行；
//    同例第二腿钉不存在 customer_id（客户可空仅显传值时检，传了坏值必须拒）。
//    改坏什么必红：预检摘除 → 两腿都静默落孤儿行红；预检误拦 customer_id=None
//    （通用价目语义）→ 首腿的合法客户段之外，第 4 例正向红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn create_sales_price_missing_refs_is_400_validation_error_with_zero_rows() {
    let db = Arc::new(setup_test_db().await);
    seed_operator(&db).await;
    seed_product(&db, SEED_PRODUCT_ID, "引用预检面料甲", "PRD-REF-0001").await;
    seed_customer(&db, SEED_CUSTOMER_ID, "引用预检客户乙").await;
    let svc = SalesPriceService::new(db.clone());

    // 腿 A：product 缺失（customer 合法，单独钉产品这一引用位）
    let err = svc
        .create_price(
            sales_create_input(MISSING_REF_ID, Some(SEED_CUSTOMER_ID)),
            OPERATOR_ID,
        )
        .await
        .expect_err("不存在的产品引用必须被预检拒绝，不得静默落孤儿价目行");
    expect_validation_400(&err);

    // 腿 B：customer 缺失（product 合法，钉客户这一引用位）
    let err = svc
        .create_price(
            sales_create_input(SEED_PRODUCT_ID, Some(MISSING_REF_ID)),
            OPERATOR_ID,
        )
        .await
        .expect_err("不存在的客户引用必须被预检拒绝，不得静默落孤儿价目行");
    expect_validation_400(&err);

    let total = sales_price::Entity::find()
        .count(db.as_ref())
        .await
        .expect("统计销售价目行失败");
    assert_eq!(total, 0, "被拒建单必须零副作用（不落行），实得 {total}");
}

// ---------------------------------------------------------------------------
// 2. ②采购建单：不存在 supplier_id → 400 + VALIDATION_ERROR + 零落行；
//    同例第二腿钉不存在 product_id（采购两侧引用均 NOT NULL，任一缺失都必须拒）。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn create_purchase_price_missing_refs_is_400_validation_error_with_zero_rows() {
    let db = Arc::new(setup_test_db().await);
    seed_operator(&db).await;
    seed_product(&db, SEED_PRODUCT_ID, "引用预检面料甲", "PRD-REF-0002").await;
    let supplier_id = seed_supplier(&db, "SUP-REF-0001").await;
    let svc = PurchasePriceService::new(db.clone());

    // 腿 A：supplier 缺失（product 合法）
    let err = svc
        .create_price(
            purchase_create_input(SEED_PRODUCT_ID, MISSING_REF_ID),
            OPERATOR_ID,
        )
        .await
        .expect_err("不存在的供应商引用必须被预检拒绝，不得静默落孤儿采购价目行");
    expect_validation_400(&err);

    // 腿 B：product 缺失（supplier 合法）
    let err = svc
        .create_price(
            purchase_create_input(MISSING_REF_ID, supplier_id),
            OPERATOR_ID,
        )
        .await
        .expect_err("不存在的产品引用必须被预检拒绝，不得静默落孤儿采购价目行");
    expect_validation_400(&err);

    let total = purchase_price::Entity::find()
        .count(db.as_ref())
        .await
        .expect("统计采购价目行失败");
    assert_eq!(total, 0, "被拒建单必须零副作用（不落行），实得 {total}");
}

// ---------------------------------------------------------------------------
// 3. ③销售 update：传入不存在 customer_id → 400 + VALIDATION_ERROR 且**原行逐列不动**
//    （先经真实服务口建出合法行，再发起坏引用更新）；钉住"拒绝发生在任何写动作之前"。
//    改坏什么必红：预检挪到 ActiveModel 改动之后/漏检 → 原行断言红（坏值已 Set）。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn update_sales_price_missing_customer_is_rejected_and_row_unchanged() {
    let db = Arc::new(setup_test_db().await);
    seed_operator(&db).await;
    seed_product(&db, SEED_PRODUCT_ID, "引用预检面料甲", "PRD-REF-0003").await;
    seed_customer(&db, SEED_CUSTOMER_ID, "引用预检客户乙").await;
    let svc = SalesPriceService::new(db.clone());

    let created = svc
        .create_price(
            sales_create_input(SEED_PRODUCT_ID, Some(SEED_CUSTOMER_ID)),
            OPERATOR_ID,
        )
        .await
        .expect("合法引用建单必须成功（前置夹具自证）");

    // 线格式注入坏客户引用；price 给一个"若被写入必可观察到"的差值，证明拒绝零副作用
    let req: UpdateSalesPriceInput =
        serde_json::from_value(json!({ "customer_id": MISSING_REF_ID, "price": "999.00" }))
            .expect("update 线格式必须可反序列化");
    let err = svc
        .update_price(created.id, req)
        .await
        .expect_err("不存在的客户引用更新必须被预检拒绝");
    expect_validation_400(&err);

    let after = sales_price::Entity::find_by_id(created.id)
        .one(db.as_ref())
        .await
        .expect("回读销售价目失败")
        .expect("被拒更新不得删行");
    assert_eq!(
        after.customer_id,
        Some(SEED_CUSTOMER_ID),
        "被拒更新不得改动 customer_id"
    );
    assert_eq!(
        after.product_id, SEED_PRODUCT_ID,
        "被拒更新不得改动 product_id"
    );
    assert_eq!(
        after.price, created.price,
        "被拒更新不得改动 price（坏请求里的 999.00 不得落库）"
    );
    assert_eq!(after.status, created.status, "被拒更新不得改动 status");
}

// ---------------------------------------------------------------------------
// 4. 正向对照（预检不得误杀合法路径）：合法引用建销售/采购价目成功；
//    update 键缺席=不校验引用、改其它列成功；update 给合法的另一客户引用成功。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn valid_refs_pass_precheck_for_create_and_update() {
    let db = Arc::new(setup_test_db().await);
    seed_operator(&db).await;
    seed_product(&db, SEED_PRODUCT_ID, "引用预检面料甲", "PRD-REF-0004").await;
    seed_customer(&db, SEED_CUSTOMER_ID, "引用预检客户乙").await;
    seed_customer(&db, OTHER_CUSTOMER_ID, "引用预检客户丁").await;
    let supplier_id = seed_supplier(&db, "SUP-REF-0002").await;

    let svc = SalesPriceService::new(db.clone());
    // 销售 create：合法 product+customer 放行；customer_id=None（通用价目语义）也必须放行
    let created = svc
        .create_price(
            sales_create_input(SEED_PRODUCT_ID, Some(SEED_CUSTOMER_ID)),
            OPERATOR_ID,
        )
        .await
        .expect("合法引用建销售价目必须成功（预检不得误杀）");
    let created_null_customer = svc
        .create_price(sales_create_input(SEED_PRODUCT_ID, None), OPERATOR_ID)
        .await
        .expect("customer_id 缺席（通用价目）必须放行，NULL 是既有合法语义");
    assert_eq!(created_null_customer.customer_id, None);

    let po_svc = PurchasePriceService::new(db.clone());
    po_svc
        .create_price(
            purchase_create_input(SEED_PRODUCT_ID, supplier_id),
            OPERATOR_ID,
        )
        .await
        .expect("合法引用建采购价目必须成功（预检不得误杀）");

    // 销售 update：键缺席不校验（只改价格）成功
    let req: UpdateSalesPriceInput =
        serde_json::from_value(json!({ "price": "105.00" })).expect("线格式必须可反序列化");
    let after = svc
        .update_price(created.id, req)
        .await
        .expect("键缺席更新不得触发引用校验（不改动引用列）");
    assert_eq!(after.price, Decimal::new(10500, 2));

    // 销售 update：给另一个合法客户引用 → 成功且落值
    let req: UpdateSalesPriceInput = serde_json::from_value(json!({
        "customer_id": OTHER_CUSTOMER_ID
    }))
    .expect("线格式必须可反序列化");
    let after = svc
        .update_price(created.id, req)
        .await
        .expect("合法客户引用更新必须成功（预检不得误杀）");
    assert_eq!(after.customer_id, Some(OTHER_CUSTOMER_ID));
}

// ---------------------------------------------------------------------------
// 5. 源码扫描锁（"把预检从落点上摘掉就必红"的结构性证明，照 wave5 先例）：
//    ①三处预检必须位于各自写库语句之前；②装配族只能是 validation_displayable；
// ③两个 service 文件不得出现 AppError::internal（收口族防回潮）。
// ---------------------------------------------------------------------------
#[test]
fn source_scan_price_ref_prechecks_precede_writes_and_stay_in_validation_family() {
    let sales = include_str!("../src/services/sales_price_service.rs").replace('\r', "");
    let insert_at = sales
        .find("active_price.insert(&*self.db)")
        .expect("sales create 的 insert 锚点不得消失");
    let pre_at = sales
        .find("self.assert_product_exists(req.product_id)")
        .expect("sales create 产品引用预检不得丢失");
    assert!(pre_at < insert_at, "sales create 预检必须位于写库之前");
    // update 侧锚点用 rfind：`self.assert_customer_exists(customer_id)` 在 create/update
    // 各出现一次，find 会命中 create 那次，必须取最后一次（update 内那次）才是本锁对象
    let upd_at = sales
        .rfind("self.assert_customer_exists(customer_id)")
        .expect("sales update 客户引用预检不得丢失");
    let update_exec_at = sales
        .find("active.update(&*self.db)")
        .expect("sales update 的 update 锚点不得消失");
    assert!(upd_at < update_exec_at, "sales update 预检必须位于写库之前");

    let purchase = include_str!("../src/services/purchase_price_service.rs").replace('\r', "");
    let p_insert_at = purchase
        .find("active_price.insert(&*self.db)")
        .expect("purchase create 的 insert 锚点不得消失");
    for pre in [
        "self.assert_product_exists(req.product_id)",
        "self.assert_supplier_exists(req.supplier_id)",
    ] {
        let at = purchase
            .find(pre)
            .unwrap_or_else(|| panic!("purchase create 引用预检丢失: {pre}"));
        assert!(at < p_insert_at, "预检 {pre} 必须位于写库 insert 之前");
    }

    // 族装配：引用拒绝只能是 validation_displayable（可外显 VALIDATION 族），
    // 且两个文件对 AppError::internal 零容忍（禁止把业务拒绝拍平成 500，任务板）
    for (name, src) in [("sales", &sales), ("purchase", &purchase)] {
        assert!(
            src.contains("AppError::validation_displayable"),
            "{name} 侧引用预检必须装配 validation_displayable（可外显 4xx），不得改族"
        );
        assert!(
            !src.contains("AppError::internal("),
            "{name} 侧 service 禁止 AppError::internal（业务拒绝拍平 500 回潮，#175 收口族）"
        );
        assert!(
            !src.contains("AppError::database(format!("),
            "{name} 侧 service 禁止 AppError::database(format!(…)) 重包装 DbErr（错误原文外泄风险）"
        );
    }
}
