//! 合同模板体系四张新表与两侧合同主表补列的活库形态锁
//!
//! 功能 在真已迁移、按用例清表的 PostgreSQL 上钉死合同模板体系的数据形态。表与列的存在性
//!   与可空性与同形性、八组词表的 CHECK 真拒词表外取值、同一类别同一立场只允许一版启用的
//!   部分唯一门、模板编码与版本的复合业务键、条款库引用与模板内正文二选一、条款位置唯一、
//!   合同条款快照的固化不可追溯、两侧合同主表十二枚条款与签署与印花税与内外销与含税列的
//!   同形，以及历史行保持空值。
//! 调用方 集成测试 crate，经测试夹具连已跑完整迁移链的 PostgreSQL，跑法
//!   `bash E:/tmp/tools/live_test_lane.sh <车道> contract_template_schema_shape_live_test`。
//! 入参 无，夹具注入库连接。
//! 传给谁 断言落在 information_schema 与 pg_catalog 的目录读数、数据库上报的 SQLSTATE 与
//!   约束名、以及回读的列值上。不经过 handler 与 service，不派发 HTTP。
//! 存什么 只写本文件自建的探针行。模板编码与条款标题与合同编号一律带 IT-CTPL 私有号段，
//!   不与任何真实业务编码相撞。合同条款快照的合同主键用 900001 与 900002 两枚私有号段值。
//! 存哪里 PostgreSQL `contract_templates`、`clause_library`、`contract_template_clauses`、
//!   `contract_clauses`、`sales_contracts`、`purchase_contracts`。
//!
//! 用例编排说明 夹具对业务表逐用例 TRUNCATE，同一集成二进制内的用例默认并行且相互清表，
//!   故每个用例都自建所需种子行并只断自己写的行。跨表往返（改版不影响已固化行）整段收在
//!   同一个用例函数内顺序执行，从结构上排除竞态。四张新表都不在夹具的种子参照表名单内，
//!   逐用例清空，起点确定。
//!
//! 兑付分工 本文件全部用例都需要已应用 m_contract_template_schema 的活库，属编译与用例类
//!   验证，按车道基线的验证分工一律由本次推送触发的云端 CI 兑付，本地不跑 cargo test，
//!   也不在本地活库执行本文件，更不在本地跑迁移。迁移在活库上的应用与 up 与 down 可用性
//!   同样由 CI 兑付，本文件只断迁移应用完成之后的库内形态；up 与 down 的互逆与域内逆序
//!   次序在文本面由 `tests/contract_template_schema_migration_lock_test.rs` 的次序锁钉住。
//!
//! 反空操作自证
//! - 模板状态词表 CHECK 缺席或被放宽时，`template_vocab_checks_reject_out_of_vocab_values`
//!   的越界插入会成功，当场判红并点名缺失的约束名。
//! - ACTIVE 部分唯一索引退回全表 UNIQUE 或整个缺席时，
//!   `active_gate_allows_only_one_active_per_type_and_side` 的竞位段或停用位段必红。
//! - 快照只存 template_id 而不存正文时，`template_revision_never_rewrites_frozen_snapshot`
//!   的改版段必红（同一条用例内有活模板侧确实已改的对照断言，防止 UPDATE 零行造成的假绿）。
//! - 采购侧补列漏一枚或列型与销售侧不一致时，
//!   `both_contract_main_tables_share_identical_column_shape` 的逐列比对必红。
//! - 迁移给新增列带了默认值时，同一条用例的默认值反向断言必红。
//! - 历史行被回填伪造内容时，`legacy_contract_rows_keep_null_and_market_check_allows_null`
//!   的空值断言必红。
//! 断言不使用恒真式，目录查询取不到行直接 panic 点名，绝不当成通过。

mod test_common;

use sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, DbErr, RuntimeErr, Statement, Value,
};
use std::collections::BTreeSet;
use test_common::setup_test_db;

/// 探针行的私有号段前缀，所有自建编码都带它。
const IT: &str = "IT-CTPL";
/// 快照探针使用的合同主键私有号段值。跨族独立性用例刻意让两族共用同一枚数值，
/// 证明多态引用按 kind 加 id 定位、同数值不互相锁死，故只需要一枚号段常量。
const IT_SALES_ID: i32 = 900_001;

/// 词表外的越界取值，只用于触发数据库拒绝，刻意不属于任何合法集。
const BANNED_STATUS: &str = "enabled";
const BANNED_CONTRACT_TYPE: &str = "eleventh_type";
const BANNED_PARTY_SIDE: &str = "agent";
const BANNED_CLAUSE_CODE: &str = "not_a_clause_code";
const BANNED_CONTRACT_KIND: &str = "labor";
const BANNED_APPLIES_WHEN: &str = "when_hot";
const BANNED_MARKET: &str = "内外销";

// ---------------------------------------------------------------------------
// 裸 SQL 助手
// ---------------------------------------------------------------------------

/// 带绑定值的 PostgreSQL 语句，取值一律走 $n 占位，不把外部串拼进 SQL 文本。
fn pg_bound(sql: String, values: Vec<Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, values)
}

/// 可空文本列的绑定形态，None 落 NULL。
fn opt_str(value: Option<&str>) -> Value {
    Value::String(value.map(|v| v.to_string()))
}

/// 可空整数列的绑定形态，None 落 NULL。
fn opt_int(value: Option<i32>) -> Value {
    Value::Int(value)
}

/// 驱动上报的 SQLSTATE。非驱动错误直接 panic，避免任何外键或类型或长度错误
/// 被只判 `is_err` 的断言当成 CHECK 生效的证据。
fn sqlstate_of(err: &DbErr) -> String {
    match err {
        DbErr::Exec(RuntimeErr::SqlxError(e)) | DbErr::Query(RuntimeErr::SqlxError(e)) => e
            .as_database_error()
            .and_then(|de| sqlx::error::DatabaseError::code(de).map(|c| c.into_owned()))
            .unwrap_or_else(|| panic!("驱动未上报 SQLSTATE，无法归因: {err}")),
        other => panic!("期望的是执行期数据库错误，实际是非驱动错误: {other}"),
    }
}

/// 驱动上报的约束名，把 23514 与 23505 与 23503 归因到本用例锁定的那一条约束或索引，
/// 而不是表上任意一条 CHECK 或任意一种失败。
fn constraint_of(err: &DbErr) -> String {
    match err {
        DbErr::Exec(RuntimeErr::SqlxError(e)) | DbErr::Query(RuntimeErr::SqlxError(e)) => e
            .as_database_error()
            .and_then(|de| sqlx::error::DatabaseError::constraint(de).map(|s| s.to_string()))
            .unwrap_or_else(|| panic!("驱动未上报约束名，无法归因: {err}")),
        other => panic!("期望的是执行期数据库错误，实际是非驱动错误: {other}"),
    }
}

