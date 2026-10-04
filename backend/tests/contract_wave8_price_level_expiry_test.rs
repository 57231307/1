//! 销售价目批 —— `price_level` 写链接入（A′）+ `expiry_date` 静默/三态 + 查询参数漂移 防回潮契约锁
//!
//! 钉死事实（决策建议书裁定 2 A′ + F1b 交回清单 §6-6/§6-7）：
//! - `price_level` 此前是"列有（m0011:185）、实体有（models/sales_price.rs:20）、读链有、
//!   写链断（两写 DTO 无键 ⇒ create 恒 NULL）"的中间态；A′ 按真实词表接入，取值域**单源**于
//!   `inventory_stock_grade`（models/status/purchase_inventory.rs:308-320，一等品/二等品/等外品），
//!   保守集 = {一等品, 二等品}（「等外品」是否开放手工定价待用户终裁，批了才扩白名单并同步改本锁）。
//!   前端伪词表 `A/B/C/D`（"A级"）必须 400 + VALIDATION_ERROR——裸收会造出既不被当标准价、
//!   又不匹配二等品的孤儿行，打断质检降级链（quality_inspection_service.rs:570-600）。
//! - create 的 `expiry_date` 此前 `.and_then(|d| d.parse().ok())` 把非法日期串静默吞成 NULL
//!   （不报错、不落库，F1b §6-7 证真），现与 `effective_date` 同口径 fail-visible 400，且拒绝路径零副作用。
//! - update `expiry_date`（可空列）三态统一（任务板 #169，照 department_service.rs:220-221 先例）：
//!   缺键=保持 / 显式 null=清空 NULL / 给值=改值；`effective_date` 为 NOT NULL 列，显式 null 拒绝而非静默保持。
//! - 列表查询参数 `keyword`（前端承诺"产品名称/客户名称"模糊匹配）与 `customer_id` 此前前端在传、
//!   后端 `SalesPriceQuery` 无键被 serde 静默忽略（#206/#160 同族假筛选），现已接收并下推谓词。
//! - 夹具为真库（`setup_test_db`，已迁移 PostgreSQL，缺 TEST_DATABASE_URL 直接 panic，禁静默回退）；
//!   种子自建自清（夹具每次调用 TRUNCATE 业务表）。只断 status + 信封机器码，
//!   **不断错误文案原文**（本仓拒绝文案永久脱敏）。

mod test_common;

use bingxi_backend::models::sales_price;
use bingxi_backend::models::status::price_approval;
use bingxi_backend::models::status::purchase_inventory::inventory_stock_grade;
use bingxi_backend::models::{customer, product, user};
use bingxi_backend::services::sales_price_service::{
    CreateSalesPriceInput, SalesPriceQueryParams, SalesPriceService, UpdateSalesPriceInput,
};
use bingxi_backend::utils::error::AppError;
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use serde_json::json;
use std::sync::Arc;
use test_common::setup_test_db;

const OPERATOR_ID: i32 = 9451;
const SEED_PRODUCT_ID: i32 = 9452;
const SEED_CUSTOMER_ID: i32 = 9453;
const OTHER_PRODUCT_ID: i32 = 9454;

fn now() -> DateTime<Utc> {
    Utc::now()
}

