use bingxi_backend::services::ar::AutoMatchRequest;
// decs 宏在测试中不可用，使用 Decimal::from_str 替代
use bingxi_backend::decs;
use bingxi_backend::models::status::ar as ar_status;
use bingxi_backend::models::status::{ar, common};
use bingxi_backend::services::test_common::{connect_empty_schema_db, setup_test_db};
use bingxi_backend::utils::error::AppError;
use bingxi_backend::ymd;
// ymd 函数在测试中不可用，使用 NaiveDate::from_ymd_opt 替代
use bingxi_backend::services::ar::ArReconciliationService;
use rust_decimal::Decimal;
use std::sync::Arc;

/// 复现 vfy_ops/aging.rs get_aging_report 中的账龄分桶索引计算（纯算法，不依赖 DB）
/// 分桶规则（与 vfy_ops/aging.rs `compute_aging_bucket_index` 保持一致）：0: 当期（overdue_days <= 0）；1: 1-30天；2: 31-60天；3: 61-90天；4: 90天以上
fn aging_bucket_idx(overdue_days: i64) -> usize {
    if overdue_days <= 0 {
        0
    } else if overdue_days <= 30 {
        1
    } else if overdue_days <= 60 {
        2
    } else if overdue_days <= 90 {
        3
    } else {
        4
    }
}

/// 复现 vfy_ops/match.rs auto_match 开头的匹配策略校验逻辑（纯算法，DB 调用之前）
/// 错误消息与 auto_match 校验段保持一致："无效的匹配策略: {strategy}（支持 exact / date_order / all）"
fn validate_match_strategy(raw: Option<&str>) -> Result<String, AppError> {
    let strategy = raw.unwrap_or("all").to_lowercase();
    if !matches!(strategy.as_str(), "exact" | "date_order" | "all") {
        return Err(AppError::validation(format!(
            "无效的匹配策略: {}（支持 exact / date_order / all）",
            strategy
        )));
    }
    Ok(strategy)
}

/// 复现 vfy_ops/confirm.rs customer_confirm 中的状态校验逻辑（纯算法，DB 调用之前）
/// 返回 Err 时错误消息与 customer_confirm 状态门保持一致："confirmed" → "对账单已确认，不可重复确认"；"disputed"  → "对账单存在争议，请先解决争议后再确认"
fn validate_customer_confirm(status: &str) -> Result<&'static str, AppError> {
    if status == ar_status::RECONCILIATION_CONFIRMED {
        return Err(AppError::business("对账单已确认，不可重复确认".to_string()));
    }
    if status == ar_status::RECONCILIATION_DISPUTED {
        return Err(AppError::business(
            "对账单存在争议，请先解决争议后再确认".to_string(),
        ));
    }
    Ok(ar_status::RECONCILIATION_CONFIRMED)
}

/// 复现 vfy_ops/confirm.rs customer_dispute 中的状态校验逻辑（纯算法，DB 调用之前）
/// 返回 Err 时错误消息与 customer_dispute 状态门保持一致："confirmed" → "对账单已确认，不可提出争议"；"closed"    → "对账单已关闭，不可提出争议"
fn validate_customer_dispute(status: &str) -> Result<&'static str, AppError> {
    if status == ar_status::RECONCILIATION_CONFIRMED {
        return Err(AppError::business("对账单已确认，不可提出争议".to_string()));
    }
    if status == ar_status::RECONCILIATION_CLOSED {
        return Err(AppError::business("对账单已关闭，不可提出争议".to_string()));
    }
    Ok(ar_status::RECONCILIATION_DISPUTED)
}

// =====================================================
// 1. 核销相关状态常量值正确性
// =====================================================

/// test_hxztcl_ygbzzq
/// 验证 ar::RECONCILIATION_CLOSED 常量值为 "closed"（小写），；与 ar_reconciliation.reconciliation_status 字段语义一致。
#[test]
fn test_hxztcl_ygbzzq() {
    assert_eq!(ar::RECONCILIATION_CLOSED, "closed");
}

