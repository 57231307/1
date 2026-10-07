use bingxi_backend::services::ar::AutoMatchRequest;
// decs! 宏在集成测试可直接使用（crate 根重导出），用于金额字面量构造
use bingxi_backend::decs;
use bingxi_backend::models::status::{ar, common};
use bingxi_backend::services::test_common::{connect_empty_schema_db, setup_test_db};
use bingxi_backend::ymd;
// ymd! 宏在集成测试可直接使用（crate 根重导出），用于日期字面量构造
use bingxi_backend::services::ar::ArReconciliationService;
use rust_decimal::Decimal;
use std::sync::Arc;

// =====================================================
// 1. 核销相关状态常量值正确性
// =====================================================

/// test_hxztcl_ygbzzq
/// 验证 ar::RECONCILIATION_CLOSED 常量值为 "closed"（小写），与 ar_reconciliation.reconciliation_status 字段语义一致。
#[test]
fn test_hxztcl_ygbzzq() {
    assert_eq!(ar::RECONCILIATION_CLOSED, "closed");
}

/// test_hxztcl_yqxzzq
/// 验证 ar::RECONCILIATION_CANCELLED 常量值为 "cancelled"（小写），与 ar_reconciliation.reconciliation_status 字段语义一致。
#[test]
fn test_hxztcl_yqxzzq() {
    assert_eq!(ar::RECONCILIATION_CANCELLED, "cancelled");
}

/// test_ppztcl_yppzzq（验证 ar::MATCH_MATCHED 常量值为 "MATCHED"（大写），与 ar_reconciliation_item.match_status 字段语义一致。）
#[test]
fn test_ppztcl_yppzzq() {
    assert_eq!(ar::MATCH_MATCHED, "MATCHED");
}

/// test_skztcl_xxstzzq
/// 验证 ar_collection.status 字段使用的小写状态值：COLLECTION_PENDING = "pending"；COLLECTION_CONFIRMED = "confirmed"；COLLECTION_CANCELLED = "cancelled"
#[test]
fn test_skztcl_xxstzzq() {
    assert_eq!(ar::COLLECTION_PENDING, "pending");
    assert_eq!(ar::COLLECTION_CONFIRMED, "confirmed");
    assert_eq!(ar::COLLECTION_CANCELLED, "cancelled");
}

/// test_tyztcl_yqxzzq
/// 验证 common::STATUS_CANCELLED 常量值为 "CANCELLED"（大写），vfy_ops 中用于 ar_invoice.Status 过滤（ne("CANCELLED")）。
#[test]
fn test_tyztcl_yqxzzq() {
    assert_eq!(common::STATUS_CANCELLED, "CANCELLED");
}

// =====================================================
// 2. 核销金额计算（直接调用生产纯函数 `compute_closing_balance`，auto_match 的 process_customer_match 即调用它）
// =====================================================

/// test_qmyejs_zccj
/// 验证 vfy_ops 生产函数 compute_closing_balance 的期末余额公式：closing_balance = opening_balance + total_invoices - total_collections
#[test]
fn test_qmyejs_zccj() {
    let closing = ArReconciliationService::compute_closing_balance(
        decs!("1000"),
        decs!("5000"),
        decs!("3000"),
    );
    assert_eq!(closing, decs!("3000"));
}

/// test_qmyejs_lskcj（验证当期无收款时，期末余额 = 期初 + 期内核销前发票额。）
#[test]
fn test_qmyejs_lskcj() {
    let closing = ArReconciliationService::compute_closing_balance(
        decs!("2000"),
        decs!("4000"),
        Decimal::ZERO,
    );
    assert_eq!(closing, decs!("6000"));
}

/// test_qmyejs_qehxcj（验证当收款总额等于期初+发票额时，期末余额归零（核销完成）。）
#[test]
fn test_qmyejs_qehxcj() {
    let closing = ArReconciliationService::compute_closing_balance(
        decs!("1000"),
        decs!("4000"),
        decs!("5000"),
    );
    assert_eq!(closing, Decimal::ZERO);
}

// =====================================================
// 3. 日期匹配阈值（直接调用生产纯判定 `is_within_match_date_window`，auto_match 策略2 run_date_order_match_pass 即调用它）
// =====================================================