/// 断言一次写入被数据库以指定的 SQLSTATE 与指定的约束或索引名拒绝。
/// 只断 SQLSTATE 是假绿来源，必须连同约束名一起断；写入竟然成功也当场判红。
async fn expect_rejected(db: &DatabaseConnection, stmt: Statement, code: &str, name: &str) {
    let err = match db.execute_raw(stmt).await {
        Ok(_) => panic!("期望被 {name} 拒绝的写入竟然成功（该约束未生效或被放宽）"),
        Err(err) => err,
    };
    let got_code = sqlstate_of(&err);
    let got_name = constraint_of(&err);
    assert_eq!(
        got_code, code,
        "拒绝成因的 SQLSTATE 应为 {code}（23514 为 CHECK，23505 为唯一约束，23503 为外键），实得 {got_code}；原始错误 {err}"
    );
    assert_eq!(
        got_name, name,
        "拒绝必须归因到约束或索引 {name}，实得 {got_name}；原始错误 {err}"
    );
}

/// 必须成功的写入。失败即点名 panic，不带默认值兜底。
async fn exec_ok(db: &DatabaseConnection, stmt: Statement, what: &str) {
    if let Err(e) = db.execute_raw(stmt).await {
        panic!("{what} 必须成功，实得数据库错误: {e}");
    }
}

/// 读一条目录或标量读数（单列文本）。取不到行直接 panic 点名，绝不静默当成通过。
async fn query_text(db: &DatabaseConnection, sql: String, values: Vec<Value>) -> String {
    let row = db
        .query_one_raw(pg_bound(sql, values))
        .await
        .unwrap_or_else(|e| panic!("目录查询失败: {e}"))
        .unwrap_or_else(|| panic!("目录查询返回 0 行，被查找的对象确实不存在"));
    row.try_get_by_index::<String>(0)
        .unwrap_or_else(|e| panic!("目录查询首列解码失败: {e}"))
}

/// 读一条单列布尔判定并转成文本，用于「某行是否全为空」这类聚合。
async fn query_bool_text(db: &DatabaseConnection, sql: String, values: Vec<Value>) -> bool {
    let text = query_text(db, sql, values).await;
    match text.as_str() {
        "t" | "true" => true,
        "f" | "false" => false,
        other => panic!("布尔读数应为 t 或 true 或 f 或 false，实得 {other:?}"),
    }
}

/// 取某张表上现行生效的 CHECK 定义原文，约束不存在即点名 panic。
async fn check_def(db: &DatabaseConnection, table: &str, constraint: &str) -> String {
    query_text(
        db,
        "SELECT pg_get_constraintdef(c.oid) FROM pg_constraint c \
          WHERE c.conrelid = to_regclass($1) AND c.conname = $2"
            .to_string(),
        vec![table.into(), constraint.into()],
    )
    .await
}

/// 提取文本中全部单引号 token，用于把 CHECK 定义里的取值集解析成集合。
/// 引号不闭合直接 panic，解析出空集也直接 panic，解析面失守绝不当成通过。
fn quoted_tokens(text: &str) -> BTreeSet<String> {
    let mut tokens = BTreeSet::new();
    let mut cursor = text;
    while let Some(open) = cursor.find('\'') {
        cursor = &cursor[open + 1..];
        let close = cursor
            .find('\'')
            .unwrap_or_else(|| panic!("CHECK 定义里的单引号未闭合: {text}"));
        tokens.insert(cursor[..close].to_string());
        cursor = &cursor[close + 1..];
    }
    assert!(
        !tokens.is_empty(),
        "从 CHECK 定义解析出空取值集合——解析失守，拒绝继续比对。定义原文={text}"
    );
    tokens
}

/// 表在当前 schema 下是否存在。
async fn table_exists(db: &DatabaseConnection, table: &str) -> bool {
    let count = query_text(
        db,
        "SELECT count(*)::text FROM information_schema.tables \
          WHERE table_schema = current_schema() AND table_name = $1"
            .to_string(),
        vec![table.into()],
    )
    .await;
    count != "0"
}

/// 一列的完整形态串（类型、宽度、标度、可空性、默认值），用于两侧同形比对。
async fn column_shape(db: &DatabaseConnection, table: &str, column: &str) -> String {
    query_text(
        db,
        "SELECT concat_ws('|', data_type, \
                   coalesce(character_maximum_length::text, '-'), \
                   coalesce(numeric_precision::text, '-'), \
                   coalesce(numeric_scale::text, '-'), \
                   is_nullable, \
                   coalesce(column_default, '<无默认>')) \
           FROM information_schema.columns \
          WHERE table_schema = current_schema() AND table_name = $1 AND column_name = $2"
            .to_string(),
        vec![table.into(), column.into()],
    )
    .await
}

// ---------------------------------------------------------------------------
// 种子助手（全部自建自清，编码带 IT-CTPL 私有号段）
// ---------------------------------------------------------------------------

/// 种一枚模板并返回其主键。active 行的 approved_by 必须给值，draft 行可给 None。
async fn seed_template(
    db: &DatabaseConnection,
    code: &str,
    contract_type: &str,
    party_side: &str,
    version: i32,
    status: &str,
    approved_by: Option<i32>,
) -> i32 {
    let row = db
        .query_one_raw(pg_bound(
            "INSERT INTO contract_templates \
               (template_code, contract_type, party_side, version, title, status, approved_by) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id"
                .to_string(),
            vec![
                code.into(),
                contract_type.into(),
                party_side.into(),
                version.into(),
                format!("{IT} 模板标题").into(),
                status.into(),
                opt_int(approved_by),
            ],
        ))
        .await
        .unwrap_or_else(|e| panic!("种模板 {code} 失败: {e}"))
        .unwrap_or_else(|| panic!("种模板 {code} 应回读一行 RETURNING id，实得 0 行"));
    row.try_get_by_index::<i32>(0)
        .unwrap_or_else(|e| panic!("模板主键解码失败: {e}"))
}

/// 种一枚条款库行并返回其主键。
async fn seed_library(db: &DatabaseConnection, clause_code: &str, body: &str, status: &str) -> i32 {
    let row = db
        .query_one_raw(pg_bound(
            "INSERT INTO clause_library (clause_code, title, body, status) \
             VALUES ($1, $2, $3, $4) RETURNING id"
                .to_string(),
            vec![
                clause_code.into(),
                format!("{IT} 条款标题").into(),
                body.into(),
                status.into(),
            ],
        ))
        .await
        .unwrap_or_else(|e| panic!("种条款库 {clause_code} 失败: {e}"))
        .unwrap_or_else(|| panic!("种条款库 {clause_code} 应回读一行 RETURNING id，实得 0 行"));
    row.try_get_by_index::<i32>(0)
        .unwrap_or_else(|e| panic!("条款库主键解码失败: {e}"))
}

