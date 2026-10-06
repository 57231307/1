//! 销售趋势端点聚合契约锁（`GET /api/v1/erp/crm/sales-analysis/trends`，别名 `/trend`）
//!
//! ## 锁定的真实契约（唯一真相 = 源码）
//! - `backend/src/services/sales_analysis_service.rs::get_trends`：**现算 `sales_orders`**、
//!   按 `granularity` 分桶的时间序列；聚合本体复用
//!   `backend/src/services/bi_analysis_ops/sales.rs::sales_by_time`（数据源、状态排除门、
//!   数据范围注入与分桶表达式的唯一实现点），出参行由 service 的**唯一映射点**
//!   `to_trend_point` 产出（`total_amount→amount`、`profit_amount→profit` 只许改这一处）。
//! - 数据源判据：`sales_statistics` 对实际销售零写入方（全仓 `sales_analysis::ActiveModel`
//!   仅 target 两处），读它必然结构性恒空；真实成交只落在 `sales_orders`。
//! - `backend/src/handlers/sales_analysis_handler.rs` 的 `TrendQuery`：
//!   `granularity` 非法 ⇒ 回落 `month` 并 `tracing::warn` 留痕（不 400、不静默）；
//!   `start_date`/`end_date` 成对且 `YYYY-MM-DD`，`end_date < start_date` ⇒ 400 +
//!   `code=VALIDATION_ERROR`；两者缺省 ⇒ 按粒度回看 12 桶。
//! - `period` 语义为**桶键等值过滤**，`Option` 缺省不过滤的已提交语义保持不变（不退回必填）；
//!   空串 `?period=` 由 `normalize_empty_query_params` 剔除 ⇒ 与键缺失同义。本测试自建路由
//!   显式挂同一中间件，两条形态都在真实链路下验证。
//!
//! ## 五条锁（缺一即本文件不完整）
//! 1. 正向非空：种跨两月真实销售单 ⇒ 桶数>0、桶键升序、每桶金额**字符串等值**＝直查
//!    `sales_orders` 的该桶 Decimal 和、订单数正确；
//! 2. 空态：无任何销售的窗口 ⇒ `200 + data: []`（空桶而非报错），与 1 同库共存，
//!    防"永远返回 []"再次溜过健康探针；
//! 3. 形状锁：每桶键集合恰为 {period, amount, order_count, quantity, profit}，
//!    金额/数量/利润=字符串（本仓 Decimal=字符串口径）、order_count=数字、period=桶键形态；
//! 4. 非法 granularity ⇒ 200 且与 `month` 粒度结果逐字节一致（回落留痕）；
//! 5. `end_date < start_date` ⇒ 400 + `code=VALIDATION_ERROR`。
//!
//! ## 夹具口径（真库自证）
//! `test_common::setup_test_db()` 打已迁移 PostgreSQL 并 TRUNCATE 业务表
//! （`sales_orders`/`customers` 均属业务表 ⇒ 每次从空表起步）。为防「夹具把迁移台账一起
//! 清掉 ⇒ 真库断言空转」的历史事故形态，用例显式自证：迁移台账 `seaql_migrations`
//! 记账行数 > 0，且自插 seed 行数经 Entity 直读确为预期值。
//! 种子日期沿用本仓 BI 真库种子范式（正午 UTC、专属远年份 2095 + 一条近月单）：
//! 正午 UTC 保证任何 |偏移|≤12 的会话时区下 `to_char` 日/月桶不落邻日邻月；
//! 2095 为本仓全量 e2e/tests grep 无占用的专属年，窗口参数天然唯一。
//! 金额一律取有限两位小数（Decimal→f64→两位字符串往返无损，桶值可精确字符串比对）。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{Next, from_fn, from_fn_with_state},
    response::Response,
    routing::get,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::sales_analysis_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::sales_order;
use bingxi_backend::models::status::sales::sales_order as so_status;
use bingxi_backend::services::sales_analysis_service::{
    SalesAnalysisService, SalesTrendPoint, SalesTrendQueryParams,
};
use bingxi_backend::utils::cache::AppCache;
use bingxi_backend::utils::query_params::normalize_empty_query_params;
use chrono::{Datelike, Months, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait, PaginatorTrait,
    QueryFilter, Statement,
};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use tower::ServiceExt;

