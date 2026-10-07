//! AR 核销侧「金额一致性」真库契约锁：收款单级核销账本读写口径
//!
//! 锁定的契约（每条均为真 PostgreSQL + 真实服务链行为证明，非源码文本比对）：
//! 1. 创建收款直连发票路径与手工/自动核销共用同一写账本入口——每张实际分配的发票
//!    同事务落核销单主记录（closed）+ INVOICE(+分配额)/RECEIPT(-分配额,
//!    document_type=AR_COLLECTION) 明细对；由此"把收款金额改成小于已核销额"在该路径
//!    同样命中修改金额的下限拒绝门（修复前该路径不落账本行，门为空判可骗过）。
//! 2. `cancel_collection` 同事务回收账本：发票侧 received_amount/unpaid_amount/status
//!    精确回滚、关联核销单 closed→cancelled（明细留痕不删行），收款占用额随之释放；
//!    任一被拒场景（confirmed 状态、明细缺失导致的回滚失败）必须整体回滚、库内零部分写入。
//! 3. 收款单级"已核销分配额"读数（collection.rs::active_receipt_ledger_items 唯一实现）
//!    只计有效核销：document_type=AR_COLLECTION ∧ 主单 reconciliation_status=closed
//!    （核销状态词表正向集合，非 `!= cancelled` 反向判据）；取消核销后该读数随之下降，
//!    以"可用余额重新可核销"端到端可观测。
//! 4. 核销账本口径（amount=本次分配额）与对账单口径（amount=单据全额、
//!    matched_amount=匹配额，document_type=COLLECTION）分族：同一求和函数不混读两种
//!    口径——自动对账产生的全额明细行不得挤占核销可用余额，也不得计入修改金额下限。
//!
//! 夹具口径：全部用例走 `test_common::setup_test_db()`（真 PostgreSQL，缺
//! TEST_DATABASE_URL 直接 panic，禁 sqlite 回退）；FK 父行（users/customers）自种；
//! 日期取 2026-04 私有窗口段，避免与种子参照数据相撞。

mod test_common;

use std::str::FromStr;
use std::sync::Arc;

use chrono::{NaiveDate, TimeZone, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};
use serde_json::json;
use test_common::setup_test_db;

use bingxi_backend::models::status::{ar as ar_status, common, master_data, payment};
use bingxi_backend::models::{
    ar_collection, ar_invoice, ar_reconciliation, ar_reconciliation_item, customer, user,
};
use bingxi_backend::services::ar::{ArReconciliationService, AutoMatchRequest};
use bingxi_backend::services::ar_service::{ArService, CreateArPaymentParams};

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn unique_tag() -> i64 {
    Utc::now()
        .timestamp_nanos_opt()
        .expect("Utc::now 纳秒必落在 chrono 可表示区间（约 1678-2262 年），None 不可达")
}

fn fixture_date() -> NaiveDate {
    // 2026-04-10：本锁全部业务日期落在 2026-04-01..2026-04-30 窗口内
    NaiveDate::from_ymd_opt(2026, 4, 10).unwrap()
}

// =========================================================
// 种子助手（FK 父行自建，不依赖外部库状态）
// =========================================================

