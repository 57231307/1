//! 仪表盘销售统计按角色数据范围过滤的活体契约锁。
//!
//! 被测对象：`DashboardService::get_sales_statistics`
//! （由 `dashboard_handler::get_sales_statistics` 以会话 `DataScopeContext`
//! 经 `new_with_data_scope` 注入调用），覆盖两条取数通路——
//! `query_daily_sales_amounts`（SeaORM builder → `apply_department_scope`）与
//! `query_sales_by_dimension`（raw SQL → `build_data_scope_sql`），
//! 以及 `build_sales_cache_key` 的范围维度。
//!
//! 本锁钉住的契约面：
//! 1. 非管理员按角色范围过滤（self/dept/all），不得看到越权行；
//! 2. 缓存键纳入数据范围，不同范围会话不互相命中。
//!
//! 播种用私有 ID 段（部门 9620/9621、用户 9611~9613、客户 9601、产品 9501、单号前缀 WS14-），
//! 断言只落在私有归属人/私有桶上，因此并行分片不受其它用例污染；
//! 时间窗取 2096 年，越窗行落 2097-06-01，草稿行与 NULL 归属行各一条用于负向判据。

mod test_common;
use test_common::setup_test_db;

use std::str::FromStr;
use std::sync::Arc;

use bingxi_backend::models::status::sales::sales_order as so_status;
use bingxi_backend::services::dashboard_service::{DashboardService, SalesStatistics};
use bingxi_backend::utils::cache::AppCache;
use bingxi_backend::utils::data_scope::{DataScope, DataScopeContext};
use chrono::{DateTime, TimeZone, Utc};
use rust_decimal::Decimal;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, Value};

const DEPT_SELF: i32 = 9620;
const DEPT_OTHER: i32 = 9621;
/// 会话本人（self/dept 的归属人）
const USER_SELF: i32 = 9611;
/// 同部门的他人：dept 可见、self 不可见
const USER_OWNER: i32 = 9612;
/// 他部门：self 与 dept 都不可见
const USER_OTHER: i32 = 9613;
/// 无任何可见行的归属人（零命中判据）
const USER_NO_ROWS: i32 = 9619;
const CUST: i32 = 9601;
const PRODUCT: i32 = 9501;

const NAME_SELF: &str = "ws14_self";
const NAME_OWNER: &str = "ws14_owner";
const NAME_OTHER: &str = "ws14_other";

fn window() -> (DateTime<Utc>, DateTime<Utc>) {
    (
        Utc.with_ymd_and_hms(2096, 1, 1, 0, 0, 0).unwrap(),
        Utc.with_ymd_and_hms(2096, 12, 31, 23, 59, 59).unwrap(),
    )
}

/// 按会话身份拼装数据范围上下文：dept_ids 为可见部门集合（空集合应退化为仅本人）
fn ctx(scope: DataScope, user_id: i32, dept_ids: Vec<i32>) -> DataScopeContext {
    let degraded = dept_ids.is_empty();
    DataScopeContext {
        scope,
        user_id,
        department_id: if degraded { None } else { Some(DEPT_SELF) },
        dept_ids,
        dept_member_user_ids: if degraded {
            vec![user_id]
        } else {
            vec![user_id, USER_OWNER]
        },
    }
}

async fn exec(db: &DatabaseConnection, sql: &str, values: Vec<Value>) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        values,
    ))
    .await
    .unwrap_or_else(|e| panic!("播种失败: {e}\nSQL={sql}"));
}

