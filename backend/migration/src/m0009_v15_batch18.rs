use sea_orm_migration::prelude::*;

pub mod m0076_add_export_audit_fields;
pub mod m0077_add_oa_visibility_consent_retention;
pub mod m0078_batch18_greige_outsourcing_quality_scheduling;
pub mod m0079_batch08_compliance_legal_env_tax_labor;
pub mod m0080_create_collection_templates;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0076_add_export_audit_fields::Migration.up(manager).await?;
        m0077_add_oa_visibility_consent_retention::Migration.up(manager).await?;
        m0078_batch18_greige_outsourcing_quality_scheduling::Migration.up(manager).await?;
        m0079_batch08_compliance_legal_env_tax_labor::Migration.up(manager).await?;
        m0080_create_collection_templates::Migration.up(manager).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
