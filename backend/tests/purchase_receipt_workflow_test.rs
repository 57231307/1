//! P0-T02 采购收货全流程集成测试（V15 Batch 487）
//!
//! 覆盖：状态常量值 + Service 实例化 + DB 异常路径（各条按被测函数体在**真库化夹具**
//! 下的真实契约逐条校准，见用例文档注释；#4671 判责 §⑤ W4 / 裁决 R-9）+ 完整流程（#[ignore]）
//! confirm_receipt 在同一事务内完成库存入库并把入库单推进到 COMPLETED 终态，
//! 集成测试覆盖 DRAFT → COMPLETED 全流程。

mod test_common;

use std::sync::Arc;

use bingxi_backend::models::status::purchase_receipt;
use bingxi_backend::services::purchase_receipt_dto::CreatePurchaseReceiptRequest;
use bingxi_backend::services::purchase_receipt_service::PurchaseReceiptService;
use sea_orm::Database;
use test_common::setup_test_db;

/// 构造最小 CreatePurchaseReceiptRequest（仅必填字段）
fn sample_request() -> CreatePurchaseReceiptRequest {
    use bingxi_backend::services::purchase_receipt_dto::CreateReceiptItemRequest;
    use chrono::NaiveDate;
    use rust_decimal::Decimal;

    CreatePurchaseReceiptRequest {
        order_id: Some(1),
        supplier_id: 1,
        warehouse_id: 1,
        receipt_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        department_id: None,
        inspector_id: None,
        notes: None,
        attachment_urls: None,
        items: vec![CreateReceiptItemRequest {
            order_item_id: Some(1),
            line_no: 1,
            material_id: 1,
            material_code: "MAT-001".to_string(),
            material_name: "测试物料".to_string(),
            quantity: Decimal::new(100, 0),
            quantity_alt: Decimal::new(50, 0),
            unit_master: "米".to_string(),
            unit_alt: Some("匹".to_string()),
            unit_price: Some(Decimal::new(10, 0)),
            batch_no: Some("B001".to_string()),
            color_code: None,
            lot_no: None,
            grade: None,
            gram_weight: None,
            width: None,
            location_code: None,
            package_no: None,
            production_date: None,
            shelf_life: None,
            notes: None,
            piece_no: None,
        }],
    }
}

// ===== 状态常量值正确性 =====

/// test_cgshztcl_zzqx
///
/// 验证采购收货状态常量值符合预期（大写风格）。
#[test]
fn test_cgshztcl_zzqx() {
    assert_eq!(purchase_receipt::DRAFT, "DRAFT");
    assert_eq!(purchase_receipt::CONFIRMED, "CONFIRMED");
    assert_eq!(purchase_receipt::COMPLETED, "COMPLETED");
}

/// test_cgshztcl_dxfgyzx
///
/// 验证采购收货状态常量均为大写 + 下划线风格。
#[test]
fn test_cgshztcl_dxfgyzx() {
    for s in [
        purchase_receipt::DRAFT,
        purchase_receipt::CONFIRMED,
        purchase_receipt::COMPLETED,
    ] {
        assert!(
            s.chars().all(|c| c.is_uppercase() || c == '_'),
            "状态 {} 应全大写",
            s
        );
    }
}

// ===== Service 实例化与 DB 异常路径 =====

/// test_purchasereceiptservice_slhbcfdb
#[tokio::test]
async fn test_purchasereceiptservice_slhbcfdb() {
    let db = setup_test_db().await;
    let svc = PurchaseReceiptService::new(Arc::new(db));
    let _ = svc;
}

/// test_purchasereceiptservice_create_receipt_kdbfherr
///
/// test_purchasereceiptservice_create_receipt_kdbfherr —— 真库化夹具前提校准
/// （#4671 判责 §⑤ W4；手法照抄 ap_payment_workflow_test 的 R-9 拆分范本）：
/// 钉"已建库空表上 create_receipt 引用不存在的前置单据 ⇒ 返回 Err 而非 panic"。
///
/// 真实契约依据（读函数体，非读注释）：`purchase_receipt_ops/crud.rs:31-71`
/// create_receipt 走事务插入 `purchase_receipt(order_id=Some(1))`，而
/// `purchase_orders` 不在 `test_common.rs` 种子白名单、已被 TRUNCATE ⇒ 23503 FK 拒绝
/// （判责原文 §2.3-C "purchase_receipt_workflow 建单失败"即此路径，Err 方向成立）。
/// 族别不钉机器码：失败可能先被维度校验（VALIDATION）或 FK（DATABASE）拦下，
/// 家族取决于门控顺序而非本条要锁的契约，本条锁"绝不 panic、绝不静默半落库"。
#[tokio::test]
async fn test_purchasereceiptservice_create_receipt_kdbfherr() {
    let db = setup_test_db().await;
    let svc = PurchaseReceiptService::new(Arc::new(db));
    let req = sample_request();
    let result = svc.create_receipt(req, 1).await;
    assert!(
        result.is_err(),
        "已建库空表上 create_receipt（order_id=1 不存在）必须返回 Err，实际：{:?}",
        result
    );
}

/// test_purchasereceiptservice_confirm_receipt_kdbfherr —— 同族逐条核：读函数体
/// `purchase_receipt_ops/state.rs:27-36,87-96`，confirm 先 begin 再 find_by_id +
/// lock_exclusive，空表 ⇒ None ⇒ `AppError::not_found`。断言由裸 `is_err()`
/// **收紧**为钉 NOT_FOUND 机器码（夹具退化成 DATABASE_ERROR 时必须显形）。
#[tokio::test]
async fn test_purchasereceiptservice_confirm_receipt_kdbfherr() {
    let db = setup_test_db().await;
    let svc = PurchaseReceiptService::new(Arc::new(db));
    let err = svc
        .confirm_receipt(1, 1)
        .await
        .expect_err("已建库空表上确认不存在的入库单必须返回 Err 而非 panic");
    assert_eq!(
        err.error_code(),
        "NOT_FOUND",
        "空表 confirm 必须命中 not_found（state.rs:96），实得 {}",
        err.error_code()
    );
}

