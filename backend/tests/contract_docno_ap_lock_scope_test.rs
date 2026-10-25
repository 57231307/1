//! 应付族单据号"取号与插入落在同一未提交事务"的活库契约锁
//!
//! 功能：钉住应付侧五处建单入口的取号锁作用域。活体判据全部取自 PostgreSQL 可观测面
//! （pg_locks 与 pg_stat_activity），不看代码文本、不断文案：
//! - 正向主锁：先在目标单据表上摆一把与 INSERT 冲突的表锁（SHARE ROW EXCLUSIVE 模式，
//!   放行旁路 SELECT、只挡住写），再真实调用建单入口，被测事务就会停在插入语句上。
//!   此刻读取锁表，要求"等待插入的那条会话"同时持有一把已授予的取号 advisory
//!   咨询锁，且两条锁记录的 pid 与 virtualtransaction 都相同；另一条会话按表计数读不到
//!   任何行（说明该事务确实未提交）。三者同时成立才叫取号与插入落在同一个未提交事务：
//!   号段占用从取号起一直覆盖到插入之后，并发会话在同前缀同日内取不到同号。
//!   放行表锁后建单必须成功落库，且本连接池的 advisory 锁计数回落到零（不留锁泄漏）。
//! - 反向锁（变异自证）：按缺陷形态复现一次取号，即用自开事务的
//!   DocumentNumberGenerator::generate_no 取号、再另开事务插入同一张表。该形态下取号
//!   返回时 advisory 锁已随内层事务提交释放，停在插入上的会话不再持任何取号锁，
//!   上面的正向判据在此必须为假——证明主锁测的是锁的作用域，不是"库里恒有锁"的恒真读数。
//! - facade 直连锁：调用方事务内取号后锁随该事务存活，回滚即释放，且只取号不落库。
//!
//! 缺陷后果（同一形态的并发行为，非判责）：取号在事务外、插入在事务内时，两次并发建单
//! 会取到同一单号并撞单据列的 UNIQUE 约束，对外表现为 DATABASE_ERROR。
//! 涉及的单据号列与唯一约束：ap_invoice.invoice_no、ap_payment_request.request_no、
//! ap_payment.payment_no（建表即带 UNIQUE，见 migration m0012 应付模块 DDL）。
//!
//! 调用方：集成测试 crate，CI 在已跑完迁移链的 PostgreSQL 上执行（nextest 串行组
//! db-integration 逐条跑，避免夹具全局清空互踩）。
//! 入参：夹具连接与探测连接都按 TEST_DATABASE_URL 建立，缺变量或指向 sqlite 直接 panic，
//! 禁回退、禁跳过；被测 service 走一条带专用 application_name 的独立连接池，
//! 判据只认该池所属后端会话，避免同库其它会话的锁与写入串进断言。
//! 传给谁：判据结果交给断言本身，无下游。
//! 存什么·存哪里：只在应付三张单据表及其父档参照表（suppliers、users、warehouses、
//! purchase_receipt、purchase_return）落少量夹具行，业务表由 setup_test_db 逐例清空；
//! 探测连接只读系统视图与行数，不写任何库对象。
//!
//! id 段避让：本文件只占用 998_0xx 段（操作人 998_001）。逐条 grep 过仓内既有登记段
//! 910_0xx、941_0xx、947_0xx、963_4xx、987_3xx、990_1xx、990_3xx、990_6xx、990_7xx、
//! 991_0xx、992_0xx、992_1xx、993_0xx、994_0xx、994_1xx、995_0xx、996_0xx、997_0xx、
//! 997_1xx、999_0xx，998 段无占用（995 段一并让开，与同期其它取号锁车道不同段，
//! 避免并行改动在同一号段上互相登记）；供应商、仓库、单据行一律走自增主键后回读真实
//! 主键，不占固定 id，suppliers 属封存参照表跨例不清空，更不得使用显式 id。

mod test_common;

use bingxi_backend::constants::{DEFAULT_CURRENCY, DEFAULT_PAYMENT_TERMS_DAYS};
use bingxi_backend::models::status::common::{STATUS_APPROVED, STATUS_DRAFT};
use bingxi_backend::models::status::purchase_inventory::{
    purchase_receipt as receipt_status, purchase_receipt_inspection, purchase_return as pr_status,
};
use bingxi_backend::models::{
    ap_invoice, ap_payment_request, purchase_receipt, purchase_return, supplier, user, warehouse,
};
use bingxi_backend::services::ap_invoice_service::{ApInvoiceService, CreateApInvoiceRequest};
use bingxi_backend::services::ap_payment_request_service::{
    ApPaymentRequestService, CreateApPaymentRequest as CreateRequestDto,
};
use bingxi_backend::services::ap_payment_service::{
    ApPaymentService, CreateApPaymentRequest as CreatePaymentDto,
};
use bingxi_backend::utils::number_generator::DocumentNumberGenerator;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ConnectOptions, ConnectionTrait, Database, DatabaseConnection,
    DatabaseTransaction, DbBackend, Set, Statement, TransactionTrait,
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use test_common::setup_test_db;
use tokio::task::JoinHandle;

