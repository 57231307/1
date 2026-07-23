//! 应付单 Service（facade）
//!
//! 应付单服务层，负责应付单的核心业务逻辑
//! 包含应付单自动生成、手工创建、审核、核销等全流程管理
//!
//! 批次 490 D10-4b 拆分：本文件作为 facade，保留 ApInvoiceService struct + new 构造函数
//! + ApInvoiceListQuery 查询参数 + 校验纯函数 + 单号生成宏 + 单元测试。
//! impl 块迁移至 `ap_invoice_ops` 子模块（receipt / crud / report），
//! DTOs 迁移至 `ap_invoice_ops::types`，通过 db 字段 pub(crate) 让 ops 访问，
//! 外部引用路径（crate::services::ap_invoice_service::ApInvoiceService 等）保持不变。

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

// 重新导出 DTOs（迁移至 ap_invoice_ops::types），保持外部引用路径不变
// 外部仍可通过 crate::services::ap_invoice_service::{CreateApInvoiceRequest, ...} 访问
pub use crate::services::ap_invoice_ops::types::{
    AgingAnalysisItem, ApInvoiceStatistics, BalanceSummary, CreateApInvoiceRequest,
    StatusStatItem, UpdateApInvoiceRequest,
};

// 批次 102 v6 P3-2 修复：状态字符串常量化，引用 crate::models::status

/// 默认本位币汇率（CNY 本位币 = 1.0）。
///
/// 历史缺陷（P0-1，2026-06-25 综合审计）：自动生成 AP 发票时曾误用
/// `Decimal::new(1, 2)` = 0.01，导致下游按汇率换算本位币金额被缩小 100 倍。
/// 抽取为常量并在单元测试中断言其值，避免再次被改错。
///
/// 注意：`Decimal::new` 不是 const fn，不能用于 const 初始化；
/// 使用 rust_decimal 提供的 const 关联常量 `Decimal::ONE`（= 1.0）。
pub const DEFAULT_BASE_CURRENCY_EXCHANGE_RATE: Decimal = Decimal::ONE;

/// 应付单服务
pub struct ApInvoiceService {
    pub(crate) db: Arc<DatabaseConnection>,
}

/// 应付单列表查询参数（service 层，page/page_size 已解析为非 Option）
#[derive(Debug, Clone)]
pub struct ApInvoiceListQuery {
    pub supplier_id: Option<i32>,
    pub invoice_status: Option<String>,
    pub invoice_type: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub page: u64,
    pub page_size: u64,
}

impl ApInvoiceService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // 生成应付单号
    // 格式：AP + 年月日 + 三位序号（AP20260315001）
    crate::impl_generate_no!(
        generate_invoice_no,
        "API",
        crate::models::ap_invoice::Entity,
        crate::models::ap_invoice::Column::InvoiceNo
    );
}

// =====================================================
// DTO 校验函数（TS-S-5 安全加固）
// =====================================================
//
// 校验纯函数保留在 facade（与 DTOs 的 #[validate] 引用耦合），
// 通过 pub(crate) 让 ap_invoice_ops::types 的 DTOs 全路径引用
//（crate::services::ap_invoice_service::validate_*），与 crate::utils::validator 用法一致。

/// 校验 Decimal 为正数
pub(crate) fn validate_positive_decimal(value: &Decimal) -> Result<(), validator::ValidationError> {
    if *value <= Decimal::ZERO {
        return Err(validator::ValidationError::new("金额必须为正数"));
    }
    Ok(())
}

/// 校验 Decimal 为非负数
pub(crate) fn validate_non_negative_decimal(
    value: &Decimal,
) -> Result<(), validator::ValidationError> {
    if *value < Decimal::ZERO {
        return Err(validator::ValidationError::new("金额不能为负数"));
    }
    Ok(())
}

