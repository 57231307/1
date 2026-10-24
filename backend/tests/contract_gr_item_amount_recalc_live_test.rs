//! 采购收货行金额重算的活库契约锁
//!
//! 功能：钉收货明细更新口（`update_receipt_item`）的一条金额不变量。改数量、改单价
//!       或显式清空单价之后，行 `amount` 必须按生效后的数量乘单价写回，单头
//!       `total_amount` 必须与明细表逐行现算的"数量乘单价之和"逐字相等。
//!       钉法是数据流而不是文本：单头取值走 GET 详情序列化的同一实体读出，
//!       和值走明细表现算，两侧在读已提交的数据上对拍。
//! 反向钉：行金额未写回，或汇总口径偏离行金额之和，即判红。
//! 变异自检：绕过服务层只把行数量改掉而让行金额停在旧值，再走同一汇总口径重算单头，
//!       断言单头与"数量乘单价之和"必然分叉，证明主判据抓得住"只重算汇总不重算
//!       行金额"这一缺陷形态，主锁不是恒真断言。
//! 调用方：集成测试 crate（CI 里对已跑完迁移链的 PostgreSQL 执行）。
//! 入参：`test_common::setup_test_db()` 注入库连接；操作人、仓库、物料由本文件自建自清。
//! 传给谁：判据结果交给断言本身，无下游。
//! 存什么：只种一单两行收货明细与其参照父行（操作人 1 名、仓 1 个、物料 1 个），
//!       suppliers 取迁移种子行不另插。
//! 存哪里：PostgreSQL `purchase_receipt` / `purchase_receipt_item` 与 `users` /
//!       `warehouses` / `products`；业务表由夹具每次 TRUNCATE 收敛，`users` 等参照表
//!       不被 TRUNCATE，故按显式 id 先删后种保证重跑收敛。
//!
//! 列型前提按真实迁移 m0009 核过：行 `amount` 为 DECIMAL(18,2) 可空列，单头
//! `total_amount` 为 DECIMAL(15,2) NOT NULL DEFAULT 0；建行口径是 amount 等于数量乘
//! 单价、单价缺省按 0，本锁沿用同一缺省语义。

mod test_common;

use bingxi_backend::decs;
use bingxi_backend::models::{purchase_receipt, purchase_receipt_item, status, user};
use bingxi_backend::services::purchase_receipt_dto::UpdateReceiptItemRequest;
use bingxi_backend::services::purchase_receipt_service::PurchaseReceiptService;
use bingxi_backend::ymd;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait,
    QueryFilter, Set, Statement,
};
use std::sync::Arc;

use test_common::setup_test_db;

const OPERATOR_ID: i32 = 9471;
const WAREHOUSE_ID: i32 = 1;
const PRODUCT_ID: i32 = 1;
const RECEIPT_ID: i32 = 9471;

fn now() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}

/// 执行一条无参原生 SQL，失败即点名是哪步种子断了
async fn exec_raw(db: &DatabaseConnection, sql: &str, what: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        Vec::<sea_orm::Value>::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("种子 {what} 失败: {e}\nSQL: {sql}"));
}

