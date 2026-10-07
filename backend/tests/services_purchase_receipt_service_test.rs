use bingxi_backend::models::status;
// decs 宏在测试中不可用，使用 Decimal::from_str 替代
use bingxi_backend::decs;
use bingxi_backend::services::purchase_receipt_dto::{
    CreatePurchaseReceiptRequest, CreateReceiptItemRequest, UpdatePurchaseReceiptRequest,
    UpdateReceiptItemRequest,
};
use bingxi_backend::services::test_common::{connect_empty_schema_db, setup_test_db};
use bingxi_backend::ymd;
// ymd 函数在测试中不可用，使用 NaiveDate::from_ymd_opt 替代
use bingxi_backend::services::purchase_receipt_service::PurchaseReceiptService;
use sea_orm::ConnectionTrait;
use std::collections::HashSet;
use std::sync::Arc;

/// 构造合法的 CreateReceiptItemRequest（单条明细）
fn sample_item() -> CreateReceiptItemRequest {
    CreateReceiptItemRequest {
        order_item_id: Some(1),
        line_no: 1,
        material_id: 1001,
        material_code: "M001".to_string(),
        material_name: "测试物料".to_string(),
        batch_no: Some("B20260719".to_string()),
        color_code: Some("RED".to_string()),
        lot_no: Some("L01".to_string()),
        grade: Some("A".to_string()),
        gram_weight: Some(decs!(200)),
        width: Some(decs!(150)),
        quantity: decs!(100),
        quantity_alt: decs!(50),
        unit_master: "M".to_string(),
        unit_alt: Some("KG".to_string()),
        unit_price: Some(decs!(10)),
        location_code: Some("A-01-01".to_string()),
        package_no: Some("P001".to_string()),
        production_date: Some(ymd!(2026, 7, 19)),
        shelf_life: Some(365),
        notes: Some("测试明细".to_string()),
        piece_no: None,
    }
}

/// 构造合法的 CreatePurchaseReceiptRequest（默认 1 条明细）
fn sample_request() -> CreatePurchaseReceiptRequest {
    CreatePurchaseReceiptRequest {
        order_id: Some(1),
        supplier_id: 100,
        receipt_date: ymd!(2026, 7, 19),
        warehouse_id: 1,
        department_id: Some(1),
        inspector_id: Some(10),
        notes: Some("测试入库单".to_string()),
        attachment_urls: Some(vec!["file://test.pdf".to_string()]),
        items: vec![sample_item()],
    }
}

// ============ 状态常量值正确性测试 ============

/// test_rkdztcl_zzqx
/// 验证 status::purchase_receipt 模块中 3 个状态常量值与状态机约定一致；（大写：DRAFT/CONFIRMED/COMPLETED，与 purchase_receipt_service.rs 中；字符串字面量 `"DRAFT"` / `status::purchase_receipt::DRAFT.to_string()` 一致）。
#[test]
fn test_rkdztcl_zzqx() {
    assert_eq!(status::purchase_receipt::DRAFT, "DRAFT");
    assert_eq!(status::purchase_receipt::CONFIRMED, "CONFIRMED");
    assert_eq!(status::purchase_receipt::COMPLETED, "COMPLETED");
}

/// test_rkdztcl_hbxt（业务规则：3 个状态必须互不相同，避免状态机歧义。）
#[test]
fn test_rkdztcl_hbxt() {
    let states = [
        status::purchase_receipt::DRAFT,
        status::purchase_receipt::CONFIRMED,
        status::purchase_receipt::COMPLETED,
    ];
    let unique: std::collections::HashSet<&str> = states.iter().copied().collect();
    assert_eq!(unique.len(), 3);
}

/// test_rkdztcl_dxfg
/// 业务规则：purchase_receipt 状态值采用大写风格（DRAFT/CONFIRMED/COMPLETED），；与 quotation 模块（小写 draft/approved/rejected/cancelled）不同。；验证所有状态均为大写字母（规则 20：注释与功能一致）。
#[test]
fn test_rkdztcl_dxfg() {
    // purchase_receipt 状态用大写（与 sales_order/quotation 小写不同）
    for s in [
        status::purchase_receipt::DRAFT,
        status::purchase_receipt::CONFIRMED,
        status::purchase_receipt::COMPLETED,
    ] {
        assert!(
            s.chars().all(|c| c.is_uppercase() || c == '_'),
            "状态 {} 应全大写",
            s
        );
    }
}

