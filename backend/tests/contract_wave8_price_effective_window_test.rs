//! 定价域「生效窗口」谓词 + 采购到期日非法值 fail-visible —— 真库契约锁（后端线）
//!
//! 被测事实一（降级联动取错价）：
//! `services/quality_inspection_service.rs::sync_sales_price_for_downgrade` 查「产品当前生效的
//! A 级标准价」时，只过滤 `status = approved` 与 `price_level ∈ {一等品, NULL}`，再按
//! `effective_date DESC` 取一条 —— **既不校验 `effective_date <= 今天`，也完全不过滤
//! `expiry_date`**。后果：「未来才生效」或「已过期」的价目会被当成当前标准价，乘折扣因子后
//! 写回 sales_prices 成为二等品价，下游按错价销售。
//!
//! 被测事实二（采购到期日静默吞值）：
//! `services/purchase_price_service.rs::create_price` 的
//! `expiry_date: Set(req.expiry_date.and_then(|d| d.parse().ok()))` 把非法日期串静默吞成
//! NULL：请求被接受、行也落库，用户以为存进去了，实际列是 NULL（长期有效语义）。销售侧
//! `services/sales_price_service.rs::create_price` 的同名字段早已 fail-visible，采购侧漏改。
//!
//! 收紧后的口径（本文件逐条钉死）：
//! - 生效窗口 = **闭区间**：`effective_date <= 今天` 且（`expiry_date IS NULL` 或
//!   `expiry_date >= 今天`）——到期日当天仍然有效，到期日留空 = 长期有效。口径与本仓定价域
//!   既有先例 `utils/price_calculator.rs::find_customer_special_price` /
//!   `utils/price_calculator.rs::find_seasonal_adjustment` 同形；与工资域
//!   `services/wage_ops/rate.rs::get_effective_by_route` 的右开口径无关，两者不得互抄。
//!   「今天」取 `chrono::Utc::now().date_naive()`，与
//!   `utils/price_calculator.rs::calculate_price` 的 `calc_date` 兜底同源。
//! - 谓词单点定义在 `services/sales_price_service.rs::effective_on`，联动只经该函数使用它；
//!   本文件**不把谓词复制进测试自证**，全部通过真实服务口驱动被测谓词执行。
//! - 测到哪一层：正向与负向例都走真实公开入口
//!   `services/quality_inspection_service.rs::process_unqualified`（B 级 + downgrade_sale ⇒
//!   同步库存等级 ⇒ 联动销售价），即新谓词在完整链路中被真实执行。负向例（未来/已过期不得
//!   取用）额外断言 `unqualified_products.stock_grade_synced` 已为真且库存等级已被改为二等品，
//!   用以排除「链路根本没跑到联动」造成的假绿。
//! - 联动取不到生效价时的既有行为保持不变：warn 跳过、不阻断主流程、不报错。
//! - 采购 create 的非法 `expiry_date` 归 VALIDATION 族：HTTP 400 + 信封 `code=VALIDATION_ERROR`，
//!   且拒绝路径零副作用（不落行）。本文件**只断 status 与信封机器码，绝不断错误文案原文**
//!   （脱敏红线：文案是否外显由构造点声明，不属本锁范围）。
//! - 夹具为真库（`setup_test_db`，已迁移 PostgreSQL；缺 `TEST_DATABASE_URL` 直接 panic，
//!   禁静默回退 sqlite）；操作人/产品/客户/供应商/仓库/库存/质检记录全部自建并回读自增主键，
//!   **绝不硬编码 ID**，也不依赖「库恰好为空」；所有断言都以自种子的 product_id 收窄。
//! - 第 3 例把边界压在「到期日 = 今天」，依赖测试进程与服务口取到同一个 UTC 日期；跨 UTC
//!   零点重跑会如实报红而非静默放过，这是钉住右界闭区间的必要代价，明示不藏。

mod test_common;

