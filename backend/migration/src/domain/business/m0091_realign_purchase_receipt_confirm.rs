//! 采购收货确认键正名：回收悬空的 `purchase-receipts:approve`，按真实键 `confirm` 补授
//!
//! ## 六要素
//! - **功能**：删除所有角色在 `role_permissions` 里的 `(purchase-receipts, approve)` 行，
//!   并给采购经理与采购员补 `(purchase-receipts, confirm)` 行。
//! - **调用方**：SeaORM 迁移链，注册在 `domain/business/mod.rs` 的 up 链末尾、down 链首位
//!   （`roles`/`role_permissions` 由 system 域先建，早于本域）。
//! - **入参**：无。角色码是本文件的编译期常量，不接任何外部输入。
//! - **传给谁**：`SchemaManager` 的裸 SQL 连接；运行期判权由
//!   `backend/src/middleware/permission.rs` 用路径末段派生出键后来查这张表。
//! - **存什么**：只写/删 `role_permissions` 的 (role_id × resource_type × action) 行。
//! - **存哪里**：PostgreSQL `public.role_permissions`。
//!
//! ## 为什么 approve 是悬空授权
//! 采购收货没有 approve 端点：确认收货走 `POST /receipts/{id}/confirm`，而 `confirm` 与
//! `approve` 是动作关键字表里的两个不同词条 ⇒ 运行期派生的键是 `purchase-receipts:confirm`。
//! 矩阵原先只授 `approve`，结果就是"能读能建收货单，但点确认恒 403"，而 `approve` 那行
//! 永远不会被任何请求命中。本迁移把授权改到真实键上，并删掉不会被命中的那行。
//!
//! ## 岗位分配
//! `confirm` 授采购经理与采购员（矩阵码 + 部署/e2e 在册别名码）：建单与确认收货同线；
//! 让步接收与复检改判仍按同域 `m0090` 的分配走，本迁移不触碰，避免"自判自放"。
//!
//! ## 幂等与可回滚
//! 补授用 `NOT EXISTS` 守卫，重跑等价；删除用精确 (resource, action) 圈定，不动同资源其它键。
//! down 先回收本迁移授予的 `confirm` 行，再按迁移前矩阵在册的采购经理码恢复 `approve` 行
//! （恢复的是本迁移删掉的原有状态，不是新造授权）。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// 收货确认键的受授角色码（矩阵码 + 在册别名码）
const CONFIRM_ROLE_CODES: &[&str] = &[
    "purchase_manager",
    "purchasing_manager",
    "purchase_clerk",
    "purchaser",
];

/// 迁移前矩阵里持有悬空 approve 行的角色码（仅采购经理侧，down 按此恢复原态）
const LEGACY_APPROVE_ROLE_CODES: &[&str] = &["purchase_manager", "purchasing_manager"];

/// 角色码写成 SQL 数组字面量（全部为本文件编译期常量，无外部输入拼接）
fn sql_array(codes: &[&str]) -> String {
    let items: Vec<String> = codes.iter().map(|c| format!("'{c}'")).collect();
    format!("ARRAY[{}]::text[]", items.join(", "))
}

/// 播报本次删掉的悬空行数：删 0 行在"库从未授过 approve"的新装库里是合法时序，
/// 故只 RAISE NOTICE 不 RAISE EXCEPTION，但禁止静默通过。
fn probe_revoke_sql() -> String {
    r#"
DO $$
DECLARE
    hit INTEGER;
BEGIN
    SELECT COUNT(*) INTO hit
      FROM "role_permissions" rp
      JOIN "roles" r ON r."id" = rp."role_id"
     WHERE rp."resource_type" = 'purchase-receipts' AND rp."action" = 'approve';
    IF hit = 0 THEN
        RAISE NOTICE '本迁移：当前库没有任何 purchase-receipts/approve 行需要回收（新装库或已回收过），仍会继续补授 confirm 键。';
    ELSE
        RAISE NOTICE '本迁移：回收 purchase-receipts/approve 悬空授权 % 行', hit;
    END IF;
END
$$;
"#
    .to_string()
}

/// 把角色码数组拼成"仅取在册码"的 WHERE 片段
fn in_clause(arr: &str) -> String {
    format!("r.\"code\" = ANY({arr})")
}

/// 回收 approve 行（按资源+动作精确圈定，不动同资源的 read/create/concession 等）
fn revoke_sql() -> String {
    r#"
DELETE FROM "role_permissions" rp
 USING "roles" r
 WHERE r."id" = rp."role_id"
   AND rp."resource_type" = 'purchase-receipts'
   AND rp."action" = 'approve';
"#
    .to_string()
}

/// 按角色码补授 confirm，NOT EXISTS 保证重跑等价
fn grant_sql(arr: &str, action: &str) -> String {
    format!(
        r#"
INSERT INTO "role_permissions" ("role_id", "resource_type", "action", "allowed", "created_at", "updated_at")
SELECT r."id", 'purchase-receipts', '{action}', true, NOW(), NOW()
  FROM "roles" r
 WHERE {scope}
   AND NOT EXISTS (
        SELECT 1 FROM "role_permissions" x
         WHERE x."role_id" = r."id" AND x."resource_type" = 'purchase-receipts' AND x."action" = '{action}'
   );
"#,
        action = action,
        scope = in_clause(arr),
    )
}

/// 播报 confirm 补授命中/缺失的角色码，禁止"跑完一条没授"静默通过
fn probe_grant_sql(arr: &str) -> String {
    format!(
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
        RAISE NOTICE '本迁移：purchase-receipts:confirm 的目标受授角色在当前库中一个都不存在，本次未授予任何权限。若是「迁移早于系统初始化建角色」的新装库属正常时序（init 角色矩阵按同口径授予），否则请核对 roles.code 是否被改名。';
    ELSE
        RAISE NOTICE '本迁移：purchase-receipts:confirm 授予命中 % 个角色；当前库不存在的角色码：%（自动跳过）', hit, COALESCE(miss, '无');
    END IF;
END
$$;
"#,
        arr = arr,
    )
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let confirm_arr = sql_array(CONFIRM_ROLE_CODES);
        let sql = format!(
            "{}{}{}{}",
            probe_revoke_sql(),
            revoke_sql(),
            probe_grant_sql(&confirm_arr),
            grant_sql(&confirm_arr, "confirm"),
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let confirm_arr = sql_array(CONFIRM_ROLE_CODES);
        let legacy_arr = sql_array(LEGACY_APPROVE_ROLE_CODES);
        let sql = format!(
            r#"
-- 先回收本迁移授予的 confirm 行
DELETE FROM "role_permissions" rp
 USING "roles" r
 WHERE r."id" = rp."role_id"
   AND rp."resource_type" = 'purchase-receipts'
   AND rp."action" = 'confirm'
   AND {confirm_scope};
-- 再恢复本迁移删除的原有 approve 行（只按迁移前在册的采购经理码，NOT EXISTS 防重）
INSERT INTO "role_permissions" ("role_id", "resource_type", "action", "allowed", "created_at", "updated_at")
SELECT r."id", 'purchase-receipts', 'approve', true, NOW(), NOW()
  FROM "roles" r
 WHERE {legacy_scope}
   AND NOT EXISTS (
        SELECT 1 FROM "role_permissions" x
         WHERE x."role_id" = r."id" AND x."resource_type" = 'purchase-receipts' AND x."action" = 'approve'
   );
"#,
            confirm_scope = in_clause(&confirm_arr),
            legacy_scope = in_clause(&legacy_arr),
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }
}
