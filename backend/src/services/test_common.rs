//! 测试公共夹具模块
//!
//! ## 为什么禁止 sqlite
//! 测试自建 DDL 在 sqlite 上存在**亲和性失真**：DECIMAL 列写成 `TEXT` 时，
//! sqlite 的 TEXT 亲和把绑定值转存文本，读回按 REAL/Decimal 解码必炸
//! （`mismatched types; Rust type Option<f64> is not compatible with SQL type TEXT`、
//! `Sqlite doesn't support array arguments`、`Type("String unsupported by sqlx-sqlite")`），
//! 与业务源码无关；断言"无 schema 时应返回数据库错误"的负前提交集同样需要
//! 空 schema 的真实 PostgreSQL。
//! 若 `TEST_DATABASE_URL` 缺失时静默回退 sqlite::memory:，用例从未验证过
//! 生产方言、报告页却显示"通过"——即本仓禁止的假绿形态，故夹具一律直接 panic。
//!
//! ## 契约
//! - [`setup_test_db`]：**必须**由 `TEST_DATABASE_URL` 指向已跑完迁移的 PostgreSQL；
//!   未设置或指向 sqlite 一律 panic（fail-loudly，绝不静默降级）。随后把 public 下的
//!   业务表清空（`TRUNCATE … RESTART IDENTITY CASCADE`），使每个用例都从
//!   "已迁移的空库 + 迁移种子主数据"起步：既有确定性（自增 ID 从 1 起、互不串库），
//!   又不必由各测试自建一份与生产不同构的表。
//! - [`connect_empty_schema_db`]：需要"库存在但 schema 为空"这一负前提交集的用例，
//!   连的是 CI 里**不跑迁移**的第二只库（`TEST_EMPTY_DATABASE_URL`）；未注入同样 panic，
//!   不允许静默跳过。
//! - 迁移种子写入的参照表（见 [`SEALED_REFERENCE_TABLES`]）**不参与清空**：它们由迁移
//!   本身播种（roles/role_permissions/departments/account_subjects/dye_batch_state_rule/
//!   crm_recycle_rules/供应商目录等），清掉等于把全部依赖种子主数据的用例一起打死。
//!   需要"干净参照行"的用例请自建专属行并按 ID 断言。
//! - 迁移台账 `seaql_migrations`（及一切 `seaql_*`/`seaorm_*` 记账表）**永不清空**：
//!   台账被抹掉会让 `Migrator::up` 判定"从未迁移"并重放 `m0001`，"迁移已应用/初始化链"
//!   类断言全部在空台账上假跑。夹具自带双向防线：清表前守卫（交集必须为空）与
//!   清表后反向自检（台账记账行数必须 > 0），任一失守直接点名 panic。

use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, QueryResult, Statement};

/// 迁移种子写入、测试不得清空的参照表。
/// 逐条来自 `grep -rhoiE 'INSERT INTO [^ (;]+' backend/migration/src/`。
/// 注意 `users` / `permissions` **不在**其中：迁移不播种，用例必须自己插。
#[allow(
    dead_code,
    reason = "警告仅由 server bin 目标报出（src/main.rs:12 以 `mod services;` 镜像整棵 lib 模块树，夹具链在 bin crate 内无任何调用方；lib 与集成测试编译均不报，日志 target name=server/kind=bin 可证）。本清单由同文件 business_tables 消费，是 TRUNCATE 排除种子参照表的判据；口径引用见 tests/contract_wave5_voucher_budget_family_test.rs:113、tests/contract_wave8_crm_read_gate_bypass_test.rs:36"
)]
const SEALED_REFERENCE_TABLES: &[&str] = &[
    "account_subjects",
    "budget_item_periods",
    "budget_plans",
    "business_mode_config",
    "collection_templates",
    "crm_recycle_rules",
    "crm_tag",
    "departments",
    "dye_batch_state_rule",
    "failover_status",
    "material_shortage_threshold_configs",
    "role_conflicts",
    "role_permissions",
    "role_relations",
    "roles",
    "supplier_product_colors",
    "supplier_products",
    "suppliers",
    "user_role",
];

