//! 仓储两岗补授 scan-receipt:create 的存量库通道聚合迁移域。
//!
//! 本域只落一条数据迁移台账，登记为新的顶层域而非并入任何已应用域：已应用域在复用库上
//! 台账已记账、整体跳过，嵌在其中的补授在存量库永远不会执行（本仓有 m0102 由 business 域
//! 升为顶层域的同一先例）。链尾新域名在复用库上属 pending，必然执行。
//! - m0121_grant_scan_receipt_keys: 按 roles.code 反查 id，为 warehouse_keeper 与
//!   warehouse_manager 补授与 init 矩阵同源的 scan-receipt create 键；NOT EXISTS 守卫使重跑
//!   等价，命中 0 个角色时以 NOTICE 报出不静默通过。
//!
//! 依赖关系：roles 与 role_permissions 建表（system 域）早于本域。资源名 scan-receipt 登记在
//! `src/services/init_service.rs::PERMISSION_RESOURCES`，通道① 矩阵授予在
//! `src/services/init_service_ops/permission.rs` 的仓储分组，本域只走通道② 存量库补授，
//! 不新增资源键、不动矩阵。
//!
//! down 只按本域角色码回收 scan-receipt 的 create 行，可独立回退，不回收人工另行授予的
//! 任何权限。

mod m0121_grant_scan_receipt_keys;

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_scan_receipt_grant"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0121_grant_scan_receipt_keys::Migration.up(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0121_grant_scan_receipt_keys::Migration
            .down(manager)
            .await?;
        Ok(())
    }
}
