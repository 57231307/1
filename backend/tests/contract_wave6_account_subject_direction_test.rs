//! 契约波次 6 · 科目余额方向词表三端同源与写入白名单（任务 #198）
//!
//! 锁定的根因：
//! 1. **比较点认中文、写入方是英文**：`account_subjects.balance_direction` 的写入方权威
//!    词表是英文 debit/credit（前端两处科目 Tab、m0006 DDL `DEFAULT 'debit'`、迁移种子
//!    28 行三端同源），而 5 处比较点（account_subject_service.rs ×2、voucher_ops/
//!    balance.rs ×3）只比较「借」⇒ 绝大多数英文 debit 行落入贷方分支，科目余额与
//!    试算平衡方向系统性反号。修复形态：全部改绑 `models::status::account_subject`
//!    常量（含 `unwrap_or` 默认值）。
//! 2. **服务层透传不校验**：create/update 原样落库任意 token，是「四值并存」根因。
//!    修复形态：写入前白名单拒收 debit/credit 之外取值 = 400 VALIDATION_ERROR
//!    （ValidationErrorDisplayable：文案只复述用户自己提交的字段与公开取值规则，
//!    满足 error.rs 保密分层 → 可外显）。
//! 3. **存量中文数据**：m0007 归一迁移（借→debit、贷→credit），备份表精确回退，幂等。
//!
//! 覆盖策略（分层，路线一真库化：全部真实 PostgreSQL 执行，无 #[ignore]；
//! 表结构唯一来源 = backend/migration，#4669 判责）：
//! - 生产服务行为锁：`AccountSubjectService::refresh_balance` 在真表上按 debit/credit
//!   两条方向各自验证期末余额符号落位（正向断言）；NULL 方向默认 debit 语义保持。
//! - 纯函数行为锁：`VoucherService::compute_ending_balance`（#198 起 pub，仅作测试缝）
//!   借贷两条 + 负值翻向 + 中文 token 不再命中借方分支的反例锁。
//! - 白名单行为锁：中文「借」create 被 400/VALIDATION_ERROR 拒且零新增写入；「贷」update
//!   被拒且存量行不变；大小写变体同样拒（不做静默归一）；"debit" 正常落库。
//!   account_subjects 属迁移播种且不清空的参照表（m0006 种子 28 行），故种子一律用
//!   W6DIR/W6M198 前缀自建行 + 按自建行断言，"零写入"以**行数增量**表达，
//!   不声称全表为空。
//! - 迁移归一行为锁（真库转正）：直接引用 m0007 的 SQL 常量在真实 account_subjects 上
//!   对本用例自建行执行，逐值断言映射、NULL 不动、重放幂等、down 按备份表精确回退；
//!   备份表工件先删后建自清洁，与活库层 rls_dept_user_sync_live_test 的备份回退同型先例。
//!   （原「sqlite 可真跑」前提随路线一作废：真库化后不存在 sqlite 可验性要求。）
//! - 禁回潮源码扫描（include_str!）：比较点不得再出现带引号中文「借/贷」字面量，必须
//!   引用 status::account_subject 常量；词表唯一来源仍在 models/status/finance.rs。

mod test_common;

use std::sync::Arc;

use bingxi_backend::models::account_subject;
use bingxi_backend::models::status::account_subject as subject_status;
use bingxi_backend::models::status::voucher as voucher_status;
use bingxi_backend::models::{voucher, voucher_item};
use bingxi_backend::services::account_subject_service::{
    AccountSubjectService, CreateSubjectRequest, UpdateSubjectRequest,
};
use bingxi_backend::services::voucher_service::VoucherService;
use bingxi_backend::utils::error::AppError;
use chrono::{NaiveDate, Utc};
use migration::domain::system::m0007_normalize_account_subject_balance_direction as m198;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DbBackend, EntityTrait,
    PaginatorTrait, QueryFilter, QueryResult, Statement, Value,
};

// ===========================================================================
// 公共辅助
// ===========================================================================

/// 真库连接：已迁移 + 清业务表；account_subjects/vouchers/voucher_items 的真表列
/// （DECIMAL/DEFAULT/FK）全部由迁移提供，测试不再自建 sqlite 同构表。
async fn live_db() -> sea_orm::DatabaseConnection {
    test_common::setup_test_db().await
}

