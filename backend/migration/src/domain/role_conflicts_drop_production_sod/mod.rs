//! 撤掉两条内置「生产 ↔ 质量」SoD 互斥种子，财务钱账分管的 6 对保持内置默认
//!
//! ## 六要素
//! - **功能**：从 `role_conflicts`（SoD 互斥角色对表）按精确
//!   `(role_a_code, role_b_code, conflict_type)` 三元组删掉 2 行内置默认
//!   （`production_manager↔qc_manager`、`dyeing_master↔quality_inspector`）。
//!   纺织厂一人多岗是常态，生产侧与质量侧不得由系统内置硬互斥；钱账分管
//!   （会计/出纳/财务经理与采购、销售收款侧共 6 对）仍作为系统内置默认保留。
//! - **调用方**：SeaORM migrator 链，注册在 `migration/src/domain/` 全部既有域之后
//!   （链尾新顶层域）。必须是新域而不是改 `migration/src/domain/finance/`（该域建表并
//!   播种这 8 对）：已应用的域在复用库上因台账已记账被整体跳过，删种动作永不执行。
//! - **入参**：无。删除目标取自本文件常量 `DROP_CONFLICT_TRIPLES`，逐字对齐 `finance`
//!   域建表时的种子值；字典序满足该表的 `chk_role_order`（`role_a_code < role_b_code`）。
//! - **传给谁**：读这张表的两处消费方——`src/services/user_service.rs` 的
//!   `check_role_conflict_for_user`（`update_user` 换角色时的写口校验，双向匹配）与
//!   `src/handlers/role_handler.rs` 的 `GET /roles/conflicts` 列表端点。表、端点、写口
//!   校验等互斥机制全部保留；企业自配的互斥行不在删除条件之内，一律不受影响。
//!   另一套 `role_relations` 的 inherit / mutual_exclusive 机制（由
//!   `migration/src/domain/v15/` 播种、`src/services/role_relation_service.rs` 消费）
//!   与本表无关，本域不触碰。
//! - **存什么**：只删数据行；不改表结构、不改约束。
//! - **存哪里**：PostgreSQL `public.role_conflicts` 表。
//!
//! ## 保留的内置默认（6 对，全部财务侧钱账分管）
//! `accountant↔finance_manager`、`accountant↔cashier`、`cashier↔finance_manager`、
//! `cashier↔purchase_manager`、`cashier↔purchase_clerk`、`cashier↔sales_manager`。
//! 该清单与本域的删除动作互锁：up 在删除前后各数一次这 6 对的在场行数，不等即
//! `RAISE EXCEPTION` 中止——误伤财务默认或企业自配行时绝不静默放过。
//!
//! ## 幂等与 0 行语义
//! up 逐对精确删除，重跑第二次命中 0 行属正常结果（该库从未跑过 `finance` 域种子，
//! 或本域已跑过一次）；影响行数一律 `RAISE NOTICE` 点名外显，不做静默跳过。
//! down 用与 `finance` 域原种子逐字相同的取值与列集合插回这 2 对，配
//! `ON CONFLICT (role_a_code, role_b_code) DO NOTHING`，重复执行不撞 `uniq_role_pair`。

use sea_orm_migration::prelude::*;

/// 本域撤掉的内置默认，逐字取自 `finance` 域建表时种进 `role_conflicts` 的取值
/// （role_a_code, role_b_code, conflict_type）。删除只按这三元组精确命中。
const DROP_CONFLICT_TRIPLES: [(&str, &str, &str); 2] = [
    ("production_manager", "qc_manager", "sod"),
    ("dyeing_master", "quality_inspector", "sod"),
];

/// 必须原样保留的 6 对内置默认（财务侧钱账分管），作为删除前后在场行数自检的判据集。
const KEEP_FINANCE_PAIRS: [(&str, &str); 6] = [
    ("accountant", "finance_manager"),
    ("accountant", "cashier"),
    ("cashier", "finance_manager"),
    ("cashier", "purchase_manager"),
    ("cashier", "purchase_clerk"),
    ("cashier", "sales_manager"),
];

/// down 插回时使用的完整行（含 description），取值逐字照 `finance` 域原种子的
/// `(role_a_code, role_b_code, conflict_type, description)` 四列。
const RESTORE_ROWS: [(&str, &str, &str, &str); 2] = [
    (
        "production_manager",
        "qc_manager",
        "sod",
        "生产与质量管理互斥",
    ),
    (
        "dyeing_master",
        "quality_inspector",
        "sod",
        "染色主管与质检员互斥",
    ),
];

