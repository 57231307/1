//! customers.tier 分层词表——**真库活体契约锁**(词表==CHECK==应用层三向集合相等、
//! NULL=未定档形态、旁路越界拒写、二级审批判据唯一来源)
//!
//! ## 被测对象(三处真源,任一漂移本文件即红)
//! 1. 迁移原文:`migration/src/domain/business/m0083_add_customer_tier_column.rs` 的
//!    `const ALLOWED_SQL`(CHECK 取值唯一种子,经 `allowed = ALLOWED_SQL` 填进
//!    `CHECK ("tier" IS NULL OR "tier" IN ({allowed}))`),注册在 business up 链尾。
//! 2. 应用权威词表:`src/constants/customer_tier.rs` 的 `ALLOWED` 与高档集合 `MAJOR`
//!    (全仓唯一出现点;大客户二级审批判据 `customer_tier::is_major` 只引此处)。
//! 3. 活库目录:`pg_constraint`(contype='c'、convalidated、pg_get_constraintdef)与
//!    `information_schema.columns`(is_nullable、column_default 缺席)。
//!
//! ## 为什么必须真库(sqlite/纸面比对测不到的形态)
//! - `convalidated`、`pg_get_constraintdef`、列无 DEFAULT 是 PostgreSQL 目录状态;
//! - "CHECK 是否真的在拒"只能靠真库旁路裸 SQL 直插取证(SQLSTATE 23514 + 约束名归因);
//! - 夹具 `setup_test_db()` 缺 `TEST_DATABASE_URL` 或指向 sqlite 直接 panic,不静默降级。
//!
//! ## 判据唯一性扫描(防第二套口径回潮)
//! 审批服务源码必须只经 `customer_tier::is_major` 判大客户,且**不得**再出现
//! `estimated_amount`、`500_000`、信用/金额阈值常量名——终裁前的临时口径若被改回,
//! 这里红(行为面另有可达性锁的"高金额未挂客户⇒非大客户"一例对撞)。
//!
//! ## 期望值来源(防恒真、防两侧一起改错)
//! 合法集合一律取运行时 `customer_tier::ALLOWED` 与迁移原文解析、活库目录读数三方
//! 对撞,测试内不手抄第二套取值清单;`MAJOR` 成员边界(VIP/GOLD 入集、SILVER/NORMAL
//! 不入集)点名断言;MAJOR 对 ALLOWED 的严格子集关系 + 对活库值域的包含关系钉同源。
//!
//! ## id 带与种子
//! `customers`/`users` 属逐用例清理的业务表;本文件独占 **990_7xx** 段(990_1xx、
//! 990_3xx、990_6xx、991_0xx、992_0xx、993xxx 已被其它锁占用,不碰)。
//! 列清单照已在 CI 跑通的既有直插模板,仅追加 tier 参数。

mod test_common;

use bingxi_backend::constants::customer_tier::{ALLOWED, GOLD, MAJOR, NORMAL, SILVER, VIP};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, RuntimeErr, Statement};
use std::collections::BTreeSet;
use test_common::setup_test_db;

/// m0083 添加的 CHECK 约束名(与 `constants/customer_tier.rs` 头注释成文约定同名)
const CHK_NAME: &str = "chk_customers_tier";
const M0083_REL: &str = "migration/src/domain/business/m0083_add_customer_tier_column.rs";
const SERVICE_REL: &str = "src/services/crm/customer_transfer_approval_service.rs";
const EVALUATE_REL: &str = "src/services/customer_credit_evaluate.rs";

const OPERATOR_ID: i32 = 990_701;
const PROBE_BASE: i32 = 990_720; // 旁路越界探针(每 token 一行,全部应当插不进去)
const PROBE_LEGAL_BASE: i32 = 990_740; // 旁路合法值探针(四档 + 显式 NULL)
const PROBE_OMIT_ID: i32 = 990_750; // 省略 tier 列的直插(NULL=未定档且无 DEFAULT)

/// 库层旁路越界探针:小写漂移(词表逐字符敏感,与价格阶梯大写词同形不同case 即脏值)、
/// 词表外新档、尾随空白混入、空串。
const BYPASS_BANNED_TOKENS: &[&str] = &["silver", "vip", "normal", "DIAMOND", "VIP ", ""];