/// 被测 service 专用连接池的应用名：pg_stat_activity 用它把"被测建单调用所在会话"
/// 与同一只库上的其它会话区分开，判据只认本池会话。
const SERVICE_APP_NAME: &str = "docno_ap_lock_test";

/// 本文件专属操作人主键（users 随业务表清空，可显式固定 id；号段避让见文件头）
const OPERATOR_ID: i32 = 998_001;

/// 三张应付单据表名，同时是本锁允许上表锁与行数断言的白名单（本文件内部常量，
/// 不接受任何外部串拼接）
const TABLE_AP_INVOICE: &str = "ap_invoice";
const TABLE_AP_PAYMENT_REQUEST: &str = "ap_payment_request";
const TABLE_AP_PAYMENT: &str = "ap_payment";

/// 等待被测事务停在插入语句上的上限：真库上取号与校验都在毫秒级完成，
/// 超上限说明调用根本没走到插入（判红并点名，不放行成假绿）
const BLOCK_WAIT_DEADLINE: Duration = Duration::from_secs(20);

fn now() -> DateTime<Utc> {
    Utc::now()
}

fn dec(value: &str) -> Decimal {
    value
        .parse::<Decimal>()
        .expect("夹具金额常量可解析为 Decimal")
}

fn today() -> chrono::NaiveDate {
    Utc::now().date_naive()
}

/// 测试库连接串：与夹具同一 PostgreSQL，指 sqlite 一律判红（禁方言失真回退）
fn test_database_url() -> String {
    let url = std::env::var("TEST_DATABASE_URL").expect(
        "本锁必须连已迁移的 PostgreSQL：未设置 TEST_DATABASE_URL（缺变量直接判红，不跳过）",
    );
    assert!(
        !url.starts_with("sqlite"),
        "TEST_DATABASE_URL 指向 sqlite，锁不可信: {url}"
    );
    url
}

/// 被测 service 专用池（带 application_name，便于按会话归属判定）
async fn service_pool() -> DatabaseConnection {
    let mut opts = ConnectOptions::new(test_database_url());
    opts.set_application_name(SERVICE_APP_NAME)
        .sqlx_logging(false)
        .max_connections(5);
    Database::connect(opts)
        .await
        .expect("被测连接池建立失败（PostgreSQL 不可达？）")
}

/// 探测池：另开一条会话，与业务会话不同后端，才能看见对方持有的锁
async fn probe_pool() -> DatabaseConnection {
    Database::connect(test_database_url())
        .await
        .expect("探测连接建立失败（PostgreSQL 不可达？）")
}

/// 在目标表上摆一把只挡写的表锁：SHARE ROW EXCLUSIVE 与 INSERT 申请的 ROW EXCLUSIVE
/// 冲突，但不与 SELECT 的 ACCESS SHARE 冲突，因此被测事务能跑完取号与校验、
/// 只在插入处排队
async fn block_table_for_inserts(
    admin: &DatabaseConnection,
    table: &'static str,
) -> DatabaseTransaction {
    let txn = admin.begin().await.expect("阻塞事务开启失败");
    txn.execute_raw(Statement::from_string(
        DbBackend::Postgres,
        "SET lock_timeout = '20s'".to_string(),
    ))
    .await
    .expect("为阻塞会话设置 lock_timeout 失败");
    txn.execute_raw(Statement::from_string(
        DbBackend::Postgres,
        format!("LOCK TABLE {table} IN SHARE ROW EXCLUSIVE MODE"),
    ))
    .await
    .unwrap_or_else(|e| panic!("在 {table} 上摆表锁失败: {e}"));
    txn
}

/// 停在插入上的会话所给出的锁证据
struct LockEvidence {
    /// 等待插入的后端会话 pid
    insert_pid: i64,
    /// 该会话当时的虚拟事务号
    virtual_transaction: String,
    /// 同一 pid 且同一虚拟事务上是否持有已授予的 advisory 咨询锁
    advisory_held_in_same_xact: bool,
    /// 该会话当前语句文本（仅用于失败点名，不参与判定，避免依赖语句文案口径）
    waiting_query: String,
}

/// 按 application_name 统计本测试连接池当前持有的已授予 advisory 锁条数
/// （限定本池会话，同库其它会话的锁不构成噪声）
async fn pool_advisory_lock_count(probe: &DatabaseConnection) -> i64 {
    let row = probe
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT COUNT(*) FROM pg_locks l \
             WHERE l.locktype = 'advisory' AND l.granted \
               AND l.pid IN (SELECT pid FROM pg_stat_activity WHERE application_name = $1)"
                .to_string(),
            [SERVICE_APP_NAME.to_string().into()],
        ))
        .await
        .expect("统计本测试连接池的 advisory 锁条数失败");
    row.and_then(|r| r.try_get_by_index::<i64>(0).ok())
        .expect("advisory 锁条数解码失败")
}

