//! 采购入库单列表四筛选项「真正生效」契约锁（wave6，）
//!
//! 缺陷形态（判责）：`GET /purchase/receipts` 的查询 DTO 只有
//! page/page_size/status/supplier_id/order_id，前端界面上的 keyword（单号/物料名）、
//! warehouse_id、receipt_date_from、receipt_date_to 四个筛选项后端整体不接收——
//! 用户怎么选都是同一份未过滤列表（静默功能缺失，非报错）。
//!
//! 本锁钉住补齐后的语义：
//! 1. 四个键进入真实 WHERE（purchase_receipt_ops/query.rs），且 `total` 与 items
//!    同源于**已过滤**查询（paginate_with_total 对同一 paginator 取数+计数，
//!    utils/pagination.rs:20-21），封死「items 过滤了、total 没过滤」第二形态；
//! 2. keyword 覆盖列 = `purchase_receipt.receipt_no`（models/purchase_receipt.rs:26）
//!    与 `purchase_receipt_item.material_name`（models/purchase_receipt_item.rs:18，
//!    入库明细自带物料名快照列，不 join products——本单明细即"产品名"权威来源）；
//!    明细为主表一对多，用 EXISTS 相关子查询匹配，禁止 LEFT JOIN 行倍增污染 total；
//! 3. 日期区间含首尾（receipt_date 为 DATE 列非时间戳，gte/lte 天然含两端日）；
//! 4. 空串/纯空白筛选在反序列化边界归一为 None（utils/query_params.rs 裁定，
//!    避免 `WHERE col=''` 恒 0 行）；非法日期 400 + code=VALIDATION_ERROR 信封
//!    （先例 contract_wave4_api_key_echo_and_expiry_test.rs 的裁定口径）。
//!
//! 覆盖策略（无 mock、真实 handler/service 调用）：
//! - 真 PostgreSQL 真跑（路线一， 判责；表结构唯一来源 = `backend/migration`，
//!   本文件不再自建同构 DDL）：经真实 handler `list_receipts`（Query DTO → service →
//!   分页信封）断言各筛选项命中集合、total 过滤后语义、EXISTS 防行倍增、空串归一
//!   （axum Query oneshot 抽取器真路径，先例 handlers_query_param_coercion_test.rs）、
//!   非法日期 VALIDATION_ERROR 信封；
//! - `#[ignore]` 活库用例（`test_common::setup_test_db()` → 已迁移 PG，缺变量由夹具
//!   直接 panic，禁止回退假绿）：仅 PG 专有语义部分——LIKE 大小写敏感（sqlite 的 LIKE
//!   对 ASCII 恒不区分大小写，且 sea-query 的 sqlite 构建器不支持 ILIKE 会直接 panic，
//!   见 query.rs 注释），小写关键字在 PG 上不得命中大写单号；日期含首尾用 2099 未来日
//!   播种隔离活库干扰数据。
//! - 外键父行按裁定 R1 自建（见 `seeded_state`）：`warehouses` 用显式主键 1/2 与筛选
//!   参数同源，`products` 自建取真实 id，`suppliers` 取迁移种子参照表的真实 id。
//! - 数据权限字段过滤（handler 内 data_permission 分支）与本契约正交，用例以
//!   role_id=None 的 AuthContext 走直通路径，不在此重复锁定。

mod test_common;

use axum::Router;
use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::purchase_receipt_handler::{self, ReceiptQueryParams};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::status::purchase_inventory::purchase_receipt_inspection;
use bingxi_backend::models::status::purchase_receipt as receipt_status;
use bingxi_backend::models::{product, purchase_receipt, purchase_receipt_item};
use bingxi_backend::services::purchase_receipt_service::PurchaseReceiptService;
use bingxi_backend::utils::error::AppError;
use chrono::{NaiveDate, TimeZone, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait,
    QueryFilter, Set,
};
use serde_json::Value as JsonValue;
use std::sync::Arc;
use tower::ServiceExt;

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

