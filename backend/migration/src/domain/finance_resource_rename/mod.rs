//! 财务域权限资源键改名波（乙波 finance）聚合迁移域。
//!
//! 本域只落一条数据迁移台账，登记为新的顶层域而非并入既有域：并入已应用的域在复用库上
//! 会因台账已记账而被整体跳过，导致 `role_permissions.resource_type` 停在旧值与代码漂移
//! （本仓有先例）。链尾新域名在复用库上属 pending，必然执行。
//! - m0001_rename_role_permission_resource_type: 按 `utils/path_utils.rs::RESOURCE_NAME_MAP`
//!   的财务域改名映射，把 `role_permissions.resource_type` 的旧 URL 段名逐对改写到随域改名的
//!   新资源名；唯一索引冲突预检 RAISE EXCEPTION，一对多无法按裸值判域的旧段设失败可见闸门。
//!
//! `down` 逐对反向 UPDATE，可独立回退。

mod m0001_rename_role_permission_resource_type;

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_finance_resource_rename"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0001_rename_role_permission_resource_type::Migration
            .up(manager)
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0001_rename_role_permission_resource_type::Migration
            .down(manager)
            .await?;
        Ok(())
    }
}
