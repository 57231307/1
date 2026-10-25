//! 工资与能耗十三列状态取值 CHECK——**真库活体契约锁**（迁移原文 / 应用词表 / 活库目录三源对撞）
//!
//! ## 只在 CI 执行
//! 本文件连已跑完迁移的真实 PostgreSQL（`setup_test_db` 缺 `TEST_DATABASE_URL` 直接 panic、
//! 禁 sqlite 回退），本地无库跑不了属预期，绝不为本地变绿放宽断言或改读源码文本判据或加 skip。
//! 纸面同源比对（不连库）在 `wage_energy_status_vocabulary_test.rs`，本文件补的是活库目录形态
//! 与真实写入拒绝，纸面测不到 CHECK 是否真在拒、约束名是否真归因、列是否真 NOT NULL 无默认。
//!
//! ## 断言分界（L1 到 L5）
//! - L1 逐值可插：每列词表内每个 token 经旁路裸 SQL 真插一行成功，并按 id 逐字符回读原文。
//! - L2 词表外必被 DB 拒：旁路写词表外值必须由本列的 CHECK 报 SQLSTATE 23514、约束名逐字
//!   归因到自己这条约束、零落库；负样本含大写漂移、首尾空白、空串、外来词，每列配同模板正例。
//! - L3 目录形态：十三列 is_nullable 为 NO 且 column_default 为空（本批约定无 DEFAULT），
//!   pg_constraint 里同名列的 CHECK 恰一条且 contype 为 c 且 convalidated 为真。
//! - L4 双向逐字一致：pg_get_constraintdef 解析集 == m0103 迁移原文解析集 == 运行时词表，
//!   集合相等不是包含，任一侧单改必红。
//! - L5 台账不可回改：seaql_migrations 含本域台账名 m_wage_energy_vocab_check 恰一行。
//!
//! ## 为什么本批不建端点层空壳断言（L6）
//! 本批十三列里 status 与 meter_type 等列没有任何客户端写入通道（handler 的 status 只进列表
//! 过滤不进写参，meter_type 等由 service 校验后落库），无端点可钉。若硬造 400 端点断言即空壳，
//! 故按裁定只在库层钉兜底，端点层待后续含 PUT 透传列的收口域再补。
//!
//! ## 期望值来源防恒真
//! 合法值集合与约束名一律取运行时 `models::status` 词表常量、m0103 迁移原文解析、活库目录读数
//! 三方对撞，测试内不手抄第二套取值清单。夹具旁路 INSERT 的其余 NOT NULL 列按建表原文填
//! 词表内合法值，保证只有所测列的 CHECK 能被触发，约束名归因才唯一。
//!
//! ## id 带
//! 本文件独占 990_8xx 段（避让已登记的 990_1xx、990_3xx、990_6xx、991_0xx、992_0xx）。

mod test_common;

use bingxi_backend::models::status::{
    energy_allocation_basis, energy_meter_status, energy_record_status, energy_recording_method,
    energy_rule_status, energy_type, wage_rate_status, wage_record_status,
};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, RuntimeErr, Statement};
use std::collections::BTreeSet;
use test_common::setup_test_db;

const M0103_REL: &str =
    "migration/src/domain/wage_energy_vocab_check/m0103_apply_wage_energy_vocab_checks.rs";
const LEDGER_NAME: &str = "m_wage_energy_vocab_check";

/// 一列的活体锁配方。
struct ColSpec {
    table: &'static str,
    column: &'static str,
    constraint: &'static str,
    /// 该表全部 NOT NULL 列的合法填充值（含被测列自身的合法默认，测它时替换成绑定参数）。
    fillers: &'static [(&'static str, &'static str)],
    /// 运行时词表（活体锁据此驱动 L1 正例，与 CHECK 值域应逐字相等）。
    vocab: &'static [&'static str],
}

fn pg_stmt(sql: &str) -> Statement {
    Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql.to_string(),
        Vec::<sea_orm::Value>::new(),
    )
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

