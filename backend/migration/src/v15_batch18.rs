//! V15 批次18
//!
//! 合并自: 5 个迁移文件

use sea_orm_migration::prelude::*;

// 导入所有迁移模块
mod m0076_add_export_audit_fields;
mod m0077_add_oa_visibility_consent_retention;
mod m0078_batch18_greige_outsourcing_quality_scheduling;
mod m0079_batch08_compliance_legal_env_tax_labor;
mod m0080_create_collection_templates;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 依次执行所有迁移
        m0076_add_export_audit_fields::Migration.up(manager).await?;
        m0077_add_oa_visibility_consent_retention::Migration.up(manager).await?;
        m0078_batch18_greige_outsourcing_quality_scheduling::Migration.up(manager).await?;
        m0079_batch08_compliance_legal_env_tax_labor::Migration.up(manager).await?;
        m0080_create_collection_templates::Migration.up(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 依次回滚所有迁移（逆序）
        m0080_create_collection_templates::Migration.down(manager).await?;
        m0079_batch08_compliance_legal_env_tax_labor::Migration.down(manager).await?;
        m0078_batch18_greige_outsourcing_quality_scheduling::Migration.down(manager).await?;
        m0077_add_oa_visibility_consent_retention::Migration.down(manager).await?;
        m0076_add_export_audit_fields::Migration.down(manager).await?;
        Ok(())
    }
}