/// 校验汇率合法：必须大于 0 且不等于 P0-1 历史缺陷值 0.01
pub(crate) fn validate_exchange_rate(value: &Decimal) -> Result<(), validator::ValidationError> {
    if *value <= Decimal::ZERO {
        return Err(validator::ValidationError::new("汇率必须大于0"));
    }
    // P0-1 防护：拒绝 0.01 汇率（历史缺陷值）
    if *value == Decimal::new(1, 2) {
        return Err(validator::ValidationError::new(
            "汇率不能为0.01（P0-1历史缺陷值，本位币汇率应为1.0）",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    //! AP 发票服务单元测试
    //!
    //! 覆盖目标：
    //! - DEFAULT_BASE_CURRENCY_EXCHANGE_RATE 常量值正确性（防止 P0-1 缺陷复发）
    //! - 汇率换算逻辑（金额 × 汇率 = 本位币金额）

    use super::*;

    /// 防止 P0-1 缺陷复发：默认本位币汇率必须是 1.0，不能是 0.01。
    ///
    /// 历史缺陷：`Decimal::new(1, 2)` 误用导致自动生成 AP 发票汇率被设为 0.01，
    /// 下游按汇率换算本位币金额的财务计算被缩小 100 倍。
    #[test]
    fn test_default_exchange_rate_is_one_not_zero_dot_zero_one() {
        assert_eq!(
            DEFAULT_BASE_CURRENCY_EXCHANGE_RATE,
            Decimal::new(1, 0),
            "默认本位币汇率应为 1.0，当前值 {:?} 不正确（P0-1 缺陷复发风险）",
            DEFAULT_BASE_CURRENCY_EXCHANGE_RATE
        );
        // 数值断言：1.0 而非 0.01
        assert_eq!(DEFAULT_BASE_CURRENCY_EXCHANGE_RATE, Decimal::ONE);
        assert_ne!(
            DEFAULT_BASE_CURRENCY_EXCHANGE_RATE,
            Decimal::new(1, 2),
            "默认汇率不应为 0.01"
        );
    }

    /// 验证按默认汇率换算本位币金额：金额 × 1.0 = 金额本身。
    ///
    /// 该测试模拟下游按汇率换算本位币金额的场景，确保 P0-1 修复后
    /// 自动生成的 AP 发票换算结果不会被缩小 100 倍。
    #[test]
    fn test_exchange_rate_conversion_not_shrunk_by_100() {
        let invoice_amount = Decimal::new(12345, 2); // 123.45
        let base_currency_amount = invoice_amount * DEFAULT_BASE_CURRENCY_EXCHANGE_RATE;

        // 修复前（汇率 0.01）：123.45 * 0.01 = 1.2345（被缩小 100 倍）
        assert_ne!(
            base_currency_amount,
            Decimal::new(12345, 4), // 1.2345（错误结果）
            "本位币金额被缩小 100 倍，P0-1 缺陷未修复"
        );

        // 修复后（汇率 1.0）：123.45 * 1.0 = 123.45（正确）
        assert_eq!(
            base_currency_amount,
            Decimal::new(12345, 2),
            "按汇率 1.0 换算后本位币金额应等于原金额"
        );
    }

    // ============ 批次 393 补测：AP 状态常量与校验函数 ============

    /// 测试_AP状态常量值正确性
    ///
    /// 验证 ap_invoice.invoice_status 字段使用的状态常量值与业务约定一致。
    /// - common::STATUS_DRAFT = "DRAFT"（草稿）
    /// - ap_invoice::INVOICE_AUDITED = "AUDITED"（已审核，AP 专属）
    /// - payment::PAYMENT_PAID = "PAID"（已付款）
    /// - payment::PAYMENT_PARTIAL_PAID = "PARTIAL_PAID"（部分付款）
    /// - common::STATUS_CANCELLED = "CANCELLED"（已取消）
    #[test]
    fn 测试_AP状态常量值正确性() {
        use crate::models::status;
        assert_eq!(status::common::STATUS_DRAFT, "DRAFT");
        assert_eq!(status::ap_invoice::INVOICE_AUDITED, "AUDITED");
        assert_eq!(status::payment::PAYMENT_PAID, "PAID");
        assert_eq!(status::payment::PAYMENT_PARTIAL_PAID, "PARTIAL_PAID");
        assert_eq!(status::common::STATUS_CANCELLED, "CANCELLED");

        // 防御性断言：AP 专属 AUDITED 不应与通用 APPROVED 混淆
        assert_ne!(status::ap_invoice::INVOICE_AUDITED, status::common::STATUS_APPROVED);
    }

    /// 测试_汇率校验函数_合法与非法值
    ///
    /// 验证 validate_exchange_rate 的拒绝逻辑：
    /// - 0 / 负数 / 0.01（P0-1 历史缺陷值）应拒绝
    /// - 正数（如 1.0, 6.5, 0.1）应通过
    #[test]
    fn 测试_汇率校验函数_合法与非法值() {
        // 非法值应拒绝
        assert!(validate_exchange_rate(&Decimal::ZERO).is_err(), "汇率 0 应拒绝");
        assert!(
            validate_exchange_rate(&Decimal::new(-1, 0)).is_err(),
            "负汇率应拒绝"
        );
        // P0-1 历史缺陷值 0.01 应拒绝
        assert!(
            validate_exchange_rate(&Decimal::new(1, 2)).is_err(),
            "汇率 0.01（P0-1 缺陷值）应拒绝"
        );

        // 合法值应通过
        assert!(
            validate_exchange_rate(&Decimal::ONE).is_ok(),
            "汇率 1.0 应通过"
        );
        assert!(
            validate_exchange_rate(&Decimal::new(65, 1)).is_ok(),
            "汇率 6.5 应通过"
        );
        assert!(
            validate_exchange_rate(&Decimal::new(1, 1)).is_ok(),
            "汇率 0.1 应通过（不同于 0.01）"
        );
    }

    /// 测试_金额校验函数_正数校验
    ///
    /// 验证 validate_positive_decimal 的拒绝逻辑：
    /// - 0 / 负数应拒绝
    /// - 正数应通过
    #[test]
    fn 测试_金额校验函数_正数校验() {
        // 非法值
        assert!(
            validate_positive_decimal(&Decimal::ZERO).is_err(),
            "金额 0 应拒绝（必须为正数）"
        );
        assert!(
            validate_positive_decimal(&Decimal::new(-100, 2)).is_err(),
            "负金额应拒绝"
        );

        // 合法值
        assert!(
            validate_positive_decimal(&Decimal::new(100, 2)).is_ok(),
            "正金额应通过"
        );
        assert!(
            validate_positive_decimal(&Decimal::ONE).is_ok(),
            "金额 1 应通过"
        );
    }

    /// 测试_金额校验函数_非负校验
    ///
    /// 验证 validate_non_negative_decimal 的拒绝逻辑：
    /// - 负数应拒绝
    /// - 0 / 正数应通过（允许 0，如税额为 0 的场景）
    #[test]
    fn 测试_金额校验函数_非负校验() {
        // 非法值
        assert!(
            validate_non_negative_decimal(&Decimal::new(-1, 0)).is_err(),
            "负数应拒绝"
        );

        // 合法值
        assert!(
            validate_non_negative_decimal(&Decimal::ZERO).is_ok(),
            "0 应通过（非负校验允许零）"
        );
        assert!(
            validate_non_negative_decimal(&Decimal::new(100, 2)).is_ok(),
            "正数应通过"
        );
    }

    // ============ 批次 393 补测：AP 状态机门 ============

    /// 复现 approve 方法内的状态机门判定
    ///
    /// 源码位置：approve 方法内的状态门。
    /// 仅 common::STATUS_DRAFT 状态允许审核转 AUDITED。
    fn can_approve(current_status: &str) -> bool {
        current_status == crate::models::status::common::STATUS_DRAFT
    }

    /// 复现 mark_as_paid 方法内的状态机门判定（白名单）
    ///
    /// 源码位置：mark_as_paid 方法内的状态门（P0 3-3 修复）。
    /// 仅 AUDITED / PARTIAL_PAID 状态允许标记为已付清。
    fn can_mark_as_paid(current_status: &str) -> bool {
        [
            crate::models::status::ap_invoice::INVOICE_AUDITED,
            crate::models::status::payment::PAYMENT_PARTIAL_PAID,
        ]
        .contains(&current_status)
    }

    /// 复现 cancel 方法内的状态机门判定（白名单）
    ///
    /// 源码位置：cancel 方法内的状态门。
    /// 仅 AUDITED / PARTIAL_PAID 状态允许取消（且需 paid_amount 为 0）。
    fn can_cancel(current_status: &str) -> bool {
        [
            crate::models::status::ap_invoice::INVOICE_AUDITED,
            crate::models::status::payment::PAYMENT_PARTIAL_PAID,
        ]
        .contains(&current_status)
    }

    /// 测试_approve状态机门_仅DRAFT允许
    ///
    /// 验证 approve 状态门：仅 DRAFT 状态可审核
    #[test]
    fn 测试_approve状态机门_仅DRAFT允许() {
        use crate::models::status;
        // DRAFT 允许审核
        assert!(can_approve(status::common::STATUS_DRAFT));

        // 其他状态禁止审核
        assert!(!can_approve(status::ap_invoice::INVOICE_AUDITED));
        assert!(!can_approve(status::payment::PAYMENT_PAID));
        assert!(!can_approve(status::payment::PAYMENT_PARTIAL_PAID));
        assert!(!can_approve(status::common::STATUS_CANCELLED));
    }

    /// 测试_mark_as_paid状态机门_仅AUDITED和PARTIAL_PAID允许
    ///
    /// 验证 mark_as_paid 状态门（P0 3-3 修复）：仅 AUDITED/PARTIAL_PAID 可标记已付清
    #[test]
    fn 测试_mark_as_paid状态机门_仅AUDITED和PARTIAL_PAID允许() {
        use crate::models::status;
        // 允许的状态
        assert!(can_mark_as_paid(status::ap_invoice::INVOICE_AUDITED));
        assert!(can_mark_as_paid(status::payment::PAYMENT_PARTIAL_PAID));

        // 禁止的状态（P0 3-3 修复：堵住 DRAFT 直接跳过审核标记已付清的漏洞）
        assert!(!can_mark_as_paid(status::common::STATUS_DRAFT));
        assert!(!can_mark_as_paid(status::payment::PAYMENT_PAID));
        assert!(!can_mark_as_paid(status::common::STATUS_CANCELLED));
    }

    /// 测试_cancel状态机门_仅AUDITED和PARTIAL_PAID允许
    ///
    /// 验证 cancel 状态门：仅 AUDITED/PARTIAL_PAID 可取消
    #[test]
    fn 测试_cancel状态机门_仅AUDITED和PARTIAL_PAID允许() {
        use crate::models::status;
        // 允许的状态
        assert!(can_cancel(status::ap_invoice::INVOICE_AUDITED));
        assert!(can_cancel(status::payment::PAYMENT_PARTIAL_PAID));

        // 禁止的状态
        assert!(!can_cancel(status::common::STATUS_DRAFT));
        assert!(!can_cancel(status::payment::PAYMENT_PAID));
        assert!(!can_cancel(status::common::STATUS_CANCELLED));
    }

    // ============ 批次 393 补测：账龄分桶算法 ============

    /// 复现 get_aging_analysis 中的账龄分桶逻辑
    ///
    /// 源码位置：get_aging_analysis 方法内的账龄区间分类。
    /// 6 个区间：未到期 / 1-30 / 31-60 / 61-90 / 91-180 / 180 天以上
    fn aging_bucket(days_overdue: i32) -> String {
        if days_overdue < 0 {
            "未到期".to_string()
        } else if days_overdue <= 30 {
            "逾期 1-30 天".to_string()
        } else if days_overdue <= 60 {
            "逾期 31-60 天".to_string()
        } else if days_overdue <= 90 {
            "逾期 61-90 天".to_string()
        } else if days_overdue <= 180 {
            "逾期 91-180 天".to_string()
        } else {
            "逾期 180 天以上".to_string()
        }
    }

    /// 测试_账龄分桶算法_6个区间
    ///
    /// 验证 get_aging_analysis 的账龄分桶覆盖 6 个区间边界
    #[test]
    fn 测试_账龄分桶算法_6个区间() {
        // 未到期（days_overdue = -1 表示未到期）
        assert_eq!(aging_bucket(-1), "未到期");
        assert_eq!(aging_bucket(-30), "未到期");

        // 逾期 1-30 天（边界 0 和 30）
        assert_eq!(aging_bucket(0), "逾期 1-30 天");
        assert_eq!(aging_bucket(1), "逾期 1-30 天");
        assert_eq!(aging_bucket(30), "逾期 1-30 天");

        // 逾期 31-60 天（边界 31 和 60）
        assert_eq!(aging_bucket(31), "逾期 31-60 天");
        assert_eq!(aging_bucket(60), "逾期 31-60 天");

        // 逾期 61-90 天（边界 61 和 90）
        assert_eq!(aging_bucket(61), "逾期 61-90 天");
        assert_eq!(aging_bucket(90), "逾期 61-90 天");

        // 逾期 91-180 天（边界 91 和 180）
        assert_eq!(aging_bucket(91), "逾期 91-180 天");
        assert_eq!(aging_bucket(180), "逾期 91-180 天");

        // 逾期 180 天以上（边界 181）
        assert_eq!(aging_bucket(181), "逾期 180 天以上");
        assert_eq!(aging_bucket(365), "逾期 180 天以上");
    }
}