async fn exec_sql(db: &sea_orm::DatabaseConnection, sql: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql.to_string(),
        Vec::<Value>::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("迁移语句执行失败: {e}\nSQL: {sql}"));
}

fn col_str_opt(row: &QueryResult, idx: usize) -> Option<String> {
    row.try_get_by_index::<Option<String>>(idx)
        .unwrap_or_else(|e| panic!("第 {idx} 列应可解码为 Option<String>: {e}"))
}

/// 只保留"代码 + 字符串字面量"：整行注释（`//`、`///`、`//!`）逐行剔除。
/// 供本文件末尾的禁回潮源码扫描使用——禁项/必备项都必须只看执行体：
/// 被锁文件里的**说明性注释**（"不用 DO $$ 等 PG 专有形态"这类自我约束声明、
/// 或"历史中文「借/贷」一律拒绝"这类口径复述）不是代码，按原文判会假判违例，
/// 反过来也可能把注释里复述的实现当成"已接线"而假判通过。
/// 按行处理而不做字符级扫描：迁移 SQL 是跨行 raw string，单行引号配平会把 SQL 里的
/// 引号当字符串起止、进而吃掉真代码文本；宁少剥（行尾尾注释/块注释不动）不可错剥。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 隔离沙箱库：CI 里"已建库但没跑迁移"的第二只 PostgreSQL（TEST_EMPTY_DATABASE_URL）。
/// 本节锁的是 m0007 三条 SQL 常量的 **语义**（CASE 映射 / NOT EXISTS 幂等 /
/// 备份表精确回退），只触达 id 与 balance_direction 两列；在沙箱库里建一张只含
/// 这两列的表是"给定输入验证变换"，不是拿假表冒充生产 schema（真表形状由
/// live_db() 那批用例覆盖）。用沙箱库而非已迁移库的原因有两个：① account_subjects
/// 是迁移播种且不清空的参照表，跑归一/回退会污染同分片后续用例的科目种子；
/// ② 沙箱库不属公共迁移库，天然与并发用例隔离。
async fn sandbox_db() -> sea_orm::DatabaseConnection {
    test_common::connect_empty_schema_db().await
}

fn col_i64(row: &QueryResult, idx: usize) -> i64 {
    row.try_get_by_index::<i64>(idx)
        .unwrap_or_else(|e| panic!("第 {idx} 列应可解码为 i64（COUNT/BIGINT 列）: {e}"))
}

fn now() -> chrono::DateTime<Utc> {
    Utc::now()
}