/// 2095 种单主键与客户 FK 行 id（业务表逐用例 TRUNCATE，固定 id 无碰撞面）
const SEED_CUSTOMER_ID: i32 = 101;

/// 跨两月的三条有效销售单：日取 10/20/10 正午 UTC（月桶对会话时区免疫）。
/// (id, order_no, 日期, 金额, 期望月桶)
const SEED_ORDERS: [(&str, &str, &str, &str); 3] = [
    ("2095-03-10T12:00:00Z", "TW11-O1", "1000.50", "2095-03"),
    ("2095-03-20T12:00:00Z", "TW11-O2", "2000.25", "2095-03"),
    ("2095-04-10T12:00:00Z", "TW11-O3", "500.00", "2095-04"),
];
/// 正向窗口的月桶期望（桶键升序；金额为两位小数字符串，与 DECIMAL(14,2) 刻度一致）
const EXPECTED_BUCKETS: [(&str, &str, i64); 2] =
    [("2095-03", "3000.75", 2), ("2095-04", "500.00", 1)];

/// 近月种子单（缺省窗口锁专用）：今天回拨 6 个月的 10 日正午 UTC，金额与 2095 各值互异
const RECENT_AMOUNT: &str = "777.77";

/// 出参桶的恰键集合（形状锁判据；多键/少键同判负）
const EXPECTED_KEYS: [&str; 5] = ["period", "amount", "order_count", "quantity", "profit"];

/// 真库自证 1：迁移台账仍可读且记账行数 > 0（台账被误清 ⇒ 一切「已迁移」前提空转，必须点名）
async fn assert_migration_ledger_nonempty(db: &DatabaseConnection) {
    let row = db
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT COUNT(*) AS cnt FROM \"seaql_migrations\"".to_string(),
        ))
        .await
        .expect("真库自证：迁移台账 seaql_migrations 必须可读（夹具连的不是已迁移 PostgreSQL？）");
    let count: i64 = row
        .and_then(|r| r.try_get_by_index::<i64>(0).ok())
        .expect("真库自证：迁移台账记账行数解码失败");
    assert!(
        count > 0,
        "真库自证：迁移台账行数为 0——本用例的库不是经迁移记账建起来的，真库断言会在空库上空转"
    );
}

/// customers 为 sales_orders 的 FK 父行（m0001_initial_schema 约束
/// `fk_sales_orders_customer`）；customer_type 显式给真实词表值（列 DDL 可空而模型非
/// Option，缺列种 NULL 会让任何按模型读 customers 的链型错）
async fn seed_customer(db: &DatabaseConnection) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        format!(
            "INSERT INTO customers (id, customer_code, customer_name, customer_type) VALUES ($1, $2, $3, '{}')",
            bingxi_backend::constants::customer_type::RETAIL
        ),
        vec![
            SEED_CUSTOMER_ID.into(),
            "TW11-TREND-C1".to_string().into(),
            "趋势聚合锁种子客户".to_string().into(),
        ],
    ))
    .await
    .unwrap_or_else(|e| panic!("夹具：自插 customers 种子行失败: {e}"));
}

/// 种一条有效销售单（pending：状态排除门只拒 cancelled/draft，pending 如实计入聚合）
async fn seed_order(db: &DatabaseConnection, order_no: &str, day_iso_utc: &str, amount: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        r#"INSERT INTO sales_orders (order_no, customer_id, order_date, total_amount, status)
           VALUES ($1, $2, $3::timestamptz, $4, $5)"#,
        vec![
            order_no.to_string().into(),
            SEED_CUSTOMER_ID.into(),
            day_iso_utc.to_string().into(),
            amount
                .parse::<Decimal>()
                .expect("夹具金额字面量必须是合法 Decimal")
                .into(),
            so_status::PENDING.into(),
        ],
    ))
    .await
    .unwrap_or_else(|e| panic!("夹具：自插 sales_orders 行 {order_no} 失败: {e}"));
}

/// 种 2095 三条跨两月订单（正向/空态/形状/回落/倒置 五锁的共用数据集）
async fn seed_trend_orders(db: &DatabaseConnection) {
    seed_customer(db).await;
    for (day, order_no, amount, _bucket) in SEED_ORDERS {
        seed_order(db, order_no, day, amount).await;
    }
}

