//! `pieces:read` / `pieces:print` 存量库补授（复审 #4669 挂账项：匹号领域权限键从未注册）
//!
//! 缺陷事实（非推断）：
//! - `/api/v1/erp/inventory/pieces`（四维追溯选匹）与 `/api/v1/erp/inventory/pieces/{id}/print`
//!   （#220 成品布入库标签）的资源段由 URL 推导：`middleware/permission.rs::extract_resource_info`
//!   对模块前缀 `inventory` 取 segment4（`utils/path_utils.rs:102-117` 默认分支），
//!   运行时权限键是 `pieces:read` / `pieces:print`；
//! - 该资源段从未进过 `init_service::PERMISSION_RESOURCES`，也没有任何角色被授予过它
//!   （`init_service_ops/permission.rs` 全文无 "pieces"，历史迁移亦无）；
//! - `matches_permission`（`middleware/permission.rs:601-610`）按资源段**精确**匹配，
//!   只有 `*:*`（admin）或 `pieces:*` 能覆盖 ⇒ 除 admin 外全部角色访问匹号领域端点一律 403。
//!   于是本波交付的「销售发货第四维匹号选择器」与「成品布入库标签」对仓管/验布员/销售
//!   实际不可用，而 CI e2e 全用 admin 登录，测不到（e2e 测不到的功能缺陷家族）。
//!
//! 三条补授通道必须同口径（本迁移是其中「存量库」通道）：
//! ① 新装库：`init_service_ops/permission.rs` 角色矩阵（本仓 init 只在 role_permissions
//!    为空时播种，且角色须已按 code 存在）；
//! ② 存量库：本迁移（按 roles.code 定位，NOT EXISTS 幂等，可重复执行）；
//! ③ e2e：`frontend/e2e/global-setup.ts` 的 SEED_ROLE_EXTRA_PERMISSIONS（CI 库的角色由
//!    ensureRoleUsers 补建，不走 init 矩阵）。
//! 通道 ①② 的清单一致性由 `backend/tests/contract_wave7_piece_permission_grant_test.rs`
//! 逐 token 双向钉死，防止一边改一边忘。
//!
//! 最小授权口径（只授实际存在的两个端点所需，绝不给 `pieces:*`）：
//! - `pieces:read`  = 按四维筛在库匹（销售发货/调拨选匹、验布标签面板、生产跟踪）
//!   inventory_manager / warehouse_keeper / warehouse_manager / sales_rep / salesperson
//!   / quality_inspector / fabric_inspector / production_manager
//! - `pieces:print` = 打印成品布入库标签（本岗产出人）
//!   inventory_manager / warehouse_keeper / warehouse_manager / quality_inspector / fabric_inspector
//! 角色码同时覆盖后端矩阵码与 e2e/部署侧别名码（salesperson/warehouse_manager），
//! 不存在的码自然跳过（见下方非致命 NOTICE）。
//!
//! 为什么不照 color_card_issue 先例加 `is_system = true` 过滤：CI 实测该库的角色由
//! e2e 补建（is_system 可能为 false），加过滤会静默授不上；code 才是授权契约，
//! is_system 只标数据来源。
//!
//! 幂等实现细节：用 `WHERE NOT EXISTS` 判定而不是 `ON CONFLICT (role_id, resource_type,
//! action)`。后者依赖 v15 域内才创建的
//! `uq_role_permissions_role_resource_action` 唯一索引，为本迁移（一条授予）去提前
//! 动全局约束、并可能因存量重复行让迁移链中断，属不必要的连带风险；迁移串行执行，
//! NOT EXISTS 足以保证重跑等价。
//!
//! 域内注册位置：`role_permissions`（m0005，system 域）与 `roles`（m0001）在本域执行前
//! 均已存在，故直接注册在 production 域 up 链尾/down 链首，无需像 m0068 那样由 v15 延后调用。
//!
//! 幂等与回滚：up = 只读统计 NOTICE → 两条 INSERT..SELECT..WHERE NOT EXISTS，
//! 重跑等价；down 精确删除本迁移授予的 (role.code × pieces × read/print) 行，
//! 不触碰任何人工授予的其它角色/动作，不留空实现。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// 需要读匹号领域（`pieces:read`）的角色码
const READ_ROLE_CODES: &[&str] = &[
    "inventory_manager",
    "warehouse_keeper",
    "warehouse_manager",
    "sales_rep",
    "salesperson",
    "quality_inspector",
    "fabric_inspector",
    "production_manager",
];