async fn seed_operator(db: &Arc<DatabaseConnection>) {
    user::ActiveModel {
        id: Set(OPERATOR_ID),
        username: Set("w8_price_level".to_string()),
        password_hash: Set("test-only-not-a-real-hash".to_string()),
        real_name: Set(Some("价格等级批操作人".to_string())),
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
        customer_code: Set(format!("CUS-PL-{id}")),
        customer_name: Set(name.to_string()),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
        // 词表安全侧：constants::customer_type::ALLOWED 成员（见 models/customer.rs 列注）。
        // 旧值 "direct" 不在 chk_customers_customer_type 五值集合内，种子插入即被 DB 拒绝
        // （CI #4675 族A 8 例同因），此处仅改数据取值对齐姊妹文件，不动任何断言。
        customer_type: Set("retail".to_string()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子客户 {id} 插入失败: {e}"));
}

/// 线格式口径建单入参（price 走十进制 string，对齐 F1b §1.2 终态；等级/到期日按用例注入）
fn create_input(price_level: Option<&str>, expiry_date: Option<&str>) -> CreateSalesPriceInput {
    CreateSalesPriceInput {
        product_id: SEED_PRODUCT_ID,
        customer_id: Some(SEED_CUSTOMER_ID),
        // 与种子客户同口径：constants::customer_type::ALLOWED 成员（sales_prices 侧为透传列，
        // 值仍取词表合法成员，避免与 customers 行语义互相矛盾）
        customer_type: Some("retail".to_string()),
        price: Decimal::new(10000, 2),
        currency: Some("CNY".to_string()),
        unit: "米".to_string(),
        price_type: "standard".to_string(),
        price_level: price_level.map(str::to_string),
        min_order_qty: None,
        effective_date: None,
        expiry_date: expiry_date.map(str::to_string),
    }
}

fn query_params() -> SalesPriceQueryParams {
    // page_size 必须显式非零：Default 归零后 limit(0) 恒空，会把"命中"测成"未命中"
    SalesPriceQueryParams {
        page: 1,
        page_size: 50,
        ..Default::default()
    }
}

async fn reload(db: &Arc<DatabaseConnection>, id: i32) -> sales_price::Model {
    sales_price::Entity::find_by_id(id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap_or_else(|| panic!("销售价目 {id} 应已落库"))
}

/// 校验拒绝必须 400 + VALIDATION_ERROR（族与销售列表 status 筛选 400 化同口径）；
/// 本仓拒绝文案永久脱敏，只钉机器码与状态码，不断文案原文。
fn expect_validation_400(err: &AppError) {
    assert_eq!(
        err.error_code(),
        "VALIDATION_ERROR",
        "白名单/格式拒绝必须归 VALIDATION_ERROR，实得 {err:?}"
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
        "校验拒绝必须 400，禁止退化为 500/其它状态"
    );
}

// ---------------------------------------------------------------------------
// 1. A′ 接入：price_level="二等品"（词表常量）建单落库，列表回读为该值。
//    改坏什么必红：删掉 create 透传分支（回到恒 NULL）→ 回读红；白名单换成伪词表
//    A/B/C/D 字面量数组 → 词表常量比对红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn create_price_level_second_grade_persists_and_lists_back() {
    let db = Arc::new(setup_test_db().await);
    seed_operator(&db).await;
    seed_product(&db, SEED_PRODUCT_ID, "波次等级面料甲", "PRD-PL-0001").await;
    seed_customer(&db, SEED_CUSTOMER_ID, "波次等级客户丙").await;
    let svc = SalesPriceService::new(db.clone());

    let created = svc
        .create_price(
            create_input(Some(inventory_stock_grade::SECOND), None),
            OPERATOR_ID,
        )
        .await
        .expect("二等品（词表真实值）建单必须成功");
    assert_eq!(
        created.price_level.as_deref(),
        Some(inventory_stock_grade::SECOND),
        "create 必须把校验后的等级落库（写链断裂回归即此处回 None）"
    );

    let mut p = query_params();
    p.product_id = Some(SEED_PRODUCT_ID);
    let (rows, total) = svc.get_prices_list(p).await.expect("列表查询必须成功");
    assert_eq!(
        total, 1,
        "自建自清后应按产品命中唯一建行，实得 total={total}"
    );
    assert_eq!(
        rows[0].price_level.as_deref(),
        Some(inventory_stock_grade::SECOND),
        "列表整 Model 回读的 price_level 必须与建单值逐字符一致"
    );
}

// ---------------------------------------------------------------------------
// 2. 伪词表拒绝："A级"/"A"/"B级"（前端表单伪造值）与保守集外的"等外品"（待用户终裁）
//    一律 400 + VALIDATION_ERROR，且拒绝路径零副作用（不落行）。
//    改坏什么必红：裸收 DTO 键不做白名单 → "A级" 段红（孤儿行打断质检降级链的入口 reopen）。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn create_price_level_out_of_whitelist_is_400_with_zero_side_effects() {
    let db = Arc::new(setup_test_db().await);
    seed_operator(&db).await;
    seed_product(&db, SEED_PRODUCT_ID, "波次等级面料甲", "PRD-PL-0002").await;
    seed_customer(&db, SEED_CUSTOMER_ID, "波次等级客户丙").await;
    let svc = SalesPriceService::new(db.clone());

    // "等外品"属词表真实值但不在本批保守集 {一等品,二等品}；用户终裁开放时本例随白名单同步扩
    for illegal in ["A级", "A", "B级", inventory_stock_grade::OFF_GRADE] {
        let err = svc
            .create_price(create_input(Some(illegal), None), OPERATOR_ID)
            .await
            .expect_err(&format!(
                "越界等级 {illegal} 必须 400，不得静默落库造孤儿行"
            ));
        expect_validation_400(&err);
    }

    let (rows, total) = svc
        .get_prices_list(query_params())
        .await
        .expect("列表查询必须成功");
    assert_eq!(total, 0, "被拒建单必须零副作用（不落行），实得 {rows:?}");
}

// ---------------------------------------------------------------------------
// 3. 缺键/空串 ⇒ NULL ⇒ 标准价语义保持：线格式 JSON（无 price_level 键）反序列化建单
//    落 NULL；空串归一 NULL；两者审批后落库回读仍是 NULL 等级 + approved——质检侧
//    quality_inspection_service.rs:576 用 price_level IS NULL 认标准价，本语义不得改成空串。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn create_without_or_empty_price_level_falls_null_and_stays_standard_price() {
    let db = Arc::new(setup_test_db().await);
    seed_operator(&db).await;
    seed_product(&db, SEED_PRODUCT_ID, "波次等级面料甲", "PRD-PL-0003").await;
    seed_customer(&db, SEED_CUSTOMER_ID, "波次等级客户丙").await;
    let svc = SalesPriceService::new(db.clone());

    // 线格式直连：JSON 里根本没有 price_level 键，钉 serde 层缺键 = None
    let body = json!({
        "product_id": SEED_PRODUCT_ID,
        "customer_id": SEED_CUSTOMER_ID,
        "price": "100.00",
        "unit": "米",
        "price_type": "standard",
    });
    let input: CreateSalesPriceInput =
        serde_json::from_value(body).expect("无 price_level 键的线格式必须可反序列化");
    let created = svc
        .create_price(input, OPERATOR_ID)
        .await
        .expect("缺键建单必须成功（向后兼容：不发键=现行为）");
    assert_eq!(
        created.price_level, None,
        "缺键必须落 NULL（标准价语义载体），不得落成空串或其它"
    );

    // 空串/纯空白同样归一 NULL（不是"非法值"，是"未填"）
    let created_empty = svc
        .create_price(create_input(Some(""), None), OPERATOR_ID)
        .await
        .expect("空串等级必须视为未填而非拒绝");
    assert_eq!(created_empty.price_level, None, "空串等级必须落 NULL");

    // 审批后两行仍真实落库：status=approved 且等级保持 NULL（标准价语义）
    svc.approve_price(created.id, OPERATOR_ID)
        .await
        .expect("pending → approved 权威路径必须成功");
    svc.approve_price(created_empty.id, OPERATOR_ID)
        .await
        .expect("pending → approved 权威路径必须成功");
    for id in [created.id, created_empty.id] {
        let after = reload(&db, id).await;
        assert_eq!(
            after.status,
            price_approval::APPROVED,
            "审批痕迹必须真实落库（夹具自证，非断被测面）"
        );
        assert!(
            after.price_level.is_none(),
            "NULL 等级建行 {id} 审批后落库回读仍必须是 NULL 等级（质检侧 \
             quality_inspection_service.rs:576 按 price_level IS NULL 认标准价的语义前提）"
        );
    }
}

