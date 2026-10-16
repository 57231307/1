//! 公司档案服务端化建表域
//!
//! 本域只落公司档案单例行表 `company_profile`,登记为一条顶层迁移台账:
//! - m0101_create_company_profile: 公司档案表 `company_profile`(全表仅一行,主键 `id` 恒 1,
//!   以具名 CHECK `ck_company_profile_singleton` 固化单例不变式;建表即播种 id=1 空名行)。
//!
//! 注册位置:全链尾。作为新的顶层域而非并入既有域——并入已应用的域在复用库上会因台账
//! 已记账而被整体跳过,导致该表缺席;链尾新域名在复用库上属 pending,必然执行。
//! `down` 精确 DROP 本表,单例行无外部引用,可完整回退。

mod m0101_create_company_profile;

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_company_profile"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0101_create_company_profile::Migration.up(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0101_create_company_profile::Migration
            .down(manager)
            .await?;
        Ok(())
    }
}