async fn seed(db: &DatabaseConnection) {
    for (id, code, name) in [
        (DEPT_SELF, "WS14-DSELF", "销售范围本部门"),
        (DEPT_OTHER, "WS14-DOTHER", "销售范围他部门"),
    ] {
        exec(
            db,
            r#"INSERT INTO departments (id, code, name) VALUES ($1, $2, $3)
               ON CONFLICT (id) DO NOTHING"#,
            vec![id.into(), code.into(), name.into()],
        )
        .await;
    }
    for (uid, uname, dept) in [
        (USER_SELF, NAME_SELF, DEPT_SELF),
        (USER_OWNER, NAME_OWNER, DEPT_SELF),
        (USER_OTHER, NAME_OTHER, DEPT_OTHER),
        (USER_NO_ROWS, "ws14_empty", DEPT_OTHER),
    ] {
        exec(
            db,
            r#"INSERT INTO users (id, username, password_hash, email, is_active, is_totp_enabled, department_id)
               VALUES ($1, $2, 'x', $2, true, false, $3)
               ON CONFLICT (id) DO NOTHING"#,
            vec![uid.into(), uname.into(), dept.into()],
        )
        .await;
    }
    exec(
        db,
        &format!(
            "INSERT INTO customers (id, customer_code, customer_name, customer_type, owner_id, created_by, department_id) \
             VALUES ($1,'WS14-C','WS14客户','{}',$2,$2,$3) ON CONFLICT (id) DO NOTHING",
            bingxi_backend::constants::customer_type::RETAIL
        ),
        vec![CUST.into(), USER_SELF.into(), DEPT_SELF.into()],
    )
    .await;
    exec(
        db,
        r#"INSERT INTO products (id, code, name) VALUES ($1,'WS14-P','WS14产品')
           ON CONFLICT (id) DO NOTHING"#,
        vec![PRODUCT.into()],
    )
    .await;

    // (单号, 归属人, 数据部门, 日期, 金额分, 状态)
    type SeedRow = (
        &'static str,
        Option<i32>,
        Option<i32>,
        &'static str,
        i64,
        &'static str,
    );
    let rows: [SeedRow; 6] = [
        (
            "WS14-o1",
            Some(USER_SELF),
            Some(DEPT_SELF),
            "2096-03-01",
            30000,
            so_status::PENDING,
        ),
        (
            "WS14-o2",
            Some(USER_OWNER),
            Some(DEPT_SELF),
            "2096-04-01",
            50000,
            so_status::PENDING,
        ),
        (
            "WS14-o3",
            Some(USER_OTHER),
            Some(DEPT_OTHER),
            "2096-05-01",
            70000,
            so_status::APPROVED,
        ),
        (
            "WS14-o4",
            None,
            None,
            "2096-06-01",
            40000,
            so_status::PENDING,
        ),
        (
            "WS14-o5",
            Some(USER_SELF),
            Some(DEPT_SELF),
            "2097-06-01",
            99900,
            so_status::PENDING,
        ),
        (
            "WS14-o6",
            Some(USER_SELF),
            Some(DEPT_SELF),
            "2096-07-01",
            12300,
            so_status::DRAFT,
        ),
    ];
    for (no, owner, dept, day, cents, status) in rows {
        exec(
            db,
            r#"INSERT INTO sales_orders (order_no, customer_id, order_date, total_amount, status, created_by, department_id)
               VALUES ($1, $2, $3::timestamptz, $4, $5, $6, $7)"#,
            vec![
                no.into(),
                CUST.into(),
                day.to_string().into(),
                Decimal::new(cents, 2).into(),
                status.into(),
                Value::Int(owner),
                Value::Int(dept),
            ],
        )
        .await;
    }
    exec(
        db,
        r#"INSERT INTO sales_order_items (order_id, product_id, quantity, unit_price, subtotal, total_amount, color_no, is_deleted)
           SELECT id, $1, 1, 300, 300, 300, 'WS14', false FROM sales_orders WHERE order_no='WS14-o1'"#,
        vec![PRODUCT.into()],
    )
    .await;
}