// ---------------------------------------------------------------------------
// 4. update 等级透传：给合法值改值；给伪词表 400 且原值不动。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn update_price_level_passes_through_and_rejects_fake_vocab() {
    let db = Arc::new(setup_test_db().await);
    seed_operator(&db).await;
    seed_product(&db, SEED_PRODUCT_ID, "波次等级面料甲", "PRD-PL-0004").await;
    seed_customer(&db, SEED_CUSTOMER_ID, "波次等级客户丙").await;
    let svc = SalesPriceService::new(db.clone());

    let created = svc
        .create_price(create_input(None, None), OPERATOR_ID)
        .await
        .expect("缺等级建单必须成功");

    let req: UpdateSalesPriceInput =
        serde_json::from_value(json!({ "price_level": inventory_stock_grade::FIRST }))
            .expect("update 线格式必须收 price_level 键");
    let after = svc
        .update_price(created.id, req)
        .await
        .expect("一等品更新必须成功");
    assert_eq!(
        after.price_level.as_deref(),
        Some(inventory_stock_grade::FIRST),
        "update 透传分支缺失（回归）即此处保持 None"
    );

    let req: UpdateSalesPriceInput =
        serde_json::from_value(json!({ "price_level": "C级" })).expect("线格式必须可反序列化");
    let err = svc
        .update_price(created.id, req)
        .await
        .expect_err("伪词表 C级 更新必须 400");
    expect_validation_400(&err);
    let after = reload(&db, created.id).await;
    assert_eq!(
        after.price_level.as_deref(),
        Some(inventory_stock_grade::FIRST),
        "被拒更新不得改动等级"
    );
}

