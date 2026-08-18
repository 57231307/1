use sea_orm_migration::prelude::*;

// V15 P1 Batch-10 12.1：user_role 关联表（多对多，支持一个用户多角色）
//
// 背景：
// - 当前 users 表通过 role_id 单字段关联角色，仅支持单角色
// - 审计计划 12.1.2 要求支持多角色（销售经理同时是销售、销售+财务互斥校验等）
// - 通过 user_role 关联表实现多对多关系
//
// 兼容策略：
// - 保留 user.role_id 字段（向后兼容，作为"主角色"）
// - 新增 user_role 关联表，存储用户的所有角色
// - 权限校验中间件聚合用户的所有角色权限
//
// 索引设计：
// - UNIQUE(user_id, role_id)：防止重复分配
// - INDEX(user_id)：按用户查询角色列表
// - INDEX(role_id)：按角色查询用户列表（角色删除/权限变更时失效缓存）

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TABLE IF NOT EXISTS user_role (
                    id BIGSERIAL PRIMARY KEY,
                    user_id INTEGER NOT NULL REFERENCES "users"("id") ON DELETE CASCADE,
                    role_id INTEGER NOT NULL REFERENCES "roles"("id") ON DELETE CASCADE,
                    assigned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    assigned_by INTEGER REFERENCES "users"("id") ON DELETE SET NULL,
                    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    -- 防止同一对 (user_id, role_id) 重复分配
                    CONSTRAINT uniq_user_role UNIQUE (user_id, role_id)
                );

                CREATE INDEX IF NOT EXISTS idx_user_role_user_id ON user_role (user_id);
                CREATE INDEX IF NOT EXISTS idx_user_role_role_id ON user_role (role_id);

                COMMENT ON TABLE user_role IS 'V15 P1 12.1：用户-角色多对多关联表，支持一个用户多角色';
                COMMENT ON COLUMN user_role.assigned_at IS '角色分配时间，用于审计追溯';
                COMMENT ON COLUMN user_role.assigned_by IS '分配人 user_id（审计用），可能为 NULL（系统初始化）';

                -- 数据迁移：将 users.role_id 现有数据同步到 user_role 表（仅迁移非空记录）
                -- 使用 ON CONFLICT DO NOTHING 保证幂等（重复执行不报错）
                INSERT INTO user_role (user_id, role_id, assigned_at)
                SELECT u.id, u.role_id, NOW()
                FROM "users" u
                WHERE u.role_id IS NOT NULL
                ON CONFLICT (user_id, role_id) DO NOTHING;
                "#,
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(r#"DROP TABLE IF EXISTS user_role;"#)
            .await?;
        Ok(())
    }
}
