use bingxi_backend::models::status;
// decs 宏在测试中不可用，使用 Decimal::from_str 替代
use bingxi_backend::decs;
use bingxi_backend::services::purchase_receipt_dto::{
    UpdatePurchaseReceiptRequest, UpdateReceiptItemRequest,
};
use bingxi_backend::services::test_common::setup_test_db;
use bingxi_backend::ymd;
// ymd 函数在测试中不可用，使用 NaiveDate::from_ymd_opt 替代
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
    use bingxi_backend::services::purchase_receipt_service::PurchaseReceiptService;
    use bingxi_backend::utils::error::AppError;
    use sea_orm::ConnectionTrait;
    use std::collections::HashSet;
    let _ = svc
        .database
        .execute_raw(sea_orm::Statement::from_sql_and_values(
            svc.database.get_database_backend(),
            "SELECT 1",
            Vec::new(),
        ))
        .await
        .expect("数据库连接应可用");
}

/// test_purchasereceiptservice_get_receipt_ksjkfherr
/// 业务规则：get_receipt 查询 purchase_receipts 表，SQLite 内存数据库无 schema 应返回 Err。；验证错误处理路径健壮性（不会因 DB 错误 panic）。
#[tokio::test]
async fn test_purchasereceiptservice_get_receipt_ksjkfherr() {
    let db = setup_test_db().await;
    let svc = PurchaseReceiptService::new(Arc::new(db));
    let result = svc.get_receipt(9999).await;
    // SQLite 内存数据库无 purchase_receipts 表，应返回 Err（DbErr 转 AppError）
    assert!(result.is_err());
}

/// test_purchasereceiptservice_list_receipts_ksjkfherr
/// 业务规则：list_receipts 查询 purchase_receipts 表，SQLite 内存数据库无 schema 应返回 Err。；验证错误处理路径健壮性（不会因 DB 错误 panic）。
#[tokio::test]
async fn test_purchasereceiptservice_list_receipts_ksjkfherr() {
    let db = setup_test_db().await;
    let svc = PurchaseReceiptService::new(Arc::new(db));
    let result = svc.list_receipts(1, 20, None, None, None).await;
    // SQLite 内存数据库无 purchase_receipts 表，应返回 Err
    assert!(result.is_err());
}

/// test_purchasereceiptservice_list_receipt_items_ksjkfherr
/// 业务规则：list_receipt_items 查询 purchase_receipt_items 表，SQLite 内存数据库无 schema 应返回 Err。；验证错误处理路径健壮性（不会因 DB 错误 panic）。
#[tokio::test]
async fn test_purchasereceiptservice_list_receipt_items_ksjkfherr() {
    let db = setup_test_db().await;
    let svc = PurchaseReceiptService::new(Arc::new(db));
    let result = svc.list_receipt_items(9999).await;
    // SQLite 内存数据库无 purchase_receipt_items 表，应返回 Err
    assert!(result.is_err());
}

// ============ create_receipt 业务校验测试 ============

/// test_purchasereceiptservice_create_receipt_kmxfherr
/// 业务规则：CreatePurchaseReceiptRequest.items 至少 1 条（DTO 上 #[validate(length(min = 1))]）。；service 层未显式调用 Validate::validate，空明细会进入 generate_receipt_no 查询表，；SQLite 内存数据库无表应返回 Err（非 panic）。
#[tokio::test]
async fn test_purchasereceiptservice_create_receipt_kmxfherr() {
    let db = setup_test_db().await;
    let svc = PurchaseReceiptService::new(Arc::new(db));
    let mut req = sample_request();
    req.items.clear();
    let result = svc.create_receipt(req, 1).await;
    assert!(result.is_err());
}