// ---------------------------------------------------------------------------
// 5. update expiry_date 三态（任务板 #169 统一口径）：缺键=保持 / 显式 null=清空 NULL /
//    给值=改值 / 非法值=400 且行不动。double_option 的"显式 null 不塌成缺键"在
//    线格式层直接取证（matches! 三态形状）。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn update_expiry_date_three_states_via_wire() {
    let db = Arc::new(setup_test_db().await);
    seed_operator(&db).await;
    seed_product(&db, SEED_PRODUCT_ID, "波次等级面料甲", "PRD-PL-0005").await;
    seed_customer(&db, SEED_CUSTOMER_ID, "波次等级客户丙").await;
    let svc = SalesPriceService::new(db.clone());

    let y2026_12_31 = NaiveDate::from_ymd_opt(2026, 12, 31).expect("夹具日期常量");
    let y2027_06_30 = NaiveDate::from_ymd_opt(2027, 6, 30).expect("夹具日期常量");
    let created = svc
        .create_price(create_input(None, Some("2026-12-31")), OPERATOR_ID)
        .await
        .expect("带合法到期日建单必须成功");
    assert_eq!(created.expiry_date, Some(y2026_12_31));

    // ① 缺键 = 保持原值
    let req: UpdateSalesPriceInput =
        serde_json::from_value(json!({ "price": "101.00" })).expect("缺键线格式必须可反序列化");
    assert!(
        req.expiry_date.is_none(),
        "键缺席必须反序列化为外层 None（三态之一）"
    );
    let after = svc
        .update_price(created.id, req)
        .await
        .expect("改价必须成功");
    assert_eq!(
        after.expiry_date,
        Some(y2026_12_31),
        "缺键不得动 expiry_date（塌成清空即三态回归）"
    );

    // ② 给值 = 改值
    let req: UpdateSalesPriceInput = serde_json::from_value(json!({ "expiry_date": "2027-06-30" }))
        .expect("线格式必须可反序列化");
    assert!(
        matches!(req.expiry_date, Some(Some(_))),
        "给值必须反序列化为 Some(Some(v))（三态之一）"
    );
    let after = svc
        .update_price(created.id, req)
        .await
        .expect("给合法日期更新必须成功");
    assert_eq!(after.expiry_date, Some(y2027_06_30), "给值必须改值");

    // ③ 显式 null = 清空为 NULL（此前无通道：前端清空到期日必然无效——F1b §6-7 证真）
    let req: UpdateSalesPriceInput = serde_json::from_value(json!({ "expiry_date": null }))
        .expect("显式 null 线格式必须可反序列化");
    assert!(
        matches!(req.expiry_date, Some(None)),
        "显式 null 必须反序列化为 Some(None)，绝不塌成缺键（double_option 失效即此红）"
    );
    let after = svc
        .update_price(created.id, req)
        .await
        .expect("清空到期日必须成功");
    assert_eq!(after.expiry_date, None, "显式 null 必须清空为 NULL");

    // ④ 非法日期 = 400 且行不动（绝不静默吞成 NULL 或其它）
    let req: UpdateSalesPriceInput = serde_json::from_value(json!({ "expiry_date": "31/12/2027" }))
        .expect("非法串仍是合法 JSON 字符串");
    let err = svc
        .update_price(created.id, req)
        .await
        .expect_err("非法日期格式必须 400");
    expect_validation_400(&err);
    let after = reload(&db, created.id).await;
    assert_eq!(
        after.expiry_date, None,
        "被拒的非法日期不得改动已清空的 NULL"
    );
}

