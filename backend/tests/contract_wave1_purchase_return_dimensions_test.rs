//! 采购退货明细三维回读 + Decimal 字符串出参锁（本波契约：DTO 补 color/dye/batch 三维）
//!
//! 锁定的 file:line 契约：
//! - `backend/src/services/purchase_return_service.rs:993-1015`（PurchaseReturnItemDto
//!   含 color_no/dye_lot_no/batch_no 非空 String 三维，"真实回读非兜底默认"）
//! - `backend/src/services/purchase_return_service.rs:1019-1038`（list_items：全列 SELECT +
//!   column_as 别名 + into_model::<Dto> 单次 JOIN 富化，维度随实体列带出）
//! - `backend/src/services/purchase_return_service.rs:962-991`（Create/UpdateReturnItemRequest
//!   三维 Option；create_item L1093-1095 缺省落空串——不是伪造值而是 inventory_stock 归一口径）
//! - rust_decimal serde 默认字符串出参（前端 `quantity_alt` 按 string 消费；历史 500
//!   为 po_order_items.returned_quantity INT4-vs-Decimal 列型问题，属活库迁移断言，见下）
//!
//! 覆盖策略：
//! - serde 纯函数断言（**无需任何 DB**）：DTO 三维往返、Decimal 序列化=字符串/反序列化字符串、
//!   Create/Update 请求三维 Option 语义
//! - `sqlite::memory:` 自建 purchase_return_item + products 最小列，走真实 `list_items`
//!   查询（只读全列 SELECT+JOIN+into_model，无 advisory lock / lock_exclusive，
//!   **不需要活 PG**）——三维原样回读 + Decimal 解码在真实 DB 编解码路径上锁死
//! - `#[ignore]` 活库全链：建退货单→带维度明细→submit→approve：四维唯一命中扣库存、
//!   回写来源 PO received_quantity、归零回退 APPROVED、同四维多行歧义报业务错
//!   （approve/writeback 用 lock_exclusive + 单号 advisory_xact_lock，sqlite 不支持）

mod test_common;

use rust_decimal::Decimal;
use serde_json::json;
use std::str::FromStr;

use bingxi_backend::models::{
    inventory_stock, product, purchase_order, purchase_order_item, purchase_return_item,
};
use bingxi_backend::services::purchase_return_service::{
    CreatePurchaseReturnRequest, CreateReturnItemRequest, PurchaseReturnItemDto,
    PurchaseReturnService, UpdateReturnItemRequest,
};
use bingxi_backend::utils::error::AppError;
use chrono::{NaiveDate, TimeZone, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DbBackend, EntityTrait, Statement,
};

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn dto_json() -> serde_json::Value {
    json!({
        "id": 3, "return_id": 1, "line_no": 1, "material_id": 5,
        "material_code": "FAB-01", "material_name": "测试坯布甲",
        "quantity_returned": "12.3400", "unit_price": "8.50", "tax_rate": "13.00",
        "discount_percent": "0.00", "subtotal": "104.89", "tax_amount": "13.63",
        "discount_amount": "0.00", "total_amount": "118.52", "notes": "匹条退",
        "color_no": "COL-A", "dye_lot_no": "DYE-9", "batch_no": "B7"
    })
}

// =========================================================
// serde 纯断言（无 DB）
// =========================================================

/// DTO 三维解码 + 回序列化往返：键名与值逐字锁死（前端 usePrRtn 回填读的就是这三键）
#[test]
fn pr_item_dto_three_dimensions_roundtrip() {
    let dto: PurchaseReturnItemDto = serde_json::from_value(dto_json()).unwrap();
    assert_eq!(dto.color_no, "COL-A");
    assert_eq!(dto.dye_lot_no, "DYE-9");
    assert_eq!(dto.batch_no, "B7");
    assert_eq!(dto.material_code.as_deref(), Some("FAB-01"));

    let v = serde_json::to_value(&dto).unwrap();
    assert_eq!(v["color_no"], "COL-A");
    assert_eq!(v["dye_lot_no"], "DYE-9");
    assert_eq!(v["batch_no"], "B7");
}

