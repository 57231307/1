//! BPM 审批岗补授（sales_manager / purchase_manager / production_manager）的存量库通道。
//!
//! ## 背景（权限键由 URL 段派生，缺键即当场 403）
//! 运行时资源键由 `middleware/permission.rs::extract_resource_info` 从 `/api/v1/erp/{seg3}/{seg4}`
//! 派生，授权行仅当 `role_permissions.resource_type` 与派生键逐字符相同才命中（精确串匹配）。
//! BPM 四页与工单/审批执行入口的派生键为 `bpm-tasks`、`bpm-approvals`、`bpm-instances`、
//! `bpm-business-relation`、`bpm-visualization`（资源名均已登记在 `utils/path_utils.rs`
//! 的消歧映射与 `init_service.rs` 的 `PERMISSION_RESOURCES`，本迁移不新增键、不改写既有条目）。
//! 三岗在 `init_service_ops/permission.rs::bpm_approval_role_resources` 已入授权矩阵，但
//! **存量库不会重跑 init**：缺本通道则升级后三岗打开待办/审批执行/流程实例/单据关联/流程图
//! 视图恒 403，且旧行也补不出来。
//!
//! ## 三条授予通道必须同口径（本迁移是「存量库」通道）
//! - 通道① 新装库：`init_service_ops/permission.rs` 的 APPROVER_KEYS × 三岗矩阵（已到位，不动）；
//! - 通道② 存量库：本迁移（按 roles.code 反查 id，幂等补授，可重复执行）；
//! - 通道③ CI 角色：`frontend/e2e/global-setup.ts` 的 SEED_ROLE_EXTRA_PERMISSIONS
//!   （键面须与通道① 逐字符相同；角色建号面与可达集基线的当前状态另见交付回报）。
//! 本迁移的 `APPROVER_KEYS` 与通道① 六枚逐字符一致，任一侧漂移即三通道不同源。
//!
//! ## 角色定位：只按 code 反查，绝不硬编 id
//! 三枚角色码以 `init_service_ops/role.rs` 的种子码为唯一真相（`sales_manager`、
//! `purchase_manager`、`production_manager`）。库里解析不到的码不静默丢弃：命中数与缺失码
//! 一律 RAISE NOTICE 点名。注意本仓历史上有 `purchase_manager`（后端种子码）与
//! `purchasing_manager`（CI/部署侧码）两套不同源写法，本迁移按后端种子码为准，
//! 不为迁就另一侧写法而多塞一个可能不存在的码。
//!
//! ## 幂等与冲突处理（列集合与真实唯一索引逐字一致）
//! `role_permissions` 的唯一索引是 `uq_role_permissions_role_resource_action`
//! （列集合 `role_id, resource_type, action`），由 `domain/v15/` 以
//! `CREATE UNIQUE INDEX IF NOT EXISTS` 创建；本域注册在全链尾，v15 域必然先于它应用，
//! 故索引在执行时必然在册，`ON CONFLICT` 冲突目标可直接用该列集合（与 v15 自身补授同法）。
//! 与链中段（business 域）的补授迁移刻意用 `WHERE NOT EXISTS` 相反——那里索引尚未创建，
//! 为一条授予提前动全局约束不值得，本域无此约束。
//!
//! ## 不静默
//! 逐（角色 × 键）打印实写行数；未插入时分辨并点名两种成因——同键 `allowed=true` 已在
//! （幂等跳过）与同键存在**显式拒绝行** `allowed=false`（按"不覆盖人工裁定"口径保留、
//! 只 RAISE WARNING 点名，与通道① 的 `explicit_denied_kept` 同语义）。
//! 命中 0 个角色（新装库上角色与本批授权都由 init 矩阵建、本迁移自然 0 行）属正常时序，
//! 不为此把 init 矩阵的授权删掉或挪进迁移。
//!
//! ## down（真实可逆，且只收窄本次授予）
//! 仅按「三枚角色码 × 六枚 (resource_type, action) 配对」删除，不触碰其它角色、其它动作，
//! 也不删通配授予。新装库上这些同键行由 init 矩阵建立、与本迁移插入行不可区分，故 down 的
//! 语义是「撤除三岗的 BPM 审批面」，重跑通道① 或本通道即可复原，不留空实现。

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m0001_grant_bpm_approver_keys"
    }
}

/// 受授角色码（`init_service_ops/role.rs` 种子码，按码反查 id，不硬编 id）
const ROLE_CODES: &[&str] = &["sales_manager", "purchase_manager", "production_manager"];

