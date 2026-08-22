//! 数据库迁移模块
//!
//! 迁移文件按业务域合并，结构清晰：
//! - 01-03: 基础迁移（核心表/业务表/修复）
//! - 04-06: 中期迁移（销售CRM/生产品质/财务合规）
//! - 07-11: 近期迁移（V15 各批次）

pub use sea_orm_migration::prelude::*;

pub mod domain;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(domain::system::Migration),
            Box::new(domain::business::Migration),
            Box::new(domain::fixes::Migration),
            Box::new(domain::sales_crm::Migration),
            Box::new(domain::production::Migration),
            Box::new(domain::finance::Migration),
            Box::new(domain::v15_core::Migration),
            Box::new(domain::v15_batch18::Migration),
            Box::new(domain::v15_batch19::Migration),
            Box::new(domain::v15_extensions::Migration),
            Box::new(domain::v15_final::Migration),
        ]
    }
}
