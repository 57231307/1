//! 契约波次 8 · #259 `customers.customer_type` 渠道词表收口——**活体列形态锁**
//!
//! ## 被测对象（三处交叉，任一漂移本文件即红）
//! 1. **源码权威词表**：`backend/src/constants/customer_type.rs:64`
//!    `ALLOWED = [retail, wholesale, distributor, manufacturer, other]`；同文件
//!    `:60 OTHER = "other"` 是全仓唯一缺省写值（`validate(None) => OTHER`，:91-103），
//!    分层词 `vip`/`normal` 被 `check`（:72-78）逐字符拒绝。
//! 2. **迁移落地**：`backend/migration/src/domain/business/m0078_finalize_customer_type_domain.rs`
//!    ——:73 建备份列 `customer_type_pre_domain_fix VARCHAR(20)`；:81-85 定案回填；
//!    :89-109 残留自证（回填没干净 RAISE EXCEPTION 拒绝进 CHECK）；:112-120
//!    `SET DEFAULT 'other'` / `SET NOT NULL` / `ADD CONSTRAINT chk_customers_customer_type
//!    CHECK (customer_type IN 五值)`；:130-159 回读 information_schema + pg_constraint
//!    （含 `convalidated`）自证。前置只读点名：同目录 `m0077_...rs:51-89`。
//!    注册位置：`business/mod.rs:231-236`（up 链尾，m0077 先于 m0078）。
//! 3. **DB 目录视图**：本文件断言的 `information_schema.columns` / `pg_constraint` /
//!    `pg_get_constraintdef` 就是收口后的**真实生效形态**，不是源码意图的转述。
//!
//! ## 为什么必须真库（sqlite/单元测试夹具测不到的形态）
//! - `is_nullable`、`column_default`、`pg_constraint.convalidated` 是**目录里的状态**，
//!   只有 PostgreSQL 有这些目录；sqlite 夹具既没有 `pg_constraint`，也没有 CHECK 的
//!   `convalidated` 语义（NOT VALID 分支根本无法表达）。
//! - `CHECK` 是否**真的在拒**（写 'vip' 报 SQLSTATE 23514）只有真库能证；应用层
//!   `validate()` 单测只能证源码侧，证不了"库里还留着一条旁路写入的脏行也能过"这件事。
//! - `DEFAULT 'other'` 只在 **INSERT 省略该列**时生效，显式写 NULL 走的是 NOT NULL
//!   （23502）；这两条分支是 PostgreSQL 的语句级行为，ORM 单测与 mock 都构造不出来。
//! - `ADD CONSTRAINT` 与"`finance/mod.rs:537` 那段 `ADD COLUMN IF NOT EXISTS
//!   customer_type VARCHAR(255)` 恒 no-op"（列早已由 system/m0001:332 建成 VARCHAR(20)）
//!   的共存关系，只能靠读回 `character_maximum_length` 与约束存在性来证。
//! - varchar 默认值的文本渲染（`'other'` vs `'other'::character varying`）是 PG 的
//!   `pg_get_expr` 输出，本文件按**允许两种渲染 + 第 4 条用例的行为回读**双重钉死；
//!   这不是放宽：渲染差异只来自是否补写类型标注，二者是同一个表达式的两种文本形态，
//!   取值集合仍是恰好的 2 个（NULL 与任何其它 token 一律拒绝），而"默认值真的求值为
//!   other"这条**语义**由第 4 条用例（省略列→回读等于 `OTHER` 且不等于 `RETAIL`）行为级
//!   证明；目录断言只负责"存在默认值、且默认值是 other 的字面量"。
//!
//! ## 参照表约束与 id 带
//! - `roles`/`departments`/`user_role` 等属**密封参照表**（`src/services/test_common.rs`
//!   的 `SEALED_REFERENCE_TABLES`，不参与逐用例 TRUNCATE）：本文件**不写**这些表，
//!   只引用种子部门 `departments.id=1` 作 users.department_id（与既有 CRM 用例同谱系，
//!   `contract_wave8_crm_read_gate_bypass_test.rs:210-217` 同样写法）。
//! - `customers`/`users`/`crm_lead` 是逐用例清空的业务表，可自由插；但迁移不播种 users，
//!   故 `customers.created_by` 的外键父行必须自建（`models/customer.rs:123-129`
//!   `CreatedByUser` 关系）。本文件独占 **991001..991099** id 带，且每个用例先
//!   `DELETE` 再 `INSERT`（`setup_test_db()` 已 TRUNCATE，这里的先删后插是给"同一用例内
//!   多段复用"和本地 `cargo test` 共享进程场景兜底，保证幂等）。
//! - 迁移里 `customers.id` 是 SERIAL；本文件全部显式指定 id，不依赖自增序列，也不占用
//!   其它用例的小号段。
//!
//! ## 陷阱声明
//! - 夹具 `setup_test_db()` 缺 `TEST_DATABASE_URL` 或指向 sqlite 会直接 panic（不做
//!   静默降级），因此**本文件全部断言都只有 CI 的活库才有结果**；本机无 PostgreSQL，
//!   本机只验证编译与格式，未验证行为——这是显式的"待 CI 活体证明"，不是跳过。
//! - `nextest` 每用例独立进程；本地 `cargo test` 同进程共享连接池：每个用例各自
//!   `setup_test_db()`（先 TRUNCATE 再自建种子），互不依赖顺序，无静态全局状态。
//! - CHECK 违例的错误码必须从 `DbErr::Exec(RuntimeErr::SqlxError)` 里取 SQLSTATE；
//!   只 `assert!(result.is_err())` 是本轮刚清掉的假绿形态（任何 FK/类型/长度错误都会
//!   让它"看起来通过"）。
//! - 每个"必须失败"的用例都配了**同模板合法值对照**，防止夹具本身写坏（列名/外键/
//!   NOT NULL）导致所有拒绝集体假绿。