/// 迁移记账表前缀（同样不参与清空，清掉会让后续 migrate 判定错乱）。
/// `seaql_` 是 sea-orm 2.x 真实台账 `seaql_migrations` 的名字前缀（CI 判责取证：
/// 旧集合只有 `_seaorm_`/`seaorm_`，与真实台账名不匹配，夹具曾逐用例把迁移历史
/// 一起 TRUNCATE，后续 `Migrator::up` 判定"从未迁移"并重放 `m0001`）；
/// `_seaorm_`/`seaorm_` 在当前 sea-orm 版本无实体表命中，保留仅作台账被
/// 覆写改名时的防线。
#[allow(
    dead_code,
    reason = "仅 server bin 镜像目标报出（src/main.rs:12 `mod services;` 把夹具编进 bin，链上无 bin 调用路径）；消费者为同文件 is_migration_ledger，是排除 seaql_/seaorm 记账表名的前缀判据"
)]
const MIGRATION_BOOKKEEPING_PREFIXES: &[&str] = &["_seaorm_", "seaorm_", "seaql_"];

/// sea-orm 迁移台账表的**精确表名**——绝不允许进入 TRUNCATE 清单，且清表后必须仍可
/// 读出非空记账行（见 `reset_business_tables` 的清表前守卫与清表后反向自检）。
/// 表名依据：sea-orm 2.x 未覆写 `MigratorTrait::migration_table_name` 时台账固定为
/// `seaql_migrations`；全仓 grep `migration_table_name` 零命中（无覆写、无第二套台账/
/// 影子台账），本仓自身也按该名直读——`src/bootstrap/service_bootstrap.rs:278`
/// `SELECT COUNT(*) as cnt FROM seaql_migrations`。
#[allow(
    dead_code,
    reason = "仅 server bin 镜像目标报出（src/main.rs:12 `mod services;`）；消费者为同文件 is_migration_ledger 与 reset_business_tables 的台账守卫/自检"
)]
const MIGRATION_LEDGER_TABLES: &[&str] = &["seaql_migrations"];

/// 台账判定的唯一口径：精确表名或记账前缀任一命中即视为迁移台账。
/// 清表排除集、清表前守卫、清表后反向自检三处共用本判据（同源防漂移）——排除集
/// 若漏认一张台账表，"迁移已应用/初始化链"类真库断言就会在空台账上静默空转。
#[allow(
    dead_code,
    reason = "仅 server bin 镜像目标报出（src/main.rs:12 `mod services;`）；消费者为同文件 business_tables 与 reset_business_tables"
)]
fn is_migration_ledger(name: &str) -> bool {
    MIGRATION_LEDGER_TABLES.contains(&name)
        || MIGRATION_BOOKKEEPING_PREFIXES
            .iter()
            .any(|p| name.starts_with(p))
}

#[allow(
    dead_code,
    reason = "仅 server bin 镜像目标报出（src/main.rs:12 `mod services;`）；消费者为同文件 connect_live_db——真实使用点 tests/bi_analysis_test.rs:26、tests/contract_wave2_explicit_null_clear_test.rs:107、tests/number_generator_document_no_taken_registry_test.rs:251"
)]
fn live_database_url() -> String {
    let url = std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
        panic!(
            "集成测试必须连真实 PostgreSQL：未设置 TEST_DATABASE_URL。\n\
             本仓集成测试禁止 sqlite::memory: 回退——sqlite 的 TEXT/数组亲和会把测试自建的假表\n\
             变成永久方言失真（CI #4669 的约 130 例解码红正源于此）。\n\
             本地开发请指向已跑迁移的库：export TEST_DATABASE_URL=postgres://user:pass@localhost:5432/bingxi_test"
        )
    });
    if url.starts_with("sqlite") {
        panic!(
            "TEST_DATABASE_URL 指向 sqlite（{url}）：夹具拒绝以失真方言验证生产 SQL，\
             请指向已跑完迁移的 PostgreSQL"
        );
    }
    url
}

#[allow(
    dead_code,
    reason = "仅 server bin 镜像目标报出（src/main.rs:12 `mod services;`）；消费者为同文件 connect_empty_schema_db——真实使用点 tests/ap_payment_workflow_test.rs:90、tests/services_voucher_service_test.rs:516 等 30+ 处负前提交集用例"
)]
fn empty_database_url() -> String {
    std::env::var("TEST_EMPTY_DATABASE_URL").unwrap_or_else(|_| {
        panic!(
            "活库负前提交集必须连『已建库但未跑迁移』的第二只 PostgreSQL：未设置 \
             TEST_EMPTY_DATABASE_URL。CI 的两个 Rust 测试 job 各建一只 bingxi_empty；\
             本地请对应建一只空库并注入该变量。禁止回退 sqlite 或静默跳过——那会把\
             『schema 缺失必须报 DATABASE_ERROR』的锁变成假绿/假跑。"
        )
    })
}

