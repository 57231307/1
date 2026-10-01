//! 任务 #942 第 5 波：凭证 / 预算错误族与保密分层契约锁
//!
//! 被锁的缺口（族口径判据由用户拍板，提交 763c7ab9；保密分层见 `utils/error.rs` 模块文档）：
//!
//! 1. `voucher_ops/crud.rs::precheck_subjects_exist_txn` 曾把「科目查无」与「科目已停用」
//!    压成同一条 `bad_request("科目不存在或已停用：…")`（HTTP 400 / BAD_REQUEST）。
//!    两者语义不同，必须分属两族：
//!    - 查无此科目（引用存在性缺失）→ `AppError::not_found` → HTTP **404** + `NOT_FOUND`；
//!    - 科目存在但状态非 `active`（引用主数据的前置状态门）→ **业务族**，文案含科目 code /
//!      记录 ID → 按安全边界走**脱敏** `AppError::business` → HTTP **400** + `BUSINESS_ERROR`
//!      + 出参常量「业务处理失败」。
//! 2. 同文件三处借贷不平衡（`create` 的非生产/生产两分支 + `update` 换分录路径）——
//!    借贷是否平衡完全由提交进来的分录金额决定，属「提交数据一致性校验」→ **校验族**；
//!    文案含借/贷合计金额数字 → 脱敏 `AppError::validation` → 400 + `VALIDATION_ERROR`
//!    + 出参常量「请求参数验证失败」（**不得**改成 displayable 外显）。
//! 3. `budget_management_service.rs::check_budget_available`：
//!    「预算方案未审批或未激活」是前置状态门 → 业务族且纯规则文案可外显；
//!    「预算方案与部门不匹配」是跨字段取值一致性 → 校验族且保持脱敏。
//!    该域运行态分支需要真实 `budget_plans` 行 + 部门外键，本轮以源码扫描锁住两条族归类，
//!    运行态补测点已移交测试专家（见交付说明）。
//!
//! 覆盖形态（无 mock、走真实 service 方法）：
//! - 借贷不平衡的拒绝发生在 `create()` 触库之前（非生产分支只做纯金额求和），
//!   故该用例不需要任何表结构，`sqlite::memory:` 空连接即可真实跑到被锁分支；
//! - 科目两分支的拒绝要先插入凭证主表再预检，需已迁移真库（`TEST_DATABASE_URL` → PostgreSQL，
//!   CI `ci-test-rust-*` 两个 job 均已注入）。本文件**不做条件跳过、也不回退 sqlite 空表**：
//!   变量缺失即在该用例行首显式失败并说明原因。
//! - 源码扫描锁（`include_str!`）：防 `bad_request("科目…")` / `bad_request("借…")` 回潮。

use axum::http::StatusCode;
use axum::response::IntoResponse;
use bingxi_backend::models::status::master_data;
use bingxi_backend::models::{account_subject, voucher, voucher_item};
use bingxi_backend::services::voucher_service::{
    CreateVoucherRequest, VoucherItemRequest, VoucherService,
};
use bingxi_backend::utils::error::AppError;
use bingxi_backend::utils::messages::err_msg;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::str::FromStr;
use std::sync::Arc;

// =========================================================
// 夹具
// =========================================================

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

/// 凭证分录请求（本文件只关心科目与金额，其余字段一律置空）
fn item(
    subject_id: Option<i32>,
    subject_code: Option<&str>,
    debit: Decimal,
    credit: Decimal,
) -> VoucherItemRequest {
    VoucherItemRequest {
        line_no: None,
        subject_id,
        subject_code: subject_code.map(|c| c.to_string()),
        subject_name: None,
        debit,
        credit,
        summary: Some("wave5 族口径契约测".to_string()),
        assist_customer_id: None,
        assist_supplier_id: None,
        assist_department_id: None,
        assist_employee_id: None,
        assist_project_id: None,
        assist_batch_id: None,
        assist_color_no_id: None,
        assist_dye_lot_id: None,
        assist_grade: None,
        assist_workshop_id: None,
        quantity_meters: None,
        quantity_kg: None,
        unit_price: None,
    }
}