/// test_rqppyz_30tnkpp（验证 vfy_ops/match.rs auto_match 策略2 中 date_diff <= 30 时应匹配。边界值：恰好 30 天也应匹配。）
#[test]
fn test_rqppyz_30tnkpp() {
    let invoice_date = ymd!(2026, 6, 1);
    // 30 天后：边界，应匹配
    let coll_date_30 = ymd!(2026, 7, 1);
    assert!(ArReconciliationService::is_within_match_date_window(
        coll_date_30,
        invoice_date
    ));

    // 15 天后：区间内，应匹配
    let coll_date_15 = ymd!(2026, 6, 16);
    assert!(ArReconciliationService::is_within_match_date_window(
        coll_date_15,
        invoice_date
    ));
}

/// test_rqppyz_c30tbpp（验证 vfy_ops/match.rs auto_match 策略2 中 date_diff > 30 时不应匹配。）
#[test]
fn test_rqppyz_c30tbpp() {
    let invoice_date = ymd!(2026, 6, 1);
    let coll_date = ymd!(2026, 7, 2); // 31 天后
    assert!(!ArReconciliationService::is_within_match_date_window(
        coll_date,
        invoice_date
    ));
}

// =====================================================
// 4. 部分匹配金额与状态判定（直接调用生产纯函数 `compute_matched_amount` /；`classify_match_status`，auto_match 策略2 即调用它们）
// =====================================================

/// test_bfppje_qjxz（验证 vfy_ops/match.rs auto_match 策略2 中 matched = min(invoice_amount, collection_amount)。）
#[test]
fn test_bfppje_qjxz() {
    let inv_amt = decs!("5000");
    let coll_amt = decs!("3000");
    assert_eq!(
        ArReconciliationService::compute_matched_amount(inv_amt, coll_amt),
        decs!("3000")
    );

    // 反向参数同样取较小值
    assert_eq!(
        ArReconciliationService::compute_matched_amount(coll_amt, inv_amt),
        decs!("3000")
    );
}

/// test_ppztpd_wqybfpp
/// 验证 vfy_ops/match.rs auto_match 策略2 中 match_status 判定规则：matched == invoice_amount → ar::MATCH_MATCHED；matched < invoice_amount  → "PARTIAL"；注："PARTIAL" 在 status 模块无对应常量，沿用 vfy_ops/match.rs 字面量。
#[test]
fn test_ppztpd_wqybfpp() {
    let inv_amt = decs!("5000");

    // 完全匹配：matched == invoice_amount
    assert_eq!(
        ArReconciliationService::classify_match_status(decs!("5000"), inv_amt),
        ar::MATCH_MATCHED
    );

    // 部分匹配：matched < invoice_amount
    assert_eq!(
        ArReconciliationService::classify_match_status(decs!("3000"), inv_amt),
        "PARTIAL"
    );
}

// =====================================================
// 5. 未匹配数量公式（直接调用生产纯函数 `compute_unmatched_count`，build_auto_match_result 即调用它）
// =====================================================

/// test_wppslgs_zq
/// 验证 vfy_ops/match.rs auto_match 末尾 unmatched_count 公式：unmatched_count = invoices.len() + collections.len() - matched_count * 2；每次匹配消耗 1 张发票 + 1 笔收款，故乘 2。
#[test]
fn test_wppslgs_zq() {
    let invoices_len = 10usize;
    let collections_len = 8usize;
    let matched_count = 5usize;
    // 10 + 8 - 10 = 8
    assert_eq!(
        ArReconciliationService::compute_unmatched_count(
            invoices_len,
            collections_len,
            matched_count
        ),
        8
    );

    // 全部匹配：matched = min(invoices, collections) = 8
    let matched_all = std::cmp::min(invoices_len, collections_len);
    // 10 + 8 - 16 = 2（剩余 2 张发票未匹配）
    assert_eq!(
        ArReconciliationService::compute_unmatched_count(
            invoices_len,
            collections_len,
            matched_all
        ),
        2
    );
}

// =====================================================
// 6. 账龄分桶（直接调用生产纯函数 `compute_aging_bucket_index`，get_aging_report 的 build_customer_aging_summaries 即调用它）
// =====================================================

/// test_zlft_dqwyq（验证 overdue_days <= 0 时落入第 0 桶（当期）。边界值：overdue_days = 0（到期日当天）也应落入当期。）
#[test]
fn test_zlft_dqwyq() {
    assert_eq!(ArReconciliationService::compute_aging_bucket_index(0), 0);
    assert_eq!(ArReconciliationService::compute_aging_bucket_index(-1), 0);
    assert_eq!(ArReconciliationService::compute_aging_bucket_index(-30), 0);
}

