//! 契约波次 8 · 价格状态词表 == 真库 DB CHECK **活体契约锁**（`price_vocab_check` 的纸面承诺落地）
//!
//! ## 为什么这个文件必须存在（审计判定的阻塞项 D1）
//! `backend/migration/src/domain/price_vocab_check/mod.rs:19`、`:103-104` 与
//! `backend/src/models/status/sales.rs`（`price_approval` 模块头注释）三处都写着
//! "取值集与 CHECK 逐项相等，由契约测试 `backend/tests/contract_wave8_price_status_parity_test.rs` 锁定"，
//! 而该文件此前**并不存在** ⇒ 这条契约锁是纸面承诺。本文件让它成为事实。
//!
//! ## 真库理由（不是可选，是这类断言的唯一取证场所）
//! 继承 `contract_wave8_customer_type_domain_live_lock_test.rs:18-34` 同一论证：
//! `information_schema.columns.is_nullable / column_default`、`pg_constraint.contype / convalidated /
//! pg_get_constraintdef` 都是**目录里的状态**，只有 PostgreSQL 有；`CHECK 是否真的在拒`
//! 只能靠真库写入取证（sqlite 夹具既没有 pg_constraint，也没有 NOT VALID 语义）。
//! 夹具 `setup_test_db()`（`src/services/test_common.rs:186-190`）缺 `TEST_DATABASE_URL` 或指向
//! sqlite 直接 panic，**不做静默降级** ⇒ 本文件全部断言只有 CI 活库才有结果；
//! 本地只验证编译与格式（`cargo check --all-targets` 哨兵），行为结果待 CI 活体证明——
//! 这是显式声明，不是跳过（同谱系声明见 `contract_wave8_customer_type_domain_live_lock_test.rs:49-52`）。
//!
//! ## 断言清单与各自钉住的缺陷面（任一条被改坏必红）
//! 1. `price_vocab_authority_is_self_consistent_and_lowercase`
//!    ——词表模块内部自洽：常量声明集 == `ALL` 列举集、取值全小写、**文本解析结果 == 运行时常量**
//!    （钉"一边改常量一边忘改 ALL"与"注释与值漂移"；大写 token 会撞逐字符敏感的 CHECK）。
//! 2. `writer_status_set_equals_live_db_check_set_exactly`
//!    ——**集合相等**（不是包含）：DB 实际 CHECK 允许值集 == 该表写入方真实可写状态集。
//!    写入方集合按源码事实推导（`status` 赋值行的 `Set(price_approval::NAME)` +
//!    入参按 `price_approval::ALL.contains(...)` 守卫后透传写入 ⇒ 该表可写集 = ALL），
//!    不是把期望值抄进测试。钉住三种漂移：
//!      · 词表/写入方扩值而 CHECK 未跟上 ⇒ 写入撞 23514 冒裸 500（本批根因同型）；
//!      · CHECK 比写入方宽 ⇒ 旁路脚本可写出从未被业务产生的脏 token；
//!      · 两表被"图省事"合并成同一个并集/交集 ⇒ 按表分钉
//!        （sales={pending,approved,rejected}、
//!        purchase={pending,approved,rejected,inactive}，
//!        基线见迁移头注释 :16-19，扩 rejected 后继见
//!        `price_vocab_extend/m0080_extend_price_status_check_rejected.rs`），
//!        差集必须恰为 `{inactive}`，两侧各自逐 token 相等。
//! 3. `status_column_default_is_pending_and_not_null`
//!    ——目录断言（`is_nullable='NO'`、`column_default` 是 'pending' 字面量）+ **行为回读**
//!    （省略该列 INSERT ⇒ 落库值就是 'pending'，且不再是建表原文的 'ACTIVE'）。
//!    钉住 m0009:249 / m0011:188 的 `DEFAULT 'ACTIVE'` 未被收敛——若只 DROP/ADD 约束而漏掉
//!    `SET DEFAULT`，任何省略 status 的写入都会造出一个 CHECK 放不过、错误归因为 500 的行。
//! 4. `bypass_write_of_out_of_vocab_status_is_refused_by_db`
//!    ——真库旁路直写（裸 SQL，绕开服务层校验）必须被拒，且**按 SQLSTATE + 约束名双归因**
//!    （23514 check_violation / 23502 not_null_violation，照 `customer_type_domain_live_lock:140-158`
//!    取驱动字段，不做 `to_string().contains` 含混匹配）；被拒写入必须**零落库**。
//!    探针含大写 `PENDING`/`APPROVED`（CHECK 逐字符敏感）、历史值 `ACTIVE`、空串、超长拼接值，
//!    以及**跨表不对称探针** `inactive`：销售侧必须拒、采购侧必须收（收/拒反过来即并集/交集漂移）。
//!    每个拒绝探针都配同模板合法值对照，防夹具本身写坏导致拒绝集体假绿。
//!
//! ## id 带与自清
//! `sales_prices`/`purchase_prices` 都是逐用例 TRUNCATE 的业务表（不在 `SEALED_REFERENCE_TABLES`），
//! 本文件只用显式 id 带 **941_0xx（sales）/ 947_0xx（purchase）**（仓内空闲段，
//! 已被占用的 990_1xx/992_0xx/991_0xx 不碰），种子自建自清（依赖夹具 TRUNCATE，不依赖他人留下的行）。
//!
//! ## FK 前提（2026-10 适配；此前本段写"无 FOREIGN KEY、种子无需父行"，该前提已失实）
//! `backend/migration/src/domain/price_fk/mod.rs`（注册在 `migration/src/lib.rs` 迁移链尾）
//! 已给 `sales_prices.product_id/customer_id`、`purchase_prices.product_id/supplier_id`
//! 施加外键 ⇒ **凡是期望成功的写入都必须先种真实父行**：
//! - `products` 属业务表、逐例 TRUNCATE ⇒ 按本文件显式 id 带种 `SEED_PRODUCT_ID` 固定父行；
//! - `suppliers` 属 `SEALED_REFERENCE_TABLES`（夹具不清空、跨例累积）⇒ 不可用显式 id
//!   （第二例起会撞主键），必须自增插入并回读真实主键——形状照
//!   `contract_wave8_price_ref_existence_test.rs::seed_product/seed_supplier`。
//! 词表外拒绝探针与 NULL 探针同样使用真实父行：FK 侧保持干净，23514/23502 的
//! 归因才唯一指向 CHECK/NOT NULL 本身，不依赖"CHECK/NOT NULL 先于 FK 触发"的执行顺序论证。
//! `customer_id` 在本文件所有写入中省略（NULL，标准价合法语义，FK 天然允许）；
//! `created_by` 仍无 FK（price_fk 只覆盖上述四列），维持既有省略形态。