mod test_common;

use bingxi_backend::constants::customer_type::{ALLOWED, OTHER, RETAIL};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, RuntimeErr, Statement};
use test_common::setup_test_db;

/// 迁移建表时的列宽（system/m0001_initial_schema.rs:332 `VARCHAR(20)`；m0078 未改型）
const COLUMN_VARCHAR_LEN: i64 = 20;
/// m0078:119 固定的约束名（与 constants/customer_type.rs:16-18 注释成文约定一致）
const CHK_NAME: &str = "chk_customers_customer_type";
/// m0078:73 建的备份列（可逆性载体）
const BACKUP_COLUMN: &str = "customer_type_pre_domain_fix";

/// 本文件专属 id 带（991xxx），见文件头「参照表约束与 id 带」
const USER_OWNER: i32 = 991_001;
const USER_CREATOR: i32 = 991_002;
/// CHECK 拒检探针行基址（每 token 一行，全部应当**插不进去**）
const PROBE_BASE: i32 = 991_010;
/// 合法值对照探针行基址（与上面同模板，必须插得进——反假绿对照）
const LEGAL_BASE: i32 = 991_030;
/// DEFAULT 生效探针行
const DEFAULT_PROBE_ID: i32 = 991_050;
/// 显式 NULL 探针行
const NULL_PROBE_ID: i32 = 991_051;
/// 备份列探针行
const BACKUP_PROBE_ID: i32 = 991_060;

/// 分层词、大写混维词与边界值——全部是 #259 判定为非法的写入形态。
/// `POTENTIAL` 属 CLV 分层 segment 词表（constants/customer_type.rs:39-42），
/// `RETAIL`/`OTHER` 是**大写形式**（读侧 `services/customer_ops/crud.rs`、`query.rs`
/// 是小写精确匹配，写大写即永远筛不到；本列 CHECK 逐字符敏感），
/// `''`（空串，边界：空串≠NULL，也是历史旁路写入口的常见产物）与 `'gold'`
/// （长度合法但词表外造词）必须同样被 CHECK 拒。
const BANNED_TOKENS: &[&str] = &["vip", "POTENTIAL", "RETAIL", "OTHER", "normal", "", "gold"];

/// PG 对 varchar 列默认值的两种等价文本渲染（`information_schema.columns.column_default`
/// = `pg_get_expr(adbin, adrelid)`，是否补写 `::character varying` 类型标注由 PG 决定，
/// 语义同一个表达式）。取值集合恰这 2 个，不接受 NULL 或其它 token——理由见文件头。
const DEFAULT_RENDERINGS: [&str; 2] = ["'other'", "'other'::character varying"];

