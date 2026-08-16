# Rust 格式检查报告

**生成时间**: 2026-08-16T02:29:57Z  
**首次检查退出码**: 1  
**模式**: 自动修正（失败时执行 cargo fmt --all 并提交）

## ✅ 已自动修正（cargo fmt --all）

### 修复前 Diff 内容（前 100 行）

```diff
info: syncing channel updates for 1.94-x86_64-unknown-linux-gnu
info: latest update on 2026-03-26 for version 1.94.1 (e408947bf 2026-03-25)
info: downloading 5 components
Diff in /home/runner/work/1/1/backend/migration/src/business_tables/m0007_add_mrp_production_bom.rs:14:
     }
 
     async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
-        let sql = include_str!("../../../migrations/20260527000003_add_mrp_production_bom/down.sql");
+        let sql =
+            include_str!("../../../migrations/20260527000003_add_mrp_production_bom/down.sql");
         if !sql.trim().is_empty() {
             manager.get_connection().execute_unprepared(sql).await?;
         }
Diff in /home/runner/work/1/1/backend/migration/src/business_tables/m0009_add_purchase_extensions.rs:14:
     }
 
     async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
-        let sql = include_str!("../../../migrations/20260527000005_add_purchase_extensions/down.sql");
+        let sql =
+            include_str!("../../../migrations/20260527000005_add_purchase_extensions/down.sql");
         if !sql.trim().is_empty() {
             manager.get_connection().execute_unprepared(sql).await?;
         }
Diff in /home/runner/work/1/1/backend/migration/src/business_tables/m0010_add_inventory_extensions.rs:6:
 #[async_trait::async_trait]
 impl MigrationTrait for Migration {
     async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
-        let sql = include_str!("../../../migrations/20260527000006_add_inventory_extensions/up.sql");
+        let sql =
+            include_str!("../../../migrations/20260527000006_add_inventory_extensions/up.sql");
         if !sql.trim().is_empty() {
             manager.get_connection().execute_unprepared(sql).await?;
         }
Diff in /home/runner/work/1/1/backend/migration/src/business_tables/m0010_add_inventory_extensions.rs:14:
     }
 
     async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
-        let sql = include_str!("../../../migrations/20260527000006_add_inventory_extensions/down.sql");
+        let sql =
+            include_str!("../../../migrations/20260527000006_add_inventory_extensions/down.sql");
         if !sql.trim().is_empty() {
             manager.get_connection().execute_unprepared(sql).await?;
         }
Diff in /home/runner/work/1/1/backend/migration/src/business_tables/m0012_add_ap_ar_finance_analysis.rs:6:
 #[async_trait::async_trait]
 impl MigrationTrait for Migration {
     async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
-        let sql = include_str!("../../../migrations/20260527000008_add_ap_ar_finance_analysis/up.sql");
+        let sql =
+            include_str!("../../../migrations/20260527000008_add_ap_ar_finance_analysis/up.sql");
         if !sql.trim().is_empty() {
             manager.get_connection().execute_unprepared(sql).await?;
         }
Diff in /home/runner/work/1/1/backend/migration/src/business_tables.rs:22:
     async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
         // 依次执行所有迁移
         m0007_add_mrp_production_bom::Migration.up(manager).await?;
-        m0008_add_supplier_and_product_extensions::Migration.up(manager).await?;
+        m0008_add_supplier_and_product_extensions::Migration
+            .up(manager)
+            .await?;
         m0009_add_purchase_extensions::Migration.up(manager).await?;
-        m0010_add_inventory_extensions::Migration.up(manager).await?;
-        m0011_add_sales_and_logistics_extensions::Migration.up(manager).await?;
-        m0012_add_ap_ar_finance_analysis::Migration.up(manager).await?;
-        m0013_add_business_process_and_traceability::Migration.up(manager).await?;
-        m0014_add_saas_notification_report_email_oa::Migration.up(manager).await?;
+        m0010_add_inventory_extensions::Migration
+            .up(manager)
+            .await?;
+        m0011_add_sales_and_logistics_extensions::Migration
+            .up(manager)
+            .await?;
+        m0012_add_ap_ar_finance_analysis::Migration
+            .up(manager)
+            .await?;
+        m0013_add_business_process_and_traceability::Migration
+            .up(manager)
+            .await?;
+        m0014_add_saas_notification_report_email_oa::Migration
+            .up(manager)
+            .await?;
         Ok(())
     }
 
Diff in /home/runner/work/1/1/backend/migration/src/business_tables.rs:35:
     async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
         // 依次回滚所有迁移（逆序）
-        m0014_add_saas_notification_report_email_oa::Migration.down(manager).await?;
-        m0013_add_business_process_and_traceability::Migration.down(manager).await?;
-        m0012_add_ap_ar_finance_analysis::Migration.down(manager).await?;
-        m0011_add_sales_and_logistics_extensions::Migration.down(manager).await?;
-        m0010_add_inventory_extensions::Migration.down(manager).await?;
-        m0009_add_purchase_extensions::Migration.down(manager).await?;
-        m0008_add_supplier_and_product_extensions::Migration.down(manager).await?;
-        m0007_add_mrp_production_bom::Migration.down(manager).await?;
+        m0014_add_saas_notification_report_email_oa::Migration
+            .down(manager)
+            .await?;
+        m0013_add_business_process_and_traceability::Migration
```

*自动修正已提交到本分支，CI 将基于修正后代码继续。*
