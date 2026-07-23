//! 应收账款服务（facade，批次 488 D10-1 拆分）
//!
//! 本文件为 facade 入口，仅保留 `ArService` struct + `new` 构造函数 + 单元测试。
//! 业务实现已按职责拆分到 `ar_ops/` 子模块（与 `ar_service` 同为 `crate::services` 下兄弟模块）：
//! - `ar_ops::collection`：收款管理（17 方法，原 L112-751）
//! - `ar_ops::verification`：核销管理（23 方法，原 L753-1778）
//! - `ar_ops::report`：报表管理（9 方法，原 L1780-2177）
//! - `ar_ops::types`：内部聚合辅助 struct + `CreateArPaymentParams`
//! - `ar_ops::json_helpers`：4 个 Model → JSON 序列化自由函数
//!
//! 设计要点（与拆分前一致）：
//! - 收款管理基于 ar_collection 表
//! - 核销管理基于 ar_reconciliation + ar_reconciliation_item 表
//! - 报表管理基于 ar_invoice + ar_collection 聚合查询
//! - 所有写操作在事务内执行，状态变更加 lock_exclusive 串行化
//! - 所有更新通过 update_with_audit 记录审计日志
//! - 金额校验 round_dp(2) 限制货币精度
//! - 期间锁定检查通过 AccountingPeriodService::check_date_locked_txn
//!
//! 拆分兼容性：
//! - 外部 handler 通过 `crate::services::ar_service::ArService::new` 调用，路径不变
//! - 外部 handler 通过 `crate::services::ar_service::CreateArPaymentParams` 引用，路径不变（此处 re-export）
//! - `db` 字段使用 `pub(crate)` 可见性，ar_ops 子模块的 impl 块可直接访问
//! - impl 块分散在 ar_ops 子模块，Rust 允许同一 crate 多文件多 impl 块

use sea_orm::DatabaseConnection;
use std::sync::Arc;

// 批次 488 D10-1 拆分：re-export 保持外部引用路径 `crate::services::ar_service::CreateArPaymentParams` 不变
pub use crate::services::ar_ops::CreateArPaymentParams;

/// 应收账款服务
///
/// struct 定义保留在 facade，impl 块按职责分散到 `ar_ops/` 子模块。
pub struct ArService {
    /// 数据库连接句柄
    ///
    /// `pub(crate)` 可见性：ar_ops 兄弟模块的 impl 块需直接访问此字段。
    pub(crate) db: Arc<DatabaseConnection>,
}

impl ArService {
    /// 创建应收账款服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[cfg(test)]
mod tests {
    //! 应收账款服务单元测试（批次 393 补测）
    //!
    //! 覆盖目标：
    //! - AR 状态常量值正确性（COLLECTION_*/MATCH_* 大小写混合）
    //! - 收款金额校验（零/负数/精度超限）
    //! - 收款状态机门（pending → confirmed）
    //! - 核销金额贪心匹配算法
    //! - ArService 实例化

    use super::*;
    use crate::models::status;
    use rust_decimal::Decimal;
    use sea_orm::Database;

    /// 复现 create_payment 中的收款金额校验逻辑
    ///
    /// 源码位置：create_payment 方法开头两道校验门。
    /// 1. amount <= 0 → Err("收款金额必须大于零")
    /// 2. amount.round_dp(2) != amount → Err("收款金额精度不能超过 2 位小数")
    fn validate_payment_amount(amount: Decimal) -> Result<(), &'static str> {
        if amount <= Decimal::ZERO {
            return Err("收款金额必须大于零");
        }
        if amount.round_dp(2) != amount {
            return Err("收款金额精度不能超过 2 位小数");
        }
        Ok(())
    }

    /// 复现 confirm_payment 中的收款状态机门判定
    ///
    /// 源码位置：confirm_payment 方法内的状态门。
    /// 仅 status::ar::COLLECTION_PENDING 状态允许确认。
    fn can_confirm_payment(current_status: &str) -> bool {
        current_status == status::ar::COLLECTION_PENDING
    }

    /// 复现 create_payment 中的核销金额贪心匹配算法
    ///
    /// 源码位置：create_payment 方法内关联多张发票的扣减循环。
    /// 按发票顺序扣减，每张发票扣减 min(剩余收款, 发票未收金额)。
    /// 返回各发票的实际核销金额列表 + 剩余未核销金额。
    fn greedy_match(
        payment_amount: Decimal,
        unpaid_amounts: &[Decimal],
    ) -> (Vec<Decimal>, Decimal) {
        let mut remaining = payment_amount;
        let mut allocations = Vec::with_capacity(unpaid_amounts.len());
        for unpaid in unpaid_amounts {
            if remaining <= Decimal::ZERO {
                allocations.push(Decimal::ZERO);
                continue;
            }
            let allocate = remaining.min(*unpaid);
            allocations.push(allocate);
            remaining -= allocate;
        }
        (allocations, remaining)
    }