/// 目标表的已提交行数（探测会话视角）
async fn visible_row_count(probe: &DatabaseConnection, table: &'static str) -> i64 {
    let row = probe
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            format!("SELECT COUNT(*) FROM {table}"),
        ))
        .await
        .unwrap_or_else(|e| panic!("统计 {table} 行数失败: {e}"));
    row.and_then(|r| r.try_get_by_index::<i64>(0).ok())
        .expect("行数解码失败")
}

/// 查询"正等待在目标表写锁上的本池会话"及其同事务 advisory 锁持有情况。
/// 采到返回证据，没采到返回 None 由调用方继续等。
async fn query_blocked_insert(
    probe: &DatabaseConnection,
    table: &'static str,
) -> Result<Option<LockEvidence>, String> {
    let row = probe
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            format!(
                "SELECT w.pid, w.virtualtransaction::text, a.query, \
                        EXISTS (SELECT 1 FROM pg_locks k \
                                WHERE k.pid = w.pid AND k.locktype = 'advisory' AND k.granted \
                                  AND k.virtualtransaction = w.virtualtransaction) \
                 FROM pg_locks w \
                 JOIN pg_class c ON c.oid = w.relation \
                 JOIN pg_stat_activity a ON a.pid = w.pid \
                 WHERE c.relname = $1 AND w.locktype = 'relation' AND NOT w.granted \
                   AND a.application_name = $2"
            ),
            [
                table.to_string().into(),
                SERVICE_APP_NAME.to_string().into(),
            ],
        ))
        .await
        .map_err(|e| format!("查询等待插入的会话失败: {e}"))?;
    let Some(row) = row else {
        return Ok(None);
    };
    let pid = row
        .try_get_by_index::<i64>(0)
        .map_err(|e| format!("等待插入会话的 pid 解码失败: {e}"))?;
    let virtual_transaction = row
        .try_get_by_index::<Option<String>>(1)
        .map_err(|e| format!("virtualtransaction 解码失败: {e}"))?
        .unwrap_or_default();
    let waiting_query = row
        .try_get_by_index::<Option<String>>(2)
        .map_err(|e| format!("query 解码失败: {e}"))?
        .unwrap_or_default();
    let advisory_held = row
        .try_get_by_index::<bool>(3)
        .map_err(|e| format!("同事务 advisory 锁持有标记解码失败: {e}"))?;
    Ok(Some(LockEvidence {
        insert_pid: pid,
        virtual_transaction,
        advisory_held_in_same_xact: advisory_held,
        waiting_query,
    }))
}

