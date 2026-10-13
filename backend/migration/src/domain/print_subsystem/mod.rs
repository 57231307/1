//! 打印子系统建表域
//!
//! 本域只落打印子系统的两张表,登记为一条顶层迁移台账:
//! - m0098_create_print_templates: 打印模板主数据表 `print_templates`。
//! - m0099_create_print_tasks: 打印任务台账表 `print_tasks`(`template_id` 外键引用模板表)。
//!
//! 注册位置:全链尾。作为新的顶层域而非并入既有域——并入已应用的域在复用库上会因台账
//! 已记账而被整体跳过,导致这两张表缺席;链尾新域名在复用库上属 pending,必然执行。
//! 域内 `up` 先建模板再建任务以满足外键依赖,`down` 逆序(先删任务再删模板)。

mod m0098_create_print_templates;
mod m0099_create_print_tasks;

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_print_subsystem"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0098_create_print_templates::Migration.up(manager).await?;
        m0099_create_print_tasks::Migration.up(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0099_create_print_tasks::Migration.down(manager).await?;
        m0098_create_print_templates::Migration
            .down(manager)
            .await?;
        Ok(())
    }
}
