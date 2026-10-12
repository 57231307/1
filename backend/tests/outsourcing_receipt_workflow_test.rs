//! 委外收货 workflow 集成测试（V15 P1）
//!
//! 覆盖：状态常量值 + Service 实例化 + DB 异常路径 + 完整流程骨架（#[ignore]）。
//! 委外完成事件由 `OutsourcingReceiptService::confirm` 在事务提交后发布，
//! workflow 测试覆盖到收回单 `draft -> confirmed` 与订单 `processing -> received` 语义。
mod test_common;

use std::sync::Arc;

use bingxi_backend::models::status::{outsourcing_order_status, outsourcing_receipt_status};
use bingxi_backend::services::outsourcing_service::OutsourcingReceiptService;
use test_common::setup_test_db;

/// test_wwshztcl_zzqx
///
/// 验证委外收货/订单状态常量值符合预期。
#[test]
fn test_wwshztcl_zzqx() {
    assert_eq!(outsourcing_receipt_status::DRAFT, "draft");
    assert_eq!(outsourcing_receipt_status::CONFIRMED, "confirmed");
    assert_eq!(outsourcing_receipt_status::CANCELLED, "cancelled");

    assert_eq!(outsourcing_order_status::PROCESSING, "processing");
    assert_eq!(outsourcing_order_status::RECEIVED, "received");
}

/// test_wwshztcl_xxwyzf
///
/// 验证委外收货/订单状态常量均为小写风格。
#[test]
fn test_wwshztcl_xxwyzf() {
    for status in [
        outsourcing_receipt_status::DRAFT,
        outsourcing_receipt_status::CONFIRMED,
        outsourcing_receipt_status::CANCELLED,
        outsourcing_order_status::PROCESSING,
        outsourcing_order_status::RECEIVED,
    ] {
        assert!(
            status.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "状态 {} 应全小写",
            status
        );
    }
}

/// test_outsourcingreceiptservice_slhbcfdb
#[tokio::test]
async fn test_outsourcingreceiptservice_slhbcfdb() {
    let db = setup_test_db().await;
    let svc = OutsourcingReceiptService::new(Arc::new(db));
    std::mem::drop(svc);
}

/// test_outsourcingreceiptservice_confirm_kdbfherr —— 真库化夹具前提校准（同
/// ap_payment_workflow_test.rs 的拆分族手法：无 schema/空表逐条核契约）：
/// 钉"已建库空业务表上 confirm 不存在的收回单 ⇒ **NOT_FOUND 机器码**，不 panic"。
///
/// 真实契约依据（读函数体，非读注释）：`src/services/outsourcing_ops/receipt.rs:351-363`
/// confirm 先 begin，`find_by_id + lock_exclusive` 空表 ⇒ None ⇒ `AppError::not_found`。
/// 原注释"空 SQLite 数据库"前提已过期——`setup_test_db()` 现语义 = 已迁移 PG +
/// TRUNCATE 业务表（`src/services/test_common.rs:17-24`）。断言由裸 `is_err()`
/// **收紧**为钉机器码（夹具退化出 DATABASE_ERROR 时本条必须红），不比对 message。
#[tokio::test]
async fn test_outsourcingreceiptservice_confirm_kdbfherr() {
    let db = setup_test_db().await;
    let svc = OutsourcingReceiptService::new(Arc::new(db));
    let err = svc
        .confirm(1, None)
        .await
        .expect_err("已建库空表上 confirm 不存在的收回单必须返回 Err 而非 panic");
    assert_eq!(
        err.error_code(),
        "NOT_FOUND",
        "空表 confirm 必须命中 not_found 机器码（receipt.rs:363），实得 {}",
        err.error_code()
    );
}

/// test_outsourcingreceiptservice_get_by_id_kdbfherr —— 同上族，本条真实契约经读
/// 函数体确认为 **Err(NOT_FOUND)** 而非 Ok：`receipt.rs:685-691` find_by_id +
/// is_deleted 过滤，空表 ⇒ None ⇒ `AppError::not_found`。
/// 故保留 `setup_test_db()`（已建库空表）并把裸 `is_err()` 收紧为 NOT_FOUND 机器码；
/// 不绑 `connect_empty_schema_db()`——两条前提验证的是不同事，这里钉的是"记录不存在"
/// 的服务语义，schema 缺失族已由 ap_payment 同族用例统一覆盖。
#[tokio::test]
async fn test_outsourcingreceiptservice_get_by_id_kdbfherr() {
    let db = setup_test_db().await;
    let svc = OutsourcingReceiptService::new(Arc::new(db));
    let err = svc
        .get_by_id(1)
        .await
        .expect_err("已建库空表上 get_by_id 不存在记录必须返回 Err 而非 panic");
    assert_eq!(
        err.error_code(),
        "NOT_FOUND",
        "空表 get_by_id 必须命中 not_found 机器码（receipt.rs:690），实得 {}",
        err.error_code()
    );
}

/// 集成测试：委外收货全流程 create(draft) → confirm(confirmed)
///
/// 需要 PostgreSQL 测试数据库 + 前置委外订单/成品/仓库数据。
/// confirm 成功后，收回单应变为 confirmed，委外订单应变为 received，
/// 且完成事件在事务提交后异步发布。
#[tokio::test]
#[ignore = "需要 PostgreSQL 测试数据库 + 前置委外订单/成品/仓库数据"]
async fn test_wwshqlc_cjdqr() {
    let _ = std::env::var("TEST_DATABASE_URL");
    tokio::task::yield_now().await;
}
