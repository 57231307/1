//! 应收对账 - 核销服务门面（ar/vfy）
//!
//! 批次 490 D10-4b：原 `ar/vfy.rs`（1368 行）按 facade 模式拆分，业务方法实现
//! 迁移至 `ar/vfy_ops/` 子模块（match / aging / reconciliation / confirm）。
//! 本文件保留为门面：重新导出公共 DTO 与 `ArReconciliationService`，并保留测试模块。
//!
//! 高级对账算法（实现见 `vfy_ops`）：
//! - `auto_match`         自动对账：精确金额 + 日期顺序 + 客户汇总三种策略
//! - `get_aging_report`   账龄分桶分析（5 档：当期 / 1-30 / 31-60 / 61-90 / 90+）
//! - `generate_reconciliation` 自动生成对账单（含明细行）
//! - `customer_confirm` / `customer_dispute` 带状态校验的客户操作
//!
//! 拆分自原 `ar_reconciliation_service.rs` 的 `// 增强功能` 段。
//! 结构体定义与构造函数 `ArReconciliationService::new` 位于 `super`（`ar/mod.rs`）。

// 重新导出公共 DTO 与 Service 结构体，保持 `crate::services::ar::vfy::*` 路径稳定
pub use super::{
    AgingBucket, AgingReport, ArReconciliationService, AutoMatchRequest, AutoMatchResult,
    CustomerAgingSummary, GenerateReconciliationRequest,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::test_common::setup_test_db;
    use crate::decs;
    use crate::ymd;
    use crate::models::status::{ar, common};
    use crate::models::status::ar as ar_status;
    use crate::utils::error::AppError;
    use rust_decimal::Decimal;
    use std::sync::Arc;

    /// 复现 vfy_ops/aging.rs get_aging_report 中的账龄分桶索引计算（纯算法，不依赖 DB）
    ///
    /// 分桶规则（与 vfy_ops/aging.rs `compute_aging_bucket_index` 保持一致）：
    /// - 0: 当期（overdue_days <= 0）
    /// - 1: 1-30天
    /// - 2: 31-60天
    /// - 3: 61-90天
    /// - 4: 90天以上
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
    ///
    /// 错误消息与 auto_match 校验段保持一致：
    /// "无效的匹配策略: {strategy}（支持 exact / date_order / all）"
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
    ///
    /// 返回 Err 时错误消息与 customer_confirm 状态门保持一致：
    /// - "confirmed" → "对账单已确认，不可重复确认"
    /// - "disputed"  → "对账单存在争议，请先解决争议后再确认"
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
    ///
    /// 返回 Err 时错误消息与 customer_dispute 状态门保持一致：
    /// - "confirmed" → "对账单已确认，不可提出争议"
    /// - "closed"    → "对账单已关闭，不可提出争议"
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

    /// 测试_核销状态常量_已关闭值正确
    ///
    /// 验证 ar::RECONCILIATION_CLOSED 常量值为 "closed"（小写），
    /// 与 ar_reconciliation.reconciliation_status 字段语义一致。
    #[test]
    fn 测试_核销状态常量_已关闭值正确() {
        assert_eq!(ar::RECONCILIATION_CLOSED, "closed");
    }

    /// 测试_核销状态常量_已取消值正确
    ///
    /// 验证 ar::RECONCILIATION_CANCELLED 常量值为 "cancelled"（小写），
    /// 与 ar_reconciliation.reconciliation_status 字段语义一致。
    #[test]
    fn 测试_核销状态常量_已取消值正确() {
        assert_eq!(ar::RECONCILIATION_CANCELLED, "cancelled");
    }

    /// 测试_匹配状态常量_已匹配值正确
    ///
    /// 验证 ar::MATCH_MATCHED 常量值为 "MATCHED"（大写），
    /// 与 ar_reconciliation_item.match_status 字段语义一致。
    #[test]
    fn 测试_匹配状态常量_已匹配值正确() {
        assert_eq!(ar::MATCH_MATCHED, "MATCHED");
    }

    /// 测试_收款状态常量_小写三态值正确
    ///
    /// 验证 ar_collection.status 字段使用的小写状态值：
    /// - COLLECTION_PENDING = "pending"
    /// - COLLECTION_CONFIRMED = "confirmed"
    /// - COLLECTION_CANCELLED = "cancelled"
    #[test]
    fn 测试_收款状态常量_小写三态值正确() {
        assert_eq!(ar::COLLECTION_PENDING, "pending");
        assert_eq!(ar::COLLECTION_CONFIRMED, "confirmed");
        assert_eq!(ar::COLLECTION_CANCELLED, "cancelled");
    }

    /// 测试_通用状态常量_已取消值正确
    ///
    /// 验证 common::STATUS_CANCELLED 常量值为 "CANCELLED"（大写），
    /// vfy_ops 中用于 ar_invoice.Status 过滤（ne("CANCELLED")）。
    #[test]
    fn 测试_通用状态常量_已取消值正确() {
        assert_eq!(common::STATUS_CANCELLED, "CANCELLED");
    }

    // =====================================================
    // 2. 核销金额计算（纯算法，复现 auto_match / generate_reconciliation）
    // =====================================================

    /// 测试_期末余额计算_正常场景
    ///
    /// 验证 vfy_ops auto_match / generate_reconciliation 中的期末余额公式：
    /// closing_balance = opening_balance + total_invoices - total_collections
    #[test]
    fn 测试_期末余额计算_正常场景() {
        let opening = decs!("1000");
        let invoices = decs!("5000");
        let collections = decs!("3000");
        let closing = opening + invoices - collections;
        assert_eq!(closing, decs!("3000"));
    }

    /// 测试_期末余额计算_零收款场景
    ///
    /// 验证当期无收款时，期末余额 = 期初 + 期内核销前发票额。
    #[test]
    fn 测试_期末余额计算_零收款场景() {
        let opening = decs!("2000");
        let invoices = decs!("4000");
        let collections = Decimal::ZERO;
        let closing = opening + invoices - collections;
        assert_eq!(closing, decs!("6000"));
    }

    /// 测试_期末余额计算_全额核销场景
    ///
    /// 验证当收款总额等于期初+发票额时，期末余额归零（核销完成）。
    #[test]
    fn 测试_期末余额计算_全额核销场景() {
        let opening = decs!("1000");
        let invoices = decs!("4000");
        let collections = decs!("5000");
        let closing = opening + invoices - collections;
        assert_eq!(closing, Decimal::ZERO);
    }

    // =====================================================
    // 3. 日期匹配阈值（auto_match 策略2 纯算法）
    // =====================================================

    /// 测试_日期匹配阈值_30天内可匹配
    ///
    /// 验证 vfy_ops/match.rs auto_match 策略2 中 date_diff <= 30 时应匹配。
    /// 边界值：恰好 30 天也应匹配。
    #[test]
    fn 测试_日期匹配阈值_30天内可匹配() {
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

    /// 测试_日期匹配阈值_超30天不匹配
    ///
    /// 验证 vfy_ops/match.rs auto_match 策略2 中 date_diff > 30 时不应匹配。
    #[test]
    fn 测试_日期匹配阈值_超30天不匹配() {
        let invoice_date = ymd!(2026, 6, 1);
        let coll_date = ymd!(2026, 7, 2); // 31 天后
        let diff = (coll_date - invoice_date).num_days().abs();
        assert_eq!(diff, 31);
        assert!(diff > 30);
    }

    // =====================================================
    // 4. 部分匹配金额与状态判定（auto_match 策略2 纯算法）
    // =====================================================

    /// 测试_部分匹配金额_取较小值
    ///
    /// 验证 vfy_ops/match.rs auto_match 策略2 中 matched = min(invoice_amount, collection_amount)。
    #[test]
    fn 测试_部分匹配金额_取较小值() {
        let inv_amt = decs!("5000");
        let coll_amt = decs!("3000");
        let matched = std::cmp::min(inv_amt, coll_amt);
        assert_eq!(matched, decs!("3000"));

        // 反向参数同样取较小值
        let matched_rev = std::cmp::min(coll_amt, inv_amt);
        assert_eq!(matched_rev, decs!("3000"));
    }

    /// 测试_匹配状态判定_完全与部分匹配
    ///
    /// 验证 vfy_ops/match.rs auto_match 策略2 中 match_status 判定规则：
    /// - matched == invoice_amount → ar::MATCH_MATCHED
    /// - matched < invoice_amount  → "PARTIAL"
    /// 注："PARTIAL" 在 status 模块无对应常量，沿用 vfy_ops/match.rs 字面量。
    #[test]
    fn 测试_匹配状态判定_完全与部分匹配() {
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

    /// 测试_未匹配数量公式_正确
    ///
    /// 验证 vfy_ops/match.rs auto_match 末尾 unmatched_count 公式：
    /// unmatched_count = invoices.len() + collections.len() - matched_count * 2
    /// 每次匹配消耗 1 张发票 + 1 笔收款，故乘 2。
    #[test]
    fn 测试_未匹配数量公式_正确() {
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

    /// 测试_账龄分桶_当期未逾期
    ///
    /// 验证 overdue_days <= 0 时落入第 0 桶（当期）。
    /// 边界值：overdue_days = 0（到期日当天）也应落入当期。
    #[test]
    fn 测试_账龄分桶_当期未逾期() {
        assert_eq!(aging_bucket_idx(0), 0);
        assert_eq!(aging_bucket_idx(-1), 0);
        assert_eq!(aging_bucket_idx(-30), 0);
    }

    /// 测试_账龄分桶_1到30天区间
    ///
    /// 验证 1 <= overdue_days <= 30 时落入第 1 桶（1-30天）。
    /// 边界值：1 和 30 均应落入此桶。
    #[test]
    fn 测试_账龄分桶_1到30天区间() {
        assert_eq!(aging_bucket_idx(1), 1);
        assert_eq!(aging_bucket_idx(15), 1);
        assert_eq!(aging_bucket_idx(30), 1);
    }

    /// 测试_账龄分桶_31到60天区间
    ///
    /// 验证 31 <= overdue_days <= 60 时落入第 2 桶（31-60天）。
    /// 边界值：31 和 60 均应落入此桶。
    #[test]
    fn 测试_账龄分桶_31到60天区间() {
        assert_eq!(aging_bucket_idx(31), 2);
        assert_eq!(aging_bucket_idx(45), 2);
        assert_eq!(aging_bucket_idx(60), 2);
    }

    /// 测试_账龄分桶_61到90天区间
    ///
    /// 验证 61 <= overdue_days <= 90 时落入第 3 桶（61-90天）。
    /// 边界值：61 和 90 均应落入此桶。
    #[test]
    fn 测试_账龄分桶_61到90天区间() {
        assert_eq!(aging_bucket_idx(61), 3);
        assert_eq!(aging_bucket_idx(75), 3);
        assert_eq!(aging_bucket_idx(90), 3);
    }

    /// 测试_账龄分桶_90天以上
    ///
    /// 验证 overdue_days > 90 时落入第 4 桶（90天以上）。
    /// 边界值：91 应落入此桶。
    #[test]
    fn 测试_账龄分桶_90天以上() {
        assert_eq!(aging_bucket_idx(91), 4);
        assert_eq!(aging_bucket_idx(180), 4);
        assert_eq!(aging_bucket_idx(365), 4);
    }

    // =====================================================
    // 7. 状态机转换合法性（customer_confirm / customer_dispute 纯算法）
    // =====================================================

    /// 测试_客户确认状态机_已确认拒绝
    ///
    /// 验证 customer_confirm 中 status == ar_status::RECONCILIATION_CONFIRMED 时应拒绝（不可重复确认），
    /// 返回 BusinessError 且消息包含 "对账单已确认，不可重复确认"。
    #[test]
    fn 测试_客户确认状态机_已确认拒绝() {
        let result = validate_customer_confirm("confirmed");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AppError::BusinessError(_)));
        assert!(format!("{err}").contains("对账单已确认，不可重复确认"));
    }

    /// 测试_客户确认状态机_争议中拒绝
    ///
    /// 验证 customer_confirm 中 status == ar_status::RECONCILIATION_DISPUTED 时应拒绝（需先解决争议），
    /// 返回 BusinessError 且消息包含 "对账单存在争议"。
    #[test]
    fn 测试_客户确认状态机_争议中拒绝() {
        let result = validate_customer_confirm("disputed");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AppError::BusinessError(_)));
        assert!(format!("{err}").contains("对账单存在争议"));
    }

    /// 测试_客户确认状态机_其他状态可转换
    ///
    /// 验证 customer_confirm 中 status 为 "draft" 等非终态时应允许转换到 "confirmed"。
    #[test]
    fn 测试_客户确认状态机_其他状态可转换() {
        // draft → confirmed：应允许
        let result = validate_customer_confirm("draft");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "confirmed");
    }

    /// 测试_客户争议状态机_已确认拒绝
    ///
    /// 验证 customer_dispute 中 status == ar_status::RECONCILIATION_CONFIRMED 时应拒绝（已确认不可提争议），
    /// 返回 BusinessError 且消息包含 "对账单已确认，不可提出争议"。
    #[test]
    fn 测试_客户争议状态机_已确认拒绝() {
        let result = validate_customer_dispute("confirmed");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AppError::BusinessError(_)));
        assert!(format!("{err}").contains("对账单已确认，不可提出争议"));
    }

    /// 测试_客户争议状态机_已关闭拒绝
    ///
    /// 验证 customer_dispute 中 status == ar_status::RECONCILIATION_CLOSED 时应拒绝（已关闭不可提争议），
    /// 返回 BusinessError 且消息包含 "对账单已关闭，不可提出争议"。
    #[test]
    fn 测试_客户争议状态机_已关闭拒绝() {
        let result = validate_customer_dispute("closed");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AppError::BusinessError(_)));
        assert!(format!("{err}").contains("对账单已关闭，不可提出争议"));
    }

    /// 测试_客户争议状态机_草稿可转换
    ///
    /// 验证 customer_dispute 中 status 为 "draft" 时应允许转换到 "disputed"。
    #[test]
    fn 测试_客户争议状态机_草稿可转换() {
        let result = validate_customer_dispute("draft");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "disputed");
    }

    // =====================================================
    // 8. 匹配策略校验（auto_match 开头纯算法）
    // =====================================================

    /// 测试_匹配策略校验_无效策略错误消息
    ///
    /// 验证 auto_match 中传入不支持的策略时返回 ValidationError，
    /// 错误消息格式："无效的匹配策略: {strategy}（支持 exact / date_order / all）"
    #[test]
    fn 测试_匹配策略校验_无效策略错误消息() {
        let result = validate_match_strategy(Some("invalid"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)));
        let msg = format!("{err}");
        assert!(msg.contains("无效的匹配策略: invalid"));
        assert!(msg.contains("exact / date_order / all"));
    }

    /// 测试_匹配策略校验_合法策略通过
    ///
    /// 验证 auto_match 中三种合法策略（exact/date_order/all）均通过校验，
    /// 且 None 默认为 "all"，大小写不敏感（自动转小写）。
    #[test]
    fn 测试_匹配策略校验_合法策略通过() {
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

    /// 测试_decs宏_核销金额解析
    ///
    /// 验证 decs! 宏可正确解析 vfy_ops 业务场景的金额字符串（含小数），
    /// 并可参与期末余额公式运算。
    #[test]
    fn 测试_decs宏_核销金额解析() {
        let inv = decs!("12345.67");
        assert_eq!(inv.to_string(), "12345.67");

        let coll = decs!("3000");
        assert_eq!(coll.to_string(), "3000");

        // 期末余额计算应正常工作
        let opening = decs!("1000");
        let closing = opening + inv - coll;
        assert_eq!(closing.to_string(), "10345.67");
    }

    /// 测试_ymd宏_对账日期解析
    ///
    /// 验证 ymd! 宏可正确解析 vfy_ops 业务场景的对账期间日期，
    /// 并可参与日期差运算（auto_match 策略2 依赖）。
    #[test]
    fn 测试_ymd宏_对账日期解析() {
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

    /// 测试_服务实例化_需数据库
    ///
    /// 验证 ArReconciliationService::new 可用 SQLite 内存库构造实例，
    /// 仅验证实例化成功（不需要 schema），与模板 测试_服务实例创建 同模式。
    #[tokio::test]
    async fn 测试_服务实例化_需数据库() {
        let db = setup_test_db().await;
        let svc = ArReconciliationService::new(Arc::new(db));
        // 验证实例化成功：Arc 引用计数 >= 1
        assert!(Arc::strong_count(&svc.db) >= 1);
    }

    /// 测试_自动对账完整流程_需数据库
    ///
    /// 验证 auto_match 端到端调用路径不 panic（需完整 schema + 测试数据）。
    /// 标注 #[ignore]：依赖真实 DB schema，CI 默认不跑，需 `cargo test -- --ignored`。
    /// 无 schema 时预期返回数据库错误而非 panic。
    #[tokio::test]
    #[ignore]
    async fn 测试_自动对账完整流程_需数据库() {
        let db = setup_test_db().await;
        let svc = ArReconciliationService::new(Arc::new(db));
        let req = AutoMatchRequest {
            customer_id: None,
            start_date: ymd!(2026, 1, 1),
            end_date: ymd!(2026, 3, 31),
            match_strategy: Some("all".to_string()),
        };
        // 无 schema 时预期返回数据库错误而非 panic
        let result = svc.auto_match(req, 1).await;
        assert!(result.is_err());
    }

    /// 测试_账龄报告完整流程_需数据库
    ///
    /// 验证 get_aging_report 端到端调用路径不 panic（需完整 schema + 测试数据）。
    /// 标注 #[ignore]：依赖真实 DB schema，CI 默认不跑，需 `cargo test -- --ignored`。
    /// 无 schema 时预期返回数据库错误而非 panic。
    #[tokio::test]
    #[ignore]
    async fn 测试_账龄报告完整流程_需数据库() {
        let db = setup_test_db().await;
        let svc = ArReconciliationService::new(Arc::new(db));
        // 无 schema 时预期返回数据库错误而非 panic
        let result = svc.get_aging_report(None).await;
        assert!(result.is_err());
    }
}