mod test_common;

use bingxi_backend::models::status::sales::price_approval;
use bingxi_backend::models::{product, supplier};
use sea_orm::{
    ActiveModelTrait, ConnectionTrait, DatabaseConnection, DbBackend, DbErr, RuntimeErr, Set,
    Statement, Value,
};
use std::collections::{BTreeMap, BTreeSet};
use test_common::setup_test_db;

/// 销售侧表与其 CHECK 名（migration/src/domain/price_vocab_check/mod.rs:101）
const SALES_TABLE: &str = "sales_prices";
const CHK_SALES: &str = "chk_sales_price_status";
/// 采购侧表与其 CHECK 名（同上 :102）
const PURCHASE_TABLE: &str = "purchase_prices";
const CHK_PURCHASE: &str = "chk_purchase_price_status";

/// 权威词表文件与模块锚点
const AUTHORITY_REL: &str = "src/models/status/sales.rs";
const AUTHORITY_ANCHOR: &str = "pub mod price_approval {";

/// 价格状态写入方登记表（相对 backend/，逐条 file:line 依据见文件头「断言清单」第 2 条）。
/// `quality_inspection_service.rs` 以 `sales_price::{self as price_model}`（:566）别名写
/// `status: Set(price_approval::APPROVED…)`（:632），只 grep `sales_price::ActiveModel` 会漏掉它。
struct PriceWriter {
    file: &'static str,
    table: &'static str,
}
const PRICE_WRITERS: &[PriceWriter] = &[
    PriceWriter {
        file: "src/services/sales_price_service.rs",
        table: SALES_TABLE,
    },
    PriceWriter {
        file: "src/services/quality_inspection_service.rs",
        table: SALES_TABLE,
    },
    PriceWriter {
        file: "src/services/purchase_price_service.rs",
        table: PURCHASE_TABLE,
    },
];

/// PG 对 varchar 列默认值的两种等价文本渲染（同一 `pg_get_expr` 表达式的两种形态，
/// 取值集合仍是恰好 1 个；"默认值真的求值为 pending"由本文件的行为回读用例证明）
const PENDING_DEFAULT_RENDERINGS: [&str; 2] = ["'pending'", "'pending'::character varying"];

/// 销售侧词表外取值探针：历史建表默认值、大写混维、空串、造词、跨表 token
const SALES_BANNED_TOKENS: &[&str] = &["ACTIVE", "PENDING", "APPROVED", "inactive", "", "pending_"];
/// 采购侧词表外取值探针（不含 inactive——它在采购侧合法，见不对称探针）
const PURCHASE_BANNED_TOKENS: &[&str] =
    &["ACTIVE", "PENDING", "APPROVED", "expired", "", "approved "];

/// sales_prices 显式 id 带（941_0xx）
const SALES_ID_DEFAULT_PROBE: i32 = 941_020; // 省略 status ⇒ DEFAULT 必须落 'pending'
const SALES_ID_NULL_PROBE: i32 = 941_021; // 显式 NULL ⇒ 必须撞 NOT NULL(23502)
const SALES_PROBE_BASE: i32 = 941_030; // 词表外拒绝探针基址（每 token 一行）
const SALES_LEGAL_BASE: i32 = 941_050; // 合法值对照基址（与拒绝探针同模板，必须插得进）
/// 逐例种入的**真实**产品父行主键（products 是业务表、夹具逐例 TRUNCATE 后可显式固定 id；
/// price_fk 迁移已给 product_id 施加 FK，父行缺失会撞 23503——见文件头「FK 前提」段）
const SEED_PRODUCT_ID: i32 = 941_900;

/// purchase_prices 显式 id 带（947_0xx）
const PURCHASE_ID_DEFAULT_PROBE: i32 = 947_020;
const PURCHASE_ID_NULL_PROBE: i32 = 947_021;
const PURCHASE_ID_INACTIVE: i32 = 947_022; // inactive 在采购侧必须可写（不对称锁）
const PURCHASE_PROBE_BASE: i32 = 947_030;
const PURCHASE_LEGAL_BASE: i32 = 947_050;
// 供应商父行主键**不设为常量**：`suppliers` 属 `SEALED_REFERENCE_TABLES`（夹具不清空、
// 跨例累积），显式固定 id 从第二个用例起必撞主键 ⇒ 逐例自增插入并回读真实 id（照
// `contract_wave8_price_ref_existence_test.rs::seed_supplier` 形状）

// ---------------------------------------------------------------------------
// 通用助手（裸 SQL 一律走 execute_raw / query_*_raw，不引入第二套连接方式）
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

