//! 生产经理补 `boms:read`：物料清单页与工单用料判据对本岗不再恒 403
//!
//! ## 六要素
//! - **功能**：给 `role_permissions` 补 `(生产经理角色 × boms × read)` 行。
//! - **调用方**：SeaORM 迁移链，注册在 `domain/business/mod.rs` 的 up 链末尾、down 链首位
//!   （`roles`/`role_permissions` 由 system 域先建，早于本域）。
//! - **入参**：无。角色码是本文件的编译期常量。
//! - **传给谁**：`SchemaManager` 的裸 SQL 连接；`GET /boms` 与 `/boms/{id}` 的判权键由
//!   `backend/src/middleware/permission.rs` 派生为 `boms:read`，前端路由门在
//!   `frontend/src/router/index.ts` 的 `/bom` meta 上声明同一枚键。
//! - **存什么**：只写 `role_permissions` 的 (role_id × 'boms' × 'read') 行。
//! - **存哪里**：PostgreSQL `public.role_permissions`。
//!
//! ## 为什么是 read 而不是 *
//! 物料清单是本岗的**用料依据**（工单按 BOM 下达、异常时要去 BOM 页核对结构），
//! 但 BOM 的建立/改版/审批属产品与工艺侧职责，生产岗不应自带改单能力；
//! 故只授读，写侧键一律不授（同 `products:read` 的分配口径）。
//!
//! ## 幂等与安全
//! `NOT EXISTS` 守卫使重跑等价；命中 0 个角色时以 NOTICE 报出而不静默通过
//! （新装库里"迁移早于系统初始化建角色"是合法时序）。down 只按本迁移的角色码回收
//! `boms/read` 行，不动人工另行授予的其它 boms 权限。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// 受授角色码（矩阵码 + 部署/e2e 在册别名码）
const ROLE_CODES: &[&str] = &["production_manager"];

/// 角色码写成 SQL 数组字面量（全部为本文件编译期常量，无外部输入拼接）
fn sql_array(codes: &[&str]) -> String {
    let items: Vec<String> = codes.iter().map(|c| format!("'{c}'")).collect();
    format!("ARRAY[{}]::text[]", items.join(", "))
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let arr = sql_array(ROLE_CODES);
        let sql = format!(
            r#"
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
      ) t;
    IF hit = 0 THEN
        RAISE NOTICE '本迁移：boms:read 的目标受授角色在当前库中一个都不存在，本次未授予任何权限。若是「迁移早于系统初始化建角色」的新装库属正常时序（init 角色矩阵按同口径授予），否则请核对 roles.code 是否被改名。';
    ELSE
        RAISE NOTICE '本迁移：boms:read 授予命中 % 个角色；当前库不存在的角色码：%（自动跳过）', hit, COALESCE(miss, '无');
    END IF;
END
$$;

INSERT INTO "role_permissions" ("role_id", "resource_type", "action", "allowed", "created_at", "updated_at")
SELECT r."id", 'boms', 'read', true, NOW(), NOW()
  FROM "roles" r
 WHERE r."code" = ANY({arr})
   AND NOT EXISTS (
        SELECT 1 FROM "role_permissions" x
         WHERE x."role_id" = r."id" AND x."resource_type" = 'boms' AND x."action" = 'read'
   );
"#,
            arr = arr,
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = format!(
            r#"
DELETE FROM "role_permissions" rp
 USING "roles" r
 WHERE r."id" = rp."role_id"
   AND rp."resource_type" = 'boms'
   AND rp."action" = 'read'
   AND r."code" = ANY({arr});
"#,
            arr = sql_array(ROLE_CODES),
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }
}