async fn one_text(db: &DatabaseConnection, sql: &str, col: &str) -> Option<String> {
    let row = db
        .query_one_raw(pg_stmt(sql))
        .await
        .unwrap_or_else(|e| panic!("单值查询失败: {e}\nSQL: {sql}"));
    row.map(|r| {
        r.try_get::<Option<String>>("", col)
            .unwrap_or_else(|e| panic!("列 {col} 解码为文本失败: {e}\nSQL: {sql}"))
            .unwrap_or_else(|| panic!("列 {col} 是真 NULL，判缺省请自行 COALESCE\nSQL: {sql}"))
    })
}

fn sqlstate_of(err: &DbErr) -> String {
    match err {
        DbErr::Exec(RuntimeErr::SqlxError(e)) | DbErr::Query(RuntimeErr::SqlxError(e)) => e
            .as_database_error()
            .and_then(|de| sqlx::error::DatabaseError::code(de).map(|c| c.into_owned()))
            .unwrap_or_else(|| panic!("驱动错误缺 SQLSTATE: {e}")),
        other => panic!("期望执行期数据库错误，实际非驱动错误: {other}"),
    }
}

fn constraint_of(err: &DbErr) -> Option<String> {
    match err {
        DbErr::Exec(RuntimeErr::SqlxError(e)) | DbErr::Query(RuntimeErr::SqlxError(e)) => e
            .as_database_error()
            .and_then(|de| sqlx::error::DatabaseError::constraint(de).map(|s| s.to_string())),
        other => panic!("期望执行期数据库错误，实际非驱动错误: {other}"),
    }
}

/// 提取文本中全部单引号 token，引号不闭合直接 panic。
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

/// 从 m0103 原文解析 `const {name}: &str = "'a','b'";` 的取值集合。
fn parse_allowed_const(src: &str, name: &str) -> BTreeSet<String> {
    let anchor = format!("const {name}: &str =");
    let start = src
        .find(&anchor)
        .unwrap_or_else(|| panic!("m0103 缺少常量锚点 {anchor:?}（取值种子形态已改动）"));
    // 等号后可换行缩进（长字面量会被行宽折到下一行），跳空白后必须紧跟起始引号。
    let rest = src[start + anchor.len()..].trim_start();
    let rest = rest
        .strip_prefix('"')
        .unwrap_or_else(|| panic!("m0103 的 {name} 在锚点后未见字面量起始引号，形态已改动"));
    let end = rest
        .find("\";")
        .unwrap_or_else(|| panic!("m0103 的 {name} 字面量未闭合"));
    let set = quoted_tokens(&rest[..end]);
    assert!(
        !set.is_empty(),
        "m0103 常量 {name} 解析出空集，解析失守拒绝继续比对"
    );
    set
}