/// 调用生产端点实现取数：service 每次新建，缓存把手可按需共享以验证范围隔离
async fn fetch(
    db: &Arc<DatabaseConnection>,
    cache: Arc<AppCache>,
    scope_ctx: Option<DataScopeContext>,
) -> SalesStatistics {
    let (start, end) = window();
    let svc = match scope_ctx {
        Some(c) => DashboardService::new_with_data_scope(db.clone(), cache, c),
        None => DashboardService::new(db.clone(), cache),
    };
    svc.get_sales_statistics(Some(start), Some(end))
        .await
        .unwrap_or_else(|e| panic!("销售统计端点取数失败（此处即 500 现场）: {e:?}"))
}

fn money(raw: &str) -> Decimal {
    Decimal::from_str(raw).unwrap_or_else(|e| panic!("金额出参应为十进制文本，实得 {raw}：{e}"))
}

/// 按销售员桶取值；缺失即判红（桶名来自播种用户名，并行分片不会污染）
fn bucket(stats: &SalesStatistics, name: &str) -> (Decimal, i64) {
    let hit = stats
        .by_salesperson
        .iter()
        .find(|row| row.name == name)
        .unwrap_or_else(|| panic!("分维结果缺少私有桶 {name}，实得 {:?}", stats.by_salesperson));
    (money(&hit.amount), hit.count)
}

fn has_bucket(stats: &SalesStatistics, name: &str) -> bool {
    stats.by_salesperson.iter().any(|row| row.name == name)
}

fn daily_total(stats: &SalesStatistics) -> Decimal {
    stats
        .daily_sales
        .iter()
        .map(|p| money(&p.amount))
        .sum::<Decimal>()
}

fn daily_dates(stats: &SalesStatistics) -> Vec<String> {
    let mut d: Vec<String> = stats.daily_sales.iter().map(|p| p.date.clone()).collect();
    d.sort();
    d
}

// —— 一、All 范围未被误收：他人行仍可见，越窗行仍被日期参数剔除 ——
// All 范围不加行级过滤，同窗内还有其它用例（多个契约测试同样在 2096 年前后种销售订单）
// 并行落库，且分维 raw SQL 带 `ORDER BY total_amount DESC LIMIT 20`——按桶名断精确值会被
// 并行分片挤出名次而假红。故此条只走 SeaORM 日销通路（无 LIMIT）断"存在且不低于本波真值"；
// 逐字相等判据留给 self/dept 两条（其范围过滤把可见集限定在私有归属人/私有部门内，
// 他人行结构上不可能进入，不受并行影响）。
#[tokio::test]
async fn sales_stats_scope_all_sees_every_qualifying_row() {
    let db = Arc::new(setup_test_db().await);
    seed(&db).await;
    let stats = fetch(
        &db,
        AppCache::arc(),
        Some(ctx(DataScope::All, USER_SELF, vec![])),
    )
    .await;
    let day_amount = |day: &str| -> Option<Decimal> {
        stats
            .daily_sales
            .iter()
            .find(|p| p.date == day)
            .map(|p| money(&p.amount))
    };
    assert!(
        day_amount("2096-04-01").is_some_and(|a| a >= Decimal::new(50000, 2)),
        "All 会话须看到同窗他人名下 2096-04-01 的 500（范围未被误收到本人），实得日销 {:?}",
        daily_dates(&stats)
    );
    assert!(
        day_amount("2096-05-01").is_some_and(|a| a >= Decimal::new(70000, 2)),
        "All 会话须看到他部门行所在日 2096-05-01 的 700，实得日销 {:?}",
        daily_dates(&stats)
    );
    assert!(
        day_amount("2096-06-01").is_some_and(|a| a >= Decimal::new(40000, 2)),
        "All 会话须含 NULL 归属行所在日 2096-06-01 的 400（管理员侧不排除无归属行），实得 {:?}",
        daily_dates(&stats)
    );
    assert!(
        day_amount("2097-06-01").is_none(),
        "越窗的 2097-06-01 必须被日期参数剔除，实得日销 {:?}",
        daily_dates(&stats)
    );
}

