//! P4-1 性能优化 - 新增复合索引
//!
//! 批次 190 规则 0 修复（2026-07-08）：
//! 原 m0025 创建了 7 个引用 tenant_id 列的索引，但 sales_orders/inventory_stocks/
//! ar_invoices/purchase_orders/inventory_reservations/operation_logs/users 表
//! 自 initial_schema 创建以来从未包含 tenant_id 列，导致迁移执行时报错
//! "column tenant_id does not exist"。租户功能已在 m0029 中完整下线，
//! 这些索引本就不应存在。删除所有 tenant_id 索引创建语句。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // 批次 190 修复：所有 tenant_id 索引已删除（引用不存在的列）
        // 保留空 up 实现，避免破坏迁移历史顺序
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // 批次 190 修复：无需回滚（原 up 未创建任何索引）
        Ok(())
    }
}
