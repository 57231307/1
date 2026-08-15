//! 数据库迁移模块
//!
//! 迁移文件按业务域合并，结构清晰：
//! - 01-03: 基础迁移（核心表/业务表/修复）
//! - 04-06: 中期迁移（销售CRM/生产品质/财务合规）
//! - 07-11: 近期迁移（V15 各批次）

pub use sea_orm_migration::prelude::*;

pub mod core_schema;
pub mod business_tables;
pub mod fixes_enhancements;
pub mod sales_crm;
pub mod production_quality;
pub mod finance_compliance;
pub mod v15_core;
pub mod v15_batch18;
pub mod v15_batch19;
pub mod v15_extensions;
pub mod v15_final;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(core_schema::Migration),
            Box::new(business_tables::Migration),
            Box::new(fixes_enhancements::Migration),
            Box::new(sales_crm::Migration),
            Box::new(production_quality::Migration),
            Box::new(finance_compliance::Migration),
            Box::new(v15_core::Migration),
            Box::new(v15_batch18::Migration),
            Box::new(v15_batch19::Migration),
            Box::new(v15_extensions::Migration),
            Box::new(v15_final::Migration),
        ]
    }
}
