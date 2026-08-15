use sea_orm_migration::prelude::*;

// V15 P1 batch-09 缺陷 10.4-1/10.4-2 修复：
//
// 补齐非 admin 角色的 color_card_issue:export 权限。
// 业务背景：handlers/color_card/issue.rs::export_issue_records 通过
//   require_issue_permission(&state, &auth, "export") 校验导出权限；
// can_view_cost_amount 也以 export 权限作为成本字段可见性判据。
// 在 init_admin_permissions.sql 中仅 admin 持有 color_card_issue:export，
// 销售经理/仓库经理/成本会计在执行导出/查看成本字段时被 403 拒绝，
// 违反角色权限矩阵（详见 handlers/color_card/issue.rs:409 注释）。
//
// 角色矩阵（与 init_admin_permissions.sql 现有 read/create/return/lost/damaged/cancel 对齐）：
//   - sales_manager     ：read + create + export（导出自己客户的发放记录，含成本字段）
//   - warehouse_manager ：read + create + return + lost + damaged + cancel + export（全流程导出）
//   - cost_accountant   ：read + export（成本核算需要查看补偿金额）
//
// 注：production_manager / lab_technician / dye_recipe_master 仅 read，不需 export。
//
// 关联文件：
//   - backend/database/init_admin_permissions.sql（admin 已持有 export，本迁移不重复插入）
//   - backend/src/handlers/color_card/issue.rs::export_issue_records（export 权限校验入口）
//   - backend/src/handlers/color_card/issue.rs::can_view_cost_amount（成本字段可见性判据）

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- ============================================================
                -- V15 P1 batch-09 缺陷 10.4-1/10.4-2：
                -- 为非 admin 业务角色补齐 color_card_issue:export 权限
                -- ============================================================
                --
                -- 策略：以 roles.code + roles.is_system=true 为筛选条件，
                --       避免硬编码 role_id；ON CONFLICT 保证幂等。
                --
                -- 销售经理（sales_manager）：导出自己客户的色卡发放记录 + 查看成本字段
                INSERT INTO role_permissions (role_id, resource_type, action, allowed, created_at, updated_at)
                SELECT r.id, 'color_card_issue', 'export', true, NOW(), NOW()
                FROM roles r
                WHERE r.code = 'sales_manager' AND r.is_system = true
                ON CONFLICT (role_id, resource_type, action) DO NOTHING;

                -- 仓库经理（warehouse_manager）：导出色卡发放全流程记录 + 查看成本字段
                INSERT INTO role_permissions (role_id, resource_type, action, allowed, created_at, updated_at)
                SELECT r.id, 'color_card_issue', 'export', true, NOW(), NOW()
                FROM roles r
                WHERE r.code = 'warehouse_manager' AND r.is_system = true
                ON CONFLICT (role_id, resource_type, action) DO NOTHING;

                -- 成本会计（cost_accountant）：导出/查看色卡发放成本字段（补偿金额核算）
                INSERT INTO role_permissions (role_id, resource_type, action, allowed, created_at, updated_at)
                SELECT r.id, 'color_card_issue', 'export', true, NOW(), NOW()
                FROM roles r
                WHERE r.code = 'cost_accountant' AND r.is_system = true
                ON CONFLICT (role_id, resource_type, action) DO NOTHING;
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
                -- 回滚：撤销本迁移为非 admin 角色授予的 color_card_issue:export 权限
                -- 仅删除 sales_manager / warehouse_manager / cost_accountant 的 export 权限，
                -- 不影响 admin 的同名权限（admin 由 init_admin_permissions.sql 维护）。
                DELETE FROM role_permissions rp
                USING roles r
                WHERE rp.role_id = r.id
                  AND rp.resource_type = 'color_card_issue'
                  AND rp.action = 'export'
                  AND r.code IN ('sales_manager', 'warehouse_manager', 'cost_accountant')
                  AND r.is_system = true;
                "#,
            )
            .await?;
        Ok(())
    }
}