// ---------------------------------------------------------------------------
// 助手（裸 SQL 一律走 execute_raw / query_*_raw，不引入第二套连接方式）
// ---------------------------------------------------------------------------

fn pg_stmt(sql: &str) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, Vec::<sea_orm::Value>::new())
}

async fn exec(db: &DatabaseConnection, sql: &str) {
    db.execute_raw(pg_stmt(sql))
        .await
        .unwrap_or_else(|e| panic!("种子 SQL 执行失败: {e}\nSQL: {sql}"));
}

/// 目录/计数类单列文本读取；查询 0 行返回 `None`（调用方据此断「对象不存在」这一负形态）
async fn one_text(db: &DatabaseConnection, sql: &str, col: &str) -> Option<String> {
    let row = db
        .query_one_raw(pg_stmt(sql))
        .await
        .unwrap_or_else(|e| panic!("单值查询失败: {e}\nSQL: {sql}"));
    row.map(|r| {
        r.try_get::<Option<String>>("", col)
            .unwrap_or_else(|e| panic!("列 {col} 解码为文本失败: {e}\nSQL: {sql}"))
            .unwrap_or_default()
    })
}

async fn one_i64(db: &DatabaseConnection, sql: &str, col: &str) -> i64 {
    let row = db
        .query_one_raw(pg_stmt(sql))
        .await
        .unwrap_or_else(|e| panic!("计数查询失败: {e}\nSQL: {sql}"));
    let r = row.unwrap_or_else(|| panic!("计数查询没有返回行: SQL: {sql}"));
    r.try_get::<i64>("", col)
        .unwrap_or_else(|e| panic!("列 {col} 解码为 i64 失败: {e}\nSQL: {sql}"))
}

/// 驱动上报的 SQLSTATE（从 `DbErr::Exec/Query(RuntimeErr::SqlxError)` 里取，
/// 不做 `to_string().contains(...)` 的含混匹配）
fn sqlstate_of(err: &DbErr) -> Option<String> {
    match err {
        DbErr::Exec(RuntimeErr::SqlxError(e)) | DbErr::Query(RuntimeErr::SqlxError(e)) => e
            .as_database_error()
            .and_then(|de| sqlx::error::DatabaseError::code(de).map(|c| c.into_owned())),
        other => panic!("期望的是执行期数据库错误，实际是非驱动错误: {other}"),
    }
}

/// 驱动上报的约束名（PG 专用）：把 23514 进一步归因到**本列的 CHECK**，
/// 而不是表上任意一条 CHECK
fn constraint_of(err: &DbErr) -> Option<String> {
    match err {
        DbErr::Exec(RuntimeErr::SqlxError(e)) | DbErr::Query(RuntimeErr::SqlxError(e)) => e
            .as_database_error()
            .and_then(|de| sqlx::error::DatabaseError::constraint(de).map(|s| s.to_string())),
        other => panic!("期望的是执行期数据库错误，实际是非驱动错误: {other}"),
    }
}

