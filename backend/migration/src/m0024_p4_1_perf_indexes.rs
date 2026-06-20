//! P4-1 性能优化 - 新增复合索引
//!
//! 业务背景：
//! 1. 销售订单按 `tenant_id + customer_id + status` 频繁查询（仪表盘、报表）
//! 2. 库存按 `tenant_id + warehouse_id + product_id` 频繁查询（盘点、库存预警）
//! 3. 应收账款按 `tenant_id + customer_id + due_date` 频繁查询（账龄分析）
//! 4. 采购订单按 `tenant_id + supplier_id + status` 频繁查询（跟单、采购报表）
//! 5. 库存预留按 `tenant_id + product_id + status` 频繁查询（可用库存计算）
//! 6. 操作日志按 `tenant_id + created_at` 频繁查询（审计追溯）
//! 7. 用户按 `tenant_id + username` 唯一约束（登录）

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // 1. 销售订单按租户+客户+状态
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_sales_orders_tenant_customer_status
             ON sales_orders (tenant_id, customer_id, status);",
        )
        .await?;

        // 2. 库存按租户+仓库+商品
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_inventory_stocks_tenant_wh_product
             ON inventory_stocks (tenant_id, warehouse_id, product_id);",
        )
        .await?;

        // 3. 应收账款按租户+客户+到期日
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_ar_invoices_tenant_customer_due
             ON ar_invoices (tenant_id, customer_id, due_date);",
        )
        .await?;

        // 4. 采购订单按租户+供应商+状态
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_purchase_orders_tenant_supplier_status
             ON purchase_orders (tenant_id, supplier_id, status);",
        )
        .await?;

        // 5. 库存预留按租户+商品+状态
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_inventory_reservations_tenant_product_status
             ON inventory_reservations (tenant_id, product_id, status);",
        )
        .await?;

        // 6. 操作日志按租户+创建时间
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_operation_logs_tenant_created
             ON operation_logs (tenant_id, created_at DESC);",
        )
        .await?;

        // 7. 用户名唯一约束（按租户隔离）
        db.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS uq_users_tenant_username
             ON users (tenant_id, username);",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        // 回滚：删除索引
        for stmt in &[
            "DROP INDEX IF EXISTS idx_sales_orders_tenant_customer_status;",
            "DROP INDEX IF EXISTS idx_inventory_stocks_tenant_wh_product;",
            "DROP INDEX IF EXISTS idx_ar_invoices_tenant_customer_due;",
            "DROP INDEX IF EXISTS idx_purchase_orders_tenant_supplier_status;",
            "DROP INDEX IF EXISTS idx_inventory_reservations_tenant_product_status;",
            "DROP INDEX IF EXISTS idx_operation_logs_tenant_created;",
            "DROP INDEX IF EXISTS uq_users_tenant_username;",
        ] {
            db.execute_unprepared(stmt).await?;
        }
        Ok(())
    }
}
