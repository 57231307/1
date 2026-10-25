//! 客户信用族授权行的存量库补授通道。
//!
//! ## 六要素
//! - 功能：为财务两岗按 roles.code 反查 id，幂等补授 customer-credits 资源下的若干动作键，
//!   使存量库的财务岗能打开客户信用评级列表、设置评级、占用与释放额度。授予面与通道一的
//!   init 角色矩阵逐字符同集合，本迁移是 存量库 通道。
//! - 调用方：SeaORM migrator 链，注册在 domain/customer_credit_family_grant 独立域、
//!   migration/src/lib.rs 链尾新顶层条目（晚于 domain/v15）。
//! - 入参：无（读 roles 与 role_permissions 两表，授予面来自本文件的 GRANTS 常量）。
//! - 传给谁：写库 role_permissions 表；运行时由 middleware/permission.rs 按 URL 段派生的
//!   资源键与动作比对命中。
//! - 存什么：每枚 (roles.code, resource_type, action) 对应的 allowed=true 授权行。
//! - 存哪里：PostgreSQL public.role_permissions。
//!
//! ## 授予面对应通道一矩阵的哪几枚键（只列键名）
//! - finance_manager 分组：customer-credits 的 read、create、update、delete、release 五枚。
//! - accountant 分组：customer-credits 的 read、create、update 三枚。
//! 停用（delete）与额度释放（release）属审核处置面，按内置的财务制单与审核互斥留在
//! finance_manager，不给 accountant，避免同岗自建自销。print 本轮不授：客户信用打印在前端
//! 零调用方，授出去等于给无人使用的口子扩面。其余岗位一律不授，尤其不用动作段通配符：通配会
//! 经前端 actionEquivalent 与域别名映射把别的域变成新增可达页并触发基线重签。
//!
//! ## 背景（权限键由 URL 段派生，缺键即入口恒 403）
//! 运行时资源键由 middleware/permission.rs 的 extract_resource_info 从 /api/v1/erp 之后的
//! 第三、第四段派生：crm 是模块前缀，第四段 customer-credits 未被 utils/path_utils.rs 的消歧
//! 映射改写，权威资源名就是 URL 段原名，且已在 init_service.rs 的 PERMISSION_RESOURCES 在册。
//! 授权行只有与派生键逐字符相同才命中，所以本迁移授予的每一枚 (resource_type, action) 都对应
//! 一条真实在册端点，不新增资源键、不改写既有条目。
//! 缺本通道时：客户信用面在存量库里只有 crm_manager 拿得到（通配授权），财务两岗打开信用评级
//! 列表、设置评级、占用与释放额度全部恒 403；而 init 角色矩阵只在系统初始化时播种，存量库不会
//! 重跑 init，光靠通道一 补不齐已建库的岗位。
//!
//! ## 三条授予通道必须同口径（本迁移是 存量库 通道）
//! - 通道一 新装库：services/init_service_ops/permission.rs 的 finance_manager 与 accountant
//!   两个分组含本域全部键；
//! - 通道二 存量库：本迁移（按 roles.code 反查 id，幂等补授，可重复执行）；
//! - 通道三 CI：两岗都在 frontend/e2e/global-setup.ts 的 SEED_ROLES 里建号，其权限面来自通道一
//!   的 init 播种，故本通道故意登记 0 枚；再往 SEED_ROLE_EXTRA_PERMISSIONS 写同键就会与 init
//!   矩阵重复落行并撞 uq_role_permissions_role_resource_action 唯一索引。
//! 通道一 与通道二 的键集合逐字符一致性由
//! tests/contract_customer_credit_family_grant_channels_test.rs 双向钉住，任一侧漂移即点名差集。
//!
//! ## 派生细节（评级设置/占用/调整/停用/评估都按方法派生成 create）
//! 这五个端点都是 POST 且末段（rating、occupy、adjust、deactivate、evaluate）都不在
//! middleware/permission.rs 的 PATH_ACTION_KEYWORDS 表内，一律按方法派生成 create，所以 create
//! 与 release 必须同岗：只授 release 会出现能释放额度却不能占用额度的半截授权，业务上不可用。
//!
//! ## 角色定位与幂等
//! 角色码以 init_service_ops/role.rs 的种子码为唯一真相（finance_manager、accountant），只按
//! code 反查 id、绝不硬编 id；库里解析不到的码不静默丢弃，命中数与缺失码一律 RAISE NOTICE 点名。
//! 唯一索引 uq_role_permissions_role_resource_action（列集合 role_id、resource_type、action）由
//! domain/v15 创建，本域注册在全链尾必然晚于它，故 ON CONFLICT 可直接用该列集合。未插入时分辨
//! 两种成因并分别打印：同键已有 allowed=true 授权行（幂等跳过，RAISE NOTICE）与同键存在显式拒绝
//! 行 allowed=false（按不覆盖人工裁定的口径保留现状，只 RAISE WARNING 点名）。新装库上命中 0 行
//! 属正常时序，不为此判失败。
//!
//! ## down（真实可逆，且只收窄本域授予）
//! 按 角色码 与 本域逐枚 (resource_type, action) 配对 三元素匹配删除，因此 accountant 的停用与
//! 释放键不会被从 finance_manager 名下删掉；且只删 allowed=true 的本域授予行：同键的人工显式拒绝
//! 行（allowed=false）不是本域写的，回退不许把它一并抹掉（up 侧对它只点名不覆盖，两侧口径必须
//! 一致）。其它角色（含 crm_manager 名下的通配授权行）、其它资源与其它动作一律不动。

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m0001_grant_customer_credit_family_keys"
    }
}

