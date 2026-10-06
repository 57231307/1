//! `customers:reveal` 存量库补授（PII 按需揭示的独立权限键）
//!
//! 运行时权限键的推导事实（非推断）：端点
//! `/api/v1/erp/crm/customers/{id}/pii/reveal` 的 resource_type 由 URL 段推导
//! （`middleware/permission.rs::extract_resource_info`：seg3=`crm` 是模块前缀，
//! 资源名取 seg4=`customers`），action 由路径末段关键字表命中 `reveal`
//! （`PATH_ACTION_KEYWORDS`，形态与 `pieces:print` 的 `print` 完全同构）。
//! 因此运行时键是 `customers:reveal`，注册表权威资源名 `customers`
//! （`init_service.rs::PERMISSION_RESOURCES` 已在册）。
//!
//! 三条授予通道必须同口径（照抄本仓 pieces:print 批次的通道划分）：
//! ① 新装库：`init_service_ops/permission.rs` 角色矩阵显式授权行；
//! ② 存量库：本迁移（按 roles.code 定位，NOT EXISTS 幂等，可重复执行）；
//! ③ e2e：`frontend/e2e/global-setup.ts` 的 SEED_ROLE_EXTRA_PERMISSIONS。
//! 通道 ①↔② 的集合相等与 ③ 的子集关系由
//! `backend/tests/contract_wave11_pii_reveal_test.rs` 逐 token 双向钉死。
//!
//! 最小授权口径（保守可回退默认，待用户终裁）：只授"管客户、打电话"的岗——
//! - `crm_rep` / `sales_rep`：客户/销售的直接联系人岗；
//! - 在册部署别名码 `salesperson`（CI/部署库由 e2e SEED_ROLES 建它而不建
//!   sales_rep，与 pieces:read 面同一在册别名口径）。
//! 管理岗（crm_manager/sales_manager/manager）不新增显式行：它们已持有
//! `("customers","*")`，wildcard 对动作的覆盖是既有语义，双写即矩阵噪音；
//! 是否收紧 wildcard 面属独立的授权面决策，不在本迁移动。
//!
//! 幂等实现细节：用 `WHERE NOT EXISTS` 判定而不是 `ON CONFLICT (role_id,
//! resource_type, action)`——后者依赖 `migration/src/domain/v15/` 域内才创建的唯一索引，为一条授予提前
//! 动全局约束属不必要的连带风险（与 m0069 同一决策，迁移串行执行，NOT EXISTS
//! 足以保证重跑等价）。
//!
//! 为什么不照 color_card_issue 先例加 `is_system = true` 过滤：CI 实测该库的
//! 角色由 e2e 补建（is_system 可能为 false），加过滤会静默授不上；code 才是
//! 授权契约，is_system 只标数据来源。
//!
//! 域内注册位置：`role_permissions`（m0005，system 域）与 `roles`（m0001）在
//! 本域执行前均已存在，故直接注册在 business 域 up 链尾/down 链首。
//!
//! 幂等与回滚：up = 只读统计 NOTICE → 一条 INSERT..SELECT..WHERE NOT EXISTS，
//! 重跑等价；down 精确删除本迁移授予的 (role.code × customers × reveal) 行，
//! 不触碰任何人工授予的其它角色/动作，不留空实现。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// 应被授予 `customers:reveal` 的角色码（矩阵码 + 部署/e2e 在册别名码）
const REVEAL_ROLE_CODES: &[&str] = &["crm_rep", "sales_rep", "salesperson"];

/// 角色码写成 SQL 数组字面量（全部为本文件编译期常量，无外部输入拼接）
fn sql_array(codes: &[&str]) -> String {
    let items: Vec<String> = codes.iter().map(|c| format!("'{c}'")).collect();
    format!("ARRAY[{}]::text[]", items.join(", "))
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let all_arr = sql_array(REVEAL_ROLE_CODES);
        let sql = format!(
            r#"
-- 非致命可见化：统计命中/缺失角色码。0 命中在「迁移先于 init 建角色」的新装库
-- 里是合法时序（init 矩阵同口径授予），故不 RAISE EXCEPTION，但必须打 NOTICE，
-- 禁止"迁移跑完一条没授"这种静默通过。
DO $$
DECLARE
    hit INTEGER;
    miss TEXT;
BEGIN
    SELECT COUNT(*) INTO hit FROM "roles" WHERE "code" = ANY({all_arr});
    SELECT string_agg(t, ', ' ORDER BY t)
      INTO miss
      FROM (
        SELECT a AS t FROM unnest({all_arr}) a
         WHERE NOT EXISTS (SELECT 1 FROM "roles" r WHERE r."code" = a)
      ) m;
    IF hit = 0 THEN
        RAISE NOTICE 'm0086：customers:reveal 的目标受授角色在当前库中一个都不存在，本次未授予任何权限。若是「迁移早于系统初始化建角色」的新装库属正常时序（init 角色矩阵按同口径授予），否则请核对 roles.code 是否被改名。';
    ELSE
        RAISE NOTICE 'm0086：customers:reveal 授予命中 % 个角色；当前库不存在的角色码：%（自动跳过）', hit, COALESCE(miss, '无');
    END IF;
END
$$;

-- customers:reveal（PII 按需揭示端点 POST /crm/customers/{{id}}/pii/reveal 的动作门）。
-- NOT EXISTS 保证重跑等价（见文件头「幂等实现细节」）
INSERT INTO "role_permissions" ("role_id", "resource_type", "action", "allowed", "created_at", "updated_at")
SELECT r."id", 'customers', 'reveal', true, NOW(), NOW()
  FROM "roles" r
 WHERE r."code" = ANY({all_arr})
   AND NOT EXISTS (
        SELECT 1 FROM "role_permissions" x
         WHERE x."role_id" = r."id" AND x."resource_type" = 'customers' AND x."action" = 'reveal'
   );
"#,
            all_arr = all_arr,
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let all_arr = sql_array(REVEAL_ROLE_CODES);
        let sql = format!(
            r#"
-- 只回收本迁移授予的键（按角色码精确圈定）；人工另授的 customers 键不动。
DELETE FROM "role_permissions" rp
 USING "roles" r
 WHERE r."id" = rp."role_id"
   AND rp."resource_type" = 'customers'
   AND rp."action" = 'reveal'
   AND r."code" = ANY({all_arr});
"#,
            all_arr = all_arr,
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }
}