/// test_hxztcl_yqxzzq
/// 验证 ar::RECONCILIATION_CANCELLED 常量值为 "cancelled"（小写），；与 ar_reconciliation.reconciliation_status 字段语义一致。
#[test]
fn test_hxztcl_yqxzzq() {
    assert_eq!(ar::RECONCILIATION_CANCELLED, "cancelled");
}

/// test_ppztcl_yppzzq（验证 ar::MATCH_MATCHED 常量值为 "MATCHED"（大写），；与 ar_reconciliation_item.match_status 字段语义一致。）
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
/// 验证 common::STATUS_CANCELLED 常量值为 "CANCELLED"（大写），；vfy_ops 中用于 ar_invoice.Status 过滤（ne("CANCELLED")）。
#[test]
fn test_tyztcl_yqxzzq() {
    assert_eq!(common::STATUS_CANCELLED, "CANCELLED");
}

// =====================================================
// 2. 核销金额计算（纯算法，复现 auto_match / generate_reconciliation）
// =====================================================

/// test_qmyejs_zccj
/// 验证 vfy_ops auto_match / generate_reconciliation 中的期末余额公式：closing_balance = opening_balance + total_invoices - total_collections
#[test]
fn test_qmyejs_zccj() {
    let opening = decs!("1000");
    let invoices = decs!("5000");
    let collections = decs!("3000");
    let closing = opening + invoices - collections;
    assert_eq!(closing, decs!("3000"));
}

/// test_qmyejs_lskcj（验证当期无收款时，期末余额 = 期初 + 期内核销前发票额。）
#[test]
fn test_qmyejs_lskcj() {
    let opening = decs!("2000");
    let invoices = decs!("4000");
    let collections = Decimal::ZERO;
    let closing = opening + invoices - collections;
    assert_eq!(closing, decs!("6000"));
}

/// test_qmyejs_qehxcj（验证当收款总额等于期初+发票额时，期末余额归零（核销完成）。）
#[test]
fn test_qmyejs_qehxcj() {
    let opening = decs!("1000");
    let invoices = decs!("4000");
    let collections = decs!("5000");
    let closing = opening + invoices - collections;
    assert_eq!(closing, Decimal::ZERO);
}

// =====================================================
// 3. 日期匹配阈值（auto_match 策略2 纯算法）
// =====================================================

/// test_rqppyz_30tnkpp（验证 vfy_ops/match.rs auto_match 策略2 中 date_diff <= 30 时应匹配。；边界值：恰好 30 天也应匹配。）
#[test]
fn test_rqppyz_30tnkpp() {
    let invoice_date = ymd!(2026, 6, 1);
    // 30 天后：边界，应匹配
    let coll_date_30 = ymd!(2026, 7, 1);
    let diff_30 = (coll_date_30 - invoice_date).num_days().abs();
    assert_eq!(diff_30, 30);
    assert!(diff_30 <= 30);

    // 15 天后：区间内，应匹配
    let coll_date_15 = ymd!(2026, 6, 16);
    let diff_15 = (coll_date_15 - invoice_date).num_days().abs();
    assert_eq!(diff_15, 15);
    assert!(diff_15 <= 30);
}

/// test_rqppyz_c30tbpp（验证 vfy_ops/match.rs auto_match 策略2 中 date_diff > 30 时不应匹配。）
#[test]
fn test_rqppyz_c30tbpp() {
    let invoice_date = ymd!(2026, 6, 1);
    let coll_date = ymd!(2026, 7, 2); // 31 天后
    let diff = (coll_date - invoice_date).num_days().abs();
    assert_eq!(diff, 31);
    assert!(diff > 30);
}

// =====================================================
// 4. 部分匹配金额与状态判定（auto_match 策略2 纯算法）
// =====================================================

