//! `role_conflicts` 内置默认集与互斥机制的**行为级**契约锁（真库通道，非 #[ignore]）
//!
//! 钉三条判据（缺一不可，全部真库回读/真调用，不看"代码里有没有这个函数"）：
//! ① 内置默认**恰好 6 对**且全是财务侧钱账分管组合（整表集合相等，不是包含）；
//! ② 生产↔质量那 2 对（`production_manager↔qc_manager`、`dyeing_master↔quality_inspector`）
//!    **不在**默认集内：同一用户在两岗之间换岗，`check_role_conflict_for_user` 必须放行
//!    （纺织厂一人多岗为常态）；而财务侧保留的默认对仍必须拦——证明本批没有把机制整体放宽；
//! ③ 机制没被改坏的铁证：手工插入一条自定义互斥规则后，同一对角色的换岗必须被**真实拦住**，
//!    错误可判（`AppError::BusinessError` / 机器码 `BUSINESS_ERROR` / 文案点名两个角色码与
//!    规则描述），删掉该规则后同一调用又放行——证明拦与放都由表驱动，不是硬编码。
//!
//! 数据来源口径：`role_conflicts` 属 `services/test_common.rs` 的 `SEALED_REFERENCE_TABLES`
//! （迁移种子参照表，`setup_test_db` **不**清空），所以本文件读到的就是迁移链落下来的
//! 内置默认：建表与 8 对种子在 `migration/src/domain/finance/`，撤除其中 2 对生产质量
//! 默认在链尾域 `migration/src/domain/role_conflicts_drop_production_sod/`。
//! 也正因该表不被清空，本文件自插的自定义规则一律按精确特征自清理，绝不留给下一个用例。
//!
//! 执行位置：CI job `ci-test-rust`（Rust 测试分片，nextest `--partition hash:m/10`，PR 事件
//! 触发；该 job 先把 `backend/migration` 跑完再注入 `TEST_DATABASE_URL`）。同 workflow 的
//! `ci-test-rust-ignored` 只跑 `#[ignore]` 子集，与本文件零重复。
//!
//! 并发口径：本文件用例都读写同一只真库的 `roles`/`users`/`role_conflicts`，用例间用进程内
//! Mutex 串行，避免行数断言被自家另一个用例撞翻。

mod test_common;

use bingxi_backend::models::{role, role_conflict};
use bingxi_backend::services::user_service::UserService;
use bingxi_backend::utils::error::AppError;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait, QueryFilter,
    Statement,
};
use std::sync::Arc;

/// 应保留的 6 对内置默认（财务侧钱账分管），与链尾域 `role_conflicts_drop_production_sod`
/// 的保留清单及 `migration/src/domain/finance/` 的种子逐字同源（此处按字典序书写，
/// 与该表 chk_role_order 约束一致）。
const BUILTIN_FINANCE_PAIRS: [(&str, &str); 6] = [
    ("accountant", "cashier"),
    ("accountant", "finance_manager"),
    ("cashier", "finance_manager"),
    ("cashier", "purchase_clerk"),
    ("cashier", "purchase_manager"),
    ("cashier", "sales_manager"),
];

/// 本批撤掉的 2 对：不得再作为内置默认出现（生产与质量侧不内置硬互斥）。
const DROPPED_PRODUCTION_QC_PAIRS: [(&str, &str); 2] = [
    ("dyeing_master", "quality_inspector"),
    ("production_manager", "qc_manager"),
];

/// 内置默认的冲突类型取值（建表种子与链尾域都按该值写）。
const BUILTIN_CONFLICT_TYPE: &str = "sod";

/// 判据②③用的两个岗（均为角色定义表在册码，见 `init_service_ops/role.rs`）：库里已有则
/// 复用、没有则自建并在用例结束时删除自建行，绝不改动其它用例依赖的种子角色。
const ROLE_DYEING_MASTER: &str = "dyeing_master";
const ROLE_QUALITY_INSPECTOR: &str = "quality_inspector";

/// 判据②的财务侧对照正例用岗（保留默认对的两端）。
const ROLE_CASHIER: &str = "cashier";
const ROLE_ACCOUNTANT: &str = "accountant";

/// 判据③自插的自定义互斥规则描述文案。它同时是"这一行是本文件自插"的唯一特征：
/// 自清理只按它精确删，删不到任何内置种子行（种子描述与此不同）。
const CUSTOM_RULE_DESCRIPTION: &str = "契约锁自配：染色班长与质检员不得兼岗（用例内插入并自清理）";

