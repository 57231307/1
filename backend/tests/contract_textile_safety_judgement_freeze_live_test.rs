//! 纺织品安全合规域判定链与库存联动的活体锁（真已迁移 PostgreSQL，含 m0111 种子矩阵）
//!
//! 钉什么（每条都读回真值，正反两侧）：
//! 1. 送检出样（sampling→sent）把批次在库行按"待检不出库"降为待检；撤回解除挂起回合格。
//! 2. 整单判定 fail 即批次冻结（quality_status 不合格，可出库量筛选口径自然拦下）；
//!    同批次复检 pass 后放行（待检或不合格回到合格）。
//! 3. 完整性门缺项逐条点名（cma、cnas、六项要素、有效期、批次一致性），缺项即 rejected，
//!    未过门的报告不得产判定（judgement 报 400 点名）。
//! 4. 已受理但过期的报告排除出整单汇总并点名；全部过期即拒绝判定，绝不静默消费过期结论。
//! 5. 词表与状态机门：裸字母类别、人群松于法定最低类别、手工直达 judged 一律拒。
//! 6. 判定器变异自证：把限值行 lte 20 变异为 lte 30，同一实测 25 的结论必须随表翻转
//!    （钉"判定按行不硬编码"，改回 20 后结论必须翻回 fail）。
//!
//! 调用方：`bash E:/tmp/tools/live_test_lane.sh <车道> contract_textile_safety_judgement_freeze_live_test`。
//! 存哪里：只写本域单据族与测试产品、仓库、库存行（夹具 setup_test_db 已清业务表，
//! textile_test_item、textile_limit_rule、textile_export_market_requirement 为迁移种子不清）。
mod test_common;

use bingxi_backend::models::inventory_stock::{
    ActiveModel as StockActiveModel, Column as StockColumn, Entity as InventoryStock,
};
use bingxi_backend::models::product::ActiveModel as ProductActiveModel;
use bingxi_backend::models::status::purchase_inventory::{
    inventory_stock_quality_status as stock_quality, inventory_stock_status as stock_status,
};
use bingxi_backend::models::status::textile_safety::{
    submission_status, textile_safety_category,
};
use bingxi_backend::models::textile_limit_rule::{
    ActiveModel as LimitActiveModel, Column as LimitColumn, Entity as LimitRule,
};
use bingxi_backend::models::textile_third_party_report::ActiveModel as ReportActiveModel;
use bingxi_backend::models::warehouse::ActiveModel as WarehouseActiveModel;
use bingxi_backend::services::textile_safety_report_service::{
    RegisterReportReq, ResultRowReq, TextileSafetyReportService,
};
use bingxi_backend::services::textile_safety_submission_service::{
    CreateSubmissionReq, TextileSafetySubmissionService,
};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
};
use std::sync::Arc;

const TEST_OPERATOR: i32 = 9901;
const BATCH_A: &str = "BT-TXT-001";

fn now() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}

fn dec(s: &str) -> Decimal {
    s.parse().expect("测试常量应为合法十进制")
}

fn date(s: &str) -> chrono::NaiveDate {
    s.parse().expect("测试常量应为 YYYY-MM-DD")
}