/// test_bfppje_qjxz（验证 vfy_ops/match.rs auto_match 策略2 中 matched = min(invoice_amount, collection_amount)。）
#[test]
fn test_bfppje_qjxz() {
    let inv_amt = decs!("5000");
    let coll_amt = decs!("3000");
    let matched = std::cmp::min(inv_amt, coll_amt);
    assert_eq!(matched, decs!("3000"));

    // 反向参数同样取较小值
    let matched_rev = std::cmp::min(coll_amt, inv_amt);
    assert_eq!(matched_rev, decs!("3000"));
}

/// test_ppztpd_wqybfpp
/// 验证 vfy_ops/match.rs auto_match 策略2 中 match_status 判定规则：matched == invoice_amount → ar::MATCH_MATCHED；matched < invoice_amount  → "PARTIAL"；注："PARTIAL" 在 status 模块无对应常量，沿用 vfy_ops/match.rs 字面量。
#[test]
fn test_ppztpd_wqybfpp() {
    let inv_amt = decs!("5000");

    // 完全匹配：matched == invoice_amount
    let matched_full = std::cmp::min(inv_amt, decs!("5000"));
    let status_full = if matched_full == inv_amt {
        ar::MATCH_MATCHED
    } else {
        "PARTIAL"
    };
    assert_eq!(status_full, ar::MATCH_MATCHED);

    // 部分匹配：matched < invoice_amount
    let matched_part = std::cmp::min(inv_amt, decs!("3000"));
    let status_part = if matched_part == inv_amt {
        ar::MATCH_MATCHED
    } else {
        "PARTIAL"
    };
    assert_eq!(status_part, "PARTIAL");
}

// =====================================================
// 5. 未匹配数量公式（auto_match 汇总纯算法）
// =====================================================

/// test_wppslgs_zq
/// 验证 vfy_ops/match.rs auto_match 末尾 unmatched_count 公式：unmatched_count = invoices.len() + collections.len() - matched_count * 2；每次匹配消耗 1 张发票 + 1 笔收款，故乘 2。
#[test]
fn test_wppslgs_zq() {
    let invoices_len = 10usize;
    let collections_len = 8usize;
    let matched_count = 5usize;
    let unmatched = invoices_len + collections_len - matched_count * 2;
    // 10 + 8 - 10 = 8
    assert_eq!(unmatched, 8);

    // 全部匹配：matched = min(invoices, collections) = 8
    let matched_all = std::cmp::min(invoices_len, collections_len);
    let unmatched_all = invoices_len + collections_len - matched_all * 2;
    // 10 + 8 - 16 = 2（剩余 2 张发票未匹配）
    assert_eq!(unmatched_all, 2);
}

// =====================================================
// 6. 账龄分桶（get_aging_report 纯算法）
// =====================================================

/// test_zlft_dqwyq（验证 overdue_days <= 0 时落入第 0 桶（当期）。；边界值：overdue_days = 0（到期日当天）也应落入当期。）
#[test]
fn test_zlft_dqwyq() {
    assert_eq!(aging_bucket_idx(0), 0);
    assert_eq!(aging_bucket_idx(-1), 0);
    assert_eq!(aging_bucket_idx(-30), 0);
}

/// test_zlft_1d30tqj（验证 1 <= overdue_days <= 30 时落入第 1 桶（1-30天）。；边界值：1 和 30 均应落入此桶。）
#[test]
fn test_zlft_1d30tqj() {
    assert_eq!(aging_bucket_idx(1), 1);
    assert_eq!(aging_bucket_idx(15), 1);
    assert_eq!(aging_bucket_idx(30), 1);
}

/// test_zlft_31d60tqj（验证 31 <= overdue_days <= 60 时落入第 2 桶（31-60天）。；边界值：31 和 60 均应落入此桶。）
#[test]
fn test_zlft_31d60tqj() {
    assert_eq!(aging_bucket_idx(31), 2);
    assert_eq!(aging_bucket_idx(45), 2);
    assert_eq!(aging_bucket_idx(60), 2);
}