/// 用例自建的用户账号（`users` 不属参照表，夹具会清空，故用例自行插入并按账号名自清理）。
const TEST_USERNAME: &str = "it_role_conflict_lock";

/// 本文件用例共享的进程内串行锁（同库同行，见文件头并发口径）。
static DB_SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// 执行无返回行的种子/清理 SQL，回影响行数；失败直接点名 panic（绝不静默继续断言）。
async fn exec(db: &DatabaseConnection, sql: &str) -> u64 {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        Vec::<sea_orm::Value>::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("种子/清理 SQL 执行失败: {e}\nSQL: {sql}"))
    .rows_affected()
}

/// 执行 `… RETURNING id` 的种子 SQL，取回新行 id。
async fn exec_returning_id(db: &DatabaseConnection, sql: &str) -> i32 {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            Vec::<sea_orm::Value>::new(),
        ))
        .await
        .unwrap_or_else(|e| panic!("种子 SQL 执行失败: {e}\nSQL: {sql}"));
    let result =
        row.unwrap_or_else(|| panic!("种子 SQL 没有回行（RETURNING id 为空）\nSQL: {sql}"));
    result
        .try_get_by_index::<i32>(0)
        .unwrap_or_else(|e| panic!("种子 SQL 的 id 列解码失败: {e}\nSQL: {sql}"))
}

/// 整表回读互斥规则并按 (a, b, type) 排序，供集合相等断言。
async fn current_rules(db: &DatabaseConnection) -> Vec<(String, String, String)> {
    let mut rows: Vec<(String, String, String)> = role_conflict::Entity::find()
        .all(db)
        .await
        .unwrap_or_else(|e| panic!("回读 role_conflicts 失败: {e}"))
        .into_iter()
        .map(|r| (r.role_a_code, r.role_b_code, r.conflict_type))
        .collect();
    rows.sort();
    rows
}

/// 按角色码取 id；库里没有该岗时自建，并回 `true` 表示"这行是自建的"（用例结束要删）。
/// 自建不写 data_scope（列默认 self，最小权限口径），也不碰权限矩阵。
async fn ensure_role(db: &DatabaseConnection, code: &str) -> (i32, bool) {
    if let Some(existing) = role::Entity::find()
        .filter(role::Column::Code.eq(code))
        .one(db)
        .await
        .unwrap_or_else(|e| panic!("查询角色 {code} 失败: {e}"))
    {
        return (existing.id, false);
    }
    let id = exec_returning_id(
        db,
        &format!(
            "INSERT INTO roles (name, code, description, permissions, is_system) \
             VALUES ('契约锁自建', '{code}', '互斥契约锁用例自建角色', '[]', false) RETURNING id"
        ),
    )
    .await;
    println!("[契约锁] 角色 {code} 库内不存在，已自建 id={id}（用例结束删除该自建行）");
    (id, true)
}

/// 按角色码取"自建就要删"的 id，并把自建的 id 登记进 created_roles。
async fn ensure_role_tracked(db: &DatabaseConnection, code: &str, created: &mut Vec<i32>) -> i32 {
    let (id, was_created) = ensure_role(db, code).await;
    if was_created {
        created.push(id);
    }
    id
}

/// 自建一名挂在 `role_id` 上的用户并回其 id。`users.username` 有唯一约束，故先按账号名
/// 清掉历史残留，保证同一用例内多次分配与跨次重放都可用。
async fn insert_user_with_role(db: &DatabaseConnection, role_id: i32) -> i32 {
    exec(
        db,
        &format!("DELETE FROM users WHERE username = '{TEST_USERNAME}'"),
    )
    .await;
    exec_returning_id(
        db,
        &format!(
            "INSERT INTO users (username, password_hash, role_id) \
             VALUES ('{TEST_USERNAME}', '$argon2id$contract-lock-placeholder', {role_id}) \
             RETURNING id"
        ),
    )
    .await
}

/// 插入一条自定义互斥规则（精确三元组 + 描述）。撞唯一键时影响 0 行，直接点名判负——
/// 说明库里已有一行同键规则不是本文件插的，绝不能退化成"静默复用别人的行"。
async fn insert_custom_rule(db: &DatabaseConnection, a: &str, b: &str) {
    let inserted = exec(
        db,
        &format!(
            "INSERT INTO role_conflicts (role_a_code, role_b_code, conflict_type, description) \
             VALUES ('{a}', '{b}', '{BUILTIN_CONFLICT_TYPE}', '{CUSTOM_RULE_DESCRIPTION}') \
             ON CONFLICT (role_a_code, role_b_code) DO NOTHING"
        ),
    )
    .await;
    assert_eq!(
        inserted, 1,
        "自定义互斥规则必须真实插入 1 行，实得 {inserted} 行：0 行说明同键已有一行不是本文件插的\
         （自清理未完成，或库里存在非内置残留）"
    );
}