    /// 测试_AR状态常量值正确性
    ///
    /// 验证 ar_collection.status（小写）和 ar_reconciliation_item.match_status（大写）
    /// 的常量值与业务约定一致，防止大小写混淆导致状态匹配失败。
    #[test]
    fn 测试_AR状态常量值正确性() {
        // ar_collection.status（小写值，批次 231 v13 P1-1 统一小写）
        assert_eq!(status::ar::COLLECTION_PENDING, "pending");
        assert_eq!(status::ar::COLLECTION_CONFIRMED, "confirmed");
        assert_eq!(status::ar::COLLECTION_CANCELLED, "cancelled");

        // ar_reconciliation.reconciliation_status（小写值）
        assert_eq!(status::ar::RECONCILIATION_DRAFT, "draft");
        assert_eq!(status::ar::RECONCILIATION_SENT, "sent");
        assert_eq!(status::ar::RECONCILIATION_CONFIRMED, "confirmed");
        assert_eq!(status::ar::RECONCILIATION_DISPUTED, "disputed");
        assert_eq!(status::ar::RECONCILIATION_CLOSED, "closed");
        assert_eq!(status::ar::RECONCILIATION_CANCELLED, "cancelled");

        // ar_reconciliation_item.match_status（大写值，注意大小写混合）
        assert_eq!(status::ar::MATCH_MATCHED, "MATCHED");
        assert_eq!(status::ar::MATCH_UNMATCHED, "UNMATCHED");

        // 防御性断言：小写/大写不应混淆
        assert_ne!(status::ar::COLLECTION_PENDING, "PENDING");
        assert_ne!(status::ar::MATCH_MATCHED, "matched");
    }

    /// 测试_收款金额校验_零或负数拒绝
    ///
    /// 场景：amount <= 0 应返回 Err（防零额收款）
    #[test]
    fn 测试_收款金额校验_零或负数拒绝() {
        // 零金额
        let result = validate_payment_amount(Decimal::ZERO);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "收款金额必须大于零");

        // 负数金额
        let result = validate_payment_amount(Decimal::new(-100, 2));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "收款金额必须大于零");
    }

    /// 测试_收款金额校验_精度超限拒绝
    ///
    /// 场景：amount.round_dp(2) != amount（超过 2 位小数）应返回 Err
    #[test]
    fn 测试_收款金额校验_精度超限拒绝() {
        // 3 位小数（123.456）应拒绝
        let result = validate_payment_amount(Decimal::new(123456, 3));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "收款金额精度不能超过 2 位小数");

        // 2 位小数（123.45）应通过
        let result = validate_payment_amount(Decimal::new(12345, 2));
        assert!(result.is_ok());

        // 整数（100）应通过
        let result = validate_payment_amount(Decimal::new(100, 0));
        assert!(result.is_ok());
    }

    /// 测试_收款状态机门_仅pending允许确认
    ///
    /// 验证 confirm_payment 的状态门：仅 pending 状态允许确认
    #[test]
    fn 测试_收款状态机门_仅pending允许确认() {
        // pending 允许确认
        assert!(can_confirm_payment(status::ar::COLLECTION_PENDING));

        // confirmed / cancelled 禁止确认
        assert!(!can_confirm_payment(status::ar::COLLECTION_CONFIRMED));
        assert!(!can_confirm_payment(status::ar::COLLECTION_CANCELLED));

        // 非法状态值禁止
        assert!(!can_confirm_payment("PENDING")); // 大写（历史 bug 值）
        assert!(!can_confirm_payment(""));
        assert!(!can_confirm_payment("unknown"));
    }

    /// 测试_核销金额贪心匹配算法
    ///
    /// 验证 create_payment 中按发票顺序扣减的核销逻辑：
    /// - 收款金额足够：每张发票扣减其 unpaid_amount，剩余为 0
    /// - 收款金额不足：按顺序扣减，最后一张部分扣减，后续发票扣减 0
    #[test]
    fn 测试_核销金额贪心匹配算法() {
        // 场景 1：收款金额 = 300，3 张发票未收金额 [100, 200, 50]
        // 期望：[100, 200, 0]，剩余 0（前两张发票完全核销，第三张未核销）
        let (allocations, remaining) = greedy_match(
            Decimal::new(300, 0),
            &[
                Decimal::new(100, 0),
                Decimal::new(200, 0),
                Decimal::new(50, 0),
            ],
        );
        assert_eq!(allocations, vec![
            Decimal::new(100, 0),
            Decimal::new(200, 0),
            Decimal::ZERO,
        ]);
        assert_eq!(remaining, Decimal::ZERO);

        // 场景 2：收款金额 = 150，3 张发票未收金额 [100, 200, 50]
        // 期望：[100, 50, 0]，剩余 0（第一张完全核销，第二张部分核销 50）
        let (allocations, remaining) = greedy_match(
            Decimal::new(150, 0),
            &[
                Decimal::new(100, 0),
                Decimal::new(200, 0),
                Decimal::new(50, 0),
            ],
        );
        assert_eq!(allocations, vec![
            Decimal::new(100, 0),
            Decimal::new(50, 0),
            Decimal::ZERO,
        ]);
        assert_eq!(remaining, Decimal::ZERO);

        // 场景 3：收款金额 = 500，2 张发票未收金额 [100, 200]
        // 期望：[100, 200]，剩余 200（全部核销，收款有盈余）
        let (allocations, remaining) = greedy_match(
            Decimal::new(500, 0),
            &[Decimal::new(100, 0), Decimal::new(200, 0)],
        );
        assert_eq!(allocations, vec![
            Decimal::new(100, 0),
            Decimal::new(200, 0),
        ]);
        assert_eq!(remaining, Decimal::new(200, 0));
    }

    /// 测试_服务实例化_SQLite内存数据库
    ///
    /// 验证 ArService 能在 SQLite 内存数据库上实例化（new 不触发 DB 操作）
    #[tokio::test]
    async fn 测试_服务实例化_SQLite内存数据库() {
        let db_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "sqlite::memory:".to_string());
        let db = Database::connect(&db_url)
            .await
            .expect("测试夹具：数据库连接失败");
        let service = ArService::new(std::sync::Arc::new(db));
        let _ = service;
    }
}