/// 种子科目：全字段 ActiveModel 插入（Decimal/时间戳由 SeaORM 原生编码，
/// 不手拼 raw SQL）。code 必须带 W6DIR 前缀——account_subjects 是迁移播种
/// （m0006 种子 28 行，code UNIQUE）且不清空的参照表，真码 1001/2202/1403 已存在，
/// 直接复用会撞 UNIQUE（这正是 #4669 判责里该族种子失败的下一形态）。
async fn seed_subject(
    db: &sea_orm::DatabaseConnection,
    code: &str,
    direction: Option<&str>,
    initial_debit: Decimal,
    initial_credit: Decimal,
) -> i32 {
    let inserted = account_subject::ActiveModel {
        code: Set(format!("W6DIR{code}")),
        name: Set(format!("科目-{code}")),
        level: Set(1),
        full_code: Set(Some(format!("W6DIR{code}"))),
        balance_direction: Set(direction.map(|s| s.to_string())),
        initial_balance_debit: Set(initial_debit),
        initial_balance_credit: Set(initial_credit),
        status: Set("active".to_string()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子科目 {code} 插入失败: {e}"));
    inserted.id
}

/// 本用例专属 code 后缀（参照表不清空且 code UNIQUE，跨用例共用固定码会在同一
/// CI 分片里互相撞约束——分片是逐用例散列分配的，不能假设"另一条用例不在这片"）。
fn unique_suffix() -> i64 {
    Utc::now()
        .timestamp_nanos_opt()
        .expect("测试环境时间戳必须可用")
}

/// 按 code 精确计数——"零新增行"判据的唯一可靠口径：参照表基数由迁移播种决定，
/// 断全表行数等于某个数字（无论 1 还是当前实际值）都会把真实的新增行缺陷或环境差异
/// 混进判定（见本文件头 §覆盖策略与 test_common.rs:26-28 的参照表说明）。
async fn count_rows_by_code(db: &sea_orm::DatabaseConnection, code: &str) -> u64 {
    account_subject::Entity::find()
        .filter(account_subject::Column::Code.eq(code))
        .count(db)
        .await
        .unwrap_or_else(|e| panic!("按 code {code} 计数失败: {e}"))
}

/// 种子已过账凭证 + 单分录（期间 2096-12，与 refresh_balance 入参对齐；
/// subject_code 传 seed_subject 的**完整真 code**（含 W6DIR 前缀））
async fn seed_posted_voucher_item(
    db: &sea_orm::DatabaseConnection,
    subject_code: &str,
    debit: Decimal,
    credit: Decimal,
) {
    let v = voucher::ActiveModel {
        voucher_no: Set(format!("W198-{subject_code}")),
        voucher_type: Set("记".to_string()),
        voucher_date: Set(NaiveDate::from_ymd_opt(2096, 12, 5).expect("测试日期必须合法")),
        status: Set(voucher_status::VOUCHER_POSTED.to_string()),
        created_by: Set(1), // vouchers.created_by 无库级外键（m0006:120 仅列声明）
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("种子凭证插入失败");
    voucher_item::ActiveModel {
        voucher_id: Set(v.id),
        line_no: Set(1),
        subject_code: Set(subject_code.to_string()),
        subject_name: Set(format!("科目-{subject_code}")),
        debit: Set(debit),
        credit: Set(credit),
        created_at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("种子凭证分录插入失败");
}

// ===========================================================================
// 1) 生产服务行为锁：refresh_balance 按 debit/credit 方向计算期末余额（正向断言）
// ===========================================================================

/// 修复前形态：英文 'debit' 行被 `== "借"` 判否落入贷方分支，
/// 期初借 100 + 借 30 - 贷 10 会被算成负差反向挂账。修复后必须 (120, 0)。
#[tokio::test]
async fn refresh_balance_debit_subject_ends_on_debit_side() {
    let db = Arc::new(live_db().await);
    let id = seed_subject(
        &db,
        "1001",
        Some(subject_status::DIRECTION_DEBIT),
        dec!(100),
        Decimal::ZERO,
    )
    .await;
    seed_posted_voucher_item(&db, "W6DIR1001", dec!(30), dec!(10)).await;

    let svc = AccountSubjectService::new(db.clone());
    let updated = svc
        .refresh_balance(id, "2096-12")
        .await
        .expect("refresh_balance 真库执行失败");

    assert_eq!(
        updated.current_period_debit,
        dec!(30),
        "本期借方发生额应聚合 30"
    );
    assert_eq!(
        updated.current_period_credit,
        dec!(10),
        "本期贷方发生额应聚合 10"
    );
    assert_eq!(
        updated.ending_balance_debit,
        dec!(120),
        "debit 科目期末=期初借100+借30-贷10=120，必须挂借方（=0 即修复前反号形态）"
    );
    assert_eq!(
        updated.ending_balance_credit,
        Decimal::ZERO,
        "余额为正应同向挂账"
    );
}

/// credit 科目：期初贷 50 + 本期贷 20 → 期末 (0, 70)
#[tokio::test]
async fn refresh_balance_credit_subject_ends_on_credit_side() {
    let db = Arc::new(live_db().await);
    let id = seed_subject(
        &db,
        "2202",
        Some(subject_status::DIRECTION_CREDIT),
        Decimal::ZERO,
        dec!(50),
    )
    .await;
    seed_posted_voucher_item(&db, "W6DIR2202", Decimal::ZERO, dec!(20)).await;

    let svc = AccountSubjectService::new(db.clone());
    let updated = svc
        .refresh_balance(id, "2096-12")
        .await
        .expect("refresh_balance 真库执行失败");

    assert_eq!(
        updated.ending_balance_credit,
        dec!(70),
        "credit 科目期末=期初贷50+贷20=70，必须挂贷方"
    );
    assert_eq!(updated.ending_balance_debit, Decimal::ZERO);
}

/// 方向缺失（NULL）默认按 debit 处理——换常量后原「借」默认值语义保持不漂移
#[tokio::test]
async fn refresh_balance_null_direction_defaults_to_debit() {
    let db = Arc::new(live_db().await);
    let id = seed_subject(&db, "1403", None, dec!(100), Decimal::ZERO).await;
    seed_posted_voucher_item(&db, "W6DIR1403", dec!(30), dec!(10)).await;

    let svc = AccountSubjectService::new(db.clone());
    let updated = svc
        .refresh_balance(id, "2096-12")
        .await
        .expect("refresh_balance 真库执行失败");
    assert_eq!(
        updated.ending_balance_debit,
        dec!(120),
        "NULL 方向必须与 DIRECTION_DEBIT 同分支"
    );
    assert_eq!(updated.ending_balance_credit, Decimal::ZERO);
}

// ===========================================================================
// 2) 纯函数行为锁：voucher_ops/balance.rs 的期末余额方向分支（过账/冲销共用）
// ===========================================================================

#[test]
fn voucher_compute_ending_balance_binds_english_vocabulary() {
    // debit：0 + 100 - 40 = 60 → 挂借方
    let (d, c) = VoucherService::compute_ending_balance(
        subject_status::DIRECTION_DEBIT,
        Decimal::ZERO,
        Decimal::ZERO,
        dec!(100),
        dec!(40),
    );
    assert_eq!((d, c), (dec!(60), Decimal::ZERO), "debit 方向正余额挂借方");

    // credit：0 + 100 - 40 = 60 → 挂贷方
    let (d, c) = VoucherService::compute_ending_balance(
        subject_status::DIRECTION_CREDIT,
        Decimal::ZERO,
        Decimal::ZERO,
        dec!(40),
        dec!(100),
    );
    assert_eq!((d, c), (Decimal::ZERO, dec!(60)), "credit 方向正余额挂贷方");

    // debit 但贷方发生倒挂 → 余额翻挂贷方（同向为正、反向为负的既有规则不变）
    let (d, c) = VoucherService::compute_ending_balance(
        subject_status::DIRECTION_DEBIT,
        Decimal::ZERO,
        Decimal::ZERO,
        dec!(10),
        dec!(70),
    );
    assert_eq!(
        (d, c),
        (Decimal::ZERO, dec!(60)),
        "debit 方向负余额应翻挂贷方"
    );

    // 反例锁：中文 token 不再是借方分支依据（比较点若回潮中文，本断言即失败）
    let (d, _c) = VoucherService::compute_ending_balance(
        "借",
        dec!(100),
        Decimal::ZERO,
        Decimal::ZERO,
        dec!(30),
    );
    assert_eq!(d, Decimal::ZERO, "「借」不得再命中借方分支");
}

// ===========================================================================
// 3) 写入白名单行为锁：非词表 token 被 400 VALIDATION_ERROR 拒且零写入
// ===========================================================================

fn create_req(code: &str, direction: Option<&str>) -> CreateSubjectRequest {
    CreateSubjectRequest {
        code: code.to_string(),
        name: "库存现金".to_string(),
        level: 1,
        parent_id: None,
        balance_direction: direction.map(|s| s.to_string()),
        assist_customer: false,
        assist_supplier: false,
        assist_batch: false,
        assist_color_no: false,
        enable_dual_unit: false,
    }
}

fn update_req(direction: Option<&str>) -> UpdateSubjectRequest {
    UpdateSubjectRequest {
        name: None,
        balance_direction: direction.map(|s| s.to_string()),
        assist_customer: false,
        assist_supplier: false,
        assist_batch: false,
        assist_color_no: false,
        enable_dual_unit: false,
        status: None,
    }
}

#[tokio::test]
async fn create_rejects_illegal_direction_validation_error_and_zero_write() {
    let db = Arc::new(live_db().await);
    let svc = AccountSubjectService::new(db.clone());
    // account_subjects 是迁移播种且不清空的参照表（种子 28 行基线）——
    // 「零写入」在真库上以**行数增量=0**表达，语义与原「全表零行」等价且更严
    let baseline = account_subject::Entity::find()
        .count(&*db)
        .await
        .expect("统计基线行数失败");

    for bad in ["借", "贷", "DEBIT", "Debit", "借借", " "] {
        let err = svc
            .create(create_req(&format!("W6DIRBAD-{bad}"), Some(bad)), 1)
            .await
            .unwrap_err_or_panic(&format!("非法方向 {bad:?} 必须被拒"));
        assert_eq!(
            err.error_code(),
            "VALIDATION_ERROR",
            "{bad:?}: 字段取值非法必须归 VALIDATION_ERROR(400)"
        );
        assert!(
            matches!(err, AppError::ValidationErrorDisplayable(_)),
            "{bad:?}: 文案只含用户自己提交的值与公开取值规则，应为可外显变体（保密分层）"
        );
    }

    // 零写入：校验先行，非法请求一律不带病触库
    let after = account_subject::Entity::find()
        .count(&*db)
        .await
        .expect("回读失败");
    assert_eq!(
        after, baseline,
        "白名单拒收后科目表行数增量必须为 0（不允许任何一条非法方向带病落库）"
    );

    // 正向对照：合法英文值可落库且逐字保存
    let created = svc
        .create(
            create_req("W6DIROK", Some(subject_status::DIRECTION_DEBIT)),
            1,
        )
        .await
        .expect("合法 debit 应创建成功");
    assert_eq!(
        created.balance_direction.as_deref(),
        Some(subject_status::DIRECTION_DEBIT)
    );
    let stored = account_subject::Entity::find_by_id(created.id)
        .one(&*db)
        .await
        .expect("回读失败")
        .expect("合法创建应落库");
    assert_eq!(stored.balance_direction.as_deref(), Some("debit"));
}

#[tokio::test]
async fn update_rejects_chinese_direction_and_keeps_stored_english() {
    // 路线一：表结构唯一来源是 backend/migration（本文件其余用例同口径），
    // 不再自建 account_subjects DDL。
    let db = Arc::new(live_db().await);
    // account_subjects 是迁移播种且**不得清空**的参照表（test_common.rs:36
    // SEALED_REFERENCE_TABLES 首项，m0006 建表时 code UNIQUE），因此本用例种一行
    // 带纳秒后缀的专属 code，零写入判据按这行专属 code 计数（文件头 §覆盖策略口径：
    // "零写入以行数增量表达，不声称全表为空"）。
    // 反例1：断"全表行数 == 1" —— 把参照表当空表，种子一在场就恒红。
    // 反例2：把期望改成"全表行数 == 当前实际值(30)" —— 会把"非法更新真的新增了一行"
    //        这一原缺陷永久吞掉，属放宽断言。
    let my_code = format!("9961{}", unique_suffix());
    let my_full_code = format!("W6DIR{my_code}");
    let id = seed_subject(
        &db,
        &my_code,
        Some(subject_status::DIRECTION_CREDIT),
        Decimal::ZERO,
        dec!(50),
    )
    .await;
    let svc = AccountSubjectService::new(db.clone());

    let err = svc
        .update(id, update_req(Some("贷")), 1)
        .await
        .unwrap_err_or_panic("中文「贷」更新必须被白名单拒收");
    assert_eq!(err.error_code(), "VALIDATION_ERROR");
    assert!(matches!(err, AppError::ValidationErrorDisplayable(_)));

    let stored = account_subject::Entity::find_by_id(id)
        .one(&*db)
        .await
        .expect("回读失败")
        .expect("存量科目应仍在");
    assert_eq!(
        stored.balance_direction.as_deref(),
        Some(subject_status::DIRECTION_CREDIT),
        "拒收后存量值必须保持 credit 原值，零写入"
    );
    // 零新增：本用例专属 code 的行数在第二次非法更新前后不变，且恒只有自己 1 行
    let before = count_rows_by_code(&db, &my_full_code).await;
    let err2 = svc
        .update(id, update_req(Some("借")), 1)
        .await
        .unwrap_err_or_panic("中文「借」更新必须被白名单拒收");
    assert_eq!(err2.error_code(), "VALIDATION_ERROR");
    assert!(matches!(err2, AppError::ValidationErrorDisplayable(_)));
    let after = count_rows_by_code(&db, &my_full_code).await;
    assert_eq!(
        (before, after),
        (1, 1),
        "非法更新不得新增任何行：按本用例专属 code 计数必须前后都是那 1 行存量科目\
         （before={before}, after={after}）"
    );
    // 存量值仍不得被改动（新增与改写是两种不同的带病写入形态，各自钉死）
    let stored_after = account_subject::Entity::find_by_id(id)
        .one(&*db)
        .await
        .expect("回读失败")
        .expect("第二次拒收后存量科目应仍在");
    assert_eq!(
        stored_after.balance_direction.as_deref(),
        Some(subject_status::DIRECTION_CREDIT),
        "两次非法更新都不得触碰存量值"
    );
}

/// Result 扩展：稳定 panic 信息（不依赖 Debug 可得性）
trait UnwrapErrOrPanic<T> {
    fn unwrap_err_or_panic(self, ctx: &str) -> AppError;
}

impl<T: std::fmt::Debug> UnwrapErrOrPanic<T> for Result<T, AppError> {
    fn unwrap_err_or_panic(self, ctx: &str) -> AppError {
        match self {
            Ok(v) => panic!("{ctx}: 预期被拒但成功返回 {v:?}"),
            Err(e) => e,
        }
    }
}

// ===========================================================================
// 4) 迁移归一 CASE 行为锁：直接执行 m0007 的 SQL 常量（在"未跑迁移的沙箱库"里跑 PG）
// ===========================================================================

async fn select_directions(db: &sea_orm::DatabaseConnection) -> Vec<(i64, Option<String>)> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT id, balance_direction FROM account_subjects ORDER BY id".to_string(),
            Vec::<Value>::new(),
        ))
        .await
        .expect("回读失败");
    rows.iter()
        .map(|r| (col_i64(r, 0), col_str_opt(r, 1)))
        .collect()
}

async fn run_m198_up(db: &sea_orm::DatabaseConnection) {
    exec_sql(db, m198::CREATE_BACKUP_TABLE_SQL).await;
    exec_sql(db, m198::BACKUP_LEGACY_ROWS_SQL).await;
    exec_sql(db, m198::NORMALIZE_TO_ENGLISH_SQL).await;
}

#[tokio::test]
async fn m198_normalize_case_mapping_idempotent_and_reversible_in_isolated_sandbox() {
    let db = sandbox_db().await;
    // 同构最小表：归一三条语句只触达 id / balance_direction
    exec_sql(&db, r#"DROP TABLE IF EXISTS "account_subjects""#).await;
    exec_sql(
        &db,
        r#"CREATE TABLE "account_subjects" ("id" BIGINT PRIMARY KEY, "balance_direction" TEXT)"#,
    )
    .await;
    for (id, dir) in [
        (1i64, "借"),
        (2, "贷"),
        (3, "debit"),
        (4, "credit"),
        (5, ""),
    ] {
        let value = if dir.is_empty() {
            Value::String(None)
        } else {
            Value::String(Some(dir.to_string()))
        };
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO account_subjects (id, balance_direction) VALUES ($1, $2)".to_string(),
            vec![id.into(), value],
        ))
        .await
        .unwrap_or_else(|e| panic!("种子行 ({id}, {dir:?}) 插入失败: {e}"));
    }

    // —— up 第一次：CASE 映射逐值断言（借→debit、贷→credit；英文与 NULL 不动，NULL 回填本批不做）
    run_m198_up(&db).await;
    assert_eq!(
        select_directions(&db).await,
        vec![
            (1, Some("debit".to_string())),
            (2, Some("credit".to_string())),
            (3, Some("debit".to_string())),
            (4, Some("credit".to_string())),
            (5, None),
        ],
        "归一后必须只剩英文权威词表 + NULL（NULL 保持原样）"
    );
    let backup = db
        .query_all_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT subject_id, old_balance_direction FROM mig198_account_subject_direction_backup ORDER BY subject_id"
                .to_string(),
        ))
        .await
        .expect("备份表回读失败");
    assert_eq!(backup.len(), 2, "备份表应恰记录两条被改写行的原值");
    assert_eq!(col_str_opt(&backup[0], 1), Some("借".to_string()));
    assert_eq!(col_str_opt(&backup[1], 1), Some("贷".to_string()));

    // —— up 重放：幂等（数据不变、备份表不膨胀）
    run_m198_up(&db).await;
    run_m198_up(&db).await;
    assert_eq!(
        select_directions(&db).await,
        vec![
            (1, Some("debit".to_string())),
            (2, Some("credit".to_string())),
            (3, Some("debit".to_string())),
            (4, Some("credit".to_string())),
            (5, None),
        ],
        "重放 up 后数据应逐值不变（幂等）"
    );
    let backup_count = db
        .query_all_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT COUNT(*) FROM mig198_account_subject_direction_backup".to_string(),
        ))
        .await
        .unwrap();
    assert_eq!(
        col_i64(&backup_count[0], 0),
        2,
        "重放不得使备份表膨胀（NOT EXISTS 守卫）"
    );

    // —— down：按备份精确还原（合法英文行不被腐蚀），并清理备份表
    exec_sql(&db, m198::RESTORE_FROM_BACKUP_SQL).await;
    exec_sql(&db, m198::DROP_BACKUP_TABLE_SQL).await;
    assert_eq!(
        select_directions(&db).await,
        vec![
            (1, Some("借".to_string())),
            (2, Some("贷".to_string())),
            (3, Some("debit".to_string())),
            (4, Some("credit".to_string())),
            (5, None),
        ],
        "down 必须只还原被 up 改写过的两行，原本合法的 debit/credit 行不受影响"
    );
    let remaining = db
        .query_all_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT COUNT(*) FROM information_schema.tables WHERE table_name = 'mig198_account_subject_direction_backup'"
                .to_string(),
        ))
        .await
        .unwrap();
    assert_eq!(
        col_i64(&remaining[0], 0),
        0,
        "down 后备份表必须删除（不留悬挂工件）"
    );
}