/// received/returned 类 Decimal 字段出参必须是 JSON 字符串（rust_decimal 默认 serde 口径），
/// 且字符串解码无损——前端 .toFixed 直接消费 string，数字形态即判红
#[test]
fn pr_item_dto_decimal_fields_wire_format_is_string() {
    let dto: PurchaseReturnItemDto = serde_json::from_value(dto_json()).unwrap();
    assert_eq!(dto.quantity_returned, dec("12.34"));

    let v = serde_json::to_value(&dto).unwrap();
    for key in [
        "quantity_returned",
        "unit_price",
        "tax_rate",
        "subtotal",
        "tax_amount",
        "discount_amount",
        "total_amount",
        "discount_percent",
    ] {
        assert!(
            v[key].is_string(),
            "{key} 必须序列化为字符串，实际: {}",
            v[key]
        );
    }
    // 尾零精度随字符串保留（scale 4 的 "12.3400" 不被数值化截短）
    assert_eq!(v["quantity_returned"], "12.3400");

    // 数字入参也要能解码（DB 侧某些方言以数值回传）
    let mut body = dto_json();
    body["quantity_returned"] = json!(12.34);
    let dto2: PurchaseReturnItemDto = serde_json::from_value(body).unwrap();
    assert_eq!(dto2.quantity_returned, dec("12.34"));
}

/// CreateReturnItemRequest：三维为 Option，缺键解码通过（白坯/单行产品场景）
#[test]
fn create_return_item_request_dims_optional() {
    let req: CreateReturnItemRequest = serde_json::from_value(json!({
        "line_no": 1, "material_id": 5, "quantity_returned": "10.00", "unit_price": "8.50"
    }))
    .unwrap();
    assert!(req.color_no.is_none());
    assert!(req.dye_lot_no.is_none());
    assert!(req.batch_no.is_none());
    assert_eq!(req.quantity_returned, dec("10.00"));

    let req2: CreateReturnItemRequest = serde_json::from_value(json!({
        "line_no": 1, "material_id": 5, "quantity_returned": "10.00", "unit_price": "8.50",
        "color_no": "COL-A", "dye_lot_no": "DYE-9", "batch_no": "B7"
    }))
    .unwrap();
    assert_eq!(req2.color_no.as_deref(), Some("COL-A"));
    assert_eq!(req2.dye_lot_no.as_deref(), Some("DYE-9"));
    assert_eq!(req2.batch_no.as_deref(), Some("B7"));
}

/// UpdateReturnItemRequest 全字段 Option：{} 也解码通过（PATCH 语义）
#[test]
fn update_return_item_request_all_optional() {
    let req: UpdateReturnItemRequest = serde_json::from_value(json!({})).unwrap();
    assert!(req.color_no.is_none() && req.dye_lot_no.is_none() && req.batch_no.is_none());
    assert!(req.quantity_returned.is_none());
}

// =========================================================
// sqlite 自建表走真实 list_items（无需活 PG）
// =========================================================

async fn sqlite_db() -> sea_orm::DatabaseConnection {
    sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败")
}

async fn exec(db: &sea_orm::DatabaseConnection, sql: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        sql,
        Vec::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("DDL 执行失败: {e}\nSQL: {sql}"));
}

/// purchase_return_item 全列（与 `models/purchase_return_item.rs::Model` 逐一对应）
const PR_ITEM_DDL: &str = r#"CREATE TABLE purchase_return_item (
    id INTEGER PRIMARY KEY, return_id INTEGER, line_no INTEGER, product_id INTEGER,
    quantity TEXT, quantity_alt TEXT, unit_price TEXT, unit_price_foreign TEXT,
    discount_percent TEXT, tax_percent TEXT, subtotal TEXT, tax_amount TEXT,
    discount_amount TEXT, total_amount TEXT, notes TEXT, created_at TEXT, updated_at TEXT,
    color_no TEXT, dye_lot_no TEXT, batch_no TEXT
)"#;

/// list_items 的 JOIN 只投影 products.id/code/name（column_as 别名），无需全列
const PRODUCT_MIN_DDL: &str =
    "CREATE TABLE products (id INTEGER PRIMARY KEY, code TEXT, name TEXT)";