/// 种一枚模板条款项。正文来源由调用方给，库引用与内文二者之一，另一个传 None。
async fn seed_template_clause(
    db: &DatabaseConnection,
    template_id: i32,
    clause_code: &str,
    title: &str,
    library_clause_id: Option<i32>,
    body: Option<&str>,
    sort: i16,
    applies_when: Option<&str>,
) -> i32 {
    let row = db
        .query_one_raw(pg_bound(
            "INSERT INTO contract_template_clauses \
               (template_id, clause_code, title, body, library_clause_id, sort, applies_when) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id"
                .to_string(),
            vec![
                template_id.into(),
                clause_code.into(),
                title.into(),
                opt_str(body),
                opt_int(library_clause_id),
                Value::SmallInt(Some(sort)),
                opt_str(applies_when),
            ],
        ))
        .await
        .unwrap_or_else(|e| panic!("种模板条款项 {clause_code} 失败: {e}"))
        .unwrap_or_else(|| panic!("种模板条款项 {clause_code} 应回读一行 RETURNING id，实得 0 行"));
    row.try_get_by_index::<i32>(0)
        .unwrap_or_else(|e| panic!("模板条款项主键解码失败: {e}"))
}

/// 种一行合同条款快照。参数全部由调用方给，便于同一用例内构造重复与越界形态。
async fn seed_snapshot(
    db: &DatabaseConnection,
    template_id: i32,
    contract_kind: &str,
    contract_id: i32,
    clause_code: &str,
    title: &str,
    body_rendered: &str,
    template_version: i32,
    sort: i16,
) {
    exec_ok(
        db,
        pg_bound(
            "INSERT INTO contract_clauses \
               (contract_kind, contract_id, clause_code, title, body_rendered, template_id, template_version, sort) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
                .to_string(),
            vec![
                contract_kind.into(),
                contract_id.into(),
                clause_code.into(),
                title.into(),
                body_rendered.into(),
                template_id.into(),
                template_version.into(),
                Value::SmallInt(Some(sort)),
            ],
        ),
        "写合同条款快照",
    )
    .await;
}

// ---------------------------------------------------------------------------
// 判据一 四张表与关键列的存在性
// ---------------------------------------------------------------------------

/// 四张新表存在，且每表的关键列真的建出来了，可空性与 DDL 逐列一致。
/// 防的漂移 域未注册进链尾，或表建了但列被漏写，读侧要到运行期才冒缺列错误。
#[tokio::test]
async fn four_new_tables_exist_with_declared_columns() {
    let db = setup_test_db().await;

    for table in [
        "contract_templates",
        "clause_library",
        "contract_template_clauses",
        "contract_clauses",
    ] {
        assert!(
            table_exists(&db, table).await,
            "表 {table} 不存在于活库——合同模板体系域未注册进迁移链尾或未应用到该库"
        );
    }

    // (表, 列, 期望 is_nullable) 三件套，缺列即由 column_shape 点名。
    let expected: &[(&str, &str, &str)] = &[
        ("contract_templates", "template_code", "NO"),
        ("contract_templates", "contract_type", "NO"),
        ("contract_templates", "party_side", "NO"),
        ("contract_templates", "version", "NO"),
        ("contract_templates", "status", "NO"),
        ("contract_templates", "effective_from", "YES"),
        ("contract_templates", "approved_by", "YES"),
        ("clause_library", "clause_code", "NO"),
        ("clause_library", "title", "NO"),
        ("clause_library", "body", "NO"),
        ("clause_library", "applies_contract_type", "YES"),
        ("contract_template_clauses", "template_id", "NO"),
        ("contract_template_clauses", "clause_code", "NO"),
        ("contract_template_clauses", "title", "NO"),
        ("contract_template_clauses", "body", "YES"),
        ("contract_template_clauses", "library_clause_id", "YES"),
        ("contract_template_clauses", "required", "NO"),
        ("contract_template_clauses", "sort", "NO"),
        ("contract_template_clauses", "applies_when", "YES"),
        ("contract_template_clauses", "legal_review_status", "NO"),
        ("contract_template_clauses", "legal_reviewed_by", "YES"),
        ("contract_clauses", "contract_kind", "NO"),
        ("contract_clauses", "contract_id", "NO"),
        ("contract_clauses", "clause_code", "NO"),
        ("contract_clauses", "title", "NO"),
        ("contract_clauses", "body_rendered", "NO"),
        ("contract_clauses", "template_id", "NO"),
        ("contract_clauses", "template_version", "NO"),
        ("contract_clauses", "sort", "NO"),
        // 链尾独立域 m0119 追加的来源指针列 可空，历史快照行为空值
        ("contract_clauses", "template_clause_id", "YES"),
    ];
    for (table, column, nullable) in expected {
        let shape = column_shape(&db, table, column).await;
        let got = shape.split('|').nth(4).unwrap_or("<无形态>");
        assert_eq!(
            got, *nullable,
            "列 {table}.{column} 的可空性应为 {nullable}，实得形态 {shape}"
        );
    }

    // 条款库正文必须是非空且没有默认值，空串冒充正文的形态由 CHECK 拒（见下一条用例）。
    let library_body = column_shape(&db, "clause_library", "body").await;
    assert!(
        library_body.ends_with("|NO|<无默认>"),
        "clause_library.body 必须 NOT NULL 且无默认值，不给自称是条款而正文为空的行落地的通道，实得形态 {library_body}"
    );
}

// ---------------------------------------------------------------------------
// 判据二 词表 CHECK 与 ACTIVE 门
// ---------------------------------------------------------------------------

/// 三列词表 CHECK 真拒词表外取值，且归因到本列自己的约束名。
/// 同一条用例里正向写一行词表内取值，证明上一条的拒绝不是表级其它约束冒名。
#[tokio::test]
async fn template_vocab_checks_reject_out_of_vocab_values() {
    let db = setup_test_db().await;

    expect_rejected(
        &db,
        pg_bound(
            "INSERT INTO contract_templates (template_code, contract_type, party_side, version, title, status) \
             VALUES ($1, 'domestic_sales', 'seller', 1, 'IT 探针标题', $2)"
                .to_string(),
            vec![format!("{IT}-STATUS").into(), BANNED_STATUS.into()],
        ),
        "23514",
        "chk_contract_templates_status",
    ).await;
    expect_rejected(
        &db,
        pg_bound(
            "INSERT INTO contract_templates (template_code, contract_type, party_side, version, title, status) \
             VALUES ($1, $2, 'seller', 1, 'IT 探针标题', 'draft')"
                .to_string(),
            vec![format!("{IT}-TYPE").into(), BANNED_CONTRACT_TYPE.into()],
        ),
        "23514",
        "chk_contract_templates_contract_type",
    ).await;
    expect_rejected(
        &db,
        pg_bound(
            "INSERT INTO contract_templates (template_code, contract_type, party_side, version, title, status) \
             VALUES ($1, 'domestic_sales', $2, 1, 'IT 探针标题', 'draft')"
                .to_string(),
            vec![format!("{IT}-SIDE").into(), BANNED_PARTY_SIDE.into()],
        ),
        "23514",
        "chk_contract_templates_party_side",
    ).await;

    // active 无复核人必须被拒（法务复核后才置 active 的结构位）。
    expect_rejected(
        &db,
        pg_bound(
            "INSERT INTO contract_templates (template_code, contract_type, party_side, version, title, status) \
             VALUES ($1, 'domestic_sales', 'buyer', 1, 'IT 探针标题', 'active')"
                .to_string(),
            vec![format!("{IT}-ACTIVE").into()],
        ),
        "23514",
        "chk_contract_templates_active_approved",
    ).await;

    // 正向 词表内取值且 draft 无复核人必须写得进。
    let id = seed_template(
        &db,
        &format!("{IT}-OK"),
        "domestic_sales",
        "seller",
        1,
        "draft",
        None,
    )
    .await;
    assert!(id > 0, "词表内取值的模板必须落库成功");

    // 三立场都必须放行，both 是给保密与年度框架这类对等义务族用的。
    for side in ["seller", "buyer", "both"] {
        seed_template(
            &db,
            &format!("{IT}-SIDE-{side}"),
            "warehousing_storage",
            side,
            1,
            "draft",
            None,
        )
        .await;
    }
}

