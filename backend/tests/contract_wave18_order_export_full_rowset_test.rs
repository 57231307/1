//! 订单导出全行集行为锁（采购与销售两腿，真库）
//!
//! 功能：钉死「交付行数 == 服务端全量总数」这条等式与订单号集合逐项相等，防取数腿
//! 重新加回分页或一万行上限后静默丢行（导出只回一页而无人报错的家族缺陷）。
//! 调用方：CI db-integration 串行组与 ci-test-rust-ignored 的 --run-ignored only 通道；
//! 判据主体为 PurchaseOrderService / SalesService 的 export_orders_to_xlsx 三元组出参
//! 与纯判据 export_rows_complete。
//! 入参：supplier_id（采购）与 SalesOrderFilter.customer_id（销售）按行集隔离取值；
//! status 与行级数据权限传 None（与导出腿现状口径一致，权限面差异已单独登记）。
//! 传给谁：结果不回任何视图，只回本测试断言；服务端总数走 SeaORM count 子查询。
//! 存什么：播种只写 purchase_orders / sales_orders 单头行的必填列，金额零值、可选列
//! 缺值按 NULL 落库，不伪造业务数据。
//! 存哪里：TEST_DATABASE_URL 指向的 bingxi_test 库；suppliers 为迁移种子参照表只读
//! 复用（m0015 恰好两条演示供应商，行集隔离取其一为另一侧），users/warehouses/customers
//! 属逐例 TRUNCATE 的业务表；id 走本用例私有大段（910 万/920 万/930 万），避开既有
//! 941x/947x/951x/990x/991x/992x 与 8xxx 段。
//!
//! 判据边界（如实声明）：250 行量级判别不了「超一万即丢」——旧封顶形态在 250 行同样
//! 全量返回，本文件的大行级 #[ignore] 用例（一万零一行）才是该形态的行为判别；
//! 常规量级两例判别的是「行集隔离下等式成立、订单号集合逐项相等、列形状 21 列」。
#![allow(clippy::unwrap_used, reason = "测试夹具：播种与断言取不到真实值即应判红")]
#![allow(clippy::too_many_lines, reason = "集成测试按场景分段，拆分会打散判据与夹具的同处可读性")]

mod test_common;

use bingxi_backend::models::{customer, purchase_order, sales_order, user, warehouse};
use bingxi_backend::search::ElasticClient;
use bingxi_backend::services::po::order::PurchaseOrderService;
use bingxi_backend::services::so::order::SalesService;
use bingxi_backend::services::so::order_query::SalesOrderFilter;
use chrono::{TimeZone, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveValue::Set, EntityTrait};
use std::collections::BTreeSet;
use std::sync::Arc;

/// 采购导出表头（与 query.rs 的 csv_headers 逐字同序同值，改动任何一侧本锁即红）
const PO_HEADERS: [&str; 21] = [
    "订单编号", "供应商ID", "供应商名称", "订单日期", "预计交货日期", "实际交货日期",
    "仓库ID", "仓库名称", "部门ID", "部门名称", "采购员ID", "币种", "汇率", "总金额",
    "总金额外币", "总数量", "总数量辅助", "状态", "付款条件", "运输条款", "备注",
];

/// 销售导出表头（与 delivery_ops/export.rs 的 build_order_csv_headers 逐字一致）
const SO_HEADERS: [&str; 21] = [
    "订单编号", "客户ID", "客户名称", "商机ID", "订单日期", "要求交货日期", "发货日期",
    "状态", "小计金额", "税额", "折扣金额", "运费", "总金额", "已付金额", "余额",
    "送货地址", "账单地址", "备注", "创建人ID", "审批人ID", "审批时间",
];

// ---- 本用例私有 id 段（见文件头存哪里）----
const U_OWNER: i32 = 9_100_001;
const WH_ID: i32 = 9_100_010;
const DEPT_ID: i32 = 1; // 迁移种子参照表 departments 的稳定主部门
const CUS_A: i32 = 9_100_020; // 行集隔离：本客户
const CUS_B: i32 = 9_100_021; // 行集隔离：别客户
const PO_ID_BASE: i32 = 9_200_000; // purchase_orders 播种基址
const SO_ID_BASE: i32 = 9_300_000; // sales_orders 播种基址