/// 把二元组列表拼成 SQL 行构造器 `IN (('a','b'),('c','d'))`。
///
/// 输入只来自本文件的有界常量数组（`'static str`，无外部输入通道），不存在注入面。
fn row_tuple_list(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(a, b)| format!("('{}','{}')", a, b))
        .collect::<Vec<String>>()
        .join(",")
}

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_role_conflicts_drop_production_sod"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let keep_pairs = row_tuple_list(&KEEP_FINANCE_PAIRS);
        let keep_count = KEEP_FINANCE_PAIRS.len();
        // 逐对一条精确三元组 DELETE + GET DIAGNOSTICS 回报行数。不用 IN 批量删、
        // 不按 conflict_type 批量删、不 TRUNCATE：三种写法都会连带删掉财务默认行
        // 与企业自配行，而这些行是本批明确要保留的。
        let deletes = DROP_CONFLICT_TRIPLES
            .iter()
            .map(|(a, b, ctype)| {
                format!(
                    r#"    DELETE FROM "role_conflicts"
      WHERE "role_a_code" = '{a}' AND "role_b_code" = '{b}' AND "conflict_type" = '{ctype}';
    GET DIAGNOSTICS hit = ROW_COUNT;
    RAISE NOTICE 'm_role_conflicts_drop_production_sod：删除内置默认互斥对 {a}<->{b}（conflict_type={ctype}），影响 % 行（0 行=该库本无此行，属正常，重跑等价）', hit;
"#
                )
            })
            .collect::<Vec<String>>()
            .join("\n");

        let up_sql = format!(
            r#"
DO $$
DECLARE
    kept_before INTEGER;
    kept_after INTEGER;
    hit INTEGER;
BEGIN
    SELECT COUNT(*) INTO kept_before FROM "role_conflicts"
      WHERE ("role_a_code", "role_b_code") IN ({keep_pairs});
    RAISE NOTICE 'm_role_conflicts_drop_production_sod：删除前自检——应保留的财务侧 {keep_count} 对内置默认当前在场 % 行（0 行说明该库从未跑过 finance 域种子，属可继续的正常形态）', kept_before;

{deletes}
    SELECT COUNT(*) INTO kept_after FROM "role_conflicts"
      WHERE ("role_a_code", "role_b_code") IN ({keep_pairs});
    IF kept_after <> kept_before THEN
        RAISE EXCEPTION 'm_role_conflicts_drop_production_sod：删除动作误伤保留清单（删前 % 行、删后 % 行）——本域只准删 production_manager<->qc_manager 与 dyeing_master<->quality_inspector 两对', kept_before, kept_after;
    END IF;
    RAISE NOTICE 'm_role_conflicts_drop_production_sod：删除后自检——应保留的财务侧 {keep_count} 对内置默认仍在场 % 行，与删除前一致', kept_after;
END;
$$;
"#
        );

        manager.get_connection().execute_unprepared(&up_sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 回退 = 按原种子取值与列集合逐字插回；created_at/updated_at 由列默认 NOW()
        // 给出，与原种子写法一致。ON CONFLICT DO NOTHING 让重复回退不撞 uniq_role_pair。
        let rows = RESTORE_ROWS
            .iter()
            .map(|(a, b, ctype, desc)| format!("        ('{a}', '{b}', '{ctype}', '{desc}')"))
            .collect::<Vec<String>>()
            .join(",\n");

        let down_sql = format!(
            r#"
DO $$
DECLARE
    hit INTEGER;
BEGIN
    INSERT INTO "role_conflicts" ("role_a_code", "role_b_code", "conflict_type", "description") VALUES
{rows}
    ON CONFLICT ("role_a_code", "role_b_code") DO NOTHING;
    GET DIAGNOSTICS hit = ROW_COUNT;
    RAISE NOTICE 'm_role_conflicts_drop_production_sod down：按原种子插回 2 对生产质量互斥默认，实际插入 % 行（0 行=已存在，重复回退等价）', hit;
END;
$$;
"#
        );

        manager
            .get_connection()
            .execute_unprepared(&down_sql)
            .await?;
        Ok(())
    }
}