/// test_purchasereceiptservice_get_receipt_kdbfherr —— 同族：读函数体
/// `purchase_receipt_ops/query.rs:141-166`，find_by_id LEFT JOIN，空表 ⇒ None ⇒
/// `AppError::not_found`（:163）。收紧为钉 NOT_FOUND 机器码。
#[tokio::test]
async fn test_purchasereceiptservice_get_receipt_kdbfherr() {
    let db = setup_test_db().await;
    let svc = PurchaseReceiptService::new(Arc::new(db));
    let err = svc
        .get_receipt(1)
        .await
        .expect_err("已建库空表上 get_receipt 不存在记录必须返回 Err 而非 panic");
    assert_eq!(
        err.error_code(),
        "NOT_FOUND",
        "空表 get_receipt 必须命中 not_found（query.rs:163），实得 {}",
        err.error_code()
    );
}

/// test_purchasereceiptservice_list_receipts_kdbfherr —— 依据裁决 R-9 拆前提后钉
/// 另一件事：已建库、业务表已清空 ⇒ `list_receipts` 不 panic 且返回**空集**（total=0）。
///
/// 真实契约依据（读函数体）：`purchase_receipt_ops/query.rs:64-138` 对空表 LEFT JOIN
/// 分页返回 `Ok(([], 0))`；原断 `is_err()` 属"把真库化夹具当空 SQLite"的过期前提
/// （判责原文 ci4671-triage.md :141 明确该条 p3 签名在过期族内）。改判据**不是**
/// 掩盖源码缺陷：空表返回空集正是列表端点的生产契约，schema 缺失报错形态不在此
/// 用例职责内（该负前提交集由本文件 get_receipt/confirm 族与 ap_payment 空 schema
/// 用例在 `bingxi_empty` 上统一钉死）。
#[tokio::test]
async fn test_purchasereceiptservice_list_receipts_kdbfherr() {
    let db = setup_test_db().await;
    let svc = PurchaseReceiptService::new(Arc::new(db));
    // 后 4 个 None = status/supplier_id/order_id 之外新增的 keyword/仓库/日期区间筛选全不传
    let (items, total) = svc
        .list_receipts(1, 20, None, None, None, None, None, None, None)
        .await
        .expect("已建库空表上 list_receipts 应返回 Ok 空集，而非 Err/panic");
    assert!(
        items.is_empty(),
        "夹具已 TRUNCATE 业务表，列表必须是空集，实得 {} 行",
        items.len()
    );
    assert_eq!(total, 0, "空表的 total 计数应为 0，实得 {total}");
}

// ===== 完整业务流程测试（需要真实 PostgreSQL，标记 ignore）=====

/// 集成测试：采购收货全流程 create(PENDING 质检) → 门控拒确认 → 质检合格回写 → confirm(COMPLETED)
///
/// 需要 PostgreSQL 测试数据库 + 前置采购订单/产品/仓库数据。
/// 门控口径（models/status/purchase_inventory.rs::purchase_receipt_inspection）：
/// 新建收货单检验状态恒为 PENDING，质检合格（PASSED）前确认入库必须被业务拒绝。
#[tokio::test]
#[ignore = "需要 PostgreSQL 测试数据库 + 前置采购订单/产品/仓库数据"]
async fn test_cgshqlc_cjdqr() {
    use bingxi_backend::models::purchase_receipt as receipt_entity;
    use bingxi_backend::models::status::purchase_inventory::purchase_receipt_inspection;
    use bingxi_backend::utils::error::AppError;
    use sea_orm::{ActiveModelTrait, Set};

    let db_url = std::env::var("TEST_DATABASE_URL").expect("需设置 TEST_DATABASE_URL 环境变量");
    let db = Database::connect(&db_url).await.expect("DB 连接失败");
    let svc = PurchaseReceiptService::new(Arc::new(db.clone()));

    // 1. 创建（DRAFT，检验状态 PENDING）
    let req = sample_request();
    let receipt = svc.create_receipt(req, 1).await.expect("创建失败");
    assert_eq!(receipt.receipt_status, purchase_receipt::DRAFT);
    assert_eq!(
        receipt.inspection_status,
        purchase_receipt_inspection::PENDING
    );

    // 2. 质检未合格前确认 → 门控拒绝（PENDING 不放行，无配置旁路）
    let err = svc
        .confirm_receipt(receipt.id, 1)
        .await
        .expect_err("质检未合格的收货单确认入库必须被拒绝");
    assert!(
        matches!(&err, AppError::BusinessErrorDisplayable(_)),
        "门控拒绝必须外显真实文案，实际: {err:?}"
    );

    // 3. 质检合格回写（模拟采购质检完成链路的最终状态；完整回写链由
    //    contract_wave5_inspection_result_authority_test 活库用例钉住）
    let mut active: receipt_entity::ActiveModel = receipt.into();
    active.inspection_status = Set(purchase_receipt_inspection::PASSED.to_string());
    let receipt = active.update(&db).await.expect("回写检验状态失败");

    // 4. 确认（DRAFT → COMPLETED，事务内完成库存入库、订单已收数量推进与应付账单生成）
    let receipt = svc.confirm_receipt(receipt.id, 1).await.expect("确认失败");
    assert_eq!(receipt.receipt_status, purchase_receipt::COMPLETED);
}
