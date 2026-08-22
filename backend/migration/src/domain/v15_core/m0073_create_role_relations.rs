use sea_orm_migration::prelude::*;

// V15 P1 Batch-10 12.2：角色关系表（Role Relations）
//
// 业务场景：
// - 角色继承：sales_manager 继承 sales 的所有权限 + 额外审批权限
// - 权限互斥：finance 与 sales 不能同时拥有（财务与销售职责分离）
// - 系统校验：用户分配角色时检查互斥规则
//
// 设计要点：
// - relation_type: 'inherit'（继承）或 'mutual_exclusive'（互斥）
// - 对于 inherit：parent_role_code 继承 child_role_code 的所有权限
// - 对于 mutual_exclusive：两个角色不可同时分配给同一用户
//
// 预置数据：
// - 继承关系：sales_manager 继承 sales_rep / purchase_manager 继承 purchase_clerk 等
// - 互斥关系：参考 role_conflicts 表已有的财务三权分立等（不重复插入）
//
// 注：与 role_conflicts 表（m0052，sod 类型）互补：
// - role_conflicts 仅用于 SoD 互斥（财务制单/审核/出纳等）
// - role_relations 用于更通用的继承 + 互斥（覆盖业务角色）

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TABLE IF NOT EXISTS "role_relations" (
                    "id" BIGSERIAL PRIMARY KEY,
                    "parent_role_code" VARCHAR(50) NOT NULL,
                    "child_role_code" VARCHAR(50) NOT NULL,
                    "relation_type" VARCHAR(30) NOT NULL,
                    "description" VARCHAR(200),
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    -- 对于 inherit：parent 继承 child 的权限
                    -- 对于 mutual_exclusive：parent 与 child 不可同时持有
                    CONSTRAINT "chk_relation_type" CHECK (
                        "relation_type" IN ('inherit', 'mutual_exclusive')
                    ),
                    -- 防止自引用
                    CONSTRAINT "chk_no_self_relation" CHECK (
                        "parent_role_code" <> "child_role_code"
                    ),
                    -- 同一对关系不可重复
                    CONSTRAINT "uniq_role_relation" UNIQUE ("parent_role_code", "child_role_code", "relation_type")
                );

                CREATE INDEX IF NOT EXISTS "idx_role_relations_parent" ON "role_relations"("parent_role_code");
                CREATE INDEX IF NOT EXISTS "idx_role_relations_child" ON "role_relations"("child_role_code");
                CREATE INDEX IF NOT EXISTS "idx_role_relations_type" ON "role_relations"("relation_type");

                COMMENT ON TABLE "role_relations" IS
                    'V15 P1 12.2：角色关系表，支持角色继承（inherit）与互斥（mutual_exclusive）校验';
                COMMENT ON COLUMN "role_relations"."relation_type" IS
                    '关系类型：inherit（继承权限）/ mutual_exclusive（不可同时持有）';

                -- 预置继承关系：经理角色继承执行角色权限
                INSERT INTO "role_relations" ("parent_role_code", "child_role_code", "relation_type", "description") VALUES
                    ('sales_manager', 'sales_rep', 'inherit', '销售经理继承销售代表权限'),
                    ('purchase_manager', 'purchase_clerk', 'inherit', '采购经理继承采购员权限'),
                    ('inventory_manager', 'warehouse_keeper', 'inherit', '库存经理继承仓库管理员权限'),
                    ('qc_manager', 'quality_inspector', 'inherit', '质量管理经理继承质检员权限'),
                    ('finance_manager', 'accountant', 'inherit', '财务经理继承会计权限'),
                    ('hr_manager', 'hr_specialist', 'inherit', '人事经理继承人事专员权限'),
                    ('crm_manager', 'crm_rep', 'inherit', 'CRM经理继承CRM专员权限')
                ON CONFLICT ("parent_role_code", "child_role_code", "relation_type") DO NOTHING;

                -- 预置互斥关系：业务角色不可同时持有（补充 role_conflicts 表）
                INSERT INTO "role_relations" ("parent_role_code", "child_role_code", "relation_type", "description") VALUES
                    ('sales_rep', 'accountant', 'mutual_exclusive', '销售与会计互斥（防止销售操控账务）'),
                    ('sales_rep', 'cashier', 'mutual_exclusive', '销售与出纳互斥（防止销售收款舞弊）'),
                    ('purchase_clerk', 'accountant', 'mutual_exclusive', '采购与会计互斥（防止采购舞弊）'),
                    ('warehouse_keeper', 'accountant', 'mutual_exclusive', '仓库与会计互斥（防止库存账务舞弊）')
                ON CONFLICT ("parent_role_code", "child_role_code", "relation_type") DO NOTHING;
                "#,
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(r#"DROP TABLE IF EXISTS "role_relations";"#)
            .await?;
        Ok(())
    }
}