use bingxi_backend::models::inventory_stock::REPLENISHMENT_REORDER_POINT;
use bingxi_backend::models::status::price_approval;
use bingxi_backend::models::status::purchase_inventory::{
    inventory_stock_grade, inventory_stock_quality_status, inventory_stock_status,
};
use bingxi_backend::models::status::quality_dyeing::{
    quality_inspection_result, quality_inspection_type,
};
use bingxi_backend::models::{
    customer, inventory_stock, product, purchase_price, quality_inspection_record, sales_price,
    supplier, unqualified_product, user, warehouse,
};
use bingxi_backend::services::purchase_price_service::{
    CreatePurchasePriceInput, PurchasePriceService,
};
use bingxi_backend::services::quality_inspection_service::{
    DOWNGRADE_PRICE_LEVEL_B, HANDLING_DOWNGRADE_SALE, ProcessUnqualifiedRequest, QUALITY_GRADE_B,
    QualityInspectionService, STANDARD_PRICE_LEVEL_A, downgrade_price_factor,
};
use bingxi_backend::utils::error::AppError;
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    Set,
};
use std::sync::Arc;
use test_common::setup_test_db;

fn now() -> DateTime<Utc> {
    Utc::now()
}

/// 与实现同源的「今天」口径（`utils/price_calculator.rs::calculate_price` 的 calc_date 兜底）
fn today() -> NaiveDate {
    Utc::now().date_naive()
}

/// 相对「今天」偏移若干天（负数=过去）；越界即 panic，不静默给出错日期
fn offset_day(base: NaiveDate, days: i64) -> NaiveDate {
    base.checked_add_signed(chrono::Duration::days(days))
        .expect("夹具日期偏移必须落在 NaiveDate 可表示范围内")
}

/// 造数唯一后缀：同一 UTC 日期内多次运行不撞唯一键（不赌「库恰好为空」）
fn nonce() -> String {
    Utc::now()
        .timestamp_nanos_opt()
        .expect("Utc::now 必落在 chrono 纳秒可表示区间（约 1678-2262 年），None 不可达")
        .to_string()
}

/// 折扣后的二等品期望价：与实现同源取 `downgrade_price_factor()`，绝不在测试里写死 0.8/80
fn expected_b_price(standard: Decimal) -> Decimal {
    (standard * downgrade_price_factor()).round_dp(4)
}