/// 目录/计数类单列文本读取；查询 0 行返回 `None`（调用方据此断"对象不存在"这一负形态）。
/// 列值为真 NULL 时**显式抛错**而不是塌成空串：那会分不清"对象不存在"与"存在但值为 NULL"。
/// 需要区分 NULL 形态的调用点（如 column_default 缺省）自行 `COALESCE(..., '<NULL>')` 取值。
async fn one_text(db: &DatabaseConnection, sql: &str, col: &str) -> Option<String> {
    let row = db
        .query_one_raw(pg_stmt(sql))
        .await
        .unwrap_or_else(|e| panic!("单值查询失败: {e}\nSQL: {sql}"));
    row.map(|r| {
        r.try_get::<Option<String>>("", col)
            .unwrap_or_else(|e| panic!("列 {col} 解码为文本失败: {e}\nSQL: {sql}"))
            .unwrap_or_else(|| {
                panic!(
                    "列 {col} 是真 NULL（不是空串）——本助手不做塌缩，\
                                      需要判定缺省时请在 SQL 里 COALESCE 出显式哨兵值\nSQL: {sql}"
                )
            })
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

/// 驱动上报的 SQLSTATE（从 `DbErr::Exec/Query(RuntimeErr::SqlxError)` 取，
/// 不做 `to_string().contains(...)` 的含混匹配——任何 FK/类型/长度错误都会让"只判 is_err"假绿）
fn sqlstate_of(err: &DbErr) -> Option<String> {
    match err {
        DbErr::Exec(RuntimeErr::SqlxError(e)) | DbErr::Query(RuntimeErr::SqlxError(e)) => e
            .as_database_error()
            .and_then(|de| sqlx::error::DatabaseError::code(de).map(|c| c.into_owned())),
        other => panic!("期望的是执行期数据库错误，实际是非驱动错误: {other}"),
    }
}

/// 驱动上报的约束名（PG 专用）：把 23514 归因到**本列这一条 CHECK**，
/// 而不是表上任意一条 CHECK / 任意一种失败
fn constraint_of(err: &DbErr) -> Option<String> {
    match err {
        DbErr::Exec(RuntimeErr::SqlxError(e)) | DbErr::Query(RuntimeErr::SqlxError(e)) => e
            .as_database_error()
            .and_then(|de| sqlx::error::DatabaseError::constraint(de).map(|s| s.to_string())),
        other => panic!("期望的是执行期数据库错误，实际是非驱动错误: {other}"),
    }
}

/// 从 `pg_get_constraintdef` 文本里提取全部单引号 token（'pending'::character varying ⇒ pending）
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
    tokens
}

// ---------------------------------------------------------------------------
// 源码侧解析：权威词表 + 写入方可写集（推导，不抄期望值）
// ---------------------------------------------------------------------------

/// 解析 `price_approval` 模块：返回（常量名 → 值）与 `ALL` 列举的常量名集合（不含 ALL 自身）
fn parse_price_approval_authority() -> (BTreeMap<String, String>, BTreeSet<String>) {
    let src = read_repo_file(AUTHORITY_REL);
    let start = src
        .find(AUTHORITY_ANCHOR)
        .unwrap_or_else(|| panic!("{AUTHORITY_REL} 缺少锚点 {AUTHORITY_ANCHOR}"));
    let block = &src[start..];
    let end = block
        .find("\n}")
        .expect("price_approval 模块未闭合（缩进为 0 的右花括号）");
    let block = &block[..end];

    let mut consts: BTreeMap<String, String> = BTreeMap::new();
    for line in block.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("pub const ") {
            let Some((name, tail)) = rest.split_once(": &str = ") else {
                continue;
            };
            let value = tail
                .strip_prefix('"')
                .and_then(|v| v.split('"').next())
                .unwrap_or_else(|| panic!("price_approval::{name} 的取值必须是双引号字面量: {t}"));
            consts.insert(name.to_string(), value.to_string());
        }
    }

    let all_anchor = "pub const ALL: &[&str] = &[";
    let all_start = block
        .find(all_anchor)
        .expect("price_approval 模块缺少 ALL 列举");
    let all_block = &block[all_start + all_anchor.len()..];
    let all_end = all_block.find(']').expect("ALL 列举未闭合");
    let all_names: BTreeSet<String> = all_block[..all_end]
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s != "ALL")
        .collect();

    (consts, all_names)
}

/// 写入方某表**可写进 status 列**的常量名集合（按源码事实推导）：
/// ① `status` 赋值行上出现的 `Set(price_approval::NAME…)`（建单/审批/标准价落库三点）；
/// ② 若文件把外部入参按 `price_approval::ALL.contains(...)` 守卫后再写入（采购侧 PUT 透传），
///    则该 ALL 列举的全部常量对该表都可写。
/// 读取用的 `Column::Status.eq(price_approval::APPROVED)` 一类行（质检联动读 approved 标准价的
/// 过滤条件）**不计入**：它是读侧过滤不是写入，不扩大可写取值集。
fn written_status_names(rel: &str, all_names: &BTreeSet<String>) -> BTreeSet<String> {
    let src = read_repo_file(rel);
    let mut names = BTreeSet::new();
    for line in src.lines() {
        let t = line.trim();
        if t.contains("Set(price_approval::") && (t.contains("status") || t.contains("Status")) {
            let rest = &t[t.find("Set(price_approval::").unwrap() + "Set(price_approval::".len()..];
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_uppercase() || *c == '_')
                .collect();
            if !name.is_empty() && name != "ALL" {
                names.insert(name);
            }
        }
        if t.contains("price_approval::ALL.contains(") {
            names.extend(all_names.iter().cloned());
        }
    }
    names
}

/// 常量名集合 → 权威词表取值集合（名字不在词表内即 panic：说明写入方引用了不存在的常量）
fn map_names_to_values(
    names: &BTreeSet<String>,
    consts: &BTreeMap<String, String>,
) -> BTreeSet<String> {
    names
        .iter()
        .map(|n| {
            consts
                .get(n)
                .unwrap_or_else(|| {
                    panic!("写入方引用了词表外常量名 {n}（{AUTHORITY_REL}::price_approval 未声明）")
                })
                .clone()
        })
        .collect()
}

/// 只读 CHECK 定义文本并抽取单引号取值集（目录/存在性断言见 `db_check_allowed_values`）
async fn check_tokens_raw(db: &DatabaseConnection, table: &str, conname: &str) -> BTreeSet<String> {
    let def = one_text(
        db,
        &format!(
            "SELECT pg_get_constraintdef(c.oid) AS def FROM pg_constraint c \
              WHERE c.conrelid='public.{table}'::regclass AND c.conname='{conname}'"
        ),
        "def",
    )
    .await
    .unwrap_or_else(|| panic!("读不到 {conname} 的定义（{table} 上不存在该约束？）"));
    quoted_tokens(&def)
}

