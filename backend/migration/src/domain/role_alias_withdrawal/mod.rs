//! 零授权别名岗回收（sales / super_admin / warehouse）聚合迁移域。
//!
//! 本域只落一条数据迁移台账，且登记为**新的顶层域**而非并入既有域：并入已应用的域在复用库上
//! 会因台账已记账而被整体跳过，别名岗在存量库里就永远留着（本仓有先例）。链尾新域在复用库上
//! 属 pending，必然执行。
//! - m0001_withdraw_zero_grant_role_aliases: 把挂在三枚别名岗上的用户改指向各自正主
//!   （sales→sales_rep、warehouse→warehouse_keeper、super_admin→system_admin），回收别名岗的
//!   授权行，再删除别名岗本身。
//!
//! 依赖关系：`roles`（system m0001）、`users`、`user_role`、`role_permissions`（system m0005）
//! 建表均早于本域；正主三枚码都在 `init_service_ops/role.rs` 种子里，别名三枚码本域起不再播种。
//!
//! 排在 `system_admin_bpm_config_grant` 之后：那域给 system_admin 补的是读面授予，本域把
//! super_admin 的用户并到 system_admin，先后顺序决定并入后是否带着那两枚 bpm 配置读键，
//! 故必须晚于它。
//!
//! `down` 只重建三枚别名岗本身（按 role.rs 原种子属性），不把已被改指向的用户挪回去——
//! 改指向是有审计含义的一次性收敛，库内没有保留"谁原先挂哪一枚"的历史可供反向恢复。

mod m0001_withdraw_zero_grant_role_aliases;

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_role_alias_withdrawal"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0001_withdraw_zero_grant_role_aliases::Migration
            .up(manager)
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0001_withdraw_zero_grant_role_aliases::Migration
            .down(manager)
            .await?;
        Ok(())
    }
}
