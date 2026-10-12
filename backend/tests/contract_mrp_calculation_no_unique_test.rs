//! （flow shard37，rs37/backend.log:16642-16645）MRP 计算 500 的契约锁。
//!
//! 缺陷场景：同一订单连打两次 MRP 批量计算，第二次 500——`{批次基数}-0` 行号
//! 撞 `mrp_results` 表 `calculation_no` 的 UNIQUE 键（mrp_results_calculation_no_key）。
//!
//! 根因（`utils/number_generator.rs::allocate_no` 旧基数判据）：MRPB 批次的
//! 裸基数本身不落库，落库的是派生行号 `{基数}-{行序}`（及 BOM 子行
//! `{基数}-{行序}-{子行}`）；这类带 `-` 后缀的行曾被判为“旁路、不参与序号基数”，
//! 基数看不见既有号段 ⇒ 重发同一基数 ⇒ 派生行直撞 UNIQUE。
//!
//! 本文件对**真实 PostgreSQL**（`setup_test_db`：已迁移库、每用例清空业务表）
//! 真实造数、不 mock、不断错误文案、无 `#[ignore]`，钉住三层行为：
//! 1. `batch_calculate` 连算两次产生两个不同批次号，且各批 `{批次号}-0` 行
//!    真实落库（正是 CI 中第二次 500 的场景）；
//! 2. `run_mrp_calculation` 连算两次产生两个不同行号，均落库、表内无重号；
//! 3. 基数行为锁：仅派生行 `{基数}-{行序}` 落库（裸基数不在表中）时，
//!    下一次取号必须以“派生行的基数流水 + 1”起号，禁止重发已被派生行占用的基数；
//!    同时含字母的旁路后缀仍不参与基数（判据不被顺手放宽）。

use bingxi_backend::models::mrp_result::{
    ActiveModel as MrpResultActiveModel, Column as MrpResultColumn, Entity as MrpResultEntity,
};
use bingxi_backend::services::mrp_engine_ops::types::{
    MrpCalculationItem, MrpCalculationQuery, MrpCalculationRequest,
};
use bingxi_backend::services::mrp_engine_service::MrpEngineService;
use bingxi_backend::services::test_common::setup_test_db;
use bingxi_backend::utils::number_generator::DocumentNumberGenerator;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::sync::Arc;

fn today() -> String {
    Utc::now().format("%Y%m%d").to_string()
}

fn fixture_date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 12, 1).expect("夹具日期 2026-12-01 必须可解析")
}