/// users 自建（`customers.created_by` 的外键父行；users 不在迁移种子里）
async fn seed_users(db: &DatabaseConnection) {
    exec(
        db,
        &format!("DELETE FROM users WHERE id IN ({USER_OWNER},{USER_CREATOR})"),
    )
    .await;
    exec(
        db,
        &format!(
            "INSERT INTO users (id,username,password_hash,is_active,is_totp_enabled,department_id,created_at,updated_at) VALUES
             ({USER_OWNER},'ct_owner','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
             ({USER_CREATOR},'ct_creator','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"
        ),
    )
    .await;
}

/// 带显式 `customer_type` 的客户行写入（返回 `DbErr` 供用例做 SQLSTATE 级断言）。
/// 列清单照抄 `contract_wave8_crm_read_gate_bypass_test.rs:220-227` 已在 CI 跑通的形态。
async fn insert_customer_typed(
    db: &DatabaseConnection,
    id: i32,
    customer_type: &str,
) -> Result<(), DbErr> {
    let values: Vec<sea_orm::Value> = vec![
        id.into(),
        format!("CT-PROBE-{id}").into(),
        USER_OWNER.into(),
        USER_CREATOR.into(),
        customer_type.into(),
    ];
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO customers (id,customer_code,customer_name,credit_limit,payment_terms,status,owner_id,created_by,customer_type,created_at,updated_at)
         VALUES ($1,$2,'渠道词表探针',0,30,'active',$3,$4,$5,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
        values,
    ))
    .await
    .map(|_| ())
}

/// **省略** `customer_type` 列的写入——DEFAULT 唯一能生效的形态
/// （m0078 头注释 :28-30 成文：DEFAULT 只在省略列时生效，显式 NULL 走 NOT NULL）
async fn insert_customer_omitting_type(db: &DatabaseConnection, id: i32) -> Result<(), DbErr> {
    let values: Vec<sea_orm::Value> = vec![
        id.into(),
        format!("CT-DEFAULT-{id}").into(),
        USER_OWNER.into(),
        USER_CREATOR.into(),
    ];
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO customers (id,customer_code,customer_name,credit_limit,payment_terms,status,owner_id,created_by,created_at,updated_at)
         VALUES ($1,$2,'缺省探针',0,30,'active',$3,$4,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
        values,
    ))
    .await
    .map(|_| ())
}

async fn customer_type_of(db: &DatabaseConnection, id: i32) -> Option<String> {
    let row = db
        .query_one_raw(pg_stmt(&format!(
            "SELECT customer_type FROM customers WHERE id={id}"
        )))
        .await
        .unwrap_or_else(|e| panic!("回读 customers.customer_type(id={id}) 失败: {e}"));
    let r = row.unwrap_or_else(|| panic!("customers 里没有 id={id} 的行（探针行未落库？）"));
    r.try_get::<Option<String>>("", "customer_type")
        .unwrap_or_else(|e| panic!("id={id} 的 customer_type 解码失败: {e}"))
}

async fn count_customers(db: &DatabaseConnection, where_sql: &str) -> i64 {
    one_i64(
        db,
        &format!("SELECT COUNT(*) AS n FROM customers WHERE {where_sql}"),
        "n",
    )
    .await
}

/// 五值词表的 SQL 字面量清单**由 `ALLOWED` 生成**（本文件不另立第二套 token 列表，
/// 否则测的就不是 constants 那一份权威词表了）
fn allowed_sql_list() -> String {
    ALLOWED
        .iter()
        .map(|t| format!("'{t}'"))
        .collect::<Vec<_>>()
        .join(",")
}

// ---------------------------------------------------------------------------
// 1. information_schema：NOT NULL + DEFAULT 是 other + 列宽未被「恒 no-op」改掉
// ---------------------------------------------------------------------------

#[tokio::test]
async fn column_form_is_not_null_default_other_and_varchar20() {
    let db = setup_test_db().await;

    let nullable = one_text(
        &db,
        "SELECT is_nullable FROM information_schema.columns
          WHERE table_schema='public' AND table_name='customers' AND column_name='customer_type'",
        "is_nullable",
    )
    .await;
    assert_eq!(
        nullable.as_deref(),
        Some("NO"),
        "customers.customer_type 的 is_nullable 必须是 'NO'（m0078:113 SET NOT NULL 的活体形态）；\n\
         None=列不存在（与 system/m0001:332 建表漂移），其它值=NOT NULL 未真实生效；\n\
         期望: is_nullable='NO'  实际: {nullable:?}"
    );

    let default_raw = one_text(
        &db,
        "SELECT column_default FROM information_schema.columns
          WHERE table_schema='public' AND table_name='customers' AND column_name='customer_type'",
        "column_default",
    )
    .await;
    let rendered = default_raw.as_deref();
    assert!(
        rendered.is_some_and(|d| DEFAULT_RENDERINGS.contains(&d)),
        "customers.customer_type 的 column_default 必须是 other 的字面量渲染——允许带/不带 \
         varchar 类型标注这 2 种 PG 渲染（同一表达式的两种文本形态，理由见文件头，不是放宽）；\n\
         期望: 恰为 {DEFAULT_RENDERINGS:?} 之一  实际: {default_raw:?}\n\
         default=NULL 意味着 m0078:112 的 SET DEFAULT 没生效（省略列的写入会撞 NOT NULL 而死，\
         应用侧缺省失去 DB 兜底）；其它 token 意味着库侧词表与 constants 漂移。"
    );

    let maxlen = one_i64(
        &db,
        "SELECT character_maximum_length::bigint AS len FROM information_schema.columns
          WHERE table_schema='public' AND table_name='customers' AND column_name='customer_type'",
        "len",
    )
    .await;
    assert_eq!(
        maxlen, COLUMN_VARCHAR_LEN,
        "列宽必须仍是 VARCHAR(20)（system/m0001:332 原样）——钉住 `finance/mod.rs:537` 那段 \
         `ADD COLUMN IF NOT EXISTS \"customer_type\" VARCHAR(255)` 恒 no-op（列早已存在）这一事实；\n\
         期望: 20  实际: {maxlen}（255=那段改型真的生效过，NULL=列已不是 varchar 型）"
    );
}