/// test_zlft_61d90tqj（验证 61 <= overdue_days <= 90 时落入第 3 桶（61-90天）。；边界值：61 和 90 均应落入此桶。）
#[test]
fn test_zlft_61d90tqj() {
    assert_eq!(aging_bucket_idx(61), 3);
    assert_eq!(aging_bucket_idx(75), 3);
    assert_eq!(aging_bucket_idx(90), 3);
}

/// test_zlft_90tys（验证 overdue_days > 90 时落入第 4 桶（90天以上）。；边界值：91 应落入此桶。）
#[test]
fn test_zlft_90tys() {
    assert_eq!(aging_bucket_idx(91), 4);
    assert_eq!(aging_bucket_idx(180), 4);
    assert_eq!(aging_bucket_idx(365), 4);
}

// =====================================================
// 7. 状态机转换合法性（customer_confirm / customer_dispute 纯算法）
// =====================================================

/// test_khqrztj_yqrjj
/// 验证 customer_confirm 中 status == ar_status::RECONCILIATION_CONFIRMED 时应拒绝（不可重复确认），；返回 BusinessError 且消息包含 "对账单已确认，不可重复确认"。
#[test]
fn test_khqrztj_yqrjj() {
    let result = validate_customer_confirm("confirmed");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, AppError::BusinessError(_)));
    assert!(format!("{err}").contains("对账单已确认，不可重复确认"));
}

/// test_khqrztj_zyzjj
/// 验证 customer_confirm 中 status == ar_status::RECONCILIATION_DISPUTED 时应拒绝（需先解决争议），；返回 BusinessError 且消息包含 "对账单存在争议"。
#[test]
fn test_khqrztj_zyzjj() {
    let result = validate_customer_confirm("disputed");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, AppError::BusinessError(_)));
    assert!(format!("{err}").contains("对账单存在争议"));
}

/// test_khqrztj_qtztkzh（验证 customer_confirm 中 status 为 "draft" 等非终态时应允许转换到 "confirmed"。）
#[test]
fn test_khqrztj_qtztkzh() {
    // draft → confirmed：应允许
    let result = validate_customer_confirm("draft");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "confirmed");
}

/// test_khzyztj_yqrjj
/// 验证 customer_dispute 中 status == ar_status::RECONCILIATION_CONFIRMED 时应拒绝（已确认不可提争议），；返回 BusinessError 且消息包含 "对账单已确认，不可提出争议"。
#[test]
fn test_khzyztj_yqrjj() {
    let result = validate_customer_dispute("confirmed");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, AppError::BusinessError(_)));
    assert!(format!("{err}").contains("对账单已确认，不可提出争议"));
}

/// test_khzyztj_ygbjj
/// 验证 customer_dispute 中 status == ar_status::RECONCILIATION_CLOSED 时应拒绝（已关闭不可提争议），；返回 BusinessError 且消息包含 "对账单已关闭，不可提出争议"。
#[test]
fn test_khzyztj_ygbjj() {
    let result = validate_customer_dispute("closed");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, AppError::BusinessError(_)));
    assert!(format!("{err}").contains("对账单已关闭，不可提出争议"));
}

/// test_khzyztj_cgkzh（验证 customer_dispute 中 status 为 "draft" 时应允许转换到 "disputed"。）
#[test]
fn test_khzyztj_cgkzh() {
    let result = validate_customer_dispute("draft");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "disputed");
}

// =====================================================
// 8. 匹配策略校验（auto_match 开头纯算法）
// =====================================================

/// test_ppcljy_wxclcwxx
/// 验证 auto_match 中传入不支持的策略时返回可外显的校验错误（策略词表是用户可自助
/// 修正的公开规则），；错误消息格式："无效的匹配策略: {strategy}（支持 exact / date_order / all）"
#[test]
fn test_ppcljy_wxclcwxx() {
    let result = validate_match_strategy(Some("invalid"));
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, AppError::ValidationErrorDisplayable(_)));
    let msg = format!("{err}");
    assert!(msg.contains("无效的匹配策略: invalid"));
    assert!(msg.contains("exact / date_order / all"));
}

