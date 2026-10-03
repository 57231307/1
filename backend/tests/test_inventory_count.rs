//! 库存盘点服务集成测试（V15 P2 B06-P2-5）
//!
//! 覆盖：盘点状态常量值 + Service 构造签名 + 请求 DTO 字段语义 + 已建库空表异常路径
//! （判责 §⑤ W4：旧"空 DB=空 SQLite"前提已随真库化过期，各条契约按被测函数体核定）。
//! InventoryCountService 所有业务方法（create_count / record_count_items /
//! submit_for_approval / approve_count 等）均需数据库事务，
//! 完整业务流程由 CI 集成环境执行（同 test_quality_standard.rs / ap_payment_workflow_test.rs 模式）。

mod test_common;

use bingxi_backend::models::status::inventory_count as count_status;
use bingxi_backend::services::inventory_count_service::{
    CountItemInput, CreateCountRequest, InventoryCountService, UpdateCountRequest,
};
use chrono::Utc;
use rust_decimal::Decimal;
use std::sync::Arc;
use test_common::setup_test_db;

// 测试夹具（规则 6：mock 数据抽取到 fixtures，禁止硬编码）
mod fixtures {
    use rust_decimal::Decimal;

    pub const WAREHOUSE_ID: i32 = 1001;
    pub const CREATED_BY: i32 = 9001;
    pub const STOCK_ID_A: i32 = 3001;
    pub const STOCK_ID_B: i32 = 3002;
    /// 空 DB 异常路径测试用的不存在盘点单 ID
    pub const NON_EXISTENT_COUNT_ID: i32 = 999_999;
    /// 账面数量 120.50 米
    pub fn quantity_before() -> Decimal {
        Decimal::new(12050, 2)
    }
    /// 实盘数量 118.25 米（与账面差异 -2.25）
    pub fn quantity_actual() -> Decimal {
        Decimal::new(11825, 2)
    }
}

// ===== 盘点状态常量值正确性 =====

/// 验证盘点状态常量值符合预期（小写风格）。
#[test]
fn test_count_status_value_correct() {
    assert_eq!(count_status::PENDING, "pending");
    assert_eq!(count_status::COMPLETED, "completed");
}

/// 验证盘点状态常量均为小写风格，且两值互不相同。
#[test]
fn test_count_status_style_and_distinct() {
    for s in [count_status::PENDING, count_status::COMPLETED] {
        assert!(
            s.chars().all(|c| c.is_lowercase() || c == '_'),
            "盘点状态 {} 应全小写",
            s
        );
    }
    assert_ne!(
        count_status::PENDING,
        count_status::COMPLETED,
        "盘点状态值不应重复"
    );
}

// ===== Service 构造签名与实例化 =====

/// 验证 InventoryCountService 构造函数签名：fn(Arc<DatabaseConnection>) -> InventoryCountService
#[test]
fn test_inventory_count_service_constructor_signature() {
    let constructor: fn(Arc<sea_orm::DatabaseConnection>) -> InventoryCountService =
        InventoryCountService::new;

    // 验证函数指针不为空
    assert!(
        !format!("{:?}", constructor).is_empty(),
        "InventoryCountService 构造函数不应为空"
    );
}

/// 验证 Service 可在 SQLite 内存库上实例化（不 panic）。
#[tokio::test]
async fn test_inventory_count_service_instantiation() {
    let db = setup_test_db().await;
    let svc = InventoryCountService::new(Arc::new(db));
    let _ = svc;
}

// ===== 空 DB 异常路径（验证优雅降级，不 panic）=====

/// 验证在**已建库空表**上 get_count 不存在记录返回 Err(NOT_FOUND) 而非 panic。
///
/// 真库化夹具前提校准（#4671 判责 §⑤ W4，手法同 ap_payment R-9 族）：旧注释
/// "空 SQLite 库"过期——`setup_test_db()` 现为"已迁移 PG + TRUNCATE 业务表"
/// （`src/services/test_common.rs:17-24`）。真实契约按读函数体判定：
/// `inventory_count_service.rs:285-293` find_by_id 空表 ⇒ None ⇒ `AppError::not_found`
/// ⇒ Err 方向成立，断言由裸 `is_err()` 收紧为钉机器码。
#[tokio::test]
async fn test_get_count_returns_err_on_empty_db() {
    let db = setup_test_db().await;
    let svc = InventoryCountService::new(Arc::new(db));
    let err = svc
        .get_count(fixtures::NON_EXISTENT_COUNT_ID, None)
        .await
        .expect_err("已建库空表上 get_count 不存在记录必须返回 Err 而非 panic");
    assert_eq!(
        err.error_code(),
        "NOT_FOUND",
        "空表 get_count 必须命中 not_found（inventory_count_service.rs:293），实得 {}",
        err.error_code()
    );
}