// ---------------------------------------------------------------------------
// 2. pg_constraint：约束在、是 CHECK、且 convalidated=true，值域与 ALLOWED 逐值同源
// ---------------------------------------------------------------------------

#[tokio::test]
async fn check_constraint_exists_is_validated_and_matches_allowed_set() {
    let db = setup_test_db().await;

    let contype = one_text(
        &db,
        &format!(
            "SELECT c.contype::text FROM pg_constraint c
              WHERE c.conrelid='customers'::regclass AND c.conname='{CHK_NAME}'"
        ),
        "contype",
    )
    .await;
    assert_eq!(
        contype.as_deref(),
        Some("c"),
        "pg_constraint 里必须存在名为 {CHK_NAME} 且 contype='c'（CHECK）的约束；\n\
         None=约束不存在（m0078:119 未落地或被 down 撤掉），其它取值=约束类型漂移（如 'f'/'u'）；\n\
         实际 contype: {contype:?}"
    );

    let validated = one_text(
        &db,
        &format!(
            "SELECT c.convalidated::text FROM pg_constraint c
              WHERE c.conrelid='customers'::regclass AND c.conname='{CHK_NAME}'"
        ),
        "convalidated",
    )
    .await;
    assert_eq!(
        validated.as_deref(),
        Some("true"),
        "convalidated 必须为 true：NOT VALID 的 CHECK 会**放行存量脏行**（只对后来的写入生效），\
         那就不是收口——m0078:149-157 的收尾自证正是按这一条拒进约束的；\n\
         期望: 'true'  实际: {validated:?}"
    );

    let def = one_text(
        &db,
        &format!(
            "SELECT pg_get_constraintdef(c.oid) AS def FROM pg_constraint c
              WHERE c.conrelid='customers'::regclass AND c.conname='{CHK_NAME}'"
        ),
        "def",
    )
    .await
    .unwrap_or_else(|| panic!("读不到 {CHK_NAME} 的定义（与上一句 contype 查询自相矛盾）"));
    assert!(
        def.contains("customer_type"),
        "CHECK 必须作用在 customer_type 列上，实际定义: {def}"
    );
    for token in ALLOWED {
        assert!(
            def.contains(&format!("'{token}'")),
            "DB 侧 CHECK 缺少权威词表成员 '{token}'（constants/customer_type.rs:64 与 \
             m0078:49 必须逐值同源，任一侧单独增删即契约漂移）；实际定义: {def}"
        );
    }
    for tier in ["vip", "normal"] {
        assert!(
            !def.contains(&format!("'{tier}'")),
            "分层词 '{tier}' 混进了 DB 侧 CHECK 值域——本列语义是渠道，分层词永不并入\
             （constants/customer_type.rs:10-14）；实际定义: {def}"
        );
    }
    // 反向防「偷偷多塞一个 token」：字面量个数与 ALLOWED 规模严格相等（每个字面量恰 2 个引号）
    let quote_count = def.matches('\'').count();
    assert_eq!(
        quote_count,
        2 * ALLOWED.len(),
        "CHECK 定义里的字符串字面量个数必须恰为 `ALLOWED` 的 {} 个（多一个=在库里造了第二套\
         词表，少一个=值域缺项）；期望引号数 {} 实际引号数 {quote_count}\n实际定义: {def}",
        ALLOWED.len(),
        2 * ALLOWED.len()
    );
}

