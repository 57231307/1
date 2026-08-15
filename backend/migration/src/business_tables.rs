//! 业务表：生产/采购/库存/销售/财务
//!
//! 合并自: 8 个迁移文件

use sea_orm_migration::prelude::*;

// 导入所有迁移模块
mod m0007_add_mrp_production_bom;
mod m0008_add_supplier_and_product_extensions;
mod m0009_add_purchase_extensions;
mod m0010_add_inventory_extensions;
mod m0011_add_sales_and_logistics_extensions;
mod m0012_add_ap_ar_finance_analysis;
mod m0013_add_business_process_and_traceability;
mod m0014_add_saas_notification_report_email_oa;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 依次执行所有迁移
        m0007_add_mrp_production_bom::Migration.up(manager).await?;
        m0008_add_supplier_and_product_extensions::Migration.up(manager).await?;
        m0009_add_purchase_extensions::Migration.up(manager).await?;
        m0010_add_inventory_extensions::Migration.up(manager).await?;
        m0011_add_sales_and_logistics_extensions::Migration.up(manager).await?;
        m0012_add_ap_ar_finance_analysis::Migration.up(manager).await?;
        m0013_add_business_process_and_traceability::Migration.up(manager).await?;
        m0014_add_saas_notification_report_email_oa::Migration.up(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 依次回滚所有迁移（逆序）
        m0014_add_saas_notification_report_email_oa::Migration.down(manager).await?;
        m0013_add_business_process_and_traceability::Migration.down(manager).await?;
        m0012_add_ap_ar_finance_analysis::Migration.down(manager).await?;
        m0011_add_sales_and_logistics_extensions::Migration.down(manager).await?;
        m0010_add_inventory_extensions::Migration.down(manager).await?;
        m0009_add_purchase_extensions::Migration.down(manager).await?;
        m0008_add_supplier_and_product_extensions::Migration.down(manager).await?;
        m0007_add_mrp_production_bom::Migration.down(manager).await?;
        Ok(())
    }
}
