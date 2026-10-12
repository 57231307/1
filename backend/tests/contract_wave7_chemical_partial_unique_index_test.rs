//! 化学品主数据三表「部分唯一索引」迁移形态契约锁（Wave-I §④ 收口批次）
//!
//! 纯迁移文本比对，不连库（索引是否真能建成由迁移 up 的 fail-closed 存量探测
//! 与活库迁移测试裁决；本锁防的是"约束形态漂移"本身）：
//! ① 三条 `CREATE UNIQUE INDEX ... WHERE "is_deleted" = false` 部分索引形态逐字钉死
//!    （全表 UNIQUE 会破坏"软删后同码可复用"既定语义，禁止回潮）；
//! ② 每索引前置只读重复探测 + RAISE EXCEPTION fail-closed（禁止静默去重/洗数据，
//!    照 m0063/m0064/m0067 先例）；迁移文本内不得出现任何 UPDATE/DELETE 数据操纵；
//! ③ down 显式三条 `DROP INDEX IF EXISTS`（本仓纪律：down 不留空实现）；
//! ④ 注册在场且顺序正确：production 域 `pub(crate) mod` 声明 + v15 域末段
//!    up（晚于 m0065，故必须晚于全部建表）/ down（先于 m0065，与 up 对称）。
//!
//! 同源依据：`services/chemical_ops/{category,master,lot}.rs` 判重仅查
//! `IsDeleted.eq(false)` 行；后端 INSERT 已带 `UniqueConstraintViolation` 竞态兜底，
//! 其生效前提即本锁钉住的三条索引。

const IDX_CATEGORY: &str = "CREATE UNIQUE INDEX IF NOT EXISTS \"uk_chemical_category_code_active\"\n    ON \"chemical_category\" (\"category_code\") WHERE \"is_deleted\" = false;";
const IDX_MASTER: &str = "CREATE UNIQUE INDEX IF NOT EXISTS \"uk_chemical_master_code_active\"\n    ON \"chemical_master\" (\"chemical_code\") WHERE \"is_deleted\" = false;";
const IDX_LOT: &str = "CREATE UNIQUE INDEX IF NOT EXISTS \"uk_chemical_lot_no_active\"\n    ON \"chemical_lot\" (\"lot_no\") WHERE \"is_deleted\" = false;";

fn mig_src() -> String {
    include_str!(
        "../migration/src/domain/production/m0068_add_chemical_code_partial_unique_constraints.rs"
    )
    .replace('\r', "")
}

#[test]
fn chemical_partial_unique_indexes_present_with_where_clause() {
    let src = mig_src();
    // 三条部分索引逐字在场（含 WHERE 谓词，索引名与后端 Wave-I §④ 建议同源）
    for needle in [IDX_CATEGORY, IDX_MASTER, IDX_LOT] {
        assert!(src.contains(needle), "部分唯一索引形态缺失:\n{needle}");
    }
    // 带谓词收尾的 CREATE UNIQUE INDEX 恰为 3 条——防被"顺手"扩成无谓词全表 UNIQUE
    assert_eq!(
        src.matches("WHERE \"is_deleted\" = false;").count(),
        3,
        "以 WHERE \"is_deleted\" = false; 收尾的唯一索引语句应恰为 3 条"
    );
    // 禁止对本域三表新增全表唯一约束/外键式约束形态
    for banned in ["ADD CONSTRAINT", "UNIQUE ("] {
        assert!(
            !src.contains(banned),
            "迁移不得引入 {banned} 形态（全表 UNIQUE 会打红软删复用码断言）"
        );
    }
}