async fn seed_user(db: &Arc<DatabaseConnection>) -> i32 {
    let u = user::ActiveModel {
        username: Set(format!("w8win_op_{}", nonce())),
        password_hash: Set("test-only-not-a-real-hash".to_string()),
        real_name: Set(Some("生效窗口契约锁操作人".to_string())),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子操作人插入失败: {e}"));
    u.id
}

async fn seed_product(db: &Arc<DatabaseConnection>) -> i32 {
    let tag = nonce();
    let p = product::ActiveModel {
        name: Set(format!("生效窗口面料_{tag}")),
        code: Set(format!("PRD-W8WIN-{tag}")),
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
    p.id
}

async fn seed_customer(db: &Arc<DatabaseConnection>) -> i32 {
    let tag = nonce();
    let c = customer::ActiveModel {
        customer_code: Set(format!("CUS-W8WIN-{tag}")),
        customer_name: Set(format!("生效窗口客户_{tag}")),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
        // 词表安全侧成员（见 models/customer.rs 列注）
        customer_type: Set("retail".to_string()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子客户插入失败: {e}"));
    c.id
}

async fn seed_supplier(db: &Arc<DatabaseConnection>) -> i32 {
    let tag = nonce();
    let s = supplier::ActiveModel {
        supplier_code: Set(format!("SUP-W8WIN-{tag}")),
        supplier_name: Set(format!("生效窗口供应商_{tag}")),
        supplier_short_name: Set("窗供".to_string()),
        supplier_type: Set("面料供应商".to_string()),
        credit_code: Set("91330000TEST00000X".to_string()),
        registered_address: Set("测试注册地址".to_string()),
        legal_representative: Set("测试法人".to_string()),
        registered_capital: Set(Decimal::ZERO),
        establishment_date: Set(
            NaiveDate::from_ymd_opt(2020, 1, 1).expect("夹具日期常量 2020-01-01 必然合法")
        ),
        taxpayer_type: Set("一般纳税人".to_string()),
        bank_name: Set("测试银行".to_string()),
        bank_account: Set("6222000000000000".to_string()),
        contact_phone: Set("13800000000".to_string()),
        created_at: Set(now().into()),
        updated_at: Set(now().into()),
        // NOT NULL bool 列，显式给值，不赌库端默认形态
        is_processor: Set(false),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子供应商插入失败: {e}"));
    s.id
}

async fn seed_warehouse(db: &Arc<DatabaseConnection>) -> i32 {
    let tag = nonce();
    let w = warehouse::ActiveModel {
        warehouse_code: Set(format!("WH-W8WIN-{tag}")),
        name: Set(format!("生效窗口仓_{tag}")),
        is_default: Set(false),
        is_active: Set(true),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子仓库插入失败: {e}"));
    w.id
}

/// 种子销售价目行的参数（列逐一对 `models/sales_price.rs` 核对；收进结构体避免长参数表）
struct PriceSeed {
    product_id: i32,
    customer_id: Option<i32>,
    price: Decimal,
    effective_date: NaiveDate,
    expiry_date: Option<NaiveDate>,
    price_level: Option<String>,
    status: String,
}

/// A 级（标准）approved 价基线形态：`price_level = 一等品`（词表常量单源）、长期有效
fn a_grade_price(product_id: i32, price: Decimal, effective_date: NaiveDate) -> PriceSeed {
    PriceSeed {
        product_id,
        customer_id: None,
        price,
        effective_date,
        expiry_date: None,
        price_level: Some(STANDARD_PRICE_LEVEL_A.to_string()),
        status: price_approval::APPROVED.to_string(),
    }
}

async fn seed_sales_price(db: &Arc<DatabaseConnection>, s: PriceSeed) -> sales_price::Model {
    sales_price::ActiveModel {
        product_id: Set(s.product_id),
        customer_id: Set(s.customer_id),
        customer_type: Set(Some("standard".to_string())),
        price: Set(s.price),
        currency: Set("CNY".to_string()),
        unit: Set("米".to_string()),
        min_order_qty: Set(Decimal::ZERO),
        price_type: Set("standard".to_string()),
        price_level: Set(s.price_level),
        effective_date: Set(s.effective_date),
        expiry_date: Set(s.expiry_date),
        status: Set(s.status),
        approved_by: Set(None),
        approved_at: Set(Some(now())),
        created_by: Set(None),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子销售价目插入失败: {e}"))
}

/// B 级降级联动场景的全部前置种子：产品 + 仓库 + 库存（一等品）+ B 级质检记录。
/// 库存行必须与质检记录同 `product_id` 且同 `batch_no`，否则
/// `services/quality_inspection_service.rs::sync_stock_grade_for_downgrade` 定位不到库存，
/// 价格联动分支根本不会触发 —— 那会让「没生成二等品价」的负向例变成假绿。
struct DowngradeFixture {
    user_id: i32,
    product_id: i32,
    customer_id: i32,
    stock_id: i32,
    record_id: i32,
}

async fn seed_downgrade_fixture(db: &Arc<DatabaseConnection>) -> DowngradeFixture {
    let user_id = seed_user(db).await;
    let product_id = seed_product(db).await;
    let customer_id = seed_customer(db).await;
    let warehouse_id = seed_warehouse(db).await;
    let batch_no = format!("BATCH-W8WIN-{}", nonce());

    let stock = inventory_stock::ActiveModel {
        warehouse_id: Set(warehouse_id),
        product_id: Set(product_id),
        quantity_on_hand: Set(Decimal::new(10000, 2)),
        quantity_available: Set(Decimal::new(10000, 2)),
        quantity_reserved: Set(Decimal::ZERO),
        quantity_shipped: Set(Decimal::ZERO),
        quantity_incoming: Set(Decimal::ZERO),
        reorder_point: Set(Decimal::ZERO),
        max_stock_point: Set(Decimal::ZERO),
        reorder_quantity: Set(Decimal::ZERO),
        created_at: Set(now()),
        updated_at: Set(now()),
        batch_no: Set(batch_no.clone()),
        color_no: Set("COL-W8WIN".to_string()),
        dye_lot_no: Set(Some("DYE-W8WIN".to_string())),
        // 降级前库存是一等品（词表常量），联动后应被改成二等品
        grade: Set(STANDARD_PRICE_LEVEL_A.to_string()),
        quantity_meters: Set(Decimal::new(10000, 2)),
        quantity_kg: Set(Decimal::ZERO),
        stock_status: Set(inventory_stock_status::NORMAL.to_string()),
        quality_status: Set(inventory_stock_quality_status::PASS.to_string()),
        version: Set(0),
        replenishment_strategy: Set(REPLENISHMENT_REORDER_POINT.to_string()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子库存插入失败: {e}"));

    let record = quality_inspection_record::ActiveModel {
        inspection_no: Set(format!("QI-W8WIN-{}", nonce())),
        inspection_type: Set(quality_inspection_type::FINISHED.to_string()),
        related_type: Set(None),
        related_id: Set(None),
        product_id: Set(product_id),
        batch_no: Set(Some(batch_no)),
        supplier_id: Set(None),
        customer_id: Set(Some(customer_id)),
        inspection_date: Set(today()),
        inspector_id: Set(Some(user_id)),
        total_qty: Set(Decimal::new(10000, 2)),
        inspected_qty: Set(Decimal::new(10000, 2)),
        qualified_qty: Set(Some(Decimal::new(8800, 2))),
        unqualified_qty: Set(Some(Decimal::new(1200, 2))),
        // 88% 合格率落在 B 级区间（阈值与实现同源：services/quality_inspection_service.rs
        // ::grade_b_threshold / ::grade_a_threshold），等级值本身也取服务口常量
        qualification_rate: Set(Some(Decimal::new(88, 0))),
        inspection_result: Set(quality_inspection_result::UNQUALIFIED.to_string()),
        remark: Set(Some("生效窗口契约锁种子".to_string())),
        grade: Set(Some(QUALITY_GRADE_B.to_string())),
        color_no: Set(Some("COL-W8WIN".to_string())),
        dye_lot_no: Set(Some("DYE-W8WIN".to_string())),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子质检记录插入失败: {e}"));

    DowngradeFixture {
        user_id,
        product_id,
        customer_id,
        stock_id: stock.id,
        record_id: record.id,
    }
}

/// 走真实公开入口驱动 B 级降级链：process_unqualified ⇒ 同步库存等级 ⇒ 联动二等品价
async fn run_b_grade_downgrade(
    db: &Arc<DatabaseConnection>,
    fx: &DowngradeFixture,
) -> unqualified_product::Model {
    let svc = QualityInspectionService::new(db.clone());
    svc.process_unqualified(
        fx.record_id,
        ProcessUnqualifiedRequest {
            unqualified_qty: Decimal::new(1200, 2),
            unqualified_reason: "生效窗口契约锁：B 级让步接收降级销售".to_string(),
            handling_method: HANDLING_DOWNGRADE_SALE.to_string(),
            remark: None,
            handling_result: None,
        },
        fx.user_id,
    )
    .await
    .expect("B 级 + downgrade_sale 的处理入口必须成功（联动内部失败按既有口径只 warn 不阻断）")
}

/// 证明联动确实跑到了「同步库存等级 ⇒ 进入价格联动」那一步（排除链路空跑造成的假绿）
async fn assert_chain_reached_downgrade_sync(
    db: &Arc<DatabaseConnection>,
    fx: &DowngradeFixture,
    unqualified_id: i32,
) {
    let reloaded = unqualified_product::Entity::find_by_id(unqualified_id)
        .one(db.as_ref())
        .await
        .unwrap_or_else(|e| panic!("回读不合格品记录 {unqualified_id} 失败: {e}"))
        .unwrap_or_else(|| panic!("不合格品记录 {unqualified_id} 应已落库"));
    assert!(
        reloaded.stock_grade_synced,
        "降级联动必须已执行到库存等级同步（stock_grade_synced=true），否则本例的结论来自链路空跑而非谓词判定"
    );
    let stock = inventory_stock::Entity::find_by_id(fx.stock_id)
        .one(db.as_ref())
        .await
        .unwrap_or_else(|e| panic!("回读库存 {} 失败: {e}", fx.stock_id))
        .unwrap_or_else(|| panic!("库存 {} 应已落库", fx.stock_id));
    assert_eq!(
        stock.grade,
        inventory_stock_grade::SECOND,
        "库存等级应被联动改为二等品（词表常量），实得 {}",
        stock.grade
    );
}

async fn level_rows(
    db: &Arc<DatabaseConnection>,
    product_id: i32,
    level: &str,
) -> Vec<sales_price::Model> {
    sales_price::Entity::find()
        .filter(sales_price::Column::ProductId.eq(product_id))
        .filter(sales_price::Column::PriceLevel.eq(level))
        .all(db.as_ref())
        .await
        .unwrap_or_else(|e| panic!("查询产品 {product_id} 的 {level} 价目失败: {e}"))
}

fn purchase_input(
    product_id: i32,
    supplier_id: i32,
    expiry_date: Option<&str>,
) -> CreatePurchasePriceInput {
    CreatePurchasePriceInput {
        product_id,
        supplier_id,
        price: Decimal::new(10000, 2),
        currency: Some("CNY".to_string()),
        unit: "米".to_string(),
        price_type: "standard".to_string(),
        min_order_qty: None,
        effective_date: None,
        expiry_date: expiry_date.map(str::to_string),
    }
}

/// 校验拒绝必须 400 + VALIDATION_ERROR：只钉 status 与信封机器码，绝不断错误文案原文
/// （脱敏红线），并显式排除被拍平成 500 的形态
fn expect_validation_400(err: &AppError) {
    assert_eq!(
        err.error_code(),
        "VALIDATION_ERROR",
        "非法日期必须归 VALIDATION_ERROR 族，实得 {err:?}"
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
        "校验拒绝必须 400，禁止退化为 500（含被拍平成 internal 族）或其它状态"
    );
}

// ---------------------------------------------------------------------------
// 1. 只有「未来才生效」的 A 级 approved 价 ⇒ 联动不得用它生成/改写二等品价
//    改坏什么必红：把生效窗口谓词摘掉（回到只看 status/price_level）→ 联动拿未来价
//    把已有的 30.00 二等品价覆写成 100.00×因子，两段断言都红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn downgrade_sync_must_not_use_price_effective_only_in_the_future() {
    let db = Arc::new(setup_test_db().await);
    let fx = seed_downgrade_fixture(&db).await;
    // 唯一存在的 A 级标准价：30 天后才生效
    seed_sales_price(
        &db,
        a_grade_price(
            fx.product_id,
            Decimal::new(10000, 2),
            offset_day(today(), 30),
        ),
    )
    .await;
    // 已存在的二等品价（正确值 30.00），用来钉「不得被未来价覆写」
    seed_sales_price(
        &db,
        PriceSeed {
            product_id: fx.product_id,
            customer_id: None,
            price: Decimal::new(3000, 2),
            effective_date: offset_day(today(), -5),
            expiry_date: None,
            price_level: Some(DOWNGRADE_PRICE_LEVEL_B.to_string()),
            status: price_approval::APPROVED.to_string(),
        },
    )
    .await;

    let created = run_b_grade_downgrade(&db, &fx).await;
    assert_chain_reached_downgrade_sync(&db, &fx, created.id).await;

    let rows = level_rows(&db, fx.product_id, DOWNGRADE_PRICE_LEVEL_B).await;
    assert_eq!(rows.len(), 1, "二等品价不得被新增成多行，实得 {rows:?}");
    assert_eq!(
        rows[0].price,
        Decimal::new(3000, 2),
        "未来才生效的标准价不得参与二等品定价，原二等品价必须保持 30.00"
    );
}

// ---------------------------------------------------------------------------
// 2. 「已过期」（expiry_date < 今天）的 A 级价 ⇒ 同样不得被取用
//    改坏什么必红：只补 effective_date <= 今天而漏掉 expiry_date（本缺陷的另一半）→
//    该行生效日在过去、到期日已过，联动会造出二等品价，本例红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn downgrade_sync_must_not_use_expired_price() {
    let db = Arc::new(setup_test_db().await);
    let fx = seed_downgrade_fixture(&db).await;
    seed_sales_price(
        &db,
        PriceSeed {
            expiry_date: Some(offset_day(today(), -1)),
            ..a_grade_price(
                fx.product_id,
                Decimal::new(10000, 2),
                offset_day(today(), -400),
            )
        },
    )
    .await;

    let created = run_b_grade_downgrade(&db, &fx).await;
    assert_chain_reached_downgrade_sync(&db, &fx, created.id).await;

    let rows = level_rows(&db, fx.product_id, DOWNGRADE_PRICE_LEVEL_B).await;
    assert!(
        rows.is_empty(),
        "已过期的标准价不得被取用生成二等品价（取不到生效价时按既有口径只 warn 跳过），实得 {rows:?}"
    );
}

// ---------------------------------------------------------------------------
// 3. 到期日**当天**的 A 级价 ⇒ 必须被取用（右界闭区间；防有人日后把谓词改成右开）
//    改坏什么必红：`ExpiryDate.gte` 改成 `gt` → 当天行被排除、二等品价不生成，本例红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn downgrade_sync_must_use_price_expiring_today_closed_interval() {
    let db = Arc::new(setup_test_db().await);
    let fx = seed_downgrade_fixture(&db).await;
    let standard = Decimal::new(10000, 2);
    seed_sales_price(
        &db,
        PriceSeed {
            expiry_date: Some(today()),
            ..a_grade_price(fx.product_id, standard, offset_day(today(), -10))
        },
    )
    .await;

    let created = run_b_grade_downgrade(&db, &fx).await;
    assert_chain_reached_downgrade_sync(&db, &fx, created.id).await;

    let rows = level_rows(&db, fx.product_id, DOWNGRADE_PRICE_LEVEL_B).await;
    assert_eq!(
        rows.len(),
        1,
        "到期日当天的标准价必须被取用（闭区间窗口），应生成一条二等品价，实得 {rows:?}"
    );
    assert_eq!(
        rows[0].price,
        expected_b_price(standard),
        "二等品价必须等于窗口内标准价 × 折扣因子"
    );
}

// ---------------------------------------------------------------------------
// 4. `expiry_date IS NULL` ⇒ 长期有效，必须被取用；价格值也证明没写死（62.50 → ×因子）
//    改坏什么必红：谓词写成 `ExpiryDate.gte(date)` 而不 `OR is_null` → 长期有效行被误排除，本例红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn downgrade_sync_must_use_price_with_null_expiry_as_long_term() {
    let db = Arc::new(setup_test_db().await);
    let fx = seed_downgrade_fixture(&db).await;
    let standard = Decimal::new(6250, 2);
    seed_sales_price(
        &db,
        a_grade_price(fx.product_id, standard, offset_day(today(), -10)),
    )
    .await;

    let created = run_b_grade_downgrade(&db, &fx).await;
    assert_chain_reached_downgrade_sync(&db, &fx, created.id).await;

    let rows = level_rows(&db, fx.product_id, DOWNGRADE_PRICE_LEVEL_B).await;
    assert_eq!(
        rows.len(),
        1,
        "expiry_date 留空即长期有效，必须被取用，实得 {rows:?}"
    );
    assert_eq!(
        rows[0].price,
        expected_b_price(standard),
        "二等品价必须按窗口内标准价 × 折扣因子计算，不得写死数值"
    );
}

// ---------------------------------------------------------------------------
// 5. 窗口内外同时存在多行时，必须取「窗口内 effective_date 最新」那条，而非绝对最新那条
//    （旧实现 `ORDER BY effective_date DESC` 取一条 ⇒ 未来行永远压过当前生效行）
//    改坏什么必红：窗口谓词失效 → 取到 today+10 的 100.00 而不是当前生效的 50.00，本例红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn downgrade_sync_prefers_latest_row_within_window_not_absolute_latest() {
    let db = Arc::new(setup_test_db().await);
    let fx = seed_downgrade_fixture(&db).await;
    // 干扰行：未来生效、价格更高，且带客户（联动按产品维度取价，不按客户收窄）
    seed_sales_price(
        &db,
        PriceSeed {
            customer_id: Some(fx.customer_id),
            ..a_grade_price(
                fx.product_id,
                Decimal::new(10000, 2),
                offset_day(today(), 10),
            )
        },
    )
    .await;
    // 真正当前生效的标准价（price_level 为 NULL 的标准价语义同样要能取到）
    let current = Decimal::new(5000, 2);
    seed_sales_price(
        &db,
        PriceSeed {
            price_level: None,
            ..a_grade_price(fx.product_id, current, offset_day(today(), -5))
        },
    )
    .await;

    let created = run_b_grade_downgrade(&db, &fx).await;
    assert_chain_reached_downgrade_sync(&db, &fx, created.id).await;

    let rows = level_rows(&db, fx.product_id, DOWNGRADE_PRICE_LEVEL_B).await;
    assert_eq!(rows.len(), 1, "应只生成一条二等品价，实得 {rows:?}");
    assert_eq!(
        rows[0].price,
        expected_b_price(current),
        "必须取窗口内当前生效的 50.00 那条，而不是未来生效的 100.00"
    );
}

// ---------------------------------------------------------------------------
// 6. 采购 create 传非法 expiry_date 字符串 ⇒ 400 + VALIDATION_ERROR，
//    且不得落库成「expiry_date=NULL 的成功行」（静默吞值形态）
//    改坏什么必红：回到 `.and_then(|d| d.parse().ok())` → 三次调用都成功落行，
//    expect_err 与零落行断言双双红；换成 business/internal 通道 → 族/状态码断言红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn purchase_create_rejects_invalid_expiry_date_instead_of_silent_null() {
    let db = Arc::new(setup_test_db().await);
    let user_id = seed_user(&db).await;
    let product_id = seed_product(&db).await;
    let supplier_id = seed_supplier(&db).await;
    let svc = PurchasePriceService::new(db.clone());

    for illegal in ["2026-13-45", "not-a-date", "31/12/2026"] {
        let err = svc
            .create_price(
                purchase_input(product_id, supplier_id, Some(illegal)),
                user_id,
            )
            .await
            .expect_err(&format!(
                "非法到期日 {illegal} 必须 fail-visible 拒绝，绝不静默吞成 NULL"
            ));
        expect_validation_400(&err);
    }

    let landed = purchase_price::Entity::find()
        .filter(purchase_price::Column::ProductId.eq(product_id))
        .count(db.as_ref())
        .await
        .expect("统计采购价目行失败");
    assert_eq!(
        landed, 0,
        "被拒建单必须零副作用：不得留下 expiry_date=NULL 的『看起来成功』行，实得 {landed}"
    );
}

// ---------------------------------------------------------------------------
// 7. 采购 create 正向：合法日期照常落值、留空照常为 NULL（收口不得误杀既有正常路径）
// ---------------------------------------------------------------------------
#[tokio::test]
async fn purchase_create_still_accepts_valid_and_absent_expiry_date() {
    let db = Arc::new(setup_test_db().await);
    let user_id = seed_user(&db).await;
    let supplier_id = seed_supplier(&db).await;
    let svc = PurchasePriceService::new(db.clone());

    // 腿 A：合法日期字符串 ⇒ 原样落库
    let product_valid = seed_product(&db).await;
    let created = svc
        .create_price(
            purchase_input(product_valid, supplier_id, Some("2026-12-31")),
            user_id,
        )
        .await
        .expect("合法到期日必须照常创建成功（收口不得误杀）");
    assert_eq!(
        created.expiry_date,
        NaiveDate::from_ymd_opt(2026, 12, 31),
        "合法日期必须落为真实值，实得 {:?}",
        created.expiry_date
    );

    // 腿 B：键缺席 ⇒ NULL（长期有效语义），仍必须成功
    let product_absent = seed_product(&db).await;
    let created = svc
        .create_price(purchase_input(product_absent, supplier_id, None), user_id)
        .await
        .expect("到期日留空是长期有效语义，必须照常创建成功");
    assert!(
        created.expiry_date.is_none(),
        "留空必须落 NULL，实得 {:?}",
        created.expiry_date
    );
}

// ---------------------------------------------------------------------------
// 8. 防回潮源码扫描锁：谓词单点、右界闭区间、采购侧不再吞值
//    ①联动函数体内必须调用 effective_on，且不得再出现第二份窗口谓词字面量；
//    ②effective_on 本体必须是 lte + (is_null OR gte) 形态，出现右开写法即红；
//    ③采购 create_price 体内不得再有 `.parse().ok()` 吞值形态，且必须走可外显校验通道。
// ---------------------------------------------------------------------------
#[test]
fn source_scan_effective_window_is_single_sourced_and_closed_interval() {
    let sales_src = std::fs::read_to_string("src/services/sales_price_service.rs")
        .expect("读取 sales_price_service.rs 失败");
    let helper_start = sales_src
        .find("fn effective_on(")
        .expect("effective_on 谓词必须仍在（单点定义不得被删）");
    let helper_tail = &sales_src[helper_start..];
    let helper = helper_tail.split("\n/// ").next().unwrap_or(helper_tail);
    assert!(
        helper.contains("Column::EffectiveDate.lte(on_date)"),
        "生效窗口必须校验 effective_date <= 今天（漏掉即『未来价被当成当前价』回归）"
    );
    assert!(
        helper.contains("Column::ExpiryDate.is_null()"),
        "expiry_date IS NULL 必须视为长期有效（否则长期有效行被误排除）"
    );
    assert!(
        helper.contains("Column::ExpiryDate.gte(on_date)"),
        "到期日右界必须是闭区间 gte（与本仓定价域 price_calculator 口径同形）"
    );
    assert!(
        !helper.contains("Column::ExpiryDate.gt("),
        "禁止把右界改成开区间 gt（到期日当天即被误判过期，与本仓定价口径冲突）"
    );
    assert!(
        !helper.contains("Column::EffectiveDate.lt("),
        "禁止把左界改成开区间 lt（生效日当天即被误判未生效）"
    );

    let qi_src = std::fs::read_to_string("src/services/quality_inspection_service.rs")
        .expect("读取 quality_inspection_service.rs 失败");
    let sync_start = qi_src
        .find("async fn sync_sales_price_for_downgrade")
        .expect("降级联动函数必须仍在（能力不得被删）");
    let sync_tail = &qi_src[sync_start..];
    let sync_body = sync_tail.split("\n    /// ").next().unwrap_or(sync_tail);
    assert!(
        sync_body.contains("effective_on("),
        "降级联动必须复用单点谓词 effective_on（自造第二份谓词就是口径漂移的起点）"
    );
    assert!(
        !sync_body.contains("Column::EffectiveDate.lte(")
            && !sync_body.contains("Column::ExpiryDate.gte("),
        "联动函数内不得再出现窗口谓词字面量（必须只经 effective_on 一处定义）"
    );

    let po_src = std::fs::read_to_string("src/services/purchase_price_service.rs")
        .expect("读取 purchase_price_service.rs 失败");
    let create_start = po_src
        .find("pub async fn create_price")
        .expect("采购 create_price 必须仍在");
    let create_tail = &po_src[create_start..];
    let create_body = create_tail
        .split("\n    /// ")
        .next()
        .unwrap_or(create_tail);
    assert!(
        !create_body.contains(".parse().ok()"),
        "采购到期日解析不得回到 `parse().ok()` 静默吞 NULL 的形态"
    );
    assert!(
        create_body.contains("AppError::validation_displayable"),
        "非法日期必须走可外显的校验通道（business 会被脱敏、internal 会拍平成 500）"
    );
}
