//! `export-inspections:*` 存量库补授（出口商检结论登记功能接线，权限资源段自始未注册）
//!
//! 缺陷事实：`export-inspections` 资源段已在直接资源白名单（`utils/path_utils.rs` 的
//! `is_core_direct_resource`），但运行时权限键 `export-inspections:{read,create,update,print}`
//! 从未进 `init_service::PERMISSION_RESOURCES`，也没有任何角色被授予它；
//! `matches_permission` 按资源段精确匹配，只有 admin 的 `*:*` 能覆盖 ⇒ 除 admin 外全部角色
//! 访问出口商检端点一律 403，本域建单/登记结果/打印端点对报关/物流/销售实际不可用，
//! 而 CI e2e 用 admin 登录测不到（假绿家族）。
//!
//! 三条补授通道必须同口径（本迁移是其中「存量库」通道）：
//! ① 新装库角色矩阵 `init_service_ops/permission.rs`；
//! ② 存量库：本迁移（按 roles.code 定位，NOT EXISTS 幂等，可重复执行）；
//! ③ e2e：`global-setup.ts` 的 SEED_ROLE_EXTRA_PERMISSIONS。
//! 通道 ①② 的清单一致性由 `backend/tests/contract_wave11_export_inspection_permission_test.rs`
//! 逐动作双向钉死，防止一边改一边忘。
//!
//! 最小授权口径（按实际存在的端点逐角色授予，绝不给 `export-inspections:*`）：
//! - read   = 看列表/详情/证书列表：customs_specialist / logistics_coordinator
//!            / sales_rep / salesperson（CI/部署库以别名码 salesperson 建销售角色）
//! - create = 建单（结论落待检）：仅 customs_specialist
//! - update = 登记/改判商检结论：仅 customs_specialist
//! - print  = 打印商检单与报关单据：customs_specialist / logistics_coordinator
//! 不存在的角色码自然跳过（见非致命 NOTICE）。
//!
//! 幂等实现细节：用 `WHERE NOT EXISTS` 判定而不是 `ON CONFLICT (role_id, resource_type,
//! action)`。后者依赖 `migration/src/domain/v15/` 域内才创建的唯一索引，为本迁移去提前动全局约束、并可能因存量
//! 重复行让迁移链中断，属不必要的连带风险；迁移串行执行，NOT EXISTS 足以保证重跑等价。
//!
//! 域内注册位置：`role_permissions`（system 域 m0005）与 `roles`（m0001）在本域执行前均已
//! 存在，且本迁移不触碰 export_inspection 业务表（建单/登记写入口在 `migration/src/domain/v15/`
//! 建表之后由应用层负责，迁移只做授权），故直接注册在 production 域 up 链尾 / down 链首，
//! 无需后置到 `migration/src/domain/v15/`。
//!
//! 可逆：down 精确删除本迁移授予的 (role.code × export-inspections × read/create/update/print)
//! 行，不触碰任何人工授予的其它角色/动作，不留空实现。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// 需要读出口商检（`export-inspections:read`）的角色码
const READ_ROLE_CODES: &[&str] = &[
    "customs_specialist",
    "logistics_coordinator",
    "sales_rep",
    "salesperson",
];

/// 需要建单（`export-inspections:create`）的角色码
const CREATE_ROLE_CODES: &[&str] = &["customs_specialist"];

/// 需要登记/改判结论（`export-inspections:update`）的角色码
const UPDATE_ROLE_CODES: &[&str] = &["customs_specialist"];

/// 需要打印商检单/报关单据（`export-inspections:print`）的角色码
const PRINT_ROLE_CODES: &[&str] = &["customs_specialist", "logistics_coordinator"];

/// 角色码写成 SQL 数组字面量（全部为本文件编译期常量，无外部输入拼接）
fn sql_array(codes: &[&str]) -> String {
    let items: Vec<String> = codes.iter().map(|c| format!("'{c}'")).collect();
    format!("ARRAY[{}]::text[]", items.join(", "))
}

/// 四个动作 × 各自受授角色码：up 授予与 down 回收共用同一份清单，杜绝两套漂移。
const ACTIONS: [(&str, &[&str]); 4] = [
    ("read", READ_ROLE_CODES),
    ("create", CREATE_ROLE_CODES),
    ("update", UPDATE_ROLE_CODES),
    ("print", PRINT_ROLE_CODES),
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 命中统计：所有动作涉及的角色码全集（去重）。
        let all_codes: Vec<&str> = {
            let mut acc: Vec<&str> = Vec::new();
            for (_, codes) in ACTIONS {
                for code in codes {
                    if !acc.contains(code) {
                        acc.push(code);
                    }
                }
            }
            acc
        };
        let all_arr = sql_array(&all_codes);

        // 1) 非致命可见化：统计命中/缺失角色码。0 命中在「迁移先于 init 建角色」的新装库里
        //    是合法时序，故不 RAISE EXCEPTION，但必须打 NOTICE，禁止"跑完一条没授"静默通过。
        let notice = format!(
            r#"
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
        RAISE NOTICE 'm0083：export-inspections 权限的目标受授角色在当前库中一个都不存在，本次未授予任何权限。若是「迁移早于系统初始化建角色」的新装库属正常时序（init 角色矩阵按同口径授予），否则请核对 roles.code 是否被改名。';
    ELSE
        RAISE NOTICE 'm0083：export-inspections 权限授予命中 % 个角色；当前库不存在的角色码：%（自动跳过）', hit, COALESCE(miss, '无');
    END IF;
END
$$;
"#,
            all_arr = all_arr,
        );
        manager.get_connection().execute_unprepared(&notice).await?;

        // 2) 逐动作授予，NOT EXISTS 幂等（重跑等价）。
        for (action, codes) in ACTIONS {
            let arr = sql_array(codes);
            let sql = format!(
                r#"
INSERT INTO "role_permissions" ("role_id", "resource_type", "action", "allowed", "created_at", "updated_at")
SELECT r."id", 'export-inspections', '{action}', true, NOW(), NOW()
  FROM "roles" r
 WHERE r."code" = ANY({arr})
   AND NOT EXISTS (
        SELECT 1 FROM "role_permissions" x
         WHERE x."role_id" = r."id" AND x."resource_type" = 'export-inspections' AND x."action" = '{action}'
   );
"#,
                action = action,
                arr = arr,
            );
            manager.get_connection().execute_unprepared(&sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 只回收本迁移授予的四组键（按角色码精确圈定）；人工另授的 export-inspections 键不动。
        for (action, codes) in ACTIONS {
            let arr = sql_array(codes);
            let sql = format!(
                r#"
DELETE FROM "role_permissions" rp
 USING "roles" r
 WHERE r."id" = rp."role_id"
   AND rp."resource_type" = 'export-inspections'
   AND rp."action" = '{action}'
   AND r."code" = ANY({arr});
"#,
                action = action,
                arr = arr,
            );
            manager.get_connection().execute_unprepared(&sql).await?;
        }
        Ok(())
    }
}
