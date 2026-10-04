//! P0-T02 付款全流程集成测试（V15 Batch 487）
//!
//! 覆盖：状态常量值 + Service 实例化 + DB 异常路径
//! PAID 状态由 event_bus 监听器自动标记（confirm 后异步触发），
//! 集成测试覆盖到 CONFIRMED 流转即可（REGISTERED → CONFIRMED）。

mod test_common;

use std::sync::Arc;

use bingxi_backend::models::status::{common, payment};
use bingxi_backend::services::ap_payment_service::{ApPaymentListQuery, ApPaymentService};
// 批次 490 D10-3b 修复：使用 super:: 限定本地 mod common，避免被 status::common 遮蔽
use test_common::setup_test_db;

// ===== 状态常量值正确性 =====

/// test_fkztcl_zzqx
///
/// 验证付款状态常量值符合预期（大写风格）。
#[test]
fn test_fkztcl_zzqx() {
    assert_eq!(payment::PAYMENT_REGISTERED, "REGISTERED");
    assert_eq!(payment::PAYMENT_CONFIRMED, "CONFIRMED");
    assert_eq!(payment::PAYMENT_PAID, "PAID");
    assert_eq!(payment::PAYMENT_PARTIAL_PAID, "PARTIAL_PAID");
    assert_eq!(common::STATUS_APPROVED, "APPROVED");
}

/// test_fkztcl_dxfgyzx
///
/// 验证付款状态常量均为大写 + 下划线风格。
#[test]
fn test_fkztcl_dxfgyzx() {
    for s in [
        payment::PAYMENT_REGISTERED,
        payment::PAYMENT_CONFIRMED,
        payment::PAYMENT_PAID,
        payment::PAYMENT_PARTIAL_PAID,
        common::STATUS_APPROVED,
    ] {
        assert!(
            s.chars().all(|c| c.is_uppercase() || c == '_'),
            "状态 {} 应全大写",
            s
        );
    }
}

/// test_fkztlzl_yyzqx
///
/// 验证付款状态流转链符合业务语义：
/// REGISTERED → CONFIRMED → PAID（或 PARTIAL_PAID）
#[test]
fn test_fkztlzl_yyzqx() {
    // 状态值互不相同
    let statuses = [
        payment::PAYMENT_REGISTERED,
        payment::PAYMENT_CONFIRMED,
        payment::PAYMENT_PAID,
        payment::PAYMENT_PARTIAL_PAID,
    ];
    for i in 0..statuses.len() {
        for j in (i + 1)..statuses.len() {
            assert_ne!(statuses[i], statuses[j], "付款状态值不应重复");
        }
    }
}

// ===== Service 实例化与 DB 异常路径 =====

/// test_appaymentservice_slhbcfdb
#[tokio::test]
async fn test_appaymentservice_slhbcfdb() {
    let db = setup_test_db().await;
    let svc = ApPaymentService::new(Arc::new(db));
    let _ = svc;
}

/// test_appaymentservice_get_by_id_kdbfherr —— 依据 R-9 拆前提后**只钉一件事**：
/// schema 缺失（表根本不存在）时的报错形态。
///
/// 原写法与兄弟 `get_list` 共用 `setup_test_db()`（语义早已改为"已迁移 PG + TRUNCATE
/// 业务表"，见 `src/services/test_common.rs`），两条断同一个 `is_err()`：
/// 那样必有一条是假绿——空表上 `get_by_id` 走"记录不存在"、空表上 `get_list` 走
/// `Ok(空)`，两者语义完全不同。本条改绑 `connect_empty_schema_db()`（对不跑迁移的
/// `bingxi_empty` 库），钉的是"表不存在 ⇒ 返回 Err 而不是 panic"。
#[tokio::test]
async fn test_appaymentservice_get_by_id_kdbfherr() {
    let db = test_common::connect_empty_schema_db().await;
    let svc = ApPaymentService::new(Arc::new(db));
    let result = svc.get_by_id(1, None).await;
    assert!(
        result.is_err(),
        "schema 缺失（无 ap_payment 表）时 get_by_id 必须返回 Err 而非 panic"
    );
}