async fn seed_user(db: &DatabaseConnection, name: &str) -> i32 {
    let now = Utc::now();
    user::ActiveModel {
        username: Set(name.to_string()),
        password_hash: Set("x".repeat(60)),
        real_name: Set(Some(name.to_string())),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
    .id
}

async fn seed_customer(db: &DatabaseConnection, owner_id: i32) -> i32 {
    let now = Utc::now();
    customer::ActiveModel {
        customer_code: Set(format!("CUS-WARV-{}", unique_tag())),
        customer_name: Set("AR核销一致性契约客户".to_string()),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set(master_data::ACTIVE.to_string()),
        customer_type: Set("retail".to_string()),
        owner_id: Set(owner_id),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
    .id
}

/// 建应收单：APPROVED、received=0、unpaid=amount（固定 2026-04 窗口日期）
async fn seed_ar_invoice(
    db: &DatabaseConnection,
    customer_id: i32,
    user_id: i32,
    amount: &str,
) -> i32 {
    let now = Utc::now();
    let amt = dec(amount);
    ar_invoice::ActiveModel {
        invoice_no: Set(format!("ARWAV-{}", unique_tag())),
        invoice_date: Set(fixture_date()),
        due_date: Set(fixture_date()),
        customer_id: Set(customer_id),
        invoice_amount: Set(amt),
        received_amount: Set(Decimal::ZERO),
        unpaid_amount: Set(amt),
        status: Set(common::STATUS_APPROVED.to_string()),
        approval_status: Set(common::STATUS_APPROVED.to_string()),
        created_by: Set(user_id),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
    .id
}

/// 直插已确认收款（模拟"收款先确认再核销"的真实链路形状）
async fn seed_confirmed_collection(
    db: &DatabaseConnection,
    customer_id: i32,
    user_id: i32,
    amount: &str,
) -> i32 {
    let now = Utc::now();
    ar_collection::ActiveModel {
        collection_no: Set(format!("COLWAV-{}", unique_tag())),
        collection_date: Set(fixture_date()),
        customer_id: Set(customer_id),
        collection_amount: Set(dec(amount)),
        collection_method: Set(Some("银行转账".to_string())),
        status: Set(ar_status::COLLECTION_CONFIRMED.to_string()),
        confirmed_by: Set(Some(user_id)),
        confirmed_at: Set(Some(now)),
        created_by: Set(user_id),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
    .id
}

/// 播 2026-04 会计期间且为 OPEN（覆盖 `fixture_date()` 2026-04-10）。
/// `create_payment` 经 `check_payment_period_locked`→`check_date_locked_txn`
/// （ar_ops/collection.rs:122 / accounting_period_service.rs:638，闭区间 start<=date<=end）
/// 要求收款日期落在已设置期间内；`accounting_periods` 非参照表、随 setup_test_db 被清空，
/// 故按 seed 范式在每用例真库播种真实前置数据（OPEN 词表常量与写入方逐字符相同）。
async fn seed_open_period_2026_04(db: &DatabaseConnection) {
    let start = NaiveDate::from_ymd_opt(2026, 4, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let end = NaiveDate::from_ymd_opt(2026, 4, 30)
        .unwrap()
        .and_hms_opt(23, 59, 59)
        .unwrap();
    bingxi_backend::models::accounting_period::ActiveModel {
        year: Set(2026),
        period: Set(4),
        period_name: Set("2026-04".to_string()),
        start_date: Set(Utc.from_utc_datetime(&start)),
        end_date: Set(Utc.from_utc_datetime(&end)),
        status: Set(bingxi_backend::models::status::accounting_period::OPEN.to_string()),
        created_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

async fn reload_invoice(db: &DatabaseConnection, id: i32) -> ar_invoice::Model {
    ar_invoice::Entity::find_by_id(id)
        .one(db)
        .await
        .unwrap()
        .unwrap_or_else(|| panic!("应收单 {id} 应存在"))
}

async fn reload_collection(db: &DatabaseConnection, id: i32) -> ar_collection::Model {
    ar_collection::Entity::find_by_id(id)
        .one(db)
        .await
        .unwrap()
        .unwrap_or_else(|| panic!("收款单 {id} 应存在"))
}

async fn reload_reconciliation(db: &DatabaseConnection, id: i32) -> ar_reconciliation::Model {
    ar_reconciliation::Entity::find_by_id(id)
        .one(db)
        .await
        .unwrap()
        .unwrap_or_else(|| panic!("核销单 {id} 应存在"))
}

/// 某收款单的 RECEIPT 明细，按 document_type 分族返回：
/// (核销账本族 AR_COLLECTION, 对账单族 COLLECTION)
async fn receipt_lines_of(
    db: &DatabaseConnection,
    payment_id: i32,
) -> (
    Vec<ar_reconciliation_item::Model>,
    Vec<ar_reconciliation_item::Model>,
) {
    let lines = ar_reconciliation_item::Entity::find()
        .filter(ar_reconciliation_item::Column::ItemType.eq("RECEIPT"))
        .filter(ar_reconciliation_item::Column::DocumentId.eq(payment_id))
        .all(db)
        .await
        .unwrap();
    let ledger = lines
        .iter()
        .filter(|l| l.document_type.as_deref() == Some("AR_COLLECTION"))
        .cloned()
        .collect();
    let statement = lines
        .iter()
        .filter(|l| l.document_type.as_deref() == Some("COLLECTION"))
        .cloned()
        .collect();
    (ledger, statement)
}

// =========================================================
// 锁 ①：创建直连发票落 RECEIPT 账本行；下调金额低于分配额被业务拒绝
// =========================================================

#[tokio::test]
async fn create_payment_linked_invoice_lands_ledger_and_blocks_decrease() {
    let db = setup_test_db().await;
    seed_open_period_2026_04(&db).await;
    let uid = seed_user(&db, "warv_link1").await;
    let cid = seed_customer(&db, uid).await;
    let inv_id = seed_ar_invoice(&db, cid, uid, "1000.00").await;

    let service = ArService::new(Arc::new(db.clone()));
    let created = service
        .create_payment(
            CreateArPaymentParams {
                customer_id: cid,
                amount: dec("300.00"),
                payment_method: "BANK_TRANSFER".to_string(),
                payment_date: fixture_date(),
                bank_account: None,
                remark: None,
                invoice_ids: Some(vec![inv_id]),
            },
            uid,
        )
        .await
        .expect("创建收款并直连发票应成功");
    let payment_id = created["id"].as_i64().expect("响应应含收款单 id") as i32;

    // 发票侧回写（既有行为不回退）
    let inv = reload_invoice(&db, inv_id).await;
    assert_eq!(
        inv.received_amount,
        dec("300.00"),
        "直连分配额必须真实回写发票"
    );
    assert_eq!(inv.unpaid_amount, dec("700.00"));
    assert_eq!(inv.status, payment::PAYMENT_PARTIAL_PAID);

    // 收款单级 RECEIPT 账本行必须存在且为核销口径（与手工核销同形状）
    let (ledger_lines, statement_lines) = receipt_lines_of(&db, payment_id).await;
    assert!(statement_lines.is_empty(), "核销路径不得产出对账单族明细");
    assert_eq!(
        ledger_lines.len(),
        1,
        "直连路径必须落收款单级 RECEIPT 账本行"
    );
    let line = &ledger_lines[0];
    assert_eq!(line.document_type.as_deref(), Some("AR_COLLECTION"));
    assert_eq!(
        line.amount,
        dec("-300.00"),
        "账本行按记账方向存负、金额=本次分配额"
    );
    assert_eq!(line.matched_amount, Some(dec("300.00")));
    let rec = reload_reconciliation(&db, line.reconciliation_id).await;
    assert_eq!(
        rec.reconciliation_status.as_deref(),
        Some(ar_status::RECONCILIATION_CLOSED),
        "直连路径的核销单主记录应与手工核销同为 closed"
    );
    let invoice_lines = ar_reconciliation_item::Entity::find()
        .filter(ar_reconciliation_item::Column::ItemType.eq("INVOICE"))
        .filter(ar_reconciliation_item::Column::ReconciliationId.eq(rec.id))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(
        invoice_lines.len(),
        1,
        "RECEIPT 行必须与 INVOICE 行成对落账"
    );
    assert_eq!(invoice_lines[0].amount, dec("300.00"));

    // 修改金额低于已核销分配额 → 业务拒绝（修复前此路径无账本行，门可被骗过）
    let err = service
        .update_payment(payment_id, json!({"amount": "200.00"}), uid)
        .await
        .err()
        .unwrap_or_else(|| panic!("新金额 200 小于已分配 300 必须被拒"));
    assert_eq!(err.error_code(), "BUSINESS_ERROR");

    // 拒绝即整体回滚：收款单与发票侧零部分写入
    let row = reload_collection(&db, payment_id).await;
    assert_eq!(
        row.collection_amount,
        dec("300.00"),
        "被拒请求不得写入新金额"
    );
    let inv = reload_invoice(&db, inv_id).await;
    assert_eq!(
        inv.received_amount,
        dec("300.00"),
        "被拒请求不得触碰发票侧金额"
    );

    // 下限边界：等于已分配额放行
    service
        .update_payment(payment_id, json!({"amount": "300.00"}), uid)
        .await
        .expect("新金额等于已分配额处于下限边界，应放行");
}

// =========================================================
// 锁 ②：cancel_collection 同事务回滚发票侧并回收 RECEIPT 账本行
// =========================================================

#[tokio::test]
async fn cancel_collection_rolls_back_invoice_and_recovers_ledger() {
    let db = setup_test_db().await;
    seed_open_period_2026_04(&db).await;
    let uid = seed_user(&db, "warv_cancel2").await;
    let cid = seed_customer(&db, uid).await;
    let inv_id = seed_ar_invoice(&db, cid, uid, "1000.00").await;

    let service = ArService::new(Arc::new(db.clone()));
    let created = service
        .create_payment(
            CreateArPaymentParams {
                customer_id: cid,
                amount: dec("300.00"),
                payment_method: "BANK_TRANSFER".to_string(),
                payment_date: fixture_date(),
                bank_account: None,
                remark: None,
                invoice_ids: Some(vec![inv_id]),
            },
            uid,
        )
        .await
        .expect("创建收款并直连发票应成功");
    let payment_id = created["id"].as_i64().unwrap() as i32;

    service
        .cancel_collection(payment_id, uid)
        .await
        .expect("pending 收款单取消应成功");

    // 收款单置 cancelled
    let row = reload_collection(&db, payment_id).await;
    assert_eq!(row.status, ar_status::COLLECTION_CANCELLED);

    // 发票侧金额与状态精确回滚（修复前：收款作废而发票仍记已收）
    let inv = reload_invoice(&db, inv_id).await;
    assert_eq!(
        inv.received_amount,
        Decimal::ZERO,
        "取消收款必须回滚发票 received"
    );
    assert_eq!(
        inv.unpaid_amount,
        dec("1000.00"),
        "取消收款必须恢复发票 unpaid"
    );
    assert_eq!(
        inv.status,
        common::STATUS_APPROVED,
        "received 归零后状态按门控回 APPROVED"
    );

    // 账本回收：明细留痕不删行，所属核销单 closed→cancelled（退出有效核销集合）
    let (ledger_lines, _) = receipt_lines_of(&db, payment_id).await;
    assert_eq!(ledger_lines.len(), 1, "回收是留痕状态流转，不删明细行");
    let rec = reload_reconciliation(&db, ledger_lines[0].reconciliation_id).await;
    assert_eq!(
        rec.reconciliation_status.as_deref(),
        Some(ar_status::RECONCILIATION_CANCELLED),
        "回收后核销单应置 cancelled，其明细不再计入已核销分配读数"
    );
    assert_eq!(
        ledger_lines[0].amount,
        dec("-300.00"),
        "留痕明细金额不得被改写"
    );

    // 闭环证明：同发票再次直连 500，分配基线是回滚后的 0（不是 300+500）
    let created2 = service
        .create_payment(
            CreateArPaymentParams {
                customer_id: cid,
                amount: dec("500.00"),
                payment_method: "BANK_TRANSFER".to_string(),
                payment_date: fixture_date(),
                bank_account: None,
                remark: None,
                invoice_ids: Some(vec![inv_id]),
            },
            uid,
        )
        .await
        .expect("回滚后发票应可再次关联");
    let payment2_id = created2["id"].as_i64().unwrap() as i32;
    let inv = reload_invoice(&db, inv_id).await;
    assert_eq!(
        inv.received_amount,
        dec("500.00"),
        "再次分配必须以回滚后余额为基线"
    );
    // 新收款单的下限读数恰为其自身分配额 500（旧回收行不计入任何收款单）
    let err = service
        .update_payment(payment2_id, json!({"amount": "400.00"}), uid)
        .await
        .err()
        .unwrap_or_else(|| panic!("新收款 2 的已分配 500 应阻断下调到 400"));
    assert_eq!(err.error_code(), "BUSINESS_ERROR");
}

// =========================================================
// 锁 ③：取消核销后收款单已核销分配读数随之下降（可用余额端到端释放）
// =========================================================

#[tokio::test]
async fn cancel_verification_releases_payment_verified_total() {
    let db = setup_test_db().await;
    let uid = seed_user(&db, "warv_cancelver3").await;
    let cid = seed_customer(&db, uid).await;
    let inv_id = seed_ar_invoice(&db, cid, uid, "1000.00").await;
    let payment_id = seed_confirmed_collection(&db, cid, uid, "600.00").await;

    let service = ArService::new(Arc::new(db.clone()));
    let v1 = service
        .manual_verify(inv_id, payment_id, dec("300.00"), None, uid)
        .await
        .expect("首次核销 300 应成功");
    let v1_id = v1["id"].as_i64().unwrap() as i32;

    // 占用后余额只剩 300：核 400 必须被拒（读数含 v1 的 300）
    let err = service
        .manual_verify(inv_id, payment_id, dec("400.00"), None, uid)
        .await
        .err()
        .unwrap_or_else(|| panic!("可用余额仅 300 时核销 400 必须被拒"));
    assert_eq!(err.error_code(), "BUSINESS_ERROR");

    service
        .cancel_verification(v1_id, uid)
        .await
        .expect("取消核销应成功");

    // 取消后占用释放：同额度 400 必须成功——锁"已作废核销仍计入已分配额"的冲突已消除
    service
        .manual_verify(inv_id, payment_id, dec("400.00"), None, uid)
        .await
        .expect("取消核销后已核销分配读数应下降，400 应可核销");

    // 进一步钉读数恰为 400：再核 200 成功（占用到 600 上限）、再核 1 被拒
    service
        .manual_verify(inv_id, payment_id, dec("200.00"), None, uid)
        .await
        .expect("收款 600 - 已核销 400 = 可用 200，核 200 应成功");
    let err = service
        .manual_verify(inv_id, payment_id, dec("1.00"), None, uid)
        .await
        .err()
        .unwrap_or_else(|| panic!("可用余额已耗尽，核 1 必须被拒"));
    assert_eq!(err.error_code(), "BUSINESS_ERROR");

    // 留痕：v1 的明细仍在库中，但其主单已 cancelled，不属有效核销集合
    let (ledger_lines, _) = receipt_lines_of(&db, payment_id).await;
    assert_eq!(
        ledger_lines.len(),
        3,
        "两笔有效核销 + 一笔已取消留痕明细应共存于库"
    );
    let mut active_count = 0usize;
    for line in &ledger_lines {
        let rec = reload_reconciliation(&db, line.reconciliation_id).await;
        if rec.reconciliation_status.as_deref() == Some(ar_status::RECONCILIATION_CLOSED) {
            active_count += 1;
        }
    }
    assert_eq!(active_count, 2, "有效核销（closed）恰为后两笔");
}

// =========================================================
// 锁 ④：核销账本口径（分配额）与对账单口径（全额）分族、各自与明细求和一致
// =========================================================

#[tokio::test]
async fn ledger_and_statement_amount_scopes_do_not_mix() {
    let db = setup_test_db().await;
    let uid = seed_user(&db, "warv_scope4").await;
    let cid = seed_customer(&db, uid).await;
    let inv1 = seed_ar_invoice(&db, cid, uid, "1000.00").await;
    let inv2 = seed_ar_invoice(&db, cid, uid, "1000.00").await;
    let payment_a = seed_confirmed_collection(&db, cid, uid, "1000.00").await;

    let service = ArService::new(Arc::new(db.clone()));
    service
        .manual_verify(inv1, payment_a, dec("400.00"), None, uid)
        .await
        .expect("手工核销 400 应成功");

    // 自动对账（精确策略）：收款 A(1000) 与某张发票全额配对，
    // 落 COLLECTION 族明细——amount=单据全额、matched_amount=匹配额
    let recon_service = ArReconciliationService::new(Arc::new(db.clone()));
    let results = recon_service
        .auto_match(
            AutoMatchRequest {
                customer_id: Some(cid),
                start_date: NaiveDate::from_ymd_opt(2026, 4, 1).unwrap(),
                end_date: NaiveDate::from_ymd_opt(2026, 4, 30).unwrap(),
                match_strategy: Some("exact".to_string()),
            },
            uid,
        )
        .await
        .expect("自动对账应成功");
    assert!(
        results.iter().any(|r| r.matched_count == 1),
        "精确策略应命中 1 对，实际 {results:?}"
    );

    // 分族读数：AR_COLLECTION 族 = 核销分配额；COLLECTION 族 = 对账单全额
    let (ledger_lines, statement_lines) = receipt_lines_of(&db, payment_a).await;
    assert_eq!(ledger_lines.len(), 1, "对账全额行不得混入核销账本族");
    assert_eq!(
        statement_lines.len(),
        1,
        "对账命中应落一条 COLLECTION 族明细"
    );
    let ledger_sum: Decimal = ledger_lines.iter().map(|l| l.amount.abs()).sum();
    let statement_sum: Decimal = statement_lines.iter().map(|l| l.amount.abs()).sum();
    assert_eq!(ledger_sum, dec("400.00"), "核销族求和=已分配额合计");
    assert_eq!(
        statement_sum,
        dec("1000.00"),
        "对账族 amount 记收款单全额（分配额在 matched_amount 列）"
    );
    let st = &statement_lines[0];
    assert_eq!(
        st.matched_amount,
        Some(dec("1000.00")),
        "对账族匹配额单列维护"
    );
    let st_parent = reload_reconciliation(&db, st.reconciliation_id).await;
    assert_eq!(
        st_parent.reconciliation_status.as_deref(),
        Some(ar_status::RECONCILIATION_DRAFT),
        "自动对账单为 draft，本就不属核销账本"
    );
    // 核销族逐行与明细 matched_amount 一致（同一写入口两列同源）
    for line in &ledger_lines {
        assert_eq!(
            line.amount.abs(),
            line.matched_amount.unwrap(),
            "分配额行 amount 与 matched_amount 同值"
        );
    }

    // 关键行为钉：核销可用余额门只读核销族——若混入对账族全额 1000，
    // 可用余额将为 1000-(400+1000)<0，下面这笔 600 必被误拒
    service
        .manual_verify(inv2, payment_a, dec("600.00"), None, uid)
        .await
        .expect("可用余额=1000-400=600（对账全额行不得挤占），核 600 应成功");

    // 收款 A 已足额核销（400+600=1000）→ 可核销收款列表按同一读数将其剔除；
    // 无账本行的收款 B 仍在列表中（正向对照，防"全量剔除"假绿）
    let payment_b = seed_confirmed_collection(&db, cid, uid, "100.00").await;
    let unverified = service
        .get_unverified_payments(json!({}))
        .await
        .expect("可核销收款列表查询应成功");
    let listed_ids: Vec<i64> = unverified
        .as_array()
        .expect("服务直调返回数组载荷")
        .iter()
        .map(|r| r["id"].as_i64().expect("行应含数字 id"))
        .collect();
    assert!(
        !listed_ids.contains(&(payment_a as i64)),
        "已核销分配合计 1000=收款全额，A 应被剔除，实际列表 {listed_ids:?}"
    );
    assert!(
        listed_ids.contains(&(payment_b as i64)),
        "无账本行的收款 B 必须仍在列表（正向对照），实际 {listed_ids:?}"
    );
}

// =========================================================
// 锁 ⑤：cancel_collection 中途失败 ⇒ 整体回滚、无部分写入
// =========================================================

#[tokio::test]
async fn cancel_collection_midway_failure_writes_nothing() {
    let db = setup_test_db().await;
    seed_open_period_2026_04(&db).await;
    let uid = seed_user(&db, "warv_rollback5").await;
    let cid = seed_customer(&db, uid).await;
    let inv_id = seed_ar_invoice(&db, cid, uid, "1000.00").await;

    let service = ArService::new(Arc::new(db.clone()));
    let created = service
        .create_payment(
            CreateArPaymentParams {
                customer_id: cid,
                amount: dec("300.00"),
                payment_method: "BANK_TRANSFER".to_string(),
                payment_date: fixture_date(),
                bank_account: None,
                remark: None,
                invoice_ids: Some(vec![inv_id]),
            },
            uid,
        )
        .await
        .expect("创建收款并直连发票应成功");
    let payment_id = created["id"].as_i64().unwrap() as i32;
    let (ledger_lines, _) = receipt_lines_of(&db, payment_id).await;
    let rec_id = ledger_lines[0].reconciliation_id;

    // 故障注入：向该核销单追加一条指向不存在应收单的 INVOICE 明细，
    // 回收链按明细逐条回滚时必然在中途命中缺失发票（真实明细 id 更小、先被处理，
    // 若回滚非整体则库中会残留"发票金额已冲减但核销单未取消"的半成品态）
    let now = Utc::now();
    ar_reconciliation_item::ActiveModel {
        reconciliation_id: Set(rec_id),
        item_type: Set("INVOICE".to_string()),
        document_type: Set(Some("SALES_INVOICE".to_string())),
        document_id: Set(Some(9_999_999)),
        document_no: Set(Some("GHOST-ROW".to_string())),
        document_date: Set(Some(fixture_date())),
        amount: Set(dec("50.00")),
        matched_amount: Set(Some(dec("50.00"))),
        match_status: Set(ar_status::MATCH_MATCHED.to_string()),
        matched_item_id: Set(None),
        remarks: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let err_msg = match service.cancel_collection(payment_id, uid).await {
        Ok(v) => panic!("指向缺失应收单的回收必须失败，实际成功: {v}"),
        Err(e) => format!("{e}"),
    };
    assert!(
        err_msg.contains("应收单"),
        "失败原因应为回收链命中缺失应收单，实际: {err_msg}"
    );

    // 整体回滚证明：收款单、核销单、发票侧、账本行全部维持原值
    let row = reload_collection(&db, payment_id).await;
    assert_eq!(
        row.status,
        ar_status::COLLECTION_PENDING,
        "失败后收款单不得被置 cancelled"
    );
    let rec = reload_reconciliation(&db, rec_id).await;
    assert_eq!(
        rec.reconciliation_status.as_deref(),
        Some(ar_status::RECONCILIATION_CLOSED),
        "失败后核销单不得被置 cancelled"
    );
    let inv = reload_invoice(&db, inv_id).await;
    assert_eq!(
        inv.received_amount,
        dec("300.00"),
        "失败后发票金额不得半途冲减"
    );
    assert_eq!(inv.unpaid_amount, dec("700.00"));
    let (ledger_lines, _) = receipt_lines_of(&db, payment_id).await;
    assert_eq!(ledger_lines[0].amount, dec("-300.00"), "账本行不得被改写");

    // 读数未被半途回收破坏：下调门仍按 300 生效
    let err = service
        .update_payment(payment_id, json!({"amount": "200.00"}), uid)
        .await
        .err()
        .unwrap_or_else(|| panic!("半途失败未回收账本，下调 200 仍必须被拒"));
    assert_eq!(err.error_code(), "BUSINESS_ERROR");
}