/// 授予的 (resource_type, action) 六枚，逐字符同于通道① 的 APPROVER_KEYS
const APPROVER_KEYS: &[(&str, &str)] = &[
    ("bpm-tasks", "read"),
    ("bpm-tasks", "approve"),
    ("bpm-approvals", "create"),
    ("bpm-instances", "read"),
    ("bpm-business-relation", "read"),
    ("bpm-visualization", "read"),
];

/// 角色码写成 SQL 数组字面量
fn sql_array(codes: &[&str]) -> String {
    let items: Vec<String> = codes.iter().map(|c| format!("'{c}'")).collect();
    format!("ARRAY[{}]::text[]", items.join(", "))
}

/// (resource_type, action) 写成 VALUES 行（供补授循环遍历）
fn values_rows(rows: &[(&str, &str)]) -> String {
    rows.iter()
        .map(|(res, act)| format!("('{}', '{}')", res, act))
        .collect::<Vec<String>>()
        .join(",\n                ")
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let arr = sql_array(ROLE_CODES);
        let rows = values_rows(APPROVER_KEYS);
        let sql = format!(
            r#"
-- 1) 非致命可见化：命中角色数与缺失角色码（0 命中属新装库正常时序，不判失败）
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
        RAISE NOTICE 'BPM 审批岗补授：三枚角色码在当前库一个都不存在，本次未授予任何权限。若是「迁移早于系统初始化建角色」的新装库属正常时序（通道① init 角色矩阵按同口径授予），否则请核对 roles.code 是否被改名。';
    ELSE
        RAISE NOTICE 'BPM 审批岗补授：命中 % 个角色；当前库不存在的角色码：%（自动跳过，不静默丢弃）', hit, COALESCE(miss, '无');
    END IF;
END
$$;

-- 2) 六枚键 × 命中角色的幂等补授（冲突目标 = uq_role_permissions_role_resource_action 的真实列集合）
DO $$
DECLARE
    v_role RECORD;
    v_key RECORD;
    v_rows INTEGER;
    v_denied INTEGER;
BEGIN
    FOR v_role IN SELECT "id" AS role_id, "code" AS role_code
                    FROM "roles"
                   WHERE "code" = ANY({arr})
                   ORDER BY "code"
    LOOP
        FOR v_key IN SELECT * FROM (VALUES
                {rows}
        ) AS k(res_name, act_name)
        LOOP
            INSERT INTO "role_permissions"
                ("role_id", "resource_type", "action", "allowed", "created_at", "updated_at")
            VALUES (v_role.role_id, v_key.res_name, v_key.act_name, true, NOW(), NOW())
            ON CONFLICT ("role_id", "resource_type", "action") DO NOTHING;
            GET DIAGNOSTICS v_rows = ROW_COUNT;
            IF v_rows > 0 THEN
                RAISE NOTICE 'BPM 审批岗补授：角色=% 键=%:% 实写 % 行',
                    v_role.role_code, v_key.res_name, v_key.act_name, v_rows;
            ELSE
                SELECT COUNT(*) INTO v_denied FROM "role_permissions" x
                 WHERE x."role_id" = v_role.role_id
                   AND x."resource_type" = v_key.res_name
                   AND x."action" = v_key.act_name
                   AND x."allowed" = false;
                IF v_denied > 0 THEN
                    RAISE WARNING 'BPM 审批岗补授：角色=% 键=%:% 库里已是同键显式拒绝行（allowed=false），按不覆盖人工裁定的口径保留现状',
                        v_role.role_code, v_key.res_name, v_key.act_name;
                ELSE
                    RAISE NOTICE 'BPM 审批岗补授：角色=% 键=%:% 既有 allowed=true 授权行，幂等跳过',
                        v_role.role_code, v_key.res_name, v_key.act_name;
                END IF;
            END IF;
        END LOOP;
    END LOOP;
END
$$;
"#,
            arr = arr,
            rows = rows,
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let arr = sql_array(ROLE_CODES);
        let pairs = values_rows(APPROVER_KEYS);
        let sql = format!(
            r#"
-- 只回收「三枚角色码 × 本域六枚键」的授权行：其它角色、其它动作、通配授予一律不动。
DO $$
DECLARE
    v_rows INTEGER;
BEGIN
    DELETE FROM "role_permissions" rp
     USING "roles" r
     WHERE r."id" = rp."role_id"
       AND r."code" = ANY({arr})
       AND (rp."resource_type", rp."action") IN (
                {pairs}
       );
    GET DIAGNOSTICS v_rows = ROW_COUNT;
    RAISE NOTICE 'BPM 审批岗补授回退：按角色码与六枚键收窄回收，实删 % 行', v_rows;
END
$$;
"#,
            arr = arr,
            pairs = pairs,
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }
}
