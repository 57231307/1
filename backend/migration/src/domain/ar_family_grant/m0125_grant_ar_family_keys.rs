//! 应收族（AR）业务面与报表面授权行的存量库补授通道。
//!
//! ## 背景（权限键由 URL 段派生，缺键即入口隐去、开页必 403）
//! 运行时资源键由 `middleware/permission.rs::extract_resource_info` 从 URL 的第三段与第四段
//! 派生，授权行仅当 role_permissions.resource_type 与派生键逐字符相同才命中（同文件 709 行
//! matches_permission 是资源段等值加通配符的精确匹配，粗域展开只存在于前端路由守卫，
//! 后端不认 finance 这类粗键）。应收族端点挂在双层前缀 ar 下，经 utils/path_utils.rs 的
//! 消歧映射落八枚权威资源名（ar-invoices、ar-payments、ar-verifications、ar-collections、
//! ar-aging、ar-daily、ar-monthly、ar-statistics），八枚都已在 init_service.rs 的
//! PERMISSION_RESOURCES 在册，本迁移不新增资源键、不改写任何既有条目，授予的每一枚
//! resource_type 与 action 配对都对应一条真实在册端点。
//!
//! ## 三条授予通道必须同口径（本迁移是「存量库」通道）
//! - 通道① 新装库 services/init_service_ops/permission.rs 的 finance_manager 与 accountant
//!   分组，本批补齐同源的应收族键；
//! - 通道② 存量库，即本迁移，按 roles.code 反查 id 幂等补授，可重复执行；
//! - 通道③ CI 侧 frontend/e2e/global-setup.ts 的 SEED_ROLES 建号，两岗的权限面来自通道①
//!   的 init 播种，故本通道故意 0 枚。再往 SEED_ROLE_EXTRA_PERMISSIONS 登记同键会与
//!   init 矩阵重复落行并撞唯一索引 uq_role_permissions_role_resource_action。
//!   三通道一致性由 backend/tests/contract_ar_family_grant_channels_test.rs 双向钉住。
//!
//! ## 探测式（只补缺失，绝不覆盖既有裁定）
//! 第一步先数存量库里本域八枚资源名是否已有任何授权行并打印命中数与逐枚缺失清单，第二步
//! 才逐枚 INSERT，冲突目标用该唯一索引的真实列集合 DO NOTHING，因此已授出的行原样保留、
//! 本迁移永不 UPDATE 也永不 DELETE 一条既有授权。未插入时分辨两种成因并分别打印：同键已有
//! allowed=true（幂等跳过）与同键存在显式拒绝行 allowed=false（按不覆盖人工裁定口径保留，
//! 只 RAISE WARNING 点名）。新装库上命中 0 行属正常时序，通道① 已按同口径授出。
//!
//! ## 授予面与逐枚理由（口径与通道① 逐字符相同）
//! 岗位划分沿用应付族既定的「财务制单与审核互斥」（role_conflicts 内置默认把 accountant 与
//! finance_manager 互斥），两岗各持半程才成立，故审核动作不给 accountant、建单动作不给
//! finance_manager。出纳（cashier）刻意不在本域角色集内，不持任何 ar-* 键。
//!
//! finance_manager 持审核面、报表面与催收出文面：
//! - ar-invoices 的 read 是审核前置（发票列表与详情端点）。
//! - ar-invoices 的 approve 对应 POST 发票审核端点，末段 approve 命中路径动作关键字表。
//! - ar-invoices 的 cancel 对应发票作废端点，作废属管理面而非制单面，与 approve 同岗。
//! - ar-payments 的 read 是收款核对前置。
//! - ar-payments 的 confirm 对应收款确认端点，把收款落到应收余额是审核腿，不给制单岗。
//! - ar-verifications 的 read 是核销复核前置。
//! - ar-collections 的 print 是本资源唯一可派生的动作（该资源只有收款单 DOCX 打印一条 GET
//!   端点，没有任何 read 端点），前端收款页签的行内打印按钮是活调用方，缺键即点了必 403。
//!   本岗持收款读权，催收出文归管理岗，故授。
//! - ar-aging、ar-daily、ar-monthly、ar-statistics 的 read 是四本应收报表，URL 段在 ar
//!   前缀下的 reports 子模块里，第四段是嵌套前缀故资源名落在第五段并经映射表消歧，四本账
//!   只有 GET，可派生动作仅 read 一种，给通配符会造出看着全权、实则只有读的悬空授权。
//!
//! accountant 持制单与记账面：
//! - ar-invoices 的 read 与 create 与 update：建单是手工录入应收发票的唯一入口，改单必需
//!   （自动落草稿的状态门只放草稿通过，不给改单则本岗无法修正金额与发票日期与付款条件，
//!   只能退回超管代改）。不给删单（草稿可用改单修正，删单属管理面，应付族同岗同样不给），
//!   不给审核与作废（与 finance_manager 制单审核互斥）。
//! - ar-payments 的 read 与 create 与 update：收款登记是本岗的记账职责，建单与改单成对
//!   （错录可改）。不给 confirm（审核腿），不给 cancel（该作废端点在前端零调用方，授了
//!   等于给无人使用的口子扩面）。
//! - ar-verifications 的 read 与 create 与 cancel：核销本身是自动与手工两条 POST 端点，
//!   两末段都不在动作关键字表内，一律按方法派生 create，故授 create 即同时放出自动核销与
//!   手工核销两条腿，这是已知捆绑、不靠少授掩盖。撤核必需，错核只能靠余额回滚修正，
//!   不给则错核永久污染应收余额且应用内无任何更正入口（与应付族同岗同判据）。
//! - ar-collections 的 print 与本岗的 ar-payments read 同页签共存，收款页签对本岗可见，
//!   行内打印按钮就必须可用，否则本岗一进页签就撞一枚必 403 的入口。
//! - 报表四枚刻意不给本岗，与应付族同口径（报表面归管理岗）。
//! - ar-invoices 的 export 刻意不授：导出按钮已由 v-permission 按该键守卫，缺键即按钮隐去，
//!   不形成 403；应付族同岗同样不授，两族保持一致。

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m0125_grant_ar_family_keys"
    }
}

