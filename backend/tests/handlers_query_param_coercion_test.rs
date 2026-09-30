//! 查询参数「真正生效」回归测试（任务 #160 同族修复）
//!
//! 根因锁定：多处 handler 曾以 `Query<serde_json::Value>` 接收查询串。Axum 的 `Query` 抽取器
//! 走 urlencoded 反序列化，`serde_json::Value` 是无类型的 ⇒ 每个值都被反序列化成
//! `Value::String`，随后 `.as_i64()` 对任何输入恒返回 `None`，代码永远走 `unwrap_or` 默认值，
//! 于是 `?page=3` / `?page_size=100` / `?customer_id=42` / `?period=...` 全部静默失效且不报错。
//!
//! 本测试不连 DB，直接对「生产 typed DTO + Axum Query 抽取器」这条真实解析路径 oneshot 断言：
//! 1. 数字字符串被正确转换为整数（证明 typed DTO 让参数真正生效）；
//! 2. 非法数值（`page=abc`）被 Axum 抽取器拒绝为 400（证明不再静默回落）；
//! 3. 必填参数缺失（`check-balance` 的 `period`）被拒绝为 400（证明不再以空串静默透传）。
//!
//! 若把任一 DTO 改回 `Query<serde_json::Value>`，第 1 组断言将拿到默认值而失败。

use axum::{
    Router,
    body::Body,
    extract::Query,
    http::{Request, StatusCode},
    routing::get,
};
use tower::ServiceExt;

use bingxi_backend::handlers::{
    assist_accounting_handler::CheckBalanceQuery,
    budget_management_handler::{BudgetListQuery, ExecutionWarningsQuery},
    financial_analysis_handler::ListReportsQuery,
    fixed_asset_handler::ListCountPlansQuery,
    sales_order_handler::OrderStatisticsQuery,
};

/// 用生产 DTO 形态命中一次 GET，返回 (状态码, 文本响应体)。
async fn get_text(uri: &str, app: Router) -> (StatusCode, String) {
    let resp = app
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (status, String::from_utf8_lossy(&bytes).to_string())
}

// ==================== budget: list_budgets ====================

async fn probe_budget_list(Query(q): Query<BudgetListQuery>) -> String {
    format!(
        "page={}|size={}|plan={:?}|itype={:?}|status={:?}",
        q.page.unwrap_or(-1),
        q.page_size.unwrap_or(-1),
        q.plan_id,
        q.item_type,
        q.status,
    )
}

#[tokio::test]
async fn budget_list_query_numeric_strings_take_effect() {
    let app = Router::new().route("/p", get(probe_budget_list));
    let (status, body) = get_text(
        "/p?page=3&page_size=50&plan_id=7&item_type=income&status=active",
        app,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "合法数字字符串应被接受: {body}");
    // 关键：旧实现下 page 恒为默认值，这里必须是提交的 3 / 50 / 7 而非默认 1 / 20 / None
    assert_eq!(
        body,
        "page=3|size=50|plan=Some(7)|itype=Some(\"income\")|status=Some(\"active\")"
    );
}

#[tokio::test]
async fn budget_list_query_illegal_number_is_400() {
    let app = Router::new().route("/p", get(probe_budget_list));
    let (status, _) = get_text("/p?page=abc", app.clone()).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "非法 page 应 400 而非静默回落"
    );
    let (status, _) = get_text("/p?plan_id=xyz", app).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "非法 plan_id 应 400");
}

// ==================== budget: budget_execution_warnings ====================

async fn probe_execution_warnings(Query(q): Query<ExecutionWarningsQuery>) -> String {
    format!("year={:?}", q.budget_year)
}

#[tokio::test]
async fn execution_warnings_year_query_takes_effect() {
    let app = Router::new().route("/p", get(probe_execution_warnings));
    let (status, body) = get_text("/p?budget_year=2025", app.clone()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body, "year=Some(2025)",
        "budget_year 必须真正生效而非恒当前年"
    );
    let (status, _) = get_text("/p?budget_year=notyear", app).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// ==================== financial_analysis: list_reports ====================

async fn probe_list_reports(Query(q): Query<ListReportsQuery>) -> String {
    format!(
        "page={}|size={}",
        q.page.unwrap_or(-1),
        q.page_size.unwrap_or(-1)
    )
}

#[tokio::test]
async fn financial_list_reports_pagination_takes_effect() {
    let app = Router::new().route("/p", get(probe_list_reports));
    let (status, body) = get_text("/p?page=2&page_size=100", app.clone()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body, "page=2|size=100",
        "?page=2 应取第二页、page_size 应生效"
    );
    let (status, _) = get_text("/p?page=1&x=&page_size=1.5", app).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "非整数 page_size 应 400");
}

// ==================== fixed_asset: list_count_plans ====================

async fn probe_count_plans(Query(q): Query<ListCountPlansQuery>) -> String {
    format!(
        "page={}|size={}",
        q.page.unwrap_or(-1),
        q.page_size.unwrap_or(-1)
    )
}

#[tokio::test]
async fn fixed_asset_count_plans_pagination_takes_effect() {
    let app = Router::new().route("/p", get(probe_count_plans));
    let (status, body) = get_text("/p?page=4&page_size=30", app.clone()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "page=4|size=30");
    let (status, _) = get_text("/p?page=abc", app).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// ==================== assist_accounting: check_assist_vs_general_balance ====================

async fn probe_check_balance(Query(q): Query<CheckBalanceQuery>) -> String {
    format!("period={}", q.period)
}

#[tokio::test]
async fn check_balance_period_required_and_takes_effect() {
    let app = Router::new().route("/p", get(probe_check_balance));
    let (status, body) = get_text("/p?period=2025-01", app.clone()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "period=2025-01");
    // 旧实现 period 缺失时以空串静默透传给 service；现必须 400
    let (status, _) = get_text("/p", app).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "period 缺失应 400 而非静默传空串"
    );
}

// ==================== sales_order: get_order_statistics ====================

async fn probe_order_stats(Query(q): Query<OrderStatisticsQuery>) -> String {
    format!(
        "cid={:?}|s={:?}|e={:?}",
        q.customer_id, q.start_date, q.end_date
    )
}

#[tokio::test]
async fn order_statistics_customer_id_filter_takes_effect() {
    let app = Router::new().route("/p", get(probe_order_stats));
    let (status, body) = get_text("/p?customer_id=42&start_date=2024-01-01", app.clone()).await;
    assert_eq!(status, StatusCode::OK);
    // 旧实现下 customer_id 经 as_i64() 恒 None ⇒ 统计恒全量；现须真正解析为 42
    assert_eq!(
        body, "cid=Some(42)|s=Some(\"2024-01-01\")|e=None",
        "customer_id 筛选必须真正生效"
    );
    let (status, _) = get_text("/p?customer_id=abc", app).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