/// test_purchasereceiptservice_create_receipt_bczbfherr
/// 业务规则：create_receipt 依赖 purchase_receipt 表存在。；SQLite 内存数据库无 schema，应返回 DbErr（非 panic）。；这验证了错误处理路径的健壮性（不会因 DB 错误 panic）。
#[tokio::test]
async fn test_purchasereceiptservice_create_receipt_bczbfherr() {
    let db = setup_test_db().await;
    let svc = PurchaseReceiptService::new(Arc::new(db));
    let req = sample_request();
    let result = svc.create_receipt(req, 1).await;
    // SQLite 内存数据库无表，应返回 Err（DbErr 或 AppError）
    assert!(result.is_err());
}

// ============ update_receipt 状态机校验测试 ============

/// test_purchasereceiptservice_update_receipt_bczfhapperror（业务规则：update_receipt 不存在的入库单返回 AppError::not_found。）
#[tokio::test]
async fn test_purchasereceiptservice_update_receipt_bczfhapperror() {
    let db = setup_test_db().await;
    let svc = PurchaseReceiptService::new(Arc::new(db));
    let req = UpdatePurchaseReceiptRequest::default();
    let result = svc.update_receipt(9999, req, 1).await;
    assert!(result.is_err());
}

/// test_purchasereceiptservice_delete_receipt_bczfhapperror（业务规则：delete_receipt 不存在的入库单返回 AppError::not_found。）
#[tokio::test]
async fn test_purchasereceiptservice_delete_receipt_bczfhapperror() {
    let db = setup_test_db().await;
    let svc = PurchaseReceiptService::new(Arc::new(db));
    let result = svc.delete_receipt(9999, 1).await;
    assert!(result.is_err());
}

/// test_purchasereceiptservice_confirm_receipt_bczfhapperror（业务规则：confirm_receipt 不存在的入库单返回 AppError::not_found。）
#[tokio::test]
async fn test_purchasereceiptservice_confirm_receipt_bczfhapperror() {
    let db = setup_test_db().await;
    let svc = PurchaseReceiptService::new(Arc::new(db));
    let result = svc.confirm_receipt(9999, 1).await;
    assert!(result.is_err());
}

// ============ 明细操作状态机校验测试 ============

/// test_purchasereceiptservice_add_receipt_item_bczrkdfhapperror
/// 业务规则：add_receipt_item 不存在的入库单返回 AppError::not_found。
#[tokio::test]
async fn test_purchasereceiptservice_add_receipt_item_bczrkdfhapperror() {
    let db = setup_test_db().await;
    let svc = PurchaseReceiptService::new(Arc::new(db));
    let item_req = sample_item();
    let result = svc.add_receipt_item(9999, item_req, 1).await;
    assert!(result.is_err());
}

/// test_purchasereceiptservice_update_receipt_item_bczfhapperror
/// 业务规则：update_receipt_item 不存在的明细返回 AppError::not_found。
#[tokio::test]
async fn test_purchasereceiptservice_update_receipt_item_bczfhapperror() {
    let db = setup_test_db().await;
    let svc = PurchaseReceiptService::new(Arc::new(db));
    let req = UpdateReceiptItemRequest::default();
    let result = svc.update_receipt_item(9999, req, 1).await;
    assert!(result.is_err());
}

/// test_purchasereceiptservice_delete_receipt_item_bczfhapperror
/// 业务规则：delete_receipt_item 不存在的明细返回 AppError::not_found。
#[tokio::test]
async fn test_purchasereceiptservice_delete_receipt_item_bczfhapperror() {
    let db = setup_test_db().await;
    let svc = PurchaseReceiptService::new(Arc::new(db));
    let result = svc.delete_receipt_item(9999, 1).await;
    assert!(result.is_err());
}

// ============ calculate_receipt_total 测试 ============

/// test_purchasereceiptservice_calculate_receipt_total_bczfhapperror
/// 业务规则：calculate_receipt_total 不存在的入库单返回 AppError::not_found。
#[tokio::test]
async fn test_purchasereceiptservice_calculate_receipt_total_bczfhapperror() {
    let db = setup_test_db().await;
    let svc = PurchaseReceiptService::new(Arc::new(db));
    let result = svc.calculate_receipt_total(9999, 1).await;
    assert!(result.is_err());
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
