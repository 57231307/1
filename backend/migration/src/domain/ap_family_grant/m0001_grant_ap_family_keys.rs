//! 应付族（AP）业务面与报表面授权行的存量库补授通道。
//!
//! ## 背景（权限键由 URL 段派生，缺键即入口隐去、开页 403）
//! 运行时资源键由 `middleware/permission.rs::extract_resource_info` 从 `/api/v1/erp/{seg3}/{seg4}`
//! 派生，授权行仅当 `role_permissions.resource_type` 与派生键逐字符相同才命中。应付族端点
//! 挂在双层前缀 `ap` 下，经 `utils/path_utils.rs` 的消歧映射落九枚权威资源名；本迁移授予的
//! 每一枚 (resource_type, action) 都对应一条真实在册端点，不新增键、不改写既有条目。
//!
//! ## 三条授予通道必须同口径（本迁移是「存量库」通道）
//! - 通道① 新装库：`init_service_ops/permission.rs` 的 finance_manager 与 accountant 分组
//!   已含本域全部键；
//! - 通道② 存量库：本迁移（按 roles.code 反查 id，幂等补授，可重复执行）；
//! - 通道③ CI：两岗由 `frontend/e2e/global-setup.ts` 的 SEED_ROLES 建号，其权限面来自
//!   通道① 的 init 播种，故本通道**故意 0 枚**——再往 SEED_ROLE_EXTRA_PERMISSIONS 登记
//!   同键会与 init 矩阵重复落行并撞 `uq_role_permissions_role_resource_action`。
//!
//! ## 授予面（口径与通道① 逐字符相同）
//! finance_manager 持审核面与报表面：五枚业务面的 read，加 ap-invoices 的 approve、
//! ap-reconciliations 的 confirm，再加四本应付报表的 read。
//! accountant 持制单与记账面：五枚业务面的 read，加 ap-invoices 的 create 与 update、
//! ap-verifications 的 create 与 cancel、ap-payment-requests 的 create、update 与 submit、
//! ap-reconciliations 的 create。
//! 两岗的互斥依据是 role_conflicts 内置默认的「财务制单与审核互斥」，因此审核动作不给
//! accountant、建单动作不给 finance_manager。付款执行面（ap-payments 的 create 与 confirm）
//! 与报表面（ap-aging、ap-daily、ap-monthly、ap-statistics）都刻意不授给 accountant，
//! 前者本轮未定岗、后者归管理岗；请款单的 approve 与 reject 也不授，因为请款服务的
//! check_approval_permission 另按角色码字面值分级硬门，矩阵授键放行不了那条门，
//! 责任留在真正的判定模块，不靠扩授权掩盖。
//!
//! ## 角色定位与幂等
//! 角色码以 `init_service_ops/role.rs` 的种子码为唯一真相，只按 code 反查、绝不硬编 id；
//! 库里解析不到时 RAISE NOTICE 点名而非静默丢弃。唯一索引
//! `uq_role_permissions_role_resource_action`（列集合 role_id, resource_type, action）由 m_v15_domain
//! 创建，本域注册在全链尾必然晚于它，故 `ON CONFLICT` 可直接用该列集合。未插入时分辨两种
//! 成因并分别打印：同键已有 allowed=true（幂等跳过）与同键存在显式拒绝行 allowed=false
//! （按不覆盖人工裁定口径保留、只 RAISE WARNING 点名）。新装库上命中 0 行属正常时序。

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m0001_grant_ap_family_keys"
    }
}

/// 授予的 (roles.code, resource_type, action) 三元组，逐字符同于通道① 的两个分组
const GRANTS: &[(&str, &str, &str)] = &[
    // finance_manager：审核面与报表面
    ("finance_manager", "ap-invoices", "read"),
    ("finance_manager", "ap-invoices", "approve"),
    ("finance_manager", "ap-payments", "read"),
    ("finance_manager", "ap-verifications", "read"),
    ("finance_manager", "ap-payment-requests", "read"),
    ("finance_manager", "ap-reconciliations", "read"),
    ("finance_manager", "ap-reconciliations", "confirm"),
    ("finance_manager", "ap-aging", "read"),
    ("finance_manager", "ap-daily", "read"),
    ("finance_manager", "ap-monthly", "read"),
    ("finance_manager", "ap-statistics", "read"),
    // accountant：制单与记账面
    ("accountant", "ap-invoices", "read"),
    ("accountant", "ap-invoices", "create"),
    ("accountant", "ap-invoices", "update"),
    ("accountant", "ap-payments", "read"),
    ("accountant", "ap-verifications", "read"),
    ("accountant", "ap-verifications", "create"),
    ("accountant", "ap-verifications", "cancel"),
    ("accountant", "ap-payment-requests", "read"),
    ("accountant", "ap-payment-requests", "create"),
    ("accountant", "ap-payment-requests", "update"),
    ("accountant", "ap-payment-requests", "submit"),
    ("accountant", "ap-reconciliations", "read"),
    ("accountant", "ap-reconciliations", "create"),
];