/// 按描述文案精确删掉本文件自插的自定义规则，回删除行数。
/// 内置种子行的描述与此不同，删不到，不存在误删内置默认的可能。
async fn delete_custom_rule(db: &DatabaseConnection) -> u64 {
    exec(
        db,
        &format!("DELETE FROM role_conflicts WHERE description = '{CUSTOM_RULE_DESCRIPTION}'"),
    )
    .await
}

/// 清掉本文件自建的行：按账号名删测试用户 + 删除自建的种子外角色（复用到的在册角色不删）。
async fn cleanup(db: &DatabaseConnection, created_roles: &[i32]) {
    exec(
        db,
        &format!("DELETE FROM users WHERE username = '{TEST_USERNAME}'"),
    )
    .await;
    delete_custom_rule(db).await;
    for rid in created_roles {
        exec(db, &format!("DELETE FROM roles WHERE id = {rid}")).await;
    }
}

/// 判据①：内置默认恰好是财务侧 6 对（整表集合相等），生产↔质量 2 对不在集内。
#[tokio::test]
async fn test_builtin_conflicts_are_exactly_six_finance_pairs() {
    let _guard = DB_SERIAL.lock().await;
    let db = test_common::setup_test_db().await;

    // 起点自清：本文件若中途崩过，自插行会留在不被清空的参照表里；先按描述精确删掉它再断
    // 集合相等，才不会被自家残留污染成假红（内置种子行不在此判据内，删不到）。
    let swept = delete_custom_rule(&db).await;
    if swept > 0 {
        println!("[契约锁] 清掉本文件历史残留的自定义互斥规则 {swept} 行，再开始断言");
    }

    let got = current_rules(&db).await;
    let want: Vec<(String, String, String)> = BUILTIN_FINANCE_PAIRS
        .iter()
        .map(|(a, b)| {
            (
                a.to_string(),
                b.to_string(),
                BUILTIN_CONFLICT_TYPE.to_string(),
            )
        })
        .collect();
    assert_eq!(
        got,
        want,
        "role_conflicts 的内置默认必须恰好是财务侧 {} 对钱账分管组合（整表集合相等，多一条=\
         误删或漏删、少一条=种子漂移），实得 {got:?}",
        BUILTIN_FINANCE_PAIRS.len()
    );

    // 显式点名那 2 对已撤除的组合，失败信息直接指向判据而不只给集合差
    for (a, b) in DROPPED_PRODUCTION_QC_PAIRS {
        assert!(
            !got.iter().any(|(x, y, _)| x == a && y == b),
            "内置默认里不得再出现生产↔质量互斥对 {a}<->{b}（一人多岗为纺织厂常态，该对已撤除）；\
             实得 {got:?}"
        );
    }
    println!(
        "[契约锁] 内置默认互斥共 {} 对且全为财务侧，生产质量 2 对不在集内 ✓",
        got.len()
    );
}

