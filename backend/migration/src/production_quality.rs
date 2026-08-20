//! 生产/质量/委外
//!
//! 合并自: 16 个迁移文件

use sea_orm_migration::prelude::*;

// 导入所有迁移模块
mod m0025_p4_1_perf_indexes;
mod m0026_extend_audit_log;
mod m0027_enable_pg_stat_statements;
mod m0028_create_slow_query_log;
mod m0029_drop_tenant_columns;
mod m0030_create_crm_recycle_rules;
mod m0041_create_import_tasks;
mod m0042_create_purchase_inspection_items;
mod m0043_add_scan_type_and_industry_columns;
mod m0044_integrate_unreferenced_migrations;
mod m0045_add_password_changed_at_to_users;
mod m0046_drop_audit_alert_rules;
mod m0047_add_last_payload_to_webhooks;
mod m0048_add_user_id_to_webhooks;
mod m0049_create_processed_events;
mod m0050_create_event_dead_letters;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 依次执行所有迁移
        m0025_p4_1_perf_indexes::Migration.up(manager).await?;
        m0026_extend_audit_log::Migration.up(manager).await?;
        m0027_enable_pg_stat_statements::Migration
            .up(manager)
            .await?;
        m0028_create_slow_query_log::Migration.up(manager).await?;
        // 顺序修复：m0044 必须在 m0029 之前执行。
        // m0044 创建 custom_orders / color_cards / process_nodes / sales_facts 等 31 张表，
        // m0029 随后对这些表执行 ALTER TABLE ... DROP COLUMN IF EXISTS "tenant_id"。
        // PostgreSQL 的 DROP COLUMN IF EXISTS 仅保护列不存在，不保护表不存在；
        // 若 m0029 先于 m0044 执行，会因 "relation xxx does not exist" 中断整个迁移链。
        // 这也符合 m0044 文件头注释的设计意图："注册在 m0028 之后、m0029 之前"。
        m0044_integrate_unreferenced_migrations::Migration
            .up(manager)
            .await?;
        m0029_drop_tenant_columns::Migration.up(manager).await?;
        m0030_create_crm_recycle_rules::Migration
            .up(manager)
            .await?;
        m0041_create_import_tasks::Migration.up(manager).await?;
        m0042_create_purchase_inspection_items::Migration
            .up(manager)
            .await?;
        m0043_add_scan_type_and_industry_columns::Migration
            .up(manager)
            .await?;
        m0045_add_password_changed_at_to_users::Migration
            .up(manager)
            .await?;
        m0046_drop_audit_alert_rules::Migration.up(manager).await?;
        m0047_add_last_payload_to_webhooks::Migration
            .up(manager)
            .await?;
        m0048_add_user_id_to_webhooks::Migration.up(manager).await?;
        m0049_create_processed_events::Migration.up(manager).await?;
        m0050_create_event_dead_letters::Migration
            .up(manager)
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 依次回滚所有迁移（逆序）
        m0050_create_event_dead_letters::Migration
            .down(manager)
            .await?;
        m0049_create_processed_events::Migration
            .down(manager)
            .await?;
        m0048_add_user_id_to_webhooks::Migration
            .down(manager)
            .await?;
        m0047_add_last_payload_to_webhooks::Migration
            .down(manager)
            .await?;
        m0046_drop_audit_alert_rules::Migration
            .down(manager)
            .await?;
        m0045_add_password_changed_at_to_users::Migration
            .down(manager)
            .await?;
        m0043_add_scan_type_and_industry_columns::Migration
            .down(manager)
            .await?;
        m0042_create_purchase_inspection_items::Migration
            .down(manager)
            .await?;
        m0041_create_import_tasks::Migration.down(manager).await?;
        m0030_create_crm_recycle_rules::Migration
            .down(manager)
            .await?;
        m0029_drop_tenant_columns::Migration.down(manager).await?;
        // 逆序：m0044 在 m0029 之后回滚（与 up 顺序相反）
        m0044_integrate_unreferenced_migrations::Migration
            .down(manager)
            .await?;
        m0028_create_slow_query_log::Migration.down(manager).await?;
        m0027_enable_pg_stat_statements::Migration
            .down(manager)
            .await?;
        m0026_extend_audit_log::Migration.down(manager).await?;
        m0025_p4_1_perf_indexes::Migration.down(manager).await?;
        Ok(())
    }
}
