//! 数据库迁移模块
//!
//! 按业务域聚合，每个域 1 个迁移文件，共 6 个：
//! system → business → sales_crm → production → finance → v15
//!
//! 顺序说明：
//! 1. system: 核心表（含 customers.owner_id/suppliers.created_by 补列）
//! 2. business: 业务表（依赖 system 的基础表）
//! 3. sales_crm: 销售报价（依赖 business 的 product_color_prices）
//! 4. production: 生产/质量（依赖 business 的 custom_orders/process_nodes）
//! 5. finance: 合规/RLS（依赖 system 的 customers.owner_id/suppliers.created_by）
//! 6. v15: V15 各批次扩展

pub use sea_orm_migration::prelude::*;

pub mod domain;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(domain::system::Migration),
            Box::new(domain::business::Migration),
            Box::new(domain::sales_crm::Migration),
            Box::new(domain::production::Migration),
            Box::new(domain::finance::Migration),
            Box::new(domain::v15::Migration),
        ]
    }
}