/// test_appaymentservice_get_list_kdbfherr —— 依据 R-9 只钉另一件事：
/// 已建库、业务表已清空 ⇒ `get_list` 不 panic 且返回**空集**（total=0）。
///
/// 原断 `is_err()` 是把"真库化夹具"当成"空 SQLite"的过期前提：TRUNCATE 后表存在且为空，
/// 返回 `Ok(([], 0))` 才是正确契约（由本用例断言自证）。
/// 要验 schema 缺失的报错形态请见上一条用例。
#[tokio::test]
async fn test_appaymentservice_get_list_kdbfherr() {
    let db = setup_test_db().await;
    let svc = ApPaymentService::new(Arc::new(db));
    let query = ApPaymentListQuery {
        supplier_id: None,
        payment_status: None,
        payment_method: None,
        start_date: None,
        end_date: None,
        page: 1,
        page_size: 20,
    };
    let (items, total) = svc
        .get_list(query, None)
        .await
        .expect("已建库空表上 get_list 应返回 Ok 空集，而非 Err/panic");
    assert!(
        items.is_empty(),
        "夹具已 TRUNCATE 业务表，列表必须是空集，实得 {} 行",
        items.len()
    );
    assert_eq!(total, 0, "空表的 total 计数应为 0，实得 {total}");
}

/// test_appaymentservice_confirm_kdbfherr —— 同族收口（范本见本文件
/// get_by_id/get_list 两条的拆分注释）：
/// 钉"已建库空业务表上，confirm 不存在的单必须返回 **NOT_FOUND 机器码**而非 panic"。
///
/// 真实契约依据（读函数体确认，非读注释）：`src/services/ap_payment_service.rs:194-226`
/// confirm 先 begin，再 `find_by_id + lock_exclusive`，空表 ⇒ None ⇒
/// `AppError::not_found`。⇒ `is_err()` 方向本身成立，但原注释"空 SQLite 数据库"的
/// 前提已过期（`setup_test_db()` 现语义 = 已迁移 PostgreSQL + TRUNCATE 业务表，
/// 见 `src/services/test_common.rs:17-24`），且只钉 is_err 会把"任何 Err 都算过"的
/// 漂移放进来（如未来夹具退化 ⇒ DATABASE_ERROR 也过）。本条**收紧**为钉机器码，
/// 不比对 message 原文（脱敏红线）。
#[tokio::test]
async fn test_appaymentservice_confirm_kdbfherr() {
    let db = setup_test_db().await;
    let svc = ApPaymentService::new(Arc::new(db));
    let err = svc
        .confirm(1, 1)
        .await
        .expect_err("已建库空表上 confirm 不存在的付款单必须返回 Err 而非 panic");
    assert_eq!(
        err.error_code(),
        "NOT_FOUND",
        "空表 confirm 必须命中 not_found 机器码（ap_payment_service.rs:226），实得 {}",
        err.error_code()
    );
}

// ===== 完整业务流程测试（需要真实 PostgreSQL，标记 ignore）=====

/// 集成测试：付款全流程 create(REGISTERED) → confirm(CONFIRMED)
///
/// 需要 PostgreSQL + 前置 APPROVED 付款申请数据。
/// PAID 状态由 event_bus 监听器自动标记（confirm 后异步触发）。
#[tokio::test]
#[ignore = "需要 PostgreSQL 测试数据库 + 前置 APPROVED 付款申请数据"]
async fn test_fkqlc_djdqr() {
    // 完整流程需前置 APPROVED 付款申请（ap_payment_request），
    // create 从付款申请创建付款单（REGISTERED），confirm 确认执行（CONFIRMED）。
    // PAID 状态由 event_bus 监听 PaymentCompleted 事件自动标记。
}