/// 近月种子单日期：今天(Utc)回拨 6 个月的 10 日；10 日正午在 |偏移|≤12 的会话时区下
/// 日与月都不会跨界
fn recent_seed_day() -> NaiveDate {
    Utc::now()
        .date_naive()
        .checked_sub_months(Months::new(6))
        .expect("当前日期回拨 6 个月必在 NaiveDate 有效范围内")
        .with_day(10)
        .expect("day=10 对任意月份恒合法")
}

/// 真库自证 2：Entity 直读 sales_orders，按 order_date 的 %Y-%m 归桶求 Decimal 和，
/// 产出「桶键 → (金额字符串, 订单数)」——这就是与端点对照的**直查侧**（不复用端点自身结果）
async fn direct_bucket_truth(db: &DatabaseConnection) -> BTreeMap<String, (String, i64)> {
    let rows = sales_order::Entity::find()
        .filter(sales_order::Column::Status.eq(so_status::PENDING))
        .all(db)
        .await
        .expect("夹具：直读 sales_orders 失败");
    let mut acc: BTreeMap<String, (Decimal, i64)> = BTreeMap::new();
    for r in rows {
        let key = r.order_date.format("%Y-%m").to_string();
        let entry = acc.entry(key).or_insert((Decimal::ZERO, 0));
        entry.0 += r.total_amount;
        entry.1 += 1;
    }
    acc.into_iter()
        .map(|(k, (sum, count))| (k, (sum.to_string(), count)))
        .collect()
}