/// 同一类别同一立场同时只允许一版 active，停用后可再启用同键另一版。
/// 反向证 只约束 active，draft 与 retired 并行不受限；换立场与换类别不受限。
#[tokio::test]
async fn active_gate_allows_only_one_active_per_type_and_side() {
    let db = setup_test_db().await;

    seed_template(
        &db,
        &format!("{IT}-A1"),
        "fabric_yarn_purchase",
        "buyer",
        1,
        "active",
        Some(7777),
    )
    .await;

    expect_rejected(
        &db,
        pg_bound(
            "INSERT INTO contract_templates \
               (template_code, contract_type, party_side, version, title, status, approved_by) \
             VALUES ($1, 'fabric_yarn_purchase', 'buyer', 2, 'IT 探针标题', 'active', 7777)"
                .to_string(),
            vec![format!("{IT}-A2").into()],
        ),
        "23505",
        "uq_contract_templates_active_type_side",
    )
    .await;

    // 反向 换立场、换类别、同键的 draft 都不该撞门。
    seed_template(
        &db,
        &format!("{IT}-A3"),
        "fabric_yarn_purchase",
        "seller",
        1,
        "active",
        Some(7777),
    )
    .await;
    seed_template(
        &db,
        &format!("{IT}-A4"),
        "dyeing_finishing",
        "buyer",
        1,
        "active",
        Some(7777),
    )
    .await;
    let draft_v2 = seed_template(
        &db,
        &format!("{IT}-A5"),
        "fabric_yarn_purchase",
        "buyer",
        2,
        "draft",
        None,
    )
    .await;

    // 改版的真实通路 旧版停用后新版必须能启用。
    exec_ok(
        &db,
        pg_bound(
            "UPDATE contract_templates SET status = 'retired' WHERE template_code = $1".to_string(),
            vec![format!("{IT}-A1").into()],
        ),
        "停用旧版",
    )
    .await;
    exec_ok(
        &db,
        pg_bound(
            "UPDATE contract_templates SET status = 'active', approved_by = 7777 WHERE id = $1"
                .to_string(),
            vec![draft_v2.into()],
        ),
        "停用旧版后启用同键另一版",
    )
    .await;
    let active_now = query_text(
        &db,
        "SELECT count(*)::text FROM contract_templates \
          WHERE contract_type = 'fabric_yarn_purchase' AND party_side = 'buyer' AND status = 'active'"
            .to_string(),
        vec![],
    )
    .await;
    assert_eq!(
        active_now.as_str(),
        "1",
        "改版往返后同键同立场的启用行必须恰好一版，实得 {active_now}"
    );
}

/// 同编码同版本不得并存两行，同编码不同版本必须并存，这是模板版本的落点。
#[tokio::test]
async fn template_code_and_version_form_the_business_key() {
    let db = setup_test_db().await;

    seed_template(
        &db,
        &format!("{IT}-VER"),
        "export_sales",
        "seller",
        1,
        "draft",
        None,
    )
    .await;
    expect_rejected(
        &db,
        pg_bound(
            "INSERT INTO contract_templates \
               (template_code, contract_type, party_side, version, title, status) \
             VALUES ($1, 'export_sales', 'seller', 1, 'IT 探针标题', 'draft')"
                .to_string(),
            vec![format!("{IT}-VER").into()],
        ),
        "23505",
        "uq_contract_templates_code_version",
    )
    .await;
    seed_template(
        &db,
        &format!("{IT}-VER"),
        "export_sales",
        "seller",
        2,
        "draft",
        None,
    )
    .await;
    let cnt = query_text(
        &db,
        "SELECT count(*)::text FROM contract_templates WHERE template_code = $1".to_string(),
        vec![format!("{IT}-VER").into()],
    )
    .await;
    assert_eq!(
        cnt.as_str(),
        "2",
        "同编码两个版本必须能并存，改版才有落点，实得 {cnt} 行"
    );

    // 反向 版本号必须闭合在正整数，0 版不是合法版本。
    expect_rejected(
        &db,
        pg_bound(
            "INSERT INTO contract_templates \
               (template_code, contract_type, party_side, version, title, status) \
             VALUES ($1, 'export_sales', 'seller', 0, 'IT 探针标题', 'draft')"
                .to_string(),
            vec![format!("{IT}-VER0").into()],
        ),
        "23514",
        "chk_contract_templates_version",
    )
    .await;
}

// ---------------------------------------------------------------------------
// 判据三 条款库与模板条款项的结构约束
// ---------------------------------------------------------------------------

/// 条款码闭合集、条款库正文非空串、条款码全表唯一，三条各自归因。
#[tokio::test]
async fn clause_library_locks_code_vocabulary_and_rejects_blank_body() {
    let db = setup_test_db().await;

    expect_rejected(
        &db,
        pg_bound(
            "INSERT INTO clause_library (clause_code, title, body) VALUES ($1, '越界条款', '正文')"
                .to_string(),
            vec![BANNED_CLAUSE_CODE.into()],
        ),
        "23514",
        "chk_clause_library_code",
    )
    .await;
    expect_rejected(
        &db,
        pg_bound(
            "INSERT INTO clause_library (clause_code, title, body) VALUES ('subject_and_spec', '空正文条款', '')"
                .to_string(),
            vec![],
        ),
        "23514",
        "chk_clause_library_body",
    ).await;

    seed_library(&db, "subject_and_spec", "标的与规格正文", "active").await;
    expect_rejected(
        &db,
        pg_bound(
            "INSERT INTO clause_library (clause_code, title, body) VALUES ('subject_and_spec', '同码第二行', '另一段正文')"
                .to_string(),
            vec![],
        ),
        "23505",
        "uq_clause_library_code",
    ).await;

    // 专章条款的适用范围必须落在十类内，空值表示通用条款。
    expect_rejected(
        &db,
        pg_bound(
            "INSERT INTO clause_library (clause_code, title, body, applies_contract_type) \
             VALUES ('export_special_terms', '出口专章', '正文', $1)"
                .to_string(),
            vec![BANNED_CONTRACT_TYPE.into()],
        ),
        "23514",
        "chk_clause_library_applies_type",
    )
    .await;
    seed_library(&db, "export_control_screening", "制裁筛查正文", "draft").await;
}