/// 授予的 (roles.code, resource_type, action) 三元组，逐字符同于通道① 的两个分组
const GRANTS: &[(&str, &str, &str)] = &[
    // finance_manager 审核面、报表面与催收出文面
    ("finance_manager", "ar-invoices", "read"),
    ("finance_manager", "ar-invoices", "approve"),
    ("finance_manager", "ar-invoices", "cancel"),
    ("finance_manager", "ar-payments", "read"),
    ("finance_manager", "ar-payments", "confirm"),
    ("finance_manager", "ar-verifications", "read"),
    ("finance_manager", "ar-collections", "print"),
    ("finance_manager", "ar-aging", "read"),
    ("finance_manager", "ar-daily", "read"),
    ("finance_manager", "ar-monthly", "read"),
    ("finance_manager", "ar-statistics", "read"),
    // accountant 制单与记账面
    ("accountant", "ar-invoices", "read"),
    ("accountant", "ar-invoices", "create"),
    ("accountant", "ar-invoices", "update"),
    ("accountant", "ar-payments", "read"),
    ("accountant", "ar-payments", "create"),
    ("accountant", "ar-payments", "update"),
    ("accountant", "ar-verifications", "read"),
    ("accountant", "ar-verifications", "create"),
    ("accountant", "ar-verifications", "cancel"),
    ("accountant", "ar-collections", "print"),
];

/// 受授角色码（init_service_ops/role.rs 的种子码，按码反查 id，不硬编 id）
fn role_codes() -> Vec<&'static str> {
    let mut codes: Vec<&'static str> = GRANTS.iter().map(|(code, _, _)| *code).collect();
    codes.sort_unstable();
    codes.dedup();
    codes
}

/// 本域八枚资源名（探测步的点名面，与授予面同源）
fn resource_names() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = GRANTS.iter().map(|(_, res, _)| *res).collect();
    names.sort_unstable();
    names.dedup();
    names
}

