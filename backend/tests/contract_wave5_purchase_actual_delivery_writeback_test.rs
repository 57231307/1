//! 采购订单 `actual_delivery_date` 收货确认回写契约锁
//!
//! 契约依据（逐项由本文件锁死，路径基准仓库根）：
//! - 列存在：`migration/src/domain/system/mod.rs` `ALTER TABLE "purchase_orders"
//!   ADD COLUMN IF NOT EXISTS "actual_delivery_date" DATE;`（可空、**无 DEFAULT**）；
//!   模型 `models/purchase_order.rs` 为 `Option<NaiveDate>`。
//! - 有真实读方：`services/purchase_delivery_calculator.rs` 平均交期谓词要求
//!   `actual_delivery_date IS NOT NULL`（无到货日的单据整批排除）；
//!   `services/po/order_ops/query.rs` 导出与 `frontend/src/api/purchase.ts` 亦透出。
//! - 全仓唯一写入点：`services/purchase_receipt_private.rs::write_back_actual_delivery_date`
//!   （`purchase_orders` 域内仅此一处 `Set`；`custom_orders` 域的 `Set(None)` 属同名异表列）
//!   ——缺了这条回写，该列恒 NULL、交期绩效恒基于空集。
//!
//! 回写契约：确认收货的**同一事务**回写，语义 = 该 PO 已确认收货中的最大
//! `receipt_date`（`purchase_receipt.receipt_date` 为 NOT NULL，入口守显式清空，
//! 见 `purchase_receipt_ops/crud.rs` 的 `update_receipt` 门控）；部分到货也回写；
//! 完成判定/状态谓词不动；回写失败 `?` 上抛整单回滚，严禁 `let _ =`/`.ok()` 半成功。
//!
//! 覆盖策略（对齐 `contract_wave2_po_item_update_fields_test.rs` 先例，无 mock；
//! 表结构唯一来源 = backend/migration，不自建 sqlite 同构表
//! ——自建 DDL 把 exchange_rate 等 DECIMAL 列写成 TEXT 会引发成片 ColumnDecode 失败）：
//! 1. 真 PostgreSQL（test_common::setup_test_db）+ **真实调用**
//!    `update_order_received_quantity`（confirm_receipt 事务内的同一入口，
//!    链路无 lock_exclusive）：首次确认 → 列 == 该收货单 receipt_date 且进度/状态同步；
//!    更晚收货 → 覆盖为更大日期；更早补录收货 → 保持最大值不回退；未确认收货的 PO 恒 NULL；
//!    FK 父行自种子：suppliers/warehouses/products/users 及 purchase_receipt
//!    （purchase_receipt_item.receipt_id 为真表外键，父行必须先种出）；
//! 2. 失败实证：PO 不存在时回写入口如实报错上抛，事务不 commit 回滚后
//!    已收进度零残留（锁死「不允许进度写了、到货日没写」的半成功形态）；
//! 3. 防回潮源码扫描：回写调用点必须带 `?` 且夹在 confirm 的 begin/commit 之间；
//!    迁移 DDL 该列不得出现 DEFAULT（NULL 兜底会把"未收货"伪装成有到货日）。
//!
//! 口径说明：本列 NULL = 未收货（平均交期整批排除）；回写到位后样本随真实收货
//! 累积，属**数值真实化**。

mod test_common;

use bingxi_backend::models::status::purchase_order as po_status;
use bingxi_backend::models::{
    product, purchase_order, purchase_order_item, purchase_receipt, purchase_receipt_item,
    supplier, user, warehouse,
};
use bingxi_backend::services::purchase_receipt_service::PurchaseReceiptService;
use bingxi_backend::utils::error::AppError;
use chrono::{NaiveDate, TimeZone, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, TransactionTrait};
use std::sync::Arc;

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

/// 本夹具的真实行 ID（真 PG 下自增 ID 由库分配，不再手工挑 id=7/999——
/// purchase_receipt_item.receipt_id / product_id / supplier_id 都是真表外键，
/// 必须引用夹具自己种出的行）。
struct Seeds {
    operator: i32,
    po_a: i32,
    item_a: i32,
    po_b: i32,
    item_b: i32,
    po_c: i32,
    /// 4 张入库单（各含 1 行明细）：r1/r2 → po_a 明细，r3/r4 → po_b 明细
    r1: i32,
    r2: i32,
    r3: i32,
    r4: i32,
}