/// 正文来源必须恰好一个，两条路都堵死；内文不许是空串。合法的二选一形态两种都得写得进。
#[tokio::test]
async fn template_clause_body_source_is_exactly_one() {
    let db = setup_test_db().await;

    let tpl = seed_template(
        &db,
        &format!("{IT}-SRC"),
        "domestic_sales",
        "seller",
        1,
        "draft",
        None,
    )
    .await;
    let lib = seed_library(&db, "notice_and_service", "通知与送达正文", "active").await;

    expect_rejected(
        &db,
        pg_bound(
            "INSERT INTO contract_template_clauses (template_id, clause_code, title, sort) \
             VALUES ($1, 'notice_and_service', '无正文条款', 1)"
                .to_string(),
            vec![tpl.into()],
        ),
        "23514",
        "chk_ctc_text_source",
    )
    .await;
    expect_rejected(
        &db,
        pg_bound(
            "INSERT INTO contract_template_clauses \
               (template_id, clause_code, title, body, library_clause_id, sort) \
             VALUES ($1, 'notice_and_service', '双真相源条款', '内文', $2, 2)"
                .to_string(),
            vec![tpl.into(), lib.into()],
        ),
        "23514",
        "chk_ctc_text_source",
    )
    .await;
    expect_rejected(
        &db,
        pg_bound(
            "INSERT INTO contract_template_clauses (template_id, clause_code, title, body, sort) \
             VALUES ($1, 'notice_and_service', '空串内文条款', '', 3)"
                .to_string(),
            vec![tpl.into()],
        ),
        "23514",
        "chk_ctc_text_source",
    )
    .await;

    seed_template_clause(
        &db,
        tpl,
        "notice_and_service",
        "引用库文的条款",
        Some(lib),
        None,
        1,
        None,
    )
    .await;
    seed_template_clause(
        &db,
        tpl,
        "annex_and_priority",
        "自带正文的条款",
        None,
        Some("附件与优先顺序内文"),
        2,
        None,
    )
    .await;

    // 引用不存在的条款库行必须撞外键，不给悬空引用留通道。
    expect_rejected(
        &db,
        pg_bound(
            "INSERT INTO contract_template_clauses \
               (template_id, clause_code, title, library_clause_id, sort) \
             VALUES ($1, 'integrity_and_personal_info', '悬空引用条款', 999999, 4)"
                .to_string(),
            vec![tpl.into()],
        ),
        "23503",
        "fk_ctc_library",
    )
    .await;
}

/// 顺序闭合、复核成对、条款码闭合集、条件词表，四条 CHECK 各自归因。
#[tokio::test]
async fn template_clause_sort_and_review_and_vocab_locks() {
    let db = setup_test_db().await;

    let tpl = seed_template(
        &db,
        &format!("{IT}-SORT"),
        "freight_transport",
        "buyer",
        1,
        "draft",
        None,
    )
    .await;
    let lib = seed_library(&db, "taxes_and_documents", "税费与票据正文", "active").await;

    expect_rejected(
        &db,
        pg_bound(
            "INSERT INTO contract_template_clauses \
               (template_id, clause_code, title, library_clause_id, sort) \
             VALUES ($1, 'taxes_and_documents', '零序条款', $2, 0)"
                .to_string(),
            vec![tpl.into(), lib.into()],
        ),
        "23514",
        "chk_ctc_sort",
    )
    .await;
    expect_rejected(
        &db,
        pg_bound(
            "INSERT INTO contract_template_clauses \
               (template_id, clause_code, title, library_clause_id, sort, legal_review_status) \
             VALUES ($1, 'taxes_and_documents', '无复核人条款', $2, 1, 'approved')"
                .to_string(),
            vec![tpl.into(), lib.into()],
        ),
        "23514",
        "chk_ctc_review_pair",
    )
    .await;
    expect_rejected(
        &db,
        pg_bound(
            "INSERT INTO contract_template_clauses \
               (template_id, clause_code, title, library_clause_id, sort) \
             VALUES ($1, $2, '越界条款码', $3, 1)"
                .to_string(),
            vec![tpl.into(), BANNED_CLAUSE_CODE.into(), lib.into()],
        ),
        "23514",
        "chk_ctc_clause_code",
    )
    .await;
    expect_rejected(
        &db,
        pg_bound(
            "INSERT INTO contract_template_clauses \
               (template_id, clause_code, title, library_clause_id, sort, applies_when) \
             VALUES ($1, 'taxes_and_documents', '越界条件', $2, 1, $3)"
                .to_string(),
            vec![tpl.into(), lib.into(), BANNED_APPLIES_WHEN.into()],
        ),
        "23514",
        "chk_ctc_applies_when",
    )
    .await;

    // 正向 复核通过且带复核人与复核时间的行必须写得进。
    exec_ok(
        &db,
        pg_bound(
            "INSERT INTO contract_template_clauses \
               (template_id, clause_code, title, library_clause_id, sort, legal_review_status, legal_reviewed_by) \
             VALUES ($1, 'taxes_and_documents', '已复核条款', $2, 1, 'approved', 6666)"
                .to_string(),
            vec![tpl.into(), lib.into()],
        ),
        "写已复核的模板条款项",
    )
    .await;
}

/// 同模板同条款码同适用条件只能一行；适用条件不同必须可并存，含税与不含税变体并存是真实形态。
#[tokio::test]
async fn template_clause_position_is_unique_but_condition_variants_coexist() {
    let db = setup_test_db().await;

    let tpl = seed_template(
        &db,
        &format!("{IT}-POS"),
        "domestic_sales",
        "seller",
        1,
        "draft",
        None,
    )
    .await;
    seed_template_clause(
        &db,
        tpl,
        "price_and_adjustment",
        "价格条款 含税变体",
        None,
        Some("含税价正文"),
        1,
        Some("tax_inclusive"),
    )
    .await;
    seed_template_clause(
        &db,
        tpl,
        "price_and_adjustment",
        "价格条款 不含税变体",
        None,
        Some("不含税价正文"),
        2,
        Some("tax_exclusive"),
    )
    .await;

    expect_rejected(
        &db,
        pg_bound(
            "INSERT INTO contract_template_clauses \
               (template_id, clause_code, title, body, sort, applies_when) \
             VALUES ($1, 'price_and_adjustment', '同条件第二行', '第三段正文', 3, 'tax_inclusive')"
                .to_string(),
            vec![tpl.into()],
        ),
        "23505",
        "uq_ctc_template_clause_position",
    )
    .await;

    // 无适用条件（NULL）的第二行也必须撞同一索引，证明折叠空值的表达式口径生效。
    seed_template_clause(
        &db,
        tpl,
        "annex_and_priority",
        "附件条款 通用",
        None,
        Some("附件正文"),
        4,
        None,
    )
    .await;
    expect_rejected(
        &db,
        pg_bound(
            "INSERT INTO contract_template_clauses \
               (template_id, clause_code, title, body, sort) \
             VALUES ($1, 'annex_and_priority', '附件条款 第二个通用行', '另一段正文', 5)"
                .to_string(),
            vec![tpl.into()],
        ),
        "23505",
        "uq_ctc_template_clause_position",
    )
    .await;
}