// ---------------------------------------------------------------------------
// 3. CHECK 真的在拒：分层词 / 大写混维 / 空串 / 造词 → SQLSTATE 23514，且零落库
// ---------------------------------------------------------------------------

#[tokio::test]
async fn check_constraint_actually_rejects_out_of_domain_tokens() {
    let db = setup_test_db().await;
    seed_users(&db).await;

    for (idx, token) in BANNED_TOKENS.iter().enumerate() {
        let id = PROBE_BASE + idx as i32;
        let err = insert_customer_typed(&db, id, token)
            .await
            .err()
            .unwrap_or_else(|| panic!("customer_type='{token}' 竟被写入成功（CHECK 未生效）"));
        let code = sqlstate_of(&err);
        assert_eq!(
            code.as_deref(),
            Some("23514"),
            "写 customer_type='{token}' 必须被 CHECK 拒成 check_violation（SQLSTATE 23514）；\n\
             期望: 23514  实际: {code:?}\n原始错误: {err}\n\
             只断 is_err() 不算收口：FK/NOT NULL/长度截断也都是 Err，会把「CHECK 已失效」看成绿。"
        );
        let constraint = constraint_of(&err);
        assert_eq!(
            constraint.as_deref(),
            Some(CHK_NAME),
            "23514 必须归因到 {CHK_NAME} 本约束（表上若还有别的 CHECK，只断状态码会把\
             「别的约束在挡」误当词表收口）；实际 constraint: {constraint:?}"
        );
        let left = count_customers(&db, &format!("id={id}")).await;
        assert_eq!(
            left, 0,
            "被 CHECK 拒掉的 customer_type='{token}' 必须零落库（单语句原子失败，不留半行）"
        );
    }

    // 反假绿对照：**同一列清单/同一模板**写合法值必须插得进。
    // 若夹具本身写坏（列名/外键/NOT NULL），上面一串「必失败」会集体假绿，此处即对照。
    for (idx, token) in ALLOWED.iter().enumerate() {
        let id = LEGAL_BASE + idx as i32;
        insert_customer_typed(&db, id, token)
            .await
            .unwrap_or_else(|e| panic!("合法渠道值 '{token}' 必须能落库（实际失败: {e}）"));
        let got = customer_type_of(&db, id).await;
        assert_eq!(
            got.as_deref(),
            Some(*token),
            "customer_type 必须**原文**存回 '{token}'（不 trim、不做大小写归一，与 \
             constants/customer_type.rs:85 的原文返回口径一致）；实际回读: {got:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// 4. DEFAULT 真的生效：省略列 → 回读 'other'（同时钉「缺省不猜 retail」）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn default_fills_other_when_column_is_omitted() {
    let db = setup_test_db().await;
    seed_users(&db).await;

    insert_customer_omitting_type(&db, DEFAULT_PROBE_ID)
        .await
        .unwrap_or_else(|e| panic!("省略 customer_type 列的 INSERT 应成功（DEFAULT 顶上）: {e}"));
    let got = customer_type_of(&db, DEFAULT_PROBE_ID).await;
    assert_eq!(
        got.as_deref(),
        Some(OTHER),
        "省略列的行必须落成 '{OTHER}'（m0078:112 `SET DEFAULT 'other'` 的行为级证明；\
         缺省语义=渠道未知）；期望: '{OTHER}'  实际: {got:?}"
    );
    assert_ne!(
        got.as_deref().unwrap_or_default(),
        RETAIL,
        "缺省**不得**猜成 '{RETAIL}'——那是替业务方做了一个可能为假的断言，会污染渠道维度的\
         全部下游筛选与统计（选型依据 constants/customer_type.rs:53-60；线索转化入口同理，\
         `services/crm/lead.rs` 的缺省分支也写 OTHER）"
    );

    let retail_defaulted = count_customers(
        &db,
        &format!("customer_code LIKE 'CT-DEFAULT-%' AND customer_type='{RETAIL}'"),
    )
    .await;
    assert_eq!(
        retail_defaulted, 0,
        "所有走 DEFAULT 的行都没有落成 retail（按缺省探针行前缀统计）；实际: {retail_defaulted} 行"
    );
}

// ---------------------------------------------------------------------------
// 5. NOT NULL 真的在拒：显式 NULL → SQLSTATE 23502（DEFAULT 不是 NULL 的免死金牌）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn not_null_rejects_explicit_null_with_sqlstate_23502() {
    let db = setup_test_db().await;
    seed_users(&db).await;

    let err = db
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO customers (id,customer_code,customer_name,credit_limit,payment_terms,status,owner_id,created_by,customer_type,created_at,updated_at)
             VALUES ($1,$2,'显式 NULL 探针',0,30,'active',$3,$4,NULL,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
            vec![
                sea_orm::Value::from(NULL_PROBE_ID),
                sea_orm::Value::from(format!("CT-NULL-{NULL_PROBE_ID}")),
                sea_orm::Value::from(USER_OWNER),
                sea_orm::Value::from(USER_CREATOR),
            ],
        ))
        .await
        .err()
        .unwrap_or_else(|| {
            panic!(
                "显式写 NULL 竟被接受（NOT NULL 未生效：m0078 定案的「NULL 不再是合法存储形态」失守，\
                 实体/DTO 侧就仍可按 Option 缺省写 NULL）"
            )
        });
    let code = sqlstate_of(&err);
    assert_eq!(
        code.as_deref(),
        Some("23502"),
        "显式 NULL 必须撞 NOT NULL（SQLSTATE 23502 not_null_violation），而不是 23514：\n\
         DEFAULT 只在**省略列**时生效（m0078:28-30 成文），显式 NULL 是另一条必须被堵住的旁路；\n\
         期望: 23502  实际: {code:?}\n原始错误: {err}"
    );
    let left = count_customers(&db, &format!("id={NULL_PROBE_ID}")).await;
    assert_eq!(
        left, 0,
        "被 NOT NULL 拒掉的行必须零落库；实际残留 {left} 行"
    );
}