// ---------------------------------------------------------------------------
// 6. update effective_date（NOT NULL 列）：显式 null 必须 fail-visible 拒绝
//    （照 department_service.rs:231-234 NOT NULL 门控先例），不得静默当成"键缺席"。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn update_effective_date_explicit_null_is_rejected_not_silent() {
    let db = Arc::new(setup_test_db().await);
    seed_operator(&db).await;
    seed_product(&db, SEED_PRODUCT_ID, "波次等级面料甲", "PRD-PL-0006").await;
    seed_customer(&db, SEED_CUSTOMER_ID, "波次等级客户丙").await;
    let svc = SalesPriceService::new(db.clone());

    let created = svc
        .create_price(create_input(None, None), OPERATOR_ID)
        .await
        .expect("建单必须成功");

    let req: UpdateSalesPriceInput = serde_json::from_value(json!({ "effective_date": null }))
        .expect("显式 null 必须可反序列化");
    assert!(
        matches!(req.effective_date, Some(None)),
        "effective_date 三态：显式 null 必须与缺键可区分"
    );
    let err = svc
        .update_price(created.id, req)
        .await
        .expect_err("NOT NULL 列显式 null 必须拒绝，不得静默保持原值");
    assert_eq!(
        err.error_code(),
        "BUSINESS_ERROR",
        "清空必填列的拒绝归业务族（department 先例同族），实得 {err:?}"
    );
    use axum::response::IntoResponse;
    assert_eq!(
        err.clone().into_response().status(),
        axum::http::StatusCode::BAD_REQUEST,
        "拒绝必须 400"
    );
    let after = reload(&db, created.id).await;
    assert_eq!(
        after.effective_date, created.effective_date,
        "被拒更新不得改动 effective_date"
    );
}

// ---------------------------------------------------------------------------
// 7. create expiry_date 非法串：F1b §6-7 证真的缺陷（旧 `.parse().ok()` 不报错不落值）
//    钉回潮面：必须 400 + VALIDATION_ERROR + 零副作用（不落行）。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn create_invalid_expiry_date_is_fail_visible_400_not_silent_null() {
    let db = Arc::new(setup_test_db().await);
    seed_operator(&db).await;
    seed_product(&db, SEED_PRODUCT_ID, "波次等级面料甲", "PRD-PL-0007").await;
    seed_customer(&db, SEED_CUSTOMER_ID, "波次等级客户丙").await;
    let svc = SalesPriceService::new(db.clone());

    for illegal in ["2026-13-45", "not-a-date", "31/12/2026"] {
        let err = svc
            .create_price(create_input(None, Some(illegal)), OPERATOR_ID)
            .await
            .expect_err(&format!(
                "非法到期日 {illegal} 必须 fail-visible，绝不静默吞成 NULL"
            ));
        expect_validation_400(&err);
    }

    let (rows, total) = svc
        .get_prices_list(query_params())
        .await
        .expect("列表查询必须成功");
    assert_eq!(total, 0, "被拒建单必须零副作用（不落行），实得 {rows:?}");
}

