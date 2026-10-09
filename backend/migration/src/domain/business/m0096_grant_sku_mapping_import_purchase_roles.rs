//! 采购两岗补 `sku-mappings:import`：批量导入对照表不再被门层判死
//!
//! ## 六要素
//! - **功能**：给 `role_permissions` 补 `(采购经理/采购员角色码 × sku-mappings × import)` 行。
//! - **调用方**：SeaORM 迁移链，注册在 `domain/business/mod.rs` 的 up 链末尾、down 链首位
//!   （`roles`/`role_permissions` 由 system 域先建，早于本域）。
//! - **入参**：无。角色码是本文件的编译期常量（矩阵码 + 部署/e2e 在册别名码）。
//! - **传给谁**：`SchemaManager` 的裸 SQL 连接。判权键由
//!   `backend/src/middleware/permission.rs` 从 `POST /api/v1/erp/purchase/sku-mappings/import`
//!   派生为 `sku-mappings:import`（`import` 是 `PATH_ACTION_KEYWORDS` 在册动作词，URL 末段
//!   优先于 HTTP 方法），前端门在同一枚键上（`frontend/src/constants/permissions.ts` 的
//!   `SKU_MAPPING_IMPORT`，用于对照表页的导入按钮）。
//! - **存什么**：只写 `role_permissions` 的 (role_id × 'sku-mappings' × 'import') 行。
//! - **存哪里**：PostgreSQL `public.role_permissions`。
//!
//! ## 为什么导入要有独立一枚键，而不是复用 create
//! 导入端点是成批写主数据（一次落多行、可覆盖既有对照），影响面大于逐条建单，
//! 单独一枚键才能被单独收回；把 `import` 从动作词表里摘掉让它退回 `create` 会让所有
//! 在册 `/{resource}/import` 端点一起失去区分度，属反向修法。
//!
//! ## 受授集合取值依据
//! 与本岗既有 `sku-mappings:create` 集合对齐：采购员逐条建单，批量导入是同一路径的成批
//! 形态，不给本岗就是"能建不能批"；采购经理持 read/update/delete 而不持 create，但审批
//! 链上需能补录整批对照，故与 update/delete 同授予。销售侧一律不授（对照表属采购保密域）。
//!
//! ## 幂等与安全
//! `NOT EXISTS` 守卫使重跑等价；命中 0 个角色时以 NOTICE 报出而不静默通过
//! （新装库里"迁移早于系统初始化建角色"是合法时序）。down 只按本迁移的角色码回收
//! `sku-mappings/import` 行，不动人工另行授予的其它 sku-mappings 权限。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// 受授角色码（矩阵码 + 部署/e2e 在册别名码）
const ROLE_CODES: &[&str] = &[
    "purchase_manager",
    "purchasing_manager",
    "purchase_clerk",
    "purchaser",
];

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
        RAISE NOTICE '本迁移：sku-mappings:import 的目标受授角色在当前库中一个都不存在，本次未授予任何权限。若是「迁移早于系统初始化建角色」的新装库属正常时序（init 角色矩阵按同口径授予），否则请核对 roles.code 是否被改名。';
    ELSE
        RAISE NOTICE '本迁移：sku-mappings:import 授予命中 % 个角色；当前库不存在的角色码：%（自动跳过）', hit, COALESCE(miss, '无');
    END IF;
END
$$;

INSERT INTO "role_permissions" ("role_id", "resource_type", "action", "allowed", "created_at", "updated_at")
SELECT r."id", 'sku-mappings', 'import', true, NOW(), NOW()
  FROM "roles" r
 WHERE r."code" = ANY({arr})
   AND NOT EXISTS (
        SELECT 1 FROM "role_permissions" x
         WHERE x."role_id" = r."id" AND x."resource_type" = 'sku-mappings' AND x."action" = 'import'
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
   AND rp."resource_type" = 'sku-mappings'
   AND rp."action" = 'import'
   AND r."code" = ANY({arr});
"#,
            arr = sql_array(ROLE_CODES),
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }
}