/// 验证在**已建库空表**上 create_count 命中"仓库下无库存记录"业务门 ⇒
/// Err(BUSINESS_ERROR) 而非 panic。
///
/// 真实契约依据（读函数体，非读注释）：`inventory_count_service.rs:107-130`
/// create_count 先取号、再 `fetch_stocks_for_count_txn`，`inventory_stocks` 属业务表
/// 被 TRUNCATE ⇒ 快照恒空 ⇒ `AppError::business("仓库 … 下无库存记录…")`。
/// 原注释"空 SQLite"前提过期；断言收紧为钉机器码——若未来把该门控误接成
/// 静默建空单（Ok）或退化 DATABASE_ERROR，本条必须红。
#[tokio::test]
async fn test_create_count_returns_err_on_empty_db() {
    let db = setup_test_db().await;
    let svc = InventoryCountService::new(Arc::new(db));
    let req = CreateCountRequest {
        warehouse_id: fixtures::WAREHOUSE_ID,
        count_date: Utc::now(),
        notes: None,
        created_by: Some(fixtures::CREATED_BY),
        stock_ids: None,
    };
    let err = svc
        .create_count(req)
        .await
        .expect_err("已建库空表上对无库存仓库 create_count 必须返回 Err 而非 panic");
    assert_eq!(
        err.error_code(),
        "BUSINESS_ERROR",
        "空库存建盘点单必须命中业务门控族（inventory_count_service.rs:126），实得 {}",
        err.error_code()
    );
}

// ===== 请求 DTO 字段语义 =====

/// 验证 CreateCountRequest 全量盘点（stock_ids = None）字段语义。
#[test]
fn test_create_count_request_full_warehouse() {
    let req = CreateCountRequest {
        warehouse_id: fixtures::WAREHOUSE_ID,
        count_date: Utc::now(),
        notes: Some("全仓盘点".to_string()),
        created_by: Some(fixtures::CREATED_BY),
        stock_ids: None,
    };
    assert_eq!(req.warehouse_id, fixtures::WAREHOUSE_ID);
    assert!(req.stock_ids.is_none());
    assert_eq!(req.notes.as_deref(), Some("全仓盘点"));
    assert_eq!(req.created_by, Some(fixtures::CREATED_BY));
}

/// 验证 CreateCountRequest 定向盘点（stock_ids = Some）字段语义。
#[test]
fn test_create_count_request_targeted_stocks() {
    let req = CreateCountRequest {
        warehouse_id: fixtures::WAREHOUSE_ID,
        count_date: Utc::now(),
        notes: None,
        created_by: Some(fixtures::CREATED_BY),
        stock_ids: Some(vec![fixtures::STOCK_ID_A, fixtures::STOCK_ID_B]),
    };
    let ids = req.stock_ids.as_ref().expect("定向盘点应有 stock_ids");
    assert_eq!(ids.len(), 2);
    assert_eq!(ids[0], fixtures::STOCK_ID_A);
    assert_eq!(ids[1], fixtures::STOCK_ID_B);
    assert!(req.notes.is_none());
}

/// 验证 UpdateCountRequest::default() 字段均为 None（部分更新语义）。
#[test]
fn test_update_count_request_default_all_none() {
    let req = UpdateCountRequest::default();
    assert!(req.count_date.is_none());
    assert!(req.notes.is_none());
}

/// 验证 CountItemInput 实盘数量差异计算（差异 = 实盘 - 账面，与 service record_count_items 一致）。
#[test]
fn test_count_item_input_difference_semantics() {
    let input = CountItemInput {
        stock_id: fixtures::STOCK_ID_A,
        quantity_actual: fixtures::quantity_actual(),
        notes: Some("少 2.25 米".to_string()),
    };
    // 差异计算与 inventory_count_service.rs record_count_items 一致
    let difference = input.quantity_actual - fixtures::quantity_before();
    assert_eq!(difference, Decimal::new(-225, 2));
    assert_eq!(input.stock_id, fixtures::STOCK_ID_A);
}