// ---------------------------------------------------------------------------
// 判据四 快照的固化语义
// ---------------------------------------------------------------------------

/// 快照的侧别与条款码与正文三条 CHECK、每合同每条款一行唯一、多态引用不得互锁，各自归因。
#[tokio::test]
async fn snapshot_rejects_bad_kind_blank_text_unrendered_placeholder_and_duplicate() {
    let db = setup_test_db().await;

    let tpl = seed_template(
        &db,
        &format!("{IT}-SNAP"),
        "domestic_sales",
        "seller",
        1,
        "draft",
        None,
    )
    .await;

    let bad = |kind: &'static str, code: &'static str, body: &'static str| {
        pg_bound(
            "INSERT INTO contract_clauses \
               (contract_kind, contract_id, clause_code, title, body_rendered, template_id, template_version, sort) \
             VALUES ($1, 900003, $2, '探针标题', $3, $4, 1, 1)"
                .to_string(),
            vec![kind.into(), code.into(), body.into(), tpl.into()],
        )
    };

    expect_rejected(
        &db,
        bad(BANNED_CONTRACT_KIND, "subject_and_spec", "正文"),
        "23514",
        "chk_cc_contract_kind",
    )
    .await;
    expect_rejected(
        &db,
        bad("sales", BANNED_CLAUSE_CODE, "正文"),
        "23514",
        "chk_cc_clause_code",
    )
    .await;
    expect_rejected(
        &db,
        bad("sales", "subject_and_spec", ""),
        "23514",
        "chk_cc_body_rendered",
    )
    .await;
    expect_rejected(
        &db,
        bad("sales", "subject_and_spec", "门幅 {{valid_width}} 克重 180"),
        "23514",
        "chk_cc_body_fully_rendered",
    )
    .await;

    seed_snapshot(
        &db,
        tpl,
        "sales",
        IT_SALES_ID,
        "subject_and_spec",
        "标的与规格",
        "门幅 160cm 克重 180",
        1,
        1,
    )
    .await;
    expect_rejected(
        &db,
        pg_bound(
            "INSERT INTO contract_clauses \
               (contract_kind, contract_id, clause_code, title, body_rendered, template_id, template_version, sort) \
             VALUES ('sales', 900001, 'subject_and_spec', '重复固化行', '另一段正文', $1, 1, 2)"
                .to_string(),
            vec![tpl.into()],
        ),
        "23505",
        "uq_contract_clauses_contract_code",
    ).await;

    // 反向 另一侧同主键同条款码不冲突，两族主键各自独立的多态引用不得互相锁死。
    seed_snapshot(
        &db,
        tpl,
        "purchase",
        IT_SALES_ID,
        "subject_and_spec",
        "标的与规格",
        "采购侧同条款正文",
        1,
        1,
    )
    .await;
}

/// 模板改版不得追溯改变已签合同的条款内容。正向建快照，反向证不可追溯。
#[tokio::test]
async fn template_revision_never_rewrites_frozen_snapshot() {
    let db = setup_test_db().await;

    // 正 前提 建 v1 启用模板，一枚库条款，一条引用它的模板条款项
    let tpl_v1 = seed_template(
        &db,
        &format!("{IT}-REV"),
        "domestic_sales",
        "seller",
        1,
        "active",
        Some(8888),
    )
    .await;
    let lib = seed_library(
        &db,
        "breach_liability_and_loss",
        "违约金按日万分之五，损失范围含可得利益与检验费与律师费",
        "active",
    )
    .await;
    seed_template_clause(
        &db,
        tpl_v1,
        "breach_liability_and_loss",
        "违约责任与损失范围",
        Some(lib),
        None,
        8,
        None,
    )
    .await;

    // 正 签约固化 把当时的成品文本与版本号写进快照
    let frozen_body = "违约金按日万分之五，损失范围含可得利益与检验费与律师费（本合同口径）";
    seed_snapshot(
        &db,
        tpl_v1,
        "sales",
        IT_SALES_ID,
        "breach_liability_and_loss",
        "违约责任与损失范围",
        frozen_body,
        1,
        8,
    )
    .await;

    // 改版 升 v2 并启用 库正文改写 v1 的条款项标题与顺序也改
    let tpl_v2 = seed_template(
        &db,
        &format!("{IT}-REV"),
        "domestic_sales",
        "seller",
        2,
        "draft",
        Some(8888),
    )
    .await;
    exec_ok(
        &db,
        pg_bound(
            "UPDATE contract_templates SET status = 'retired' WHERE id = $1".to_string(),
            vec![tpl_v1.into()],
        ),
        "停用旧版模板",
    )
    .await;
    exec_ok(
        &db,
        pg_bound(
            "UPDATE contract_templates SET status = 'active' WHERE id = $1".to_string(),
            vec![tpl_v2.into()],
        ),
        "启用新版模板",
    )
    .await;
    exec_ok(
        &db,
        pg_bound(
            "UPDATE clause_library SET body = '改版后的库正文，措辞完全不同' WHERE id = $1"
                .to_string(),
            vec![lib.into()],
        ),
        "改写条款库正文",
    )
    .await;
    exec_ok(
        &db,
        pg_bound(
            "UPDATE contract_template_clauses SET title = '改版后的标题', sort = 3 WHERE template_id = $1"
                .to_string(),
            vec![tpl_v1.into()],
        ),
        "改写模板条款项的标题与顺序",
    )
    .await;

    // 对照 模板侧确实被改到了，否则下面的断言是零行 UPDATE 造成的假绿。
    let clause_title_now = query_text(
        &db,
        "SELECT title FROM contract_template_clauses WHERE template_id = $1".to_string(),
        vec![tpl_v1.into()],
    )
    .await;
    assert_eq!(
        clause_title_now, "改版后的标题",
        "对照前提：模板条款项的标题必须确实被改成改版后的标题，实得 {clause_title_now}"
    );
    let lib_body_now = query_text(
        &db,
        "SELECT body FROM clause_library WHERE id = $1".to_string(),
        vec![lib.into()],
    )
    .await;
    assert_eq!(
        lib_body_now, "改版后的库正文，措辞完全不同",
        "对照前提：条款库正文必须确实被改写，实得 {lib_body_now}"
    );

    // 反 已固化行必须一个字节都没动
    let row = db
        .query_one_raw(pg_bound(
            "SELECT body_rendered, title, template_version, template_id \
               FROM contract_clauses WHERE contract_kind = 'sales' AND contract_id = $1"
                .to_string(),
            vec![IT_SALES_ID.into()],
        ))
        .await
        .unwrap_or_else(|e| panic!("回读快照失败: {e}"))
        .unwrap_or_else(|| panic!("改版后快照行必须仍在，绝不因模板侧改动而消失"));
    let body_back: String = row
        .try_get("", "body_rendered")
        .unwrap_or_else(|e| panic!("body_rendered 解码失败: {e}"));
    let title_back: String = row
        .try_get("", "title")
        .unwrap_or_else(|e| panic!("title 解码失败: {e}"));
    let version_back: i32 = row
        .try_get("", "template_version")
        .unwrap_or_else(|e| panic!("template_version 解码失败: {e}"));
    let template_back: i32 = row
        .try_get("", "template_id")
        .unwrap_or_else(|e| panic!("template_id 解码失败: {e}"));

    assert_eq!(
        body_back, frozen_body,
        "模板改版与条款库改写不得追溯改变已签合同的条款正文。改后读回={body_back}\n固化原文={frozen_body}"
    );
    assert_eq!(
        title_back, "违约责任与损失范围",
        "模板条款项改标题不得追溯改变已固化快照的标题，实得 {title_back}"
    );
    assert_eq!(
        version_back, 1,
        "快照必须留在固化当时的版本号 1，实得 {version_back}（改版回溯即审计口径失效）"
    );
    assert_eq!(
        template_back, tpl_v1,
        "快照的来源模板必须仍是当时那一版模板行，实得 {template_back}，期望 {tpl_v1}"
    );

    // 反 删除模板条款项不得牵连快照，这是快照片刻不建外键指向条款项的形态证据。
    exec_ok(
        &db,
        pg_bound(
            "DELETE FROM contract_template_clauses WHERE template_id = $1".to_string(),
            vec![tpl_v1.into()],
        ),
        "删除模板条款项",
    )
    .await;
    let clause_gone = query_bool_text(
        &db,
        "SELECT (NOT EXISTS (SELECT 1 FROM contract_template_clauses WHERE template_id = $1))::text"
            .to_string(),
        vec![tpl_v1.into()],
    )
    .await;
    assert!(
        clause_gone,
        "对照前提：模板条款项必须确实已被删除，否则下一条断言是无从判定的假绿"
    );
    let survive = query_text(
        &db,
        "SELECT count(*)::text FROM contract_clauses WHERE contract_kind = 'sales' AND contract_id = $1"
            .to_string(),
        vec![IT_SALES_ID.into()],
    )
    .await;
    assert_eq!(
        survive.as_str(),
        "1",
        "模板条款项被删后快照行必须仍在，实得 {survive} 行（快照若跟着模板行走就是追溯改写的假锁）"
    );

    // 反 被签约引用的模板不得被删掉，否则快照的来源可追溯性断链。
    expect_rejected(
        &db,
        pg_bound(
            "DELETE FROM contract_templates WHERE id = $1".to_string(),
            vec![tpl_v1.into()],
        ),
        "23503",
        "fk_cc_template",
    )
    .await;
}