#[tokio::test]
async fn sales_stats_scope_self_sees_only_own_orders() {
    let db = Arc::new(setup_test_db().await);
    seed(&db).await;
    let stats = fetch(
        &db,
        AppCache::arc(),
        Some(ctx(DataScope::Self_, USER_SELF, vec![])),
    )
    .await;
    assert_eq!(
        stats.by_salesperson.len(),
        1,
        "Self 会话的分维结果只允许本人一个桶，实得 {:?}",
        stats.by_salesperson
    );
    assert_eq!(
        bucket(&stats, NAME_SELF),
        (Decimal::new(30000, 2), 1),
        "Self 桶须恰为 300/1，同部门他人 500、他部门 700、NULL 归属 400 都不得出现"
    );
}

#[tokio::test]
async fn sales_stats_scope_dept_sees_own_plus_visible_department() {
    let db = Arc::new(setup_test_db().await);
    seed(&db).await;
    let stats = fetch(
        &db,
        AppCache::arc(),
        Some(ctx(DataScope::Dept, USER_SELF, vec![DEPT_SELF])),
    )
    .await;
    assert_eq!(
        bucket(&stats, NAME_SELF),
        (Decimal::new(30000, 2), 1),
        "Dept 须含本人行"
    );
    assert_eq!(
        bucket(&stats, NAME_OWNER),
        (Decimal::new(50000, 2), 1),
        "Dept 须含可见部门的他人行"
    );
    assert!(
        !has_bucket(&stats, NAME_OTHER),
        "Dept 不得看到未授权部门行（他部门 700）"
    );
    assert_eq!(
        daily_total(&stats),
        Decimal::new(80000, 2),
        "Dept 的日销汇总须恰为 800（本人 ∪ 可见部门），NULL 归属行与草稿/越窗行不得进入"
    );
    assert_eq!(
        daily_dates(&stats),
        vec!["2096-03-01".to_string(), "2096-04-01".to_string()],
        "Dept 日销点位日期须恰为可见两行所在日"
    );
}

// —— 二、退化与零命中：这两形态此前最容易拼出非法语句（IN ()）或撞占位符 ——
#[tokio::test]
async fn sales_stats_scope_dept_without_visible_department_degrades_to_self() {
    let db = Arc::new(setup_test_db().await);
    seed(&db).await;
    let stats = fetch(
        &db,
        AppCache::arc(),
        Some(ctx(DataScope::Dept, USER_SELF, vec![])),
    )
    .await;
    assert_eq!(
        bucket(&stats, NAME_SELF),
        (Decimal::new(30000, 2), 1),
        "空可见部门集合须退化为仅本人（300/1）而不是空集或报错"
    );
    assert!(
        !has_bucket(&stats, NAME_OWNER),
        "退化后不得把同部门他人放行进来"
    );
}

#[tokio::test]
async fn sales_stats_scope_zero_visible_rows_returns_empty_not_error() {
    let db = Arc::new(setup_test_db().await);
    seed(&db).await;
    let stats = fetch(
        &db,
        AppCache::arc(),
        Some(ctx(DataScope::Dept, USER_NO_ROWS, vec![DEPT_OTHER])),
    )
    .await;
    assert!(
        stats.by_salesperson.is_empty()
            && stats.by_customer.is_empty()
            && stats.by_product.is_empty()
            && stats.daily_sales.is_empty(),
        "零命中归属人四个集合全须为空（fetch 内已 unwrap，报错即判红），实得 {:?}",
        stats
    );
}