/// 只连接、不清空（同一用例内需要保留前序写入、或多段共用一库时使用）。
#[allow(
    dead_code,
    reason = "仅 server bin 镜像目标报出（src/main.rs:12 `mod services;`，夹具非 bin 生产路径）；真实使用点 tests/bi_analysis_test.rs:26、tests/contract_wave2_explicit_null_clear_test.rs:107、tests/number_generator_document_no_taken_registry_test.rs:251/274，另被同文件 setup_test_db 调用"
)]
pub async fn connect_live_db() -> DatabaseConnection {
    Database::connect(live_database_url())
        .await
        .expect("测试夹具：PostgreSQL 连接失败（TEST_DATABASE_URL 指向的库不可达？）")
}

/// 表名白名单：只接受迁移产出的小写标识符，杜绝把任何外部串拼进 TRUNCATE。
#[allow(
    dead_code,
    reason = "仅 server bin 镜像目标报出（src/main.rs:12 `mod services;`）；消费者为同文件 business_tables，是 TRUNCATE 拼接前的标识符白名单校验"
)]
fn is_safe_ident(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 63
        && name.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

#[allow(
    dead_code,
    reason = "仅 server bin 镜像目标报出（src/main.rs:12 `mod services;`）；消费者为同文件 business_tables，用于解码 pg_tables.tablename 列"
)]
fn col_string(row: &QueryResult, idx: usize) -> String {
    row.try_get_by_index::<Option<String>>(idx)
        .unwrap_or_else(|e| panic!("夹具：pg_tables.tablename 应可解码为文本，第 {idx} 列: {e}"))
        .unwrap_or_default()
}

/// public schema 下、排除参照种子与迁移记账后的全部业务表名。
#[allow(
    dead_code,
    reason = "仅 server bin 镜像目标报出（src/main.rs:12 `mod services;`）；消费者为同文件 reset_business_tables，属 setup_test_db 夹具链（集成测试普遍使用，如 tests/ap_payment_workflow_test.rs:75）"
)]
async fn business_tables(db: &DatabaseConnection) -> Vec<String> {
    let rows: Vec<QueryResult> = db
        .query_all_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT tablename FROM pg_catalog.pg_tables \
             WHERE schemaname = 'public' ORDER BY tablename"
                .to_string(),
        ))
        .await
        .expect("测试夹具：读取 public 表清单失败（连接不是 PostgreSQL？）");
    rows.into_iter()
        .map(|row| col_string(&row, 0))
        .filter(|name| {
            is_safe_ident(name)
                && !SEALED_REFERENCE_TABLES.contains(&name.as_str())
                && !is_migration_ledger(name)
        })
        .collect()
}

