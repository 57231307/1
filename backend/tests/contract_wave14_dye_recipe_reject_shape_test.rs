//! 染色配方「拒绝态」活库形态锁
//!
//! 功能：在真实 PostgreSQL 上钉死 `dye_recipe.status` 的拒绝终态形态——现行 CHECK
//! `chk_dye_recipe_status` 的取值集逐项等于权威词表 `models::status::dye_recipe::ALL`
//! 的五值（含 `rejected`）、拒绝理由专列 `rejected_reason` 真的存在且可空无默认、
//! `status='rejected'` 的行真能写进库并原样读回、以及词表外取值仍被数据库以 23514
//! 拒绝。既有静态锁只解析迁移源码文本，约束的**现行生效定义**与列存在性只能在活库上量。
//!
//! 调用方：`cargo test` 集成用例，经测试夹具连 CI 已跑完整迁移链的 PostgreSQL。
//! 入参：无（夹具注入库连接）。
//! 传给谁：断言的取值集来自写入方权威词表 `models::status::dye_recipe`，与其逐项对拍，
//! 不在此另立第二套取值；负向探针的越界值刻意取词表外字面量，只用于触发数据库拒绝。
//! 存什么：只在 `dye_recipe` 表种/删自建的探针行；目录侧只读 pg_constraint 与
//! information_schema.columns，不改任何 schema。
//! 存哪里：PostgreSQL `public.dye_recipe` 表与其约束、列元数据。
//!
//! 结构说明：夹具对业务表按用例 TRUNCATE，同一集成二进制内的多个用例默认并行、
//! 会相互清表——故本锁把四条判据收在**同一个** `#[tokio::test]` 里顺序执行，从结构上
//! 排除竞态；负向越界探针与正向 `rejected` 写入紧邻于同一库、同一函数内完成。

mod test_common;

use bingxi_backend::models::status::dye_recipe as recipe_status;
use sea_orm::{ConnectionTrait, DbBackend, DbErr, RuntimeErr, Statement};
use std::collections::BTreeSet;
use test_common::setup_test_db;

/// 现行生效 CHECK 的约束名（链尾扩集迁移在同一名字上把四值放宽为五值）。
const CHK_NAME: &str = "chk_dye_recipe_status";
/// 被检查的目标表与拒绝理由专列（information_schema 侧按名定位）。
const TARGET_TABLE: &str = "dye_recipe";
const REJECTED_REASON_COL: &str = "rejected_reason";
/// 正向 rejected 探针行与负向越界探针行各自的唯一 recipe_no（读回与清理都按它定位，
/// 不依赖自增主键；两行都不与既有数据同键，避免读回串形）。
const REJECT_PROBE_NO: &str = "IT-DRREJSHAPE-PROBE";
const BANNED_PROBE_NO: &str = "IT-DRREJSHAPE-BANNED";
/// 正向行的拒绝理由原文（读回须与之逐字相等，证明该列真的承载理由而非被丢弃）。
const REJECT_REASON_TEXT: &str = "色差超出允收范围";
/// 词表外的越界取值（刻意不属于 ANY 合法集，只用于触发数据库 CHECK 拒绝）。
const BANNED_STATUS_VALUE: &str = "not_a_status";

// ---------------------------------------------------------------------------
// 裸 SQL 助手（一律走 execute_raw / query_one_raw，取值以 $n 绑定，不拼串防注入面）
// ---------------------------------------------------------------------------

/// 带绑定值的 PostgreSQL 语句。sql 直接给 `String`（避免 `&format!` 的多余借用），
/// 取值以 `$n` 占位，杜绝把外部串拼进 SQL 文本。
fn pg_bound(sql: String, values: Vec<sea_orm::Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, values)
}

/// 提取文本中全部单引号 token（`'draft'::character varying` ⇒ `draft`）。
/// 引号不闭合直接 panic——解析面失守必须炸，绝不静默产出残缺集合。
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

/// 驱动上报的 SQLSTATE：从 `DbErr::Exec/Query(RuntimeErr::SqlxError)` 取驱动字段，
/// 不做 `to_string().contains(...)` 的含混匹配（任何 FK/类型/长度错误都会让只判
/// is_err 的断言"看起来通过"）。非驱动错误直接 panic。
fn sqlstate_of(err: &DbErr) -> Option<String> {
    match err {
        DbErr::Exec(RuntimeErr::SqlxError(e)) | DbErr::Query(RuntimeErr::SqlxError(e)) => e
            .as_database_error()
            .and_then(|de| sqlx::error::DatabaseError::code(de).map(|c| c.into_owned())),
        other => panic!("期望的是执行期数据库错误，实际是非驱动错误: {other}"),
    }
}

/// 驱动上报的约束名（PG 专用）：把 23514 归因到**本列这一条 CHECK**，
/// 而不是表上任意一条 CHECK 或任意一种失败（只断状态码是假绿来源）。
fn constraint_of(err: &DbErr) -> Option<String> {
    match err {
        DbErr::Exec(RuntimeErr::SqlxError(e)) | DbErr::Query(RuntimeErr::SqlxError(e)) => e
            .as_database_error()
            .and_then(|de| sqlx::error::DatabaseError::constraint(de).map(|s| s.to_string())),
        other => panic!("期望的是执行期数据库错误，实际是非驱动错误: {other}"),
    }
}

