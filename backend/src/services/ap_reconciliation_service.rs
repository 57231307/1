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