/// 从 m0103 原文解析 COLUMN_CHECKS，返回约束名到它绑定的 ALLOWED 常量名的映射。
fn parse_constraint_to_allowed(src: &str) -> Vec<(String, String)> {
    let re = regex::Regex::new(r"(?s)ColumnCheck \{.*?\}").unwrap();
    let con = regex::Regex::new(r#"constraint: "([^"]+)""#).unwrap();
    let allowed = regex::Regex::new(r"allowed: ([A-Z_]+),").unwrap();
    let up_pos = src
        .find("async fn up")
        .unwrap_or_else(|| panic!("m0103 未找到 up 段，无法定位 COLUMN_CHECKS"));
    let region = &src[..up_pos];
    let mut out = Vec::new();
    for m in re.captures_iter(region) {
        let blk = m.get(0).unwrap().as_str();
        let c = con
            .captures(blk)
            .unwrap_or_else(|| panic!("COLUMN_CHECKS 行缺 constraint 字段"))
            .get(1)
            .unwrap()
            .as_str()
            .to_string();
        let al = allowed
            .captures(blk)
            .unwrap_or_else(|| panic!("COLUMN_CHECKS 行缺 allowed 绑定"))
            .get(1)
            .unwrap()
            .as_str()
            .to_string();
        out.push((c, al));
    }
    out
}

/// 旁路裸 SQL 直插，绕开全部应用校验，只有所测列由绑定参数决定，其余 NOT NULL 列填合法值。
async fn bypass_insert(
    db: &DatabaseConnection,
    spec: &ColSpec,
    id: i32,
    token: &str,
) -> Result<(), DbErr> {
    let mut cols: Vec<String> = vec!["id".to_string(), "\"is_deleted\"".to_string()];
    let mut vals: Vec<String> = vec![id.to_string(), "FALSE".to_string()];
    // 单据号列用 id 保证互不冲突（各表 first filler 约定为业务号列）。
    let (no_col, _) = spec.fillers[0];
    cols.push(format!("\"{no_col}\""));
    vals.push(format!("'{no_col}-lck-{id}'"));
    for (c, v) in spec.fillers {
        if *c == no_col || *c == spec.column {
            continue;
        }
        cols.push(format!("\"{c}\""));
        vals.push((*v).to_string());
    }
    cols.push(format!("\"{}\"", spec.column));
    vals.push("$1".to_string());
    let sql = format!(
        "INSERT INTO \"{}\" ({}) VALUES ({})",
        spec.table,
        cols.join(","),
        vals.join(",")
    );
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        vec![token.into()],
    ))
    .await
    .map(|_| ())
}

async fn read_col(db: &DatabaseConnection, spec: &ColSpec, id: i32) -> Option<String> {
    let row = db
        .query_one_raw(pg_stmt(&format!(
            "SELECT \"{}\" AS v FROM \"{}\" WHERE id={id}",
            spec.column, spec.table
        )))
        .await
        .unwrap_or_else(|e| panic!("回读 {}.{}(id={id}) 失败: {e}", spec.table, spec.column));
    row.unwrap_or_else(|| panic!("{} 里没有 id={id} 的行（探针未落库？）", spec.table))
        .try_get::<Option<String>>("", "v")
        .unwrap_or_else(|e| panic!("id={id} 的 {} 解码失败: {e}", spec.column))
}

async fn count_by_id(db: &DatabaseConnection, table: &str, id: i32) -> i64 {
    one_i64(
        db,
        &format!("SELECT COUNT(*) AS n FROM \"{table}\" WHERE id={id}"),
        "n",
    )
    .await
}