// =========================================================
// 真库种子（列与 models/*.rs + backend/migration 建表语句逐列对齐）
// =========================================================

/// 外键父行 `warehouses`：显式主键 1/2。
///
/// 为什么用显式主键而不是取序列返回值：用例的筛选参数与断言逐值锁定 1/2
/// （`warehouse_id=1` 命中两张、`=2` 命中一张、`=99` 必须零行），夹具与断言必须同源；
/// `setup_test_db()` 每次 `TRUNCATE … RESTART IDENTITY` 清空业务表，本用例随后只插
/// 入库单/明细（各走自己的序列），不会再有第三方仓库行与本主键相撞。
async fn seed_warehouses(db: &DatabaseConnection) {
    db.execute_raw(sea_orm::Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO warehouses (id, warehouse_code, name, is_active, is_default, created_at, updated_at) \
         VALUES (1,'WH-W6F-A','波次六入库筛选仓甲',TRUE,FALSE,NOW(),NOW()),\
                (2,'WH-W6F-B','波次六入库筛选仓乙',TRUE,FALSE,NOW(),NOW())",
        Vec::<sea_orm::Value>::new(),
    ))
    .await
    .expect("夹具：外键父行 warehouses(1,2) 写入失败");
}

/// 外键父行 `suppliers`：迁移种子参照表（m0015 播种、`setup_test_db` 不清空、不删其数据），
/// 取其真实首个 id 作 `purchase_receipt.supplier_id` 的父行，不硬编码常量。
async fn first_seeded_supplier_id(db: &DatabaseConnection) -> i32 {
    let row = db
        .query_one_raw(sea_orm::Statement::from_string(
            DbBackend::Postgres,
            "SELECT id FROM suppliers ORDER BY id LIMIT 1".to_string(),
        ))
        .await
        .expect("夹具：读取 suppliers 参照表失败")
        .unwrap_or_else(|| panic!("夹具：迁移未播种 suppliers 参照表（m0015），FK 父行缺失"));
    row.try_get_by_index::<i32>(0)
        .expect("夹具：suppliers.id 应可解码为 i32")
}