// ============ PurchaseReceiptService 构造与 DB 连接测试 ============

/// test_purchasereceiptservice_new_zqcysjklj（验证 new(Arc<DatabaseConnection>) 构造的 service 实例可以执行简单查询。）
#[tokio::test]
async fn test_purchasereceiptservice_new_zqcysjklj() {
    let db = Arc::new(setup_test_db().await);
    let svc = PurchaseReceiptService::new(db.clone());
    let _ = svc
        .db
        .execute_raw(sea_orm::Statement::from_sql_and_values(
            svc.db.get_database_backend(),
            "SELECT 1",
            Vec::new(),
        ))
        .await
        .expect("数据库连接应可用");
}

/// test_purchasereceiptservice_get_receipt_ksjkfherr —— CI §A.1 收紧族
/// "schema 缺失必须报 Err"的负前提交集改绑 `connect_empty_schema_db()`
/// （不跑迁移的 `bingxi_empty` 库）。原写法把 `setup_test_db()`（现语义 = 已迁移
/// PG + TRUNCATE）当"无表 SQLite"用，前提已过期：已建表空库上 get_receipt(9999)
/// 的真实契约是 Err(NOT_FOUND)（`purchase_receipt_ops/query.rs:163`），与本条要钉的
/// "缺表报错"是两件事（后者已由 purchase_receipt_workflow 的空表族钉 NOT_FOUND）。
/// 断言由裸 `is_err()` 收紧为钉 DATABASE_ERROR 机器码。
///
/// 真实契约依据（读函数体）：`purchase_receipt_ops/query.rs:141-166` find_by_id
/// LEFT JOIN `one(&*self.db).await?`；缺表 ⇒ DbErr::Query ⇒ `utils/error.rs:562-565`
/// AppError::database ⇒ error_code "DATABASE_ERROR"（error.rs:747）。
#[tokio::test]
async fn test_purchasereceiptservice_get_receipt_ksjkfherr() {
    let db = connect_empty_schema_db().await;
    let svc = PurchaseReceiptService::new(Arc::new(db));
    let err = svc
        .get_receipt(9999)
        .await
        .expect_err("schema 缺失（无 purchase_receipts 表）时必须返回 Err 而非 panic");
    assert_eq!(
        err.error_code(),
        "DATABASE_ERROR",
        "缺表必须命中数据库错误族（query.rs:161 → error.rs:562-565），实得 {}",
        err.error_code()
    );
}

/// test_purchasereceiptservice_list_receipts_ksjkfherr —— 同族收紧：改绑
/// `connect_empty_schema_db()`，裸 `is_err()` 收紧为钉 DATABASE_ERROR。
///
/// 真实契约依据（读函数体）：`purchase_receipt_ops/query.rs:64-138` list_receipts
/// 经 `paginate_with_total` 对 purchase_receipts LEFT JOIN 取页/计数，`?` 透传
/// DbErr；缺表 ⇒ DbErr::Query ⇒ "DATABASE_ERROR"（error.rs:562-565,747）。
/// 已建表空库上真实契约是 `Ok(([], 0))`——该正向形态由 workflow 文件
/// `test_purchasereceiptservice_list_receipts_kdbfherr` 在真库钉死，本条只锁缺表。
#[tokio::test]
async fn test_purchasereceiptservice_list_receipts_ksjkfherr() {
    let db = connect_empty_schema_db().await;
    let svc = PurchaseReceiptService::new(Arc::new(db));
    // 后 4 个 None = status/supplier_id/order_id 之外新增的 keyword/仓库/日期区间筛选全不传
    let err = svc
        .list_receipts(1, 20, None, None, None, None, None, None, None)
        .await
        .expect_err("schema 缺失（无 purchase_receipts 表）时必须返回 Err 而非 panic");
    assert_eq!(
        err.error_code(),
        "DATABASE_ERROR",
        "缺表必须命中数据库错误族（query.rs:135 → error.rs:562-565），实得 {}",
        err.error_code()
    );
}

