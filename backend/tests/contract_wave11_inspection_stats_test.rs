//! 契约测试：采购质检统计卡端点（GET /purchase/inspections/stats）真库契约锁
//!
//! ## 锁的对象与通道（为什么直连夹具库 + 生产 service 入口）
//! - 本缺陷根因是"分母用服务端 total（全量）、分子只数当前页行集"导致合格率
//!   随翻页漂移且恒偏高，且 partial 不计入不合格。正解 = 后端聚合，四值全部
//!   与列表**同一条件构造点**（`PurchaseInspectionService::base_filtered_query`）。
//! - 通道与生产 handler 完全同构（`handlers/purchase_inspection_handler.rs` 就是
//!   `PurchaseInspectionService::new(state.db.clone())` 再调本方法），因此不需要
//!   AppState，天然规避 `AppState::default()` 把 service 绑 Disconnected 哨兵的假绿坑
//!   （同 contract_wave11_customer_credit_constraint_test.rs 的判据）。
//! - 全部断言只打 **HTTP 层之下的数值**（统计四值、列表 total、分页条数），
//!   禁止断用户可见文案（脱敏红线：文案随 messages 演进，按文案判契约是假绿来源）。
//!
//! ## 语义前提（与写入侧权威词表同源，判据原文见各常量文档注释）
//! - 分桶取值全部引用 `models::status::purchase_inventory::{purchase_inspection,
//!   purchase_inspection_result}`：pending=PENDING；passed=PASS；
//!   failed=IN (FAIL, PARTIAL)——partial 归不合格侧的既有裁定是
//!   `to_receipt_inspection_status`（fail/partial → REJECTED）。
//! - 当前代码写入规则下建单恒 (PENDING, result=NULL)、完成恒 (COMPLETED,
//!   result∈ALL)，故正常数据上 pending+passed+failed == total（本文件同时钉该
//!   恒等式与"stats 等于手算真值"两条，后者是真值锁、前者只在恒等成立时有意义）。
//!
//! ## 夹具与 id 带
//! - 夹具 `setup_test_db()` 缺 `TEST_DATABASE_URL`/指向 sqlite 直接 panic，
//!   ⇒ 本文件真库用例**只有 CI 活库才有结果**（本机仅证明编译与格式）。
//! - `purchase_inspection` 属逐用例 TRUNCATE 的业务表；`suppliers` 是迁移种子
//!   参照表**不被清空**，故本文件自插两个供应商父行（supplier_code 带纳秒后缀
//!   防唯一键撞车，收尾按捕获 id 自清，不依赖执行顺序）。
//! - 全部断言收在**同一个** `#[tokio::test]` 里顺序执行：夹具 TRUNCATE 与种子行
//!   在同库多测试线程下有相互清表风险，单函数从结构上排除该耦合。

mod test_common;