/// 需要打印成品布入库标签（`pieces:print`）的角色码
const PRINT_ROLE_CODES: &[&str] = &[
    "inventory_manager",
    "warehouse_keeper",
    "warehouse_manager",
    "quality_inspector",
    "fabric_inspector",
];

/// 角色码写成 SQL 数组字面量（全部为本文件编译期常量，无外部输入拼接）
fn sql_array(codes: &[&str]) -> String {
    let items: Vec<String> = codes.iter().map(|c| format!("'{c}'")).collect();
    format!("ARRAY[{}]::text[]", items.join(", "))
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let read_arr = sql_array(READ_ROLE_CODES);
        let print_arr = sql_array(PRINT_ROLE_CODES);
        let all_arr = {
            let mut all: Vec<&str> = READ_ROLE_CODES.to_vec();
            for code in PRINT_ROLE_CODES {
                if !all.contains(code) {
                    all.push(code);
                }
            }
            sql_array(&all)
        };
        let sql = format!(
            r#"
-- 1) 非致命可见化：统计命中/缺失角色码。
--    0 命中在「迁移先于 init 建角色」的新装库里是合法时序，故不 RAISE EXCEPTION
--    （角色齐后重跑本迁移即补授；e2e 库由 global-setup 通道授予），但必须打 NOTICE，
--    禁止"迁移跑完一条没授"这种静默通过。
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
        RAISE NOTICE 'm0069：pieces:read/pieces:print 的目标受授角色在当前库中一个都不存在，本次未授予任何权限。若是「迁移早于系统初始化建角色」的新装库属正常时序（init 角色矩阵按同口径授予），否则请核对 roles.code 是否被改名。';
    ELSE
        RAISE NOTICE 'm0069：pieces 权限授予命中 % 个角色；当前库不存在的角色码：%（自动跳过）', hit, COALESCE(miss, '无');
    END IF;
END
$$;

-- 2) pieces:read（按四维筛在库匹）。NOT EXISTS 保证重跑等价（见文件头「幂等实现细节」）
INSERT INTO "role_permissions" ("role_id", "resource_type", "action", "allowed", "created_at", "updated_at")
SELECT r."id", 'pieces', 'read', true, NOW(), NOW()
  FROM "roles" r
 WHERE r."code" = ANY({read_arr})
   AND NOT EXISTS (
        SELECT 1 FROM "role_permissions" x
         WHERE x."role_id" = r."id" AND x."resource_type" = 'pieces' AND x."action" = 'read'
   );

-- 3) pieces:print（成品布入库标签，本岗产出人）
INSERT INTO "role_permissions" ("role_id", "resource_type", "action", "allowed", "created_at", "updated_at")
SELECT r."id", 'pieces', 'print', true, NOW(), NOW()
  FROM "roles" r
 WHERE r."code" = ANY({print_arr})
   AND NOT EXISTS (
        SELECT 1 FROM "role_permissions" x
         WHERE x."role_id" = r."id" AND x."resource_type" = 'pieces' AND x."action" = 'print'
   );
"#,
            all_arr = all_arr,
            read_arr = read_arr,
            print_arr = print_arr,
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let read_arr = sql_array(READ_ROLE_CODES);
        let print_arr = sql_array(PRINT_ROLE_CODES);
        let sql = format!(
            r#"
-- 只回收本迁移授予的两组键（按角色码精确圈定）；人工另授的 pieces 键不动。
DELETE FROM "role_permissions" rp
 USING "roles" r
 WHERE r."id" = rp."role_id"
   AND rp."resource_type" = 'pieces'
   AND (
        (rp."action" = 'read'  AND r."code" = ANY({read_arr}))
     OR (rp."action" = 'print' AND r."code" = ANY({print_arr}))
   );
"#,
            read_arr = read_arr,
            print_arr = print_arr,
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }
}