/// 外键父行 `products`：`purchase_receipt_item.product_id` 有 FK，业务表会被清空 ⇒ 自建。
async fn seed_product(db: &DatabaseConnection) -> i32 {
    product::ActiveModel {
        code: Set(format!(
            "PRD-W6F-{}",
            Utc::now().timestamp_nanos_opt().unwrap()
        )),
        name: Set("波次六入库筛选契约产品".to_string()),
        unit: Set("米".to_string()),
        // 状态 token 与写入方词表同源（models::status::master_data::ACTIVE = "active"）
        status: Set(bingxi_backend::models::status::master_data::ACTIVE.to_string()),
        product_type: Set("fabric".to_string()),
        is_deleted: Set(false),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("夹具：外键父行 products 写入失败")
    .id
}

/// 播种夹具：3 张入库单（每次进夹具即真空库，单号固定便于关键字断言）
/// - PRW6FLAG001：仓库 1，2026-08-10，明细「棉平纹坯布」
/// - PRW6FLAG002：仓库 2，2026-09-15，明细「灯芯绒面料」
/// - PRW6FLAG003：仓库 1，2026-09-30，明细「涤纶塔丝隆」「涤纶里布」两行同含「涤纶」
///   （专防 EXISTS→LEFT JOIN 退化导致的行倍增与 total 污染）
async fn seeded_state() -> AppState {
    let db = test_common::setup_test_db().await;
    seed_warehouses(&db).await;
    let supplier_id = first_seeded_supplier_id(&db).await;
    let product_id = seed_product(&db).await;
    let mut offset = 0i64;
    for (no, wh, d, materials) in [
        ("PRW6FLAG001", 1, date(2026, 8, 10), vec!["棉平纹坯布"]),
        ("PRW6FLAG002", 2, date(2026, 9, 15), vec!["灯芯绒面料"]),
        (
            "PRW6FLAG003",
            1,
            date(2026, 9, 30),
            vec!["涤纶塔丝隆", "涤纶里布"],
        ),
    ] {
        offset += 60;
        let r = purchase_receipt::ActiveModel {
            receipt_no: Set(no.to_string()),
            supplier_id: Set(supplier_id),
            receipt_date: Set(d),
            warehouse_id: Set(wh),
            // 状态 token 与写入方词表同源，不手写第二套常量
            inspection_status: Set(purchase_receipt_inspection::PENDING.to_string()),
            receipt_status: Set(receipt_status::DRAFT.to_string()),
            total_quantity: Set(Decimal::ZERO),
            total_quantity_alt: Set(Decimal::ZERO),
            total_amount: Set(Decimal::ZERO),
            // created_by 在真库里无 FK 约束（仅 sales_orders.created_by 有），
            // 与 AuthContext 的 user_id 同一操作人，本契约不依赖 users 行
            created_by: Set(1),
            created_at: Set(Utc.timestamp_opt(1_700_000_000 + offset, 0).unwrap()),
            updated_at: Set(Utc.timestamp_opt(1_700_000_000 + offset, 0).unwrap()),
            ..Default::default()
        }
        .insert(&db)
        .await
        .expect("播种入库单失败");
        for (i, m) in materials.iter().enumerate() {
            purchase_receipt_item::ActiveModel {
                receipt_id: Set(r.id),
                line_no: Set(i as i32 + 1),
                product_id: Set(product_id),
                material_code: Set("FAB-W6F".to_string()),
                material_name: Set(m.to_string()),
                quantity: Set(Decimal::ONE),
                unit_master: Set("米".to_string()),
                ..Default::default()
            }
            .insert(&db)
            .await
            .expect("播种入库明细失败");
        }
    }
    AppState {
        db: Arc::new(db),
        ..Default::default()
    }
}

fn make_auth() -> AuthContext {
    AuthContext {
        user_id: 1,
        username: "w6f_tester".to_string(),
        role_id: None,
        department_id: None,
        data_scope: None,
        dept_ids: None,
        dept_member_user_ids: None,
    }
}

fn params() -> ReceiptQueryParams {
    ReceiptQueryParams {
        page: Some(1),
        page_size: Some(20),
        status: None,
        supplier_id: None,
        order_id: None,
        keyword: None,
        warehouse_id: None,
        receipt_date_from: None,
        receipt_date_to: None,
    }
}

/// 真实 handler 调用 → 信封 JSON（{data:{items,total,page,page_size}}）
async fn call_list(state: &AppState, p: ReceiptQueryParams) -> JsonValue {
    let resp = purchase_receipt_handler::list_receipts(Query(p), State(state.clone()), make_auth())
        .await
        .expect("列表查询应成功");
    serde_json::to_value(resp.0).expect("信封序列化")
}

fn items(body: &JsonValue) -> Vec<String> {
    body["data"]["items"]
        .as_array()
        .unwrap_or_else(|| panic!("出参缺 data.items，实际: {body}"))
        .iter()
        .map(|i| {
            i["receipt_no"]
                .as_str()
                .expect("items 行应含 receipt_no")
                .to_string()
        })
        .collect()
}

fn total(body: &JsonValue) -> u64 {
    body["data"]["total"]
        .as_u64()
        .unwrap_or_else(|| panic!("出参缺 data.total，实际: {body}"))
}

fn sorted(mut v: Vec<String>) -> Vec<String> {
    v.sort();
    v
}

// =========================================================
// 基线：无任何筛选项回全量 3 条（后续各断言以此为对照，证明确由筛选驱动）
// =========================================================

#[tokio::test]
async fn baseline_without_filters_returns_all_three() {
    let state = seeded_state().await;
    let body = call_list(&state, params()).await;
    assert_eq!(total(&body), 3, "无筛选基线应为全量 3 条");
    assert_eq!(
        sorted(items(&body)),
        ["PRW6FLAG001", "PRW6FLAG002", "PRW6FLAG003"]
    );
}

// =========================================================
// 1) warehouse_id 等值筛选
// =========================================================

#[tokio::test]
async fn warehouse_filter_hits_only_matching_receipts() {
    let state = seeded_state().await;

    let mut p = params();
    p.warehouse_id = Some(1);
    let body = call_list(&state, p).await;
    assert_eq!(
        total(&body),
        2,
        "仓库 1 只有 001/003，total 不得恒等于全库 3"
    );
    assert_eq!(sorted(items(&body)), ["PRW6FLAG001", "PRW6FLAG003"]);

    let mut p = params();
    p.warehouse_id = Some(2);
    let body = call_list(&state, p).await;
    assert_eq!(total(&body), 1);
    assert_eq!(items(&body), ["PRW6FLAG002"]);

    // 不存在的仓库 → 0 行（修复前参数被忽略会回全量 3，此断言防假绿）
    let mut p = params();
    p.warehouse_id = Some(99);
    let body = call_list(&state, p).await;
    assert_eq!(total(&body), 0, "未命中仓库必须 0 行，而非未过滤全量");
    assert!(items(&body).is_empty());
}

// =========================================================
// 2) 日期区间含首尾（from==to 命中当日行；界外一日不命中）
// =========================================================

#[tokio::test]
async fn date_range_is_inclusive_on_both_ends() {
    let state = seeded_state().await;

    // 含首：from == to == 09-15 必须命中当日那张（两端都是闭区间）
    let mut p = params();
    p.receipt_date_from = Some("2026-09-15".to_string());
    p.receipt_date_to = Some("2026-09-15".to_string());
    let body = call_list(&state, p).await;
    assert_eq!(total(&body), 1, "from==to 必须含当日行（含首尾）");
    assert_eq!(items(&body), ["PRW6FLAG002"]);

    // 只给下界
    let mut p = params();
    p.receipt_date_from = Some("2026-09-15".to_string());
    let body = call_list(&state, p).await;
    assert_eq!(sorted(items(&body)), ["PRW6FLAG002", "PRW6FLAG003"]);

    // 只给上界
    let mut p = params();
    p.receipt_date_to = Some("2026-09-15".to_string());
    let body = call_list(&state, p).await;
    assert_eq!(sorted(items(&body)), ["PRW6FLAG001", "PRW6FLAG002"]);

    // 真空窗：08-11 ~ 09-14 落在 08-10 与 09-15 两行种子之间，三行全部在界外 ⇒ 0 行
    // （证明边界真实参与比较）。原判据写"08-11~09-29 → 空集"是**假空窗**：
    // 该区间含 09-15（PRW6FLAG002），返回 1 行才是正确契约，断 0 属断言侧算错
    // （正解即"改真空窗"，不得把期望改成 1——那会把
    // 空窗防线丢掉）。"两端闭"这一真实判据已由上面三条通过证明。
    let mut p = params();
    p.receipt_date_from = Some("2026-08-11".to_string());
    p.receipt_date_to = Some("2026-09-14".to_string());
    let body = call_list(&state, p).await;
    assert_eq!(total(&body), 0, "真空窗不得混入任何行");
    assert!(items(&body).is_empty());
}

// =========================================================
// 3) keyword：单号列 或 明细物料名；多明细命中不倍增行/total
// =========================================================

#[tokio::test]
async fn keyword_matches_receipt_no_or_item_material_name_without_fanout() {
    let state = seeded_state().await;

    // 命中入库单号列（purchase_receipt.receipt_no）
    let mut p = params();
    p.keyword = Some("PRW6FLAG002".to_string());
    let body = call_list(&state, p).await;
    assert_eq!(total(&body), 1);
    assert_eq!(items(&body), ["PRW6FLAG002"]);

    // 命中明细物料名列（purchase_receipt_item.material_name，经 EXISTS 子查询）
    let mut p = params();
    p.keyword = Some("灯芯绒".to_string());
    let body = call_list(&state, p).await;
    assert_eq!(items(&body), ["PRW6FLAG002"]);

    // 「涤纶」同时命中 003 的两行明细：EXISTS 语义必须只回 1 行、total=1
    // （若实现退化为 LEFT JOIN，本断言会拿到 2 行/total 2）
    let mut p = params();
    p.keyword = Some("涤纶".to_string());
    let body = call_list(&state, p).await;
    assert_eq!(items(&body), ["PRW6FLAG003"], "多明细命中不得行倍增");
    assert_eq!(total(&body), 1, "total 不得随明细命中数膨胀");

    // 部分片段关键字（LIKE %kw% 语义）
    let mut p = params();
    p.keyword = Some("W6FLAG00".to_string());
    let body = call_list(&state, p).await;
    assert_eq!(total(&body), 3, "共享片段应命中全部三张单号");

    // 未命中关键字 → 0 行（修复前该参数整体不存在/被忽略，会回全量）
    let mut p = params();
    p.keyword = Some("zzz不存在的关键字".to_string());
    let body = call_list(&state, p).await;
    assert_eq!(total(&body), 0);
    assert!(items(&body).is_empty());
}

// =========================================================
// 4) 组合筛选 + total/items 同步（分页语义）
// =========================================================

#[tokio::test]
async fn combined_filters_total_follows_items() {
    let state = seeded_state().await;

    let mut p = params();
    p.warehouse_id = Some(1);
    p.receipt_date_from = Some("2026-09-01".to_string());
    p.keyword = Some("塔丝隆".to_string());
    let body = call_list(&state, p).await;
    assert_eq!(items(&body), ["PRW6FLAG003"]);
    assert_eq!(total(&body), 1, "组合条件下 total 必须是过滤后行数");
    assert_eq!(
        total(&body),
        items(&body).len() as u64,
        "page_size=20 未翻页时 total 与 items 长度一致"
    );

    // 组合条件把唯一候选滤空（仓库 2 + 日期只含 08-10）
    let mut p = params();
    p.warehouse_id = Some(2);
    p.receipt_date_to = Some("2026-08-31".to_string());
    let body = call_list(&state, p).await;
    assert_eq!(total(&body), 0);
    assert!(items(&body).is_empty());
}

// =========================================================
// 5) 空串/纯空白在反序列化边界归一为 None（axum Query 真实抽取路径，
//    先例 handlers_query_param_coercion_test.rs）
// =========================================================

async fn probe_list(Query(q): Query<ReceiptQueryParams>) -> String {
    let opt = |v: &Option<String>| {
        v.as_deref()
            .map_or("NONE".to_string(), |s| format!("SOME({s})"))
    };
    format!(
        "kw={}|from={}|to={}|wh={}",
        opt(&q.keyword),
        opt(&q.receipt_date_from),
        opt(&q.receipt_date_to),
        q.warehouse_id
            .map_or("NONE".to_string(), |v| format!("SOME({v})"))
    )
}

async fn probe(uri: &str) -> String {
    let app = Router::new().route("/q", get(probe_list));
    let resp = app
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "探针请求应抽取成功: {uri}");
    let bytes = axum::body::to_bytes(resp.into_body(), 64 * 1024)
        .await
        .unwrap();
    String::from_utf8_lossy(&bytes).to_string()
}