/// test_zlft_1d30tqj（验证 1 <= overdue_days <= 30 时落入第 1 桶（1-30天）。边界值：1 和 30 均应落入此桶。）
#[test]
fn test_zlft_1d30tqj() {
    assert_eq!(ArReconciliationService::compute_aging_bucket_index(1), 1);
    assert_eq!(ArReconciliationService::compute_aging_bucket_index(15), 1);
    assert_eq!(ArReconciliationService::compute_aging_bucket_index(30), 1);
}

/// test_zlft_31d60tqj（验证 31 <= overdue_days <= 60 时落入第 2 桶（31-60天）。边界值：31 和 60 均应落入此桶。）
#[test]
fn test_zlft_31d60tqj() {
    assert_eq!(ArReconciliationService::compute_aging_bucket_index(31), 2);
    assert_eq!(ArReconciliationService::compute_aging_bucket_index(45), 2);
    assert_eq!(ArReconciliationService::compute_aging_bucket_index(60), 2);
}

/// test_zlft_61d90tqj（验证 61 <= overdue_days <= 90 时落入第 3 桶（61-90天）。边界值：61 和 90 均应落入此桶。）
#[test]
fn test_zlft_61d90tqj() {
    assert_eq!(ArReconciliationService::compute_aging_bucket_index(61), 3);
    assert_eq!(ArReconciliationService::compute_aging_bucket_index(75), 3);
    assert_eq!(ArReconciliationService::compute_aging_bucket_index(90), 3);
}

/// test_zlft_90tys（验证 overdue_days > 90 时落入第 4 桶（90天以上）。边界值：91 应落入此桶。）
#[test]
fn test_zlft_90tys() {
    assert_eq!(ArReconciliationService::compute_aging_bucket_index(91), 4);
    assert_eq!(ArReconciliationService::compute_aging_bucket_index(180), 4);
    assert_eq!(ArReconciliationService::compute_aging_bucket_index(365), 4);
}

// =====================================================
// 7. 状态机转换合法性（直接调用生产状态门 `check_customer_confirm_status` /
//    `check_customer_dispute_status`，customer_confirm/customer_dispute 本体即调用这两函数）
// =====================================================

/// test_khqrztj_yqrjj
/// 验证 customer_confirm 状态门中 status == confirmed 时应拒绝（不可重复确认），返回 BusinessError 且消息包含 "对账单已确认，不可重复确认"。
#[test]
fn test_khqrztj_yqrjj() {
    let err = ArReconciliationService::check_customer_confirm_status("confirmed")
        .expect_err("confirmed 状态应拒绝重复确认");
    assert_eq!(
        err.error_code(),
        "BUSINESS_ERROR",
        "拒绝族别必须命中业务错误机器码，实得 {}",
        err.error_code()
    );
    assert!(format!("{err}").contains("对账单已确认，不可重复确认"));
}

/// test_khqrztj_zyzjj
/// 验证 customer_confirm 状态门中 status == disputed 时应拒绝（需先解决争议），返回 BusinessError 且消息包含 "对账单存在争议"。
#[test]
fn test_khqrztj_zyzjj() {
    let err = ArReconciliationService::check_customer_confirm_status("disputed")
        .expect_err("disputed 状态应拒绝确认（需先解决争议）");
    assert_eq!(
        err.error_code(),
        "BUSINESS_ERROR",
        "拒绝族别必须命中业务错误机器码，实得 {}",
        err.error_code()
    );
    assert!(format!("{err}").contains("对账单存在争议"));
}

/// test_khqrztj_qtztkzh（验证 customer_confirm 状态门中 status 为 "draft" 等非终态时应允许转换到 "confirmed"。）
#[test]
fn test_khqrztj_qtztkzh() {
    // draft → confirmed：应允许
    let result = ArReconciliationService::check_customer_confirm_status("draft");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "confirmed");
}

