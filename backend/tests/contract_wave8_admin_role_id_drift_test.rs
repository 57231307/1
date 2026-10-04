//! 契约波次 8 · admin 判定唯一权威源「行为级活体锁」（461efcd9 收口的真正验收点）
//!
//! 被测对象：`bingxi_backend::utils::admin_checker::is_admin_role(&DatabaseConnection, i32)`
//! —— 判据必须是 `roles.code = 'admin'`（`ADMIN_ROLE_CODE` 全等），**与主键无关**；
//! 查不到 / 出错一律 fail-closed=false（admin_checker.rs:86-102）。
//! 本波把 11 处 `role_id != 1` 字面量换成该函数；角色表主键由序列生成、按名称播种
//! （m0001_initial_schema.rs:609-612 播种不带 id），所以字面量回潮的两类病必须用
//! 活体数据双向打出：
//! - admin 行落在 id≠1 → 判 true（否则静默剔 admin 字段 = 功能坏）；
//! - 非 admin 行恰好坐在 id=1 → 判 false（否则 `role_id == 1` 静默扩权 = 越权）。
//! 另两条：code 貌似 admin（super_admin / 本仓真实存在的 admin_assistant，历史上
//! e2e 曾因兜底到它被误当 admin）必须 false；不存在的 role_id 必须 false 不 panic。
//!
//! ## 为什么场景 1/2 建在「私有 schema 的 roles 副本表」上（而非直接插 public.roles）
//! public.roles 是密封参照表（test_common.rs:39-59，不参与逐用例 TRUNCATE），且
//! `code` 列带 UNIQUE（m0001_initial_schema.rs:23-27）、主键 id=1 被种子 admin 永久
//! 占据（m0001:609-612 首行 admin，新表自增从 1 起）—— ⇒ 在 public 上
//! ①再插一条 code='admin' 与 UNIQUE 冲突（同批的
//! contract_wave8_crm_read_gate_bypass_test.rs 原先正是这么写的，:684-696 现已改为
//! **不再**自造第二条 admin、并把"admin 落在 id≠1"的活体证明让给本文件——依据正是这条
//! UNIQUE；本文件也不跟随"往 public 插第二条 admin"的写法）；②"非 admin 坐 id=1"与种子行
//! PK 冲突。两条都只有临时改名/挪位种子行才能凑出来，而那会污染全部依赖种子的
//! 用例（密封表红线）。私有 schema 副本表（列集 = models/role.rs:8-20 实体列）+
//! 专用连接池 `set_schema_search_path`（sea-orm 2.0.2 能力，池内每条物理连接建连时
//! 执行 SET，无会话粘滞问题）在不触碰 public 任何表/结构/序列的前提下把两条字面
//! 场景原样构造出来。custom 池上先探针 `current_schema()` + 私有表行数，防止
//! search_path 未生效时"悄悄查回 public 还宣称测了漂移场景"的假绿。
//!
//! ## 缓存陷阱与 role_id 唯一性（admin_checker.rs:41-79，TTL 5min，键=role_id）
//! nextest 每用例独立进程；本地 cargo test 多线程共用 ADMIN_ROLE_CACHE。为两种
//! 跑法都零顺序依赖：全文件 7 个缓存键互不相同，且每断言前 clear_admin_role_cache
//! 清键、断言后再清（防止测得值经缓存外泄给同进程其他用例）。键表：
//! - 990101（t1 admin@id≠1）、990102（t1 对照）、1 与 990103（t2 专用，id=1 全文件
//!   仅 t2 触碰一次）、990301/990302（t3）、max(id)+886422（t4，运行时探测必为
//!   新键；t1-t3 只用 ≤990302 与 1，public 种子/其他测试只用 id≤99，不互撞）。
//! 高段 990xxx 为本文件专属带（仓内既有测试仅用到 97/98/99，见
//! contract_wave8_crm_read_gate_bypass_test.rs:585-590、:685-692 的删插法）。
//!
//! ## 夹具与范式出处（照抄实际编译通过的写法）
//! - `setup_test_db()`：src/services/test_common.rs:193（返回 `DatabaseConnection`，
//!   直接 `&db` 喂 is_admin_role，无需 `.inner` 取法）；tests 侧 shim
//!   tests/test_common/mod.rs:18-20 + `mod test_common;`（同
//!   contract_wave8_crm_read_gate_bypass_test.rs:40）。缺 TEST_DATABASE_URL / 指
//!   sqlite 直接 panic（test_common.rs:72-88），禁 AppState::default()——
//!   is_admin_role 对 Disconnected 恒 false（admin_checker.rs:63-65），用之全假绿。
//! - 裸 SQL exec 助手 + `Vec::<sea_orm::Value>::new()`：抄
//!   contract_wave8_crm_read_gate_bypass_test.rs:130-138；显式 id 插 roles 的列形
//!   状抄 :587-588（created_at/updated_at 用 '…Z' 字面量，绕开 sea-orm Value 变体
//!   改名坑——本文件全程不构造 sea_orm::Value）。
//! - 专用池（ConnectOptions::new + 按 URL 连库）：抄 tests/rls_context_test.rs:270-274。
//! - 密封表自造行「先删后插」幂等：同 read_gate_bypass :581-590。