// ---------------------------------------------------------------------------
// 判据五 两侧合同主表同形与历史空值
// ---------------------------------------------------------------------------

/// 两侧合同主表的十二枚条款与签署与印花税与内外销与含税列必须同形，且一律可空、无默认。
/// 同形的判据不是列名相同，而是类型与宽度与标度与可空性与默认值五件事逐列相等。
#[tokio::test]
async fn both_contract_main_tables_share_identical_column_shape() {
    let db = setup_test_db().await;

    let shared: &[&str] = &[
        "quality_terms",
        "breach_liability",
        "dispute_resolution",
        "performance_period",
        "signed_at",
        "signed_by_user_id",
        "signature_hash",
        "signature_image_url",
        "signature_certificate",
        "stamp_tax_amount",
        "contract_market",
        "tax_inclusive",
    ];

    for column in shared {
        let sales = column_shape(&db, "sales_contracts", column).await;
        let purchase = column_shape(&db, "purchase_contracts", column).await;
        assert_eq!(
            sales, purchase,
            "列 {column} 在两侧合同主表必须同形，销售侧={sales}，采购侧={purchase}（能力不对称即合同显示缺内容的根因）"
        );
        let fields: Vec<&str> = sales.split('|').collect();
        assert_eq!(
            fields.get(4).copied(),
            Some("YES"),
            "列 {column} 必须可空，历史合同要保持空值作模板前签约判据，实得形态 {sales}"
        );
        assert_eq!(
            fields.last().copied(),
            Some("<无默认>"),
            "列 {column} 不得带默认值，否则历史行会在新读法下冒充成已采集过的空条款，实得形态 {sales}"
        );
    }

    // 印花税与含税两列的类型形态单独点名，防止两侧同时错成同一副样子还比对通过。
    let stamp = column_shape(&db, "sales_contracts", "stamp_tax_amount").await;
    assert!(
        stamp.starts_with("numeric|") && stamp.contains("|18|4|"),
        "stamp_tax_amount 必须是 numeric(18,4) 与金额口径同源，实得形态 {stamp}"
    );
    let market = column_shape(&db, "purchase_contracts", "contract_market").await;
    assert!(
        market.starts_with("character varying|16|"),
        "contract_market 必须是 varchar(16) 的词表列形态，实得形态 {market}"
    );
    let tax = column_shape(&db, "sales_contracts", "tax_inclusive").await;
    assert!(
        tax.starts_with("boolean|"),
        "tax_inclusive 必须是 boolean 的三态列形态，实得形态 {tax}"
    );
}