/// suppliers 属迁移种子参照表（不参与 TRUNCATE），m0015 恰有两行，按稳定顺序取
async fn supplier_at(db: &sea_orm::DatabaseConnection, offset: i32) -> i32 {
    use sea_orm::{ConnectionTrait, Statement};
    let row = db
        .query_one(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            format!("SELECT id FROM suppliers ORDER BY id LIMIT 1 OFFSET {offset}"),
        ))
        .await
        .unwrap()
        .expect("迁移种子 suppliers 应有两条演示供应商（m0015）");
    row.try_get_by_index::<i32>(0).unwrap()
}

async fn seed_shell(db: &sea_orm::DatabaseConnection) {
    user::ActiveModel {
        id: Set(U_OWNER),
        username: Set("exportlock_u".to_string()),
        password_hash: Set("test-only-not-a-real-hash".to_string()),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        department_id: Set(Some(DEPT_ID)),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    warehouse::ActiveModel {
        id: Set(WH_ID),
        warehouse_code: Set("EXP-LOCK-WH".to_string()),
        name: Set("导出锁测试仓".to_string()),
        is_default: Set(false),
        is_active: Set(true),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    for (cid, name) in [(CUS_A, "导出锁客户甲"), (CUS_B, "导出锁客户乙")] {
        customer::ActiveModel {
            id: Set(cid),
            customer_code: Set(format!("EXP-LOCK-{cid}")),
            customer_name: Set(name.to_string()),
            credit_limit: Set(Decimal::ZERO),
            payment_terms: Set(30),
            status: Set("active".to_string()),
            customer_type: Set("retail".to_string()),
            owner_id: Set(U_OWNER),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }
}

fn ts(k: i32) -> chrono::DateTime<Utc> {
    Utc.timestamp_opt(1_700_000_000 + i64::from(k), 0).unwrap()
}

/// 采购单头播种：Entity::insert_many 按 500 分块（Postgres 单语句 65535 参数上限，
/// 一万行必须分块；全仓 tests 无 insert_many 先例，src 侧见 bom_ops/crud.rs）
async fn seed_po(db: &sea_orm::DatabaseConnection, supplier_id: i32, seq_from: i32, count: i32, order_status: &str) {
    let models: Vec<purchase_order::ActiveModel> = (0..count)
        .map(|k| {
            let id = PO_ID_BASE + seq_from + k;
            purchase_order::ActiveModel {
                id: Set(id),
                order_no: Set(format!("EXPLOCK-PO-{id}")),
                supplier_id: Set(supplier_id),
                order_date: Set(ts(k + seq_from).date_naive()),
                warehouse_id: Set(WH_ID),
                department_id: Set(DEPT_ID),
                purchaser_id: Set(U_OWNER),
                currency: Set("CNY".to_string()),
                exchange_rate: Set(Decimal::ONE),
                total_amount: Set(Decimal::ZERO),
                total_amount_foreign: Set(Decimal::ZERO),
                total_quantity: Set(Decimal::ZERO),
                total_quantity_alt: Set(Decimal::ZERO),
                order_status: Set(order_status.to_string()),
                created_by: Set(U_OWNER),
                created_at: Set(ts(k + seq_from)),
                updated_at: Set(ts(k + seq_from)),
                ..Default::default()
            }
        })
        .collect();
    for chunk in models.chunks(500) {
        purchase_order::Entity::insert_many(chunk.to_vec())
            .exec(db)
            .await
            .unwrap();
    }
}

/// 销售单头播种：同样 500 分块；status 词表小写（写入方 models/status 销售订单节）
async fn seed_so(db: &sea_orm::DatabaseConnection, customer_id: i32, seq_from: i32, count: i32, order_status: &str) {
    let models: Vec<sales_order::ActiveModel> = (0..count)
        .map(|k| {
            let id = SO_ID_BASE + seq_from + k;
            sales_order::ActiveModel {
                id: Set(id),
                order_no: Set(format!("EXPLOCK-SO-{id}")),
                customer_id: Set(customer_id),
                order_date: Set(ts(k + seq_from)),
                status: Set(order_status.to_string()),
                subtotal: Set(Decimal::ZERO),
                tax_amount: Set(Decimal::ZERO),
                discount_amount: Set(Decimal::ZERO),
                shipping_cost: Set(Decimal::ZERO),
                total_amount: Set(Decimal::ZERO),
                paid_amount: Set(Decimal::ZERO),
                balance_amount: Set(Decimal::ZERO),
                created_by: Set(Some(U_OWNER)),
                created_at: Set(ts(k + seq_from)),
                updated_at: Set(ts(k + seq_from)),
                ..Default::default()
            }
        })
        .collect();
    for chunk in models.chunks(500) {
        sales_order::Entity::insert_many(chunk.to_vec())
            .exec(db)
            .await
            .unwrap();
    }
}

/// 表头逐字节比对（列名与列序任何一侧漂移即红）
fn assert_headers_byte_equal(headers: &[String], expected: &[&str; 21]) {
    let got: Vec<&str> = headers.iter().map(String::as_str).collect();
    assert_eq!(got, expected.to_vec(), "导出表头与逐字节基线不一致（列名或列序漂移）");
}

/// 按列名取单元格；列名不在表头即判红
fn cell(headers: &[&str; 21], row: &[String], name: &str) -> String {
    let idx = headers
        .iter()
        .position(|h| *h == name)
        .unwrap_or_else(|| panic!("表头基线缺列 {name}"));
    row[idx].clone()
}

/// 常规量级（采购腿）：行集隔离下等式成立 + 订单号集合逐项相等 + 列形状 + 判据为真
#[tokio::test]
async fn purchase_export_rowset_equals_server_total() {
    let db = Arc::new(test_common::setup_test_db().await);
    seed_shell(&db).await;
    let sup_a = supplier_at(&db, 0).await;
    let sup_b = supplier_at(&db, 1).await;
    // 行集隔离：供应商甲 250 行（200 DRAFT + 50 APPROVED），供应商乙 30 行混在同表；
    // 导出按供应商筛，筛错即漏行或多行。
    seed_po(&db, sup_a, 0, 200, "DRAFT").await;
    seed_po(&db, sup_a, 200, 50, "APPROVED").await;
    seed_po(&db, sup_b, 1000, 30, "DRAFT").await;

    let svc = PurchaseOrderService::new(Arc::clone(&db));
    let (headers, rows, server_total) = svc.export_orders_to_xlsx(None, Some(sup_a)).await.unwrap();

    assert_eq!(server_total, 250, "服务端全量总数应为供应商甲播种 250 行");
    assert_eq!(rows.len() as u64, server_total, "交付行数与服务端全量总数不等：等式即根判据");
    assert!(
        PurchaseOrderService::export_rows_complete(rows.len(), server_total),
        "等式成立时完整性判据必须为真"
    );
    assert_headers_byte_equal(&headers, &PO_HEADERS);
    assert!(rows.iter().all(|r| r.len() == 21), "每行必须 21 列，缺列即形状漂移");

    let seeded: BTreeSet<String> =
        (0..250_i32).map(|k| format!("EXPLOCK-PO-{}", PO_ID_BASE + k)).collect();
    let delivered: BTreeSet<String> =
        rows.iter().map(|r| cell(&PO_HEADERS, r, "订单编号")).collect();
    assert_eq!(delivered, seeded, "订单号集合必须与播种集合逐项相等（尾部行不得缺失）");
}

/// 常规量级（销售腿）：等式成立 + 列形状 + 不装配明细仍带出客户名称
/// （导出列形状与明细装配无关的运行时侧；静态侧由同 commit 的 static_lock 锁
/// 行键与表头逐字同序，两条装配路径各自独立通过）
#[tokio::test]
async fn sales_export_rowset_equals_server_total() {
    let db = Arc::new(test_common::setup_test_db().await);
    seed_shell(&db).await;
    seed_so(&db, CUS_A, 0, 200, "pending").await;
    seed_so(&db, CUS_A, 200, 50, "approved").await;
    seed_so(&db, CUS_B, 1000, 30, "pending").await;

    let svc = SalesService::new(Arc::clone(&db), Arc::new(ElasticClient::mock()));
    let (headers, rows, server_total) = svc
        .export_orders_to_xlsx(SalesOrderFilter {
            customer_id: Some(CUS_A),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(server_total, 250, "服务端全量总数应为客户甲 250 行");
    assert_eq!(rows.len() as u64, server_total, "交付行数与服务端全量总数不等");
    assert!(
        SalesService::export_rows_complete(rows.len(), server_total),
        "等式成立时完整性判据必须为真"
    );
    assert_headers_byte_equal(&headers, &SO_HEADERS);
    assert!(rows.iter().all(|r| r.len() == 21), "每行必须 21 列");

    let seeded: BTreeSet<String> =
        (0..250_i32).map(|k| format!("EXPLOCK-SO-{}", SO_ID_BASE + k)).collect();
    let delivered: BTreeSet<String> =
        rows.iter().map(|r| cell(&SO_HEADERS, r, "订单编号")).collect();
    assert_eq!(delivered, seeded, "订单号集合必须逐项相等");

    // 导出腿不调明细装配：客户名称必须来自共用条件构造器的左连接且逐行等于播种名
    let names: Vec<String> = rows.iter().map(|r| cell(&SO_HEADERS, r, "客户名称")).collect();
    assert!(
        names.iter().all(|n| n == "导出锁客户甲"),
        "跳过装配后客户名称仍须由共用查询带出，不得变空或变错客户"
    );
}

/// 纯判据四形态（两腿各测，不连库）：相等真、少行假、多行假（并发删除形态）、空结果真
#[test]
fn export_rows_complete_predicate_both_legs() {
    for (returned, total, want) in [
        (250usize, 250u64, true),
        (100, 250, false),
        (251, 250, false),
        (0, 0, true),
    ] {
        assert_eq!(
            PurchaseOrderService::export_rows_complete(returned, total),
            want,
            "采购判据 ({returned},{total}) 应为 {want}"
        );
        assert_eq!(
            SalesService::export_rows_complete(returned, total),
            want,
            "销售判据 ({returned},{total}) 应为 {want}"
        );
    }
}

/// 大行级（采购，一万零一行）：旧封顶形态在此只会回 10_000 行且等式不成立，必红。
/// 不进常规 CI；由 ci-test-rust-ignored 作业 --run-ignored only 执行。
#[tokio::test]
#[ignore = "一万零一行播种与全量导出仅在 ignore 专用通道执行"]
async fn purchase_export_beyond_ten_thousand_keeps_every_row() {
    let db = Arc::new(test_common::setup_test_db().await);
    seed_shell(&db).await;
    let sup_a = supplier_at(&db, 0).await;
    let sup_b = supplier_at(&db, 1).await;
    seed_po(&db, sup_a, 0, 10_001, "DRAFT").await;
    seed_po(&db, sup_b, 20_000, 5, "DRAFT").await;

    let svc = PurchaseOrderService::new(Arc::clone(&db));
    let (headers, rows, server_total) = svc.export_orders_to_xlsx(None, Some(sup_a)).await.unwrap();
    assert_eq!(server_total, 10_001, "服务端总数应为一万零一行");
    assert_eq!(rows.len(), 10_001, "交付行数必须等于服务端总数（旧封顶形态在此判红）");
    let last = format!("EXPLOCK-PO-{}", PO_ID_BASE + 10_000);
    assert!(
        rows.iter().any(|r| cell(&headers, r, "订单编号") == last),
        "末位订单号必须出现在导出里"
    );
}

/// 大行级（销售，一万零一行）：同型判据独立成立
#[tokio::test]
#[ignore = "一万零一行播种与全量导出仅在 ignore 专用通道执行"]
async fn sales_export_beyond_ten_thousand_keeps_every_row() {
    let db = Arc::new(test_common::setup_test_db().await);
    seed_shell(&db).await;
    seed_so(&db, CUS_A, 0, 10_001, "pending").await;
    seed_so(&db, CUS_B, 20_000, 5, "pending").await;

    let svc = SalesService::new(Arc::clone(&db), Arc::new(ElasticClient::mock()));
    let (headers, rows, server_total) = svc
        .export_orders_to_xlsx(SalesOrderFilter {
            customer_id: Some(CUS_A),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(server_total, 10_001);
    assert_eq!(rows.len(), 10_001, "交付行数必须等于服务端总数");
    let last = format!("EXPLOCK-SO-{}", SO_ID_BASE + 10_000);
    assert!(
        rows.iter().any(|r| cell(&headers, r, "订单编号") == last),
        "末位订单号必须出现在导出里"
    );
}