use bingxi_backend::models::purchase_inspection;
use bingxi_backend::models::status::purchase_inventory::{
    purchase_inspection as pis_status, purchase_inspection_result,
};
use bingxi_backend::models::supplier;
use bingxi_backend::services::purchase_inspection_service::PurchaseInspectionService;
use chrono::{TimeZone, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::sync::Arc;
use test_common::setup_test_db;

fn unique_suffix() -> i64 {
    Utc::now().timestamp_nanos_opt().unwrap()
}

/// 种一个供应商父行（suppliers 不被夹具清空，列形态照
/// contract_wave5_inspection_result_authority_test.rs::seed_receipt_referents
/// 已验证种子逐列复用）
async fn seed_supplier(db: &DatabaseConnection, tag: &str) -> i32 {
    let suffix = unique_suffix();
    let sup = supplier::ActiveModel {
        supplier_code: Set(format!("SUP-{tag}-{suffix}")),
        supplier_name: Set(format!("统计卡契约测试供应商-{tag}-{suffix}")),
        supplier_short_name: Set(format!("统{tag}").chars().take(4).collect()),
        supplier_type: Set("面料供应商".to_string()),
        credit_code: Set(format!("91330000STAT{tag}{suffix:0>6}X")),
        registered_address: Set("契约测试注册地址".to_string()),
        legal_representative: Set("契约法人".to_string()),
        registered_capital: Set(Decimal::ZERO),
        establishment_date: Set(chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
        taxpayer_type: Set("一般纳税人".to_string()),
        bank_name: Set("契约测试银行".to_string()),
        bank_account: Set("6222000000000001".to_string()),
        contact_phone: Set("13800000001".to_string()),
        is_processor: Set(false),
        created_at: Set(Utc::now().fixed_offset()),
        updated_at: Set(Utc::now().fixed_offset()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("夹具：种 suppliers 父行失败");
    sup.id
}

/// 直接按目标形态种一行质检单（行集分布是"已知真值"的前提，故不走状态机写入口；
/// status/result 取值仍逐字符引用权威词表常量，与写入侧同源）
async fn seed_row(
    db: &DatabaseConnection,
    supplier_id: i32,
    status: &str,
    result: Option<&str>,
    ymd: (i32, u32, u32),
) -> purchase_inspection::Model {
    purchase_inspection::ActiveModel {
        inspection_no: Set(format!(
            "PI-W11ST-{supplier_id}-{suffix}",
            suffix = unique_suffix()
        )),
        supplier_id: Set(supplier_id),
        inspection_date: Set(Utc
            .with_ymd_and_hms(ymd.0, ymd.1, ymd.2, 0, 0, 0)
            .unwrap()
            .date_naive()),
        inspection_status: Set(Some(status.to_string())),
        inspection_result: Set(result.map(|s| s.to_string())),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("播种质检单失败")
}

#[tokio::test]
async fn inspection_stats_are_total_truth_and_share_list_filter_pipeline() {
    let db = Arc::new(setup_test_db().await);
    let svc = PurchaseInspectionService::new(db.clone());

    // ---------- 已知分布（手算真值的来源，逐行可数） ----------
    // 供应商 S1（2026-09）：pending×2、pass×3、fail×1、partial×1 ⇒ 共 7 行
    // 供应商 S2（2026-10）：pending×1、pass×1 ⇒ 共 2 行
    // 全量：total=9, pending=3, passed=4, failed=2（fail+partial 各 1）
    //       恒等式 3+4+2=9 成立（正常写入形态）
    let s1 = seed_supplier(&db, "S1").await;
    let s2 = seed_supplier(&db, "S2").await;

    let seeded_ids: Vec<i32> = [
        seed_row(&db, s1, pis_status::PENDING, None, (2026, 9, 10)).await,
        seed_row(&db, s1, pis_status::PENDING, None, (2026, 9, 11)).await,
        seed_row(
            &db,
            s1,
            pis_status::COMPLETED,
            Some(purchase_inspection_result::PASS),
            (2026, 9, 12),
        )
        .await,
        seed_row(
            &db,
            s1,
            pis_status::COMPLETED,
            Some(purchase_inspection_result::PASS),
            (2026, 9, 13),
        )
        .await,
        seed_row(
            &db,
            s1,
            pis_status::COMPLETED,
            Some(purchase_inspection_result::PASS),
            (2026, 9, 14),
        )
        .await,
        seed_row(
            &db,
            s1,
            pis_status::COMPLETED,
            Some(purchase_inspection_result::FAIL),
            (2026, 9, 15),
        )
        .await,
        seed_row(
            &db,
            s1,
            pis_status::COMPLETED,
            Some(purchase_inspection_result::PARTIAL),
            (2026, 9, 16),
        )
        .await,
        seed_row(&db, s2, pis_status::PENDING, None, (2026, 10, 5)).await,
        seed_row(
            &db,
            s2,
            pis_status::COMPLETED,
            Some(purchase_inspection_result::PASS),
            (2026, 10, 6),
        )
        .await,
    ]
    .into_iter()
    .map(|m| m.id)
    .collect();
    assert_eq!(seeded_ids.len(), 9, "夹具必须恰好种 9 行（手算真值前提）");

    // ---------- 1. 无筛选：四值 = 手算真值（数值逐键断，不是 >0） ----------
    let stats = svc
        .inspection_stats(None, None, None, None, None, None)
        .await
        .expect("无筛选统计必须成功");
    assert_eq!(stats.total, 9, "total 必须是全量行数 9");
    assert_eq!(stats.pending, 3, "pending 必须是 status=PENDING 的 3 行");
    assert_eq!(
        stats.passed, 4,
        "passed 必须是 result=PASS 的 4 行（只含 pass）"
    );
    assert_eq!(
        stats.failed, 2,
        "failed 必须是 result∈{{fail,partial}} 的 2 行"
    );
    // partial 分桶归属正向锁：failed=2 恰由 1 fail + 1 partial 组成（若 partial
    // 被漏计则 failed=1，若被塞进 passed 则 passed=5，两头都当场红）。
    assert_eq!(
        stats.pending + stats.passed + stats.failed,
        stats.total,
        "正常写入形态下恒等式 pending+passed+failed == total 必须成立"
    );

    // ---------- 2. 翻页不改统计（直接锁死原缺陷复发） ----------
    let p1 = svc
        .list_inspections(1, 2, None, None, None, None, None, None)
        .await
        .expect("第 1 页列表必须成功");
    assert_eq!(p1.0.len(), 2, "page_size=2 时每页行集恰 2 行");
    assert_eq!(p1.1, 9, "列表 total 是全量 9，不随页缩小");
    let p5 = svc
        .list_inspections(5, 2, None, None, None, None, None, None)
        .await
        .expect("末页列表必须成功");
    assert_eq!(p5.0.len(), 1, "9 行按 2/页翻到第 5 页只剩 1 行");
    assert_eq!(p5.1, 9, "末页 total 仍必须是 9");
    let stats_after_page1 = svc
        .inspection_stats(None, None, None, None, None, None)
        .await
        .expect("翻页后统计必须成功");
    assert_eq!(stats_after_page1.total, 9);
    assert_eq!(stats_after_page1.pending, 3);
    assert_eq!(stats_after_page1.passed, 4);
    assert_eq!(stats_after_page1.failed, 2);
    // 原缺陷形态的反向锁：分母/分子若都退化成"页内行集"，四值会随页漂移；
    // 上面两条"不同页码 + 同一组四值"已把它钉死。

    // ---------- 3. 筛选跟随：stats 与同条件列表行数一致 ----------
    let stats_s1 = svc
        .inspection_stats(None, Some(s1), None, None, None, None)
        .await
        .expect("供应商筛选统计必须成功");
    assert_eq!(stats_s1.total, 7, "S1 全量 7 行");
    assert_eq!(stats_s1.pending, 2, "S1 待检 2 行");
    assert_eq!(stats_s1.passed, 3, "S1 合格 3 行");
    assert_eq!(stats_s1.failed, 2, "S1 不合格 2 行（fail+partial）");
    let list_s1 = svc
        .list_inspections(1, 100, None, Some(s1), None, None, None, None)
        .await
        .expect("同条件列表必须成功");
    assert_eq!(
        list_s1.1, stats_s1.total,
        "stats.total 与同筛选条件列表 total 必须逐键相等（同源性判据）"
    );
    // 日期筛选同样跟随（2026-10 区间只剩 S2 的 2 行）
    let stats_oct = svc
        .inspection_stats(
            None,
            None,
            None,
            None,
            Some("2026-10-01".to_string()),
            Some("2026-10-31".to_string()),
        )
        .await
        .expect("日期区间统计必须成功");
    assert_eq!(stats_oct.total, 2, "10 月区间只有 S2 的 2 行");
    assert_eq!(stats_oct.pending, 1);
    assert_eq!(stats_oct.passed, 1);
    assert_eq!(stats_oct.failed, 0);

    // ---------- 4. partial 归位单行核（result 筛选 + 分桶交叉） ----------
    let stats_partial = svc
        .inspection_stats(
            None,
            None,
            None,
            Some(purchase_inspection_result::PARTIAL.to_string()),
            None,
            None,
        )
        .await
        .expect("partial 结果筛选统计必须成功");
    assert_eq!(
        stats_partial.total, 1,
        "词表 token 等值筛选恰命中 1 行 partial"
    );
    assert_eq!(stats_partial.passed, 0, "partial 绝不计入合格侧");
    assert_eq!(stats_partial.failed, 1, "partial 必须计入不合格侧");
    // fail 各自归位（防"全塞一侧"）
    let stats_fail = svc
        .inspection_stats(
            None,
            None,
            None,
            Some(purchase_inspection_result::FAIL.to_string()),
            None,
            None,
        )
        .await
        .expect("fail 结果筛选统计必须成功");
    assert_eq!(stats_fail.total, 1);
    assert_eq!(stats_fail.passed, 0);
    assert_eq!(stats_fail.failed, 1);

    // ---------- 收尾自清：只删本文件种的行，不依赖执行顺序 ----------
    // purchase_inspection 属逐用例 TRUNCATE 的业务表，但 suppliers 是**不被清空**的
    // 迁移种子参照表：先删本文件质检行（FK 子端）再删自种的两个供应商父行。
    for sid in [s1, s2] {
        purchase_inspection::Entity::delete_many()
            .filter(purchase_inspection::Column::SupplierId.eq(sid))
            .exec(db.as_ref())
            .await
            .expect("夹具：清理本文件自种的质检行失败");
        supplier::Entity::delete_by_id(sid)
            .exec(db.as_ref())
            .await
            .expect("夹具：清理本文件自种的 suppliers 父行失败");
    }
}