// ---------------------------------------------------------------------------
// 6. 备份列：可逆性的活体证据（存在、可空、且不被 CHECK 覆盖）
//
// 为什么这里**不**主张「备份列逐行等于回填后的值域」：CI 的活库里迁移先跑完、夹具随后
// 把 customers TRUNCATE（`src/services/test_common.rs::reset_business_tables`），本文件
// 自插的行从未经历过 m0078:81-85 的回填映射，其备份列按构造必为 NULL（m0078 down 的
// :209-210 注释成文：up 之后新插入的行备份列为 NULL，语义与「收口前该列可空」一致）。
// 把「逐行相等」写成断言会是假前提下的恒真/恒假绿。可逆性真正需要钉的是**载体形态**：
// 列在、可空、且不被本列的 CHECK 波及——下面用「把收口前的原值形态 'vip' 写进备份列」
// 这条行为路径把它钉死（写不进=快照无处存放=down 不可逆）。
// ---------------------------------------------------------------------------

#[tokio::test]
async fn backup_column_exists_and_stays_outside_check_constraint() {
    let db = setup_test_db().await;
    seed_users(&db).await;

    let exists = one_text(
        &db,
        &format!(
            "SELECT column_name FROM information_schema.columns
              WHERE table_schema='public' AND table_name='customers' AND column_name='{BACKUP_COLUMN}'"
        ),
        "column_name",
    )
    .await;
    assert_eq!(
        exists.as_deref(),
        Some(BACKUP_COLUMN),
        "备份列 {BACKUP_COLUMN} 必须在（m0078:73 建列）：它是 down 迁移真实还原原值的载体\
         （m0078:216 `UPDATE ... SET customer_type = customer_type_pre_domain_fix`），\
         缺列时 down 只能 NOTICE 认栽、不可逆；实际: {exists:?}"
    );

    let backup_nullable = one_text(
        &db,
        &format!(
            "SELECT is_nullable FROM information_schema.columns
              WHERE table_schema='public' AND table_name='customers' AND column_name='{BACKUP_COLUMN}'"
        ),
        "is_nullable",
    )
    .await;
    assert_eq!(
        backup_nullable.as_deref(),
        Some("YES"),
        "备份列必须保持可空（m0078:73 建的是 VARCHAR(20)、无 NOT NULL）：收口后新插入的行\
         没有「收口前的原值」，强塞 NOT NULL 会让 DEFAULT 路径与备份语义互相打架；实际: {backup_nullable:?}"
    );

    // 行为级证明「CHECK 只作用于 customer_type」：把收口前的脏形态 'vip' 写进备份列必须成功。
    // 这就是可逆性的实质证据——若约束误覆盖备份列，down 还原时必炸（原值无处存放）。
    let sql = format!(
        "INSERT INTO customers (id,customer_code,customer_name,credit_limit,payment_terms,status,owner_id,created_by,customer_type,\"{BACKUP_COLUMN}\",created_at,updated_at)
         VALUES ($1,$2,'备份列探针',0,30,'active',$3,$4,$5,$6,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"
    );
    let values: Vec<sea_orm::Value> = vec![
        BACKUP_PROBE_ID.into(),
        format!("CT-BACKUP-{BACKUP_PROBE_ID}").into(),
        USER_OWNER.into(),
        USER_CREATOR.into(),
        OTHER.into(),
        "vip".into(),
    ];
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        &sql,
        values,
    ))
    .await
    .unwrap_or_else(|e| {
        panic!(
            "往备份列写收口前的原值形态 'vip' 必须成功（CHECK 不该覆盖备份列，否则快照\
                 无处可存、down 不可逆）；实际错误: {e}"
        )
    });
    let row = db
        .query_one_raw(pg_stmt(&format!(
            "SELECT \"{BACKUP_COLUMN}\" AS snap FROM customers WHERE id={BACKUP_PROBE_ID}"
        )))
        .await
        .unwrap_or_else(|e| panic!("回读备份列失败: {e}"));
    let snap = row
        .unwrap_or_else(|| panic!("备份列探针行未落库（id={BACKUP_PROBE_ID}）"))
        .try_get::<Option<String>>("", "snap")
        .unwrap_or_else(|e| panic!("备份列解码失败: {e}"));
    assert_eq!(
        snap.as_deref(),
        Some("vip"),
        "备份列必须原样存下 'vip'（快照无损才谈得上还原）；实际: {snap:?}"
    );
}

