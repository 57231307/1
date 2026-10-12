//! 合同/价格 `reject` 存量库补授（审批改造「通过+拒绝各给理由」波：reject 权限键的存量库通道）
//!
//! 六要素：
//! - 功能·调用方：init 角色矩阵只在系统初始化时落库，存量库不会重跑 init；本迁移由
//!   迁移框架在 business 域 up 链尾执行，为存量库补上矩阵应有而库里缺失的 4 组授权行。
//! - 入参：无外部入参；角色码与资源/动作全部为本文件编译期常量。
//! - 传给谁：拼好的 SQL 交给当前数据库连接（execute_unprepared），PostgreSQL 执行。
//! - 存什么：`role_permissions` 表中 (sales_manager × sales-contracts × reject)、
//!   (sales_manager × sales-prices × reject)、(purchase_manager × purchase-contracts ×
//!   reject)、(purchase_manager × purchase-prices × reject) 四组 allowed=true 授权行，
//!   并同口径覆盖 e2e/部署侧采购审批别名码 purchasing_manager。
//! - 存哪里：`role_permissions`（system 域 m0005 建表；列名 resource_type/action/allowed
//!   以该 DDL 为准。本仓无独立 permissions 资源注册表，资源键即 resource_type 取值，
//!   与 middleware/permission.rs 由 URL 段推导的运行时权限键逐字符一致）。
//!
//! 与三条通道的关系（同 m0069/m0072 范式，本迁移是「存量库」通道）：
//! ① 新装库：init_service_ops/permission.rs 矩阵内 sales_manager/purchase_manager 的
//!    reject 行（已在本波加入，仅初始化时播种）；
//! ② 存量库：本迁移；
//! ③ e2e：global-setup.ts 补建角色通道（不在本迁移职责内，勿在此处代授）。
//!
//! 角色按 code 定位，禁止写死 role_id 数字（本仓 role_id 字面量已收编单源，迁移内
//! 必须按 code join；m0069 起同口径）。别名码 salesperson/purchaser 分别对应
//! sales_rep/purchase_clerk（制单岗，SoD 不持审批面），故**不**出现在 reject 名单。
//!
//! 幂等与人工改判语义：`WHERE NOT EXISTS` 只按 (role_id, resource_type, action) 判存在，
//! 不看 allowed 取值 ⇒ 已有行（含人工把 allowed 改成 false 的改判行）原样保留、不翻转、
//! 不重复插；重跑等价。不用 `ON CONFLICT`：其依赖 v15 域内才创建的
//! `uq_role_permissions_role_resource_action` 唯一索引，存量库若有历史重复行会让
//! 索引创建与冲突判定双双不可靠，迁移串行执行下 NOT EXISTS 足以（同 m0069 实测裁定）。
//!
//! fail-visible（非静默跳过）：
//! - 角色码命中/缺失逐码点名 RAISE NOTICE；
//! - 另设悬空检测：某目标角色在库、但该 (角色,资源) 无配对 approve 行时点名——
//!   reject 与 approve 成对口径见 init 矩阵注释，缺配对说明该资源在此库尚未落全。
//!   0 命中/悬空均不 RAISE EXCEPTION：「迁移早于 init 建角色」的新装库与 CI 分片库
//!   （角色由 global-setup 补建）是 m0069 已裁定的合法时序，判死会打断全部新装链；
//!   可见化交迁移日志点名，禁止"跑完一条没授"当作无事发生。
//!
//! down 对称可回退：只按 (角色码 × 上述四资源 × action='reject') 精确回收本迁移授予的
//! 行；同角色其它动作、人工另授的 reject 行（其它角色）一律不动，不留空实现。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// 销售侧 reject 受授角色码（init 矩阵码；salesperson 是 sales_rep 别名，SoD 不授审批面）
const SALES_REJECT_ROLE_CODES: &[&str] = &["sales_manager"];

/// 采购侧 reject 受授角色码（init 矩阵码 purchase_manager + e2e/部署侧别名码
/// purchasing_manager；purchaser 是 purchase_clerk 别名，SoD 不授审批面。
/// 库中不存在的码自然跳过，见 up 的 NOTICE 点名）
const PURCHASE_REJECT_ROLE_CODES: &[&str] = &["purchase_manager", "purchasing_manager"];

/// 销售侧补 reject 的资源（与 init 矩阵 ("sales-contracts","reject") /
/// ("sales-prices","reject") 逐字符一致）
const SALES_REJECT_RESOURCES: &[&str] = &["sales-contracts", "sales-prices"];

/// 采购侧补 reject 的资源（同上，("purchase-contracts","reject") /
/// ("purchase-prices","reject")）
const PURCHASE_REJECT_RESOURCES: &[&str] = &["purchase-contracts", "purchase-prices"];

/// 角色码/资源名写成 SQL 数组字面量（全部为本文件编译期常量，无外部输入拼接）
fn sql_array(items: &[&str]) -> String {
    let quoted: Vec<String> = items.iter().map(|i| format!("'{i}'")).collect();
    format!("ARRAY[{}]::text[]", quoted.join(", "))
}