#[test]
fn duplicate_probe_is_fail_closed_and_wash_free() {
    let src = mig_src();
    // 只统计 up 执行体（头注释也含 RAISE/探测等描述文字，按整文件计数会误伤）
    let up_start = src.find("async fn up").expect("m0068 缺少 up 实现");
    let up_end = src.find("async fn down").expect("m0068 缺少 down 实现");
    let up = &src[up_start..up_end];
    // 三表各有前置探测：只读统计 + HAVING 判重 + RAISE EXCEPTION 中止
    assert_eq!(
        up.matches("HAVING COUNT(*) > 1").count(),
        6,
        "每表探测应含统计与样例两处 HAVING，共 6 处"
    );
    assert_eq!(
        up.matches("RAISE EXCEPTION").count(),
        3,
        "每表应恰有一条 fail-closed RAISE EXCEPTION"
    );
    // 探测口径与索引谓词同源：仅查未删行（每表统计+样例两处，共 6 处）
    assert_eq!(
        up.matches("WHERE \"is_deleted\" = false\n").count(),
        6,
        "探测查询必须带 WHERE \"is_deleted\" = false（与判重/索引同口径）"
    );
    // 迁移绝不洗数据：全文件无任何 UPDATE/DELETE 数据操纵语句（RAISE 消息中的建议文字除外）
    for banned in ["UPDATE \"", "DELETE FROM", "SET \"is_deleted\"", "RENAME"] {
        assert!(!src.contains(banned), "迁移文本出现数据操纵形态: {banned}");
    }
}

#[test]
fn down_drops_all_three_indexes_explicitly() {
    let src = mig_src();
    // down 段应显式回滚三条索引（本仓已登记"down 空实现"教训），且不触碰数据行
    let down_start = src.find("async fn down").expect("m0068 缺少 down 实现");
    let down = &src[down_start..];
    for name in [
        "DROP INDEX IF EXISTS \"uk_chemical_category_code_active\";",
        "DROP INDEX IF EXISTS \"uk_chemical_master_code_active\";",
        "DROP INDEX IF EXISTS \"uk_chemical_lot_no_active\";",
    ] {
        assert!(down.contains(name), "down 缺少显式回滚: {name}");
    }
    assert_eq!(
        down.matches("execute_unprepared").count(),
        1,
        "down 应恰有一次执行"
    );
    assert!(!down.contains("DELETE FROM"), "down 不得删数据行");
}

#[test]
fn registration_in_production_and_v15_tail_locked() {
    // 编排注册锁：文件留 production 域但由 v15 末段调用（m0058/m0063/m0065 先例），
    // 任一处漏合并本用例即红，防"迁移写了但从未执行"的静默失效。
    let prod_mod = include_str!("../migration/src/domain/production/mod.rs").replace('\r', "");
    assert!(
        prod_mod.contains("pub(crate) mod m0068_add_chemical_code_partial_unique_constraints;"),
        "production/mod.rs 缺少 pub(crate) mod m0068 声明"
    );
    let v15 = include_str!("../migration/src/domain/v15/mod.rs").replace('\r', "");
    let up_call =
        "crate::domain::production::m0068_add_chemical_code_partial_unique_constraints::Migration";
    let n = v15.matches(up_call).count();
    assert_eq!(n, 2, "v15/mod.rs 应有 up/down 两处 m0068 调用，实测 {n} 处");
    // 顺序锁：up 中 m0068 晚于 m0065（建表/回填全部完成后才建索引）；
    // down 中 m0068 先于 m0065（与 up 对称，最后应用者最先回滚）。
    let m65 = "m0065_backfill_outsourcing_reserved_pieces::Migration";
    let first_68 = v15.find(up_call).expect("m0068 调用缺失");
    let second_68 = v15[first_68 + 1..]
        .find(up_call)
        .map(|i| first_68 + 1 + i)
        .expect("m0068 down 调用缺失");
    let first_65 = v15.find(m65).expect("m0065 调用缺失");
    let second_65 = v15[first_65 + 1..]
        .find(m65)
        .map(|i| first_65 + 1 + i)
        .expect("m0065 down 调用缺失");
    assert!(first_68 > first_65, "up 段 m0068 必须晚于 m0065");
    assert!(
        second_68 < second_65,
        "down 段 m0068 必须早于 m0065（对称回滚）"
    );
}
