//! 采购收货「让步接收 / 复检改判」两枚显式权限键的存量库补授
//!
//! 功能：给 `role_permissions` 补 `(角色 × purchase-receipts × concession)` 与
//! `(角色 × purchase-receipts × rejudge)` 两类行，使这两个处置动作各自受独立权限键约束。
//!
//! 调用方：迁移链，注册在 `migration/src/domain/business/mod.rs` 的 up 链末尾、
//! down 链首位（`roles`/`role_permissions` 由 system 域建立，早于本域）。
//! 模块声明与 up/down 调用位均在同文件内注明。
//!
//! 入参：无。传给谁：`SchemaManager` 的裸 SQL 连接。
//! 存什么/存哪里：只写 `role_permissions`（role_id × resource_type='purchase-receipts'
//! × action ∈ {concession, rejudge} × allowed=true），不改任何业务表数据。
//!
//! 为什么要有这两枚键：让步接收与复检改判的端点路径末段原先不在
//! `backend/src/middleware/permission.rs` 的动作关键字表内，权限中间件按方法回落成
//! `create`，于是"能建收货单"就等于"能让步 + 能改判"。二者业务含义不同——让步是
//! 对不合格品的放行决定，改判是对既有质检结论的推翻——必须各自可授可回收。
//!
//! 岗位分配（刻意不同集合，构成职责分离）：
//! - `concession` 授采购经理与质量经理：放行决定由采购与质量共签，
//!   不授建单收货的采购员与仓管（ISO 9001:2015 §8.7 要求让步须由授权人批准，
//!   §8.7.2 要求记录"作出决定的授权方"；准收者不应即建单收货者）。
//! - `rejudge` 授质检与质量经理：改判由质量条线独立执行，不授任何采购岗。
//!
//! 角色码写法：矩阵码 + 部署/e2e 在册别名码一并列入（`qc_manager` 的在册别名为
//! `quality_manager`），与 customers:reveal 补授同范式；库中不存在的码由 NOT EXISTS
//! 自然跳过并以 NOTICE 报出，绝不静默通过。
//!
//! 幂等：`NOT EXISTS` 守卫使重跑等价；down 只按本迁移授予的
//! (角色码 × purchase-receipts × {concession, rejudge}) 精确删除，
//! 人工另行授予的其它 purchase-receipts 权限不动。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// 让步接收键的受授角色码（矩阵码 + 在册别名码）
const CONCESSION_ROLE_CODES: &[&str] = &["purchase_manager", "qc_manager", "quality_manager"];

/// 复检改判键的受授角色码（矩阵码 + 在册别名码）
const REJUDGE_ROLE_CODES: &[&str] = &["quality_inspector", "qc_manager", "quality_manager"];

/// 角色码写成 SQL 数组字面量（全部为本文件编译期常量，无外部输入拼接）
fn sql_array(codes: &[&str]) -> String {
    let items: Vec<String> = codes.iter().map(|c| format!("'{c}'")).collect();
    format!("ARRAY[{}]::text[]", items.join(", "))
}

/// 统计并播报授予命中/缺失的角色码：0 命中在"迁移早于系统初始化建角色"的新装库里
/// 是合法时序，故不 RAISE EXCEPTION，但必须打 NOTICE，禁止"跑完一条没授"静默通过。
fn probe_sql(arr: &str, action: &str) -> String {
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
        RAISE NOTICE '本迁移：{action} 的目标受授角色在当前库中一个都不存在，本次未授予任何权限。若是「迁移早于系统初始化建角色」的新装库属正常时序（init 角色矩阵按同口径授予），否则请核对 roles.code 是否被改名。';
    ELSE
        RAISE NOTICE '本迁移：{action} 授予命中 % 个角色；当前库不存在的角色码：%（自动跳过）', hit, COALESCE(miss, '无');
    END IF;
END
$$;
"#,
        arr = arr,
        action = action,
    )
}

/// 按 (resource, action) 补授，NOT EXISTS 保证重跑等价
fn grant_sql(arr: &str, action: &str) -> String {
    format!(
        r#"
INSERT INTO "role_permissions" ("role_id", "resource_type", "action", "allowed", "created_at", "updated_at")
SELECT r."id", 'purchase-receipts', '{action}', true, NOW(), NOW()
  FROM "roles" r
 WHERE r."code" = ANY({arr})
   AND NOT EXISTS (
        SELECT 1 FROM "role_permissions" x
         WHERE x."role_id" = r."id" AND x."resource_type" = 'purchase-receipts' AND x."action" = '{action}'
   );
"#,
        arr = arr,
        action = action,
    )
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let concession_arr = sql_array(CONCESSION_ROLE_CODES);
        let rejudge_arr = sql_array(REJUDGE_ROLE_CODES);
        let sql = format!(
            "{}{}{}{}",
            probe_sql(&concession_arr, "purchase-receipts:concession"),
            probe_sql(&rejudge_arr, "purchase-receipts:rejudge"),
            grant_sql(&concession_arr, "concession"),
            grant_sql(&rejudge_arr, "rejudge"),
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = format!(
            r#"
-- 只回收本迁移授予的两枚键（按角色码精确圈定）；人工另授的其它权限不动。
DELETE FROM "role_permissions" rp
 USING "roles" r
 WHERE r."id" = rp."role_id"
   AND rp."resource_type" = 'purchase-receipts'
   AND rp."action" = 'concession'
   AND r."code" = ANY({concession_arr});

DELETE FROM "role_permissions" rp
 USING "roles" r
 WHERE r."id" = rp."role_id"
   AND rp."resource_type" = 'purchase-receipts'
   AND rp."action" = 'rejudge'
   AND r."code" = ANY({rejudge_arr});
"#,
            concession_arr = sql_array(CONCESSION_ROLE_CODES),
            rejudge_arr = sql_array(REJUDGE_ROLE_CODES),
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }
}