/// 把业务表清到空：`RESTART IDENTITY` 让自增 ID 从 1 起，`CASCADE` 处理外键依赖。
///
/// 分块执行只为控制单条 SQL 长度，语义等价于一次全表 TRUNCATE。
#[allow(
    dead_code,
    reason = "仅 server bin 镜像目标报出（src/main.rs:12 `mod services;`）；消费者为同文件 setup_test_db（tests/ 集成测试普遍经 shim 使用），tests/ 侧无直接调用方故不再是 pub"
)]
async fn reset_business_tables(db: &DatabaseConnection) {
    let tables = business_tables(db).await;
    if tables.is_empty() {
        panic!("测试夹具：public schema 下没有可清空的业务表 —— 迁移未生效，拒绝以空库跑测试");
    }
    // 清表前守卫：待清集合与台账集合的交集必须为空（判据与排除集同源 is_migration_ledger）。
    // 未来若有新增/改名台账绕过排除集，这里必须点名炸，绝不允许台账被静默清掉。
    if let Some(hit) = tables.iter().find(|t| is_migration_ledger(t)) {
        panic!(
            "测试夹具守卫：TRUNCATE 清单混入迁移台账表 {hit}——清台账会让后续 Migrator::up \
             重放 m0001，全部“迁移已应用”类断言在空台账上假跑；排除判据见 is_migration_ledger"
        );
    }
    for chunk in tables.chunks(80) {
        let list = chunk
            .iter()
            .map(|t| format!("\"{t}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!("TRUNCATE TABLE {list} RESTART IDENTITY CASCADE");
        db.execute_raw(Statement::from_string(DbBackend::Postgres, sql))
            .await
            .expect("测试夹具：清空业务表失败（TRUNCATE）");
    }
    // 清表后反向自检：台账表必须仍可读出非空记账行。台账被任何路径抹掉时在此点名
    // fail（绝不静默）——否则"初始化链/幂等重放"类断言会在空台账上假跑，红也红得没有指向。
    for ledger in MIGRATION_LEDGER_TABLES {
        let stmt = Statement::from_string(
            DbBackend::Postgres,
            format!("SELECT COUNT(*) AS cnt FROM \"{ledger}\""),
        );
        let row = db.query_one_raw(stmt).await.unwrap_or_else(|e| {
            panic!(
                "测试夹具反向自检：迁移台账 {ledger} 不可读（{e}）——该库不是经迁移记账建起来的，\
                     清空业务表后任何“已迁移”前提都不可信"
            )
        });
        let count = row
            .and_then(|r| r.try_get_by_index::<i64>(0).ok())
            .unwrap_or_else(|| panic!("测试夹具反向自检：迁移台账 {ledger} 记账行数解码失败"));
        if count == 0 {
            panic!(
                "测试夹具反向自检：清空业务表后迁移台账 {ledger} 记账行数为 0——台账已被抹掉，\
                 后续 Migrator::up 会重放 m0001，初始化/幂等类断言全部空转"
            );
        }
    }
}

/// 集成测试唯一入口：连接已迁移的 PostgreSQL 并清空业务表（保留迁移种子参照表）。
pub async fn setup_test_db() -> DatabaseConnection {
    let db = connect_live_db().await;
    reset_business_tables(&db).await;
    db
}

/// 负前提交集专用：『库存在但没有任何业务表』的 PostgreSQL。
///
/// 用例以此验证 schema 缺失时服务报 `DATABASE_ERROR`（而不是被 `map_err(internal)`
/// 拍平成 500，或被静默吞成空结果）。该库由 CI 单独创建、**不跑迁移**，因此不会与
/// [`setup_test_db`] 的已迁移库互相污染。落地时反向校验它确实是空库——若里面已经有
/// 业务表，说明 env 指错了库，那条锁验证的就是假前提。
#[allow(
    dead_code,
    reason = "仅 server bin 镜像目标报出（src/main.rs:12 `mod services;`，夹具非 bin 生产路径）；真实使用点 tests/ap_payment_workflow_test.rs:90、tests/services_inventory_stock_service_test.rs:207、tests/services_voucher_service_test.rs:516 等 30+ 处空 schema 负前提交集用例"
)]
pub async fn connect_empty_schema_db() -> DatabaseConnection {
    let db = Database::connect(empty_database_url())
        .await
        .expect("测试夹具：空 schema PostgreSQL 连接失败（TEST_EMPTY_DATABASE_URL）");
    // 判据是"迁移从未在此库跑过"，而不是"库里一张表都没有"：本库正是给负前提交集与
    // 迁移 SQL 语义锁当沙箱用的，用例会在其中建自己的临时表；用"0 表"当断言会让
    // 同库的第二个沙箱用例必然 panic（顺序耦合的假红）。sales_orders 是迁移建的表，
    // 它不存在 == 迁移没跑过，这才是本库存在的意义。
    let migrated = db
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT to_regclass('public.sales_orders')::text AS t".to_string(),
        ))
        .await
        .ok()
        .flatten()
        .and_then(|r| r.try_get::<Option<String>>("", "t").ok().flatten());
    if migrated.is_some() {
        panic!(
            "TEST_EMPTY_DATABASE_URL 指向的库里已存在 sales_orders（{migrated:?}）——\
             该库必须是不跑迁移的库，否则\"无 schema 应报 DATABASE_ERROR\"与迁移 SQL 语义锁验证的是假前提"
        );
    }
    db
}