#[tokio::test]
async fn empty_and_blank_filters_deserialize_to_none() {
    // 页面未填写的筛选项以 ?keyword=&receipt_date_from= 形态提交 → None
    // （依据 utils/query_params.rs 头注裁定：缺失键与空串在边界收敛为同一语义）。
    // 注：warehouse_id 等数值键的空串剔除由全局中间件
    // normalize_empty_query_params（bootstrap/middleware_bootstrap.rs:135/359）负责，
    // 探针 Router 不含该层，故此处只锁字段级 empty_str_as_none 的三个字符串键。
    assert_eq!(
        probe("/q?keyword=&receipt_date_from=&receipt_date_to=").await,
        "kw=NONE|from=NONE|to=NONE|wh=NONE"
    );
    // 纯空白同样视为未提供（is_empty_query_value 口径）
    assert_eq!(
        probe("/q?keyword=%20%20&receipt_date_from=+").await,
        "kw=NONE|from=NONE|to=NONE|wh=NONE"
    );
    // 合法值原样保留、不裁剪
    assert_eq!(
        probe("/q?keyword=%E6%B6%A4%E7%BA%B6&warehouse_id=2").await,
        "kw=SOME(涤纶)|from=NONE|to=NONE|wh=SOME(2)"
    );
}

// =========================================================
// 6) 非法日期 → 400 + code=VALIDATION_ERROR + 真实文案外显（信封裁定）
// =========================================================