// ---------------------------------------------------------------------------
// 7. 负向前置：表内不得存在五值外的残留行（m0078「回填没干净不许进 CHECK」的活体回声）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn no_residual_row_outside_allowed_domain_in_live_table() {
    let db = setup_test_db().await;
    seed_users(&db).await;

    // 先铺满五个合法值 + 一条缺省行，让这条断言有真实分母，而不是在空表上恒真。
    for (idx, token) in ALLOWED.iter().enumerate() {
        insert_customer_typed(&db, LEGAL_BASE + idx as i32, token)
            .await
            .unwrap_or_else(|e| panic!("种合法行失败（token='{token}'）: {e}"));
    }
    insert_customer_omitting_type(&db, DEFAULT_PROBE_ID)
        .await
        .unwrap_or_else(|e| panic!("种缺省行失败: {e}"));

    let residual = one_i64(
        &db,
        &format!(
            "SELECT COUNT(*) AS n FROM customers WHERE customer_type IS NULL OR customer_type NOT IN ({})",
            allowed_sql_list()
        ),
        "n",
    )
    .await;
    assert_eq!(
        residual, 0,
        "活体库里 customer_type 落在五值词表之外（或 NULL）的行必须为 0——这正是 m0078:89-109\
         「回填没干净就不许进 CHECK」那条自证的活体回声（约束存在且 convalidated=true 时，\
         残留只能来自约束加入前未被回填干净的存量脏行）；实际残留: {residual} 行"
    );

    let total = one_i64(&db, "SELECT COUNT(*) AS n FROM customers", "n").await;
    assert_eq!(
        total,
        (ALLOWED.len() + 1) as i64,
        "本用例自建行数应为 5 个显式合法值 + 1 条缺省行（分母核对，防空表恒真）；实际: {total} 行"
    );
}
