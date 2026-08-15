use sea_orm_migration::prelude::*;

pub mod m0041_create_import_tasks;
pub mod m0042_create_purchase_inspection_items;
pub mod m0043_add_scan_type_and_industry_columns;
pub mod m0044_integrate_unreferenced_migrations;
pub mod m0045_add_password_changed_at_to_users;
pub mod m0046_drop_audit_alert_rules;
pub mod m0047_add_last_payload_to_webhooks;
pub mod m0048_add_user_id_to_webhooks;
pub mod m0049_create_processed_events;
pub mod m0050_create_event_dead_letters;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0041_create_import_tasks::Migration.up(manager).await?;
        m0042_create_purchase_inspection_items::Migration.up(manager).await?;
        m0043_add_scan_type_and_industry_columns::Migration.up(manager).await?;
        m0044_integrate_unreferenced_migrations::Migration.up(manager).await?;
        m0045_add_password_changed_at_to_users::Migration.up(manager).await?;
        m0046_drop_audit_alert_rules::Migration.up(manager).await?;
        m0047_add_last_payload_to_webhooks::Migration.up(manager).await?;
        m0048_add_user_id_to_webhooks::Migration.up(manager).await?;
        m0049_create_processed_events::Migration.up(manager).await?;
        m0050_create_event_dead_letters::Migration.up(manager).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