#[tokio::test]
async fn invalid_date_params_return_validation_error_envelope() {
    let state = seeded_state().await;
    for (field, bad) in [
        ("receipt_date_from", "2026-13-45"),
        ("receipt_date_to", "2026-09-15T00:00"),
        ("receipt_date_from", "not-a-date"),
    ] {
        let mut p = params();
        match field {
            "receipt_date_from" => p.receipt_date_from = Some(bad.to_string()),
            _ => p.receipt_date_to = Some(bad.to_string()),
        }
        let err =
            purchase_receipt_handler::list_receipts(Query(p), State(state.clone()), make_auth())
                .await
                .expect_err("非法日期必须被拒绝，不得静默套默认值吞掉筛选");
        let body = err.to_response();
        assert_eq!(
            body.code, "VALIDATION_ERROR",
            "字段取值错误裁定为 VALIDATION_ERROR，实际: {body:?} (bad={bad})"
        );
        assert!(
            body.message.contains(field),
            "拒绝文案必须点名字段，实际: {} (bad={bad})",
            body.message
        );
        let resp: Response = err.into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "非法日期必须 400");
    }
    // 确认是 AppError 构造口径（防 to_response 与类型脱钩）
    let e = AppError::validation_displayable("x".to_string());
    assert_eq!(e.error_code(), "VALIDATION_ERROR");
}