/// 文本数组字面量（供 roles.code 与 resource_type 的命中判定与缺失点名）
fn sql_array(items: &[&str]) -> String {
    let cells: Vec<String> = items.iter().map(|c| format!("'{c}'")).collect();
    format!("ARRAY[{}]::text[]", cells.join(", "))
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
        let res_arr = sql_array(&resource_names());
        let rows = values_rows();
        let sql = format!(
            r#"
-- 1) 探测步：先数存量库里本域八枚资源名已有的授权行，逐枚点名有与无，不改任何数据
DO $$
DECLARE
    v_res RECORD;
    v_have INTEGER;
    v_missing TEXT;
    v_total INTEGER;
BEGIN
    SELECT COUNT(*) INTO v_total FROM "role_permissions"
     WHERE "resource_type" = ANY({res_arr});
    RAISE NOTICE '应收族授权补授探测：本域八枚资源名在角色维共 % 条既有授权行（0 条即除超管外谁都调不到，正是本迁移要补的缺口）', v_total;

    FOR v_res IN SELECT unnest({res_arr}) AS r
    LOOP
        SELECT COUNT(*) INTO v_have FROM "role_permissions"
         WHERE "resource_type" = v_res.r;
        IF v_have = 0 THEN
            RAISE NOTICE '应收族授权补授探测：资源名 % 存量零授权行，本次按本域键集补授', v_res.r;
        ELSE
            RAISE NOTICE '应收族授权补授探测：资源名 % 已有 % 条授权行，既有行一律保留不覆盖', v_res.r, v_have;
        END IF;
    END LOOP;

    SELECT string_agg(t, ', ' ORDER BY t) INTO v_missing
      FROM (SELECT a AS t FROM unnest({res_arr}) a
             WHERE NOT EXISTS (SELECT 1 FROM "role_permissions" x WHERE x."resource_type" = a)) m;
    IF v_missing IS NOT NULL THEN
        RAISE NOTICE '应收族授权补授探测：完全缺失的资源名清单：%（缺失即非超管角色打开该面必 403）', v_missing;
    END IF;
END
$$;

-- 2) 非致命可见化：命中角色数与缺失角色码（0 命中属新装库正常时序，不判失败）
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
        RAISE NOTICE '应收族授权补授：角色码在当前库不存在，本次未授予任何权限。若是「迁移早于系统初始化建角色」的新装库属正常时序（通道① init 角色矩阵按同口径授予），否则请核对 roles.code 是否被改名。';
    ELSE
        RAISE NOTICE '应收族授权补授：命中 % 个角色；当前库不存在的角色码：%（自动跳过，不静默丢弃）', hit, COALESCE(miss, '无');
    END IF;
END
$$;

-- 3) 逐枚键按角色码幂等补授（冲突目标 = uq_role_permissions_role_resource_action 的真实列集合）
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
            RAISE NOTICE '应收族授权补授：角色码 % 在当前库不存在，键 % 的 % 动作本次跳过（不静默丢弃）',
                v_grant.role_code, v_grant.res_name, v_grant.act_name;
            CONTINUE;
        END IF;
        INSERT INTO "role_permissions"
            ("role_id", "resource_type", "action", "allowed", "created_at", "updated_at")
        VALUES (v_role_id, v_grant.res_name, v_grant.act_name, true, NOW(), NOW())
        ON CONFLICT ("role_id", "resource_type", "action") DO NOTHING;
        GET DIAGNOSTICS v_rows = ROW_COUNT;
        IF v_rows > 0 THEN
            RAISE NOTICE '应收族授权补授：角色=% 键=% 动作=% 实写 % 行',
                v_grant.role_code, v_grant.res_name, v_grant.act_name, v_rows;
        ELSE
            SELECT COUNT(*) INTO v_denied FROM "role_permissions" x
             WHERE x."role_id" = v_role_id
               AND x."resource_type" = v_grant.res_name
               AND x."action" = v_grant.act_name
               AND x."allowed" = false;
            IF v_denied > 0 THEN
                RAISE WARNING '应收族授权补授：角色=% 键=% 动作=% 库里已是同键显式拒绝行（allowed=false），按不覆盖人工裁定的口径保留现状',
                    v_grant.role_code, v_grant.res_name, v_grant.act_name;
            ELSE
                RAISE NOTICE '应收族授权补授：角色=% 键=% 动作=% 既有 allowed=true 授权行，幂等跳过',
                    v_grant.role_code, v_grant.res_name, v_grant.act_name;
            END IF;
        END IF;
    END LOOP;
END
$$;
"#,
            arr = arr,
            res_arr = res_arr,
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
-- 只回收「本域角色码 乘 本域逐枚 (resource_type, action) 配对」的授权行：按三元组配对匹配，
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
    RAISE NOTICE '应收族授权回退：按角色码与本域键收窄回收，实删 % 行', v_rows;
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
