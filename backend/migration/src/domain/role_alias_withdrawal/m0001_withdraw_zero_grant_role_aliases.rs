//! 零授权别名岗的存量库回收：把用户并到正主，再删掉别名岗本身。
//!
//! ## 为什么这三枚必须回收
//! sales（销售员）、warehouse（仓库员）、super_admin（超级管理员）由 `init_service_ops/role.rs`
//! 建过角色行，但 `init_service_ops/permission.rs` 的 40 个分组里没有它们。运行期超管旁路只认
//! `utils/admin_checker.rs::ADMIN_ROLE_CODE` 的全等比较，`is_system` 不授予任何权限，
//! 应用外壳码（dashboard/notifications）也只在分组循环内逐组下发，于是这三枚岗的
//! role_permission 行数恒为 0：被指派它们的用户登录后被前端路由守卫送到 /403，连落地页都没有。
//! 本域起这三枚码不再播种，正主分别是 sales_rep、warehouse_keeper、system_admin。
//!
//! ## 改指向的两条通道都要动
//! 用户与角色的关联有两处：`users.role_id`（登录态实际读取的列）与 `user_role` 关联表
//! （带 uniq_user_role 唯一键）。只改一处会让另一处继续指向已删角色——`user_role.role_id`
//! 的外键是 ON DELETE CASCADE 会被静默清空，而 `users.role_id` 的外键是 NO ACTION，
//! 不先改指向就直接删角色会让整条迁移在 FK 上失败并阻断启动。
//! 先把"同一用户已同时挂着别名与正主"的重复关联行删掉，再改剩余关联行，避免撞 uniq_user_role。
//!
//! ## 不销毁人工配置
//! `data_permissions` 与 `field_permissions` 对 roles 是 NO ACTION 外键，且存的是人工配置行。
//! 别名岗若在这两张表里有配置，说明有人真在用这套面，此时**保留角色行并 RAISE WARNING 点名**，
//! 交人工处置；两张表都没有行时才删角色行。`role_permissions` 里别名岗的行按定义是永不命中
//! 的死授权（矩阵从未给过它们），回收并打印行数。`permission_change_audits` 与
//! `customer_field_permission` 的 role_id 是无外键的历史/软引用，不动，留作留痕。
//!
//! ## 幂等
//! 别名岗不存在时（新装库从未建过，或本域已跑过）RAISE NOTICE 报"本次未回收任何行"。
//! 正主不存在时 RAISE EXCEPTION 中止——把人并进一个不存在的角色比留着别名更糟，必须可见失败。

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m0001_withdraw_zero_grant_role_aliases"
    }
}

/// (别名岗码, 正主岗码, 别名岗名称, 别名岗描述, 别名岗 data_scope)——名称与描述取
/// init_service_ops/role.rs 撤销前的种子原值，供 down 重建时逐字恢复
const ALIAS_PAIRS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "sales",
        "sales_rep",
        "销售员",
        "销售基础操作（订单录入与客户跟进）",
        "self",
    ),
    (
        "warehouse",
        "warehouse_keeper",
        "仓库员",
        "仓库收发货与库存操作",
        "self",
    ),
    (
        "super_admin",
        "system_admin",
        "超级管理员",
        "系统级配置与运维管理（仅用于系统配置）",
        "all",
    ),
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for (alias, canonical, _name, _desc, _scope) in ALIAS_PAIRS {
            let sql = format!(
                r#"
DO $$
DECLARE
    v_alias INTEGER;
    v_canon INTEGER;
    v_dupe INTEGER;
    v_moved INTEGER;
    v_users INTEGER;
    v_grants INTEGER;
    v_cfg INTEGER;
BEGIN
    SELECT id INTO v_alias FROM "roles" WHERE "code" = '{alias}' LIMIT 1;
    IF v_alias IS NULL THEN
        RAISE NOTICE '别名岗 {alias} 回收：当前库不存在该角色码，本次未回收任何行（新装库从未播种或本域已执行过均属正常）';
        RETURN;
    END IF;

    SELECT id INTO v_canon FROM "roles" WHERE "code" = '{canonical}' LIMIT 1;
    IF v_canon IS NULL THEN
        RAISE EXCEPTION '别名岗 {alias} 回收中止：正主岗 {canonical} 在当前库解析不到，把人并入不存在的角色会让该用户彻底失去可判定权限面。请先确认系统初始化已建过 {canonical}，或核对 roles.code 是否被改名';
    END IF;

    DELETE FROM "user_role" ur
     WHERE ur."role_id" = v_alias
       AND EXISTS (SELECT 1 FROM "user_role" x WHERE x."user_id" = ur."user_id" AND x."role_id" = v_canon);
    GET DIAGNOSTICS v_dupe = ROW_COUNT;

    UPDATE "user_role" SET "role_id" = v_canon, "updated_at" = NOW() WHERE "role_id" = v_alias;
    GET DIAGNOSTICS v_moved = ROW_COUNT;

    UPDATE "users" SET "role_id" = v_canon, "updated_at" = NOW() WHERE "role_id" = v_alias;
    GET DIAGNOSTICS v_users = ROW_COUNT;

    DELETE FROM "role_permissions" WHERE "role_id" = v_alias;
    GET DIAGNOSTICS v_grants = ROW_COUNT;

    SELECT COUNT(*) INTO v_cfg FROM (
        SELECT 1 FROM "data_permissions" WHERE "role_id" = v_alias
        UNION ALL
        SELECT 1 FROM "field_permissions" WHERE "role_id" = v_alias
    ) c;

    IF v_cfg > 0 THEN
        RAISE WARNING '别名岗 {alias} 回收停在删角色前：该岗在 data_permissions 或 field_permissions 里有 % 行人工配置（外键为 NO ACTION，删角色会连带失败）。已改指向正主 users=% 行、user_role=% 行（另删重复关联 % 行），回收死授权 % 行，角色行保留待人工处置',
            v_cfg, v_users, v_moved, v_dupe, v_grants;
    ELSE
        DELETE FROM "roles" WHERE "id" = v_alias;
        RAISE NOTICE '别名岗 {alias} 已回收并删除：用户改指向 {canonical} 共 users=% 行、user_role=% 行（另删重复关联 % 行），回收永不命中的授权 % 行，删除角色行 1 行',
            v_users, v_moved, v_dupe, v_grants;
    END IF;
END
$$;
"#,
                alias = alias,
                canonical = canonical,
            );
            manager.get_connection().execute_unprepared(&sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for (alias, _canonical, name, desc, scope) in ALIAS_PAIRS {
            let sql = format!(
                r#"
DO $$
DECLARE
    v_rows INTEGER;
BEGIN
    INSERT INTO "roles" ("name", "code", "description", "is_system", "is_deleted", "data_scope", "created_at", "updated_at")
    VALUES ('{name}', '{alias}', '{desc}', TRUE, FALSE, '{scope}', NOW(), NOW())
    ON CONFLICT ("code") DO NOTHING;
    GET DIAGNOSTICS v_rows = ROW_COUNT;
    RAISE NOTICE '别名岗 {alias} 回退：重建角色行 % 行（已存在则幂等跳过）。注意本回退不把 users 与 user_role 的指向挪回别名岗——该岗本就零授权，挪回去等于恢复 /403 状态',
        v_rows;
END
$$;
"#,
                alias = alias,
                name = name,
                desc = desc,
                scope = scope,
            );
            manager.get_connection().execute_unprepared(&sql).await?;
        }
        Ok(())
    }
}
