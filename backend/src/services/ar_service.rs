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

/// 应收账款服务（facade，impl 块分散到 `ar_ops/` 子模块）
pub struct ArService {
    /// 数据库连接句柄（`pub(crate)` 供 ar_ops 兄弟模块访问）
    pub(crate) db: Arc<DatabaseConnection>,
}

impl ArService {
    /// 创建应收账款服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}