fn create_req(items: Vec<VoucherItemRequest>) -> CreateVoucherRequest {
    CreateVoucherRequest {
        voucher_type: "记".to_string(),
        voucher_date: NaiveDate::from_ymd_opt(2026, 3, 1).expect("夹具日期必须合法"),
        source_type: None,
        source_module: None,
        source_bill_id: None,
        source_bill_no: None,
        batch_no: None,
        color_no: None,
        items,
    }
}

/// 活库连接（已迁移 PostgreSQL）。
///
/// 缺失 `TEST_DATABASE_URL` 时**显式失败并说明原因**：科目两分支的用例必经 `create()`
/// 写真实 `vouchers` 主表（并走 PG 方言的凭证号生成），回退 `sqlite::memory:` 空表只会
/// 得到 `DATABASE_ERROR`，那是假绿/假红而不是覆盖。
async fn live_pg_db() -> DatabaseConnection {
    let url = std::env::var("TEST_DATABASE_URL").expect(
        "本用例需要 TEST_DATABASE_URL 指向已跑完迁移的 PostgreSQL（CI 的 ci-test-rust-* job \
         已注入）。缺失该变量时不回退 sqlite、也不跳过——按 #942 要求显式失败：\
         本地请 export TEST_DATABASE_URL=postgres://user:pass@localhost:5432/bingxi_test",
    );
    assert!(
        url.starts_with("postgres"),
        "本用例写真实 vouchers 并用 PG 方言取号，TEST_DATABASE_URL 必须是 PostgreSQL，实际={url}"
    );
    sea_orm::Database::connect(&url)
        .await
        .expect("活库用例：TEST_DATABASE_URL 连接失败")
}