/// 真库 pg_constraint 里该 CHECK 的实际允许值集合（同时校验 contype='c'、convalidated=true、
/// 该表引用 status 列的 CHECK **只有这一条**——多一条宽 CHECK 就是第二套词表）
async fn db_check_allowed_values(
    db: &DatabaseConnection,
    table: &str,
    conname: &str,
) -> BTreeSet<String> {
    let contype = one_text(
        db,
        &format!(
            "SELECT c.contype::text FROM pg_constraint c \
              WHERE c.conrelid='public.{table}'::regclass AND c.conname='{conname}'"
        ),
        "contype",
    )
    .await;
    assert_eq!(
        contype.as_deref(),
        Some("c"),
        "pg_constraint 里必须存在名为 {conname} 且 contype='c'（CHECK）的约束；\n\
         None=约束不存在（price_vocab_check:101-102 未落地/被 down 撤掉/迁移链未注册），\n\
         其它值=约束类型漂移（'f'/'u'…）；实际 contype: {contype:?}"
    );

    let validated = one_text(
        db,
        &format!(
            "SELECT c.convalidated::text FROM pg_constraint c \
              WHERE c.conrelid='public.{table}'::regclass AND c.conname='{conname}'"
        ),
        "convalidated",
    )
    .await;
    assert_eq!(
        validated.as_deref(),
        Some("true"),
        "{conname} 的 convalidated 必须为 true：NOT VALID 的 CHECK 放行存量脏行、\
         只对后续写入生效，那就不是收口（口径同 customer_type_domain_live_lock:332-347）；\n\
         期望 'true' 实际: {validated:?}"
    );

    let other_status_checks = one_i64(
        db,
        &format!(
            "SELECT COUNT(*) AS n FROM pg_constraint c \
              WHERE c.conrelid='public.{table}'::regclass AND c.contype='c' \
                AND c.conname <> '{conname}' AND pg_get_constraintdef(c.oid) LIKE '%status%'"
        ),
        "n",
    )
    .await;
    assert_eq!(
        other_status_checks, 0,
        "{table} 上除 {conname} 外还另有 {other_status_checks} 条涉及 status 的 CHECK——\n\
         同列多 CHECK 会形成第二套词表（本契约锁的\"集合相等\"判据即失去唯一真源），必须先归一"
    );

    let def = one_text(
        db,
        &format!(
            "SELECT pg_get_constraintdef(c.oid) AS def FROM pg_constraint c \
              WHERE c.conrelid='public.{table}'::regclass AND c.conname='{conname}'"
        ),
        "def",
    )
    .await
    .unwrap_or_else(|| panic!("读不到 {conname} 的定义（与前面 contype 查询自相矛盾）"));
    assert!(
        def.contains("status"),
        "{conname} 必须作用在 status 列上，实际定义: {def}"
    );
    quoted_tokens(&def)
}

// ---------------------------------------------------------------------------
// 种子（自建自清：夹具 setup_test_db 已 TRUNCATE 业务表，本文件不依赖他人留下的行）
// ---------------------------------------------------------------------------

/// 种**真实**产品父行（price_fk 的 products.id 参照对象；products 逐例被 TRUNCATE，
/// 显式固定 id 安全。形状照 `contract_wave8_price_ref_existence_test.rs::seed_product`）
async fn seed_product_row(db: &DatabaseConnection, id: i32, name: &str, code: &str) {
    product::ActiveModel {
        id: Set(id),
        name: Set(name.to_string()),
        code: Set(code.to_string()),
        unit: Set("米".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("fabric".to_string()),
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子产品 {id} 插入失败: {e}"));
}

/// 种**真实**供应商父行：`suppliers` 是迁移种子参照表、夹具不清空 ⇒ 不指定显式 id，
/// 自增插入后回读真实主键给采购价目写入使用（形状照
/// `contract_wave8_price_ref_existence_test.rs::seed_supplier`，列对 models/supplier.rs
/// NOT NULL 列逐一核对；supplier_code 逐例取不同值避免跨例累积撞唯一键）
async fn seed_supplier_row(db: &DatabaseConnection, code: &str) -> i32 {
    let ts = supplier::ActiveModel {
        supplier_code: Set(code.to_string()),
        supplier_name: Set("价格词表parity契约锁供应商".to_string()),
        supplier_short_name: Set("parity供".to_string()),
        supplier_type: Set("面料供应商".to_string()),
        credit_code: Set("91330000TEST00000X".to_string()),
        registered_address: Set("测试注册地址".to_string()),
        legal_representative: Set("测试法人".to_string()),
        registered_capital: Set(rust_decimal::Decimal::ZERO),
        establishment_date: Set(chrono::NaiveDate::from_ymd_opt(2020, 1, 1).expect("夹具日期常量")),
        taxpayer_type: Set("一般纳税人".to_string()),
        bank_name: Set("测试银行".to_string()),
        bank_account: Set("6222000000000000".to_string()),
        contact_phone: Set("13800000000".to_string()),
        created_at: Set(chrono::Utc::now().into()),
        updated_at: Set(chrono::Utc::now().into()),
        // is_processor 为 NOT NULL bool 列（models/supplier.rs），显式给值不赌库端默认
        is_processor: Set(false),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子供应商插入失败: {e}"));
    ts.id
}

/// 裸 SQL 往 status 列写指定值（`None` = 显式 NULL）；返回驱动错误供 SQLSTATE 级断言。
/// 走裸 SQL 而非 ORM 的用意：绕开所有服务层入参校验，模拟"旁路/脚本写入"这一 CHECK 要拦的形态。
/// product_id/supplier_id 必须是先种好的**真实**父行 id（price_fk 已施加 FK，见文件头）。
async fn insert_sales_price_status(
    db: &DatabaseConnection,
    id: i32,
    product_id: i32,
    status: Option<&str>,
) -> Result<(), DbErr> {
    let values = vec![
        id.into(),
        product_id.into(),
        Value::String(status.map(|s| s.to_string())),
    ];
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO sales_prices \
         (id,product_id,price,unit,price_type,min_order_qty,effective_date,status) \
         VALUES ($1,$2,'12.340000','米','base',0,'2026-01-01',$3)"
            .to_string(),
        values,
    ))
    .await
    .map(|_| ())
}

/// 省略 status 列的写入 —— DEFAULT 唯一能生效的形态（显式 NULL 走的是 NOT NULL，两条分支不同）
async fn insert_sales_price_omitting_status(
    db: &DatabaseConnection,
    id: i32,
    product_id: i32,
) -> Result<(), DbErr> {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO sales_prices \
         (id,product_id,price,unit,price_type,min_order_qty,effective_date) \
         VALUES ($1,$2,'12.340000','米','base',0,'2026-01-01')"
            .to_string(),
        vec![id.into(), product_id.into()],
    ))
    .await
    .map(|_| ())
}

