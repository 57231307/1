//! 系统管理岗（system_admin）补授 BPM 流程配置读面的存量库通道。
//!
//! ## 背景（权限键由 URL 段派生，缺键即入口隐去、开页 403）
//! 运行时资源键由 `middleware/permission.rs::extract_resource_info` 从 `/api/v1/erp/{seg3}/{seg4}`
//! 派生，授权行仅当 `role_permissions.resource_type` 与派生键逐字符相同才命中。流程定义页与
//! 流程模板页的门控键是 `bpm-definitions` 与 `bpm-templates` 的 read 形态，两枚资源名都已
//! 登记在 `utils/path_utils.rs` 的消歧映射与 `init_service.rs` 的 `PERMISSION_RESOURCES`，
//! 本迁移不新增键、不改写既有条目。
//!
//! ## 三条授予通道必须同口径（本迁移是「存量库」通道）
//! - 通道① 新装库：`init_service_ops/permission.rs` 的 system_admin 分组已含这两枚键；
//! - 通道② 存量库：本迁移（按 roles.code 反查 id，幂等补授，可重复执行）；
//! - 通道③ CI：system_admin 由 `frontend/e2e/global-setup.ts` 的 SEED_ROLES 建号，其权限面
//!   来自通道① 的 init 播种，故本通道**故意 0 枚**——再往 SEED_ROLE_EXTRA_PERMISSIONS 登记
//!   同键会与 init 矩阵重复落行并撞 `uq_role_permissions_role_resource_action`。
//!
//! ## 只授读面
//! 审批执行面（bpm-tasks 与 bpm-approvals）不授给本岗，与 `bpm_approver_grant` 域授予的
//! 三岗审批面互为对称，保持"配置流程的人不执行审批"的拆岗口径。写面（definitions 与
//! templates 的增删改）本轮也不授，属后续待裁，不自签扩权。
//!
//! ## 角色定位与幂等
//! 角色码以 `init_service_ops/role.rs` 的种子码为唯一真相，只按 code 反查、绝不硬编 id；
//! 库里解析不到时 RAISE NOTICE 点名而非静默丢弃。唯一索引
//! `uq_role_permissions_role_resource_action`（列集合 role_id, resource_type, action）由 v15 域
//! 创建，本域注册在全链尾必然晚于它，故 `ON CONFLICT` 可直接用该列集合。未插入时分辨两种
//! 成因并分别打印：同键已有 allowed=true（幂等跳过）与同键存在显式拒绝行 allowed=false
//! （按不覆盖人工裁定口径保留、只 RAISE WARNING 点名）。新装库上命中 0 行属正常时序。
//!
//! ## down（真实可逆，且只收窄本次授予）
//! 仅按「system_admin × 这两枚 (resource_type, action) 配对」删除，不触碰其它角色、其它动作，
//! 也不删通配授予。新装库上同键行由通道① 建立、与本域插入行不可区分，故 down 的语义是
//! 「撤除系统管理岗的 BPM 流程配置读面」，重跑通道① 或本通道即可复原。

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m0001_grant_system_admin_bpm_config_keys"
    }
}

/// 受授角色码（`init_service_ops/role.rs` 种子码，按码反查 id，不硬编 id）
const ROLE_CODES: &[&str] = &["system_admin"];

/// 授予的 (resource_type, action) 两枚，逐字符同于通道① 的 system_admin 分组
const CONFIG_KEYS: &[(&str, &str)] = &[("bpm-definitions", "read"), ("bpm-templates", "read")];

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
        let rows = values_rows(CONFIG_KEYS);
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
        RAISE NOTICE '系统管理岗 BPM 配置读面补授：角色码在当前库不存在，本次未授予任何权限。若是「迁移早于系统初始化建角色」的新装库属正常时序（通道① init 角色矩阵按同口径授予），否则请核对 roles.code 是否被改名。';
    ELSE
        RAISE NOTICE '系统管理岗 BPM 配置读面补授：命中 % 个角色；当前库不存在的角色码：%（自动跳过，不静默丢弃）', hit, COALESCE(miss, '无');
    END IF;
END
$$;

-- 2) 两枚键 × 命中角色的幂等补授（冲突目标 = uq_role_permissions_role_resource_action 的真实列集合）
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
                RAISE NOTICE '系统管理岗 BPM 配置读面补授：角色=% 键=%:% 实写 % 行',
                    v_role.role_code, v_key.res_name, v_key.act_name, v_rows;
            ELSE
                SELECT COUNT(*) INTO v_denied FROM "role_permissions" x
                 WHERE x."role_id" = v_role.role_id
                   AND x."resource_type" = v_key.res_name
                   AND x."action" = v_key.act_name
                   AND x."allowed" = false;
                IF v_denied > 0 THEN
                    RAISE WARNING '系统管理岗 BPM 配置读面补授：角色=% 键=%:% 库里已是同键显式拒绝行（allowed=false），按不覆盖人工裁定的口径保留现状',
                        v_role.role_code, v_key.res_name, v_key.act_name;
                ELSE
                    RAISE NOTICE '系统管理岗 BPM 配置读面补授：角色=% 键=%:% 既有 allowed=true 授权行，幂等跳过',
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
        let pairs = values_rows(CONFIG_KEYS);
        let sql = format!(
            r#"
-- 只回收「system_admin × 本域两枚键」的授权行：其它角色、其它动作、通配授予一律不动。
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
    RAISE NOTICE '系统管理岗 BPM 配置读面回退：按角色码与两枚键收窄回收，实删 % 行', v_rows;
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