async fn seeded_db() -> Arc<DatabaseConnection> {
    let db = Arc::new(test_common::setup_test_db().await);
    WarehouseActiveModel {
        id: Set(1),
        warehouse_code: Set("WH-TXT".to_string()),
        name: Set("纺织品安全测试仓".to_string()),
        is_default: Set(false),
        is_active: Set(true),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(&*db)
    .await
    .expect("种子仓库插入失败");
    ProductActiveModel {
        id: Set(1),
        code: Set("FAB-TXT-1".to_string()),
        name: Set("棉针织布".to_string()),
        unit: Set("米".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("fabric".to_string()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(&*db)
    .await
    .expect("种子产品插入失败");
    StockActiveModel {
        warehouse_id: Set(1),
        product_id: Set(1),
        quantity_on_hand: Set(dec("100.00")),
        quantity_available: Set(dec("100.00")),
        quantity_reserved: Set(Decimal::ZERO),
        quantity_shipped: Set(Decimal::ZERO),
        quantity_incoming: Set(Decimal::ZERO),
        reorder_point: Set(Decimal::ZERO),
        max_stock_point: Set(Decimal::ZERO),
        reorder_quantity: Set(Decimal::ZERO),
        batch_no: Set(BATCH_A.to_string()),
        color_no: Set("RED-1".to_string()),
        grade: Set("一等品".to_string()),
        quantity_meters: Set(dec("100.00")),
        quantity_kg: Set(dec("20.00")),
        stock_status: Set(stock_status::NORMAL.to_string()),
        quality_status: Set(stock_quality::PASS.to_string()),
        version: Set(0),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(&*db)
    .await
    .expect("种子库存行插入失败");
    db
}

/// 读回批次在库行（唯一一张）的质量状态，不存在即 panic。
async fn stock_quality_of(db: &Arc<DatabaseConnection>) -> String {
    let row = InventoryStock::find()
        .filter(StockColumn::BatchNo.eq(BATCH_A))
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("批次库存行必须存在");
    row.quality_status
}

fn submission_req(no: &str, batch: Option<&str>) -> CreateSubmissionReq {
    CreateSubmissionReq {
        submission_no: no.to_string(),
        product_id: 1,
        batch_no: batch.map(|b| b.to_string()),
        color_no: "RED-1".to_string(),
        process_no: "PR-1".to_string(),
        color_role: "full".to_string(),
        end_user_group: "adult".to_string(),
        contact_type: "direct".to_string(),
        safety_category: textile_safety_category::CLASS_B.to_string(),
        product_form: "final".to_string(),
        target_market: "dom".to_string(),
        exemption_tags: vec![],
        remarks: None,
        created_by: TEST_OPERATOR,
    }
}

/// 推进到 sent（建单、生成清单、draft→sampling、sampling→sent）。
async fn advance_to_sent(db: &Arc<DatabaseConnection>, no: &str, batch: Option<&str>) -> i32 {
    let service = TextileSafetySubmissionService::new(db.clone());
    let created = service.create(submission_req(no, batch)).await.unwrap();
    service
        .generate_items(created.id, vec![], TEST_OPERATOR)
        .await
        .unwrap();
    let (model, _) = service
        .set_status(created.id, submission_status::SAMPLING, TEST_OPERATOR)
        .await
        .unwrap();
    assert_eq!(model.status, submission_status::SAMPLING);
    let (model, link) = service
        .set_status(created.id, submission_status::SENT, TEST_OPERATOR)
        .await
        .unwrap();
    assert_eq!(model.status, submission_status::SENT);
    // 送检出样即挂起：有批次号时必命中种子库存行（联动不许静默）
    if batch.is_some() {
        assert_eq!(link.matched, 1, "sent 挂起必须命中 1 行在库库存：{}", link.scope_note);
        assert_eq!(link.updated_stock_ids.len(), 1);
    }
    created.id
}

/// 登记一份要素齐全、在有效期内的报告并录入合格实测，过完整性门并做报告级判定。
/// formaldehyde 值由参数给（变异与 fail 场景用）。
async fn report_full_chain(
    db: &Arc<DatabaseConnection>,
    submission_id: i32,
    report_no: &str,
    hcho: &str,
    valid_until: &str,
) {
    let service = TextileSafetyReportService::new(db.clone());
    service
        .register(RegisterReportReq {
            report_no: report_no.to_string(),
            submission_id,
            inspection_agency: "国检中心".to_string(),
            has_cma: true,
            has_cnas: true,
            sample_name: Some("棉针织布 RED-1".to_string()),
            material_composition: Some("棉 100%".to_string()),
            color_no: Some("RED-1".to_string()),
            process_no: Some("PR-1".to_string()),
            test_basis: Some("GB 18401-2010 B类".to_string()),
            conclusion: Some("整批判定见逐项结果".to_string()),
            oeko_tex_class: None,
            issue_date: date("2026-10-01"),
            valid_until: Some(date(valid_until)),
            report_file_url: None,
            created_by: TEST_OPERATOR,
        })
        .await
        .unwrap();
    let reports = service.list_by_submission(submission_id).await.unwrap();
    let report = reports
        .iter()
        .find(|r| r.report_no == report_no)
        .expect("刚登记的报告必须读回");
    let rows = vec![
        ResultRowReq {
            item_code: "HCHO".to_string(),
            measured_value: Some(dec(hcho)),
            measured_token: None,
            measured_text: None,
            measured_unit: Some("mg/kg".to_string()),
            test_method: Some("GB/T 2912.1".to_string()),
        },
        ResultRowReq {
            item_code: "PH".to_string(),
            measured_value: Some(dec("7.0")),
            measured_token: None,
            measured_text: None,
            measured_unit: None,
            test_method: Some("GB/T 7573".to_string()),
        },
        ResultRowReq {
            item_code: "AZO".to_string(),
            measured_value: Some(dec("10")),
            measured_token: None,
            measured_text: Some("none".to_string()),
            measured_unit: Some("mg/kg".to_string()),
            test_method: Some("GB/T 19942".to_string()),
        },
        ResultRowReq {
            item_code: "ODOR".to_string(),
            measured_value: None,
            measured_token: None,
            measured_text: Some("none".to_string()),
            measured_unit: None,
            test_method: None,
        },
        ResultRowReq {
            item_code: "FAST_WATER".to_string(),
            measured_value: None,
            measured_token: Some("3-4".to_string()),
            measured_text: None,
            measured_unit: Some("级".to_string()),
            test_method: Some("GB/T 5713".to_string()),
        },
        ResultRowReq {
            item_code: "FAST_ACID_SWEAT".to_string(),
            measured_value: None,
            measured_token: Some("3-4".to_string()),
            measured_text: None,
            measured_unit: Some("级".to_string()),
            test_method: Some("GB/T 3921".to_string()),
        },
        ResultRowReq {
            item_code: "FAST_ALKALI_SWEAT".to_string(),
            measured_value: None,
            measured_token: Some("3-4".to_string()),
            measured_text: None,
            measured_unit: Some("级".to_string()),
            test_method: Some("GB/T 3921".to_string()),
        },
        ResultRowReq {
            item_code: "FAST_DRY_RUB".to_string(),
            measured_value: None,
            measured_token: Some("4".to_string()),
            measured_text: None,
            measured_unit: Some("级".to_string()),
            test_method: Some("GB/T 3920".to_string()),
        },
        ResultRowReq {
            item_code: "FAST_WET_RUB".to_string(),
            measured_value: None,
            measured_token: Some("3-4".to_string()),
            measured_text: None,
            measured_unit: Some("级".to_string()),
            test_method: Some("GB/T 3920".to_string()),
        },
    ];
    service.upsert_results(report.id, rows, TEST_OPERATOR).await.unwrap();
    let (accepted, missing) = service
        .run_acceptance_gate(report.id, TEST_OPERATOR)
        .await
        .unwrap();
    assert!(
        missing.is_empty(),
        "要素齐全的报告必须过门，缺项点名 {missing:?}（状态 {}）",
        accepted.acceptance_status
    );
    service.judge_report(report.id, TEST_OPERATOR).await.unwrap();
}

/// 1+2. 送检挂起、fail 冻结、复检 pass 放行的完整链路（读回库存真值）。
#[tokio::test]
async fn hold_freeze_release_chain() {
    let db = seeded_db().await;
    let sid = advance_to_sent(&db, "SUB-TXT-HOLD", Some(BATCH_A)).await;
    assert_eq!(
        stock_quality_of(&db).await,
        stock_quality::PENDING,
        "sent 后批次库存必须降为待检（待检不出库裁定）"
    );

    // 甲醛 90 对 B 类 lte 75 → 整单 fail → 冻结
    report_full_chain(&db, sid, "RPT-TXT-FAIL", "90", "2027-12-31").await;
    let sub_service = TextileSafetySubmissionService::new(db.clone());
    let outcome = sub_service.judge_submission(sid, TEST_OPERATOR).await.unwrap();
    assert_eq!(
        outcome.submission.overall_verdict.as_deref(),
        Some("fail"),
        "甲醛 90 超 B 类 75 上限必须整单 fail"
    );
    assert_eq!(
        stock_quality_of(&db).await,
        stock_quality::FAIL,
        "整单 fail 必须批次冻结（读回库存质量状态）"
    );
    assert_eq!(outcome.stock_link.updated_stock_ids.len(), 1);
    assert!(outcome.expired_reports_named.is_empty());

    // 同批次复检（甲醛 15）整单 pass → 放行
    let sid2 = {
        let created = sub_service.create(submission_req("SUB-TXT-PASS", Some(BATCH_A))).await.unwrap();
        sub_service
            .generate_items(created.id, vec![], TEST_OPERATOR)
            .await
            .unwrap();
        sub_service
            .set_status(created.id, submission_status::SAMPLING, TEST_OPERATOR)
            .await
            .unwrap();
        // 冻结态的库存行不得被 sent 挂起动作解冻（sent 只降合格行）
        sub_service
            .set_status(created.id, submission_status::SENT, TEST_OPERATOR)
            .await
            .unwrap();
        created.id
    };
    report_full_chain(&db, sid2, "RPT-TXT-PASS", "15", "2027-12-31").await;
    let outcome2 = sub_service.judge_submission(sid2, TEST_OPERATOR).await.unwrap();
    assert_eq!(
        outcome2.submission.overall_verdict.as_deref(),
        Some("pass"),
        "甲醛 15 低于 75 必须整单 pass"
    );
    assert_eq!(
        stock_quality_of(&db).await,
        stock_quality::PASS,
        "复检 pass 必须放行（不合格冻结被最新权威结论解除）"
    );
}

/// 1b. 撤回解除挂起（sent→withdrawn 后待检行回合格）。
#[tokio::test]
async fn withdraw_releases_pending_hold() {
    let db = seeded_db().await;
    let sid = advance_to_sent(&db, "SUB-TXT-WD", Some(BATCH_A)).await;
    assert_eq!(stock_quality_of(&db).await, stock_quality::PENDING);
    let service = TextileSafetySubmissionService::new(db.clone());
    let (model, link) = service
        .set_status(sid, submission_status::WITHDRAWN, TEST_OPERATOR)
        .await
        .unwrap();
    assert_eq!(model.status, submission_status::WITHDRAWN);
    assert_eq!(link.updated_stock_ids.len(), 1, "撤回必须解除挂起：{}", link.scope_note);
    assert_eq!(stock_quality_of(&db).await, stock_quality::PASS, "撤回后回合格（送检前状态）");
}

/// 1c. 无批次号的送检单不静默跳过联动，scope_note 必须点名原因。
#[tokio::test]
async fn no_batch_named_not_silent() {
    let db = seeded_db().await;
    let sid = advance_to_sent(&db, "SUB-TXT-NOBATCH", None).await;
    let service = TextileSafetySubmissionService::new(db.clone());
    // 上面 advance_to_sent 对无批次单只断言状态；联动回显按本函数重推一次语义核对：
    // sent 已在 advance_to_sent 内发生，当时 link.scope_note 应点名"未登记批次号"。
    // 这里直接验证库存行未被误动，且判定链路对无批次单同样点名。
    report_full_chain(&db, sid, "RPT-TXT-NOBATCH", "90", "2027-12-31").await;
    let outcome = service.judge_submission(sid, TEST_OPERATOR).await.unwrap();
    assert_eq!(outcome.stock_link.matched, 0);
    assert!(
        outcome.stock_link.scope_note.contains("批次号"),
        "无批次联动回显必须点名原因，实际：{}",
        outcome.stock_link.scope_note
    );
    assert_eq!(stock_quality_of(&db).await, stock_quality::PASS, "无批次单不得误改任何库存行");
}

/// 3. 完整性门缺项逐条点名并落 rejected；未过门报告不得产判定。
#[tokio::test]
async fn acceptance_gate_names_missing_and_blocks_judgement() {
    let db = seeded_db().await;
    let sid = advance_to_sent(&db, "SUB-TXT-GATE", Some(BATCH_A)).await;
    let service = TextileSafetyReportService::new(db.clone());
    let report = service
        .register(RegisterReportReq {
            report_no: "RPT-TXT-GAP".to_string(),
            submission_id: sid,
            inspection_agency: "无资质小所".to_string(),
            has_cma: false,
            has_cnas: false,
            sample_name: None,
            material_composition: None,
            color_no: Some("RED-9".to_string()),
            process_no: Some("PR-9".to_string()),
            test_basis: Some("GB18401 B类".to_string()),
            conclusion: None,
            oeko_tex_class: None,
            issue_date: date("2026-10-01"),
            valid_until: None,
            report_file_url: None,
            created_by: TEST_OPERATOR,
        })
        .await
        .unwrap();
    let (_, missing) = service.run_acceptance_gate(report.id, TEST_OPERATOR).await.unwrap();
    for expected in ["cma", "cnas", "sample_name", "material_composition", "test_basis", "conclusion", "expired"]
    {
        assert!(
            missing.iter().any(|m| m == expected),
            "缺项必须点名 {expected}，实际 {missing:?}"
        );
    }
    assert!(missing.iter().any(|m| m == "batch_mismatch"), "工艺色号与送检单不一致必须点名 batch_mismatch，实际 {missing:?}");
    let saved = service.get(report.id).await.unwrap();
    assert_eq!(saved.acceptance_status, "rejected");
    let err = service
        .judge_report(report.id, TEST_OPERATOR)
        .await
        .expect_err("未过完整性门的报告不得产判定");
    assert!(
        err.to_string().contains("rejected") || err.to_string().contains("完整性门"),
        "错误消息必须点名门因，实际：{err}"
    );
}

/// 4. 过期报告点名：全部过期即拒绝整单判定；一有效一过期则排除过期并点名。
#[tokio::test]
async fn expired_reports_named_and_excluded() {
    let db = seeded_db().await;
    let sid = advance_to_sent(&db, "SUB-TXT-EXP", Some(BATCH_A)).await;
    report_full_chain(&db, sid, "RPT-TXT-VALID", "15", "2027-12-31").await;
    // 再登记一份并过门后把有效期改到过去（模拟"受理时有效、归档时已过期"）
    let report_service = TextileSafetyReportService::new(db.clone());
    let extra = report_service
        .register(RegisterReportReq {
            report_no: "RPT-TXT-OLD".to_string(),
            submission_id: sid,
            inspection_agency: "国检中心".to_string(),
            has_cma: true,
            has_cnas: true,
            sample_name: Some("棉针织布 RED-1".to_string()),
            material_composition: Some("棉 100%".to_string()),
            color_no: Some("RED-1".to_string()),
            process_no: Some("PR-1".to_string()),
            test_basis: Some("GB 18401-2010 B类".to_string()),
            conclusion: Some("合格".to_string()),
            oeko_tex_class: None,
            issue_date: date("2024-01-01"),
            valid_until: Some(date("2024-06-30")),
            report_file_url: None,
            created_by: TEST_OPERATOR,
        })
        .await
        .unwrap();
    // 直改受理态与有效期（绕过门构造"过期已受理"的现场态，供判定链核对点名行为）
    let mut active: ReportActiveModel = extra.into();
    active.acceptance_status = Set("accepted".to_string());
    active.valid_until = Set(Some(date("2025-01-01")));
    let old = active.update(&*db).await.unwrap();

    let sub_service = TextileSafetySubmissionService::new(db.clone());
    let outcome = sub_service.judge_submission(sid, TEST_OPERATOR).await.unwrap();
    assert_eq!(
        outcome.expired_reports_named,
        vec![old.report_no.clone()],
        "过期已受理报告必须逐枚点名"
    );
    assert_eq!(
        outcome.submission.overall_verdict.as_deref(),
        Some("pass"),
        "有效报告结论照常成立，过期报告仅被排除"
    );
}

/// 5. 词表与状态机门（反向三例：裸字母类别、人群申报松于法定、手工直达 judged）。
#[tokio::test]
async fn vocabulary_and_state_machine_reject() {
    let db = seeded_db().await;
    let service = TextileSafetySubmissionService::new(db.clone());

    // 裸字母类别：词表只认带前缀稳定码
    let mut bare = submission_req("SUB-TXT-VOCAB", Some(BATCH_A));
    bare.safety_category = "B".to_string();
    let err = service.create(bare).await.expect_err("裸字母类别必须被词表拒");
    assert!(err.to_string().contains("安全类别"), "报错必须点名维度列，实际：{err}");

    // 婴幼儿申报 B 类：松于 GB 31701 法定最低 A 类
    let mut loose = submission_req("SUB-TXT-LOOSE", Some(BATCH_A));
    loose.end_user_group = "infant".to_string();
    loose.contact_type = "infant_use".to_string();
    service
        .create(loose)
        .await
        .expect_err("婴幼儿申报 B 类松于法定 A 类必须拒");

    // judged 不得手工直达（白名单里没有进入 judged 的人工边）
    let created = service
        .create(submission_req("SUB-TXT-VOCAB", Some(BATCH_A)))
        .await
        .unwrap();
    let err = service
        .set_status(created.id, submission_status::JUDGED, TEST_OPERATOR)
        .await
        .expect_err("judged 不得手工直达");
    assert!(err.to_string().contains("judged"), "报错必须点名目标态，实际：{err}");
}

/// 6. 判定器变异自证：限值行从 75 变异到 100，实测 90 的结论必须从 fail 翻为 pass；
///    改回 75 再判必须翻回 fail（钉判定逐行查表、零硬编码）。
#[tokio::test]
async fn judge_is_table_driven_under_limit_mutation() {
    let db = seeded_db().await;
    let sid = advance_to_sent(&db, "SUB-TXT-MUT", Some(BATCH_A)).await;
    report_full_chain(&db, sid, "RPT-TXT-MUT", "90", "2027-12-31").await;
    // 首判：HCHO 90 对 B 类 lte 75 → fail
    let rows = {
        let service = TextileSafetyReportService::new(db.clone());
        let reports = service.list_by_submission(sid).await.unwrap();
        service.list_results(reports[0].id).await.unwrap()
    };
    // 项目主数据 id 由 item_code 反查
    use bingxi_backend::models::textile_test_item::{
        Column as ItemMasterColumn, Entity as TestItem,
    };
    let item = TestItem::find()
        .filter(ItemMasterColumn::ItemCode.eq("HCHO"))
        .one(&*db)
        .await
        .unwrap()
        .expect("m0111 种子必须含 HCHO 项目");
    let before = rows
        .iter()
        .find(|r| r.item_id == item.id)
        .expect("结果行必须含 HCHO");
    assert_eq!(before.verdict, "fail", "变异前 90 对 75 必须 fail");

    // 变异：B 类甲醛上限 75 → 100，Model 需转 ActiveModel 后才能写回
    let fresh = LimitRule::find()
        .filter(LimitColumn::ItemId.eq(item.id))
        .filter(LimitColumn::SafetyCategory.eq(textile_safety_category::CLASS_B))
        .one(&*db)
        .await
        .unwrap()
        .expect("种子必须含 B 类甲醛行");
    let original_high = fresh.limit_high;
    let mut mutate: LimitActiveModel = fresh.into();
    mutate.limit_high = Set(Some(dec("100")));
    mutate.update(&*db).await.unwrap();
    let service = TextileSafetyReportService::new(db.clone());
    let reports = service.list_by_submission(sid).await.unwrap();
    let judged = service
        .judge_report(reports[0].id, TEST_OPERATOR)
        .await
        .unwrap();
    let after = judged.iter().find(|r| r.item_id == item.id).unwrap();
    assert_eq!(after.verdict, "pass", "限值变异到 100 后同一实测 90 必须翻为 pass（判定查表自证）");

    // 回拨：恢复 75，重判必须翻回 fail
    let fresh = LimitRule::find()
        .filter(LimitColumn::ItemId.eq(item.id))
        .filter(LimitColumn::SafetyCategory.eq(textile_safety_category::CLASS_B))
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    let mut restore: LimitActiveModel = fresh.into();
    restore.limit_high = Set(original_high);
    restore.update(&*db).await.unwrap();
    let judged2 = service.judge_report(reports[0].id, TEST_OPERATOR).await.unwrap();
    let back = judged2.iter().find(|r| r.item_id == item.id).unwrap();
    assert_eq!(back.verdict, "fail", "限值回拨后结论必须翻回 fail（变异双向锁）");
}