async fn insert_purchase_price_status(
    db: &DatabaseConnection,
    id: i32,
    product_id: i32,
    supplier_id: i32,
    status: Option<&str>,
) -> Result<(), DbErr> {
    let values = vec![
        id.into(),
        product_id.into(),
        supplier_id.into(),
        Value::String(status.map(|s| s.to_string())),
    ];
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO purchase_prices \
         (id,product_id,supplier_id,price,unit,price_type,min_order_qty,effective_date,status) \
         VALUES ($1,$2,$3,'12.340000','米','base',0,'2026-01-01',$4)"
            .to_string(),
        values,
    ))
    .await
    .map(|_| ())
}

async fn insert_purchase_price_omitting_status(
    db: &DatabaseConnection,
    id: i32,
    product_id: i32,
    supplier_id: i32,
) -> Result<(), DbErr> {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO purchase_prices \
         (id,product_id,supplier_id,price,unit,price_type,min_order_qty,effective_date) \
         VALUES ($1,$2,$3,'12.340000','米','base',0,'2026-01-01')"
            .to_string(),
        vec![id.into(), product_id.into(), supplier_id.into()],
    ))
    .await
    .map(|_| ())
}

async fn status_of(db: &DatabaseConnection, table: &str, id: i32) -> Option<String> {
    let row = db
        .query_one_raw(pg_stmt(&format!(
            "SELECT status FROM {table} WHERE id={id}"
        )))
        .await
        .unwrap_or_else(|e| panic!("回读 {table}(id={id}) 的 status 失败: {e}"));
    let r = row.unwrap_or_else(|| panic!("{table} 里没有 id={id} 的行（探针行未落库？）"));
    r.try_get::<Option<String>>("", "status")
        .unwrap_or_else(|e| panic!("id={id} 的 status 解码失败: {e}"))
}

async fn row_count(db: &DatabaseConnection, table: &str, id: i32) -> i64 {
    one_i64(
        db,
        &format!("SELECT COUNT(*) AS n FROM {table} WHERE id={id}"),
        "n",
    )
    .await
}

// ---------------------------------------------------------------------------
// 1. 权威词表内部自洽 + 文本解析 == 运行时常量
// ---------------------------------------------------------------------------

#[test]
fn price_vocab_authority_is_self_consistent_and_lowercase() {
    let (consts, all_names) = parse_price_approval_authority();

    // 词表声明的常量名集合 == ALL 列举集合（一边改一边忘即红）
    let declared: BTreeSet<String> = consts.keys().cloned().collect();
    assert_eq!(
        declared, all_names,
        "price_approval 常量声明集与 ALL 列举集不一致（一边加常量一边忘进 ALL，\
         服务层取值域就会漏项）；声明={declared:?} ALL={all_names:?}"
    );

    // 取值全小写：DB CHECK 与既有 SQL 过滤都是逐字符敏感，写大写即永远筛不到且撞 CHECK
    for (name, value) in &consts {
        assert_eq!(
            value,
            &value.to_ascii_lowercase(),
            "price_approval::{name} 的取值必须全小写（与 DB CHECK 逐字符一致），实际: {value}"
        );
        assert!(
            !value.is_empty() && value.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "price_approval::{name} 的取值只能是 [a-z_] 形态，实际: {value:?}"
        );
    }

    // 文本解析结果 == 运行时常量（防"注释/文档与真实常量漂移"）
    let runtime_all: BTreeSet<String> = price_approval::ALL.iter().map(|s| s.to_string()).collect();
    let parsed_values: BTreeSet<String> = consts.values().cloned().collect();
    assert_eq!(
        parsed_values, runtime_all,
        "文本解析出的 price_approval 取值集与运行时 `ALL` 不一致（解析锚点失效或常量改动未同步）；\n\
         解析={parsed_values:?} 运行时={runtime_all:?}"
    );
    assert_eq!(
        consts.get("PENDING").map(String::as_str),
        Some(price_approval::PENDING),
        "PENDING 常量与文本解析不一致"
    );
    assert_eq!(
        consts.get("APPROVED").map(String::as_str),
        Some(price_approval::APPROVED),
        "APPROVED 常量与文本解析不一致"
    );
    assert_eq!(
        consts.get("INACTIVE").map(String::as_str),
        Some(price_approval::INACTIVE),
        "INACTIVE 常量与文本解析不一致"
    );
    assert_eq!(
        consts.get("REJECTED").map(String::as_str),
        Some(price_approval::REJECTED),
        "REJECTED 常量与文本解析不一致"
    );
    // 词表恰 4 值（pending/approved/rejected/inactive）；两处 CHECK 与本锁随扩集同步
    assert_eq!(
        runtime_all.len(),
        4,
        "price_approval::ALL 应恰为 pending/approved/rejected/inactive 4 值，实际: {runtime_all:?}"
    );
}

// ---------------------------------------------------------------------------
// 2. 写入方可写集 == 真库 CHECK 允许值集（逐表、集合相等）
// ---------------------------------------------------------------------------

/// 真实写入方特征（本锁登记表与源码扫描的**共用**判据）：文件必须对 `status` 列存在
/// 落库写入形态，而不是仅仅引用 `price_approval::`：
/// - 形态①常量直写：同一行同时含 `Set(price_approval::` 与 `status`/`Status`
///   （建单 PENDING / 审批 APPROVED / 质检联动 APPROVED 三点均属此类）；
/// - 形态②守卫后透传：文件出现 `price_approval::ALL.contains(` 入参守卫，**且**同文件
///   存在含 `Set(` 的 status 写列行（守卫本身不写列不算——那只是读/筛选参数校验）。
/// 背景（CI #4675 族C）：旧判据「文件里出现 `price_approval::` 就算写入方」把两个 handler
/// 的**读/筛选白名单**（`SALES_PRICE_STATUS_FILTER_ALLOWED`、`validate_purchase_price_status_param`）
/// 误当写入方，锁测量的不是它声称测的东西。判据收窄为真实写列特征后，新增写列文件
/// （无论常量直写还是守卫透传）仍会被抓到并因未登记而判红；只加读引用的文件不再误伤。
fn is_status_writer(src: &str) -> bool {
    let mut const_write = false;
    let mut vocab_guard = false;
    let mut status_set_line = false;
    for line in src.lines() {
        let t = line.trim();
        let touches_status = t.contains("status") || t.contains("Status");
        if t.contains("Set(price_approval::") && touches_status {
            const_write = true;
        }
        if t.contains("price_approval::ALL.contains(") {
            vocab_guard = true;
        }
        if t.contains("Set(") && touches_status {
            status_set_line = true;
        }
    }
    const_write || (vocab_guard && status_set_line)
}