/// 一组 (角色码集合 × 资源 × reject) 的幂等授予语句
fn grant_insert(role_codes: &[&str], resource: &str) -> String {
    format!(
        r#"INSERT INTO "role_permissions" ("role_id", "resource_type", "action", "allowed", "created_at", "updated_at")
SELECT r."id", '{resource}', 'reject', true, NOW(), NOW()
  FROM "roles" r
 WHERE r."code" = ANY({codes_arr})
   AND NOT EXISTS (
        SELECT 1 FROM "role_permissions" x
         WHERE x."role_id" = r."id" AND x."resource_type" = '{resource}' AND x."action" = 'reject'
   );"#,
        codes_arr = sql_array(role_codes),
    )
}

/// (角色码, 资源) 配对写成 VALUES 字面量，供悬空检测 DO 块使用（同为编译期常量）
fn reject_pairs_values() -> String {
    let mut rows: Vec<String> = Vec::new();
    for code in SALES_REJECT_ROLE_CODES {
        for resource in SALES_REJECT_RESOURCES {
            rows.push(format!("('{code}', '{resource}')"));
        }
    }
    for code in PURCHASE_REJECT_ROLE_CODES {
        for resource in PURCHASE_REJECT_RESOURCES {
            rows.push(format!("('{code}', '{resource}')"));
        }
    }
    rows.join(", ")
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let all_arr = sql_array(
            &SALES_REJECT_ROLE_CODES
                .iter()
                .chain(PURCHASE_REJECT_ROLE_CODES.iter())
                .copied()
                .collect::<Vec<&str>>(),
        );
        let pairs_values = reject_pairs_values();
        let inserts: String = SALES_REJECT_RESOURCES
            .iter()
            .map(|res| grant_insert(SALES_REJECT_ROLE_CODES, res))
            .collect::<Vec<String>>()
            .join("\n\n");
        let inserts = inserts
            + "\n\n"
            + &PURCHASE_REJECT_RESOURCES
                .iter()
                .map(|res| grant_insert(PURCHASE_REJECT_ROLE_CODES, res))
                .collect::<Vec<String>>()
                .join("\n\n");
        let sql = format!(
            r#"
-- 1) 非致命可见化：统计命中/缺失角色码（合法时序与禁止静默通过的口径见文件头）。
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
        RAISE NOTICE 'm0081：合同/价格 reject 补授的目标角色（sales_manager/purchase_manager/purchasing_manager）在当前库中一个都不存在，本次未授予任何权限。若是「迁移早于系统初始化建角色」的新装库属正常时序（init 角色矩阵按同口径授予），否则请核对 roles.code 是否被改名。';
    ELSE
        RAISE NOTICE 'm0081：reject 补授命中 % 个角色；当前库不存在的角色码：%（自动跳过并在日志点名）', hit, COALESCE(miss, '无');
    END IF;
END
$$;

-- 2) 悬空点名：角色在库、但同 (角色,资源) 无配对 approve 授权行时逐对点名。
--    reject 与 approve 成对是 init 矩阵口径；缺配对说明该资源面在此库尚未落全，
--    必须看得见，不许静默授半套。
DO $$
DECLARE
    dangling TEXT;
BEGIN
    SELECT string_agg(p.role_code || ' × ' || p.resource_type, ', ' ORDER BY p.role_code, p.resource_type)
      INTO dangling
      FROM (VALUES {pairs_values}) p(role_code, resource_type)
     WHERE EXISTS (SELECT 1 FROM "roles" r WHERE r."code" = p.role_code)
       AND NOT EXISTS (
            SELECT 1 FROM "role_permissions" x
             JOIN "roles" r2 ON r2."id" = x."role_id"
             WHERE r2."code" = p.role_code AND x."resource_type" = p.resource_type AND x."action" = 'approve'
       );
    IF dangling IS NOT NULL THEN
        RAISE NOTICE 'm0081：以下 (角色 × 资源) 已授/将授 reject 但库中缺配对 approve 行，成对口径被破坏，请核对 init 矩阵落库情况：%', dangling;
    END IF;
END
$$;

-- 3) 四条幂等授予（NOT EXISTS 保证重跑等价、不覆盖人工 allowed=false 改判，见文件头）
{inserts}
"#,
            all_arr = all_arr,
            pairs_values = pairs_values,
            inserts = inserts,
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sales_arr = sql_array(SALES_REJECT_ROLE_CODES);
        let purchase_arr = sql_array(PURCHASE_REJECT_ROLE_CODES);
        let sales_res_arr = sql_array(SALES_REJECT_RESOURCES);
        let purchase_res_arr = sql_array(PURCHASE_REJECT_RESOURCES);
        let sql = format!(
            r#"
-- 只回收本迁移授予的四对 (角色码 × 资源 × reject)；同角色其它动作与人工另授不动。
DELETE FROM "role_permissions" rp
 USING "roles" r
 WHERE r."id" = rp."role_id"
   AND rp."action" = 'reject'
   AND (
        (rp."resource_type" = ANY({sales_res_arr}) AND r."code" = ANY({sales_arr}))
     OR (rp."resource_type" = ANY({purchase_res_arr}) AND r."code" = ANY({purchase_arr}))
   );
"#,
            sales_res_arr = sales_res_arr,
            sales_arr = sales_arr,
            purchase_res_arr = purchase_res_arr,
            purchase_arr = purchase_arr,
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }
}