// ---------------------------------------------------------------------------
// 8. 查询参数漂移（#206/#160 同族）：keyword（产品名称/客户名称模糊）与 customer_id
//    后端已接收并下推谓词——命中、不命中、等值三类形态各钉一条。
//    改坏什么必红：谓词摘回"收了不用"→ 命中段红；JOIN 误用 InnerJoin → 无客户行丢、0 段仍绿
//    但命中段（客户 NULL 行按产品名命中）红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn list_keyword_and_customer_id_filters_take_effect() {
    let db = Arc::new(setup_test_db().await);
    seed_operator(&db).await;
    seed_product(&db, SEED_PRODUCT_ID, "波次等级面料甲", "PRD-PL-0008").await;
    seed_product(&db, OTHER_PRODUCT_ID, "无关布料乙", "PRD-PL-0009").await;
    seed_customer(&db, SEED_CUSTOMER_ID, "波次等级客户丙").await;
    let svc = SalesPriceService::new(db.clone());

    // row1：产品名/客户名都含"波次等级"；row2：无客户、名称不含
    let row1 = svc
        .create_price(create_input(None, None), OPERATOR_ID)
        .await
        .expect("row1 建单必须成功");
    let mut input2 = create_input(None, None);
    input2.product_id = OTHER_PRODUCT_ID;
    input2.customer_id = None;
    let row2 = svc
        .create_price(input2, OPERATOR_ID)
        .await
        .expect("row2 建单必须成功");

    // keyword 命中产品名称（LEFT JOIN 多对一，不倍增行）
    let mut p = query_params();
    p.keyword = Some("面料甲".to_string());
    let (rows, total) = svc
        .get_prices_list(p)
        .await
        .expect("keyword 列表查询必须成功");
    assert_eq!(total, 1, "产品名 keyword 必须只命中 row1，实得 ids 见 rows");
    assert_eq!(rows[0].id, row1.id);

    // keyword 命中客户名称（row2 客户为 NULL，LeftJoin 下仅不命中，不得被吞掉整行存在性）
    let mut p = query_params();
    p.keyword = Some("客户丙".to_string());
    let (rows, _total) = svc
        .get_prices_list(p)
        .await
        .expect("keyword 列表查询必须成功");
    assert_eq!(rows.len(), 1, "客户名 keyword 必须只命中 row1");
    assert_eq!(rows[0].id, row1.id);

    // keyword 全不命中
    let mut p = query_params();
    p.keyword = Some("绝对不存在的关键词丁".to_string());
    let (_rows, total) = svc
        .get_prices_list(p)
        .await
        .expect("keyword 列表查询必须成功");
    assert_eq!(total, 0, "全不命中必须空集（而非静默全量）");

    // customer_id 等值筛选：命中自建客户；row2（customer_id NULL）必须被排除
    let mut p = query_params();
    p.customer_id = Some(SEED_CUSTOMER_ID);
    let (rows, total) = svc
        .get_prices_list(p)
        .await
        .expect("customer_id 列表查询必须成功");
    assert_eq!(total, 1, "customer_id 等值必须只命中 row1");
    assert_eq!(rows[0].id, row1.id);

    // 无关客户 → 空集（防止谓词写成恒真兜底）
    let mut p = query_params();
    p.customer_id = Some(9999);
    let (_rows, total) = svc.get_prices_list(p).await.expect("列表查询必须成功");
    assert_eq!(
        total, 0,
        "未引用的 customer_id 必须零命中（row2 的 NULL 也不许被当成命中）"
    );
    let _ = row2;
}