/// 写入方登记表完整性：src/ 下任何新增的**真实 status 写入方**（判据见 `is_status_writer`）
/// 都必须先在此登记，否则"词表==CHECK"这条锁会被"新增写入方没人核对约束"绕开（本文件要防的正是这个）。
#[test]
fn price_vocab_writer_registry_is_complete() {
    let root = format!("{}/src", env!("CARGO_MANIFEST_DIR"));
    let mut found: BTreeSet<String> = BTreeSet::new();
    let mut stack: Vec<std::path::PathBuf> = vec![root.into()];
    while let Some(dir) = stack.pop() {
        let rd = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("读取目录 {} 失败: {e}", dir.display()));
        for entry in rd {
            let entry = entry.expect("read_dir 条目读取失败");
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let src = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("读取 {:?} 失败: {e}", path));
            if is_status_writer(&src) {
                let rel = path
                    .strip_prefix(env!("CARGO_MANIFEST_DIR"))
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                found.insert(rel);
            }
        }
    }

    let declared: BTreeSet<String> = PRICE_WRITERS.iter().map(|w| w.file.to_string()).collect();
    assert_eq!(
        found, declared,
        "src/ 下真实 status 写入方（判据=is_status_writer：对 status 列有 Set(...) 落库写入，\
         含常量直写与守卫后透传两形态）的文件集合与本锁的写入方登记表不一致。\n\
         实际写入方={found:?}\n登记表={declared:?}\n\
         新增写入方必须先在 PRICE_WRITERS 登记并核对该表 CHECK 取值集（漏登记=新增写入方无人核对约束，\
         本契约锁即失效）；删除写入点则应同步收缩登记表。只引用 `price_approval::` 做读/筛选白名单\
         的文件（如 handler 参数校验）**不属于**本登记表——写入方的义务是核对自己写进列的值域，\
         读侧过滤由各自的入参校验锁钉。"
    );
}