/// test_purchasereceiptservice_list_receipt_items_ksjkfherr —— 同族收紧：改绑
/// `connect_empty_schema_db()`，裸 `is_err()` 收紧为钉 DATABASE_ERROR。
///
/// 真实契约依据（读函数体）：`purchase_receipt_ops/query.rs:169-180`
/// purchase_receipt_items find `.all(&*self.db).await?`；缺表 ⇒ DbErr::Query ⇒
/// "DATABASE_ERROR"（error.rs:562-565,747）。注意：已建表空库上本函数对不存在
/// 单据返回 `Ok(vec![])`（无 not_found 门）——"吞空集"是否算缺陷不在本条职责内，
/// 本条只锁"缺表绝不静默吞成 Ok"。
#[tokio::test]
async fn test_purchasereceiptservice_list_receipt_items_ksjkfherr() {
    let db = connect_empty_schema_db().await;
    let svc = PurchaseReceiptService::new(Arc::new(db));
    let err = svc
        .list_receipt_items(9999)
        .await
        .expect_err("schema 缺失（无 purchase_receipt_items 表）时必须返回 Err 而非 panic");
    assert_eq!(
        err.error_code(),
        "DATABASE_ERROR",
        "缺表必须命中数据库错误族（query.rs:176 → error.rs:562-565），实得 {}",
        err.error_code()
    );
}

// ============ create_receipt 业务校验测试 ============

/// test_purchasereceiptservice_create_receipt_kmxfherr —— 同族收紧：原注释自陈
/// 前提为"SQLite 内存数据库无表应返回 Err（非 panic）"，`setup_test_db()` 真库化后
/// 该前提过期，改绑 `connect_empty_schema_db()`，裸 `is_err()` 收紧为钉 DATABASE_ERROR。
///
/// 真实契约依据（读函数体）：`purchase_receipt_ops/crud.rs:31-51` create_receipt 对
/// items 只做 `validate_receipt_item_dimensions`（四维准入，空集合平凡通过；service
/// 层不调 DTO 的 `#[validate(length(min=1))]`，"空明细必填"由 handler 边界 validate
/// 负责，不属本条职责），首个 DB 步为事务 begin（:43，连库正常）后的黑名单查询
/// （:46-48，`supplier_blacklist_service.rs:238-242` `?` 透传 DbErr）；
/// 缺表 ⇒ "DATABASE_ERROR"（error.rs:562-565,747）。
#[tokio::test]
async fn test_purchasereceiptservice_create_receipt_kmxfherr() {
    let db = connect_empty_schema_db().await;
    let svc = PurchaseReceiptService::new(Arc::new(db));
    let mut req = sample_request();
    req.items.clear();
    let err = svc.create_receipt(req, 1).await.expect_err(
        "schema 缺失（无 supplier_blacklist/purchase_receipts 表）时必须返回 Err 而非 panic",
    );
    assert_eq!(
        err.error_code(),
        "DATABASE_ERROR",
        "缺表必须命中数据库错误族（crud.rs:46-48 → error.rs:562-565），实得 {}",
        err.error_code()
    );
}

/// test_purchasereceiptservice_create_receipt_bczbfherr —— 同族收紧：钉"缺表报错、
/// 绝不静默吞"（原注释即"无 schema 应返回 DbErr 非 panic"），改绑
/// `connect_empty_schema_db()`，裸 `is_err()` 收紧为钉 DATABASE_ERROR。
/// 已建库空表上"引用不存在前置单据 ⇒ Err"的形态由 workflow 文件
/// `test_purchasereceiptservice_create_receipt_kdbfherr` 负责，本条只锁缺表。
#[tokio::test]
async fn test_purchasereceiptservice_create_receipt_bczbfherr() {
    let db = connect_empty_schema_db().await;
    let svc = PurchaseReceiptService::new(Arc::new(db));
    let req = sample_request();
    let err = svc.create_receipt(req, 1).await.expect_err(
        "schema 缺失（无 supplier_blacklist/purchase_receipts 表）时必须返回 Err 而非 panic",
    );
    assert_eq!(
        err.error_code(),
        "DATABASE_ERROR",
        "缺表必须命中数据库错误族（crud.rs:46-48 → error.rs:562-565），实得 {}",
        err.error_code()
    );
}