mod test_common;

use bingxi_backend::utils::admin_checker::{clear_admin_role_cache, is_admin_role};
use sea_orm::{ConnectOptions, ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use test_common::setup_test_db;

// ---------------------------------------------------------------------------
// 助手
// ---------------------------------------------------------------------------

async fn exec(db: &DatabaseConnection, sql: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        Vec::<sea_orm::Value>::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("种子/清理执行失败: {e}\nSQL: {sql}"));
}

async fn scalar_i64(db: &DatabaseConnection, sql: &str) -> i64 {
    let row = db
        .query_one_raw(Statement::from_string(DbBackend::Postgres, sql.to_string()))
        .await
        .unwrap_or_else(|e| panic!("标量查询失败: {e}\nSQL: {sql}"))
        .unwrap_or_else(|| panic!("标量查询无行: SQL: {sql}"));
    row.try_get::<i64>("", "v")
        .unwrap_or_else(|e| panic!("标量列应可解码为 i64: {e}\nSQL: {sql}"))
}

async fn current_schema(db: &DatabaseConnection) -> String {
    let row = db
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT current_schema()::text AS s".to_string(),
        ))
        .await
        .expect("current_schema() 探针查询失败")
        .expect("current_schema() 必有行");
    row.try_get::<String>("", "s")
        .expect("current_schema() 应为文本")
}

/// 在私有 schema 建一张列集与 `role::Model`（models/role.rs:8-20，含迁移后
/// data_scope 列，finance/mod.rs:23）逐一对应的 roles 副本表并插入给定行。
/// schema 名与行值全部是本文件内常量拼接（无外部输入，不存在注入口径）。
async fn spawn_drift_roles(base: &DatabaseConnection, schema: &str, value_rows: &str) {
    exec(base, &format!("DROP SCHEMA IF EXISTS {schema} CASCADE")).await;
    exec(base, &format!("CREATE SCHEMA {schema}")).await;
    exec(
        base,
        &format!(
            "CREATE TABLE {schema}.roles (
                id INTEGER NOT NULL PRIMARY KEY,
                name VARCHAR(100) NOT NULL,
                code VARCHAR(50) NOT NULL,
                description TEXT,
                permissions TEXT,
                is_system BOOLEAN NOT NULL DEFAULT FALSE,
                data_scope VARCHAR(10) NOT NULL DEFAULT 'self',
                created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
            )"
        ),
    )
    .await;
    exec(
        base,
        &format!(
            "INSERT INTO {schema}.roles (id,name,code,is_system,data_scope,created_at,updated_at) VALUES {value_rows}"
        ),
    )
    .await;
}

/// 指向私有 schema 的专用连接池：sea-orm 在每条物理连接建立时执行
/// `SET search_path = "{schema}","public"`（sqlx_postgres after_connect），
/// 之后 `role::Entity::find_by_id` 的无 schema 限定 `"roles"` 即解析到副本表。
async fn drift_conn(schema: &str) -> DatabaseConnection {
    let url = std::env::var("TEST_DATABASE_URL")
        .expect("setup_test_db 已强制要求 TEST_DATABASE_URL，此处缺失即夹具契约被破坏");
    let mut opts = ConnectOptions::new(url);
    opts.set_schema_search_path(format!("{schema},public"));
    sea_orm::Database::connect(opts)
        .await
        .expect("专用 search_path 池连接 PostgreSQL 失败")
}

