//! print_templates 默认位库侧唯一门域
//!
//! 本域只落一支迁移，登记为一条顶层迁移台账：
//! - m0120_add_print_template_active_default_unique: 给打印模板表按单据类型
//!   doc_type 建"启用且默认至多一行"的部分唯一索引，并更正 is_default 列注释，
//!   使注释如实描述互斥域是 doc_type 而非 module。
//!
//! 注册位置：全链尾。作为新的顶层域而非并入 print_subsystem 既有域——并入已应用
//! 的域在复用库上会因台账已记账而被整体跳过，导致索引与注释更正永不落地；链尾
//! 新域名在复用库上属 pending，必然执行。print_templates 表由 print_subsystem 域
//! 的 m0098 建表，该域早于本域，表必已存在。
//!
//! 台账名取域名，注册在 lib.rs 迁移链尾；up 与 down 逐一委托域内单支，对称可回退。

mod m0120_add_print_template_active_default_unique;

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_print_template_default_slot"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0120_add_print_template_active_default_unique::Migration
            .up(manager)
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0120_add_print_template_active_default_unique::Migration
            .down(manager)
            .await?;
        Ok(())
    }
}