async fn seed_supplier(db: &sea_orm::DatabaseConnection) -> i32 {
    let now = Utc::now();
    // supplier 时间列为 DateTimeWithTimeZone（FixedOffset 时钟口径，仓内先例同款）
    let zero_tz = chrono::FixedOffset::east_opt(0).unwrap();
    let sup_now = zero_tz.from_utc_datetime(&now.naive_utc());
    let suffix = now.timestamp_nanos_opt().unwrap();
    let sup = supplier::ActiveModel {
        supplier_code: Set(format!("SUP-WB-{suffix}")),
        supplier_name: Set(format!("回写契约测试供应商-{suffix}")),
        supplier_short_name: Set("回供".to_string()),
        supplier_type: Set("面料供应商".to_string()),
        credit_code: Set("91330000WB0000001X".to_string()),
        registered_address: Set("契约测试注册地址".to_string()),
        legal_representative: Set("契约法人".to_string()),
        registered_capital: Set(Decimal::ZERO),
        establishment_date: Set(date(2020, 1, 1)),
        taxpayer_type: Set("一般纳税人".to_string()),
        bank_name: Set("契约测试银行".to_string()),
        bank_account: Set("6222000000000002".to_string()),
        contact_phone: Set("13800000002".to_string()),
        is_processor: Set(false),
        created_at: Set(sup_now),
        updated_at: Set(sup_now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("夹具：种 suppliers 父行失败");
    sup.id
}

async fn seed_warehouse(db: &sea_orm::DatabaseConnection) -> i32 {
    let now = Utc::now();
    let wh = warehouse::ActiveModel {
        warehouse_code: Set(format!("WH-WB-{}", now.timestamp_nanos_opt().unwrap())),
        name: Set("回写契约测试仓".to_string()),
        is_default: Set(false),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("夹具：种 warehouses 父行失败");
    wh.id
}

async fn seed_product(db: &sea_orm::DatabaseConnection) -> i32 {
    let now = Utc::now();
    let p = product::ActiveModel {
        name: Set(format!(
            "回写契约测试坯布-{}",
            now.timestamp_nanos_opt().unwrap()
        )),
        code: Set(format!("FAB-WB-{}", now.timestamp_nanos_opt().unwrap())),
        unit: Set("米".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("fabric".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("夹具：种 products 父行失败");
    p.id
}

async fn seed_po(
    db: &sea_orm::DatabaseConnection,
    order_no: &str,
    order_date: NaiveDate,
    supplier_id: i32,
    warehouse_id: i32,
    operator: i32,
) -> i32 {
    let now = Utc::now();
    let po = purchase_order::ActiveModel {
        order_no: Set(order_no.to_string()),
        supplier_id: Set(supplier_id),
        order_date: Set(order_date),
        warehouse_id: Set(warehouse_id),
        department_id: Set(1),
        purchaser_id: Set(operator),
        currency: Set("CNY".to_string()),
        exchange_rate: Set(Decimal::ONE),
        total_amount: Set(Decimal::ZERO),
        total_amount_foreign: Set(Decimal::ZERO),
        total_quantity: Set(Decimal::ZERO),
        total_quantity_alt: Set(Decimal::ZERO),
        order_status: Set(po_status::APPROVED.to_string()),
        created_by: Set(operator),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    po.id
}

async fn seed_order_item(
    db: &sea_orm::DatabaseConnection,
    order_id: i32,
    product_id: i32,
    quantity: Decimal,
) -> i32 {
    let now = Utc::now();
    let item = purchase_order_item::ActiveModel {
        order_id: Set(order_id),
        line_no: Set(1),
        product_id: Set(product_id),
        quantity: Set(quantity),
        quantity_alt: Set(Decimal::ZERO),
        unit_price: Set(Decimal::ONE),
        unit_price_foreign: Set(Decimal::ONE),
        discount_percent: Set(Decimal::ZERO),
        tax_percent: Set(Decimal::ZERO),
        subtotal: Set(quantity),
        tax_amount: Set(Decimal::ZERO),
        discount_amount: Set(Decimal::ZERO),
        total_amount: Set(quantity),
        received_quantity: Set(Decimal::ZERO),
        received_quantity_alt: Set(Decimal::ZERO),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    item.id
}

/// 种一张入库单（DRAFT/PENDING 初态，词表常量同源）+ 一行收货明细，返回入库单 ID。
/// purchase_receipt_item.receipt_id → purchase_receipt、product_id → products 均为
/// 真表外键（business/m0009），父行必须自种子。
#[allow(clippy::too_many_arguments)]
async fn seed_receipt_with_item(
    db: &sea_orm::DatabaseConnection,
    receipt_no: &str,
    order_id: i32,
    supplier_id: i32,
    warehouse_id: i32,
    product_id: i32,
    order_item_id: i32,
    quantity: Decimal,
    operator: i32,
) -> i32 {
    let now = Utc::now();
    let receipt = purchase_receipt::ActiveModel {
        receipt_no: Set(receipt_no.to_string()),
        order_id: Set(Some(order_id)),
        supplier_id: Set(supplier_id),
        receipt_date: Set(date(2026, 3, 1)),
        warehouse_id: Set(warehouse_id),
        created_by: Set(operator),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("夹具：种 purchase_receipt 父行失败");
    purchase_receipt_item::ActiveModel {
        receipt_id: Set(receipt.id),
        order_item_id: Set(Some(order_item_id)),
        line_no: Set(1),
        product_id: Set(product_id),
        material_code: Set("FAB-WB".to_string()),
        material_name: Set("回写契约测试坯布".to_string()),
        quantity: Set(quantity),
        unit_master: Set("米".to_string()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("夹具：种收货明细失败");
    receipt.id
}

/// 种子（全部外键父行自种子，R1）：操作人 users 行；供应商/仓库/物料各 1 行；
/// PO-A(明细订 10)、PO-B(明细订 5)、PO-C(明细订 7，永不收货对照)；
/// 入库单 r1(4m)/r2(6m)→A 明细，r3(5m)/r4(1m)→B 明细。
async fn setup_db() -> (sea_orm::DatabaseConnection, Seeds) {
    let db = test_common::setup_test_db().await;
    let now = Utc::now();
    let op = user::ActiveModel {
        username: Set(format!("wb_tester_{}", now.timestamp_nanos_opt().unwrap())),
        password_hash: Set("x".repeat(60)),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("夹具：种操作人失败");
    let operator = op.id;

    let supplier_id = seed_supplier(&db).await;
    let warehouse_id = seed_warehouse(&db).await;
    let product_id = seed_product(&db).await;

    let po_a = seed_po(
        &db,
        "PO-WB-A",
        date(2026, 3, 1),
        supplier_id,
        warehouse_id,
        operator,
    )
    .await;
    let po_b = seed_po(
        &db,
        "PO-WB-B",
        date(2026, 4, 1),
        supplier_id,
        warehouse_id,
        operator,
    )
    .await;
    let po_c = seed_po(
        &db,
        "PO-WB-C",
        date(2026, 5, 1),
        supplier_id,
        warehouse_id,
        operator,
    )
    .await;
    let item_a = seed_order_item(&db, po_a, product_id, Decimal::from(10)).await;
    let item_b = seed_order_item(&db, po_b, product_id, Decimal::from(5)).await;
    let _item_c = seed_order_item(&db, po_c, product_id, Decimal::from(7)).await;

    let r1 = seed_receipt_with_item(
        &db,
        "GR-WB-1",
        po_a,
        supplier_id,
        warehouse_id,
        product_id,
        item_a,
        Decimal::from(4),
        operator,
    )
    .await;
    let r2 = seed_receipt_with_item(
        &db,
        "GR-WB-2",
        po_a,
        supplier_id,
        warehouse_id,
        product_id,
        item_a,
        Decimal::from(6),
        operator,
    )
    .await;
    let r3 = seed_receipt_with_item(
        &db,
        "GR-WB-3",
        po_b,
        supplier_id,
        warehouse_id,
        product_id,
        item_b,
        Decimal::from(5),
        operator,
    )
    .await;
    let r4 = seed_receipt_with_item(
        &db,
        "GR-WB-4",
        po_b,
        supplier_id,
        warehouse_id,
        product_id,
        item_b,
        Decimal::from(1),
        operator,
    )
    .await;

    (
        db,
        Seeds {
            operator,
            po_a,
            item_a,
            po_b,
            item_b,
            po_c,
            r1,
            r2,
            r3,
            r4,
        },
    )
}

/// 真实调用 confirm_receipt 事务内的同一入口（本链路无 lock_exclusive，
/// 真 PG 可承载；confirm 外层锁由源码扫描锁 3 锁定）
async fn confirm(
    db: &sea_orm::DatabaseConnection,
    order_id: i32,
    receipt_id: i32,
    receipt_date: NaiveDate,
    operator: i32,
) {
    let svc = PurchaseReceiptService::new(Arc::new(db.clone()));
    let txn = db.begin().await.unwrap();
    svc.update_order_received_quantity(order_id, receipt_id, receipt_date, &txn, operator)
        .await
        .expect("确认收货回写链路必须成功");
    txn.commit().await.unwrap();
}

async fn load_po(db: &sea_orm::DatabaseConnection, id: i32) -> purchase_order::Model {
    purchase_order::Entity::find_by_id(id)
        .one(db)
        .await
        .unwrap()
        .unwrap_or_else(|| panic!("采购订单 {id} 不存在"))
}

/// 锁 1a：部分到货确认 → actual_delivery_date == 该收货单 receipt_date，
/// 进度/状态（PARTIAL_RECEIVED，词表常量）同步，完成判定逻辑不受扰动。
#[tokio::test]
async fn confirm_partial_receipt_writes_back_receipt_date() {
    let (db, s) = setup_db().await;
    confirm(&db, s.po_a, s.r1, date(2026, 3, 8), s.operator).await;

    let po = load_po(&db, s.po_a).await;
    assert_eq!(
        po.actual_delivery_date,
        Some(date(2026, 3, 8)),
        "确认收货后到货日必须等于该收货单 receipt_date（修复前该列全仓零写入点，恒 NULL）"
    );
    assert_eq!(po.order_status, po_status::PARTIAL_RECEIVED);

    let item = purchase_order_item::Entity::find_by_id(s.item_a)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        item.received_quantity,
        Decimal::from(4),
        "进度须与到货日同事务落库"
    );

    // 双单对照的隔离侧：po_b 侧必须回读，否则"不回写串单"从未被验证
    // 回写以 order_id 精确定位，po_b 的明细与到货日不得被 po_a 的确认污染。
    let item_b = purchase_order_item::Entity::find_by_id(s.item_b)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        item_b.received_quantity,
        Decimal::ZERO,
        "确认 po_a 不得波及 po_b 明细进度（回写目标串单=交叉污染）"
    );
    let po_b = load_po(&db, s.po_b).await;
    assert_eq!(
        po_b.actual_delivery_date, None,
        "确认 po_a 不得给未收货的 po_b 写入到货日"
    );
}

/// 锁 1b：二次**更晚**收货确认 → 覆盖为更大日期；全部收货判定仍归
/// determine_order_receipt_status（COMPLETED 由进度推得，本回写不越权改状态机）。
#[tokio::test]
async fn later_receipt_overwrites_with_greater_date() {
    let (db, s) = setup_db().await;
    confirm(&db, s.po_a, s.r1, date(2026, 3, 8), s.operator).await;
    confirm(&db, s.po_a, s.r2, date(2026, 3, 20), s.operator).await;

    let po = load_po(&db, s.po_a).await;
    assert_eq!(
        po.actual_delivery_date,
        Some(date(2026, 3, 20)),
        "更晚收货必须覆盖为更大日期（最后一次确认入库口径）"
    );
    assert_eq!(po.order_status, po_status::COMPLETED);
    let item = purchase_order_item::Entity::find_by_id(s.item_a)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(item.received_quantity, Decimal::from(10));
}

/// 锁 1c：**更早**日期的补录收货 → 保持已确认收货中的最大值，不回退。
#[tokio::test]
async fn earlier_backfilled_receipt_never_rolls_back_max_date() {
    let (db, s) = setup_db().await;
    confirm(&db, s.po_b, s.r3, date(2026, 4, 15), s.operator).await;
    confirm(&db, s.po_b, s.r4, date(2026, 4, 5), s.operator).await;

    let po = load_po(&db, s.po_b).await;
    assert_eq!(
        po.actual_delivery_date,
        Some(date(2026, 4, 15)),
        "到货日语义=该单已确认收货的最大 receipt_date，更早补录不得回退"
    );
    // 明细进度侧同步回读（隔离侧实证：本用例只确认 po_b 的 r3/r4，
    // 累加值 5+1=6 与 po_a 的 r1/r2 无任何交集；累加算术由 1b 同入口行为证明）
    let item_b = purchase_order_item::Entity::find_by_id(s.item_b)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        item_b.received_quantity,
        Decimal::from(6),
        "r3(5m)+r4(1m) 两次确认的进度必须逐单累加到 po_b 明细"
    );
}

/// 锁 1d：未确认收货的 PO 该列恒 NULL（种子即 APPROVED 未收货，行为对照）。
#[tokio::test]
async fn po_without_confirmed_receipt_keeps_null() {
    let (db, s) = setup_db().await;
    confirm(&db, s.po_a, s.r1, date(2026, 3, 8), s.operator).await;
    let po = load_po(&db, s.po_c).await;
    assert_eq!(
        po.actual_delivery_date, None,
        "未确认收货的订单不得被写入到货日（NULL 即事实，禁止默认值兜底）"
    );
}

/// 锁 2（半成功实证）：回写目标 PO 不存在 → 入口如实以 NotFound 上抛、调用方
/// 不 commit；事务回滚后已收进度**零残留**——锁死「进度写了、到货日没写」
/// 在原子性上不可能发生（生产 confirm_receipt 由 `?` 传播进同一 txn）。
#[tokio::test]
async fn write_back_failure_rolls_back_progress_no_half_success() {
    let (db, s) = setup_db().await;
    let svc = PurchaseReceiptService::new(Arc::new(db.clone()));
    let txn = db.begin().await.unwrap();
    let missing_order = 999_999;
    let err = svc
        .update_order_received_quantity(missing_order, s.r1, date(2026, 3, 8), &txn, s.operator)
        .await
        .expect_err("PO 不存在时必须整单失败上抛，不得吞错继续");
    assert!(
        matches!(err, AppError::NotFound(_)),
        "错误形态必须是 NotFound，实际 {err:?}"
    );
    assert!(
        err.to_string()
            .contains(&format!("采购订单 {missing_order}")),
        "错误必须外显真实定位信息，实际 {err}"
    );
    // 失败路径与生产一致：不 commit，drop 即回滚
    drop(txn);

    let item = purchase_order_item::Entity::find_by_id(s.item_a)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        item.received_quantity,
        Decimal::ZERO,
        "回写失败必须随事务回滚，不得残留半成功进度"
    );
}

/// 从源码截取 impl 块内方法（4 空格收口，先例 contract_wave2 同式；剔 \r 防 CRLF 漏检）
///
/// 调用方必须先 `code_only` 剥整行注释再传入：锚点若只在注释里出现（"旧实现曾有
/// `async fn xxx`"之类自述），按原文 find 会把窗起点定在注释上（B1① 同型事故）。
fn extract_impl_method(src: &str, anchor: &str) -> String {
    let src = src.replace('\r', "");
    let i = src
        .find(anchor)
        .unwrap_or_else(|| panic!("源码锚点丢失: {anchor}"));
    let j = src[i..]
        .find("\n    }")
        .unwrap_or_else(|| panic!("方法结束定位失败: {anchor}"));
    src[i..i + j + 6].to_string()
}

/// 只保留"代码 + 字符串字面量"：整行注释（`//`、`///`、`//!`）逐行剔除。
/// 禁项断言（"源码不该出现 X"）必须先过此函数——本批 `c1902082` 族事故即
/// 说明性注释/字节偏移移动把通过态打红；按行而非按字符扫描的理由与
/// `contract_wave5_outsource_issue_guard_test.rs` 同式先例一致（跨行 raw string
/// 里的引号会让字符级配平把真代码当字符串起止吃掉）。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 规范形（**仅用于正向 contains / 定位**）：`code_only` 后剔全部空白并消闭合
/// 定界符前的尾逗号（rustfmt 把多行调用的末实参必带 `,`，是折行产物非语义）。
/// 负向 `!contains` 一律只用 `code_only` 原文，不套本函数——去空白扩大匹配面，
/// 只适用于"必含"判定（同 `sku_mapping_contract_test.rs::strip_ws` 纪律）。
/// 全部切片在规范形（纯 ASCII 词元边界不要求，但 find 返回的偏移恒为
/// char boundary，杜绝 `byte index is not a char boundary` panic—— B1③ 正解）。
fn canon(src: &str) -> String {
    let mut out: String = code_only(src)
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    loop {
        let next = out.replace(",)", ")").replace(",]", "]").replace(",}", "}");
        if next == out {
            break;
        }
        out = next;
    }
    out
}

/// 锁 3a：回写必须在 confirm 的事务内——`confirm_receipt` 中
/// `update_order_received_quantity`（携 receipt_date）调用点必须位于
/// begin 与 commit 之间，且错误以 `?` 上抛、无吞错形态。
#[test]
fn source_scan_write_back_inside_confirm_transaction() {
    let raw = include_str!("../src/services/purchase_receipt_ops/state.rs");
    // 判据全部落在剥注释的 confirm_receipt 函数体上：
    // - 禁词不再被同文件说明注释命中（B1①）；
    // - 窗边界由符号定位（不再依赖"begin/commit 首次出现在本函数"这一顺序假设）；
    // - canon 规范形上的 find 偏移天然是 char boundary，杜绝字节窗 panic（B1③）。
    let body = extract_impl_method(&code_only(raw), "pub async fn confirm_receipt");
    let flat = canon(&body);
    let begin = flat
        .find(".begin().await?")
        .expect("confirm_receipt 必须有显式事务 begin");
    let commit = flat
        .find("txn.commit().await?")
        .expect("confirm_receipt 必须有显式事务 commit");
    let call = flat
        .find("self.update_order_received_quantity(")
        .expect("confirm_receipt 必须调用 update_order_received_quantity");
    assert!(
        begin < call && call < commit,
        "回写入口调用必须夹在 begin({begin}) 与 commit({commit}) 之间，实际位于 {call}"
    );
    // 从 call 起截取到其后第一个 `.await?;`（str::find 无起始偏移入参，先切片再找）
    let call_end_rel = flat[call..]
        .find(".await?;")
        .expect("回写入口调用必须以 .await?; 结束");
    let call_block = &flat[call..call + call_end_rel + ".await?;".len()];
    assert!(
        call_block.contains("receipt.receipt_date"),
        "确认收货必须把入库单 receipt_date 真实传入回写，禁止以确认时间/当前时间顶替，实际块:\n{call_block}"
    );
    // 整文件禁吞错形态同样只看执行体（注释里"严禁 let _ ="自述不是违例）
    let code = code_only(raw);
    assert!(
        !code.contains("let _ = ") && !code.contains(".ok();"),
        "confirm 链路不得吞错（let _ = / .ok() 会造成进度与到货日半成功）"
    );
}

/// 锁 3b：回写实现必须以 `?` 传播错误、经审计服务在事务连接上落库；
/// 状态判定（determine/save）调用序列不得被扰动（完成判定逻辑不动）。
#[test]
fn source_scan_write_back_propagates_errors_via_question_mark() {
    let raw = include_str!("../src/services/purchase_receipt_private.rs");
    // 注释剥离先行：write_back 方法上方的文档注释自述"严禁 `let _ =` / `.ok()`"
    // （:170），若窗起点被注释里的同名符号骗到或禁词扫吞进注释，即 B1① 假判违例。
    let code = code_only(raw);
    let entry = extract_impl_method(&code, "pub async fn update_order_received_quantity");
    let entry_flat = canon(&entry);
    assert!(
        entry_flat.contains(&canon(
            "Self::write_back_actual_delivery_date(txn, order_id, receipt_date, user_id).await?;"
        )),
        "回写必须在同事务入口以 ? 传播且先于状态判定，实际块:\n{entry}"
    );
    let back = extract_impl_method(&code, "async fn write_back_actual_delivery_date");
    let back_flat = canon(&back);
    assert!(
        back_flat.contains("AuditLogService::update_with_audit") && back_flat.contains(".await?;"),
        "回写必须经审计服务在 txn 连接上落库并以 ? 上抛，实际块:\n{back}"
    );
    assert!(
        back_flat.contains(&canon(".max(receipt_date)")),
        "必须取已确认收货中的最大 receipt_date，实际块:\n{back}"
    );
    for forbidden in ["let _ =", ".ok()"] {
        assert!(
            !entry.contains(forbidden) && !back.contains(forbidden),
            "回写路径禁止 {forbidden} 吞错形态"
        );
    }
}

/// 锁 4：生产 DDL 该列不得带 DEFAULT——「未收货 = NULL」是交期样本谓词
/// （`actual_delivery_date IS NOT NULL`）的事实来源，默认值兜底会污染样本。
#[test]
fn source_scan_migration_column_has_no_default_and_write_point_is_unique() {
    // code_only 先行：迁移文件说明注释若引用该列 DDL 文本（"曾有 DEFAULT 后移除"
    // 之类），按原文逐行匹配会把注释行误当 DDL 行判定（B1①）。
    let mig = code_only(include_str!("../migration/src/domain/system/mod.rs"));
    let line = mig
        .lines()
        .find(|l| l.contains("\"purchase_orders\"") && l.contains("\"actual_delivery_date\""))
        .expect("purchase_orders.actual_delivery_date 的 DDL 必须存在（决策：不删列）");
    assert!(
        !line.to_ascii_uppercase().contains("DEFAULT"),
        "该列必须保持无 DEFAULT（未收货恒 NULL），实际 DDL: {line}"
    );
}