/// 受授角色码（`init_service_ops/role.rs` 种子码，按码反查 id，不硬编 id）
fn role_codes() -> Vec<&'static str> {
    let mut codes: Vec<&'static str> = GRANTS.iter().map(|(code, _, _)| *code).collect();
    codes.sort_unstable();
    codes.dedup();
    codes
}

/// 文本数组字面量（供 roles.code 命中判定与缺失点名）
fn sql_array(codes: &[&str]) -> String {
    let items: Vec<String> = codes.iter().map(|c| format!("'{c}'")).collect();
    format!("ARRAY[{}]::text[]", items.join(", "))
}

/// (roles.code, resource_type, action) 写成 VALUES 行（供补授循环遍历）
fn values_rows() -> String {
    GRANTS
        .iter()
        .map(|(code, res, act)| format!("('{}', '{}', '{}')", code, res, act))
        .collect::<Vec<String>>()
        .join(",\n                ")
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let arr = sql_array(&role_codes());
        let rows = values_rows();
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
        RAISE NOTICE '应付族授权补授：角色码在当前库不存在，本次未授予任何权限。若是「迁移早于系统初始化建角色」的新装库属正常时序（通道① init 角色矩阵按同口径授予），否则请核对 roles.code 是否被改名。';
    ELSE
        RAISE NOTICE '应付族授权补授：命中 % 个角色；当前库不存在的角色码：%（自动跳过，不静默丢弃）', hit, COALESCE(miss, '无');
    END IF;
END
$$;

-- 2) 逐枚键按角色码幂等补授（冲突目标 = uq_role_permissions_role_resource_action 的真实列集合）
DO $$
DECLARE
    v_grant RECORD;
    v_role_id INTEGER;
    v_rows INTEGER;
    v_denied INTEGER;
BEGIN
    FOR v_grant IN SELECT * FROM (VALUES
                {rows}
    ) AS g(role_code, res_name, act_name)
    LOOP
        SELECT id INTO v_role_id FROM "roles" WHERE "code" = v_grant.role_code;
        IF v_role_id IS NULL THEN
            RAISE NOTICE '应付族授权补授：角色码 % 在当前库不存在，键 %:% 本次跳过（不静默丢弃）',
                v_grant.role_code, v_grant.res_name, v_grant.act_name;
            CONTINUE;
        END IF;
        INSERT INTO "role_permissions"
            ("role_id", "resource_type", "action", "allowed", "created_at", "updated_at")
        VALUES (v_role_id, v_grant.res_name, v_grant.act_name, true, NOW(), NOW())
        ON CONFLICT ("role_id", "resource_type", "action") DO NOTHING;
        GET DIAGNOSTICS v_rows = ROW_COUNT;
        IF v_rows > 0 THEN
            RAISE NOTICE '应付族授权补授：角色=% 键=%:% 实写 % 行',
                v_grant.role_code, v_grant.res_name, v_grant.act_name, v_rows;
        ELSE
            SELECT COUNT(*) INTO v_denied FROM "role_permissions" x
             WHERE x."role_id" = v_role_id
               AND x."resource_type" = v_grant.res_name
               AND x."action" = v_grant.act_name
               AND x."allowed" = false;
            IF v_denied > 0 THEN
                RAISE WARNING '应付族授权补授：角色=% 键=%:% 库里已是同键显式拒绝行（allowed=false），按不覆盖人工裁定的口径保留现状',
                    v_grant.role_code, v_grant.res_name, v_grant.act_name;
            ELSE
                RAISE NOTICE '应付族授权补授：角色=% 键=%:% 既有 allowed=true 授权行，幂等跳过',
                    v_grant.role_code, v_grant.res_name, v_grant.act_name;
            END IF;
        END IF;
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
        let arr = sql_array(&role_codes());
        let triples = values_rows();
        let sql = format!(
            r#"
-- 只回收「本域角色码 × 本域逐枚 (resource_type, action) 配对」的授权行：按三元组配对匹配，
-- 因此 accountant 的建单键不会被从 finance_manager 名下删掉；其它角色与其它动作一律不动。
-- 且只删 allowed=true 的本域授予行：同键的人工显式拒绝行（allowed=false）不是本域写的，
-- 回退不许把它一并抹掉（up 侧对它只点名不覆盖，两侧口径必须一致）。
DO $$
DECLARE
    v_rows INTEGER;
BEGIN
    DELETE FROM "role_permissions" rp
     USING "roles" r
     WHERE r."id" = rp."role_id"
       AND r."code" = ANY({arr})
       AND rp."allowed" = true
       AND (r."code", rp."resource_type", rp."action") IN (
                {triples}
       );
    GET DIAGNOSTICS v_rows = ROW_COUNT;
    RAISE NOTICE '应付族授权回退：按角色码与本域键收窄回收，实删 % 行', v_rows;
END
$$;
"#,
            arr = arr,
            triples = triples,
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }
}