// ---------------------------------------------------------------------------
// 助手(照既有词表活体锁模板,裸 SQL 一律 execute_raw / query_one_raw)
// ---------------------------------------------------------------------------

fn read_repo_file(rel: &str) -> String {
    let path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {path} 失败: {e}"))
}

fn pg_stmt(sql: &str) -> Statement {
    Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql.to_string(),
        Vec::<sea_orm::Value>::new(),
    )
}

async fn exec(db: &DatabaseConnection, sql: &str) {
    db.execute_raw(pg_stmt(sql))
        .await
        .unwrap_or_else(|e| panic!("种子 SQL 执行失败: {e}\nSQL: {sql}"));
}

/// 目录/计数类单列文本读取;查询 0 行返回 `None`(调用方据此断"对象不存在"负形态)。
/// 列值为真 NULL 时显式抛错而不是塌成空串。
async fn one_text(db: &DatabaseConnection, sql: &str, col: &str) -> Option<String> {
    let row = db
        .query_one_raw(pg_stmt(sql))
        .await
        .unwrap_or_else(|e| panic!("单值查询失败: {e}\nSQL: {sql}"));
    row.map(|r| {
        r.try_get::<Option<String>>("", col)
            .unwrap_or_else(|e| panic!("列 {col} 解码为文本失败: {e}\nSQL: {sql}"))
            .unwrap_or_else(|| panic!("列 {col} 是真 NULL(本助手不塌缩)\nSQL: {sql}"))
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

/// 驱动上报的 SQLSTATE(不做 to_string().contains 含混匹配)。
fn sqlstate_of(err: &DbErr) -> Option<String> {
    match err {
        DbErr::Exec(RuntimeErr::SqlxError(e)) | DbErr::Query(RuntimeErr::SqlxError(e)) => e
            .as_database_error()
            .and_then(|de| sqlx::error::DatabaseError::code(de).map(|c| c.into_owned())),
        other => panic!("期望的是执行期数据库错误,实际是非驱动错误: {other}"),
    }
}

/// 驱动上报的约束名(PG 专用):把 23514 归因到本列这一条 CHECK。
fn constraint_of(err: &DbErr) -> Option<String> {
    match err {
        DbErr::Exec(RuntimeErr::SqlxError(e)) | DbErr::Query(RuntimeErr::SqlxError(e)) => e
            .as_database_error()
            .and_then(|de| sqlx::error::DatabaseError::constraint(de).map(|s| s.to_string())),
        other => panic!("期望的是执行期数据库错误,实际是非驱动错误: {other}"),
    }
}

/// 提取文本中全部单引号 token;引号不闭合直接 panic(解析面失守必须炸)。
fn quoted_tokens(text: &str) -> BTreeSet<String> {
    let mut tokens = BTreeSet::new();
    let mut cursor = text;
    while let Some(open) = cursor.find('\'') {
        cursor = &cursor[open + 1..];
        let close = cursor
            .find('\'')
            .unwrap_or_else(|| panic!("文本里的单引号未闭合: {text}"));
        tokens.insert(cursor[..close].to_string());
        cursor = &cursor[close + 1..];
    }
    tokens
}

/// 从迁移原文解析 `const ALLOWED_SQL` 字面量段的取值集合(锚点缺失即 panic)。
fn parse_allowed_sql_tokens(src: &str, file: &str) -> BTreeSet<String> {
    let anchor = "const ALLOWED_SQL: &str = \"";
    let start = src
        .find(anchor)
        .unwrap_or_else(|| panic!("{file} 缺少锚点 {anchor:?}(CHECK 取值种子形态已改动)"));
    let rest = &src[start + anchor.len()..];
    let end = rest
        .find("\";")
        .unwrap_or_else(|| panic!("{file} 的 ALLOWED_SQL 字面量未闭合"));
    let tokens = quoted_tokens(&rest[..end]);
    assert!(
        !tokens.is_empty(),
        "{file} 解析出空取值集合——解析失守,拒绝继续比对"
    );
    tokens
}

/// users 自建(customers.created_by 的外键父行;department_id=1 引用迁移种子部门)。
async fn seed_users(db: &DatabaseConnection) {
    exec(db, &format!("DELETE FROM users WHERE id={OPERATOR_ID}")).await;
    exec(
        db,
        &format!(
            "INSERT INTO users (id,username,password_hash,is_active,is_totp_enabled,department_id,created_at,updated_at)
             VALUES ({OPERATOR_ID},'tier_w11_lock_op','test-only-not-a-real-hash',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"
        ),
    )
    .await;
}

/// 带显式 `tier`(或 NULL)的旁路直插(绕开全部应用校验,模拟 CHECK 要拦的写入形态)。
async fn insert_customer_tier(
    db: &DatabaseConnection,
    id: i32,
    tier: Option<&str>,
) -> Result<(), DbErr> {
    let values: Vec<sea_orm::Value> = vec![
        id.into(),
        format!("TIER-LCK-{id}").into(),
        OPERATOR_ID.into(),
        OPERATOR_ID.into(),
        tier.into(),
    ];
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO customers (id,customer_code,customer_name,credit_limit,payment_terms,status,owner_id,created_by,customer_type,tier,created_at,updated_at)
         VALUES ($1,$2,'分层词表活体锁探针',0,30,'active',$3,$4,'other',$5,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
        values,
    ))
    .await
    .map(|_| ())
}

/// **省略** `tier` 列的旁路直插——列无 DEFAULT 时唯一落 NULL 的形态。
async fn insert_customer_omitting_tier(db: &DatabaseConnection, id: i32) -> Result<(), DbErr> {
    let values: Vec<sea_orm::Value> = vec![
        id.into(),
        format!("TIER-LCK-OMIT-{id}").into(),
        OPERATOR_ID.into(),
        OPERATOR_ID.into(),
    ];
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO customers (id,customer_code,customer_name,credit_limit,payment_terms,status,owner_id,created_by,customer_type,created_at,updated_at)
         VALUES ($1,$2,'省略tier探针',0,30,'active',$3,$4,'other','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
        values,
    ))
    .await
    .map(|_| ())
}

async fn tier_of_id(db: &DatabaseConnection, id: i32) -> String {
    let row = db
        .query_one_raw(pg_stmt(&format!(
            "SELECT COALESCE(tier, '<NULL>') AS tier FROM customers WHERE id={id}"
        )))
        .await
        .unwrap_or_else(|e| panic!("回读 customers.tier(id={id}) 失败: {e}"));
    let r = row.unwrap_or_else(|| panic!("customers 里没有 id={id} 的行(探针行未落库?)"));
    r.try_get::<String>("", "tier")
        .unwrap_or_else(|e| panic!("id={id} 的 tier 解码失败: {e}"))
}

async fn count_by_id(db: &DatabaseConnection, id: i32) -> i64 {
    one_i64(
        db,
        &format!("SELECT COUNT(*) AS n FROM customers WHERE id={id}"),
        "n",
    )
    .await
}

// ---------------------------------------------------------------------------
// 1. 同源三向对撞:迁移原文解析集 == 运行时 ALLOWED == 活库 CHECK 取值集,
//    逐元素**集合相等**;MAJOR 对两者的子集关系;目录形态(可空、无 DEFAULT、
//    唯一 CHECK);回填映射与评级词表锚点;判据源码唯一性扫描。
//    改坏什么必红:
//    · m0083 ALLOWED_SQL 增/删/改任一 token(含大小写)⇒ 与运行时集合相等断言红;
//    · constants ALLOWED 单边改动 ⇒ 同一断言红;
//    · MAJOR 扩到非高档档或缩并 ⇒ 成员点名/严格子集/行为锁(可达性锁文件)三处红;
//    · 活库 CHECK 与前两者不等、NOT VALID、表上多出第二条涉 tier 的 CHECK ⇒ 目录断言红;
//    · 列被改成 NOT NULL 或加了 DEFAULT ⇒ "未定档"形态失真,目录断言红;
//    · 审批判据回潮读金额/额度 ⇒ 服务源码扫描红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn tier_vocab_in_migration_equals_app_allowed_equals_live_db_check() {
    let db = setup_test_db().await;
    let m0083_src = read_repo_file(M0083_REL);

    let from_migration = parse_allowed_sql_tokens(&m0083_src, M0083_REL);
    let runtime: BTreeSet<String> = ALLOWED.iter().map(|t| t.to_string()).collect();

    assert_eq!(
        from_migration, runtime,
        "m0083 ALLOWED_SQL 解析集与运行时 constants::customer_tier::ALLOWED 不是同一集合\
         (集合相等判据,包含关系不算;大小写漂移同样红):\n迁移原文={from_migration:?}\n应用词表={runtime:?}"
    );

    // 接线形态:CHECK 取值只能来自 ALLOWED_SQL 这一份种子
    assert!(
        m0083_src.contains("ADD CONSTRAINT \"chk_customers_tier\""),
        "m0083 必须以带引号约束名 {CHK_NAME} 添加 CHECK(本锁与错误归因按该名),\
         实际原文未出现该字面量"
    );
    assert!(
        m0083_src.contains("CHECK (\"tier\" IS NULL OR \"tier\" IN ({allowed}))"),
        "m0083 的 CHECK 必须是 `\"tier\" IS NULL OR \"tier\" IN ({{allowed}})` 占位形态——\
         值域唯一由 ALLOWED_SQL 注入且放行 NULL(未定档),改成内联第二套清单即漂移"
    );
    assert!(
        m0083_src.contains("allowed = ALLOWED_SQL,"),
        "m0083 的 format 实参必须把 {{allowed}} 绑定到 ALLOWED_SQL(同一份清单),\
         换绑其它来源=库里词表与 constants 脱钩"
    );

    // 回填映射锚点:评级六档 → 分层四档的序位映射在迁移原文内成文;
    // 评级六档词表的真源(计算处)锚点仍在——任一侧被改动都必须重裁映射。
    for anchor in [
        "WHEN 'AAA' THEN 'VIP'",
        "WHEN 'AA'  THEN 'GOLD'",
        "WHEN 'A'   THEN 'SILVER'",
        "WHEN 'BBB' THEN 'NORMAL'",
        "WHEN 'BB'  THEN 'NORMAL'",
        "WHEN 'B'   THEN 'NORMAL'",
    ] {
        assert!(
            m0083_src.contains(anchor),
            "m0083 回填映射缺少锚点 {anchor:?}(评级→分层序位对齐被单边改动)"
        );
    }
    let evaluate_src = read_repo_file(EVALUATE_REL);
    assert!(
        evaluate_src.contains("(\"AAA\", 1000000)") && evaluate_src.contains("(\"B\", 10000)"),
        "评级计算处(customer_credit_evaluate)的六档词表锚点缺失——回填映射的依据来源\
         已改动,映射必须随之重裁,本锁拒绝静默放行"
    );

    // —— 高档集合 MAJOR:非空、严格子集、成员点名、对活库值域保持包含(同源链) ——
    assert!(
        !MAJOR.is_empty() && MAJOR.len() < ALLOWED.len(),
        "MAJOR 必须非空且是 ALLOWED 的严格子集(全体都高档=二级审批门失效,空集=二级审批永不可达)"
    );
    assert_eq!(
        MAJOR,
        &[VIP, GOLD],
        "高档集合成员必须恰为分层阶梯最高两档 {{VIP, GOLD}};实际 {MAJOR:?}"
    );
    for token in MAJOR {
        assert!(
            from_migration.contains(*token) && runtime.contains(*token),
            "MAJOR 成员 '{token}' 不在迁移词表/应用词表内——判据集合与 CHECK 词表脱钩(第二套口径)"
        );
    }
    for token in [SILVER, NORMAL] {
        assert!(
            !MAJOR.contains(&token),
            "SILVER/NORMAL 属阶梯低两档,不得进高档集合(否则二级审批口径整体下移);'{token}' 混入"
        );
    }

    // —— 活库目录:约束真实存在、类型/校验位正确、值域逐元素相等、同列唯一 CHECK ——
    let contype = one_text(
        &db,
        &format!(
            "SELECT c.contype::text FROM pg_constraint c \
              WHERE c.conrelid='customers'::regclass AND c.conname='{CHK_NAME}'"
        ),
        "contype",
    )
    .await;
    assert_eq!(
        contype.as_deref(),
        Some("c"),
        "pg_constraint 里必须存在名为 {CHK_NAME} 且 contype='c'(CHECK)的约束;\n\
         None=约束不存在(m0083 未注册/被 down 撤掉),其它值=约束类型漂移;实际: {contype:?}"
    );

    let validated = one_text(
        &db,
        &format!(
            "SELECT c.convalidated::text FROM pg_constraint c \
              WHERE c.conrelid='customers'::regclass AND c.conname='{CHK_NAME}'"
        ),
        "convalidated",
    )
    .await;
    assert_eq!(
        validated.as_deref(),
        Some("true"),
        "convalidated 必须为 true:NOT VALID 的 CHECK 放行存量脏行,不算收口;实际: {validated:?}"
    );

    let other_tier_checks = one_i64(
        &db,
        &format!(
            "SELECT COUNT(*) AS n FROM pg_constraint c \
              WHERE c.conrelid='customers'::regclass AND c.contype='c' \
                AND c.conname <> '{CHK_NAME}' \
                AND pg_get_constraintdef(c.oid) LIKE '%tier%'"
        ),
        "n",
    )
    .await;
    assert_eq!(
        other_tier_checks, 0,
        "customers 上除 {CHK_NAME} 外还另有 {other_tier_checks} 条涉及 tier 的 CHECK——\
         同列多 CHECK 即第二套词表,本锁的\"集合相等\"判据失去唯一真源,必须先归一"
    );

    let def = one_text(
        &db,
        &format!(
            "SELECT pg_get_constraintdef(c.oid) AS def FROM pg_constraint c \
              WHERE c.conrelid='customers'::regclass AND c.conname='{CHK_NAME}'"
        ),
        "def",
    )
    .await
    .unwrap_or_else(|| panic!("读不到 {CHK_NAME} 的定义(与上一句 contype 查询自相矛盾)"));
    assert!(
        def.contains("tier"),
        "{CHK_NAME} 必须作用在 tier 列上,实际定义: {def}"
    );
    let live = quoted_tokens(&def);
    assert_eq!(
        live, runtime,
        "活库 CHECK 取值集 != 应用词表运行时集合(逐元素相等,不是包含;\n\
         迁移原文解析集已与 runtime 逐元素相等钉过,故活库==原文==应用三向闭环成立):\n\
         活库={live:?}\n应用={runtime:?}\n\
         活库多于应用=旁路可写入业务永不产生的脏 token;活库少于应用=某个合法档位\
         会在写入时撞 23514 冒裸 500——两侧任一侧单独增删值都是契约漂移"
    );

    // —— 目录形态:可空(NULL=未定档是合法存储形态)且**无 DEFAULT**(默认值=造数) ——
    let nullable = one_text(
        &db,
        "SELECT is_nullable FROM information_schema.columns \
          WHERE table_schema='public' AND table_name='customers' AND column_name='tier'",
        "is_nullable",
    )
    .await;
    assert_eq!(
        nullable.as_deref(),
        Some("YES"),
        "customers.tier 的 is_nullable 必须是 'YES'(NULL=未定档是合法形态);\
         'NO'=有人把未定档强制成了某个档位,实际: {nullable:?}"
    );
    let default_raw = one_text(
        &db,
        "SELECT COALESCE(column_default, '<NULL>') AS column_default \
          FROM information_schema.columns \
          WHERE table_schema='public' AND table_name='customers' AND column_name='tier'",
        "column_default",
    )
    .await
    .unwrap_or_else(|| panic!("information_schema 里查不到 customers.tier(建列未生效/迁移未注册)"));
    assert_eq!(
        default_raw, "<NULL>",
        "customers.tier 不得带 DEFAULT('<NULL>'=无默认才是期望形态):给默认值=\
         替全部存量客户伪造一个档位,未定档必须是 NULL;实际渲染: {default_raw:?}"
    );

    // —— 判据唯一性扫描:审批服务只读分层列,金额/额度旧口径不许残留在判据路径 ——
    let service_src = read_repo_file(SERVICE_REL);
    assert!(
        service_src.contains("customer_tier::is_major"),
        "二级审批判据必须唯一经 `constants::customer_tier::is_major`(高档集合唯一真源)"
    );
    for banned in [
        "estimated_amount",
        "500_000",
        "DEFAULT_LARGE_CUSTOMER_CREDIT_THRESHOLD",
        "LARGE_CUSTOMER_ESTIMATED_AMOUNT_THRESHOLD",
    ] {
        assert!(
            !service_src.contains(banned),
            "审批服务源码残留第二口径痕迹 {banned:?}——判据只许读 customers.tier,\
             金额/额度代理判大客户已被终裁废除"
        );
    }
}

// ---------------------------------------------------------------------------
// 2. 值域活体形态:四个合法档位与显式 NULL/省略列都能真实落库(逐字符原样),
//    词表外形态(小写漂移/新造档/尾随空白/空串)被 {CHK_NAME} 报 23514 且零落库。
//    改坏什么必红:CHECK 被撤/未注册 ⇒ 越界直插成功红;CHECK 值域漂移 ⇒ 合法档撞
//    23514 红;列加了 NOT NULL ⇒ 省略/NULL 探针撞 23502 红;归因到别的约束 ⇒ 约束名红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn tier_column_admits_four_tiers_and_null_and_rejects_out_of_vocab() {
    let db = setup_test_db().await;
    seed_users(&db).await;

    // 前置清理本文件全部探针 id(幂等重跑)
    let null_probe_id = PROBE_LEGAL_BASE + ALLOWED.len() as i32; // 显式 NULL 探针
    let mut probe_ids: Vec<String> = (0..BYPASS_BANNED_TOKENS.len())
        .map(|i| (PROBE_BASE + i as i32).to_string())
        .collect();
    probe_ids.extend(
        ALLOWED
            .iter()
            .enumerate()
            .map(|(i, _)| (PROBE_LEGAL_BASE + i as i32).to_string()),
    );
    probe_ids.extend([null_probe_id.to_string(), PROBE_OMIT_ID.to_string()]);
    exec(
        &db,
        &format!(
            "DELETE FROM customers WHERE id IN ({})",
            probe_ids.join(",")
        ),
    )
    .await;

    // 四个合法档位:逐 token 直插成功且原样落库(大写逐字符)
    for (idx, token) in ALLOWED.iter().enumerate() {
        let id = PROBE_LEGAL_BASE + idx as i32;
        insert_customer_tier(&db, id, Some(token))
            .await
            .unwrap_or_else(|e| {
                panic!("合法档位 '{token}' 直插必须成功(词表/CHECK 漏掉该值即红): {e}")
            });
        let landed = tier_of_id(&db, id).await;
        assert_eq!(
            landed, *token,
            "落库值必须逐字符等于写入值(写链/列上任何归一都会在这里红):期望 '{token}' 实际 {landed:?}"
        );
    }

    // 显式 NULL 与省略列:两种"未定档"形态都必须可落且落真 NULL
    insert_customer_tier(&db, null_probe_id, None)
        .await
        .unwrap_or_else(|e| {
            panic!("显式 tier=NULL(未定档)必须可落——CHECK 的 IS NULL 放行形态: {e}")
        });
    insert_customer_omitting_tier(&db, PROBE_OMIT_ID)
        .await
        .unwrap_or_else(|e| panic!("省略 tier 列必须可落(列无 DEFAULT,落 NULL): {e}"));
    assert_eq!(
        tier_of_id(&db, null_probe_id).await,
        "<NULL>",
        "显式 NULL 探针必须落真 NULL(被默认值/兜底改写=未定档形态失真)"
    );
    assert_eq!(
        tier_of_id(&db, PROBE_OMIT_ID).await,
        "<NULL>",
        "省略列探针必须落 NULL 而非任何默认档位"
    );

    // 词表外:旁路直插必须撞 {CHK_NAME} 23514 且零落库
    for (idx, token) in BYPASS_BANNED_TOKENS.iter().enumerate() {
        let id = PROBE_BASE + idx as i32;
        let err = insert_customer_tier(&db, id, Some(token))
            .await
            .err()
            .unwrap_or_else(|| {
                panic!("旁路直插 tier='{token}' 竟成功——库层兜底失效(小写/新档/空白混入都是脏值)")
            });
        assert_eq!(
            sqlstate_of(&err).as_deref(),
            Some("23514"),
            "旁路写 '{token}' 必须撞 CHECK 成 check_violation(SQLSTATE 23514);\
             只断 is_err 是假绿:期望 23514 实际 SQLSTATE={:?}\n原始错误: {err}",
            sqlstate_of(&err)
        );
        assert_eq!(
            constraint_of(&err).as_deref(),
            Some(CHK_NAME),
            "23514 必须归因到 {CHK_NAME} 本约束(只断状态码会把表上别的 CHECK 在挡\
             误当本锁生效);实际约束名={:?}\n原始错误: {err}",
            constraint_of(&err)
        );
        assert_eq!(
            count_by_id(&db, id).await,
            0,
            "被 CHECK 拒掉的 '{token}' 写入必须零落库(单语句原子失败,不留半行)"
        );
    }
}
