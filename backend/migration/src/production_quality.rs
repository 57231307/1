//! 生产/质量/委外
//!
//! 合并自: 10 个迁移文件

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // === m0041_create_import_tasks.rs ===
let sql = include_str!("../../migrations/20260705000003_create_import_tasks/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
        // === m0042_create_purchase_inspection_items.rs ===
let sql =
            include_str!("../../migrations/20260705000004_create_purchase_inspection_items/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
        // === m0043_add_scan_type_and_industry_columns.rs ===
let sql = include_str!(
            "../../migrations/20260706000006_add_scan_type_and_industry_columns/up.sql"
        );
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
        // === m0044_integrate_unreferenced_migrations.rs ===
let db = manager.get_connection();
        for (name, sql) in UNREFERENCED_MIGRATIONS {
            if !sql.trim().is_empty() {
                // 修复 BIGINT 外键类型不匹配后再执行
                let fixed_sql = fix_fk_types(sql);
                db.execute_unprepared(&fixed_sql)
                    .await
                    .map_err(|e| DbErr::Custom(format!("执行整合迁移 {} 失败: {}", name, e)))?;
            }
        }
        Ok(())
        // === m0045_add_password_changed_at_to_users.rs ===
let sql =
            include_str!("../../migrations/20260708000001_add_password_changed_at_to_users/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
        // === m0046_drop_audit_alert_rules.rs ===
let sql = include_str!("../../migrations/20260708000002_drop_audit_alert_rules/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
        // === m0047_add_last_payload_to_webhooks.rs ===
let sql =
            include_str!("../../migrations/20260710000001_add_last_payload_to_webhooks/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
        // === m0048_add_user_id_to_webhooks.rs ===
let sql = include_str!("../../migrations/20260712000001_add_user_id_to_webhooks/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
        // === m0049_create_processed_events.rs ===
let sql = include_str!("../../migrations/20260713000001_create_processed_events/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
        // === m0050_create_event_dead_letters.rs ===
let sql = include_str!("../../migrations/20260714000001_create_event_dead_letters/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