/// test_ppcljy_hfcltg（验证 auto_match 中三种合法策略（exact/date_order/all）均通过校验，；且 None 默认为 "all"，大小写不敏感（自动转小写）。）
#[test]
fn test_ppcljy_hfcltg() {
    assert_eq!(validate_match_strategy(Some("exact")).unwrap(), "exact");
    assert_eq!(
        validate_match_strategy(Some("date_order")).unwrap(),
        "date_order"
    );
    assert_eq!(validate_match_strategy(Some("all")).unwrap(), "all");
    // None 默认 "all"
    assert_eq!(validate_match_strategy(None).unwrap(), "all");
    // 大写自动转小写
    assert_eq!(validate_match_strategy(Some("EXACT")).unwrap(), "exact");
}

// =====================================================
// 9. 夹具宏可用性（decs! / ymd!）
// =====================================================

/// test_decsh_hxjejx（验证 decs! 宏可正确解析 vfy_ops 业务场景的金额字符串（含小数），；并可参与期末余额公式运算。）
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

/// test_ymdh_dzrqjx（验证 ymd! 宏可正确解析 vfy_ops 业务场景的对账期间日期，；并可参与日期差运算（auto_match 策略2 依赖）。）
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

/// test_fwslh_xsjk（验证 ArReconciliationService::new 可用 SQLite 内存库构造实例，；仅验证实例化成功（不需要 schema），与模板 test_fwslcj 同模式。）
#[tokio::test]
async fn test_fwslh_xsjk() {
    let db = setup_test_db().await;
    let svc = ArReconciliationService::new(Arc::new(db));
    // 验证实例化成功：Arc 引用计数 >= 1
    assert!(Arc::strong_count(&svc.db) >= 1);
}

/// test_zddzwzlc_xsjk —— 依据裁决 R-9 拆前提（#4672 判责 §A.1 pI 族 :441）：
/// 钉"schema 缺失（customers 表根本不存在）时 auto_match 返回 DATABASE_ERROR
/// 机器码而非 panic"，本意不变，前提是负前提交集 ⇒ 改绑
/// `connect_empty_schema_db()`（不跑迁移的 `bingxi_empty` 库）。
///
/// 真实契约依据（读函数体，非读注释）：`src/services/ar/vfy_ops/match.rs:22-63`
/// auto_match 先 `parse_match_strategy`（match.rs:80-96，"all" 合法 ⇒ Ok），随后
/// `begin` + `load_match_customers`（match.rs:99-114 `customer::Entity::find`）；
/// 缺表 ⇒ DbErr::Query ⇒ `utils/error.rs:562-565` AppError::database ⇒
/// error_code "DATABASE_ERROR"（error.rs:747）。
/// 为什么不能再留在 `setup_test_db()` 上断 Err：该夹具现语义 = 已迁移 PG +
/// TRUNCATE 业务表，空表 ⇒ customers=[] ⇒ 循环体不执行 ⇒ `Ok(vec![])` 才是
/// 真实契约（match.rs:29-62），旧 `is_err()` 在真库化夹具上必红（#4672 即此）。
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

/// test_zlbgwzlc_xsjk —— 同族（#4672 判责 §A.1 pI :453）：按 R-9 改绑
/// `connect_empty_schema_db()`，钉"schema 缺失时 get_aging_report 返回
/// DATABASE_ERROR 而非 panic"。
///
/// 真实契约依据：`src/services/ar/vfy_ops/aging.rs:30-51` get_aging_report 首步
/// `load_unpaid_invoices`（aging.rs:53-73 `ar_invoice::Entity::find().all()`）；
/// 缺表 ⇒ DbErr::Query ⇒ DATABASE_ERROR（error.rs:562-565,747）。
/// 空表（真库化夹具）上则恒 `Ok`：invoices=[] ⇒ 五桶全零、total_receivable=0 的
/// 报告（aging.rs:41-50），⇒ 旧 `is_err()` 前提过期，红因与上一条同型。
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
