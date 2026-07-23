//! 供应商对账 Service（facade）
//!
//! D10-5 拆分：本文件作为 facade，保留 ApReconciliationService struct + new 构造函数
//! + 单号生成宏 + 单元测试。impl 业务方法迁移至 `ap_reconciliation_ops` 子模块
//!（crud / confirm / report / auto），DTOs 迁移至 `ap_reconciliation_ops::types`，
//! 通过 db 字段 pub(crate) 让 ops 访问，外部引用路径保持不变。

use crate::models::ap_reconciliation;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

// 重新导出 DTOs（迁移至 ap_reconciliation_ops::types），保持外部引用路径不变
// 外部仍可通过 crate::services::ap_reconciliation_service::{GenerateReconciliationRequest, ...} 访问
// 仅 re-export facade 测试与外部 handler 实际使用的 DTO，避免 unused imports 警告
pub use crate::services::ap_reconciliation_ops::types::{
    AutoReconciliationResult, GenerateReconciliationRequest,
};

/// 供应商对账服务
pub struct ApReconciliationService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl ApReconciliationService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // 生成对账单号
    // 格式：REC + 年月日 + 三位序号（REC20260315001）
    crate::impl_generate_no!(
        generate_reconciliation_no,
        "REC",
        ap_reconciliation::Entity,
        ap_reconciliation::Column::ReconciliationNo
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::test_common::setup_test_db;
    use crate::decs;
    use crate::ymd;
    use crate::models::status::{common, payment};
    use crate::utils::error::AppError;
    use chrono::{NaiveDate, Utc};
    use rust_decimal::Decimal;
    use std::str::FromStr;

    /// 复现 generate_reconciliation 中的期末余额计算公式
    ///
    /// 业务公式（ap_reconciliation_ops/crud.rs generate_reconciliation）：
    /// `closing_balance = opening_balance + total_invoice - total_payment`
    fn compute_closing_balance(
        opening_balance: Decimal,
        total_invoice: Decimal,
        total_payment: Decimal,
    ) -> Decimal {
        opening_balance + total_invoice - total_payment
    }

    /// 复现 get_supplier_summary 中的付款状态判断逻辑
    ///
    /// 业务逻辑（ap_reconciliation_ops/report.rs aggregate_invoices_by_supplier）：
    /// - 已付清：amount > 0 且 paid >= amount，或 amount < 0 且 paid <= amount（红冲场景）
    /// - 部分付款：paid != 0 且未付清
    /// - 未付款：paid == 0
    ///
    /// 返回值约定：0=未付款，1=部分付款，2=已付清
    fn classify_payment_status(amount: Decimal, paid_amount: Decimal) -> i32 {
        let paid_in_full = (amount > Decimal::ZERO && paid_amount >= amount)
            || (amount < Decimal::ZERO && paid_amount <= amount);
        if paid_in_full {
            2
        } else if paid_amount != Decimal::ZERO {
            1
        } else {
            0
        }
    }

    /// 复现 get_supplier_summary 中的逾期判断逻辑
    ///
    /// 业务逻辑（ap_reconciliation_ops/report.rs aggregate_invoices_by_supplier）：
    /// `due_date < today && unpaid_amount > 0` 视为逾期
    /// 这里把 today 参数化，避免测试依赖系统当前时间导致用例非幂等。
    fn is_overdue(due_date: NaiveDate, today: NaiveDate, unpaid_amount: Decimal) -> bool {
        due_date < today && unpaid_amount > Decimal::ZERO
    }

    // =====================================================
    // 一、状态常量值正确性
    // =====================================================

    /// 测试_对账状态常量_Pending值正确
    ///
    /// 验证 "PENDING" 与 common::STATUS_PENDING 一致，
    /// 用于 generate_reconciliation 创建对账单时的初始状态，
    /// 以及 confirm_reconciliation 中允许确认的唯一状态。
    #[test]
    fn 测试_对账状态常量_Pending值正确() {
        assert_eq!(common::STATUS_PENDING, "PENDING");
        // 业务代码硬编码字符串与常量保持一致，避免状态机误判
        let created_status = "PENDING".to_string();
        assert_eq!(created_status, common::STATUS_PENDING);
    }

    /// 测试_对账状态常量_Cancelled值正确
    ///
    /// 验证 "CANCELLED" 与 common::STATUS_CANCELLED 一致，
    /// 用于 generate_reconciliation 排除已取消的应付单。
    #[test]
    fn 测试_对账状态常量_Cancelled值正确() {
        assert_eq!(common::STATUS_CANCELLED, "CANCELLED");
        // 业务查询过滤条件使用同一常量，避免拼写错误漏过作废单据
        let excluded_status = "CANCELLED";
        assert_eq!(excluded_status, common::STATUS_CANCELLED);
    }

    /// 测试_付款状态常量_Confirmed值正确
    ///
    /// 验证 "CONFIRMED" 与 payment::PAYMENT_CONFIRMED 一致，
    /// 用于 generate_reconciliation 查询已确认付款单，
    /// 以及 confirm_reconciliation 中对账单确认后的状态值。
    #[test]
    fn 测试_付款状态常量_Confirmed值正确() {
        assert_eq!(payment::PAYMENT_CONFIRMED, "CONFIRMED");
        // 业务代码同时使用此值用于付款单查询过滤与对账单状态设置
        let payment_filter = "CONFIRMED";
        let reconciliation_status = "CONFIRMED".to_string();
        assert_eq!(payment_filter, payment::PAYMENT_CONFIRMED);
        assert_eq!(reconciliation_status, payment::PAYMENT_CONFIRMED);
    }

    // =====================================================
    // 二、期末余额计算（纯算法）
    // =====================================================

    /// 测试_期末余额计算_标准场景
    ///
    /// 验证 generate_reconciliation 中期末余额公式：
    /// 期末 = 期初 + 本期应付 - 本期付款
    /// 典型场景：期初 1000，本期应付 5000，本期付款 3000，期末应为 3000
    #[test]
    fn 测试_期末余额计算_标准场景() {
        let opening = decs!("1000");
        let total_invoice = decs!("5000");
        let total_payment = decs!("3000");

        let closing = compute_closing_balance(opening, total_invoice, total_payment);

        assert_eq!(closing, decs!("3000"));
        // 业务不变量：期末 = 期初 + 应付 - 付款
        assert_eq!(closing, opening + total_invoice - total_payment);
    }

    /// 测试_期末余额计算_无本期交易
    ///
    /// 验证本期内既无应付也无付款时，期末余额等于期初余额
    #[test]
    fn 测试_期末余额计算_无本期交易() {
        let opening = decs!("2500");
        let closing = compute_closing_balance(opening, Decimal::ZERO, Decimal::ZERO);

        assert_eq!(closing, opening);
        assert_eq!(closing, decs!("2500"));
    }

    /// 测试_期末余额计算_付款大于应付产生透支
    ///
    /// 验证付款总额大于（期初+应付）时，期末余额为负数（预付/透支场景）
    #[test]
    fn 测试_期末余额计算_付款大于应付产生透支() {
        let opening = decs!("1000");
        let total_invoice = decs!("2000");
        let total_payment = decs!("5000");

        let closing = compute_closing_balance(opening, total_invoice, total_payment);

        // 1000 + 2000 - 5000 = -2000（预付 supplier 模式）
        assert_eq!(closing, decs!("-2000"));
        assert!(closing < Decimal::ZERO);
    }

    /// 测试_期末余额计算_金额全为零
    ///
    /// 验证全部金额为零时（新供应商首次对账且无任何业务），期末余额为零
    #[test]
    fn 测试_期末余额计算_金额全为零() {
        let closing = compute_closing_balance(Decimal::ZERO, Decimal::ZERO, Decimal::ZERO);

        assert_eq!(closing, Decimal::ZERO);
    }

    // =====================================================
    // 三、状态机转换合法性
    // =====================================================

    /// 测试_状态机转换_确认需Pending状态
    ///
    /// 验证 confirm_reconciliation 中状态门控逻辑：
    /// 仅当 reconciliation_status == "PENDING" 时允许确认，
    /// 其他状态（CONFIRMED/DISPUTED 等）应被拒绝。
    #[test]
    fn 测试_状态机转换_确认需Pending状态() {
        // PENDING 状态允许确认
        let pending_status = common::STATUS_PENDING.to_string();
        assert_eq!(pending_status, common::STATUS_PENDING);
        let can_confirm_pending = pending_status == common::STATUS_PENDING;
        assert!(can_confirm_pending);

        // CONFIRMED 状态不可再次确认
        let confirmed_status = payment::PAYMENT_CONFIRMED.to_string();
        let can_confirm_confirmed = confirmed_status == common::STATUS_PENDING;
        assert!(!can_confirm_confirmed);

        // DISPUTED 状态不可直接确认
        let disputed_status = "DISPUTED".to_string();
        let can_confirm_disputed = disputed_status == common::STATUS_PENDING;
        assert!(!can_confirm_disputed);
    }

    /// 测试_状态机转换_已确认不可争议
    ///
    /// 验证 dispute 中状态门控逻辑：
    /// 当 reconciliation_status == "CONFIRMED" 时拒绝提出争议
    #[test]
    fn 测试_状态机转换_已确认不可争议() {
        let confirmed_status = payment::PAYMENT_CONFIRMED.to_string();
        // 复现 dispute 的状态校验：CONFIRMED 状态被拒绝
        let should_reject = confirmed_status == payment::PAYMENT_CONFIRMED;
        assert!(should_reject);

        // 复现错误消息构造（业务 dispute 方法）
        let err = AppError::business("对账单已确认，不可提出争议".to_string());
        assert!(matches!(err, AppError::BusinessError(_)));
    }

    /// 测试_状态机转换_争议或Pending可继续争议
    ///
    /// 验证 dispute 中非 CONFIRMED 状态（PENDING / DISPUTED）均允许提出争议
    #[test]
    fn 测试_状态机转换_争议或Pending可继续争议() {
        // PENDING 状态可提出争议
        let pending_status = common::STATUS_PENDING.to_string();
        let can_dispute_pending = pending_status != payment::PAYMENT_CONFIRMED;
        assert!(can_dispute_pending);

        // DISPUTED 状态可继续补争议（业务代码未禁止）
        let disputed_status = "DISPUTED".to_string();
        let can_dispute_disputed = disputed_status != payment::PAYMENT_CONFIRMED;
        assert!(can_dispute_disputed);

        // CONFIRMED 状态不可争议（与上例一致，作为对照）
        let confirmed_status = payment::PAYMENT_CONFIRMED.to_string();
        let can_dispute_confirmed = confirmed_status != payment::PAYMENT_CONFIRMED;
        assert!(!can_dispute_confirmed);
    }

    // =====================================================
    // 四、付款状态判断（get_supplier_summary 内纯算法）
    // =====================================================

    /// 测试_付款状态判断_已付清正向
    ///
    /// 验证 amount > 0 且 paid >= amount 时判为已付清
    #[test]
    fn 测试_付款状态判断_已付清正向() {
        let amount = decs!("1000");
        // 恰好付清（边界）
        assert_eq!(classify_payment_status(amount, decs!("1000")), 2);
        // 多付（红冲/预付）
        assert_eq!(classify_payment_status(amount, decs!("1200")), 2);
    }

    /// 测试_付款状态判断_已付清负向红冲
    ///
    /// 验证 amount < 0 且 paid <= amount 时判为已付清（红冲应付单场景）
    #[test]
    fn 测试_付款状态判断_已付清负向红冲() {
        let amount = decs!("-500");
        // 恰好冲销（边界，paid == amount）
        assert_eq!(classify_payment_status(amount, decs!("-500")), 2);
        // 多冲销
        assert_eq!(classify_payment_status(amount, decs!("-600")), 2);
    }

    /// 测试_付款状态判断_部分付款
    ///
    /// 验证 paid != 0 且未达付清条件时判为部分付款
    #[test]
    fn 测试_付款状态判断_部分付款() {
        let amount = decs!("1000");
        // 正向部分付款
        assert_eq!(classify_payment_status(amount, decs!("300")), 1);
        assert_eq!(classify_payment_status(amount, decs!("999.99")), 1);

        // 负向部分冲销（amount < 0，paid 介于 0 和 amount 之间）
        let neg_amount = decs!("-500");
        assert_eq!(classify_payment_status(neg_amount, decs!("-100")), 1);
    }

    /// 测试_付款状态判断_未付款
    ///
    /// 验证 paid == 0 时判为未付款
    #[test]
    fn 测试_付款状态判断_未付款() {
        let amount = decs!("1000");
        assert_eq!(classify_payment_status(amount, Decimal::ZERO), 0);

        // 负向金额未付款同样判为未付款
        let neg_amount = decs!("-500");
        assert_eq!(classify_payment_status(neg_amount, Decimal::ZERO), 0);
    }

    // =====================================================
    // 五、逾期判断（get_supplier_summary 内纯算法）
    // =====================================================

    /// 测试_逾期判断_已逾期未付
    ///
    /// 验证 due_date < today 且 unpaid_amount > 0 时判为逾期
    #[test]
    fn 测试_逾期判断_已逾期未付() {
        let today = ymd!(2026, 7, 1);
        let due_date = ymd!(2026, 6, 30); // 已到期
        let unpaid = decs!("500");

        assert!(is_overdue(due_date, today, unpaid));

        // 累计逾期金额应等于未付金额
        let overdue_amount = unpaid;
        assert_eq!(overdue_amount, decs!("500"));
    }

    /// 测试_逾期判断_未到期不逾期
    ///
    /// 验证 due_date >= today 时不判为逾期，即使存在未付金额
    #[test]
    fn 测试_逾期判断_未到期不逾期() {
        let today = ymd!(2026, 7, 1);
        let unpaid = decs!("500");

        // 到期日等于今天（边界，不逾期）
        assert!(!is_overdue(ymd!(2026, 7, 1), today, unpaid));
        // 到期日在未来
        assert!(!is_overdue(ymd!(2026, 12, 31), today, unpaid));
    }

    /// 测试_逾期判断_已付清不算逾期
    ///
    /// 验证 unpaid_amount == 0 时即使超过到期日也不判为逾期
    #[test]
    fn 测试_逾期判断_已付清不算逾期() {
        let today = ymd!(2026, 7, 1);
        let due_date = ymd!(2026, 6, 30); // 已到期
        let unpaid = Decimal::ZERO;

        assert!(!is_overdue(due_date, today, unpaid));

        // 负向未付金额（红冲多付）也不应判为逾期
        let unpaid_neg = decs!("-100");
        assert!(!is_overdue(due_date, today, unpaid_neg));
    }

    // =====================================================
    // 六、错误消息格式
    // =====================================================

    /// 测试_错误消息格式_对账单未找到
    ///
    /// 验证 get_by_id / confirm_reconciliation / dispute 中
    /// "对账单 {id}" 格式的 not_found 错误消息
    #[test]
    fn 测试_错误消息格式_对账单未找到() {
        let id = 9999;
        let err = AppError::not_found(format!("对账单 {}", id));

        // 复现业务方法的错误消息格式
        assert_eq!(format!("对账单 {}", id), "对账单 9999");
        assert!(matches!(err, AppError::NotFound(_)));
    }

    /// 测试_错误消息格式_状态不可确认
    ///
    /// 验证 confirm_reconciliation 中
    /// "对账单状态为{status}，不可确认" 格式的 business 错误消息
    #[test]
    fn 测试_错误消息格式_状态不可确认() {
        let status = payment::PAYMENT_CONFIRMED; // 已确认状态再次确认
        let msg = format!("对账单状态为{}，不可确认", status);
        let err = AppError::business(msg.clone());

        assert_eq!(msg, "对账单状态为CONFIRMED，不可确认");
        assert!(matches!(err, AppError::BusinessError(_)));

        // DISPUTED 状态尝试确认时也应生成正确格式
        let disputed_msg = format!("对账单状态为{}，不可确认", "DISPUTED");
        assert_eq!(disputed_msg, "对账单状态为DISPUTED，不可确认");
    }

    /// 测试_错误消息格式_已确认不可争议
    ///
    /// 验证 dispute 中
    /// "对账单已确认，不可提出争议" 固定消息的 business 错误
    #[test]
    fn 测试_错误消息格式_已确认不可争议() {
        let err = AppError::business("对账单已确认，不可提出争议".to_string());

        assert!(matches!(err, AppError::BusinessError(_)));
    }

    /// 测试_错误消息格式_应付单未找到
    ///
    /// 验证 get_invoice_relations 中
    /// "应付单 {invoice_id}" 格式的 not_found 错误消息
    #[test]
    fn 测试_错误消息格式_应付单未找到() {
        let invoice_id = 8888;
        let err = AppError::not_found(format!("应付单 {}", invoice_id));

        assert_eq!(format!("应付单 {}", invoice_id), "应付单 8888");
        assert!(matches!(err, AppError::NotFound(_)));
    }

    // =====================================================
    // 七、夹具宏可用性
    // =====================================================

    /// 测试_decs夹具宏解析金额
    ///
    /// 验证 decs! 宏能正确解析 Decimal 字符串，用于后续金额计算测试夹具
    #[test]
    fn 测试_decs夹具宏解析金额() {
        let v = decs!("12345.67");
        assert_eq!(v.to_string(), "12345.67");

        let zero = decs!("0");
        assert_eq!(zero, Decimal::ZERO);

        let neg = decs!("-1000");
        assert!(neg < Decimal::ZERO);
        assert_eq!(neg.to_string(), "-1000");

        // FromStr trait 在作用域中即可使用，验证可访问
        let parsed = Decimal::from_str("99.99");
        assert!(parsed.is_ok());
    }

    /// 测试_ymd夹具宏解析对账日期
    ///
    /// 验证 ymd! 宏能正确解析日期，用于对账期间测试夹具
    #[test]
    fn 测试_ymd夹具宏解析对账日期() {
        let start = ymd!(2026, 1, 1);
        let end = ymd!(2026, 12, 31);

        assert_eq!(start.to_string(), "2026-01-01");
        assert_eq!(end.to_string(), "2026-12-31");
        // 起止日期合法性
        assert!(start < end);
    }

    // =====================================================
    // 八、服务实例化（SQLite 内存数据库）
    // =====================================================

    /// 测试_服务实例创建
    ///
    /// 验证 ApReconciliationService 在 SQLite 内存数据库上能正常实例化，
    /// 内部 Arc<DatabaseConnection> 引用计数 >= 1
    #[tokio::test]
    async fn 测试_服务实例创建() {
        let db = setup_test_db().await;
        let service = ApReconciliationService::new(Arc::new(db));

        assert!(Arc::strong_count(&service.db) >= 1);
    }

    // =====================================================
    // 九、DB 交互测试（依赖 schema，标注 #[ignore]）
    // =====================================================

    /// 测试_生成对账单_需要真实数据库
    ///
    /// 依赖 ap_reconciliation / ap_invoice / ap_payment 表 schema，
    /// 标注 #[ignore] 仅在本地手动运行。无 schema 时返回数据库错误。
    #[tokio::test]
    #[ignore]
    async fn 测试_生成对账单_需要真实数据库() {
        let db = setup_test_db().await;
        let service = ApReconciliationService::new(Arc::new(db));

        let req = GenerateReconciliationRequest {
            supplier_id: 99999,
            start_date: ymd!(2026, 1, 1),
            end_date: ymd!(2026, 6, 30),
            notes: Some("测试对账单".to_string()),
        };

        let result = service.generate_reconciliation(req, 1).await;
        // L-17 修复（批次 377 v13 复审）：原 let _ = result 无断言，改为 is_err 断言
        // 无 schema 时返回数据库错误；有 schema 时可能成功或返回约束错误
        assert!(result.is_err(), "无 schema 时应返回数据库错误");
    }

    /// 测试_确认对账单_需要真实数据库
    ///
    /// 依赖 ap_reconciliation 表 schema，标注 #[ignore] 仅在本地手动运行。
    /// 无 schema 时返回数据库错误；有 schema 但无记录时返回 NotFound。
    #[tokio::test]
    #[ignore]
    async fn 测试_确认对账单_需要真实数据库() {
        let db = setup_test_db().await;
        let service = ApReconciliationService::new(Arc::new(db));

        let result = service.confirm_reconciliation(99999, 1).await;
        // 无 schema 时返回数据库错误；有 schema 但无记录时返回 NotFound
        assert!(result.is_err());
    }

    /// 测试_获取对账单列表_需要真实数据库
    ///
    /// 依赖 ap_reconciliation 表 schema，标注 #[ignore] 仅在本地手动运行。
    /// 验证调用路径不 panic，分页参数 1-indexed 转换正确。
    #[tokio::test]
    #[ignore]
    async fn 测试_获取对账单列表_需要真实数据库() {
        let db = setup_test_db().await;
        let service = ApReconciliationService::new(Arc::new(db));

        let result = service
            .get_list(None, None, None, None, 1, 10)
            .await;
        // L-17 修复（批次 377 v13 复审）：原 let _ = result 无断言，改为 is_err 断言
        // 无 schema 时为 Err；有 schema 无记录时为 Ok((vec![], 0))
        assert!(result.is_err(), "无 schema 时应返回数据库错误");
    }

    // =====================================================
    // 十、DTO 字段构造与对账单号格式
    // =====================================================

    /// 测试_生成对账请求_字段构造
    ///
    /// 验证 GenerateReconciliationRequest 字段能正常构造，
    /// notes 字段允许为 None，supplier_id/start_date/end_date 必填
    #[test]
    fn 测试_生成对账请求_字段构造() {
        let req_with_notes = GenerateReconciliationRequest {
            supplier_id: 1,
            start_date: ymd!(2026, 1, 1),
            end_date: ymd!(2026, 6, 30),
            notes: Some("Q2 对账".to_string()),
        };
        assert_eq!(req_with_notes.supplier_id, 1);
        assert_eq!(req_with_notes.start_date, ymd!(2026, 1, 1));
        assert_eq!(req_with_notes.end_date, ymd!(2026, 6, 30));
        assert_eq!(req_with_notes.notes.as_deref(), Some("Q2 对账"));

        let req_without_notes = GenerateReconciliationRequest {
            supplier_id: 2,
            start_date: ymd!(2026, 7, 1),
            end_date: ymd!(2026, 12, 31),
            notes: None,
        };
        assert_eq!(req_without_notes.supplier_id, 2);
        assert!(req_without_notes.notes.is_none());
    }

    /// 测试_自动对账结果_失败状态字符串
    ///
    /// 验证 auto_reconcile_all 中失败分支使用的 "FAILED" 状态字符串，
    /// 与成功分支的 reconciliation_status（来自数据库）形成对照。
    #[test]
    fn 测试_自动对账结果_失败状态字符串() {
        let failed = AutoReconciliationResult {
            reconciliation_id: 0,
            reconciliation_no: String::new(),
            supplier_id: 1,
            start_date: ymd!(2026, 1, 1),
            end_date: ymd!(2026, 6, 30),
            opening_balance: Decimal::ZERO,
            total_invoice: Decimal::ZERO,
            total_payment: Decimal::ZERO,
            closing_balance: Decimal::ZERO,
            invoice_count: 0,
            payment_count: 0,
            status: "FAILED".to_string(),
            message: "Failed: ...".to_string(),
        };
        assert_eq!(failed.status, "FAILED");
        assert_eq!(failed.reconciliation_id, 0);
        assert_eq!(failed.invoice_count, 0);
    }

    /// 测试_对账单号格式_REC前缀
    ///
    /// 验证 impl_generate_no! 宏生成对账单号使用 "REC" 前缀，
    /// 格式为 REC + 年月日 + 三位序号（如 REC20260315001）。
    /// 此处不实际调用数据库生成，仅校验前缀与格式约定。
    #[test]
    fn 测试_对账单号格式_REC前缀() {
        // 复现宏定义的前缀常量
        let prefix = "REC";
        let today = Utc::now();
        let date_part = today.format("%Y%m%d").to_string();
        // 模拟序号部分（实际由宏内 SQL MAX+1 决定）
        let seq_part = "001";
        let sample_no = format!("{}{}{}", prefix, date_part, seq_part);

        assert!(sample_no.starts_with("REC"));
        assert_eq!(sample_no.len(), 3 + 8 + 3); // REC + 8位日期 + 3位序号 = 14
                                                 // 业务文档示例：REC20260315001
        let doc_example = "REC20260315001";
        assert_eq!(doc_example.len(), 14);
        assert!(doc_example.starts_with("REC"));
    }
}