#[tokio::test]
async fn writer_status_set_equals_live_db_check_set_exactly() {
    let db = setup_test_db().await;
    let (consts, all_names) = parse_price_approval_authority();

    let mut sales_written: BTreeSet<String> = BTreeSet::new();
    let mut purchase_written: BTreeSet<String> = BTreeSet::new();
    for writer in PRICE_WRITERS {
        let names = written_status_names(writer.file, &all_names);
        assert!(
            !names.is_empty(),
            "{} 未解析出任何 status 写入点——写入方形态已改动（或本锁解析锚点失效），\
             必须重新核对后更新本锁，禁止放任为空集让\"相等\"退化成\"CHECK 也空\"",
            writer.file
        );
        let values = map_names_to_values(&names, &consts);
        match writer.table {
            SALES_TABLE => sales_written.extend(values),
            PURCHASE_TABLE => purchase_written.extend(values),
            other => panic!("写入方登记表出现未知表名 {other}"),
        }
    }

    let sales_check = db_check_allowed_values(&db, SALES_TABLE, CHK_SALES).await;
    let purchase_check = db_check_allowed_values(&db, PURCHASE_TABLE, CHK_PURCHASE).await;

    // —— 集合相等（双向；先报数量便于定位）——
    assert_eq!(
        sales_check.len(),
        sales_written.len(),
        "{SALES_TABLE} 的 CHECK 取值数({})与写入方可写集({})数量不等（一边改一边忘）",
        sales_check.len(),
        sales_written.len()
    );
    assert_eq!(
        sales_check, sales_written,
        "{SALES_TABLE}.status 的 DB CHECK 允许值集 != 销售侧写入方可写集（定案应为 \
         pending/approved/rejected，rejected 由审批拒绝写入方产生）。\nDB CHECK={sales_check:?}\n写入方={sales_written:?}\n\
         DB 多于写入方=旁路脚本可写入业务永不产生的脏 token；DB 少于写入方=该写入必撞 23514 冒裸 500。"
    );
    assert_eq!(
        purchase_check.len(),
        purchase_written.len(),
        "{PURCHASE_TABLE} 的 CHECK 取值数({})与写入方可写集({})数量不等（一边改一边忘）",
        purchase_check.len(),
        purchase_written.len()
    );
    assert_eq!(
        purchase_check, purchase_written,
        "{PURCHASE_TABLE}.status 的 DB CHECK 允许值集 != 采购侧写入方可写集（定案应为 \
         pending/approved/rejected/inactive，inactive 有真实生产者：采购价目页 PUT 透传 status）。\n\
         DB CHECK={purchase_check:?}\n写入方={purchase_written:?}"
    );

    // —— 分表钉的不对称性必须真实存在（合并成并集/交集都判红）——
    let only_purchase: BTreeSet<String> =
        purchase_check.difference(&sales_check).cloned().collect();
    let only_sales: BTreeSet<String> = sales_check.difference(&purchase_check).cloned().collect();
    assert_eq!(
        only_purchase,
        [price_approval::INACTIVE.to_string()]
            .into_iter()
            .collect::<BTreeSet<_>>(),
        "两表 CHECK 差集必须恰为 {{inactive}}（分表口径自基线迁移 m_price_vocab_check 起、\
         经扩 rejected 的后继迁移 m_price_vocab_extend 保持不变）；\n\
         实际仅采购侧有={only_purchase:?} 仅销售侧有={only_sales:?}\n\
         取并集⇒销售侧旁路能写 inactive；取交集⇒采购侧真实停用被拒。两者都是已否决的口径。"
    );
    assert!(
        only_sales.is_empty(),
        "销售侧 CHECK 出现了采购侧没有的 token {only_sales:?}——分表口径被改动，须回到定案重判"
    );

    // —— 约束值域不得越出权威词表 ——
    let authority_all = price_approval::ALL;
    for token in sales_check.union(&purchase_check) {
        assert!(
            authority_all.contains(&token.as_str()),
            "DB CHECK 里出现权威词表外的取值 '{token}'（库里造了第二套词表）；\
             词表={authority_all:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// 3. status 列 DEFAULT 收敛为 'pending' + NOT NULL（目录断言 + 行为回读双钉）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn status_column_default_is_pending_and_not_null() {
    let db = setup_test_db().await;
    // FK 前提（price_fk）：本例全部是"期望成功"的写入，必须先种真实父行
    seed_product_row(&db, SEED_PRODUCT_ID, "词表parity面料甲", "PRD-W8PARITY-DEF").await;
    let supplier_id = seed_supplier_row(&db, "SUP-W8PARITY-DEF").await;

    for table in [SALES_TABLE, PURCHASE_TABLE] {
        let nullable = one_text(
            &db,
            &format!(
                "SELECT is_nullable FROM information_schema.columns \
                  WHERE table_schema='public' AND table_name='{table}' AND column_name='status'"
            ),
            "is_nullable",
        )
        .await;
        assert_eq!(
            nullable.as_deref(),
            Some("NO"),
            "{table}.status 的 is_nullable 必须是 'NO'（CHECK 直接写 IN (...) 不留 NULL 放行，\
             见 price_vocab_check 头注释 :21-22）；None=列不存在，其它值=NOT NULL 未真实生效；\
             实际: {nullable:?}"
        );

        // column_default 是 information_schema 里唯一可能为真 NULL 的判定对象（列没有默认值时即 NULL）：
        // 用 COALESCE 显式取出 '<NULL>' 哨兵，让"根本没有默认值"与"默认值是别的 token"两种负形态
        // 在失败信息里可区分，不靠塌缩成空串掩盖。
        let default_raw = one_text(
            &db,
            &format!(
                "SELECT COALESCE(column_default, '<NULL>') AS column_default \
                  FROM information_schema.columns \
                  WHERE table_schema='public' AND table_name='{table}' AND column_name='status'"
            ),
            "column_default",
        )
        .await;
        let default_rendering = default_raw.unwrap_or_else(|| {
            panic!(
            "{table}.status 这一列在 information_schema.columns 里查不到（列不存在或与 m0009:249/\
             m0011:188 建表形态漂移），无法判定默认值"
        )
        });
        assert!(
            PENDING_DEFAULT_RENDERINGS.contains(&default_rendering.as_str()),
            "{table}.status 的 column_default 必须是 'pending' 字面量（允许带/不带 varchar 类型标注 \
             这 2 种 PG 渲染，语义同一个表达式，不是放宽）；\n\
             期望恰为 {PENDING_DEFAULT_RENDERINGS:?} 之一，实际: {default_rendering:?}\n\
             '<NULL>'=该列根本没有 DEFAULT（省略 status 的写入会直接撞 NOT NULL 而死）；\n\
             '''ACTIVE''' 形态=建表原文 DEFAULT 'ACTIVE'（m0009:249/m0011:188）未被收敛，\n\
             省略 status 的写入会造出 CHECK 放不过、错误归因为裸 500 的行；其它 token=库侧默认值与词表漂移。"
        );

        // 行为回读：省略该列 INSERT ⇒ 落库值真为 'pending'（目录断言只证"存在默认值"，
        // "默认值真的求值成 pending"必须由写入证明，同 customer_type_domain_live_lock:31-34）
        let probe_id = if table == SALES_TABLE {
            SALES_ID_DEFAULT_PROBE
        } else {
            PURCHASE_ID_DEFAULT_PROBE
        };
        if table == SALES_TABLE {
            insert_sales_price_omitting_status(&db, probe_id, SEED_PRODUCT_ID)
                .await
                .unwrap_or_else(|e| {
                    panic!("{table} 省略 status 的写入应成功（DEFAULT 兜底）: {e}")
                });
        } else {
            insert_purchase_price_omitting_status(&db, probe_id, SEED_PRODUCT_ID, supplier_id)
                .await
                .unwrap_or_else(|e| {
                    panic!("{table} 省略 status 的写入应成功（DEFAULT 兜底）: {e}")
                });
        }
        let landed = status_of(&db, table, probe_id).await;
        let pending_token = price_approval::PENDING;
        assert_eq!(
            landed.as_deref(),
            Some(pending_token),
            "{table} 省略 status 写入后落库值必须是权威词表的 PENDING（'{pending_token}'），\
             实际: {landed:?}（'ACTIVE'=建表默认值仍在生效；NULL=DEFAULT 不存在）"
        );
        assert_ne!(
            landed.as_deref(),
            Some("ACTIVE"),
            "{table}.status 仍会默认成历史值 'ACTIVE' —— v15/price_vocab_check 的默认值收敛未生效"
        );
    }
}

// ---------------------------------------------------------------------------
// 4. 真库旁路写入：词表外值必被拒（SQLSTATE + 约束名双归因 + 零落库 + 合法值对照）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn bypass_write_of_out_of_vocab_status_is_refused_by_db() {
    let db = setup_test_db().await;
    // FK 前提（price_fk）：拒绝探针（23514/23502）与合法对照/采购 inactive（期望成功）
    // 共用同一批**真实**父行——FK 侧保持干净，SQLSTATE 归因才唯一指向 CHECK/NOT NULL 本身，
    // 不必借道"CHECK/NOT NULL 先于 FK 触发"的顺序论证（见文件头「FK 前提」段）。
    seed_product_row(&db, SEED_PRODUCT_ID, "词表parity面料甲", "PRD-W8PARITY-BYP").await;
    let supplier_id = seed_supplier_row(&db, "SUP-W8PARITY-BYP").await;

    // —— 4a. 两表各自的历史值/大写/空串/造词一律 23514 拒，且点名到本表 CHECK ——
    for (table, conname, banned, base) in [
        (
            SALES_TABLE,
            CHK_SALES,
            SALES_BANNED_TOKENS,
            SALES_PROBE_BASE,
        ),
        (
            PURCHASE_TABLE,
            CHK_PURCHASE,
            PURCHASE_BANNED_TOKENS,
            PURCHASE_PROBE_BASE,
        ),
    ] {
        for (idx, token) in banned.iter().copied().enumerate() {
            let id = base + idx as i32;
            let err = if table == SALES_TABLE {
                insert_sales_price_status(&db, id, SEED_PRODUCT_ID, Some(token))
                    .await
                    .expect_err(&format!(
                        "{table}.status='{token}' 属权威词表外取值，裸 SQL 旁路写入必须被数据库拒绝"
                    ))
            } else {
                insert_purchase_price_status(&db, id, SEED_PRODUCT_ID, supplier_id, Some(token))
                    .await
                    .expect_err(&format!(
                        "{table}.status='{token}' 属权威词表外取值，裸 SQL 旁路写入必须被数据库拒绝"
                    ))
            };
            assert_eq!(
                sqlstate_of(&err).as_deref(),
                Some("23514"),
                "{table} 写 status='{token}' 必须撞 CHECK 成 check_violation（SQLSTATE 23514）；\n\
                 期望 23514 实际 SQLSTATE={:?}\n原始错误: {err}\n\
                 只断 is_err 是假绿：列宽超限(22001)/类型不符(42804)/FK(23503) 都会\"看起来通过\"",
                sqlstate_of(&err),
            );
            assert_eq!(
                constraint_of(&err).as_deref(),
                Some(conname),
                "23514 必须归因到 {conname} 本约束（只断状态码会把表上别的 CHECK 违例当成本锁生效）；\n\
                 实际约束名={:?}\n原始错误: {err}",
                constraint_of(&err)
            );
            assert_eq!(
                row_count(&db, table, id).await,
                0,
                "{table} 被 CHECK 拒掉的 status='{token}' 写入必须零落库（不得留半行）"
            );
        }

        // —— 4b. 同模板合法值对照：证明上面被拒是"取值越界"，不是夹具本身写坏列/漏 NOT NULL 列 ——
        let legal = check_tokens_raw(&db, table, conname).await;
        assert!(
            !legal.is_empty(),
            "{conname} 解析出空取值集合——本用例的\"拒绝\"就失去了对照，判红而非放过"
        );
        for (idx, token) in legal.iter().enumerate() {
            let id = if table == SALES_TABLE {
                SALES_LEGAL_BASE + idx as i32
            } else {
                PURCHASE_LEGAL_BASE + idx as i32
            };
            let res = if table == SALES_TABLE {
                insert_sales_price_status(&db, id, SEED_PRODUCT_ID, Some(token.as_str())).await
            } else {
                insert_purchase_price_status(
                    &db,
                    id,
                    SEED_PRODUCT_ID,
                    supplier_id,
                    Some(token.as_str()),
                )
                .await
            };
            res.unwrap_or_else(|e| {
                panic!("{table} 写合法 status='{token}'（{conname} 允许值）应成功，实际被拒: {e}")
            });
            assert_eq!(
                status_of(&db, table, id).await.as_deref(),
                Some(token.as_str()),
                "{table} 落库值必须逐字符等于写入值（不得被触发器/默认值改写）"
            );
        }

        // —— 4c. 显式 NULL：走 NOT NULL(23502)，不是 CHECK(23514)；两分支各归各因 ——
        let null_id = if table == SALES_TABLE {
            SALES_ID_NULL_PROBE
        } else {
            PURCHASE_ID_NULL_PROBE
        };
        let null_err = if table == SALES_TABLE {
            insert_sales_price_status(&db, null_id, SEED_PRODUCT_ID, None)
                .await
                .expect_err("显式 NULL 必须被 NOT NULL 拒绝")
        } else {
            insert_purchase_price_status(&db, null_id, SEED_PRODUCT_ID, supplier_id, None)
                .await
                .expect_err("显式 NULL 必须被 NOT NULL 拒绝")
        };
        assert_eq!(
            sqlstate_of(&null_err).as_deref(),
            Some("23502"),
            "{table}.status 显式 NULL 必须撞 NOT NULL（23502 not_null_violation），\
             而不是 23514：DEFAULT 只在省略列时生效（本用例 3 已按该分支取证）；\n\
             实际 SQLSTATE={:?}\n原始错误: {null_err}",
            sqlstate_of(&null_err)
        );
        assert_eq!(
            row_count(&db, table, null_id).await,
            0,
            "{table} 被 NOT NULL 拒掉的 NULL 写入必须零落库"
        );
    }

    // —— 4d. 跨表不对称活性锁：inactive 在销售侧被拒、在采购侧被收（并集/交集漂移都判红）——
    let sales_inactive_id = SALES_LEGAL_BASE + 40;
    let err = insert_sales_price_status(
        &db,
        sales_inactive_id,
        SEED_PRODUCT_ID,
        Some(price_approval::INACTIVE),
    )
    .await
    .expect_err("销售侧无 inactive 写入方：旁路写 inactive 必须被 chk_sales_price_status 拒绝");
    assert_eq!(
        sqlstate_of(&err).as_deref(),
        Some("23514"),
        "销售侧写 inactive 必须是 CHECK 违例，实际 SQLSTATE={:?}\n原始错误: {err}",
        sqlstate_of(&err)
    );
    assert_eq!(
        constraint_of(&err).as_deref(),
        Some(CHK_SALES),
        "销售侧 inactive 拒绝必须归因到 {CHK_SALES}，实际={:?}",
        constraint_of(&err)
    );
    assert_eq!(
        row_count(&db, SALES_TABLE, sales_inactive_id).await,
        0,
        "被拒的销售侧 inactive 写入必须零落库"
    );

    insert_purchase_price_status(
        &db,
        PURCHASE_ID_INACTIVE,
        SEED_PRODUCT_ID,
        supplier_id,
        Some(price_approval::INACTIVE),
    )
    .await
    .unwrap_or_else(|e| {
        panic!(
            "采购侧 inactive 有真实生产者（采购价目页 PUT 透传 status），必须可写，实际被拒: {e}"
        )
    });
    assert_eq!(
        status_of(&db, PURCHASE_TABLE, PURCHASE_ID_INACTIVE)
            .await
            .as_deref(),
        Some(price_approval::INACTIVE),
        "采购侧 inactive 应如实落库"
    );
}