// ============ update_receipt 状态机校验测试 ============

/// test_purchasereceiptservice_update_receipt_bczfhapperror（业务规则：update_receipt 不存在的入库单返回 AppError::not_found（机器码 NOT_FOUND，crud.rs:389）。）
#[tokio::test]
async fn test_purchasereceiptservice_update_receipt_bczfhapperror() {
    let db = setup_test_db().await;
    let svc = PurchaseReceiptService::new(Arc::new(db));
    let req = UpdatePurchaseReceiptRequest::default();
    let err = svc
        .update_receipt(9999, req, 1)
        .await
        .expect_err("不存在的入库单 id 必须返回 Err 而非 panic");
    assert_eq!(
        err.error_code(),
        "NOT_FOUND",
        "不存在 id 必须命中 not_found 族（crud.rs:389），实得 {}",
        err.error_code()
    );
}

/// test_purchasereceiptservice_delete_receipt_bczfhapperror（业务规则：delete_receipt 不存在的入库单返回 AppError::not_found（机器码 NOT_FOUND，crud.rs:460）。）
#[tokio::test]
async fn test_purchasereceiptservice_delete_receipt_bczfhapperror() {
    let db = setup_test_db().await;
    let svc = PurchaseReceiptService::new(Arc::new(db));
    let err = svc
        .delete_receipt(9999, 1)
        .await
        .expect_err("不存在的入库单 id 必须返回 Err 而非 panic");
    assert_eq!(
        err.error_code(),
        "NOT_FOUND",
        "不存在 id 必须命中 not_found 族（crud.rs:460），实得 {}",
        err.error_code()
    );
}

/// test_purchasereceiptservice_confirm_receipt_bczfhapperror（业务规则：confirm_receipt 不存在的入库单返回 AppError::not_found（机器码 NOT_FOUND，state.rs:96）。）
#[tokio::test]
async fn test_purchasereceiptservice_confirm_receipt_bczfhapperror() {
    let db = setup_test_db().await;
    let svc = PurchaseReceiptService::new(Arc::new(db));
    let err = svc
        .confirm_receipt(9999, 1)
        .await
        .expect_err("不存在的入库单 id 必须返回 Err 而非 panic");
    assert_eq!(
        err.error_code(),
        "NOT_FOUND",
        "不存在 id 必须命中 not_found 族（state.rs:96），实得 {}",
        err.error_code()
    );
}

// ============ 明细操作状态机校验测试 ============

/// test_purchasereceiptservice_add_receipt_item_bczrkdfhapperror
/// 业务规则：add_receipt_item 不存在的入库单返回 AppError::not_found（机器码 NOT_FOUND，items.rs:46）。
#[tokio::test]
async fn test_purchasereceiptservice_add_receipt_item_bczrkdfhapperror() {
    let db = setup_test_db().await;
    let svc = PurchaseReceiptService::new(Arc::new(db));
    let item_req = sample_item();
    let err = svc
        .add_receipt_item(9999, item_req, 1)
        .await
        .expect_err("不存在的入库单 id 必须返回 Err 而非 panic");
    assert_eq!(
        err.error_code(),
        "NOT_FOUND",
        "不存在 id 必须命中 not_found 族（items.rs:46），实得 {}",
        err.error_code()
    );
}