/// 种一单两行的干净起点，单头先走汇总口径对齐建行金额
async fn seed_receipt_with_items(db: &DatabaseConnection, svc: &PurchaseReceiptService) {
    // users 是参照表不被 TRUNCATE，先删后种保证重跑收敛
    user::Entity::delete_by_id(OPERATOR_ID)
        .exec(db)
        .await
        .unwrap();
    user::ActiveModel {
        id: Set(OPERATOR_ID),
        username: Set("it_gr_amount_recalc_op".to_string()),
        password_hash: Set("amount-recalc-lock-not-a-real-hash".to_string()),
        real_name: Set(Some("收货金额重算锁操作人".to_string())),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("种子操作人失败");

    exec_raw(
        db,
        &format!(
            "INSERT INTO warehouses (id, name, warehouse_code, is_active) VALUES \
             ({WAREHOUSE_ID}, '收货金额重算锁仓', 'AMT-W1', true)"
        ),
        "warehouses 父行",
    );
    exec_raw(
        db,
        &format!(
            "INSERT INTO products (id, code, name, unit, status, product_type) VALUES \
             ({PRODUCT_ID}, 'PT-AMT-R1', '收货金额重算锁面料', '米', 'active', '成品布')"
        ),
        "products 父行",
    );

    purchase_receipt::ActiveModel {
        id: Set(RECEIPT_ID),
        receipt_no: Set("IT-GR-AMT-0001".to_string()),
        order_id: Set(None),
        supplier_id: Set(1),
        receipt_date: Set(ymd!(2026, 10, 1)),
        warehouse_id: Set(WAREHOUSE_ID),
        receipt_status: Set(status::purchase_receipt::DRAFT.to_string()),
        total_quantity: Set(decs!(0)),
        total_quantity_alt: Set(decs!(0)),
        total_amount: Set(decs!(0)),
        created_by: Set(OPERATOR_ID),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("种子收货单失败");

    seed_item(db, 1, decs!(10), Some(decs!(5))).await;
    seed_item(db, 2, decs!(2), Some(decs!(3))).await;

    svc.calculate_receipt_total(RECEIPT_ID, OPERATOR_ID)
        .await
        .expect("起点汇总对齐失败");
}

/// 按建行口径种一行明细，amount 等于数量乘单价（单价缺省按 0）
async fn seed_item(
    db: &DatabaseConnection,
    line_no: i32,
    quantity: rust_decimal::Decimal,
    unit_price: Option<rust_decimal::Decimal>,
) -> purchase_receipt_item::Model {
    let amount = quantity * unit_price.unwrap_or_else(|| decs!(0));
    purchase_receipt_item::ActiveModel {
        receipt_id: Set(RECEIPT_ID),
        line_no: Set(line_no),
        product_id: Set(PRODUCT_ID),
        material_code: Set(format!("AMT-M{line_no}")),
        material_name: Set(format!("金额重算锁物料{line_no}")),
        quantity: Set(quantity),
        unit_master: Set("米".to_string()),
        unit_price: Set(unit_price),
        amount: Set(Some(amount)),
        created_at: Set(Some(now())),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("种子收货明细失败")
}

/// 从明细表现算数量乘单价之和，单价为空按 0，与建行口径同词
async fn derived_line_sum(db: &DatabaseConnection) -> rust_decimal::Decimal {
    let items = purchase_receipt_item::Entity::find()
        .filter(purchase_receipt_item::Column::ReceiptId.eq(RECEIPT_ID))
        .all(db)
        .await
        .expect("读明细失败");
    let mut sum = decs!(0);
    for item in items {
        sum += item.quantity * item.unit_price.unwrap_or_else(|| decs!(0));
    }
    sum
}

/// 读单头，走 GET 详情序列化的同一实体
async fn read_header(db: &DatabaseConnection) -> purchase_receipt::Model {
    purchase_receipt::Entity::find_by_id(RECEIPT_ID)
        .one(db)
        .await
        .expect("读单头失败")
        .expect("收货单不存在")
}

/// 主锁：改数量、改单价、显式清空单价三步走完更新口，每步之后
/// 单头 total_amount 必须与数量乘单价之和逐字相等，行 amount 必须当场写回
#[tokio::test]
async fn receipt_total_amount_matches_line_derivation_after_quantity_and_price_updates() {
    let db = setup_test_db().await;
    let svc = PurchaseReceiptService::new(Arc::new(db.clone()));
    seed_receipt_with_items(&db, &svc).await;

    let line_one = purchase_receipt_item::Entity::find()
        .filter(purchase_receipt_item::Column::LineNo.eq(1))
        .filter(purchase_receipt_item::Column::ReceiptId.eq(RECEIPT_ID))
        .one(&db)
        .await
        .unwrap()
        .expect("第 1 行不存在");

    // 第一步，只改数量，10 改到 20，单价 5 不动，行金额应从 50 重算成 100
    let updated = svc
        .update_receipt_item(
            line_one.id,
            UpdateReceiptItemRequest {
                quantity: Some(Some(decs!(20))),
                ..Default::default()
            },
            OPERATOR_ID,
        )
        .await
        .expect("改数量失败");
    assert_eq!(
        updated.amount,
        Some(decs!(100)),
        "改数量后行 amount 没按新数量乘单价写回"
    );
    let header = read_header(&db).await;
    let derived = derived_line_sum(&db).await;
    assert_eq!(
        header.total_amount, derived,
        "改数量后单头 total_amount 与数量乘单价之和脱钩，期望 {derived}，实得 {}",
        header.total_amount
    );
    assert_eq!(
        header.total_amount,
        decs!(106),
        "改数量后的期望总额口径漂移"
    );
    assert_eq!(header.total_quantity, decs!(22), "改数量后总数量应随之重算");

    // 第二步，只改单价，第 2 行 3 改到 4，数量 2 不动，行金额应从 6 重算成 8
    let line_two = purchase_receipt_item::Entity::find()
        .filter(purchase_receipt_item::Column::LineNo.eq(2))
        .filter(purchase_receipt_item::Column::ReceiptId.eq(RECEIPT_ID))
        .one(&db)
        .await
        .unwrap()
        .expect("第 2 行不存在");
    let updated = svc
        .update_receipt_item(
            line_two.id,
            UpdateReceiptItemRequest {
                unit_price: Some(Some(decs!(4))),
                ..Default::default()
            },
            OPERATOR_ID,
        )
        .await
        .expect("改单价失败");
    assert_eq!(
        updated.amount,
        Some(decs!(8)),
        "改单价后行 amount 没按数量乘新单价写回"
    );
    let header = read_header(&db).await;
    let derived = derived_line_sum(&db).await;
    assert_eq!(
        header.total_amount, derived,
        "改单价后单头 total_amount 与数量乘单价之和脱钩，期望 {derived}，实得 {}",
        header.total_amount
    );

    // 第三步，显式清空单价，数量 2 不动，行金额按建行缺省口径归 0
    let updated = svc
        .update_receipt_item(
            line_two.id,
            UpdateReceiptItemRequest {
                unit_price: Some(None),
                ..Default::default()
            },
            OPERATOR_ID,
        )
        .await
        .expect("清空单价失败");
    assert_eq!(updated.unit_price, None, "显式清空单价应落 NULL");
    assert_eq!(
        updated.amount,
        Some(decs!(0)),
        "单价清空后行 amount 应按数量乘缺省 0 重算，与建行口径一致"
    );
    let header = read_header(&db).await;
    let derived = derived_line_sum(&db).await;
    assert_eq!(
        header.total_amount, derived,
        "清空单价后单头 total_amount 与数量乘单价之和脱钩，期望 {derived}，实得 {}",
        header.total_amount
    );
}

/// 变异自检：绕过服务层只把行数量改成 30 而让行 amount 停在 50，
/// 再走同一汇总口径重算单头，单头必然与数量乘单价之和分叉。
/// 该分叉就是主锁的灵敏度证明，只重算汇总不重算行金额的形态在这条判据下必红
#[tokio::test]
async fn mutation_recomputing_only_aggregate_diverges_from_line_derivation() {
    let db = setup_test_db().await;
    let svc = PurchaseReceiptService::new(Arc::new(db.clone()));
    seed_receipt_with_items(&db, &svc).await;

    let line_one = purchase_receipt_item::Entity::find()
        .filter(purchase_receipt_item::Column::LineNo.eq(1))
        .filter(purchase_receipt_item::Column::ReceiptId.eq(RECEIPT_ID))
        .one(&db)
        .await
        .unwrap()
        .expect("第 1 行不存在");

    exec_raw(
        &db,
        &format!(
            "UPDATE purchase_receipt_item SET quantity = 30.0000 WHERE id = {}",
            line_one.id
        ),
        "变异注入改数量不动行金额",
    );
    svc.calculate_receipt_total(RECEIPT_ID, OPERATOR_ID)
        .await
        .expect("变异后重算汇总失败");

    let header = read_header(&db).await;
    let derived = derived_line_sum(&db).await;
    assert_eq!(
        header.total_amount,
        decs!(56),
        "汇总按行 amount 求和时应停在旧行金额之和 56，说明行金额确实是没被重算的旧值"
    );
    assert_eq!(
        derived,
        decs!(156),
        "变异后数量乘单价之和应为 30 乘 5 加 2 乘 3"
    );
    assert_ne!(
        header.total_amount, derived,
        "只重算汇总不重算行金额的形态必须与主锁判据分叉，两侧同值说明主锁断言恒真失效"
    );
}