// ===========================================================================
// 5) 禁回潮源码扫描（include_str!）+ 词表唯一来源锁
// ===========================================================================

#[test]
fn source_scan_status_vocabulary_is_single_source_in_english() {
    let src = code_only(&include_str!("../src/models/status/finance.rs").replace('\r', ""));
    for needle in [
        "pub mod account_subject {",
        "pub const DIRECTION_DEBIT: &str = \"debit\";",
        "pub const DIRECTION_CREDIT: &str = \"credit\";",
        "pub const ALL: &[&str] = &[DIRECTION_DEBIT, DIRECTION_CREDIT];",
    ] {
        assert!(src.contains(needle), "finance.rs 词表缺失: {needle}");
    }
}

#[test]
fn source_scan_comparison_points_no_longer_match_chinese_literal() {
    for (file, src) in [
        (
            "account_subject_service.rs",
            code_only(
                &include_str!("../src/services/account_subject_service.rs").replace('\r', ""),
            ),
        ),
        (
            "voucher_ops/balance.rs",
            code_only(&include_str!("../src/services/voucher_ops/balance.rs").replace('\r', "")),
        ),
    ] {
        for banned in [r#""借""#, r#""贷""#, r#"'借'"#, r#"'贷'"#] {
            assert!(
                !src.contains(banned),
                "{file}: 比较点/默认值仍出现带引号中文方向字面量 {banned}——必须绑 status::account_subject 常量"
            );
        }
        assert!(
            src.contains("subject_status::DIRECTION_DEBIT"),
            "{file}: 必须引用词表常量（比较点回潮丢失）"
        );
    }

    let svc =
        code_only(&include_str!("../src/services/account_subject_service.rs").replace('\r', ""));
    for needle in [
        "fn validate_balance_direction",
        "subject_status::ALL.contains(&direction)",
        "AppError::validation_displayable",
    ] {
        assert!(svc.contains(needle), "白名单形态缺失: {needle}");
    }
    let gate_sites = svc
        .matches("Self::validate_balance_direction(direction)?;")
        .count();
    assert!(
        gate_sites >= 2,
        "create/update 两处入口都应先过白名单（命中 {gate_sites} 次 < 2）"
    );
}