/// test_purchasereceiptservice_update_receipt_item_bczfhapperror
/// 业务规则：update_receipt_item 不存在的明细返回 AppError::not_found（机器码 NOT_FOUND，items.rs:138）。
#[tokio::test]
async fn test_purchasereceiptservice_update_receipt_item_bczfhapperror() {
    let db = setup_test_db().await;
    let svc = PurchaseReceiptService::new(Arc::new(db));
    let req = UpdateReceiptItemRequest::default();
    let err = svc
        .update_receipt_item(9999, req, 1)
        .await
        .expect_err("不存在的明细 id 必须返回 Err 而非 panic");
    assert_eq!(
        err.error_code(),
        "NOT_FOUND",
        "不存在 id 必须命中 not_found 族（items.rs:138），实得 {}",
        err.error_code()
    );
}

/// test_purchasereceiptservice_delete_receipt_item_bczfhapperror
/// 业务规则：delete_receipt_item 不存在的明细返回 AppError::not_found（机器码 NOT_FOUND，items.rs:272）。
#[tokio::test]
async fn test_purchasereceiptservice_delete_receipt_item_bczfhapperror() {
    let db = setup_test_db().await;
    let svc = PurchaseReceiptService::new(Arc::new(db));
    let err = svc
        .delete_receipt_item(9999, 1)
        .await
        .expect_err("不存在的明细 id 必须返回 Err 而非 panic");
    assert_eq!(
        err.error_code(),
        "NOT_FOUND",
        "不存在 id 必须命中 not_found 族（items.rs:272），实得 {}",
        err.error_code()
    );
}

// ============ calculate_receipt_total 测试 ============

/// test_purchasereceiptservice_calculate_receipt_total_bczfhapperror
/// 业务规则：calculate_receipt_total 不存在的入库单返回 AppError::not_found（机器码 NOT_FOUND，items.rs:340）。
#[tokio::test]
async fn test_purchasereceiptservice_calculate_receipt_total_bczfhapperror() {
    let db = setup_test_db().await;
    let svc = PurchaseReceiptService::new(Arc::new(db));
    let err = svc
        .calculate_receipt_total(9999, 1)
        .await
        .expect_err("不存在的入库单 id 必须返回 Err 而非 panic");
    assert_eq!(
        err.error_code(),
        "NOT_FOUND",
        "不存在 id 必须命中 not_found 族（items.rs:340），实得 {}",
        err.error_code()
    );
}

// ============ DTO 字段完整性测试 ============

/// test_createreceiptitemrequest_zdwzgz（验证 CreateReceiptItemRequest 所有字段可以正确构造，；确保后续业务方法接收到完整 DTO 时不会因字段缺失 panic。）
#[test]
fn test_createreceiptitemrequest_zdwzgz() {
    let item = sample_item();
    assert_eq!(item.material_id, 1001);
    assert_eq!(item.material_code, "M001");
    assert_eq!(item.quantity, decs!(100));
    assert_eq!(item.unit_price, Some(decs!(10)));
    assert!(item.batch_no.is_some());
    assert!(item.color_code.is_some());
    assert!(item.lot_no.is_some());
    assert!(item.grade.is_some());
}

/// test_updatepurchasereceiptrequest_mrzqwnone
/// 业务规则：UpdatePurchaseReceiptRequest 使用 #[derive(Default)]，；所有字段默认为 None，表示不更新该字段。
#[test]
fn test_updatepurchasereceiptrequest_mrzqwnone() {
    let req = UpdatePurchaseReceiptRequest::default();
    assert!(req.supplier_id.is_none());
    assert!(req.receipt_date.is_none());
    assert!(req.department_id.is_none());
    assert!(req.inspector_id.is_none());
    assert!(req.notes.is_none());
    assert!(req.attachment_urls.is_none());
}

/// test_updatereceiptitemrequest_mrzqwnone
/// 业务规则：UpdateReceiptItemRequest 使用 #[derive(Default)]，；所有字段默认为 None，表示不更新该字段。
#[test]
fn test_updatereceiptitemrequest_mrzqwnone() {
    let req = UpdateReceiptItemRequest::default();
    assert!(req.line_no.is_none());
    assert!(req.material_id.is_none());
    assert!(req.material_code.is_none());
    assert!(req.quantity.is_none());
    assert!(req.unit_price.is_none());
    assert!(req.notes.is_none());
}
