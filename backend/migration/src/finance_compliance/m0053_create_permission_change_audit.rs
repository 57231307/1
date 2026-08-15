use sea_orm_migration::prelude::*;

// V15 P0-S06 修复：权限变更审计表
//
// 记录角色权限变更和用户角色变更的审计日志，用于合规审查和安全追溯。
// 每当 role_permission 表发生变更（assign/remove）或用户角色变更时写入记录。

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TABLE IF NOT EXISTS permission_change_audits (
                    id SERIAL PRIMARY KEY,
                    -- 变更类型：role_permission_assign / role_permission_remove / user_role_change
                    change_type VARCHAR(50) NOT NULL,
                    -- 操作人 ID
                    operator_id INTEGER NOT NULL,
                    -- 受影响角色 ID
                    role_id INTEGER,
                    -- 受影响用户 ID（user_role_change 时有值）
                    user_id INTEGER,
                    -- 资源类型（role_permission 变更时有值）
                    resource_type VARCHAR(100),
                    -- 操作权限码（role_permission 变更时有值）
                    action VARCHAR(50),
                    -- 旧值（如旧 role_id / 旧 allowed）
                    old_value VARCHAR(200),
                    -- 新值（如新 role_id / 新 allowed）
                    new_value VARCHAR(200),
                    -- 变更时间
                    changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    -- 客户端 IP
                    client_ip VARCHAR(45),
                    -- 备注
                    remark TEXT
                );

                -- 创建索引加速查询
                CREATE INDEX idx_pca_change_type ON permission_change_audits (change_type);
                CREATE INDEX idx_pca_operator ON permission_change_audits (operator_id);
                CREATE INDEX idx_pca_role ON permission_change_audits (role_id);
                CREATE INDEX idx_pca_user ON permission_change_audits (user_id);
                CREATE INDEX idx_pca_changed_at ON permission_change_audits (changed_at);
                "#,
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(r#"DROP TABLE IF EXISTS permission_change_audits;"#)
            .await?;
        Ok(())
    }
}