/// 十三列配方，fillers 首元素必须是该表的业务号列（会被赋成 id 唯一串）。
fn col_specs() -> Vec<ColSpec> {
    let pwr_fill: &[(&str, &str)] = &[
        ("rate_no", "x"),
        ("process_route_id", "1"),
        ("route_code", "'x'"),
        ("route_name", "'x'"),
        ("wage_type", "'piece'"),
        ("piece_price", "0"),
        ("time_price", "0"),
        ("grade_a_ratio", "1"),
        ("grade_b_ratio", "1"),
        ("grade_c_ratio", "1"),
        ("effective_date", "'2026-01-01'"),
        ("status", "'draft'"),
        ("created_at", "'2026-01-01T00:00:00Z'"),
        ("updated_at", "'2026-01-01T00:00:00Z'"),
    ];
    let wr_fill: &[(&str, &str)] = &[
        ("record_no", "x"),
        ("period_start", "'2026-01-01'"),
        ("period_end", "'2026-01-31'"),
        ("total_workers", "1"),
        ("total_step_records", "1"),
        ("total_qualified_quantity", "0"),
        ("total_duration_minutes", "0"),
        ("total_amount", "0"),
        ("status", "'draft'"),
        ("created_at", "'2026-01-01T00:00:00Z'"),
        ("updated_at", "'2026-01-01T00:00:00Z'"),
    ];
    let em_fill: &[(&str, &str)] = &[
        ("meter_no", "x"),
        ("meter_name", "'x'"),
        ("meter_type", "'water'"),
        ("unit", "'t'"),
        ("current_reading", "0"),
        ("previous_reading", "0"),
        ("unit_price", "0"),
        ("status", "'active'"),
        ("created_at", "'2026-01-01T00:00:00Z'"),
        ("updated_at", "'2026-01-01T00:00:00Z'"),
    ];
    let ec_fill: &[(&str, &str)] = &[
        ("record_no", "x"),
        ("meter_type", "'water'"),
        ("unit", "'t'"),
        ("previous_reading", "0"),
        ("current_reading", "0"),
        ("consumption", "0"),
        ("unit_price", "0"),
        ("total_cost", "0"),
        ("period_start", "'2026-01-01T00:00:00Z'"),
        ("period_end", "'2026-01-02T00:00:00Z'"),
        ("recording_method", "'manual'"),
        ("recorded_at", "'2026-01-01T00:00:00Z'"),
        ("status", "'draft'"),
        ("created_at", "'2026-01-01T00:00:00Z'"),
        ("updated_at", "'2026-01-01T00:00:00Z'"),
    ];
    let ear_fill: &[(&str, &str)] = &[
        ("rule_no", "x"),
        ("rule_name", "'x'"),
        ("meter_type", "'water'"),
        ("allocation_basis", "'by_duration'"),
        ("effective_date", "'2026-01-01'"),
        ("standard_consumption_per_unit", "0"),
        ("status", "'draft'"),
        ("created_at", "'2026-01-01T00:00:00Z'"),
        ("updated_at", "'2026-01-01T00:00:00Z'"),
    ];
    let eal_fill: &[(&str, &str)] = &[
        ("allocation_no", "x"),
        ("period_start", "'2026-01-01T00:00:00Z'"),
        ("period_end", "'2026-01-02T00:00:00Z'"),
        ("meter_type", "'water'"),
        ("allocation_basis", "'by_duration'"),
        ("total_consumption", "0"),
        ("total_cost", "0"),
        ("allocation_basis_value", "0"),
        ("allocation_ratio", "0"),
        ("allocated_consumption", "0"),
        ("allocated_cost", "0"),
        ("status", "'draft'"),
        ("created_at", "'2026-01-01T00:00:00Z'"),
        ("updated_at", "'2026-01-01T00:00:00Z'"),
    ];
    vec![
        ColSpec {
            table: "process_wage_rate",
            column: "status",
            constraint: "chk_process_wage_rate_status",
            fillers: pwr_fill,
            vocab: &[
                wage_rate_status::DRAFT,
                wage_rate_status::ACTIVE,
                wage_rate_status::DISABLED,
            ],
        },
        ColSpec {
            table: "wage_record",
            column: "status",
            constraint: "chk_wage_record_status",
            fillers: wr_fill,
            vocab: &[
                wage_record_status::DRAFT,
                wage_record_status::CONFIRMED,
                wage_record_status::PAID,
                wage_record_status::CANCELLED,
            ],
        },
        ColSpec {
            table: "energy_meter",
            column: "status",
            constraint: "chk_energy_meter_status",
            fillers: em_fill,
            vocab: &[
                energy_meter_status::ACTIVE,
                energy_meter_status::INACTIVE,
                energy_meter_status::MAINTENANCE,
            ],
        },
        ColSpec {
            table: "energy_meter",
            column: "meter_type",
            constraint: "chk_energy_meter_meter_type",
            fillers: em_fill,
            vocab: &[
                energy_type::WATER,
                energy_type::ELECTRICITY,
                energy_type::STEAM,
                energy_type::GAS,
                energy_type::COMPRESSED_AIR,
            ],
        },
        ColSpec {
            table: "energy_consumption_record",
            column: "status",
            constraint: "chk_energy_consumption_record_status",
            fillers: ec_fill,
            vocab: &[
                energy_record_status::DRAFT,
                energy_record_status::CONFIRMED,
                energy_record_status::CANCELLED,
            ],
        },
        ColSpec {
            table: "energy_consumption_record",
            column: "recording_method",
            constraint: "chk_energy_consumption_record_recording_method",
            fillers: ec_fill,
            vocab: &[
                energy_recording_method::MANUAL,
                energy_recording_method::IOT,
                energy_recording_method::AUTO_CALC,
            ],
        },
        ColSpec {
            table: "energy_consumption_record",
            column: "meter_type",
            constraint: "chk_energy_consumption_record_meter_type",
            fillers: ec_fill,
            vocab: &[
                energy_type::WATER,
                energy_type::ELECTRICITY,
                energy_type::STEAM,
                energy_type::GAS,
                energy_type::COMPRESSED_AIR,
            ],
        },
        ColSpec {
            table: "energy_allocation_rule",
            column: "status",
            constraint: "chk_energy_allocation_rule_status",
            fillers: ear_fill,
            vocab: &[
                energy_rule_status::DRAFT,
                energy_rule_status::ACTIVE,
                energy_rule_status::DISABLED,
            ],
        },
        ColSpec {
            table: "energy_allocation_rule",
            column: "allocation_basis",
            constraint: "chk_energy_allocation_rule_allocation_basis",
            fillers: ear_fill,
            vocab: &[
                energy_allocation_basis::BY_DURATION,
                energy_allocation_basis::BY_OUTPUT,
                energy_allocation_basis::BY_EQUIPMENT,
                energy_allocation_basis::BY_WORKSHOP,
            ],
        },
        ColSpec {
            table: "energy_allocation_rule",
            column: "meter_type",
            constraint: "chk_energy_allocation_rule_meter_type",
            fillers: ear_fill,
            vocab: &[
                energy_type::WATER,
                energy_type::ELECTRICITY,
                energy_type::STEAM,
                energy_type::GAS,
                energy_type::COMPRESSED_AIR,
            ],
        },
        ColSpec {
            table: "energy_allocation_record",
            column: "status",
            constraint: "chk_energy_allocation_record_status",
            fillers: eal_fill,
            vocab: &[
                energy_record_status::DRAFT,
                energy_record_status::CONFIRMED,
                energy_record_status::CANCELLED,
            ],
        },
        ColSpec {
            table: "energy_allocation_record",
            column: "allocation_basis",
            constraint: "chk_energy_allocation_record_allocation_basis",
            fillers: eal_fill,
            vocab: &[
                energy_allocation_basis::BY_DURATION,
                energy_allocation_basis::BY_OUTPUT,
                energy_allocation_basis::BY_EQUIPMENT,
                energy_allocation_basis::BY_WORKSHOP,
            ],
        },
        ColSpec {
            table: "energy_allocation_record",
            column: "meter_type",
            constraint: "chk_energy_allocation_record_meter_type",
            fillers: eal_fill,
            vocab: &[
                energy_type::WATER,
                energy_type::ELECTRICITY,
                energy_type::STEAM,
                energy_type::GAS,
                energy_type::COMPRESSED_AIR,
            ],
        },
    ]
}