// —— 三、范围绑定与日期绑定同语句共存：占位符撞号会把越窗行算进来或直接报错 ——
#[tokio::test]
async fn sales_stats_scope_with_date_window_does_not_collide_placeholder() {
    let db = Arc::new(setup_test_db().await);
    seed(&db).await;
    let stats = fetch(
        &db,
        AppCache::arc(),
        Some(ctx(DataScope::Self_, USER_SELF, vec![])),
    )
    .await;
    let (amount, count) = bucket(&stats, NAME_SELF);
    assert_eq!(
        amount,
        Decimal::new(30000, 2),
        "Self 范围绑定占 $3、日期须顺延到 $4/$5；撞号会让越窗的 999 并入本人桶"
    );
    assert_eq!(count, 1, "越窗与草稿行都不得计入本人桶");
    assert!(
        !daily_dates(&stats).contains(&"2097-06-01".to_string()),
        "日销点须被日期窗剔除越窗行，实得 {:?}",
        daily_dates(&stats)
    );
}

// —— 四、缓存键的范围维度：同库同日期窗下，后到的窄范围会话绝不可命中先到的宽范围结果 ——
#[tokio::test]
async fn sales_stats_cache_key_is_scoped_and_does_not_leak_across_roles() {
    let db = Arc::new(setup_test_db().await);
    seed(&db).await;
    let shared = AppCache::arc();
    let wide = fetch(
        &db,
        shared.clone(),
        Some(ctx(DataScope::All, USER_SELF, vec![])),
    )
    .await;
    assert!(
        has_bucket(&wide, NAME_OTHER),
        "前置：宽范围会话须先看到全部三个桶（否则本用例的隔离判据空转）"
    );
    let narrow = fetch(&db, shared, Some(ctx(DataScope::Self_, USER_SELF, vec![]))).await;
    assert_eq!(
        narrow.by_salesperson.len(),
        1,
        "窄范围会话命中了宽范围的缓存条目 ⇒ 缓存键未纳入数据范围，跨角色串数据"
    );
    assert_eq!(
        bucket(&narrow, NAME_SELF).0,
        Decimal::new(30000, 2),
        "串数据判据：窄范围金额须为本人 300 而非缓存里的全库值"
    );
}

// —— 五、源码扫描：两条取数通路都须接范围，且禁止把 DbErr 包装成外泄内部原文的裸 500 ——
#[test]
fn source_scan_sales_stats_applies_scope_and_no_internal_leak_wrapping() {
    let src = include_str!("../src/services/dashboard_service.rs").replace('\r', "");
    let code: String = src
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");

    let raw = fn_body(&code, "async fn query_sales_by_dimension");
    assert!(
        raw.contains("build_data_scope_sql"),
        "raw 分维必须经 build_data_scope_sql 下推角色数据范围，否则非管理员看到全库"
    );
    assert!(
        raw.contains("self.data_scope"),
        "raw 分维必须读取会话的数据范围上下文"
    );

    let daily = fn_body(&code, "async fn query_daily_sales_amounts");
    assert!(
        daily.contains("apply_department_scope"),
        "日销取数必须经 apply_department_scope 下推角色数据范围"
    );

    let key = fn_body(&code, "fn build_sales_cache_key");
    assert!(
        key.contains("data_scope"),
        "销售统计缓存键必须纳入数据范围，否则不同范围会话互相命中对方条目"
    );

    for body in [&raw, &daily] {
        assert!(
            !body.contains("map_err"),
            "取数函数不得用 map_err 包装 DbErr（须以 ? 传播），否则外泄内部原文并伪装成裸 500"
        );
    }
}

fn fn_body(code_src: &str, signature: &str) -> String {
    let anchor = code_src
        .find(signature)
        .unwrap_or_else(|| panic!("待锁符号不存在: {signature}"));
    let tail = &code_src[anchor..];
    let next = [
        "\n    async fn ",
        "\n    fn ",
        "\n    pub async fn ",
        "\n    pub fn ",
        "\nasync fn ",
        "\nfn ",
    ]
    .iter()
    .filter_map(|m| tail.find(m).filter(|pos| *pos > 0))
    .min()
    .unwrap_or(tail.len());
    tail[..next].to_string()
}
