use sea_orm_migration::prelude::*;

// Batch 464 P0-S25 修复：行级数据权限 RLS 策略启用
//
// 为 5 张敏感表启用 PostgreSQL Row Level Security：
//   customers / suppliers / sales_orders / crm_lead / crm_opportunity
//
// 设计原则：
// 1. 纵深防御：应用层 apply_data_scope（utils/data_scope.rs）已覆盖，
//    RLS 作为数据库层兜底防线，防止 SQL 注入与直连 DB 越权
// 2. 安全降级：app.user_id 未设置时（应用层未接入 SET LOCAL），
//    current_setting 返回 NULL，NULL IS NULL 为 true，允许所有访问
// 3. 角色分层：admin/gm/deputy_gm 可见所有数据；其他角色按 owner_id/created_by 过滤
// 4. 公海数据：customers.owner_id=0 对所有用户可见；
//    suppliers/sales_orders.created_by IS NULL 历史数据对所有用户可见
//
// 注意：不使用 FORCE ROW LEVEL SECURITY，避免限制表 owner 访问
// 完整策略定义见 backend/database/rls.sql

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- 1. customers 表（owner_id NOT NULL，0 表示公海）
                ALTER TABLE customers ENABLE ROW LEVEL SECURITY;

                CREATE POLICY customers_isolation ON customers
                    FOR ALL
                    USING (
                        current_setting('app.user_id', true) IS NULL
                        OR current_setting('app.role_code', true) IN ('admin', 'gm', 'deputy_gm')
                        OR owner_id = current_setting('app.user_id', true)::int
                        OR owner_id = 0
                    )
                    WITH CHECK (
                        current_setting('app.user_id', true) IS NULL
                        OR current_setting('app.role_code', true) IN ('admin', 'gm', 'deputy_gm')
                        OR owner_id = current_setting('app.user_id', true)::int
                        OR owner_id = 0
                    );

                -- 2. suppliers 表（created_by 可空，历史数据 NULL）
                ALTER TABLE suppliers ENABLE ROW LEVEL SECURITY;

                CREATE POLICY suppliers_isolation ON suppliers
                    FOR ALL
                    USING (
                        current_setting('app.user_id', true) IS NULL
                        OR current_setting('app.role_code', true) IN ('admin', 'gm', 'deputy_gm')
                        OR created_by IS NULL
                        OR created_by = current_setting('app.user_id', true)::int
                    )
                    WITH CHECK (
                        current_setting('app.user_id', true) IS NULL
                        OR current_setting('app.role_code', true) IN ('admin', 'gm', 'deputy_gm')
                        OR created_by IS NULL
                        OR created_by = current_setting('app.user_id', true)::int
                    );

                -- 3. sales_orders 表（created_by 可空，历史数据 NULL）
                ALTER TABLE sales_orders ENABLE ROW LEVEL SECURITY;

                CREATE POLICY sales_orders_isolation ON sales_orders
                    FOR ALL
                    USING (
                        current_setting('app.user_id', true) IS NULL
                        OR current_setting('app.role_code', true) IN ('admin', 'gm', 'deputy_gm')
                        OR created_by IS NULL
                        OR created_by = current_setting('app.user_id', true)::int
                    )
                    WITH CHECK (
                        current_setting('app.user_id', true) IS NULL
                        OR current_setting('app.role_code', true) IN ('admin', 'gm', 'deputy_gm')
                        OR created_by IS NULL
                        OR created_by = current_setting('app.user_id', true)::int
                    );

                -- 4. crm_lead 表（owner_id NOT NULL）
                ALTER TABLE crm_lead ENABLE ROW LEVEL SECURITY;

                CREATE POLICY crm_lead_isolation ON crm_lead
                    FOR ALL
                    USING (
                        current_setting('app.user_id', true) IS NULL
                        OR current_setting('app.role_code', true) IN ('admin', 'gm', 'deputy_gm')
                        OR owner_id = current_setting('app.user_id', true)::int
                    )
                    WITH CHECK (
                        current_setting('app.user_id', true) IS NULL
                        OR current_setting('app.role_code', true) IN ('admin', 'gm', 'deputy_gm')
                        OR owner_id = current_setting('app.user_id', true)::int
                    );

                -- 5. crm_opportunity 表（owner_id NOT NULL）
                ALTER TABLE crm_opportunity ENABLE ROW LEVEL SECURITY;

                CREATE POLICY crm_opportunity_isolation ON crm_opportunity
                    FOR ALL
                    USING (
                        current_setting('app.user_id', true) IS NULL
                        OR current_setting('app.role_code', true) IN ('admin', 'gm', 'deputy_gm')
                        OR owner_id = current_setting('app.user_id', true)::int
                    )
                    WITH CHECK (
                        current_setting('app.user_id', true) IS NULL
                        OR current_setting('app.role_code', true) IN ('admin', 'gm', 'deputy_gm')
                        OR owner_id = current_setting('app.user_id', true)::int
                    );
                "#,
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- 回滚顺序：先 DROP POLICY，再 DISABLE ROW LEVEL SECURITY
                DROP POLICY IF EXISTS crm_opportunity_isolation ON crm_opportunity;
                ALTER TABLE crm_opportunity DISABLE ROW LEVEL SECURITY;

                DROP POLICY IF EXISTS crm_lead_isolation ON crm_lead;
                ALTER TABLE crm_lead DISABLE ROW LEVEL SECURITY;

                DROP POLICY IF EXISTS sales_orders_isolation ON sales_orders;
                ALTER TABLE sales_orders DISABLE ROW LEVEL SECURITY;

                DROP POLICY IF EXISTS suppliers_isolation ON suppliers;
                ALTER TABLE suppliers DISABLE ROW LEVEL SECURITY;

                DROP POLICY IF EXISTS customers_isolation ON customers;
                ALTER TABLE customers DISABLE ROW LEVEL SECURITY;
                "#,
            )
            .await?;
        Ok(())
    }
}
