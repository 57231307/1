//! 通道（路线一，#4669 判责）：夹具经 `test_common::setup_test_db()` 连已迁移
//! PostgreSQL，表结构唯一来源 = `backend/migration`；缺 `TEST_DATABASE_URL` 或指向
//! sqlite 时夹具直接 panic。原写法在变量缺失时**静默回退 sqlite::memory:**，
//! 那正是本批假绿的根（"没连上真库也算通过"），回退分支已彻底删除。
//! 本文件用例锁的是"非法参数在触库前必须被拒"，因此断言与后端方言无关，
//! 但在真库通道上跑才有意义：若未来有人把校验挪到 SQL 之后，用例会显式变红。
mod test_common;

use bingxi_backend::database::*;
use bingxi_backend::handlers::bi_handler::*;
use bingxi_backend::services::bi_analysis_service::*;
use chrono::NaiveDate;
use std::sync::Arc;

/// 测试辅助：构造连到测试库的 service 实例（参数校验测试仅调用 DB 查询前的校验路径）。
///
/// 原实现返回 `Option` 并在 `DATABASE_URL` 缺失/连接失败时让调用方 `if let Some`
/// 整体跳过断言 = "没跑也算 PASS" 的假绿守卫；随后一版改为读 `TEST_DATABASE_URL`
/// 并回退 `sqlite::memory:` —— 回退同样是假绿（从未验证生产方言却显示通过）。
/// 现统一走路线一夹具：连接失败或环境缺失即 panic，断言无条件执行。
async fn make_service() -> BiAnalysisService {
    let db = test_common::setup_test_db().await;
    BiAnalysisService::new(Arc::new(db))
}

#[tokio::test]
async fn test_drilldown_invalid_year() {
    let service = make_service().await;
    let result = service.drilldown_year_to_month(1800).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_slice_invalid_dimension() {
    let service = make_service().await;
    let result = service.slice("invalid_dim", &serde_json::json!({})).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_sales_by_time_invalid_dates() {
    let service = make_service().await;
    let result = service
        .sales_by_time(
            chrono::NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
            chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            "month",
        )
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_drilldown_invalid_month() {
    let service = make_service().await;
    let result = service.drilldown_month_to_day(2026, 13).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_drilldown_customer_invalid_id() {
    let service = make_service().await;
    let result = service.drilldown_customer_to_order(0).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_drilldown_product_invalid_id() {
    let service = make_service().await;
    let result = service.drilldown_product_to_order(-1).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_rollup_invalid_level() {
    let service = make_service().await;
    let result = service.rollup("invalid", "month").await;
    assert!(result.is_err());
}

/// v11 批次 144 P1-3：透视矩阵参数校验测试
#[tokio::test]
async fn test_pivot_invalid_row_dim() {
    let service = make_service().await;
    let result = service.pivot("invalid", "customer", "total_amount").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_pivot_invalid_col_dim() {
    let service = make_service().await;
    let result = service.pivot("customer", "invalid", "total_amount").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_pivot_same_dim() {
    let service = make_service().await;
    let result = service.pivot("customer", "customer", "total_amount").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_pivot_invalid_measure() {
    let service = make_service().await;
    let result = service
        .pivot("customer", "product", "invalid_measure")
        .await;
    assert!(result.is_err());
}

// ==================== 批次 252：dim_to_expr / measure_to_expr 单元测试 ====================
// 验证原 unreachable!() 分支现在返回错误而非 panic 崩溃

/// 测试 dim_to_expr 对所有合法维度返回 Ok
#[test]
fn test_dim_to_expr_valid_dims() {
    let valid_dims = ["customer", "product", "region", "category", "time"];
    for dim in valid_dims {
        assert!(dim_to_expr(dim).is_ok(), "维度 {} 应返回 Ok", dim);
    }
}

/// 测试 dim_to_expr 对非法维度返回 Err（原 unreachable!() 会 panic）
#[test]
fn test_dim_to_expr_invalid_dim_returns_error() {
    let result = dim_to_expr("invalid_dim");
    assert!(result.is_err(), "非法维度应返回错误而非 panic");
}

/// 测试 dim_to_expr 对空字符串返回 Err
#[test]
fn test_dim_to_expr_empty_string_returns_error() {
    let result = dim_to_expr("");
    assert!(result.is_err(), "空字符串维度应返回错误而非 panic");
}

/// 测试 measure_to_expr 对所有合法度量在项级和订单级均返回 Ok
#[test]
fn test_measure_to_expr_valid_measures() {
    let valid_measures = ["total_amount", "order_count", "quantity", "profit_amount"];
    for measure in valid_measures {
        assert!(
            measure_to_expr(measure, true).is_ok(),
            "度量 {} 项级聚合应返回 Ok",
            measure
        );
        assert!(
            measure_to_expr(measure, false).is_ok(),
            "度量 {} 订单级聚合应返回 Ok",
            measure
        );
    }
}

/// 测试 measure_to_expr 对非法度量返回 Err（原 unreachable!() 会 panic）
#[test]
fn test_measure_to_expr_invalid_measure_returns_error() {
    assert!(
        measure_to_expr("invalid_measure", true).is_err(),
        "非法度量项级聚合应返回错误而非 panic"
    );
    assert!(
        measure_to_expr("invalid_measure", false).is_err(),
        "非法度量订单级聚合应返回错误而非 panic"
    );
}