#[test]
fn source_scan_m198_migration_shape_locked() {
    let src = include_str!(
        "../migration/src/domain/system/m0007_normalize_account_subject_balance_direction.rs"
    )
    .replace('\r', "");
    // 禁项只看执行体：m0007 文件头 §设计 里正当出现"不用 UPDATE...FROM、DO $$ 等 PG
    // 专有形态"这句**说明文字**，按原文判禁词会把自我约束声明当成违例（#4671 判责 B1①
    // 同一形态）。必备项同理只在剥注释后的文本上查——注释里复述 SQL 片段不算实现。
    let code = code_only(&src);
    for needle in [
        "WHEN '借' THEN 'debit'",
        "WHEN '贷' THEN 'credit'",
        "WHERE \"balance_direction\" IN ('借', '贷')",
        "CREATE TABLE IF NOT EXISTS",
        "NOT EXISTS",
        "DROP TABLE IF EXISTS",
    ] {
        assert!(code.contains(needle), "m0007 迁移语句形态缺失: {needle}");
    }
    // 双方言守护：不得引入 PG 专有 UPDATE...FROM / DO $$（会破坏 sqlite 可真跑性）
    for banned in ["DO $$", "TIMESTAMPTZ"] {
        assert!(
            !code.contains(banned),
            "m0007 引入 PG 专有形态将破坏 sqlite 可验性: {banned}"
        );
    }
}