/// 授予的 (roles.code, resource_type, action) 三元组，逐字符同于通道一 的两个分组
const GRANTS: &[(&str, &str, &str)] = &[
    // finance_manager：授信、调整、停用与额度释放的完整处置面
    ("finance_manager", "customer-credits", "read"),
    ("finance_manager", "customer-credits", "create"),
    ("finance_manager", "customer-credits", "update"),
    ("finance_manager", "customer-credits", "delete"),
    ("finance_manager", "customer-credits", "release"),
    // accountant：查看与建单改单面，处置面（delete 与 release）不在本岗
    ("accountant", "customer-credits", "read"),
    ("accountant", "customer-credits", "create"),
    ("accountant", "customer-credits", "update"),
];

/// 受授角色码（init_service_ops/role.rs 种子码，按码反查 id，不硬编 id）
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

/// (roles.code, resource_type, action) 写成 VALUES 行（供补授循环遍历与回退配对匹配）
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
        RAISE NOTICE '客户信用族授权补授：角色码在当前库不存在，未授予任何权限。若是（迁移早于系统初始化建角色）的新装库属正常时序（通道一 的 init 角色矩阵按同口径授予），否则请核对 roles.code 是否被改名。';
    ELSE
        RAISE NOTICE '客户信用族授权补授：命中 % 个角色；当前库不存在的角色码：%（自动跳过，不静默丢弃）', hit, COALESCE(miss, '无');
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
            RAISE NOTICE '客户信用族授权补授：角色码 % 在当前库不存在，键 %:% 跳过（不静默丢弃）',
                v_grant.role_code, v_grant.res_name, v_grant.act_name;
            CONTINUE;
        END IF;
        INSERT INTO "role_permissions"
            ("role_id", "resource_type", "action", "allowed", "created_at", "updated_at")
        VALUES (v_role_id, v_grant.res_name, v_grant.act_name, true, NOW(), NOW())
        ON CONFLICT ("role_id", "resource_type", "action") DO NOTHING;
        GET DIAGNOSTICS v_rows = ROW_COUNT;
        IF v_rows > 0 THEN
            RAISE NOTICE '客户信用族授权补授：角色=% 键=%:% 实写 % 行',
                v_grant.role_code, v_grant.res_name, v_grant.act_name, v_rows;
        ELSE
            SELECT COUNT(*) INTO v_denied FROM "role_permissions" x
             WHERE x."role_id" = v_role_id
               AND x."resource_type" = v_grant.res_name
               AND x."action" = v_grant.act_name
               AND x."allowed" = false;
            IF v_denied > 0 THEN
                RAISE WARNING '客户信用族授权补授：角色=% 键=%:% 库里已是同键显式拒绝行（allowed=false），按不覆盖人工裁定的口径保留现状',
                    v_grant.role_code, v_grant.res_name, v_grant.act_name;
            ELSE
                RAISE NOTICE '客户信用族授权补授：角色=% 键=%:% 既有 allowed=true 授权行，幂等跳过',
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
-- 只回收（本域角色码 与 本域逐枚 (resource_type, action) 配对）的授权行：按三元组配对匹配，
-- 因此 accountant 没有的停用与释放键不会被从 finance_manager 名下删掉；其它角色（含
-- crm_manager 的通配授权行）、其它资源与其它动作一律不动。
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
    RAISE NOTICE '客户信用族授权回退：按角色码与本域键收窄回收，实删 % 行', v_rows;
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