/// test_khzyztj_yqrjj
/// 验证 customer_dispute 状态门中 status == confirmed 时应拒绝（已确认不可提争议），返回 BusinessError 且消息包含 "对账单已确认，不可提出争议"。
#[test]
fn test_khzyztj_yqrjj() {
    let err = ArReconciliationService::check_customer_dispute_status("confirmed")
        .expect_err("confirmed 状态应拒绝提争议");
    assert_eq!(
        err.error_code(),
        "BUSINESS_ERROR",
        "拒绝族别必须命中业务错误机器码，实得 {}",
        err.error_code()
    );
    assert!(format!("{err}").contains("对账单已确认，不可提出争议"));
}

/// test_khzyztj_ygbjj
/// 验证 customer_dispute 状态门中 status == closed 时应拒绝（已关闭不可提争议），返回 BusinessError 且消息包含 "对账单已关闭，不可提出争议"。
#[test]
fn test_khzyztj_ygbjj() {
    let err = ArReconciliationService::check_customer_dispute_status("closed")
        .expect_err("closed 状态应拒绝提争议");
    assert_eq!(
        err.error_code(),
        "BUSINESS_ERROR",
        "拒绝族别必须命中业务错误机器码，实得 {}",
        err.error_code()
    );
    assert!(format!("{err}").contains("对账单已关闭，不可提出争议"));
}

/// test_khzyztj_cgkzh（验证 customer_dispute 状态门中 status 为 "draft" 时应允许转换到 "disputed"。）
#[test]
fn test_khzyztj_cgkzh() {
    let result = ArReconciliationService::check_customer_dispute_status("draft");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "disputed");
}

// =====================================================
// 8. 匹配策略校验（直接调用生产纯函数 `normalize_match_strategy`，auto_match 的 parse_match_strategy 即调用它）
// =====================================================

/// test_ppcljy_wxclcwxx
/// 验证 auto_match 中传入不支持的策略时返回可外显的校验错误（策略词表是用户可自助
/// 修正的公开规则），错误消息格式："无效的匹配策略: {strategy}（支持 exact / date_order / all）"
#[test]
fn test_ppcljy_wxclcwxx() {
    let err = ArReconciliationService::normalize_match_strategy(Some("invalid"))
        .expect_err("非法策略应返回校验错误");
    assert_eq!(
        err.error_code(),
        "VALIDATION_ERROR",
        "策略词表拒绝必须命中可外显校验族机器码，实得 {}",
        err.error_code()
    );
    let msg = format!("{err}");
    assert!(msg.contains("无效的匹配策略: invalid"));
    assert!(msg.contains("exact / date_order / all"));
}

/// test_ppcljy_hfcltg（验证 auto_match 中三种合法策略（exact/date_order/all）均通过校验，且 None 默认为 "all"，大小写不敏感（自动转小写）。）
#[test]
fn test_ppcljy_hfcltg() {
    assert_eq!(
        ArReconciliationService::normalize_match_strategy(Some("exact")).unwrap(),
        "exact"
    );
    assert_eq!(
        ArReconciliationService::normalize_match_strategy(Some("date_order")).unwrap(),
        "date_order"
    );
    assert_eq!(
        ArReconciliationService::normalize_match_strategy(Some("all")).unwrap(),
        "all"
    );
    // None 默认 "all"
    assert_eq!(
        ArReconciliationService::normalize_match_strategy(None).unwrap(),
        "all"
    );
    // 大写自动转小写
    assert_eq!(
        ArReconciliationService::normalize_match_strategy(Some("EXACT")).unwrap(),
        "exact"
    );
}

// =====================================================
// 9. 夹具宏可用性（decs! / ymd!）
// =====================================================

/// test_decsh_hxjejx（验证 decs! 宏可正确解析 vfy_ops 业务场景的金额字符串（含小数），并可参与期末余额公式运算。）
#[test]
fn test_decsh_hxjejx() {
    let inv = decs!("12345.67");
    assert_eq!(inv.to_string(), "12345.67");

    let coll = decs!("3000");
    assert_eq!(coll.to_string(), "3000");

    // 期末余额计算应正常工作
    let opening = decs!("1000");
    let closing = opening + inv - coll;
    assert_eq!(closing.to_string(), "10345.67");
}

/// test_ymdh_dzrqjx（验证 ymd! 宏可正确解析 vfy_ops 业务场景的对账期间日期，并可参与日期差运算（auto_match 策略2 依赖）。）
#[test]
fn test_ymdh_dzrqjx() {
    let start = ymd!(2026, 1, 1);
    let end = ymd!(2026, 3, 31);
    assert_eq!(start.to_string(), "2026-01-01");
    assert_eq!(end.to_string(), "2026-03-31");

    // 日期差计算（auto_match 策略2 依赖）
    let diff = (end - start).num_days();
    assert_eq!(diff, 89);
}

