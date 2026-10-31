//! 仓储两岗补 `scan-receipt:create`：扫码入库对非超管不再恒 403（存量库通道）
//!
//! ## 六要素
//! - **功能**：给 `role_permissions` 补 `(仓库管理员/仓库经理角色码 × scan-receipt × create)` 行。
//! - **调用方**：顶层域 scan_receipt_grant（台账名 m_scan_receipt_grant）的链尾聚合迁移调用；
//!   roles/role_permissions 由 system 域先建，早于本域。立为顶层域而不并入任何已应用域：
//!   已应用域在复用库上台账已记账、整体跳过，嵌入其中的补授在存量库永远不执行。
//! - **入参**：无。角色码与资源键都是本文件的编译期常量，无外部输入拼接。
//! - **传给谁**：`SchemaManager` 的裸 SQL 连接。运行时判权键由
//!   `backend/src/middleware/permission.rs::extract_resource_info` 从
//!   `POST /api/v1/erp/scanner/scan-receipt` 取模块前缀下的第四段派生为 `scan-receipt`，
//!   动作按方法派生为 create，合成键 `scan-receipt:create`。
//! - **存什么**：只写 `role_permissions` 的 (role_id × 'scan-receipt' × 'create') 行。
//! - **存哪里**：PostgreSQL `public.role_permissions`。
//!
//! ## 受授集合取值依据
//! 逐匹扫码入库是收货现场的动作，执行人是仓库管理员与仓库经理（坯布建档口径同授这两岗）。
//! `matches_permission` 按资源段精确匹配，两岗已有的 inventory 与 pieces 授权都不覆盖这一枚
//! 新键，缺它则除通配超管外的收货岗访问该端点一律 403。新装库的同口径授予在
//! `init_service_ops/permission.rs` 的 warehouse_keeper 与 warehouse_manager 分组，
//! 资源名登记在 `init_service.rs::PERMISSION_RESOURCES`，两条通道逐字符同源。
//! 只授 create：该端点只有 POST 一面，read/update/delete 是永不命中的死码授权，不予补授。
//!
//! ## 幂等与安全
//! `NOT EXISTS` 守卫使重跑等价；命中 0 个角色时以 NOTICE 报出而不静默通过
//! （新装库里「迁移早于系统初始化建角色」是合法时序）。down 只按本迁移的角色码回收
//! `scan-receipt/create` 行，不动人工另行授予的其它权限。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// 受授角色码（仓储收货两岗，与 init 矩阵分组同码）
const ROLE_CODES: &[&str] = &["warehouse_keeper", "warehouse_manager"];

/// 授予的 (resource_type, action) 配对
const GRANTS: &[(&str, &str)] = &[("scan-receipt", "create")];

/// 角色码写成 SQL 数组字面量（全部为本文件编译期常量，无外部输入拼接）
fn sql_array(codes: &[&str]) -> String {
    let items: Vec<String> = codes.iter().map(|c| format!("'{c}'")).collect();
    format!("ARRAY[{}]::text[]", items.join(", "))
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let arr = sql_array(ROLE_CODES);
        let inserts: String = GRANTS
            .iter()
            .map(|(res, act)| {
                format!(
                    r#"INSERT INTO "role_permissions" ("role_id", "resource_type", "action", "allowed", "created_at", "updated_at")
SELECT r."id", '{res}', '{act}', true, NOW(), NOW()
  FROM "roles" r
 WHERE r."code" = ANY({arr})
   AND NOT EXISTS (
        SELECT 1 FROM "role_permissions" x
         WHERE x."role_id" = r."id" AND x."resource_type" = '{res}' AND x."action" = '{act}'
   );"#,
                    arr = arr,
                )
            })
            .collect::<Vec<String>>()
            .join("\n\n");
        let sql = format!(
            r#"
-- 1) 非致命可见化：统计命中与缺失角色码
DO $$
DECLARE
    hit INTEGER;
    miss TEXT;
BEGIN
    SELECT COUNT(*) INTO hit FROM "roles" WHERE "code" = ANY({arr});
    SELECT string_agg(t, ', ' ORDER BY t)
      INTO miss
      FROM (
        SELECT a AS t FROM unnest({arr}) a
         WHERE NOT EXISTS (SELECT 1 FROM "roles" r WHERE r."code" = a)
      ) m;
    IF hit = 0 THEN
        RAISE NOTICE 'm0121：scan-receipt:create 的目标受授角色在当前库中一个都不存在，本次未授予任何权限。若是「迁移早于系统初始化建角色」的新装库属正常时序（init 角色矩阵按同口径授予），否则请核对 roles.code 是否被改名。';
    ELSE
        RAISE NOTICE 'm0121：scan-receipt:create 授予命中 % 个角色；当前库不存在的角色码：%（自动跳过）', hit, COALESCE(miss, '无');
    END IF;
END
$$;

-- 2) 幂等授予（NOT EXISTS 保证重跑等价）
{inserts}
"#,
            arr = arr,
            inserts = inserts,
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let arr = sql_array(ROLE_CODES);
        let deletes: String = GRANTS
            .iter()
            .map(|(res, act)| {
                format!(
                    r#"DELETE FROM "role_permissions" rp
 USING "roles" r
 WHERE r."id" = rp."role_id"
   AND rp."resource_type" = '{res}'
   AND rp."action" = '{act}'
   AND r."code" = ANY({arr});"#,
                    arr = arr,
                )
            })
            .collect::<Vec<String>>()
            .join("\n\n");
        let sql = format!(
            r#"
-- 只回收本迁移按角色码授予的 scan-receipt/create 行；人工另授的其它键不动。
{deletes}
"#,
            deletes = deletes,
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }
}