/// 摆锁、发起一次真实建单、等它停在插入处采集证据与未提交行数，然后原样交回控制权。
/// work 只负责把真实建单调用交给运行器（返回其 JoinHandle），本函数不认识具体单据类型。
/// 采不到证据时先放行表锁再判红，避免把库留在锁住状态。
async fn open_blocked_window<T>(
    admin: &DatabaseConnection,
    probe: &DatabaseConnection,
    table: &'static str,
    work: impl FnOnce() -> JoinHandle<T>,
) -> (LockEvidence, i64, DatabaseTransaction, JoinHandle<T>) {
    let blocker = block_table_for_inserts(admin, table).await;
    let handle = work();

    let deadline = Instant::now() + BLOCK_WAIT_DEADLINE;
    let evidence = loop {
        match query_blocked_insert(probe, table).await {
            Ok(Some(evidence)) => break evidence,
            Ok(None) => {}
            Err(message) => {
                let _ = blocker.rollback().await;
                panic!("{message}");
            }
        }
        if Instant::now() >= deadline {
            let _ = blocker.rollback().await;
            let _ = handle.await;
            panic!(
                "等待 {BLOCK_WAIT_DEADLINE:?} 仍未观察到停在 {table} 插入上的本池会话：\
                 被测建单没有走到插入语句，或表锁没能挡住它，判据空转即判红"
            );
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    };

    // 行数必须在放行之前读：这一步才是"该事务尚未提交"的直接证据
    let pending_visible_rows = visible_row_count(probe, table).await;
    (evidence, pending_visible_rows, blocker, handle)
}

/// 采集证据、放行表锁并等建单调用返回，然后断言正向判据（取号锁覆盖到插入语句）。
/// 返回锁证据与建单结果供用例继续断言业务落库。
async fn assert_lock_covers_pending_insert<T>(
    admin: &DatabaseConnection,
    probe: &DatabaseConnection,
    table: &'static str,
    doc_column: &str,
    work: impl FnOnce() -> JoinHandle<T>,
) -> (LockEvidence, T) {
    let (evidence, pending_visible_rows, blocker, handle) =
        open_blocked_window(admin, probe, table, work).await;
    blocker.rollback().await.expect("放行目标表上的阻塞锁失败");
    let result = handle.await.expect("被测建单任务不应 panic");

    assert!(
        evidence.advisory_held_in_same_xact,
        "取号锁必须覆盖到插入语句：{table} 的插入停在 pid {} 的虚拟事务 {} 上，\
         该会话在同一虚拟事务内必须已持有取号所用的 advisory 咨询锁；取号一旦挪到调用方\
         事务之外，锁会随那个内层事务提交提前释放，两次并发建单就会取到同一单号并撞 \
         {doc_column} 唯一约束（当时语句: {}）",
        evidence.insert_pid, evidence.virtual_transaction, evidence.waiting_query
    );
    assert_eq!(
        pending_visible_rows, 0,
        "阻塞窗口内另一会话应读不到任何行（{table} 的建单事务必须仍未提交），实得 {pending_visible_rows} 行"
    );
    (evidence, result)
}

// =========================================================
// 夹具（FK 父行自种子；表结构唯一来源是 backend/migration，本文件不自建 DDL）
// =========================================================

async fn seed_operator(admin: &DatabaseConnection) {
    user::ActiveModel {
        id: Set(OPERATOR_ID),
        username: Set("docno_ap_lock_operator".to_string()),
        password_hash: Set("test-only-not-a-real-hash".to_string()),
        real_name: Set(Some("应付取号锁契约锁操作人".to_string())),
        is_active: Set(true),
        ..Default::default()
    }
    .insert(admin)
    .await
    .expect("种子操作人失败");
}

/// 种子供应商：suppliers 属封存参照表跨例不清空，自增插入后回读真实主键，不用显式 id
async fn seed_supplier(admin: &DatabaseConnection, tag: &str) -> i32 {
    let s = supplier::ActiveModel {
        supplier_code: Set(format!("SUP-{tag}")),
        supplier_name: Set("应付取号锁契约锁供应商".to_string()),
        supplier_short_name: Set("应锁".to_string()),
        supplier_type: Set("面料供应商".to_string()),
        credit_code: Set("91330000TEST00000X".to_string()),
        registered_address: Set("测试注册地址".to_string()),
        legal_representative: Set("测试法人".to_string()),
        registered_capital: Set(Decimal::ZERO),
        establishment_date: Set(chrono::NaiveDate::from_ymd_opt(2020, 1, 1).expect("夹具日期常量")),
        taxpayer_type: Set("一般纳税人".to_string()),
        bank_name: Set("测试银行".to_string()),
        bank_account: Set("6222000000000000".to_string()),
        contact_phone: Set("13800000000".to_string()),
        created_at: Set(now().into()),
        updated_at: Set(now().into()),
        is_processor: Set(false),
        ..Default::default()
    }
    .insert(admin)
    .await
    .expect("种子供应商失败");
    s.id
}

async fn seed_warehouse(admin: &DatabaseConnection, tag: &str) -> i32 {
    let w = warehouse::ActiveModel {
        warehouse_code: Set(format!("WH-{tag}")),
        name: Set("应付取号锁契约锁仓".to_string()),
        is_default: Set(false),
        is_active: Set(true),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(admin)
    .await
    .expect("种子仓库失败");
    w.id
}

/// 种一行质检合格的采购入库单（结算门控只放行 PASSED；零金额零明细，使 commit 后的
/// 应付凭证生成按既有零额口径显式跳过，不把凭证链路牵进本锁）
async fn seed_passed_receipt(admin: &DatabaseConnection, tag: &str, supplier_id: i32) -> i32 {
    let warehouse_id = seed_warehouse(admin, tag).await;
    let r = purchase_receipt::ActiveModel {
        receipt_no: Set(format!("GR-{tag}")),
        supplier_id: Set(supplier_id),
        receipt_date: Set(today()),
        warehouse_id: Set(warehouse_id),
        inspection_status: Set(purchase_receipt_inspection::PASSED.to_string()),
        receipt_status: Set(receipt_status::DRAFT.to_string()),
        total_quantity: Set(Decimal::ZERO),
        total_quantity_alt: Set(Decimal::ZERO),
        total_amount: Set(Decimal::ZERO),
        created_by: Set(OPERATOR_ID),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(admin)
    .await
    .expect("种子采购入库单失败");
    r.id
}

/// 种一行采购退货单（零金额，使 commit 后的红字冲销凭证按既有零额口径跳过）
async fn seed_return(admin: &DatabaseConnection, tag: &str, supplier_id: i32) -> i32 {
    let r = purchase_return::ActiveModel {
        return_no: Set(format!("RT-{tag}")),
        supplier_id: Set(supplier_id),
        return_date: Set(today()),
        return_status: Set(Some(pr_status::DRAFT.to_string())),
        total_quantity: Set(Some(Decimal::ZERO)),
        total_quantity_alt: Set(Some(Decimal::ZERO)),
        total_amount: Set(Some(Decimal::ZERO)),
        created_by: Set(Some(OPERATOR_ID)),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(admin)
    .await
    .expect("种子采购退货单失败");
    r.id
}

/// 种一行已审批的付款申请（付款单创建的父行）
async fn seed_approved_request(admin: &DatabaseConnection, tag: &str, supplier_id: i32) -> i32 {
    let r = ap_payment_request::ActiveModel {
        request_no: Set(format!("PRQ-{tag}")),
        request_date: Set(today()),
        supplier_id: Set(supplier_id),
        payment_type: Set("FINAL".to_string()),
        payment_method: Set("TT".to_string()),
        request_amount: Set(dec("100.00")),
        approval_status: Set(STATUS_APPROVED.to_string()),
        created_by: Set(OPERATOR_ID),
        ..Default::default()
    }
    .insert(admin)
    .await
    .expect("种子已审批付款申请失败");
    r.id
}

/// 当日单号前缀（前缀字面量与各 service facade 的 impl_generate_no! 声明同源）
fn date_prefix(prefix: &str) -> String {
    format!("{}{}", prefix, Utc::now().format("%Y%m%d"))
}

/// 单号形态顺带钉住：前缀加年月日加三位流水
fn assert_no_shape(no: &str, prefix: &str) {
    let head = date_prefix(prefix);
    assert!(
        no.starts_with(&head),
        "单号应以 {prefix} 前缀与当日日期起头，实得 {no}"
    );
    assert_eq!(
        no.len(),
        head.len() + 3,
        "单号总长应为前缀日期段加三位流水，实得 {no}"
    );
}

// =========================================================
// 用例一：facade 直连锁（事务内取号持锁到事务结束、回滚即释放、只取号不落库）
// =========================================================

#[tokio::test]
async fn ap_invoice_number_taken_on_caller_txn_holds_advisory_lock_until_rollback() {
    let admin = setup_test_db().await;
    let probe = probe_pool().await;
    let pool = service_pool().await;
    let service = ApInvoiceService::new(Arc::new(pool.clone()));

    assert_eq!(
        pool_advisory_lock_count(&probe).await,
        0,
        "调用前本测试连接池不应持有任何 advisory 锁（基线自证）"
    );

    let txn = pool.begin().await.expect("开启取号事务失败");
    let invoice_no = service
        .generate_invoice_no(&txn)
        .await
        .expect("调用方事务内取号失败");
    assert_no_shape(&invoice_no, "API");

    assert!(
        pool_advisory_lock_count(&probe).await > 0,
        "取号锁必须存活到调用方事务结束：事务未提交前本池会话应持有 advisory 锁"
    );

    txn.rollback().await.expect("回滚取号事务失败");
    assert_eq!(
        pool_advisory_lock_count(&probe).await,
        0,
        "调用方事务结束后取号锁必须释放，不得泄漏"
    );
    assert_eq!(
        visible_row_count(&probe, TABLE_AP_INVOICE).await,
        0,
        "只取号不插入的路径不得在 ap_invoice 留下任何行"
    );
    drop(admin);
}

// =========================================================
// 用例二：手工创建应付单（ap_invoice_ops crud 的 create_manual）
// =========================================================

#[tokio::test]
async fn manual_ap_invoice_create_numbers_and_inserts_in_one_uncommitted_transaction() {
    let admin = setup_test_db().await;
    let probe = probe_pool().await;
    let pool = service_pool().await;
    seed_operator(&admin).await;
    let supplier_id = seed_supplier(&admin, "APMANUAL").await;

    let req = CreateApInvoiceRequest {
        supplier_id: Some(supplier_id),
        invoice_type: None,
        invoice_date: Some(today()),
        due_date: Some(today()),
        payment_terms: Some(DEFAULT_PAYMENT_TERMS_DAYS),
        amount: Some(dec("100.00")),
        currency: None,
        exchange_rate: None,
        tax_amount: None,
        notes: None,
        attachment_urls: None,
    };

    let (_evidence, invoice) = assert_lock_covers_pending_insert(
        &admin,
        &probe,
        TABLE_AP_INVOICE,
        "ap_invoice.invoice_no",
        move || {
            tokio::spawn(async move {
                let service = ApInvoiceService::new(Arc::new(pool));
                service.create_manual(req, OPERATOR_ID).await
            })
        },
    )
    .await;
    let invoice = invoice.expect("手工创建应付单必须成功");
    assert_no_shape(&invoice.invoice_no, "API");
    assert_eq!(
        invoice.invoice_status, STATUS_DRAFT,
        "手工单初始状态词表同源"
    );
    assert_eq!(
        visible_row_count(&probe, TABLE_AP_INVOICE).await,
        1,
        "放行表锁后建单事务必须真的提交落库"
    );
    assert_eq!(
        pool_advisory_lock_count(&probe).await,
        0,
        "建单事务结束后取号锁必须释放"
    );
}

// =========================================================
// 用例三：入库结算自动生成应付单（ap_invoice_ops receipt 的收货入口）
// =========================================================

#[tokio::test]
async fn receipt_driven_ap_invoice_create_numbers_and_inserts_in_one_uncommitted_transaction() {
    let admin = setup_test_db().await;
    let probe = probe_pool().await;
    let pool = service_pool().await;
    seed_operator(&admin).await;
    let supplier_id = seed_supplier(&admin, "APRECEIPT").await;
    let receipt_id = seed_passed_receipt(&admin, "APRECEIPT", supplier_id).await;

    let (_evidence, invoice) = assert_lock_covers_pending_insert(
        &admin,
        &probe,
        TABLE_AP_INVOICE,
        "ap_invoice.invoice_no",
        move || {
            tokio::spawn(async move {
                let service = ApInvoiceService::new(Arc::new(pool));
                service
                    .auto_generate_from_receipt(receipt_id, OPERATOR_ID)
                    .await
            })
        },
    )
    .await;
    let invoice = invoice.expect("合格入库单生成应付必须成功");
    assert_no_shape(&invoice.invoice_no, "API");
    assert_eq!(
        invoice.source_type.as_deref(),
        Some("PURCHASE_RECEIPT"),
        "结算入口生成的应付单必须回指入库单来源"
    );
    assert_eq!(
        visible_row_count(&probe, TABLE_AP_INVOICE).await,
        1,
        "放行表锁后应付单必须真的提交落库"
    );
    assert_eq!(
        pool_advisory_lock_count(&probe).await,
        0,
        "建单事务结束后取号锁必须释放"
    );
}

// =========================================================
// 用例四：退货红字应付（ap_invoice_ops receipt 的退货入口）
// =========================================================

#[tokio::test]
async fn return_driven_ap_invoice_create_numbers_and_inserts_in_one_uncommitted_transaction() {
    let admin = setup_test_db().await;
    let probe = probe_pool().await;
    let pool = service_pool().await;
    seed_operator(&admin).await;
    let supplier_id = seed_supplier(&admin, "APRETURN").await;
    let return_id = seed_return(&admin, "APRETURN", supplier_id).await;

    let (_evidence, invoice) = assert_lock_covers_pending_insert(
        &admin,
        &probe,
        TABLE_AP_INVOICE,
        "ap_invoice.invoice_no",
        move || {
            tokio::spawn(async move {
                let service = ApInvoiceService::new(Arc::new(pool));
                service
                    .auto_generate_from_return(return_id, OPERATOR_ID)
                    .await
            })
        },
    )
    .await;
    let invoice = invoice.expect("退货单生成红字应付必须成功");
    assert_no_shape(&invoice.invoice_no, "API");
    assert_eq!(
        invoice.source_type.as_deref(),
        Some("PURCHASE_RETURN"),
        "退货入口生成的应付单必须回指退货来源"
    );
    assert_eq!(
        visible_row_count(&probe, TABLE_AP_INVOICE).await,
        1,
        "放行表锁后红字应付必须真的提交落库"
    );
    assert_eq!(
        pool_advisory_lock_count(&probe).await,
        0,
        "建单事务结束后取号锁必须释放"
    );
}

// =========================================================
// 用例五：付款申请创建（ap_payment_request_service 的 create）
// =========================================================

#[tokio::test]
async fn payment_request_create_numbers_and_inserts_in_one_uncommitted_transaction() {
    let admin = setup_test_db().await;
    let probe = probe_pool().await;
    let pool = service_pool().await;
    seed_operator(&admin).await;
    let supplier_id = seed_supplier(&admin, "APREQ").await;

    let req = CreateRequestDto {
        supplier_id,
        request_date: today(),
        payment_type: "FINAL".to_string(),
        payment_method: "TT".to_string(),
        request_amount: dec("100.00"),
        currency: None,
        exchange_rate: None,
        expected_payment_date: None,
        bank_name: None,
        bank_account: None,
        bank_account_name: None,
        notes: None,
        attachment_urls: None,
        items: None,
    };

    let (_evidence, request) = assert_lock_covers_pending_insert(
        &admin,
        &probe,
        TABLE_AP_PAYMENT_REQUEST,
        "ap_payment_request.request_no",
        move || {
            tokio::spawn(async move {
                let service = ApPaymentRequestService::new(Arc::new(pool));
                service.create(req, OPERATOR_ID).await
            })
        },
    )
    .await;
    let request = request.expect("创建付款申请必须成功");
    assert_no_shape(&request.request_no, "PRQ");
    assert_eq!(
        visible_row_count(&probe, TABLE_AP_PAYMENT_REQUEST).await,
        1,
        "放行表锁后付款申请必须真的提交落库"
    );
    assert_eq!(
        pool_advisory_lock_count(&probe).await,
        0,
        "建单事务结束后取号锁必须释放"
    );
}

// =========================================================
// 用例六：付款单创建（ap_payment_service 的 create）
// =========================================================

#[tokio::test]
async fn payment_create_numbers_and_inserts_in_one_uncommitted_transaction() {
    let admin = setup_test_db().await;
    let probe = probe_pool().await;
    let pool = service_pool().await;
    seed_operator(&admin).await;
    let supplier_id = seed_supplier(&admin, "APPAY").await;
    let request_id = seed_approved_request(&admin, "APPAY", supplier_id).await;

    let req = CreatePaymentDto {
        request_id,
        payment_date: today(),
        notes: None,
        attachment_urls: None,
    };

    let (_evidence, payment) = assert_lock_covers_pending_insert(
        &admin,
        &probe,
        TABLE_AP_PAYMENT,
        "ap_payment.payment_no",
        move || {
            tokio::spawn(async move {
                let service = ApPaymentService::new(Arc::new(pool));
                service.create(req, OPERATOR_ID).await
            })
        },
    )
    .await;
    let payment = payment.expect("从已审批付款申请创建付款单必须成功");
    assert_no_shape(&payment.payment_no, "PAY");
    assert_eq!(
        visible_row_count(&probe, TABLE_AP_PAYMENT).await,
        1,
        "放行表锁后付款单必须真的提交落库"
    );
    assert_eq!(
        pool_advisory_lock_count(&probe).await,
        0,
        "建单事务结束后取号锁必须释放"
    );
}

// =========================================================
// 用例七：变异自证（缺陷形态必须让上面的正向判据为假）
// =========================================================

/// 按缺陷形态复现：用自开事务的 DocumentNumberGenerator::generate_no 取号，
/// 再另开一条会话的事务插入同一张表。取号返回时锁已随内层事务提交释放，
/// 停在插入上的会话不再持任何取号锁，正向判据在此必须为假。
#[tokio::test]
async fn mutation_self_opening_numbering_leaves_the_pending_insert_unlocked() {
    let admin = setup_test_db().await;
    let probe = probe_pool().await;
    let pool = service_pool().await;
    seed_operator(&admin).await;
    let supplier_id = seed_supplier(&admin, "APMUT").await;

    let mutation_no = DocumentNumberGenerator::generate_no(
        &pool,
        "API",
        ap_invoice::Entity,
        ap_invoice::Column::InvoiceNo,
    )
    .await
    .expect("自开事务形态取号失败");
    assert_no_shape(&mutation_no, "API");
    assert_eq!(
        pool_advisory_lock_count(&probe).await,
        0,
        "自开事务取号返回后锁应已释放——这正是被取代的缺陷形态留下的并发窗口"
    );

    let blocker = block_table_for_inserts(&admin, TABLE_AP_INVOICE).await;
    let insert_conn = pool.clone();
    let handle = tokio::spawn(async move {
        let txn = insert_conn.begin().await.expect("缺陷形态插入事务开启失败");
        let model = ap_invoice::ActiveModel {
            invoice_no: Set(mutation_no),
            supplier_id: Set(supplier_id),
            invoice_type: Set("PURCHASE".to_string()),
            source_type: Set(Some("MANUAL".to_string())),
            invoice_date: Set(today()),
            due_date: Set(today()),
            payment_terms: Set(DEFAULT_PAYMENT_TERMS_DAYS),
            amount: Set(dec("100.00")),
            paid_amount: Set(Decimal::ZERO),
            unpaid_amount: Set(dec("100.00")),
            invoice_status: Set(STATUS_DRAFT.to_string()),
            currency: Set(DEFAULT_CURRENCY.to_string()),
            exchange_rate: Set(Decimal::ONE),
            tax_amount: Set(Decimal::ZERO),
            created_by: Set(OPERATOR_ID),
            ..Default::default()
        };
        let inserted = model
            .insert(&txn)
            .await
            .expect("缺陷形态的插入最终应成功落库");
        txn.commit().await.expect("缺陷形态插入事务提交失败");
        inserted
    });

    let deadline = Instant::now() + BLOCK_WAIT_DEADLINE;
    let evidence = loop {
        match query_blocked_insert(&probe, TABLE_AP_INVOICE).await {
            Ok(Some(evidence)) => break evidence,
            Ok(None) => {}
            Err(message) => {
                let _ = blocker.rollback().await;
                panic!("{message}");
            }
        }
        if Instant::now() >= deadline {
            let _ = blocker.rollback().await;
            let _ = handle.await;
            panic!(
                "等待 {BLOCK_WAIT_DEADLINE:?} 仍未观察到停在 ap_invoice 插入上的会话，\
                 变异自证空跑即判红"
            );
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    };

    blocker.rollback().await.expect("放行阻塞表锁失败");
    let inserted = handle.await.expect("缺陷形态插入任务不应 panic");

    assert!(
        !evidence.advisory_held_in_same_xact,
        "变异自证失效：缺陷形态下停在插入语句上的会话（pid {} 虚拟事务 {}）竟被读到持有\
         同事务 advisory 锁，说明正向判据是恒真读数、测不到锁的作用域（当时语句: {}）",
        evidence.insert_pid, evidence.virtual_transaction, evidence.waiting_query
    );
    assert_eq!(
        inserted.supplier_id, supplier_id,
        "变异形态的插入本身要真的落库，否则对照在空转"
    );
    assert_eq!(
        visible_row_count(&probe, TABLE_AP_INVOICE).await,
        1,
        "变异形态落库行数自证"
    );
}

// =========================================================
// 源码级防回潮锁（与活体锁配对：活体锁判行为，本锁判形态，形态一退回立刻点名）
// =========================================================

/// 读取本 crate 内源码（内容按原样取，空白在判定前统一剔除）
fn read_src(rel: &str) -> String {
    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("读取 {rel} 失败: {e}"))
}

/// 去掉全部空白后的内容：宏注册与调用点的匹配不受换行与缩进影响
fn flattened(src: &str) -> String {
    src.chars().filter(|c| !c.is_whitespace()).collect()
}

/// 防回潮锁一：三处 facade 的取号宏注册必须是带调用方连接的形态（第五个元变量 txn），
/// 四元变量的无参形态一旦回潮，门面方法就会重新自开事务取号，锁随内层事务提交提前释放
#[test]
fn source_scan_ap_numbering_registrations_keep_the_caller_txn_form() {
    for (file, registration) in [
        (
            "src/services/ap_invoice_service.rs",
            "impl_generate_no!(generate_invoice_no,\"API\",crate::models::ap_invoice::Entity,\
             crate::models::ap_invoice::Column::InvoiceNo,txn);",
        ),
        (
            "src/services/ap_payment_request_service.rs",
            "impl_generate_no!(generate_request_no,\"PRQ\",ap_payment_request::Entity,\
             ap_payment_request::Column::RequestNo,txn);",
        ),
        (
            "src/services/ap_payment_service.rs",
            "impl_generate_no!(generate_payment_no,\"PAY\",ap_payment::Entity,\
             ap_payment::Column::PaymentNo,txn);",
        ),
    ] {
        let registration = registration.replace(' ', "");
        let flat = flattened(&read_src(file));
        assert!(
            flat.contains(&registration),
            "{file} 的取号宏注册必须是带 txn 的形态（只保留调用方事务内取号），实际未匹配到:\n{registration}"
        );
        let bare = registration.replace(",txn);", ");");
        assert!(
            !flat.contains(&bare),
            "{file} 不得退回不带调用方连接的无参注册形态:\n{bare}"
        );
    }
}

/// 防回潮锁二：五处调用点必须把已开启的建单事务传进去（按文件逐处计次，
/// 少一处或多一处都说明入口形态漂移）
#[test]
fn source_scan_ap_create_sites_pass_the_open_transaction() {
    for (file, call, expected) in [
        (
            "src/services/ap_invoice_ops/crud.rs",
            "self.generate_invoice_no(&txn).await?",
            1,
        ),
        (
            "src/services/ap_invoice_ops/receipt.rs",
            "self.generate_invoice_no(&txn).await?",
            2,
        ),
        (
            "src/services/ap_payment_request_service.rs",
            "self.generate_request_no(&txn).await?",
            1,
        ),
        (
            "src/services/ap_payment_service.rs",
            "self.generate_payment_no(&txn).await?",
            1,
        ),
    ] {
        let flat = flattened(&read_src(file));
        assert_eq!(
            flat.matches(call).count(),
            expected,
            "{file} 必须有 {expected} 处以事务内形态取号（判据 {call}），实得 {} 处",
            flat.matches(call).count()
        );
    }
}

/// 防回潮锁三：上述四个文件内不得再出现无参取号调用，也不得把取号挪到 begin 之前
#[test]
fn source_scan_ap_create_sites_have_no_bare_numbering_call() {
    for (file, bare) in [
        (
            "src/services/ap_invoice_ops/crud.rs",
            "self.generate_invoice_no()",
        ),
        (
            "src/services/ap_invoice_ops/receipt.rs",
            "self.generate_invoice_no()",
        ),
        (
            "src/services/ap_payment_request_service.rs",
            "self.generate_request_no()",
        ),
        (
            "src/services/ap_payment_service.rs",
            "self.generate_payment_no()",
        ),
    ] {
        let flat = flattened(&read_src(file));
        assert!(
            !flat.contains(bare),
            "{file} 仍留有事务外取号调用形态 {bare}：取号必须落在建单事务之内"
        );
    }
}