/// 历史合同行不写新列必须读出空值，内外销 CHECK 放行空值但拒词表外值。
#[tokio::test]
async fn legacy_contract_rows_keep_null_and_market_check_allows_null() {
    let db = setup_test_db().await;

    let sales_row = db
        .query_one_raw(pg_bound(
            "INSERT INTO sales_contracts (contract_no, contract_name, customer_id, created_by) \
             VALUES ($1, '模板前签约的销售合同', 900001, 1) RETURNING id"
                .to_string(),
            vec![format!("{IT}-LEG-S").into()],
        ))
        .await
        .unwrap_or_else(|e| panic!("不写任何新列的销售合同必须写得进: {e}"))
        .unwrap_or_else(|| panic!("销售合同探针应回读一行 RETURNING id，实得 0 行"));
    let sales_id: i32 = sales_row
        .try_get_by_index(0)
        .unwrap_or_else(|e| panic!("销售合同主键解码失败: {e}"));

    let purchase_row = db
        .query_one_raw(pg_bound(
            "INSERT INTO purchase_contracts (contract_no, contract_name, supplier_id, created_by) \
             VALUES ($1, '模板前签约的采购合同', 900002, 1) RETURNING id"
                .to_string(),
            vec![format!("{IT}-LEG-P").into()],
        ))
        .await
        .unwrap_or_else(|e| panic!("不写任何新列的采购合同必须写得进: {e}"))
        .unwrap_or_else(|| panic!("采购合同探针应回读一行 RETURNING id，实得 0 行"));
    let purchase_id: i32 = purchase_row
        .try_get_by_index(0)
        .unwrap_or_else(|e| panic!("采购合同主键解码失败: {e}"));

    for (table, id) in [
        ("sales_contracts", sales_id),
        ("purchase_contracts", purchase_id),
    ] {
        let any_filled = query_bool_text(
            &db,
            format!(
                "SELECT (contract_market IS NOT NULL OR tax_inclusive IS NOT NULL \
                     OR quality_terms IS NOT NULL OR breach_liability IS NOT NULL \
                     OR dispute_resolution IS NOT NULL OR performance_period IS NOT NULL \
                     OR signed_at IS NOT NULL OR signed_by_user_id IS NOT NULL \
                     OR signature_hash IS NOT NULL OR signature_image_url IS NOT NULL \
                     OR signature_certificate IS NOT NULL OR stamp_tax_amount IS NOT NULL)::text \
                  FROM {table} WHERE id = $1"
            ),
            vec![id.into()],
        )
        .await;
        assert!(
            !any_filled,
            "{table} 的历史行十二枚新列必须全部保持空值，实得非空。迁移回填伪造内容即审计失真"
        );

        expect_rejected(
            &db,
            pg_bound(
                format!("UPDATE {table} SET contract_market = $1 WHERE id = $2"),
                vec![BANNED_MARKET.into(), id.into()],
            ),
            "23514",
            &format!("chk_{table}_contract_market"),
        )
        .await;
    }

    // 词表内值与空值都必须放行，证明 CHECK 不是恒假。
    exec_ok(
        &db,
        pg_bound(
            "UPDATE sales_contracts SET contract_market = 'export', tax_inclusive = true WHERE id = $1"
                .to_string(),
            vec![sales_id.into()],
        ),
        "写词表内的内外销与含税值",
    )
    .await;
    exec_ok(
        &db,
        pg_bound(
            "UPDATE sales_contracts SET contract_market = NULL WHERE id = $1".to_string(),
            vec![sales_id.into()],
        ),
        "把内外销改回空值",
    )
    .await;
    let back_to_null = query_bool_text(
        &db,
        "SELECT (contract_market IS NULL)::text FROM sales_contracts WHERE id = $1".to_string(),
        vec![sales_id.into()],
    )
    .await;
    assert!(back_to_null, "内外销空值必须放行（历史行的既有形态）");
}

// ---------------------------------------------------------------------------
// 判据六 域登记与两侧词表同源
// ---------------------------------------------------------------------------

/// 新域必须以域名登记进迁移台账。缺行即该域从未在此库应用，本文件全部断言无从谈起。
#[tokio::test]
async fn new_domain_is_recorded_in_migration_ledger() {
    let db = setup_test_db().await;

    let got = query_text(
        &db,
        "SELECT version FROM seaql_migrations WHERE version = $1".to_string(),
        vec!["m_contract_template_schema".into()],
    )
    .await;
    assert_eq!(
        got, "m_contract_template_schema",
        "合同模板体系域必须以域名 m_contract_template_schema 记入台账，实得 {got}"
    );
}

/// 八组词表在库侧 CHECK 的现行取值集必须与迁移域内常量逐项相等。
/// 防的漂移 常量改了但库没重建（半态库），或 CHECK 被旁路放宽。
#[tokio::test]
async fn db_check_vocabularies_equal_migration_constants_exactly() {
    let db = setup_test_db().await;

    let set = |tokens: &[&str]| {
        tokens
            .iter()
            .map(|t| (*t).to_string())
            .collect::<BTreeSet<_>>()
    };

    let cases: Vec<(&str, &str, BTreeSet<String>)> = vec![
        (
            "contract_templates",
            "chk_contract_templates_contract_type",
            set(&[
                "domestic_sales",
                "export_sales",
                "fabric_yarn_purchase",
                "accessory_packaging_purchase",
                "outsourcing_processing",
                "dyeing_finishing",
                "labor_employment",
                "freight_transport",
                "warehousing_storage",
                "confidentiality_framework",
            ]),
        ),
        (
            "contract_templates",
            "chk_contract_templates_party_side",
            set(&["seller", "buyer", "both"]),
        ),
        (
            "contract_templates",
            "chk_contract_templates_status",
            set(&["draft", "active", "retired"]),
        ),
        (
            "clause_library",
            "chk_clause_library_status",
            set(&["draft", "active", "retired"]),
        ),
        (
            "contract_template_clauses",
            "chk_ctc_legal_review_status",
            set(&["pending", "approved", "rejected"]),
        ),
        (
            "contract_template_clauses",
            "chk_ctc_applies_when",
            set(&[
                "export",
                "domestic",
                "tax_inclusive",
                "tax_exclusive",
                "custom_made",
                "framework",
            ]),
        ),
        (
            "contract_clauses",
            "chk_cc_contract_kind",
            set(&["sales", "purchase"]),
        ),
        (
            "sales_contracts",
            "chk_sales_contracts_contract_market",
            set(&["domestic", "export"]),
        ),
        (
            "purchase_contracts",
            "chk_purchase_contracts_contract_market",
            set(&["domestic", "export"]),
        ),
    ];

    for (table, constraint, want) in cases {
        let def = check_def(&db, table, constraint).await;
        let got = quoted_tokens(&def);
        assert_eq!(
            got, want,
            "{table}.{constraint} 的现行取值集必须与迁移词表常量逐项相等：\n库侧={got:?}\n期望={want:?}\n定义原文={def}"
        );
    }

    // 条款码闭合集在三处必须同一套，防止模板能引用库里不存在的码、快照能落进库外的码。
    let library_def = check_def(&db, "clause_library", "chk_clause_library_code").await;
    let template_def = check_def(&db, "contract_template_clauses", "chk_ctc_clause_code").await;
    let snapshot_def = check_def(&db, "contract_clauses", "chk_cc_clause_code").await;
    let library_set = quoted_tokens(&library_def);
    assert_eq!(
        library_set,
        quoted_tokens(&template_def),
        "条款库与模板条款项的条款码取值集必须同一套，否则模板能引用条款库里不存在的码"
    );
    assert_eq!(
        library_set,
        quoted_tokens(&snapshot_def),
        "条款库与合同条款快照的条款码取值集必须同一套，否则快照能落进库外的码"
    );
    assert_eq!(
        library_set.len(),
        19,
        "条款码是十六枚必备条款加三枚专章条款的闭合集，实得 {} 枚：{library_set:?}",
        library_set.len()
    );

    // 条款库专章适用范围必须与模板合同类别同源。
    let applies_def = check_def(&db, "clause_library", "chk_clause_library_applies_type").await;
    let type_def = check_def(
        &db,
        "contract_templates",
        "chk_contract_templates_contract_type",
    )
    .await;
    assert_eq!(
        quoted_tokens(&applies_def),
        quoted_tokens(&type_def),
        "条款库专章适用范围与模板合同类别必须同一套十类，库侧定义原文={applies_def}"
    );
}