// =========================================================
// 7) 活库（PG 专有语义）：LIKE 大小写敏感 + 未来日隔离的含首尾区间
// =========================================================

/// 与 contract_wave6_receipt_gate_test.rs 同款夹具断言：缺变量显式失败，禁止 sqlite 回退假绿
async fn require_postgres(db: &DatabaseConnection) {
    let url = std::env::var("TEST_DATABASE_URL");
    assert!(
        matches!(&url, Ok(u) if u.starts_with("postgres")),
        "本用例锁定 PG 专有的 LIKE 大小写敏感语义（sqlite 的 LIKE 对 ASCII 恒不区分、\
         且 sea-query sqlite 构建器不支持 ILIKE 会 unimplemented!() panic），\
         必须跑在 TEST_DATABASE_URL 指向的已迁移 PostgreSQL 上（ci-test-rust-ignored 注入）。\
         当前 TEST_DATABASE_URL={url:?}"
    );
    assert_eq!(
        db.get_database_backend(),
        DbBackend::Postgres,
        "夹具解析出的后端不是 PostgreSQL，活库锁不可信"
    );
}

#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL（PG 专有 LIKE 大小写语义）；FK 父行（warehouses/products/suppliers）由本用例自建，不依赖环境已有主数据"]
async fn live_postgres_keyword_case_sensitivity_and_inclusive_range() {
    let db = test_common::setup_test_db().await;
    require_postgres(&db).await;
    // 外键父行自建（裁定 R1）：仓库/产品按夹具同款方式落库，供应商取迁移种子参照表真实 id
    seed_warehouses(&db).await;
    let supplier_id = first_seeded_supplier_id(&db).await;
    let product_id = seed_product(&db).await;
    let suffix = Utc::now().timestamp_nanos_opt().unwrap();
    let no_a = format!("PRW6L{suffix}A");
    let no_b = format!("PRW6L{suffix}B");
    // 2099 未来日播种：活库真实业务数据不会落在这两天，区间计数确定无干扰
    for (i, (no, d)) in [
        (no_a.as_str(), date(2099, 1, 5)),
        (no_b.as_str(), date(2099, 1, 7)),
    ]
    .into_iter()
    .enumerate()
    {
        let r = purchase_receipt::ActiveModel {
            receipt_no: Set(no.to_string()),
            supplier_id: Set(supplier_id),
            receipt_date: Set(d),
            warehouse_id: Set(1),
            inspection_status: Set(purchase_receipt_inspection::PENDING.to_string()),
            receipt_status: Set(receipt_status::DRAFT.to_string()),
            total_quantity: Set(Decimal::ZERO),
            total_quantity_alt: Set(Decimal::ZERO),
            total_amount: Set(Decimal::ZERO),
            created_by: Set(1),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap_or_else(|e| panic!("活库播种入库单失败: {e}"));
        purchase_receipt_item::ActiveModel {
            // PG 下 insert 经 RETURNING 直接带回真实 id，无需回读；
            // 明细仅关键字命中用，物料名带唯一后缀
            receipt_id: Set(r.id),
            line_no: Set(i as i32 + 1),
            product_id: Set(product_id),
            material_code: Set("FAB-W6L".to_string()),
            material_name: Set(format!("活库关键字坯布{suffix}")),
            quantity: Set(Decimal::ONE),
            unit_master: Set("米".to_string()),
            ..Default::default()
        }
        .insert(&db)
        .await
        .expect("活库播种入库明细失败");
    }

    let svc = PurchaseReceiptService::new(Arc::new(db.clone()));

    // 日期含首尾（活库 DATE 列语义与真库筛选用例的交叉验证）
    let (_, t) = svc
        .list_receipts(
            1,
            20,
            None,
            None,
            None,
            None,
            None,
            Some(date(2099, 1, 5)),
            Some(date(2099, 1, 5)),
        )
        .await
        .expect("from==to 查询应成功");
    assert_eq!(t, 1, "PG 上 from==to 必须含当日行");

    // keyword 大写命中
    let (rows, t) = svc
        .list_receipts(
            1,
            20,
            None,
            None,
            None,
            Some(format!("PRW6L{suffix}")),
            None,
            None,
            None,
        )
        .await
        .expect("关键字查询应成功");
    assert_eq!(t, 2, "唯一后缀关键字应恰命中本用例两张");
    assert_eq!(rows.len(), 2);

    // PG 专有：LIKE 大小写敏感——小写变体不得命中大写单号（sqlite 上此断言不成立，
    // 故只能在活库锁；钉住与采购订单列表（crud.rs:695-702）同一 LIKE 口径，
    // 未来若全域改 ILIKE 需一并评估此锁）
    let (rows, t) = svc
        .list_receipts(
            1,
            20,
            None,
            None,
            None,
            Some(format!("prw6l{suffix}").to_lowercase()),
            None,
            None,
            None,
        )
        .await
        .expect("小写关键字查询应成功");
    assert_eq!(t, 0, "PG 的 LIKE 大小写敏感：小写关键字不得命中大写单号");
    assert!(rows.is_empty());

    // 清理
    let ids: Vec<i32> = purchase_receipt::Entity::find()
        .filter(purchase_receipt::Column::ReceiptNo.is_in(vec![no_a.clone(), no_b.clone()]))
        .all(&db)
        .await
        .unwrap()
        .into_iter()
        .map(|r| r.id)
        .collect();
    purchase_receipt_item::Entity::delete_many()
        .filter(purchase_receipt_item::Column::ReceiptId.is_in(ids.clone()))
        .exec(&db)
        .await
        .expect("清理活库入库明细失败");
    purchase_receipt::Entity::delete_many()
        .filter(purchase_receipt::Column::ReceiptNo.is_in(vec![no_a, no_b]))
        .exec(&db)
        .await
        .expect("清理活库入库单失败");
}