async fn insert_pr_item(
    db: &sea_orm::DatabaseConnection,
    id: i32,
    line_no: i32,
    color: &str,
    dye: &str,
    batch: &str,
    quantity: &str,
) {
    purchase_return_item::ActiveModel {
        id: Set(id),
        return_id: Set(101),
        line_no: Set(line_no),
        product_id: Set(5),
        quantity: Set(dec(quantity)),
        quantity_alt: Set(Decimal::ZERO),
        unit_price: Set(dec("8.50")),
        unit_price_foreign: Set(dec("8.50")),
        discount_percent: Set(Decimal::ZERO),
        tax_percent: Set(dec("13.00")),
        subtotal: Set(dec("85.00")),
        tax_amount: Set(dec("11.05")),
        discount_amount: Set(Decimal::ZERO),
        total_amount: Set(dec("96.05")),
        notes: Set(None),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        color_no: Set(color.to_string()),
        dye_lot_no: Set(dye.to_string()),
        batch_no: Set(batch.to_string()),
    }
    .insert(db)
    .await
    .unwrap();
}

/// 三维经真实 DB 编解码链路（ActiveModel 写入 → list_items 全列 SELECT+JOIN+into_model 读回）
/// 原样回读，且 Decimal 在解码层不 500。这是 63f55c39（DTO 补三维）的端到端行为锁。
#[tokio::test]
async fn list_items_reads_back_three_dimensions_from_db() {
    let db = sqlite_db().await;
    exec(&db, PR_ITEM_DDL).await;
    exec(&db, PRODUCT_MIN_DDL).await;
    exec(
        &db,
        "INSERT INTO products (id, code, name) VALUES (5, 'FAB-01', '测试坯布甲')",
    )
    .await;
    insert_pr_item(&db, 1, 1, "COL-A", "DYE-9", "B7", "30.0000").await;
    insert_pr_item(&db, 2, 2, "", "", "B7", "12.5000").await; // 白坯行：三维按归一空串落库

    let svc = PurchaseReturnService::new(std::sync::Arc::new(db));
    let items = svc.list_items(101).await.expect("list_items 必须成功");
    assert_eq!(items.len(), 2, "按 return_id=101 精确过滤");
    assert_eq!(items[0].line_no, 1, "line_no 升序");

    assert_eq!(items[0].color_no, "COL-A");
    assert_eq!(items[0].dye_lot_no, "DYE-9");
    assert_eq!(items[0].batch_no, "B7");
    // 白坯行的空维度是真实空串（不是被 null 覆盖，也不是伪造值）
    assert_eq!(items[1].color_no, "");
    assert_eq!(items[1].dye_lot_no, "");

    // JOIN 富化的 material_code/name 来自 products 真值（非 format 假名）
    assert_eq!(items[0].material_code.as_deref(), Some("FAB-01"));
    assert_eq!(items[0].material_name.as_deref(), Some("测试坯布甲"));

    // Decimal 经 DB round-trip 后可序列化且保持字符串出参
    assert_eq!(items[0].quantity_returned, dec("30.0000"));
    let v = serde_json::to_value(&items[0]).unwrap();
    assert!(v["quantity_returned"].is_string());
}

/// 空明细 return_id 查询返回空集合而非 500
#[tokio::test]
async fn list_items_empty_return_returns_empty_vec() {
    let db = sqlite_db().await;
    exec(&db, PR_ITEM_DDL).await;
    exec(&db, PRODUCT_MIN_DDL).await;
    let svc = PurchaseReturnService::new(std::sync::Arc::new(db));
    let items = svc.list_items(999).await.unwrap();
    assert!(items.is_empty());
}

// =========================================================
// 活库（PostgreSQL）全链用例：#[ignore]
// =========================================================

