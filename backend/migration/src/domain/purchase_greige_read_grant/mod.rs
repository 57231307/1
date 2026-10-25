//! 采购两岗补授 greige-fabrics:read 的存量库通道聚合迁移域。
//!
//! 本域只落一条数据迁移台账，登记为新的顶层域而非并入 business 域：m0102 此前嵌在
//! business 域 up 链尾，而 business 域在复用库上台账已记账、整体跳过，导致补授在存量库
//! 永远不会执行（本仓有迁移嵌已应用域内不重跑的先例）。链尾新域名在复用库上属 pending，必然执行。
//! - m0102_grant_purchase_greige_read: 按 roles.code 反查 id，为 purchase_manager 与
//!   purchase_clerk 补授与 init 矩阵同源的 greige-fabrics read 键；NOT EXISTS 守卫使重跑
//!   等价，命中 0 个角色时以 NOTICE 报出不静默通过。
//!
//! 依赖关系：roles 与 role_permissions 建表（system 域）早于本域，本域不新增资源键、
//! 不动通道① 矩阵，只走通道② 存量库补授。
//!
//! down 只按本域角色码回收 greige-fabrics 的 read 行，可独立回退，不触碰 m0097 授给
//! 生产与仓储岗的同名键，也不回收人工另行授予的其它权限。

mod m0102_grant_purchase_greige_read;

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_purchase_greige_read_grant"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0102_grant_purchase_greige_read::Migration
            .up(manager)
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0102_grant_purchase_greige_read::Migration
            .down(manager)
            .await?;
        Ok(())
    }
}
