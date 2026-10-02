//! 测试公共夹具模块（P0-D11 → #4669 路线一改造）
//!
//! ## 为什么必须改（实证，非推测）
//! CI #4669 的 215 例 Rust 失败里约 130 例的签名是
//! `mismatched types; Rust type Option<f64> is not compatible with SQL type TEXT`、
//! `Sqlite doesn't support array arguments`、`Type("String unsupported by sqlx-sqlite")`——
//! 全部来自**测试自建 DDL 在 sqlite 上的亲和性失真**（测试把 DECIMAL 列写成 `TEXT`，
//! sqlite 的 TEXT 亲和把绑定值转存文本，读回按 REAL/Decimal 解码必炸），与业务源码无关。
//! 另一族（约 45 例）断言的是"无 schema 时应返回数据库错误"，前提同样是
//! "测试连的是空 sqlite 内存库"。
//!
//! 旧实现 `TEST_DATABASE_URL` 缺失时**静默回退 sqlite::memory:**，于是
//! "分片 job 起了 PostgreSQL 并跑完迁移"这件事对这些用例毫无作用：它们从未验证过
//! 生产方言，报告页却显示"通过"。这就是本仓反复禁止的假绿形态。
//!
//! ## 新契约
//! - [`setup_test_db`]：**必须**由 `TEST_DATABASE_URL` 指向已跑完迁移的 PostgreSQL；
//!   未设置或指向 sqlite 一律 panic（fail-loudly，不再静默降级）。随后把 public 下的
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

use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, QueryResult, Statement};

/// 迁移种子写入、测试不得清空的参照表。
/// 逐条来自 `grep -rhoiE 'INSERT INTO [^ (;]+' backend/migration/src/`。
/// 注意 `users` / `permissions` **不在**其中：迁移不播种，用例必须自己插。
pub const SEALED_REFERENCE_TABLES: &[&str] = &[
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

/// 迁移记账表前缀（同样不参与清空，清掉会让后续 migrate 判定错乱）
const MIGRATION_BOOKKEEPING_PREFIXES: &[&str] = &["_seaorm_", "seaorm_"];

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
pub async fn connect_live_db() -> DatabaseConnection {
    Database::connect(live_database_url())
        .await
        .expect("测试夹具：PostgreSQL 连接失败（TEST_DATABASE_URL 指向的库不可达？）")
}

/// 表名白名单：只接受迁移产出的小写标识符，杜绝把任何外部串拼进 TRUNCATE。
fn is_safe_ident(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 63
        && name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn col_string(row: &QueryResult, idx: usize) -> String {
    row.try_get_by_index::<Option<String>>(idx)
        .unwrap_or_else(|e| panic!("夹具：pg_tables.tablename 应可解码为文本，第 {idx} 列: {e}"))
        .unwrap_or_default()
}

/// public schema 下、排除参照种子与迁移记账后的全部业务表名。
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
                && !MIGRATION_BOOKKEEPING_PREFIXES
                    .iter()
                    .any(|p| name.starts_with(p))
        })
        .collect()
}

/// 把业务表清到空：`RESTART IDENTITY` 让自增 ID 从 1 起，`CASCADE` 处理外键依赖。
///
/// 分块执行只为控制单条 SQL 长度，语义等价于一次全表 TRUNCATE。
pub async fn reset_business_tables(db: &DatabaseConnection) {
    let tables = business_tables(db).await;
    if tables.is_empty() {
        panic!("测试夹具：public schema 下没有可清空的业务表 —— 迁移未生效，拒绝以空库跑测试");
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
pub async fn connect_empty_schema_db() -> DatabaseConnection {
    let db = Database::connect(empty_database_url())
        .await
        .expect("测试夹具：空 schema PostgreSQL 连接失败（TEST_EMPTY_DATABASE_URL）");
    let tables = business_tables(&db).await;
    if !tables.is_empty() {
        panic!(
            "TEST_EMPTY_DATABASE_URL 指向的库里已有 {} 张业务表（首张 {}）——该库必须是不跑迁移的空库",
            tables.len(),
            tables[0]
        );
    }
    db
}
