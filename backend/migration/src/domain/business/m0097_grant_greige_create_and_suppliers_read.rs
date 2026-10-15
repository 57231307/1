//! 坯布建档与供应商下拉三岗补授（production_manager / warehouse_keeper /
//! warehouse_manager 存量库通道）
//!
//! 背景事实：
//! - `/api/v1/erp/production/greige-fabrics` 路由在生产子路由内以相对路径挂载、于 erp 根域
//!   nest 一次；资源段取自路径段（消歧后的顶层 `greige-fabrics`）；运行时权限键为
//!   `greige-fabrics:read` 与 `greige-fabrics:create`；
//! - `/api/v1/erp/purchase/suppliers` 的派生键为 `suppliers:read`（建档表单的供应商
//!   下拉走 GET）；
//! - 上述键在 `init_service_ops/permission.rs` 已对三岗完成矩阵授予，但存量库不会
//!   重跑 init，缺迁移通道将致三岗访问坯布建档/供应商下拉恒 403。
//!
//! 三条授予通道必须同口径（本迁移是「存量库」通道，同 m0072/m0069 范式）：
//! - 通道 1 新装库：`init_service_ops/permission.rs` 三岗各三条授予；
//! - 通道 2 存量库：本迁移（按 roles.code 定位，NOT EXISTS 幂等，可重复执行）；
//! - 通道 3 e2e：`frontend/e2e/global-setup.ts` SEED_ROLE_EXTRA_PERMISSIONS
//!   production_manager / warehouse_keeper / warehouse_manager 三条目。
//! 通道一致性由 `backend/tests/contract_wave7_greige_suppliers_grant_test.rs`
//! 双向钉死。
//!
//! 最小授权：只授 read、create 两枚动作（坯布建档）与 read 一枚（供应商下拉只读），
//! 不授 update、delete 或通配，写面仍归 admin 或系统配置岗。
//!
//! 幂等实现细节同 m0072/m0069：用 `WHERE NOT EXISTS` 而非 `ON CONFLICT`，后者依赖
//! `domain/v15/` 内才创建的
//! `uq_role_permissions_role_resource_action` 唯一索引，为一条授予提前动全局约束
//! 不值得；迁移串行执行，NOT EXISTS 足以保证重跑等价。
//!
//! 域内注册位置：`roles`（m0001）与 `role_permissions`（m0005）均属 system 域，
//! 早于 business 域执行，注册在 business 域 up 链尾、down 链首。
//!
//! 0 命中不判失败（合法时序：迁移早于 init 建角色的新装库、以及 CI 分片库角色由
//! global-setup 补建），但必须 RAISE NOTICE 可见化，禁止静默通过。
//!
//! down 真实可逆：只删除本迁移按角色码授予的三枚键行，不触碰人工另授的其它
//! 角色或动作，不留空实现。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// 受授角色码（三岗矩阵码，均为 init 播种真角色，无部署/e2e 别名码）
const ROLE_CODES: &[&str] = &[
    "production_manager",
    "warehouse_keeper",
    "warehouse_manager",
];

/// 授予的 (resource_type, action) 配对
const GRANTS: &[(&str, &str)] = &[
    ("greige-fabrics", "read"),
    ("greige-fabrics", "create"),
    ("suppliers", "read"),
];

/// 角色码写成 SQL 数组字面量
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
        RAISE NOTICE 'm0097：greige-fabrics/suppliers 补授的目标角色在当前库中一个都不存在，本次未授予任何权限。若是「迁移早于系统初始化建角色」的新装库属正常时序（init 角色矩阵按同口径授予），否则请核对 roles.code 是否被改名。';
    ELSE
        RAISE NOTICE 'm0097：greige-fabrics/suppliers 补授命中 % 个角色；当前库不存在的角色码：%（自动跳过）', hit, COALESCE(miss, '无');
    END IF;
END
$$;

-- 2) 三条幂等授予（NOT EXISTS 保证重跑等价）
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
-- 只回收本迁移按角色码授予的三枚键行；人工另授不动。
{deletes}
"#,
            deletes = deletes,
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }
}
