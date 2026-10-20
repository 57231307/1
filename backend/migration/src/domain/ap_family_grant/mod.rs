//! 应付族（AP）授权面补授（finance_manager 与 accountant）聚合迁移域。
//!
//! 本域只落一条数据迁移台账，登记为新的顶层域而非并入既有域：并入已应用的域在复用库上
//! 会因台账已记账而被整体跳过，导致应付族入口在存量库里永远缺位（本仓有先例）。
//! 链尾新域名在复用库上属 pending，必然执行。
//! - m0001_grant_ap_family_keys: 按 roles.code 反查 id，为两岗补授通道①
//!   （`init_service_ops/permission.rs` 的 finance_manager 与 accountant 分组）同源的
//!   应付族键，冲突目标用 `role_permissions` 的真实唯一索引列集合，逐键打印实写行数与
//!   跳过成因，同键显式拒绝行只点名不覆盖。
//!
//! 依赖关系：`roles` 与 `role_permissions` 建表（system 域）以及
//! `uq_role_permissions_role_resource_action` 唯一索引（v15 域）均早于本域，故本域可安全
//! 使用 `ON CONFLICT`；九枚资源名（ap-invoices、ap-payments、ap-verifications、
//! ap-payment-requests、ap-reconciliations、ap-aging、ap-daily、ap-monthly、
//! ap-statistics）在 `utils/path_utils.rs` 的消歧映射与 `init_service.rs` 的
//! `PERMISSION_RESOURCES` 均已在册，本域不新增资源键、不动通道① 矩阵。
//!
//! 出纳（cashier）刻意不在本域角色集内：口径是应付族的审核面归管理岗、制单记账面归会计，
//! 出纳不持任何 ap-* 键，不在本域擅自补授。
//!
//! `down` 按「角色码 × 本域逐枚 (resource_type, action) 配对」收窄回收，可独立回退，
//! 不触碰其它角色、其它动作与通配授予。

mod m0001_grant_ap_family_keys;

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_ap_family_grant"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0001_grant_ap_family_keys::Migration.up(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0001_grant_ap_family_keys::Migration.down(manager).await?;
        Ok(())
    }
}