/// 建一枚指定状态的科目（数值/布尔列全走 DDL 默认值，只 Set 用例关心的 code/name/status）
async fn seed_subject_with_status(
    db: &DatabaseConnection,
    tag: &str,
    status: &str,
) -> (i32, String) {
    let ts = Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let code = format!("W5F{tag}{ts}");
    let inserted = account_subject::ActiveModel {
        code: Set(code.clone()),
        name: Set(format!("wave5 族口径测试科目{tag}")),
        level: Set(1),
        status: Set(status.to_string()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("插入测试科目（status={status}）失败：{e}"));
    (inserted.id, code)
}

/// 清理本用例自建科目：删除失败必须显式报错（不静默留下脏种子）
async fn cleanup_subjects(db: &DatabaseConnection, ids: &[i32]) {
    if ids.is_empty() {
        return;
    }
    let deleted = account_subject::Entity::delete_many()
        .filter(account_subject::Column::Id.is_in(ids.iter().copied().collect::<Vec<_>>()))
        .exec(db)
        .await
        .unwrap_or_else(|e| panic!("清理测试科目 {ids:?} 失败：{e}"));
    assert_eq!(
        deleted.rows_affected,
        ids.len() as u64,
        "测试科目清理行数与自建行数不符（残留会污染后续用例）"
    );
}

/// 清理 create() 已经落库的凭证（分录 + 主表）。
///
/// 走实体直删而不是 `VoucherService::delete`：后者带审计链（需要 users 行），
/// 本函数只在「预期被拒却成功」的异常分支与对照用例里做数据回收。
async fn cleanup_voucher(db: &DatabaseConnection, voucher_id: i32) {
    let items = voucher_item::Entity::delete_many()
        .filter(voucher_item::Column::VoucherId.eq(voucher_id))
        .exec(db)
        .await
        .unwrap_or_else(|e| panic!("清理凭证 {voucher_id} 的分录失败：{e}"));
    let head = voucher::Entity::delete_by_id(voucher_id)
        .exec(db)
        .await
        .unwrap_or_else(|e| panic!("清理凭证 {voucher_id} 主表失败：{e}"));
    assert_eq!(
        head.rows_affected, 1,
        "凭证主表未被回收，voucher_id={voucher_id}"
    );
    assert!(
        items.rows_affected > 0,
        "对照用例应至少落库一条分录，实际 rows_affected={}",
        items.rows_affected
    );
}

/// 断言失败出参的完整形状：HTTP status + 机器码 + message（外显原文或脱敏常量）
fn assert_envelope(err: &AppError, status: StatusCode, code: &str, message: &str) {
    let got_status = err.clone().into_response().status();
    assert_eq!(
        got_status, status,
        "HTTP 状态码必须是被锁族的状态码，实际={got_status}，err={err:?}"
    );
    let body = err.to_response();
    assert_eq!(body.code, code, "机器码不符，期望={code}，err={err:?}");
    assert_eq!(
        body.message, message,
        "出参 message 不符（族/保密分层判据见 utils/error.rs 模块文档），err={err:?}"
    );
}

// =========================================================
// 缺口 2：借贷不平衡 → 校验族 + 脱敏（不得落 BAD_REQUEST）
// =========================================================

/// 借贷不平衡被拒：400 + VALIDATION_ERROR + 出参常量「请求参数验证失败」。
///
/// 该拒绝在 `create()` 触库之前发生，故空 sqlite 连接即可真实跑到被锁分支；
/// 若未来有人把平衡校验挪到取号/落库之后，本用例会以 `DATABASE_ERROR` 显式变红而非静默。
#[tokio::test]
async fn unbalanced_voucher_is_sanitized_validation_error() {
    assert!(
        !bingxi_backend::utils::config::is_production(),
        "本用例依赖 APP_ENV 非 production（凭证创建在非生产分支先做借贷平衡、不触库），\
         实际 APP_ENV={:?}",
        std::env::var("APP_ENV").ok()
    );

    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败");
    let svc = VoucherService::new(Arc::new(db));

    // 借 100 / 贷 99：故意不平 1 元。
    let err = svc
        .create(
            create_req(vec![
                item(Some(1), Some("1001"), dec("100"), Decimal::ZERO),
                item(Some(2), Some("2001"), Decimal::ZERO, dec("99")),
            ]),
            1,
        )
        .await
        .expect_err("借贷不平衡必须被拒，绝不能落库成功");

    // 族：校验族的**脱敏**变体（不得是 ValidationErrorDisplayable / BadRequest / Business）。
    assert!(
        matches!(err, AppError::ValidationError(_)),
        "借贷不平衡必须是脱敏校验族 ValidationError，实际={err:?}"
    );
    assert_envelope(
        &err,
        StatusCode::BAD_REQUEST,
        "VALIDATION_ERROR",
        err_msg::VALIDATION_PUBLIC,
    );
    assert_eq!(
        err_msg::VALIDATION_PUBLIC,
        "请求参数验证失败",
        "出参常量取自 utils/messages.rs 的 VALIDATION_PUBLIC，若该常量变动需同步 e2e 断言"
    );
    // 真实文案（含金额数字）仍可在日志侧 Display 查到，但不进 HTTP 出参。
    let logged = err.to_string();
    assert!(
        logged.contains("凭证借贷不平衡"),
        "真实拒绝原因必须仍可查于日志侧 Display，实际={logged}"
    );
    assert!(
        !err.to_response().message.contains("100"),
        "借方金额数字不得外显到 HTTP 出参"
    );
}

// =========================================================
// 缺口 1：科目已停用（状态门）与科目查无（存在性）必须分属两族
// =========================================================

/// 分录引用**已停用**科目：400 + BUSINESS_ERROR + 脱敏常量（真实 code 只进日志）。
#[tokio::test]
async fn disabled_subject_reference_is_sanitized_business_error() {
    let db = live_pg_db().await;
    let (disabled_id, disabled_code) =
        seed_subject_with_status(&db, "DISID", master_data::INACTIVE).await;
    let (active_id, _) = seed_subject_with_status(&db, "DISACT", master_data::ACTIVE).await;
    let svc = VoucherService::new(Arc::new(db.clone()));

    let result = svc
        .create(
            create_req(vec![
                item(Some(disabled_id), None, dec("100"), Decimal::ZERO),
                item(Some(active_id), None, Decimal::ZERO, dec("100")),
            ]),
            1,
        )
        .await;

    let err = match result {
        Ok(created) => {
            // 状态门失效属于真缺陷：先把残留收干净再红，不静默留脏数据。
            cleanup_voucher(&db, created.id).await;
            cleanup_subjects(&db, &[disabled_id, active_id]).await;
            panic!(
                "引用已停用科目的凭证竟然创建成功（状态门失效），voucher_no={}",
                created.voucher_no
            );
        }
        Err(e) => e,
    };

    assert!(
        matches!(err, AppError::BusinessError(_)),
        "科目已停用属前置状态门，必须是**脱敏** BusinessError（不得 displayable、\
         不得回到 BAD_REQUEST），实际={err:?}"
    );
    assert_envelope(
        &err,
        StatusCode::BAD_REQUEST,
        "BUSINESS_ERROR",
        err_msg::BUSINESS_PUBLIC,
    );
    assert_eq!(
        err_msg::BUSINESS_PUBLIC,
        "业务处理失败",
        "出参常量取自 utils/messages.rs 的 BUSINESS_PUBLIC，若该常量变动需同步 e2e 断言"
    );
    let logged = err.to_string();
    assert!(
        logged.contains(disabled_code.as_str()),
        "真实科目编码必须留在日志侧 Display 里可查，实际={logged}"
    );
    let public = err.to_response().message;
    assert!(
        !public.contains(disabled_code.as_str()),
        "科目编码不得出现在 HTTP 出参里，实际出参={public}"
    );

    cleanup_subjects(&db, &[disabled_id, active_id]).await;
}

/// 同一状态门的 code 分支（分录只带 `subject_code`）族归类与 id 分支一致。
#[tokio::test]
async fn disabled_subject_reference_by_code_is_same_business_error() {
    let db = live_pg_db().await;
    let (disabled_id, disabled_code) =
        seed_subject_with_status(&db, "DISCODE", master_data::INACTIVE).await;
    let svc = VoucherService::new(Arc::new(db.clone()));

    let err = svc
        .create(
            create_req(vec![
                item(None, Some(&disabled_code), dec("100"), Decimal::ZERO),
                item(None, Some(&disabled_code), Decimal::ZERO, dec("100")),
            ]),
            1,
        )
        .await
        .expect_err("引用已停用科目（按 code 提交）必须被状态门拒绝");

    assert!(
        matches!(err, AppError::BusinessError(_)),
        "按 code 提交的已停用科目必须与按 id 提交同族（脱敏 BusinessError），实际={err:?}"
    );
    assert_envelope(
        &err,
        StatusCode::BAD_REQUEST,
        "BUSINESS_ERROR",
        err_msg::BUSINESS_PUBLIC,
    );

    cleanup_subjects(&db, &[disabled_id]).await;
}

/// 分录引用**查无此科目**：404 + NOT_FOUND（与「已停用」的业务族必须可分辨）。
#[tokio::test]
async fn missing_subject_reference_is_not_found() {
    let db = live_pg_db().await;
    let (active_id, active_code) =
        seed_subject_with_status(&db, "MISSACT", master_data::ACTIVE).await;
    let svc = VoucherService::new(Arc::new(db.clone()));

    // id 分支：一个绝不会存在的科目 ID
    let err_by_id = svc
        .create(
            create_req(vec![
                item(Some(999_999_999), None, dec("100"), Decimal::ZERO),
                item(Some(active_id), None, Decimal::ZERO, dec("100")),
            ]),
            1,
        )
        .await
        .expect_err("引用不存在的科目 ID 必须被拒");
    assert!(
        matches!(err_by_id, AppError::NotFound(_)),
        "引用存在性缺失属 NOT_FOUND 族，实际={err_by_id:?}"
    );
    assert_envelope(
        &err_by_id,
        StatusCode::NOT_FOUND,
        "NOT_FOUND",
        err_msg::NOT_FOUND_PUBLIC,
    );
    assert_eq!(
        err_msg::NOT_FOUND_PUBLIC,
        "资源未找到",
        "出参常量取自 utils/messages.rs 的 NOT_FOUND_PUBLIC，若该常量变动需同步 e2e 断言"
    );

    // code 分支：一个绝不会存在的科目编码
    let err_by_code = svc
        .create(
            create_req(vec![
                item(None, Some("W5FNOTEXIST9999"), dec("100"), Decimal::ZERO),
                item(None, Some(&active_code), Decimal::ZERO, dec("100")),
            ]),
            1,
        )
        .await
        .expect_err("引用不存在的科目编码必须被拒");
    assert!(
        matches!(err_by_code, AppError::NotFound(_)),
        "按 code 提交的查无此科目同样属 NOT_FOUND 族，实际={err_by_code:?}"
    );
    assert_envelope(
        &err_by_code,
        StatusCode::NOT_FOUND,
        "NOT_FOUND",
        err_msg::NOT_FOUND_PUBLIC,
    );

    // 本文件最核心的那条线：停用（400/业务族）与查无（404/未找到族）code 必须不同，
    // 否则前端仍无法区分「编码写错」与「科目被停用」。
    assert_ne!(
        err_by_id.error_code(),
        "BUSINESS_ERROR",
        "查无此科目不得再被归成业务族（与已停用不可分辨）"
    );

    cleanup_subjects(&db, &[active_id]).await;
}

/// 正向对照：同一科目处于 `active` 时同一凭证可创建成功（证明上面两条拒绝确由
/// 状态/存在性触发，而不是夹具本身跑不通）。
#[tokio::test]
async fn active_subject_reference_creates_voucher() {
    let db = live_pg_db().await;
    let (subject_id, _) = seed_subject_with_status(&db, "OKACT", master_data::ACTIVE).await;
    let svc = VoucherService::new(Arc::new(db.clone()));

    let created = svc
        .create(
            create_req(vec![
                item(Some(subject_id), None, dec("100"), Decimal::ZERO),
                item(Some(subject_id), None, Decimal::ZERO, dec("100")),
            ]),
            1,
        )
        .await
        .expect("active 科目 + 借贷平衡的凭证必须创建成功（对照用例）");

    assert_eq!(
        created.status,
        bingxi_backend::models::status::voucher::VOUCHER_DRAFT,
        "新建凭证应落库为草稿态（对照用例的前置）"
    );

    cleanup_voucher(&db, created.id).await;
    cleanup_subjects(&db, &[subject_id]).await;
}

// =========================================================
// 防回潮源码扫描锁
// =========================================================

const CRUD_SRC: &str = include_str!("../src/services/voucher_ops/crud.rs");
const BUDGET_SRC: &str = include_str!("../src/services/budget_management_service.rs");

/// 取第一条**非注释**且包含 `needle` 的行前后窗口源码。
///
/// needle 不存在（或只存在于注释里）即 panic：视为该契约点已从源码消失。
fn code_window_around(src: &str, needle: &str, before: usize, after: usize) -> String {
    let lines: Vec<&str> = src.lines().collect();
    let idx = lines
        .iter()
        .position(|l| {
            let t = l.trim_start();
            l.contains(needle) && !t.starts_with("//")
        })
        .unwrap_or_else(|| panic!("契约文案 {needle:?} 已从源码非注释位置消失，族归类被挪走/删除"));
    let lo = idx.saturating_sub(before);
    let hi = (idx + after + 1).min(lines.len());
    lines[lo..hi].join("\n")
}

/// `voucher_ops/crud.rs` 防回潮：科目 / 借贷不平衡两类拒绝不得再走 BAD_REQUEST 兜底族。
#[test]
fn crud_source_scan_no_bad_request_left_for_subject_and_balance() {
    // 任务明列的两条锁（按源码文本前缀）。
    assert!(
        !CRUD_SRC.contains("bad_request(\"科目"),
        "crud.rs 内不得再出现 bad_request(\"科目…"
    );
    assert!(
        !CRUD_SRC.contains("bad_request(\"借"),
        "crud.rs 内不得再出现 bad_request(\"借…"
    );
    // 该文件的四类拒绝已全部归位（查无→NOT_FOUND、停用→业务、不平→校验、状态门→业务族），
    // BAD_REQUEST 在此不再有合法用途；任何回潮都违反 #942 族口径。
    assert!(
        !CRUD_SRC.contains("AppError::bad_request("),
        "crud.rs 内 AppError::bad_request( 必须为 0 处"
    );
    // 被合并过的旧文案不得回来（合并即意味着两族再次不可分辨）。
    assert!(
        !CRUD_SRC.contains("科目不存在或已停用"),
        "「不存在」与「已停用」不得再合并成一条文案"
    );
    // 正向锁：三个族构造子各就各位。窗口取「文案行 ±（上 1 下 2）」——文案字面量同时出现在
    // 上方 warn! 里，窗口必须把紧随其后的 return 语句包住才判得准（也容忍 rustfmt 换行）。
    let nf_window = code_window_around(CRUD_SRC, "科目不存在：", 1, 2);
    assert!(
        nf_window.contains("AppError::not_found("),
        "科目查无必须是 not_found，实际窗口={nf_window}"
    );
    let disabled_window = code_window_around(CRUD_SRC, "科目已停用：", 1, 2);
    assert!(
        disabled_window.contains("AppError::business("),
        "科目已停用必须是脱敏 business，实际窗口={disabled_window}"
    );
    assert!(
        !disabled_window.contains("AppError::business_displayable("),
        "含科目 code / 记录 ID 的文案不得外显"
    );
    // 借贷不平衡统一走 balance_error 单一装配点：脱敏校验族。
    let balance_window = code_window_around(CRUD_SRC, "fn balance_error", 2, 8);
    assert!(
        balance_window.contains("AppError::validation("),
        "balance_error 装配点必须返回脱敏 validation，实际窗口={balance_window}"
    );
    assert!(
        !balance_window.contains("validation_displayable(")
            && !balance_window.contains("business_displayable("),
        "含借/贷金额数字的文案不得外显"
    );
    // 三处平衡校验不得各自装配、绕开单一装配点。
    assert_eq!(
        CRUD_SRC.matches("Self::balance_error(").count(),
        3,
        "create 两分支 + update 换分录路径应共 3 处调用 balance_error"
    );
}

/// `budget_management_service.rs` 防回潮：状态门文案归业务族，匹配性文案归校验族。
#[test]
fn budget_source_scan_state_gate_is_business_not_validation() {
    let gate_window = code_window_around(BUDGET_SRC, "预算方案未审批或未激活", 5, 1);
    assert!(
        gate_window.contains("AppError::business_displayable("),
        "「预算方案未审批或未激活」是前置状态门 → 必须 business_displayable，\
         实际窗口={gate_window}"
    );
    assert!(
        !gate_window.contains("AppError::validation(")
            && !gate_window.contains("AppError::bad_request("),
        "状态门文案不得再挂校验族 / BAD_REQUEST 构造子"
    );

    let mismatch_window = code_window_around(BUDGET_SRC, "预算方案与部门不匹配", 5, 1);
    assert!(
        mismatch_window.contains("AppError::validation("),
        "方案/部门不一致属跨字段取值问题 → 必须留在校验族，实际窗口={mismatch_window}"
    );
    assert!(
        !mismatch_window.contains("AppError::business"),
        "该分支不涉状态，不得被并进业务族"
    );
}