/// 判据②：生产↔质量两岗之间换岗放行，而保留的财务侧默认对仍拦。
#[tokio::test]
async fn test_production_qc_role_switch_is_allowed_by_builtin_defaults() {
    let _guard = DB_SERIAL.lock().await;
    let db = test_common::setup_test_db().await;
    delete_custom_rule(&db).await;

    let mut created_roles: Vec<i32> = Vec::new();
    let dyeing_id = ensure_role_tracked(&db, ROLE_DYEING_MASTER, &mut created_roles).await;
    let qc_id = ensure_role_tracked(&db, ROLE_QUALITY_INSPECTOR, &mut created_roles).await;
    let svc = UserService::new(Arc::new(db.clone()));

    // 正向：现挂染色班长 → 换质检员，必须放行
    let user_id = insert_user_with_role(&db, dyeing_id).await;
    svc.check_role_conflict_for_user(user_id, qc_id)
        .await
        .unwrap_or_else(|e| {
            panic!(
                "{ROLE_DYEING_MASTER} -> {ROLE_QUALITY_INSPECTOR} 不该被拦（该互斥对已从内置默认\
                 撤除），实得 {e:?}"
            )
        });

    // 反向：现挂质检员 → 换染色班长，同样放行（互斥与方向无关，双向匹配）
    let user_id = insert_user_with_role(&db, qc_id).await;
    svc.check_role_conflict_for_user(user_id, dyeing_id)
        .await
        .unwrap_or_else(|e| {
            panic!(
                "{ROLE_QUALITY_INSPECTOR} -> {ROLE_DYEING_MASTER} 不该被拦（同一对用户互斥与\
                 方向无关），实得 {e:?}"
            )
        });

    // 对照正例：财务侧保留的默认对必须仍拦——证明本批只撤生产质量，没有把机制整体放宽
    let cashier_id = ensure_role_tracked(&db, ROLE_CASHIER, &mut created_roles).await;
    let accountant_id = ensure_role_tracked(&db, ROLE_ACCOUNTANT, &mut created_roles).await;
    let user_id = insert_user_with_role(&db, cashier_id).await;
    let blocked = svc
        .check_role_conflict_for_user(user_id, accountant_id)
        .await
        .expect_err(
            "cashier<->accountant 仍是内置默认互斥对，换岗必须被拦；放行说明财务侧默认被误删",
        );
    assert_eq!(blocked.error_code(), "BUSINESS_ERROR", "实得 {blocked:?}");

    cleanup(&db, &created_roles).await;
    println!("[契约锁] 生产质量换岗放行、财务侧默认仍拦 ✓");
}

/// 判据③：自配一条互斥规则后写口真的拦住，删掉后又放行（拦与放都由表驱动）。
#[tokio::test]
async fn test_custom_conflict_rule_blocks_then_release() {
    let _guard = DB_SERIAL.lock().await;
    let db = test_common::setup_test_db().await;
    delete_custom_rule(&db).await;

    let mut created_roles: Vec<i32> = Vec::new();
    let dyeing_id = ensure_role_tracked(&db, ROLE_DYEING_MASTER, &mut created_roles).await;
    let qc_id = ensure_role_tracked(&db, ROLE_QUALITY_INSPECTOR, &mut created_roles).await;
    let user_id = insert_user_with_role(&db, dyeing_id).await;
    let svc = UserService::new(Arc::new(db.clone()));

    // 拦之前先确认可过（默认集里没有这一对，与判据②同源）
    svc.check_role_conflict_for_user(user_id, qc_id)
        .await
        .unwrap_or_else(|e| panic!("插入自定义规则前不该被拦，实得 {e:?}"));

    insert_custom_rule(&db, ROLE_DYEING_MASTER, ROLE_QUALITY_INSPECTOR).await;

    let err = match svc.check_role_conflict_for_user(user_id, qc_id).await {
        Ok(()) => panic!(
            "已插入 {ROLE_DYEING_MASTER}<->{ROLE_QUALITY_INSPECTOR} 自定义互斥规则，换岗仍放行\
             ——说明互斥校验被改坏（表不再驱动判定）"
        ),
        Err(e) => e,
    };
    // 可判性：变体 + 机器码 + 文案点名两个角色码与规则描述
    let message = match &err {
        AppError::BusinessError(msg) => msg.clone(),
        other => panic!("互斥拦截必须是业务错误（AppError::BusinessError），实得 {other:?}"),
    };
    assert_eq!(err.error_code(), "BUSINESS_ERROR", "实得 {err:?}");
    for token in [
        "角色互斥冲突",
        ROLE_DYEING_MASTER,
        ROLE_QUALITY_INSPECTOR,
        CUSTOM_RULE_DESCRIPTION,
    ] {
        assert!(
            message.contains(token),
            "拦截文案必须包含 {token}，实得：{message}"
        );
    }
    println!("[契约锁] 自定义互斥规则真实拦住，文案：{message}");

    // 删掉规则后必须立刻放行：证明拦与放都由表驱动，不是写死在代码里
    let removed = delete_custom_rule(&db).await;
    assert_eq!(
        removed, 1,
        "自插的自定义规则应恰好删掉 1 行，实得 {removed} 行"
    );
    svc.check_role_conflict_for_user(user_id, qc_id)
        .await
        .unwrap_or_else(|e| panic!("删掉自定义互斥规则后仍被拦，说明拦截不由表驱动，实得 {e:?}"));

    cleanup(&db, &created_roles).await;
    println!("[契约锁] 互斥机制仍由 role_conflicts 表驱动 ✓");
}
