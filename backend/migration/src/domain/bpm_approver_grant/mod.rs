//! BPM 审批岗补授（sales_manager / purchase_manager / production_manager）聚合迁移域。
//!
//! 本域只落一条数据迁移台账，登记为新的顶层域而非并入既有域：并入已应用的域在复用库上
//! 会因台账已记账而被整体跳过，导致三岗的 BPM 审批面在存量库里永远缺位（本仓有先例）。
//! 链尾新域名在复用库上属 pending，必然执行。
//! - m0001_grant_bpm_approver_keys: 按 roles.code 反查 id，为三岗补授通道①
//!   （`init_service_ops/permission.rs` 里 sales_manager、purchase_manager、production_manager
//!   三个既有分组各自的六枚 BPM 条目）的同一批派生键，冲突目标用 `role_permissions` 的真实
//!   唯一索引列集合，逐键打印实写行数与跳过成因，同键显式拒绝行只点名不覆盖。
//!
//! 依赖关系：`roles`（system m0001）与 `role_permissions`（system m0005）建表、以及
//! `uq_role_permissions_role_resource_action` 唯一索引（v15 域）均早于本域，故本域可安全
//! 使用 `ON CONFLICT`；六枚资源名在 `utils/path_utils.rs` 消歧映射与
//! `init_service.rs::PERMISSION_RESOURCES` 均已在册，本域不新增资源键、不动通道① 矩阵。
//!
//! `down` 按「三枚角色码 × 六枚键」收窄回收，可独立回退。

mod m0001_grant_bpm_approver_keys;

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_bpm_approver_grant"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0001_grant_bpm_approver_keys::Migration.up(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0001_grant_bpm_approver_keys::Migration
            .down(manager)
            .await?;
        Ok(())
    }
}