fn make_auth() -> AuthContext {
    AuthContext {
        user_id: 7,
        username: "sales_trend_aggregate_fixture_user".to_string(),
        role_id: Some(2),
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

/// 只挂本端点的两条真实挂载路径，并复刻应用边界的 query 归一化层
/// （注册在后 = 更外层，故 `normalize_empty_query_params` 先于鉴权与 handler 执行，与生产一致）
fn build_app(db: Arc<DatabaseConnection>) -> Router {
    Router::new()
        .route(
            "/sales-analysis/trends",
            get(sales_analysis_handler::get_trends),
        )
        .route(
            "/sales-analysis/trend",
            get(sales_analysis_handler::get_trends),
        )
        .with_state(AppState {
            db,
            ..Default::default()
        })
        .layer(from_fn_with_state(make_auth(), inject_auth))
        .layer(from_fn(normalize_empty_query_params))
}

/// 发一次 GET，返回（状态码, 完整信封 JSON）。
/// 成功路径不预先反序列化成行模型——形状锁必须直接钉 `serde_json::Value` 的键与类型。
async fn get_envelope(db: &Arc<DatabaseConnection>, uri: &str) -> (StatusCode, Value) {
    let app = build_app(Arc::clone(db));
    let req = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let v: Value = serde_json::from_slice(&bytes).unwrap_or_else(|e| {
        panic!(
            "响应体必须是 JSON 信封，uri={uri} 解析失败: {e}，实际: {}",
            String::from_utf8_lossy(&bytes)
        )
    });
    (status, v)
}

/// 断成功信封并取出 `data` 数组（原始 Value 形态，供形状/内容两类断言共用）
async fn get_data_array(db: &Arc<DatabaseConnection>, uri: &str) -> Vec<Value> {
    let (status, v) = get_envelope(db, uri).await;
    assert_eq!(status, StatusCode::OK, "uri={uri} 应 200，实际信封: {}", v);
    assert_eq!(
        v["code"], 200,
        "成功信封 code 契约（utils/response.rs::ApiResponse），uri={uri}"
    );
    match &v["data"] {
        Value::Array(rows) => rows.clone(),
        other => panic!("data 必须是数组（契约：出参恒为数组），uri={uri} 实际: {other}"),
    }
}

// =========================================================
// 锁 1：正向非空——桶数>0、桶键升序、金额字符串等值＝直查 sales_orders、订单数正确
// =========================================================

#[tokio::test]
async fn trends_month_buckets_equal_direct_sales_orders_sum() {
    let db = Arc::new(test_common::setup_test_db().await);
    assert_migration_ledger_nonempty(&db).await;
    seed_trend_orders(&db).await;

    // 前置自证：直读确为 3 单（seed 没进库则后面的对照全部无意义）
    let seeded = sales_order::Entity::find()
        .count(&*db)
        .await
        .expect("夹具：直读 sales_orders 行数失败");
    assert_eq!(seeded, 3, "seed 必须是 3 条销售单，否则桶值对照无意义");

    let truth = direct_bucket_truth(&db).await;
    let buckets = get_data_array(
        &db,
        "/sales-analysis/trends?granularity=month&start_date=2095-03-01&end_date=2095-04-30",
    )
    .await;

    assert_eq!(
        buckets.len(),
        EXPECTED_BUCKETS.len(),
        "跨两月种单必须产出恰 2 个月桶（稀疏不补零），实际: {buckets:?}"
    );
    let keys: Vec<&str> = buckets
        .iter()
        .map(|b| b["period"].as_str().expect("period 必须是字符串"))
        .collect();
    assert_eq!(
        keys,
        vec![EXPECTED_BUCKETS[0].0, EXPECTED_BUCKETS[1].0],
        "桶键必须升序且与种子月份一致（字符串升序＝时间升序）"
    );
    for (i, bucket) in buckets.iter().enumerate() {
        let (want_period, want_amount, want_count) = EXPECTED_BUCKETS[i];
        let (truth_amount, truth_count) = truth
            .get(want_period)
            .unwrap_or_else(|| panic!("直查侧应存在桶 {want_period}，实际: {truth:?}"));
        assert_eq!(
            bucket["amount"].as_str(),
            Some(truth_amount.as_str()),
            "桶 {want_period} 金额字符串必须逐字符等于直查 sales_orders 的 Decimal 和（双保险：期望常量 {want_amount}）"
        );
        assert_eq!(
            bucket["amount"].as_str(),
            Some(want_amount),
            "直查和还必须等于用例内期望常量，排除夹具与期望同时漂移"
        );
        assert_eq!(
            bucket["order_count"].as_i64(),
            Some(*truth_count),
            "桶 {want_period} 订单数必须等于直查计数"
        );
        assert_eq!(
            bucket["order_count"].as_i64(),
            Some(want_count),
            "桶 {want_period} 订单数还必须等于期望常量"
        );
    }
}

// =========================================================
// 锁 2：空态——无销售窗口返回空桶数组而不是报错
// =========================================================

#[tokio::test]
async fn trends_empty_window_returns_empty_array_not_error() {
    let db = Arc::new(test_common::setup_test_db().await);
    assert_migration_ledger_nonempty(&db).await;
    seed_trend_orders(&db).await;

    // 同库先证正向窗口非空（排除"端点永远返回 []"的恒空壳形态）
    let nonempty = get_data_array(
        &db,
        "/sales-analysis/trends?granularity=month&start_date=2095-03-01&end_date=2095-04-30",
    )
    .await;
    assert!(
        !nonempty.is_empty(),
        "对照面前置：种单窗口必须非空，否则空态断言与恒空壳不可区分"
    );

    let empty = get_data_array(
        &db,
        "/sales-analysis/trends?granularity=month&start_date=2095-08-01&end_date=2095-08-31",
    )
    .await;
    assert!(
        empty.is_empty(),
        "窗口内真无销售必须 200 + data: []（空桶而非报错、而非补零假桶），实际: {empty:?}"
    );
}

// =========================================================
// 锁 3：形状——每桶键集合恰为 {period,amount,order_count,quantity,profit}，
//        金额/数量/利润=字符串（Decimal=字符串口径）、order_count=数字、桶键定宽形态
// =========================================================

#[tokio::test]
async fn trends_bucket_shape_lock() {
    let db = Arc::new(test_common::setup_test_db().await);
    assert_migration_ledger_nonempty(&db).await;
    seed_trend_orders(&db).await;

    let buckets = get_data_array(
        &db,
        "/sales-analysis/trends?granularity=month&start_date=2095-03-01&end_date=2095-04-30",
    )
    .await;
    assert!(
        !buckets.is_empty(),
        "形状锁必须钉在真实桶上（正向窗口空转 = 形状锁失效）"
    );
    for bucket in &buckets {
        let obj = bucket
            .as_object()
            .unwrap_or_else(|| panic!("每个桶必须是 JSON 对象，实际: {bucket}"));
        let keys: HashSet<&str> = obj.keys().map(|k| k.as_str()).collect();
        let expected: HashSet<&str> = EXPECTED_KEYS.iter().copied().collect();
        assert_eq!(
            keys, expected,
            "桶键集合必须恰为 {{period,amount,order_count,quantity,profit}}，多键少键同判负，实际: {keys:?}"
        );
        // 逐键类型：金额线形态必须是字符串（禁 JSON number＝财务口径禁浮点）
        let period = bucket["period"].as_str().expect("period 必须是字符串");
        assert!(
            regex_lite_month_key(period),
            "月粒度桶键形态必须为 YYYY-MM，实际: {period}"
        );
        for k in ["amount", "quantity", "profit"] {
            let s = bucket[k].as_str().unwrap_or_else(|| {
                panic!("{k} 必须是字符串（Decimal=字符串口径），实际: {bucket}")
            });
            assert!(
                regex_lite_two_decimal(s),
                "{k} 必须是两位小数形态的字符串，实际: {s}"
            );
        }
        assert!(
            bucket["order_count"].is_i64(),
            "order_count 必须是整数（i64 出参），实际: {bucket}"
        );
    }
}

/// 桶键 YYYY-MM 形态的最小判定（不引外部 regex 依赖，按字符逐位核）
fn regex_lite_month_key(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 7
        && b[0].is_ascii_digit()
        && b[1].is_ascii_digit()
        && b[2].is_ascii_digit()
        && b[3].is_ascii_digit()
        && b[4] == b'-'
        && b[5].is_ascii_digit()
        && b[6].is_ascii_digit()
}

/// 两位小数十进制串形态：<digits>.<digit><digit>
fn regex_lite_two_decimal(s: &str) -> bool {
    let Some((int, frac)) = s.split_once('.') else {
        return false;
    };
    !int.is_empty()
        && int.bytes().all(|c| c.is_ascii_digit())
        && frac.len() == 2
        && frac.bytes().all(|c| c.is_ascii_digit())
}

// =========================================================
// 锁 4：非法 granularity —— 200 且回落月桶，与 month 粒度结果完全一致（留痕不 400 不静默）
// =========================================================

#[tokio::test]
async fn trends_invalid_granularity_falls_back_to_month() {
    let db = Arc::new(test_common::setup_test_db().await);
    assert_migration_ledger_nonempty(&db).await;
    seed_trend_orders(&db).await;

    let window = "start_date=2095-03-01&end_date=2095-04-30";
    let month_buckets = get_data_array(
        &db,
        &format!("/sales-analysis/trends?granularity=month&{window}"),
    )
    .await;
    let fallback_buckets = get_data_array(
        &db,
        &format!("/sales-analysis/trends?granularity=weekly&{window}"),
    )
    .await;

    assert!(
        !fallback_buckets.is_empty(),
        "非法粒度必须照常返回月桶数据（回落+留痕处置），不能空转"
    );
    assert_eq!(
        fallback_buckets, month_buckets,
        "非法 granularity 的回落结果必须与 month 粒度逐字节一致（词表外值不另造口径）"
    );
    for bucket in &fallback_buckets {
        let period = bucket["period"].as_str().expect("period 字符串");
        assert!(
            regex_lite_month_key(period),
            "回落后桶键必须是月形态 YYYY-MM，实际: {period}"
        );
    }
}

// =========================================================
// 锁 5：区间倒置 —— 400 + code=VALIDATION_ERROR（机器码判点在复用聚合，不吞不 500）
// =========================================================

#[tokio::test]
async fn trends_reversed_window_returns_400_validation_error() {
    let db = Arc::new(test_common::setup_test_db().await);
    assert_migration_ledger_nonempty(&db).await;
    seed_trend_orders(&db).await;

    let (status, v) = get_envelope(
        &db,
        "/sales-analysis/trends?granularity=month&start_date=2095-04-01&end_date=2095-03-01",
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "end_date < start_date 必须 400（不许静默吞、不许 500），实际信封: {v}"
    );
    assert_eq!(
        v["code"], "VALIDATION_ERROR",
        "倒置区间的机器码必须是 VALIDATION_ERROR（AppError 统一信封），实际: {v}"
    );
}

/// 成对契约与形态非法同族：只给一半 ⇒ 400 + VALIDATION_ERROR（判点在窗口解析处，
/// 缺另一半时无法确定窗口，不允许静默按缺省窗口执行）
#[tokio::test]
async fn trends_half_date_pair_returns_400_validation_error() {
    let db = Arc::new(test_common::setup_test_db().await);
    assert_migration_ledger_nonempty(&db).await;
    seed_trend_orders(&db).await;

    let (status, v) = get_envelope(
        &db,
        "/sales-analysis/trends?granularity=month&start_date=2095-04-01",
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "只给 start_date 必须 400，实际: {v}"
    );
    assert_eq!(
        v["code"], "VALIDATION_ERROR",
        "机器码必须是 VALIDATION_ERROR，实际: {v}"
    );

    let (status_bad, v_bad) = get_envelope(
        &db,
        "/sales-analysis/trends?granularity=month&start_date=2095-04-01&end_date=04-01-2095",
    )
    .await;
    assert_eq!(
        status_bad,
        StatusCode::BAD_REQUEST,
        "日期形态非法必须 400，实际: {v_bad}"
    );
    assert_eq!(
        v_bad["code"], "VALIDATION_ERROR",
        "日期形态非法的机器码必须是 VALIDATION_ERROR，实际: {v_bad}"
    );
}

// =========================================================
// 骨架保留（缺省不过滤/空串剔除/别名同语义）：出参换桶后三条骨架逐一升格
// =========================================================

/// 缺省（不带 granularity/日期/period）＝ 月粒度 + 回看 12 桶 + 不过滤。
/// 种近月单证明缺省窗口真的覆盖当期并回读到真实桶；2095 数据在窗口外 ⇒ 不得混入。
#[tokio::test]
async fn trends_default_window_uses_recent_month_buckets() {
    let db = Arc::new(test_common::setup_test_db().await);
    assert_migration_ledger_nonempty(&db).await;
    seed_trend_orders(&db).await;
    let recent_day = recent_seed_day();
    seed_order(
        &db,
        "TW11-ORECENT",
        &format!("{recent_day}T12:00:00Z"),
        RECENT_AMOUNT,
    )
    .await;
    let recent_key = recent_day.format("%Y-%m").to_string();

    let buckets = get_data_array(&db, "/sales-analysis/trends").await;
    let keys: Vec<&str> = buckets
        .iter()
        .map(|b| b["period"].as_str().expect("period 字符串"))
        .collect();
    assert!(
        keys.contains(&recent_key.as_str()),
        "缺省请求必须命中近月种子桶 {recent_key}（缺省≠恒空），实际桶键: {keys:?}"
    );
    let recent = buckets
        .iter()
        .find(|b| b["period"].as_str() == Some(recent_key.as_str()))
        .expect("近月桶存在性上面已断");
    assert_eq!(
        recent["amount"].as_str(),
        Some(RECENT_AMOUNT),
        "近月桶金额必须=种子单金额（缺省窗口回读的是真数据）"
    );
    for k in &keys {
        assert!(
            !k.starts_with("2095"),
            "缺省回看 12 桶不得混入 2095 窗口外种子，实际: {k}"
        );
    }
}

/// 空串 period 与键缺失同义（经真实边界中间件）：同样得到缺省全桶
#[tokio::test]
async fn trends_with_empty_period_is_treated_as_absent() {
    let db = Arc::new(test_common::setup_test_db().await);
    assert_migration_ledger_nonempty(&db).await;
    seed_trend_orders(&db).await;

    let absent = get_data_array(
        &db,
        "/sales-analysis/trends?granularity=month&start_date=2095-03-01&end_date=2095-04-30",
    )
    .await;
    let empty_str = get_data_array(
        &db,
        "/sales-analysis/trends?granularity=month&start_date=2095-03-01&end_date=2095-04-30&period=",
    )
    .await;
    assert_eq!(
        empty_str, absent,
        "`?period=` 被边界中间件剔除后必须与键缺失同桶集；若为空说明生成了桶键=''的恒 0 过滤"
    );
}

/// 提供 period ⇒ 桶键等值过滤只剩该桶（"过滤真的生效"，不是把原始行当桶）
#[tokio::test]
async fn trends_with_period_filters_to_that_bucket_only() {
    let db = Arc::new(test_common::setup_test_db().await);
    assert_migration_ledger_nonempty(&db).await;
    seed_trend_orders(&db).await;

    let buckets = get_data_array(
        &db,
        "/sales-analysis/trends?granularity=month&start_date=2095-03-01&end_date=2095-04-30&period=2095-03",
    )
    .await;
    assert_eq!(buckets.len(), 1, "period=2095-03 必须只剩该桶");
    assert_eq!(buckets[0]["period"].as_str(), Some("2095-03"));
    assert_eq!(
        buckets[0]["amount"].as_str(),
        Some("3000.75"),
        "该桶金额必须=该月两单之和"
    );
}

/// 别名 `/trend` 与 `/trends` 挂同一 handler（routes/crm.rs 两处挂载），
/// 同参数的桶必须逐字节一致——防止只修一条路径、别名继续撞旧契约。
#[tokio::test]
async fn trend_alias_shares_bucket_semantics() {
    let db = Arc::new(test_common::setup_test_db().await);
    assert_migration_ledger_nonempty(&db).await;
    seed_trend_orders(&db).await;

    let window = "granularity=month&start_date=2095-03-01&end_date=2095-04-30";
    let plural = get_data_array(&db, &format!("/sales-analysis/trends?{window}")).await;
    let alias = get_data_array(&db, &format!("/sales-analysis/trend?{window}")).await;
    assert_eq!(
        alias, plural,
        "别名 /trend 的桶语义必须与 /trends 完全一致（同一 handler）"
    );
}

/// service 层新签名直测：粒度归一、桶键过滤、非法粒度回落与 HTTP 层同口径
/// （若有人把参数退回 `Option<&str>` 旧签名，本用例第一行就编译不过）
#[tokio::test]
async fn service_get_trends_buckets_period_filter_and_granularity_fallback() {
    let db = Arc::new(test_common::setup_test_db().await);
    seed_trend_orders(&db).await;

    let svc = SalesAnalysisService::new(Arc::clone(&db));
    let scope = make_auth().to_data_scope_context();
    let base = SalesTrendQueryParams {
        granularity: Some("month".to_string()),
        start_date: Some("2095-03-01".to_string()),
        end_date: Some("2095-04-30".to_string()),
        period: None,
    };

    let all: Vec<SalesTrendPoint> = svc
        .get_trends(base.clone(), scope.clone(), AppCache::arc())
        .await
        .expect("缺省 period 不得报错，且必须是不过滤语义");
    assert_eq!(all.len(), 2, "None ⇒ 窗口内全部 2 个桶");
    assert_eq!(all[0].period, "2095-03");
    assert_eq!(all[0].amount, "3000.75");
    assert_eq!(all[0].order_count, 2);

    let one = svc
        .get_trends(
            SalesTrendQueryParams {
                period: Some("2095-04".to_string()),
                ..base.clone()
            },
            scope.clone(),
            AppCache::arc(),
        )
        .await
        .expect("提供桶键 period 时不得报错");
    assert_eq!(one.len(), 1, "Some(2095-04) ⇒ 该桶 1 个");
    assert_eq!(one[0].amount, "500.00", "单桶过滤返回的必须是该桶金额");

    let fallback = svc
        .get_trends(
            SalesTrendQueryParams {
                granularity: Some("daily".to_string()),
                ..base
            },
            scope,
            AppCache::arc(),
        )
        .await
        .expect("非法粒度回落路径不得报错");
    assert_eq!(
        fallback
            .iter()
            .map(|p| (p.period.clone(), p.amount.clone()))
            .collect::<Vec<_>>(),
        all.iter()
            .map(|p| (p.period.clone(), p.amount.clone()))
            .collect::<Vec<_>>(),
        "service 层非法粒度回落必须与 month 粒度同桶同值（与 HTTP 层锁 4 同口径）"
    );
}
