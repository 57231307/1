//! `product-categories:read` 存量库补授（角色矩阵 purchaser 真缺口）
//!
//! 背景事实：
//! - `/api/v1/erp/product-categories*` 由 `routes/mod.rs:368-394` 的别名段注册，资源段取自
//!   路径段 ⇒ 运行时权限键是 `product-categories:read`；
//! - 该键从未出现在 `init_service_ops/permission.rs` 角色矩阵，`PERMISSION_RESOURCES`
//!   也只注册了 `categories`（另一条资源码），历史迁移亦无授予；
//! - `matches_permission`（`middleware/permission.rs:621-640`）按资源段**精确**匹配，
//!   `categories:*` 不覆盖 `product-categories` ⇒ 除 admin（`*:*`）外任何采购岗访问
//!   产品分类树恒 403。采购员维护「我方 SKU↔供应商 SKU 对照表」时 /product 页挂载即拉
//!   分类树，功能对目标岗位实际不可用（e2e 主体用 admin 登录，CI 测不到）。
//!
//! 授权口径：产品分类树是**我方内部主数据**，非供应商保密面（保密面隔离的对象是
//! 供应商侧编码/名称，方向不同），采购岗应见 ⇒ 补真实种子，**不采用**"前端按权限降级隐藏"。
//!
//! 三条授予通道必须同口径（本迁移是其中「存量库」通道，同 m0069 范式）：
//! ① 新装库：`init_service_ops/permission.rs` purchase 域分组（purchase_manager /
//!    purchase_clerk / sourcing_specialist 各一条 `("product-categories", "read")`）；
//! ② 存量库：本迁移（按 roles.code 定位，NOT EXISTS 幂等，可重复执行）；
//! ③ e2e：`frontend/e2e/global-setup.ts` SEED_ROLE_EXTRA_PERMISSIONS 的 purchaser 条目。
//! 通道一致性由 `backend/tests/contract_wave7_product_category_grant_test.rs` 双向钉死。
//!
//! 最小授权：只授 `read`。分类树的写面（create/update/delete）仍归 admin/系统配置岗，
//! 本迁移不扩权；`("product-categories", "*")` 为禁项（由锁测把关）。
//!
//! 幂等实现细节同 m0069：用 `WHERE NOT EXISTS` 而非 `ON CONFLICT`——后者依赖
//! backend/migration/src/domain/v15/ 域内才创建的
//! `uq_role_permissions_role_resource_action` 唯一索引，为一条授予提前动全局
//! 约束不值得；迁移串行执行，NOT EXISTS 足以保证重跑等价。
//!
//! 域内注册位置：`roles`（m0001）与 `role_permissions`（m0005）均属 system 域、早于
//! business 域执行，故注册在 business 域 up 链尾 / down 链首。
//!
//! 0 命中不判失败（合法时序：迁移早于 init 建角色的新装库、以及 CI 分片库角色由
//! global-setup 补建），但必须 RAISE NOTICE 可见化，禁止"跑完一条没授"静默通过。
//!
//! down 真实可逆：只删除本迁移按角色码授予的 `product-categories:read` 行，不触碰
//! 人工另授的其它角色/动作，不留空实现。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// 需要读产品分类树的角色码：后端 init 矩阵的三个采购岗码 + e2e 侧别名码 purchaser
/// （库中不存在的码自然跳过，见 up 的 NOTICE）
const READ_ROLE_CODES: &[&str] = &[
    "purchase_manager",
    "purchase_clerk",
    "sourcing_specialist",
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
        let read_arr = sql_array(READ_ROLE_CODES);
        let sql = format!(
            r#"
-- 1) 非致命可见化：统计命中/缺失角色码
DO $$
DECLARE
    hit INTEGER;
    miss TEXT;
BEGIN
    SELECT COUNT(*) INTO hit FROM "roles" WHERE "code" = ANY({read_arr});
    SELECT string_agg(t, ', ' ORDER BY t)
      INTO miss
      FROM (
        SELECT a AS t FROM unnest({read_arr}) a
         WHERE NOT EXISTS (SELECT 1 FROM "roles" r WHERE r."code" = a)
      ) m;
    IF hit = 0 THEN
        RAISE NOTICE 'm0072：product-categories:read 的目标受授角色在当前库中一个都不存在，本次未授予任何权限。若是「迁移早于系统初始化建角色」的新装库属正常时序（init 角色矩阵按同口径授予），否则请核对 roles.code 是否被改名。';
    ELSE
        RAISE NOTICE 'm0072：product-categories:read 授予命中 % 个角色；当前库不存在的角色码：%（自动跳过）', hit, COALESCE(miss, '无');
    END IF;
END
$$;

-- 2) product-categories:read（产品分类树只读；NOT EXISTS 保证重跑等价）
INSERT INTO "role_permissions" ("role_id", "resource_type", "action", "allowed", "created_at", "updated_at")
SELECT r."id", 'product-categories', 'read', true, NOW(), NOW()
  FROM "roles" r
 WHERE r."code" = ANY({read_arr})
   AND NOT EXISTS (
        SELECT 1 FROM "role_permissions" x
         WHERE x."role_id" = r."id" AND x."resource_type" = 'product-categories' AND x."action" = 'read'
   );
"#,
            read_arr = read_arr,
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let read_arr = sql_array(READ_ROLE_CODES);
        let sql = format!(
            r#"
-- 只回收本迁移按角色码授予的 product-categories:read；人工另授不动。
DELETE FROM "role_permissions" rp
 USING "roles" r
 WHERE r."id" = rp."role_id"
   AND rp."resource_type" = 'product-categories'
   AND rp."action" = 'read'
   AND r."code" = ANY({read_arr});
"#,
            read_arr = read_arr,
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }
}
