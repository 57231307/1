//! P0-T02 采购收货全流程集成测试（V15 Batch 487）
//!
//! 覆盖：状态常量值 + Service 实例化 + DB 异常路径（各条按被测函数体在**真库化夹具**
//! 下的真实契约逐条校准，见用例文档注释； 判责 §⑤ W4 / 裁决 R-9）+ 完整流程（#[ignore]）
//! confirm_receipt 在同一事务内完成库存入库并把入库单推进到 COMPLETED 终态，
//! 集成测试覆盖 DRAFT → COMPLETED 全流程。

mod test_common;

use std::sync::Arc;

use bingxi_backend::models::status::purchase_receipt;
use bingxi_backend::services::purchase_receipt_dto::CreatePurchaseReceiptRequest;
use bingxi_backend::services::purchase_receipt_service::PurchaseReceiptService;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use test_common::setup_test_db;

/// 构造最小 CreatePurchaseReceiptRequest（仅必填字段）
fn sample_request() -> CreatePurchaseReceiptRequest {
    use bingxi_backend::services::purchase_receipt_dto::CreateReceiptItemRequest;
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
/// （判责 §⑤ W4；手法照抄 ap_payment_workflow_test 的 R-9 拆分范本）
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

/// test_purchasereceiptservice_list_receipts_kdbfherr —— 拆分夹具前提后钉
/// 另一件事：已建库、业务表已清空 ⇒ `list_receipts` 不 panic 且返回**空集**（total=0）。
///
/// 真实契约依据（读函数体）：`purchase_receipt_ops/query.rs:64-138` 对空表 LEFT JOIN
/// 分页返回 `Ok(([], 0))`；原断 `is_err()` 属"把真库化夹具当空 SQLite"的过期前提。
/// 改判据**不是**
/// 掩盖源码缺陷：空表返回空集正是列表端点的生产契约，schema 缺失报错形态不在此
/// 用例职责内（该负前提交集由 services_purchase_receipt_service_test.rs 的
/// ksjkfherr/bczbfherr 族五条用例在 `bingxi_empty` 上钉 DATABASE_ERROR；本文件
/// get_receipt/confirm 族在真库钉的是空表
/// NOT_FOUND，与缺表是两件事）。
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
/// 真库化前提补齐（CI §A.2 种子族，只补剩下这条前提，600e5640 已提交的
/// 判据用例不动）：`sample_request()` 引用 order_id=1 / warehouse_id=1 /
/// material_id=1 / order_item_id=1，读函数体
/// `purchase_receipt_ops/crud.rs::create_receipt` 与 DDL 逐一对应——
/// - purchase_receipt.warehouse_id FK NOT NULL（m0009_add_purchase_extensions.rs:72/:90
///   `fk_purchase_receipt_warehouse`），warehouses 空表 ⇒ 23503 ⇒ 本轮 CI 红原文
///   `DatabaseError("数据库查询错误")`；
/// - purchase_receipt_item.product_id FK（m0009:135），products 须有父行；
/// - 明细 order_item_id=Some(1) 走 `link_receipt_items_to_order_items` fail-closed 守卫
///   （purchase_receipt_ops/crud.rs）：该 id 必须是 order 1 的 **`purchase_order_item`
///   （单数表）** 行、且 product_id 与入库明细一致、入库量 ≤ 订单量×(1+容差)。
///   ⚠ 表名唯一事实来源是实体 models/purchase_order_item.rs 上的
///   `#[sea_orm(table_name = "purchase_order_item")]`（DDL 见该表同名迁移建表项）；
///   m0001_initial_schema.rs 里的复数 `purchase_order_items` 是 SeaORM 实体永不读取的
///   遗留表——种子写进复数表时守卫在单数表查无明细并报「采购订单 1 没有明细行」，
///   该拒绝属**正确触发**，处置只改种子侧、不放松守卫。
///   订单量 200 ≥ 入库 100；supplier_id=1 由迁移种子参照表 m0015 恒在（不清空）。
/// 列形态按实体整 Model 解码需求：
/// - `purchase_order_item`（该表迁移建表项）NOT NULL 无默认的金额/数量列全部补写
///   （line_no/quantity_alt/unit_price_foreign/discount_percent/tax_percent/
///   tax_amount/discount_amount/total_amount/received_quantity_alt/created_at/
///   updated_at，models/purchase_order_item.rs:12-106）；quantity_tolerance_pct 留
///   NULL=未行级指定，按品类默认解析（米→5%，crud.rs:287-300）。
/// - purchase_orders 头行：confirm 链的 write_back_actual_delivery_date /
///   save_order_status_update 全行解码 purchase_order::Model
///   （purchase_receipt_private.rs:180/146），其 warehouse_id/department_id/
///   purchaser_id/currency/exchange_rate/total_*/order_status/created_by 为非
///   Option（models/purchase_order.rs:41-99），生效 DDL 却是可空后补列
///   （system/mod.rs:337-350），种子必须按服务层写入形态补全；order_status 用
///   状态词表 token（APPROVED，收货对已审批单进行）。
/// - products 行：link 守卫按 product_id 全行解码 product::Model 取 unit 做品类
///   容差判定（crud.rs:290-300），unit/status/product_type 非 Option
///   （models/product.rs:26/35/45），写入侧恒非空（handlers/product_handler.rs:383-390）。
/// 走 `setup_test_db()`（TRUNCATE + RESTART IDENTITY）使显式 id=1 对齐引用；
/// ignored lane `--test-threads=1` 串行无竞态。
///
/// 门控口径（models/status/purchase_inventory.rs::purchase_receipt_inspection，不变）：
/// 新建收货单检验状态恒为 PENDING，质检合格（PASSED）前确认入库必须被业务拒绝。
#[tokio::test]
#[ignore = "需要 PostgreSQL 测试数据库 + 前置采购订单/产品/仓库数据"]
async fn test_cgshqlc_cjdqr() {
    use bingxi_backend::models::purchase_receipt as receipt_entity;
    use bingxi_backend::models::status::purchase_inventory::purchase_receipt_inspection;
    use bingxi_backend::utils::error::AppError;
    use sea_orm::{ActiveModelTrait, Set};

    let db = setup_test_db().await;
    for (sql, what) in [
        (
            "INSERT INTO warehouses (id, name, warehouse_code, is_active) VALUES \
             (1, '收货全流程测试仓', 'WF-W1', true)",
            "warehouses 父行",
        ),
        (
            "INSERT INTO products (id, code, name, unit, status, product_type) VALUES \
             (1, 'PT-WF-R1', '收货全流程测试面料', '米', 'active', '成品布')",
            "products 父行",
        ),
        (
            "INSERT INTO purchase_orders (id, order_no, supplier_id, order_date, \
             warehouse_id, department_id, purchaser_id, currency, exchange_rate, \
             total_amount, total_amount_foreign, total_quantity, total_quantity_alt, \
             order_status, created_by) \
             VALUES (1, 'WF-PO-0001', 1, '2026-01-01', 1, 1, 1, 'CNY', 1.000000, \
             2000.00, 2000.00, 200.0000, 0.0000, 'APPROVED', 1)",
            "purchase_orders 父行",
        ),
        (
            "INSERT INTO purchase_order_item (id, order_id, line_no, product_id, \
             quantity, quantity_alt, unit_price, unit_price_foreign, discount_percent, \
             tax_percent, subtotal, tax_amount, discount_amount, total_amount, \
             received_quantity, received_quantity_alt, created_at, updated_at) \
             VALUES (1, 1, 1, 1, 200.0000, 0.0000, 10.000000, 0.000000, 0.0000, \
             0.0000, 2000.00, 0.00, 0.00, 2000.00, 0.0000, 0.0000, \
             CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            "purchase_order_item 父行（单数表=实体真表）",
        ),
    ] {
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            Vec::<sea_orm::Value>::new(),
        ))
        .await
        .unwrap_or_else(|e| panic!("种子 {what} 写入失败: {e}\nSQL: {sql}"));
    }
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