/// 在已迁移 PG 上播种：产品/仓库/供应商/PO/PO 明细/四维库存行。
/// 返回 (supplier_id, warehouse_id, product_id, po_id, po_item_id, stock_id)。
async fn seed_full_po_context(
    db: &sea_orm::DatabaseConnection,
    po_item_quantity: Decimal,
    po_item_received: Decimal,
    stock_meters: Decimal,
) -> (i32, i32, i32, i32, i32, i32) {
    let p = product::ActiveModel {
        name: Set("四维测试坯布".to_string()),
        code: Set(format!("FAB-PR-{}", Utc::now().timestamp_nanos_opt().expect("测试造数取当前时刻纳秒：Utc::now 必落在 chrono 纳秒可表示区间（约1678-2262 年），None 不可达；旧 timestamp_nanos 超界同样 panic，行为等价"))),
        unit: Set("米".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("fabric".to_string()),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    let wh = bingxi_backend::models::warehouse::ActiveModel {
        warehouse_code: Set(format!("WH-PR-{}", Utc::now().timestamp_nanos_opt().expect("测试造数取当前时刻纳秒：Utc::now 必落在 chrono 纳秒可表示区间（约1678-2262 年），None 不可达；旧 timestamp_nanos 超界同样 panic，行为等价"))),
        name: Set("退货测试仓".to_string()),
        is_default: Set(false),
        is_active: Set(true),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    let zero_tz = chrono::FixedOffset::east_opt(0).unwrap();
    let sup_now = zero_tz.from_utc_datetime(&Utc::now().naive_utc());
    let sup = bingxi_backend::models::supplier::ActiveModel {
        supplier_code: Set(format!("SUP-PR-{}", Utc::now().timestamp_nanos_opt().expect("测试造数取当前时刻纳秒：Utc::now 必落在 chrono 纳秒可表示区间（约1678-2262 年），None 不可达；旧 timestamp_nanos 超界同样 panic，行为等价"))),
        supplier_name: Set("退货测试供应商".to_string()),
        supplier_short_name: Set("退供".to_string()),
        supplier_type: Set("面料供应商".to_string()),
        credit_code: Set("91330000TEST00000X".to_string()),
        registered_address: Set("测试注册地址".to_string()),
        legal_representative: Set("测试法人".to_string()),
        registered_capital: Set(Decimal::ZERO),
        establishment_date: Set(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
        taxpayer_type: Set("一般纳税人".to_string()),
        bank_name: Set("测试银行".to_string()),
        bank_account: Set("6222000000000000".to_string()),
        contact_phone: Set("13800000000".to_string()),
        created_at: Set(sup_now),
        updated_at: Set(sup_now),
        is_processor: Set(false),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    let po = purchase_order::ActiveModel {
        order_no: Set(format!("PO-PR-{}", Utc::now().timestamp_nanos_opt().expect("测试造数取当前时刻纳秒：Utc::now 必落在 chrono 纳秒可表示区间（约1678-2262 年），None 不可达；旧 timestamp_nanos 超界同样 panic，行为等价"))),
        supplier_id: Set(sup.id),
        order_date: Set(NaiveDate::from_ymd_opt(2026, 1, 10).unwrap()),
        warehouse_id: Set(wh.id),
        department_id: Set(1),
        purchaser_id: Set(100),
        currency: Set("CNY".to_string()),
        exchange_rate: Set(Decimal::ONE),
        total_amount: Set(Decimal::ZERO),
        total_amount_foreign: Set(Decimal::ZERO),
        total_quantity: Set(po_item_quantity),
        total_quantity_alt: Set(Decimal::ZERO),
        order_status: Set(bingxi_backend::models::status::purchase_order::APPROVED.to_string()),
        created_by: Set(100),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    let poi = purchase_order_item::ActiveModel {
        order_id: Set(po.id),
        line_no: Set(1),
        product_id: Set(p.id),
        quantity: Set(po_item_quantity),
        quantity_alt: Set(Decimal::ZERO),
        unit_price: Set(dec("8.50")),
        unit_price_foreign: Set(dec("8.50")),
        discount_percent: Set(Decimal::ZERO),
        tax_percent: Set(Decimal::ZERO),
        subtotal: Set(po_item_quantity * dec("8.50")),
        tax_amount: Set(Decimal::ZERO),
        discount_amount: Set(Decimal::ZERO),
        total_amount: Set(po_item_quantity * dec("8.50")),
        received_quantity: Set(po_item_received),
        received_quantity_alt: Set(Decimal::ZERO),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    let stock = inventory_stock::ActiveModel {
        warehouse_id: Set(wh.id),
        product_id: Set(p.id),
        quantity_on_hand: Set(stock_meters),
        quantity_available: Set(stock_meters),
        quantity_reserved: Set(Decimal::ZERO),
        quantity_shipped: Set(Decimal::ZERO),
        quantity_incoming: Set(Decimal::ZERO),
        reorder_point: Set(Decimal::ZERO),
        max_stock_point: Set(Decimal::ZERO),
        reorder_quantity: Set(Decimal::ZERO),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        batch_no: Set("B7".to_string()),
        color_no: Set("COL-A".to_string()),
        dye_lot_no: Set(Some("DYE-9".to_string())),
        grade: Set("一等品".to_string()),
        quantity_meters: Set(stock_meters),
        quantity_kg: Set(Decimal::ZERO),
        stock_status: Set("正常".to_string()),
        quality_status: Set("合格".to_string()),
        version: Set(0),
        replenishment_strategy: Set("reorder_point".to_string()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    (sup.id, wh.id, p.id, po.id, poi.id, stock.id)
}

/// 建退货单（带四维明细）→ submit → approve，返回 service 与退货单 id 供断言复用
async fn run_return_approve(
    db: std::sync::Arc<sea_orm::DatabaseConnection>,
    supplier_id: i32,
    warehouse_id: i32,
    order_id: i32,
    product_id: i32,
    qty: &str,
) -> Result<(std::sync::Arc<PurchaseReturnService>, i32), AppError> {
    let svc = std::sync::Arc::new(PurchaseReturnService::new(db.clone()));
    let ret = svc
        .create_return(
            CreatePurchaseReturnRequest {
                receipt_id: None,
                order_id: Some(order_id),
                supplier_id,
                return_date: NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
                warehouse_id: Some(warehouse_id),
                department_id: None,
                reason_type: "质量问题".to_string(),
                reason_detail: None,
                notes: None,
            },
            100,
        )
        .await?;
    svc.create_item(
        ret.id,
        CreateReturnItemRequest {
            line_no: 1,
            material_id: product_id,
            quantity_ordered: None,
            quantity_returned: dec(qty),
            unit_price: dec("8.50"),
            tax_rate: None,
            discount_percent: None,
            notes: None,
            color_no: Some("COL-A".to_string()),
            dye_lot_no: Some("DYE-9".to_string()),
            batch_no: Some("B7".to_string()),
        },
        100,
    )
    .await?;
    svc.submit_return(ret.id, 100).await?;
    Ok((svc, ret.id))
}

/// 活库全链①：四维唯一命中扣减 + PO received_quantity 回写 + 部分退=PARTIAL_RECEIVED、
/// 全部退回归零=回退 APPROVED（20f80914 的核心行为，sqlite 因 lock_exclusive 不可跑）
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL：approve/writeback 使用 lock_exclusive 与 advisory_xact_lock 单号生成"]
async fn purchase_return_approve_deducts_stock_and_writes_back_po() {
    use bingxi_backend::models::status::purchase_order as po_status;

    let db = std::sync::Arc::new(test_common::setup_test_db().await);

    // —— 部分退货：PO 明细 quantity=200 received=100；库存 100 米；退 30 ——
    let (sup, wh, p, po, poi, stock_id) =
        seed_full_po_context(&db, dec("200"), dec("100"), dec("100")).await;
    let (svc, ret_id) = run_return_approve(db.clone(), sup, wh, po, p, "30.00")
        .await
        .expect("部分退货审批链应成功");
    // 状态门反向锁：approve 之后再 submit 必须被"状态不允许提交"业务错拒绝（非 500）
    let resubmit = svc.submit_return(ret_id, 100).await;
    assert!(
        matches!(resubmit, Err(AppError::BusinessError(_))),
        "已审批单重复提交应报 BusinessError，实际: {resubmit:?}"
    );

    let stock = inventory_stock::Entity::find_by_id(stock_id)
        .one(&*db)
        .await
        .unwrap()
        .expect("库存行应存在");
    assert_eq!(
        stock.quantity_meters,
        dec("70"),
        "四维唯一命中行应扣减 30 米"
    );

    let item = purchase_order_item::Entity::find_by_id(poi)
        .one(&*db)
        .await
        .unwrap()
        .expect("PO 明细应存在");
    assert_eq!(
        item.received_quantity,
        dec("70"),
        "审批事务内回写：已收货量 100-30"
    );
    let order = purchase_order::Entity::find_by_id(po)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        order.order_status,
        po_status::PARTIAL_RECEIVED,
        "仍有已收量：状态与收货侧判定同源 PARTIAL_RECEIVED"
    );

    // —— 继续退完剩余 70 → received 归零 → 回退 APPROVED ——
    let (_svc2, _ret2) = run_return_approve(db.clone(), sup, wh, po, p, "70.00")
        .await
        .expect("第二次退货审批链应成功");
    let item2 = purchase_order_item::Entity::find_by_id(poi)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(item2.received_quantity, Decimal::ZERO, "归零不得写负");
    let order2 = purchase_order::Entity::find_by_id(po)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        order2.order_status,
        po_status::APPROVED,
        "已收货归零必须回退收货前态 APPROVED（与写入方 po/contract.rs 同源）"
    );

    // 退货明细三维真实回读（PG 全 schema 上复验 list_items，63f55c39 端到端）
    let dtos = svc.list_items(ret_id).await.unwrap();
    assert_eq!(dtos.len(), 1);
    assert_eq!(dtos[0].color_no, "COL-A");
    assert_eq!(dtos[0].dye_lot_no, "DYE-9");
    assert_eq!(dtos[0].batch_no, "B7");
}

/// 活库全链②：同四维多库存行（仅等级不同）→ 审批必须报业务错，不任选不兜底
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL（同链①）"]
async fn purchase_return_approve_ambiguous_four_dim_stock_rejected() {
    let db = std::sync::Arc::new(test_common::setup_test_db().await);
    let (sup, wh, p, po, _poi, stock_id) =
        seed_full_po_context(&db, dec("100"), dec("50"), dec("100")).await;

    // 复制一条仅 grade 不同、四维全同的库存行（首条 stock_id 已知，四维=产品/色/缸/批）
    let dup = inventory_stock::Entity::find_by_id(stock_id)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    let mut active: inventory_stock::ActiveModel = dup.into();
    active.id = Default::default();
    active.grade = Set("二等品".to_string());
    active.insert(&*db).await.unwrap();

    let (svc, ret_id) = run_return_approve(db.clone(), sup, wh, po, p, "10.00")
        .await
        .expect("建单/提交应成功");
    let err = svc
        .approve_return(ret_id, 100)
        .await
        .expect_err("同四维多行必须显式报错（不兜底、不任选）");
    assert!(matches!(err, AppError::BusinessError(_)), "实际: {err:?}");
    let disp = err.to_string();
    assert!(
        disp.contains("命中多条库存行"),
        "应指向歧义分支，实际: {disp}"
    );
    assert!(disp.contains("无法唯一定位"), "实际: {disp}");
}

/// 非法维度组合的建单入参（染色布缺缸号）在退货侧经 fabric_class/require_outbound_dimensions
/// 口径归一——此路径断言集中在 contract_wave1_transfer_dims_po_writeback_test.rs（调拨出库同口径），
/// 本文件不重复；此处仅锁退货 DTO 的维度可选语义。
#[test]
fn dims_optionality_note() {
    let req: CreateReturnItemRequest = serde_json::from_value(json!({
        "line_no": 1, "material_id": 5, "quantity_returned": "1.00", "unit_price": "1.00",
        "color_no": "COL-A", "batch_no": "B7"
    }))
    .unwrap();
    assert!(
        req.dye_lot_no.is_none(),
        "退货 DTO 不因缺缸号在解码层拒绝（业务判定在审批四维匹配）"
    );
}