// =====================================================
// 10. 数据库交互测试（服务实例化 + 标注 #[ignore] 的端到端）
// =====================================================

/// test_fwslh_xsjk（验证 ArReconciliationService::new 可用 SQLite 内存库构造实例，仅验证实例化成功（不需要 schema），与模板 test_fwslcj 同模式。）
#[tokio::test]
async fn test_fwslh_xsjk() {
    let db = setup_test_db().await;
    let svc = ArReconciliationService::new(Arc::new(db));
    // 验证实例化成功：Arc 引用计数 >= 1
    assert!(Arc::strong_count(&svc.db) >= 1);
}

/// test_zddzwzlc_xsjk —— 钉"schema 缺失（customers 表根本不存在）时 auto_match
/// 返回 DATABASE_ERROR 机器码而非 panic"，属负前提用例，绑
/// `connect_empty_schema_db()`（不跑迁移的 `bingxi_empty` 库）。
///
/// 真实契约依据（读函数体，非读注释）：`src/services/ar/vfy_ops/match.rs:22-63`
/// auto_match 先 `parse_match_strategy`（match.rs:80-86，委托 `normalize_match_strategy`
/// :92-101，"all" 合法 ⇒ Ok），随后
/// `begin` + `load_match_customers`（match.rs:151-166 `customer::Entity::find`）；
/// 缺表 ⇒ DbErr::Query ⇒ `utils/error.rs:561-578` AppError::database ⇒
/// error_code "DATABASE_ERROR"（error.rs:798）。
/// 本断言必须绑负前提交集而非 `setup_test_db()`：后者语义 = 已迁移 PG +
/// TRUNCATE 业务表，空表 ⇒ customers=[] ⇒ 循环体不执行 ⇒ `Ok(vec![])` 才是
/// 真实契约（match.rs:29-62），在其上断 `is_err` 必红。
#[tokio::test]
#[ignore]
async fn test_zddzwzlc_xsjk() {
    let db = connect_empty_schema_db().await;
    let svc = ArReconciliationService::new(Arc::new(db));
    let req = AutoMatchRequest {
        customer_id: None,
        start_date: ymd!(2026, 1, 1),
        end_date: ymd!(2026, 3, 31),
        match_strategy: Some("all".to_string()),
    };
    let err = svc
        .auto_match(req, 1)
        .await
        .expect_err("schema 缺失（无 customers 表）时 auto_match 必须返回 Err 而非 panic");
    assert_eq!(
        err.error_code(),
        "DATABASE_ERROR",
        "缺表必须命中数据库错误族（match.rs:29 → error.rs:562-565），实得 {}",
        err.error_code()
    );
}

/// test_zlbgwzlc_xsjk —— 与 test_zddzwzlc_xsjk 同型负前提：绑
/// `connect_empty_schema_db()`，钉"schema 缺失时 get_aging_report 返回
/// DATABASE_ERROR 而非 panic"。
///
/// 真实契约依据：`src/services/ar/vfy_ops/aging.rs:31-52` get_aging_report 首步
/// `load_unpaid_invoices`（aging.rs:54-73 `ar_invoice::Entity::find().all()`）；
/// 缺表 ⇒ DbErr::Query ⇒ DATABASE_ERROR（error.rs:561-578,798）。
/// 空表（真库化夹具）上则恒 `Ok`：invoices=[] ⇒ 五桶全零、total_receivable=0 的
/// 报告（aging.rs:41-51），故本例不能绑 `setup_test_db()`，否则 `is_err()` 必红。
#[tokio::test]
#[ignore]
async fn test_zlbgwzlc_xsjk() {
    let db = connect_empty_schema_db().await;
    let svc = ArReconciliationService::new(Arc::new(db));
    let err = svc
        .get_aging_report(None, None, None)
        .await
        .expect_err("schema 缺失（无 ar_invoices 表）时 get_aging_report 必须返回 Err 而非 panic");
    assert_eq!(
        err.error_code(),
        "DATABASE_ERROR",
        "缺表必须命中数据库错误族（aging.rs:37 → error.rs:562-565），实得 {}",
        err.error_code()
    );
}