/// 探针：确认这个池真的在查私有副本表而不是悄悄回落到 public（若回落，
/// current_schema 与行数都会不吻合——防"漂移场景其实没被构造"的假绿）。
async fn assert_resolved_to_drift_schema(db: &DatabaseConnection, schema: &str, expect_rows: i64) {
    let cur = current_schema(db).await;
    assert_eq!(
        cur.as_str(),
        schema,
        "专用池 search_path 未生效（current_schema={cur}，期望 {schema}）——本用例将查的是 \
         public.roles，漂移场景根本没构造出来，属假绿前提，必须红"
    );
    // 若在 public 会看到迁移种子行（admin/manager/operator ≥3 行）⇒ 探针失效。
    let n = scalar_i64(db, "SELECT COUNT(*)::BIGINT AS v FROM roles").await;
    assert_eq!(
        n, expect_rows,
        "roles 实际按 search_path 解析到的表应恰为私有副本（{expect_rows} 行）；\
         若在 public 会看到迁移种子行 ⇒ 探针失效"
    );
}

// ---------------------------------------------------------------------------
// 用例 1：code='admin' 的行坐在 id≠1 —— 必须判 admin（防主键字面量回潮剔 admin）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn w8_drift_admin_role_code_at_non_one_id_is_true() {
    const SCHEMA: &str = "contract_w8_drift_adm_ne1";
    const ADMIN_ID: i32 = 990_101; // 私有副本内 code='admin' 的显式非 1 主键
    const CTL_ID: i32 = 990_102; // 同副本内 code≠'admin' 对照行

    clear_admin_role_cache(Some(ADMIN_ID));
    clear_admin_role_cache(Some(CTL_ID));
    let db = setup_test_db().await;
    spawn_drift_roles(
        &db,
        SCHEMA,
        "(990101,'W8漂移Admin','admin',FALSE,'all','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (990102,'W8漂移对照','w8_drift_ctl',FALSE,'dept','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;
    let cdb = drift_conn(SCHEMA).await;
    assert_resolved_to_drift_schema(&cdb, SCHEMA, 2).await;

    let admin_hit = is_admin_role(&cdb, ADMIN_ID).await;
    let ctl_hit = is_admin_role(&cdb, CTL_ID).await;

    exec(&db, &format!("DROP SCHEMA IF EXISTS {SCHEMA} CASCADE")).await;
    clear_admin_role_cache(Some(ADMIN_ID));
    clear_admin_role_cache(Some(CTL_ID));

    assert!(
        admin_hit,
        "code='admin' 且 id=990101（≠1）被判非 admin —— 这是在防主键字面量回潮：\
         若判定实现重新掺入 id==1，角色播种漂移时 admin 会被静默剔除字段（功能坏）"
    );
    assert!(
        !ctl_hit,
        "对照组失守：同副本内 code='w8_drift_ctl' 的行被判 admin —— 判据已不是 \
         roles.code='admin' 全等，属主键/其他字面量回潮"
    );
}

// ---------------------------------------------------------------------------
// 用例 2：非 admin 行恰好坐在 id=1 —— 必须判非 admin（防 role_id==1 静默扩权）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn w8_drift_nonadmin_row_at_id_one_is_false() {
    const SCHEMA: &str = "contract_w8_drift_nonadm1";
    // 私有副本里"恰好播种到 1"的非 admin 行：public 的 id=1 是种子 admin，
    // 密封表动不得 ⇒ 场景只能在副本上原样构造；缓存键 1 全文件仅本用例触碰一次。
    const DRIFT_ONE: i32 = 1;
    const ADMIN_ID: i32 = 990_103; // 同刻对照：code='admin' 在 id≠1 必须 true

    clear_admin_role_cache(Some(DRIFT_ONE));
    clear_admin_role_cache(Some(ADMIN_ID));
    let db = setup_test_db().await;
    spawn_drift_roles(
        &db,
        SCHEMA,
        "(1,'W8旁观者','w8_id1_bystander',FALSE,'all','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (990103,'W8漂移Admin','admin',FALSE,'all','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;
    let cdb = drift_conn(SCHEMA).await;
    assert_resolved_to_drift_schema(&cdb, SCHEMA, 2).await;

    let one_hit = is_admin_role(&cdb, DRIFT_ONE).await;
    let admin_hit = is_admin_role(&cdb, ADMIN_ID).await;

    exec(&db, &format!("DROP SCHEMA IF EXISTS {SCHEMA} CASCADE")).await;
    clear_admin_role_cache(Some(DRIFT_ONE));
    clear_admin_role_cache(Some(ADMIN_ID));

    assert!(
        !one_hit,
        "id=1 但 code≠'admin' 的行被判 admin —— 这是在防主键字面量回潮：`role_id == 1` \
         式判定会给任何恰好坐在主键 1 上的非 admin 角色静默扩权（成本/金额列越权可见）"
    );
    assert!(
        admin_hit,
        "同刻对照失守：副本内 code='admin'（id=990103）未被判 admin —— 判据必须且只能 \
         是 roles.code，与主键位置无关"
    );
}