// ---------------------------------------------------------------------------
// L1 逐值可插，词表内每个 token 旁路直插成功并按 id 逐字符回读
// ---------------------------------------------------------------------------
#[tokio::test]
async fn l1_each_vocab_value_inserts_and_reads_back_verbatim() {
    let db = setup_test_db().await;
    let specs = col_specs();
    let mut id = 990_800_i32;
    for spec in &specs {
        for token in spec.vocab {
            bypass_insert(&db, spec, id, token)
                .await
                .unwrap_or_else(|e| {
                    panic!(
                        "合法值 '{token}' 插入 {}.{} 必须成功: {e}",
                        spec.table, spec.column
                    )
                });
            let landed = read_col(&db, spec, id).await;
            assert_eq!(
                landed.as_deref(),
                Some(*token),
                "{}.{} 落库值必须逐字符等于写入值（写链任何归一即红）：期望 '{token}' 实际 {landed:?}",
                spec.table,
                spec.column
            );
            id += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// L2 词表外必被本列 CHECK 拒，23514 且约束名逐字归因并零落库
// ---------------------------------------------------------------------------
#[tokio::test]
async fn l2_out_of_vocab_rejected_by_own_check_with_zero_write() {
    let db = setup_test_db().await;
    let specs = col_specs();
    let mut id = 990_820_i32;
    for spec in &specs {
        let first = spec.vocab[0];
        // 负样本，大写漂移、首尾空白混入、空串、外来词
        let negatives: Vec<String> = vec![
            first.to_uppercase(),
            format!("{first} "),
            format!(" {first}"),
            String::new(),
            "__bogus__".to_string(),
        ];
        for token in &negatives {
            let err = bypass_insert(&db, spec, id, token)
                .await
                .err()
                .unwrap_or_else(|| {
                    panic!(
                        "词表外值 '{token}' 插入 {}.{} 竟成功，库层兜底失效",
                        spec.table, spec.column
                    )
                });
            assert_eq!(
                sqlstate_of(&err),
                "23514",
                "{}.{} 写 '{token}' 必须撞 CHECK 成 check_violation 23514，实得 {}，原始 {err}",
                spec.table,
                spec.column,
                sqlstate_of(&err)
            );
            assert_eq!(
                constraint_of(&err).as_deref(),
                Some(spec.constraint),
                "23514 必须归因到本列约束 {}（只断状态码会把别的 CHECK 在挡误当本锁），实得 {:?}",
                spec.constraint,
                constraint_of(&err)
            );
            assert_eq!(
                count_by_id(&db, spec.table, id).await,
                0,
                "被 CHECK 拒掉的 '{token}' 必须零落库（单语句原子失败不留半行）"
            );
            id += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// L3 目录形态，NOT NULL 且无默认且同名 CHECK 恰一条且 contype c 且 convalidated
// ---------------------------------------------------------------------------
#[tokio::test]
async fn l3_columns_are_not_null_no_default_with_single_validated_check() {
    let db = setup_test_db().await;
    for spec in col_specs() {
        let nullable = one_text(
            &db,
            &format!(
                "SELECT is_nullable FROM information_schema.columns \
                  WHERE table_schema='public' AND table_name='{}' AND column_name='{}'",
                spec.table, spec.column
            ),
            "is_nullable",
        )
        .await
        .unwrap_or_else(|| {
            panic!(
                "information_schema 查不到 {}.{}（建表漂移）",
                spec.table, spec.column
            )
        });
        assert_eq!(
            nullable, "NO",
            "{}.{} 的 is_nullable 必须是 NO，实得 {}",
            spec.table, spec.column, nullable
        );

        let default_raw = one_text(
            &db,
            &format!(
                "SELECT COALESCE(column_default, '<NONE>') AS column_default \
                  FROM information_schema.columns \
                  WHERE table_schema='public' AND table_name='{}' AND column_name='{}'",
                spec.table, spec.column
            ),
            "column_default",
        )
        .await
        .unwrap_or_else(|| {
            panic!(
                "information_schema 查不到 {}.{}（建表漂移）",
                spec.table, spec.column
            )
        });
        assert_eq!(
            default_raw, "<NONE>",
            "{}.{} 的 column_default 必须为空（本批约定无默认），实得 {}",
            spec.table, spec.column, default_raw
        );

        let con_meta = one_i64(
            &db,
            &format!(
                "SELECT COUNT(*) AS n FROM pg_constraint WHERE conrelid='{}'::regclass \
                  AND conname='{}' AND contype='c' AND convalidated",
                spec.table, spec.constraint
            ),
            "n",
        )
        .await;
        assert_eq!(
            con_meta, 1,
            "{} 必须是 contype='c' 且 convalidated 的恰一条（0=未注册或被 down 撤或 NOT VALID，>1=同名重复），实得 {}",
            spec.constraint, con_meta
        );

        let other_checks = one_i64(
            &db,
            &format!(
                "SELECT COUNT(*) AS n FROM pg_constraint c WHERE c.conrelid='{}'::regclass \
                  AND c.contype='c' AND c.conname <> '{}' \
                  AND pg_get_constraintdef(c.oid) LIKE '%{}%'",
                spec.table, spec.constraint, spec.column
            ),
            "n",
        )
        .await;
        assert_eq!(
            other_checks, 0,
            "{}.{} 上除 {} 外另有 {} 条涉及该列的 CHECK，同列多 CHECK 即第二套词表",
            spec.table, spec.column, spec.constraint, other_checks
        );
    }
}

// ---------------------------------------------------------------------------
// L4 三源对撞，活库 CHECK 取值集 == m0103 原文解析集 == 运行时词表，集合相等非包含
// ---------------------------------------------------------------------------
#[tokio::test]
async fn l4_live_check_set_equals_migration_source_equals_runtime_vocab() {
    let db = setup_test_db().await;
    let mig_src = std::fs::read_to_string(format!("{}/{}", env!("CARGO_MANIFEST_DIR"), M0103_REL))
        .expect("读取 m0103 迁移原文失败");
    let con_to_allowed = parse_constraint_to_allowed(&mig_src);

    for spec in col_specs() {
        let runtime: BTreeSet<String> = spec.vocab.iter().map(|t| t.to_string()).collect();

        // 迁移原文侧，按约束名找它绑定的 ALLOWED 常量再解析
        let allowed_name = con_to_allowed
            .iter()
            .find(|(c, _)| c == spec.constraint)
            .unwrap_or_else(|| panic!("m0103 COLUMN_CHECKS 未覆盖约束 {}", spec.constraint))
            .1
            .clone();
        let from_migration = parse_allowed_const(&mig_src, &allowed_name);

        // 活库侧
        let def = one_text(
            &db,
            &format!(
                "SELECT pg_get_constraintdef(c.oid) AS def FROM pg_constraint c \
                  WHERE c.conrelid='{}'::regclass AND c.conname='{}'",
                spec.table, spec.constraint
            ),
            "def",
        )
        .await
        .unwrap_or_else(|| panic!("读不到 {} 的定义", spec.constraint));
        let from_live = quoted_tokens(&def);

        assert_eq!(
            from_live, from_migration,
            "活库 CHECK 取值集 != m0103 迁移原文解析集（迁移未注册、down 半态、旁路又加约束都会红）：\n活库={from_live:?}\n迁移={from_migration:?}"
        );
        assert_eq!(
            from_live, runtime,
            "活库 CHECK 取值集 != 运行时词表（逐元素相等非包含，任一侧单改必红）：\n活库={from_live:?}\n词表={runtime:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// L5 已应用序不可回改，迁移台账含本域恰一行
// ---------------------------------------------------------------------------
#[tokio::test]
async fn l5_migration_ledger_contains_this_domain() {
    let db = setup_test_db().await;
    let n = one_i64(
        &db,
        // 迁移台账的迁移名落在 version 列（该列同时是主键，故同名重复物理上不可能），
        // 按 name 过滤会让整条计数 SQL 直接 42703 失红，取不到任何计数。
        &format!(
            "SELECT COUNT(*) AS n FROM \"seaql_migrations\" WHERE version='{}'",
            LEDGER_NAME
        ),
        "n",
    )
    .await;
    assert_eq!(
        n, 1,
        "seaql_migrations 台账必须恰含一行 {LEDGER_NAME}（0=本域未注册或未应用，>1=台账重复），实得 {n}"
    );
    // 台账非空防线，本域应用前迁移链已建表
    let total = one_i64_sql_dummy(&db).await;
    assert!(total > 0, "迁移台账总行数必须大于 0，否则本域真库断言空转");
}

async fn one_i64_sql_dummy(db: &DatabaseConnection) -> i64 {
    one_i64(db, "SELECT COUNT(*) AS n FROM \"seaql_migrations\"", "n").await
}
