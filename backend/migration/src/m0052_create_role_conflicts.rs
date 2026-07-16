use sea_orm_migration::prelude::*;

// V15 P0-S05 修复：SoD（职责分离）互斥角色表
//
// 记录互斥角色对，防止用户同时持有冲突角色（如制单+审核、采购+付款）。
// 财务三权分立：制单/审核/支付互斥
// 采购与供应商管理互斥
// 销售与财务收款互斥
//
// check_role_conflict(user_id, new_role_id) 在用户分配角色时校验。

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 创建 role_conflicts 表
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TABLE IF NOT EXISTS role_conflicts (
                    id SERIAL PRIMARY KEY,
                    role_a_code VARCHAR(50) NOT NULL,
                    role_b_code VARCHAR(50) NOT NULL,
                    conflict_type VARCHAR(50) NOT NULL DEFAULT 'sod',
                    description VARCHAR(200),
                    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    -- 确保 role_a_code < role_b_code 避免重复对（A-B 和 B-A）
                    CONSTRAINT chk_role_order CHECK (role_a_code < role_b_code),
                    CONSTRAINT uniq_role_pair UNIQUE (role_a_code, role_b_code)
                );

                -- 创建索引加速查询
                CREATE INDEX idx_role_conflicts_a ON role_conflicts (role_a_code);
                CREATE INDEX idx_role_conflicts_b ON role_conflicts (role_b_code);

                -- 预置财务三权分立互斥规则
                INSERT INTO role_conflicts (role_a_code, role_b_code, conflict_type, description) VALUES
                    ('accountant', 'finance_manager', 'sod', '财务制单与审核互斥'),
                    ('accountant', 'cashier', 'sod', '财务制单与出纳互斥'),
                    ('finance_manager', 'cashier', 'sod', '财务审核与出纳互斥'),
                    -- 采购与付款互斥
                    ('purchase_manager', 'cashier', 'sod', '采购审批与付款互斥'),
                    ('purchase_clerk', 'cashier', 'sod', '采购执行与付款互斥'),
                    -- 销售与收款互斥
                    ('sales_manager', 'cashier', 'sod', '销售审批与收款互斥'),
                    -- 生产与质量互斥
                    ('production_manager', 'qc_manager', 'sod', '生产与质量管理互斥'),
                    ('dyeing_master', 'quality_inspector', 'sod', '染色主管与质检员互斥')
                ON CONFLICT (role_a_code, role_b_code) DO NOTHING;
                "#,
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(r#"DROP TABLE IF EXISTS role_conflicts;"#)
            .await?;
        Ok(())
    }
}