// ---------------------------------------------------------------------------
// 用例 3：名字"像 admin"≠权威 code —— 必须 false（public 真表，专属高段 id）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn w8_drift_admin_lookalike_codes_are_false() {
    const SUPER_ID: i32 = 990_301;
    const ASSISTANT_ID: i32 = 990_302;

    clear_admin_role_cache(Some(SUPER_ID));
    clear_admin_role_cache(Some(ASSISTANT_ID));
    let db = setup_test_db().await;
    // 先删后插保幂等（同 wave8 对自造 role 行的既有纪律）；is_system=TRUE +
    // data_scope='all' 是给"按系统标志/范围兜底判 admin"的假实现准备的陷阱位。
    exec(
        &db,
        "DELETE FROM roles WHERE id IN (990301,990302) AND code IN ('super_admin','admin_assistant')",
    )
    .await;
    exec(
        &db,
        "INSERT INTO roles (id,name,code,is_system,data_scope,created_at,updated_at) VALUES
         (990301,'W8伪超管','super_admin',TRUE,'all','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (990302,'W8行政助理','admin_assistant',TRUE,'all','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;

    let super_hit = is_admin_role(&db, SUPER_ID).await;
    let assistant_hit = is_admin_role(&db, ASSISTANT_ID).await;

    exec(
        &db,
        "DELETE FROM roles WHERE id IN (990301,990302) AND code IN ('super_admin','admin_assistant')",
    )
    .await;
    clear_admin_role_cache(Some(SUPER_ID));
    clear_admin_role_cache(Some(ASSISTANT_ID));

    assert!(
        !super_hit,
        "code='super_admin' 被判 admin —— 权威源是 ADMIN_ROLE_CODE 的**全等**比较，\
         任何'包含/前缀 admin'式模糊匹配都是扩权（本用例即在 public 真表上钉死）"
    );
    assert!(
        !assistant_hit,
        "code='admin_assistant' 被判 admin —— 这是本仓真实存在的角色码（\
         services/init_service_ops/role.rs:290），历史上 e2e 曾因兜底到它被误当 admin；\
         is_system=TRUE 也不得改变判定（唯一判据 = code 全等 'admin'）"
    );
}

// ---------------------------------------------------------------------------
// 用例 4：查不存在的 role_id —— fail-closed=false，不 panic（admin_checker.rs:88）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn w8_drift_missing_role_id_is_fail_closed_false() {
    let db = setup_test_db().await;
    // 从 public.roles 当前 max(id) 往上取一段专属偏移，保证是**从未存在过**的键
    // （缓存必为 miss 分支），且天然不与任何既有/并发插入的 id 相撞。
    let max_id = scalar_i64(
        &db,
        "SELECT (COALESCE(MAX(id),0))::BIGINT AS v FROM public.roles",
    )
    .await;
    let probe_id_64 = max_id + 886_422;
    assert!(
        probe_id_64 <= i32::MAX as i64,
        "探测 id 溢出 i32（max(id)={max_id} 异常大？）——夹具前提被破坏"
    );
    let probe_id = probe_id_64 as i32;
    assert!(
        probe_id != 1 && ![990_101, 990_102, 990_103, 990_301, 990_302].contains(&probe_id),
        "缓存键唯一性守卫：探测 id（{probe_id}）不得与本文件其他用例的键相同，\
         更不得等于 1（id=1 由用例 2 专属表达越权场景）"
    );
    clear_admin_role_cache(Some(probe_id));

    let first = is_admin_role(&db, probe_id).await; // DB 查询 miss 分支
    let second = is_admin_role(&db, probe_id).await; // 缓存命中分支（同值才算锁）

    clear_admin_role_cache(Some(probe_id));

    assert!(
        !first,
        "不存在的 role_id（{probe_id}）未 fail-closed —— admin_checker.rs:88 规定 \
         Ok(None)=>false；若返回 true 等于给任意未分配主键发管理员，这是主键兜底回潮\
         的镜像形态（函数正常返回本身即证明无 panic）"
    );
    assert!(
        !second,
        "同一未命中键第二次调用翻转了判定 —— 缓存层（ADMIN_ROLE_CACHE）把 fail-closed \
         结果改写或旁路了，属权威源旁路"
    );
}
