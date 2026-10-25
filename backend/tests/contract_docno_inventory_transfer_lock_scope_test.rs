//! 库存调拨单号取号锁作用域的活库契约锁
//!
//! 功能：钉调拨单号取号把前缀级 advisory 锁占用在调用方事务的整个存活期内。
//!       判据是锁的可见性而不是文案：事务未结束前，另一条连接读 pg_locks 必须看到
//!       advisory 锁条数比基线多；事务结束后必须回落到基线。取号一旦改用自开事务的
//!       形态，锁随那个内层事务提交立即释放，建单事务的 INSERT 之前就有了并发窗口，
//!       两次并发建单会取到同一调拨单号并撞 transfer_no 唯一约束。
//! 反向钉：锁不在册或事务结束后仍在册（锁泄漏）都判红。
//! 变异自检：第二条用例显式调用自开事务的 DocumentNumberGenerator::generate_no，
//!       断言其返回后 advisory 锁条数与基线相等——即缺陷形态确实不留锁，
//!       证明主判据测的是锁的作用域，不是"库里恒定有锁"这种恒真读数。
//! 调用方：集成测试 crate（CI 里对已跑完迁移链的 PostgreSQL 执行，nextest 串行组逐条跑）。
//! 入参：test_common::setup_test_db 注入业务连接；探测连接按 TEST_DATABASE_URL 另开一条，
//!       确保与业务连接不在同一后端会话，才能看见对方持有的锁。
//! 传给谁：判据结果交给断言本身，无下游。
//! 存什么·存哪里：不落任何业务行——取号只做号段探测，主用例以回滚收尾，
//!       探测连接只读 pg_locks 计数，故无需任何父表种子。

mod test_common;

use bingxi_backend::models::inventory_transfer;
use bingxi_backend::services::inv::InventoryTransferService;
use bingxi_backend::utils::number_generator::DocumentNumberGenerator;
use chrono::Utc;
use sea_orm::{
    ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement, TransactionTrait,
};
use std::sync::Arc;
use test_common::setup_test_db;

/// 另开一条探测连接（与业务连接不同会话，才能看见业务会话持有的锁）
async fn probe_connection() -> DatabaseConnection {
    let url = std::env::var("TEST_DATABASE_URL")
        .expect("探测 pg_locks 需要 TEST_DATABASE_URL（与业务夹具同一 PostgreSQL）");
    Database::connect(url)
        .await
        .expect("探测连接建立失败（PostgreSQL 不可达？）")
}

/// 读当前会话集里 advisory 锁的条数（只读，不申请任何锁）
async fn advisory_lock_count(db: &DatabaseConnection) -> i64 {
    let row = db
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT COUNT(*) AS cnt FROM pg_locks WHERE locktype = 'advisory'".to_string(),
        ))
        .await
        .expect("读取 pg_locks 的 advisory 锁条数失败");
    row.and_then(|r| r.try_get_by_index::<i64>(0).ok())
        .expect("advisory 锁条数解码失败")
}

/// 主锁：事务内取号后锁必须随调用方事务存活，事务结束时释放
#[tokio::test]
async fn transfer_no_taken_on_caller_txn_keeps_advisory_lock_until_txn_ends() {
    let db = setup_test_db().await;
    let probe = probe_connection().await;
    let baseline = advisory_lock_count(&probe).await;

    let txn = db.begin().await.expect("开启建单事务失败");
    let service = InventoryTransferService::new(Arc::new(db.clone()));
    let transfer_no = service
        .generate_transfer_no(&txn)
        .await
        .expect("建单事务内取号失败");

    // 单号形态顺带钉住：前缀 TRF 加年月日加三位流水（allocate_no 的格式口径，
    // 前缀字面量与 facade 的 impl_generate_no! 声明一致，漂移由
    // tests/contract_wave2_document_no_parity_test.rs 的格式对拍拦截）
    let date_prefix = format!("TRF{}", Utc::now().format("%Y%m%d"));
    assert!(
        transfer_no.starts_with(&date_prefix),
        "单号应以调拨前缀与当日日期起头，实得 {transfer_no}"
    );
    assert_eq!(
        transfer_no.len(),
        date_prefix.len() + 3,
        "单号总长应为前缀日期段加三位流水，实得 {transfer_no}"
    );

    let held = advisory_lock_count(&probe).await;
    assert!(
        held > baseline,
        "取号锁必须存活到调用方事务结束：事务未提交前另一连接看到的 advisory 锁\
         应比基线 {baseline} 条更多，实得 {held} 条"
    );

    txn.rollback().await.expect("回滚取号事务失败");
    assert_eq!(
        advisory_lock_count(&probe).await,
        baseline,
        "调用方事务结束后取号锁必须释放，不得泄漏"
    );
}

/// 变异自检：自开事务的取号形态在自己的内层事务提交后就放锁，
/// 与主判据构成对照，证明主判据不是恒真读数
#[tokio::test]
async fn mutation_self_opening_generator_releases_lock_before_caller_writes_row() {
    let db = setup_test_db().await;
    let probe = probe_connection().await;
    let baseline = advisory_lock_count(&probe).await;

    let transfer_no = DocumentNumberGenerator::generate_no(
        &db,
        "TRF",
        inventory_transfer::Entity,
        inventory_transfer::Column::TransferNo,
    )
    .await
    .expect("自开事务形态取号失败");
    assert!(
        !transfer_no.is_empty(),
        "对照形态同样要真的取到号，否则下面的放锁断言是在空转"
    );

    assert_eq!(
        advisory_lock_count(&probe).await,
        baseline,
        "自开事务的取号返回后锁应已释放——这正是被取代的缺陷形态：\
         调用方还没写库，锁已经不在，两次并发取号会落到同一个号"
    );
}