// ---------------------------------------------------------------------------
// 活库形态锁（四条判据在同一函数内顺序执行，规避同二进制并行用例相互 TRUNCATE）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dye_recipe_reject_terminal_state_shape_is_db_enforced() {
    let db = setup_test_db().await;

    // 判据一：现行生效 CHECK 的取值集逐项等于权威词表 ALL（不多不少、顺序无关）。
    // 防的漂移：CHECK 被人改回四值（丢 rejected）或加了词表外值——两者集合都不再相等。
    let check_row = db
        .query_one_raw(pg_bound(
            "SELECT contype::text AS ctype, convalidated AS valid, \
                    pg_get_constraintdef(c.oid) AS def \
               FROM pg_constraint c \
              WHERE c.conrelid = 'dye_recipe'::regclass AND c.conname = $1"
                .to_string(),
            vec![CHK_NAME.into()],
        ))
        .await
        .unwrap_or_else(|e| panic!("查询 pg_constraint 失败: {e}"))
        .unwrap_or_else(|| {
            panic!(
                "约束 {CHK_NAME} 不在活库 dye_recipe 上——拒绝态扩集迁移未在该测试库生效，\
                 本锁无从量取现行取值集（检查迁移是否注册并跑进链）"
            )
        });
    let ctype: String = check_row
        .try_get("", "ctype")
        .unwrap_or_else(|e| panic!("contype 解码失败: {e}"));
    let convalidated: bool = check_row
        .try_get("", "valid")
        .unwrap_or_else(|e| panic!("convalidated 解码失败: {e}"));
    let constraint_def: String = check_row
        .try_get("", "def")
        .unwrap_or_else(|e| panic!("pg_get_constraintdef 解码失败: {e}"));

    assert_eq!(
        ctype.as_str(),
        "c",
        "{CHK_NAME} 必须是 CHECK 约束（contype='c'），实得 contype={ctype}"
    );
    assert!(
        convalidated,
        "{CHK_NAME} 必须是已校验生效的约束（convalidated=true）；NOT VALID 形态会对新行漏检，实得 {constraint_def}"
    );

    let db_tokens = quoted_tokens(&constraint_def);
    assert!(
        !db_tokens.is_empty(),
        "从 CHECK 定义解析出空取值集合——解析失守，拒绝继续比对。定义原文={constraint_def}"
    );
    let want: BTreeSet<String> = recipe_status::ALL.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        db_tokens, want,
        "CHECK {CHK_NAME} 取值集必须与权威词表 ALL 逐项相等（不多不少、顺序无关）；\
         实得={db_tokens:?}\n期望={want:?}\n定义原文={constraint_def}"
    );
    assert!(
        db_tokens.contains(recipe_status::REJECTED),
        "CHECK {CHK_NAME} 必须放行 rejected 终态（词表已含、约束须同含）；\
         实得={db_tokens:?}\n定义原文={constraint_def}"
    );

    // 判据二：拒绝理由专列存在、可空、类型 text、无默认（与 sales_orders.rejected_reason 同形）。
    // 断"列真的存在"而非只断查询成功：列缺失时目录查询 0 行，据 None 直接判红。
    let col_row = db
        .query_one_raw(pg_bound(
            "SELECT is_nullable AS nullable, data_type AS dtype, column_default AS cdef \
               FROM information_schema.columns \
              WHERE table_schema = 'public' AND table_name = $1 AND column_name = $2"
                .to_string(),
            vec![TARGET_TABLE.into(), REJECTED_REASON_COL.into()],
        ))
        .await
        .unwrap_or_else(|e| panic!("查询 information_schema.columns 失败: {e}"))
        .unwrap_or_else(|| {
            panic!(
                "列 {TARGET_TABLE}.{REJECTED_REASON_COL} 不存在于活库——拒绝态扩集迁移未加该列，\
                 拒绝理由无处可存（检查迁移是否注册并跑进链）"
            )
        });
    let nullable: String = col_row
        .try_get("", "nullable")
        .unwrap_or_else(|e| panic!("is_nullable 解码失败: {e}"));
    let dtype: String = col_row
        .try_get("", "dtype")
        .unwrap_or_else(|e| panic!("data_type 解码失败: {e}"));
    let col_default: Option<String> = col_row
        .try_get("", "cdef")
        .unwrap_or_else(|e| panic!("column_default 解码失败: {e}"));

    assert_eq!(
        nullable.as_str(),
        "YES",
        "{REJECTED_REASON_COL} 必须可空（历史行未采集理由须为 NULL），实得 is_nullable={nullable}"
    );
    assert_eq!(
        dtype.as_str(),
        "text",
        "{REJECTED_REASON_COL} 必须为 TEXT（data_type='text'），实得 data_type={dtype}"
    );
    assert!(
        col_default.is_none(),
        "{REJECTED_REASON_COL} 不得带默认值（拒绝理由由服务端 trim 非空强制，禁默认造值），\
         实得 column_default={col_default:?}"
    );

    // 判据三：活体正向——写一行 status='rejected' + rejected_reason 必须成功、且原样读回。
    // 这一步同时证明 CHECK 真的放行该终态（写不进=约束被改回四值）与理由列真的承载数据。
    // recipe_name/color_code 为建表即 NOT NULL，须一并提供；created_at/updated_at 走库默认。
    db.execute_raw(pg_bound(
        "INSERT INTO dye_recipe (recipe_no, recipe_name, color_code, status, rejected_reason) \
         VALUES ($1, $2, $3, $4, $5)"
            .to_string(),
        vec![
            REJECT_PROBE_NO.into(),
            "配方拒绝终态锁".into(),
            "CC-DRREJSHAPE".into(),
            recipe_status::REJECTED.into(),
            REJECT_REASON_TEXT.into(),
        ],
    ))
    .await
    .unwrap_or_else(|e| {
        panic!(
            "写 status='rejected'（终态词表值）的 dye_recipe 行必须被 CHECK 放行，\
             却被数据库拒绝=约束未扩到五值或拒绝通道被收紧；\
             错误: {e}\nCHECK 定义原文={constraint_def}"
        )
    });

    let readback = db
        .query_one_raw(pg_bound(
            "SELECT status AS s, rejected_reason AS r FROM dye_recipe WHERE recipe_no = $1"
                .to_string(),
            vec![REJECT_PROBE_NO.into()],
        ))
        .await
        .unwrap_or_else(|e| panic!("按 recipe_no 回读 rejected 行失败: {e}"))
        .unwrap_or_else(|| panic!("回读不到刚种的 rejected 行（写入未落库？）"));
    let status_back: Option<String> = readback
        .try_get("", "s")
        .unwrap_or_else(|e| panic!("读回 status 解码失败: {e}"));
    let reason_back: Option<String> = readback
        .try_get("", "r")
        .unwrap_or_else(|e| panic!("读回 rejected_reason 解码失败: {e}"));
    assert_eq!(
        status_back.as_deref(),
        Some(recipe_status::REJECTED),
        "读回 status 必须逐字等于终态词表值 rejected，实得 {status_back:?}"
    );
    assert_eq!(
        reason_back.as_deref(),
        Some(REJECT_REASON_TEXT),
        "拒绝理由必须按提交原文落 {REJECTED_REASON_COL} 列并可原样读回，实得 {reason_back:?}"
    );

    // 判据四：活体负向——词表外取值必须被数据库以 CHECK 违例（SQLSTATE 23514）拒绝，
    // 且归因到本约束名。证明收紧没被写成"任意值都放行"，也排除别的 CHECK/约束替本锁挡红。
    let banned_err = db
        .execute_raw(pg_bound(
            "INSERT INTO dye_recipe (recipe_no, recipe_name, color_code, status) \
             VALUES ($1, $2, $3, $4)"
                .to_string(),
            vec![
                BANNED_PROBE_NO.into(),
                "配方拒绝终态锁越界探针".into(),
                "CC-DRREJSHAPE".into(),
                BANNED_STATUS_VALUE.into(),
            ],
        ))
        .await
        .err()
        .unwrap_or_else(|| {
            panic!(
                "词表外取值 '{BANNED_STATUS_VALUE}' 必须被 CHECK {CHK_NAME} 拒绝；\
                 写成功=约束被放宽成任意值放行（收紧形同虚设）"
            )
        });
    assert_eq!(
        sqlstate_of(&banned_err).as_deref(),
        Some("23514"),
        "越界写必须撞 CHECK 成 check_violation（SQLSTATE 23514）；\
         实得 SQLSTATE={:?}\n原始错误: {banned_err}",
        sqlstate_of(&banned_err)
    );
    assert_eq!(
        constraint_of(&banned_err).as_deref(),
        Some(CHK_NAME),
        "23514 必须归因到 {CHK_NAME} 本约束（只断状态码会把表上别的 CHECK 在挡红\
         误当本锁生效）；实得约束名={:?}\n原始错误: {banned_err}",
        constraint_of(&banned_err)
    );

    // 收尾自清：只删本用例种的探针行（正向行；越界行未落库一并清以防万一），
    // 不依赖夹具清表、不外泄给后续用例。
    db.execute_raw(pg_bound(
        "DELETE FROM dye_recipe WHERE recipe_no = $1".to_string(),
        vec![REJECT_PROBE_NO.into()],
    ))
    .await
    .unwrap_or_else(|e| panic!("清理正向探针行失败: {e}"));
    let _ = db
        .execute_raw(pg_bound(
            "DELETE FROM dye_recipe WHERE recipe_no = $1".to_string(),
            vec![BANNED_PROBE_NO.into()],
        ))
        .await;
}