/// 直接落一条 mrp_results 行（仅用于造“存量派生行/旁路行”前提；
/// 必填列对齐 migration/src/domain/business/m0007_add_mrp_production_bom.rs:103-117，
/// status 走 DB 默认 'PLANNED'）
async fn seed_mrp_row(db: &DatabaseConnection, no: &str) {
    let now = Utc::now();
    MrpResultActiveModel {
        calculation_no: Set(no.to_string()),
        product_id: Set(1),
        required_quantity: Set(Decimal::from(10_i64)),
        source_type: Set("SALES_ORDER".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("夹具：落库 mrp_results 行 {no} 失败: {e}"));
}

async fn exists(db: &DatabaseConnection, no: &str) -> bool {
    MrpResultEntity::find()
        .filter(MrpResultColumn::CalculationNo.eq(no))
        .one(db)
        .await
        .unwrap_or_else(|e| panic!("夹具：查询 {no} 失败: {e}"))
        .is_some()
}

/// 表内全部 calculation_no（用于全表无重号断言）
async fn all_calculation_nos(db: &DatabaseConnection) -> Vec<String> {
    MrpResultEntity::find()
        .all(db)
        .await
        .expect("夹具：全表查询 mrp_results 失败")
        .into_iter()
        .map(|m| m.calculation_no)
        .collect()
}

fn batch_request() -> MrpCalculationRequest {
    MrpCalculationRequest {
        items: vec![MrpCalculationItem {
            product_id: 1,
            required_quantity: Decimal::from(10_i64),
            required_date: fixture_date(),
        }],
        source_type: "SALES_ORDER".to_string(),
        source_id: None,
        consider_safety_stock: false,
        consider_in_transit: false,
    }
}

/// CI 场景本体：同一订单（同参数）连算两次批量 MRP。
/// 第二次不得 500（旧实现重发同一基数导致 `{基数}-0` 撞 UNIQUE），
/// 两批必须拿到不同批次号，且各批首行 `{批次号}-0` 真实落库、表内无重号。
#[tokio::test]
async fn batch_calculate_twice_yields_distinct_calculation_no_both_persisted() {
    let db = setup_test_db().await;
    let svc = MrpEngineService::new(Arc::new(db.clone()));

    let first = svc
        .batch_calculate(batch_request())
        .await
        .expect("第一次批量计算必须成功");
    let second = svc
        .batch_calculate(batch_request())
        .await
        .expect("同一订单第二次批量计算必须成功（历史上该调用曾返回 500）");

    assert_ne!(
        first.calculation_no, second.calculation_no,
        "两次批量计算的批次号必须不同（基数必须看见上一批的派生行号）"
    );
    for no in [&first.calculation_no, &second.calculation_no] {
        assert!(
            no.starts_with(&format!("MRPB{}", today())),
            "批次号必须走统一取号器格式 MRPB{{YYYYMMDD}}{{流水}}，实际 {no}"
        );
        assert!(
            exists(&db, &format!("{no}-0")).await,
            "批次 {no} 的首行 `{no}-0` 必须真实落库"
        );
    }

    let all = all_calculation_nos(&db).await;
    let mut dedup = all.clone();
    dedup.sort();
    dedup.dedup();
    assert_eq!(
        dedup.len(),
        all.len(),
        "mrp_results.calculation_no（UNIQUE 列）出现重号，取号器基数判据回潮"
    );
    // 无 BOM 夹具下每批恰 1 行（主行），两批共 2 行
    assert_eq!(
        all.len(),
        first.results.len() + second.results.len(),
        "两批结果行必须全部落库"
    );
}

/// 单次计算入口同锁：连算两次拿到不同行号、都落库（主行为裸单号，
/// 旧基数虽可见，但重试/映射改造后仍不得引入重号或失败）。
#[tokio::test]
async fn run_mrp_calculation_twice_yields_distinct_line_no_both_persisted() {
    let db = setup_test_db().await;
    let svc = MrpEngineService::new(Arc::new(db.clone()));

    let query = MrpCalculationQuery {
        product_id: 1,
        required_quantity: Decimal::from(5_i64),
        required_date: fixture_date(),
        source_type: "SALES_ORDER".to_string(),
        source_id: None,
        consider_safety_stock: false,
        consider_in_transit: false,
    };
    let r1 = svc
        .run_mrp_calculation(query.clone())
        .await
        .expect("第一次单次 MRP 计算必须成功");
    let r2 = svc
        .run_mrp_calculation(query)
        .await
        .expect("第二次单次 MRP 计算必须成功");

    let no1 = &r1.first().expect("主行必须存在").calculation_no;
    let no2 = &r2.first().expect("主行必须存在").calculation_no;
    assert_ne!(no1, no2, "两次单次计算的行号必须不同");
    assert!(exists(&db, no1).await, "第一次主行 {no1} 必须落库");
    assert!(exists(&db, no2).await, "第二次主行 {no2} 必须落库");

    let all = all_calculation_nos(&db).await;
    let mut dedup = all.clone();
    dedup.sort();
    dedup.dedup();
    assert_eq!(dedup.len(), all.len(), "mrp_results 表内不得出现重号");
}

/// 基数判据行为锁：模拟一次已提交批次的落库形态——**只有**派生行
/// `{基数}-{行序}` 在表内（MRPB 裸基数从不落库），下一次取号必须以
/// 派生行的基数流水 + 1 起号；若判据回潮为“带后缀行不参与基数”，
/// 将重发已被占用的基数 010，`{010}-0` 落库即撞 UNIQUE（CI 500 本体）。
/// 同段再造一条含字母的旁路行 `07X-1`：它**仍不得**参与基数（判据不被
/// 顺手放宽成“截取任意前导数字”）。
#[tokio::test]
async fn allocate_base_includes_derived_line_rows_but_not_letter_bypass() {
    let db = setup_test_db().await;
    let day = today();

    seed_mrp_row(&db, &format!("MRPB{day}010-0")).await;
    seed_mrp_row(&db, &format!("MRPB{day}07X-1")).await;

    let no = DocumentNumberGenerator::generate_no(
        &db,
        "MRPB",
        MrpResultEntity,
        MrpResultColumn::CalculationNo,
    )
    .await
    .expect("仅派生行落库的号段下取号必须成功，且必须避开已占用基数");

    assert_eq!(
        no,
        format!("MRPB{day}011"),
        "带 `-行序` 后缀的派生行必须参与取号基数（010-0 ⇒ 基数 010 ⇒ 下一个 011）；\
         得到 010 说明排除判据回潮，得到其它值说明旁路行被误计入基数"
    );

    // 取号结果真实落库不撞 23505（探测给出的号必须可用）
    seed_mrp_row(&db, &no).await;
    assert!(exists(&db, &no).await, "取号器返回的号必须可落库");
}
