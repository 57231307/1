//! 客户编码并发与撞键重试的活库契约锁
//!
//! 与源码级锁 contract_docno_customer_txn_arm_source_lock_test.rs 互补，本锁测运行时行为。
//! 钉三件事：
//! 1. 顺序两次自动建客户各取不同编码且都真实落库（取号走当日号段顺延）；
//! 2. 并发两次自动建客户两笔都必须成功且编码互异，不得出现 DATABASE_ERROR
//!    （修复前取号在事务外自开、另起事务 INSERT，同秒并发各取同号，一枚 INSERT
//!    撞 customers_customer_code_key 唯一约束被原样抛成数据库错误，正是本用例所钉的崩溃面）；
//! 3. 撞键重试路径，旁路连接先占当日首号并保持未提交，令服务侧 INSERT 阻塞在同键上，
//!    观测到阻塞后提交旁路，服务侧收到 23505 后必须在保存点内重新取号顺延成功，
//!    写后按主键读回的真实编码必须等于返回编码；若未观测到阻塞（服务已先看到旁路行并顺延），
//!    接受成功回落并钉两号不同且不落重复。任何结果都禁止是 DATABASE_ERROR 类错误。
//! 调用方为集成测试 crate，仅在 CI 对已跑完迁移链的 PostgreSQL 执行，
//! 活体串行组逐条跑（--test-threads=1，共享真库夹具，并行会互相 TRUNCATE）。
//! 入参为 test_common::setup_test_db 注入业务连接（夹具每例 TRUNCATE 业务表，customers 属之），
//! 旁路连接按 TEST_DATABASE_URL 另开一条，确保与业务连接不在同一后端会话才能互相阻塞。
//! ES 同步用 ElasticClient::mock，落库判定只认 PostgreSQL，不依赖 ES。
//! 探针行与建单行由 setup_test_db 的逐用例 TRUNCATE 回收，冲突用例额外显式清理。

mod test_common;

use bingxi_backend::models::customer;
use bingxi_backend::search::{ElasticClient, SearchClient};
use bingxi_backend::services::customer_service::{CreateCustomerArgs, CustomerService};
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ConnectionTrait, Database, DatabaseConnection, DbBackend, EntityTrait, Statement,
    TransactionTrait,
};
use std::sync::Arc;
use test_common::setup_test_db;

/// 当日客户编码前缀（与生成器口径一致，CUS 加 YYYYMMDD）
fn date_prefix() -> String {
    format!("CUS{}", Utc::now().format("%Y%m%d"))
}

/// 另开一条旁路连接（与业务连接不同会话，才能持有未提交占位行并互相阻塞）
async fn bypass_connection() -> DatabaseConnection {
    let url = std::env::var("TEST_DATABASE_URL")
        .expect("并发与撞键用例需要 TEST_DATABASE_URL（与业务夹具同一 PostgreSQL）");
    Database::connect(url)
        .await
        .expect("旁路连接建立失败（PostgreSQL 不可达）")
}

/// 判定错误是否为被禁止的数据库错误类（原始唯一键冲突抛给用户的形态）
fn is_database_error(e: &bingxi_backend::utils::error::AppError) -> bool {
    let display = format!("{e}");
    let debug = format!("{e:?}");
    display.contains("数据库错误")
        || display.contains("DATABASE_ERROR")
        || debug.contains("DATABASE_ERROR")
        || debug.contains("duplicate key")
        || display.contains("duplicate key")
}

/// 构造一枚自动取号（customer_code=None）、必填列齐备的建客户入参
fn make_args(name: &str) -> CreateCustomerArgs {
    CreateCustomerArgs {
        customer_code: None,
        customer_name: name.to_string(),
        contact_person: None,
        contact_phone: None,
        contact_email: None,
        address: None,
        city: None,
        province: None,
        country: Some("中国".to_string()),
        postal_code: None,
        credit_limit: Decimal::ZERO,
        payment_terms: 30,
        tax_id: None,
        bank_name: None,
        bank_account: None,
        customer_type: bingxi_backend::constants::customer_type::OTHER.to_string(),
        status: None,
        customer_industry: None,
        main_products: None,
        annual_purchase: None,
        quality_requirement: None,
        inspection_standard: None,
        notes: None,
        created_by: Some(1),
    }
}

/// 建一个带 mock ES 客户端的客户域服务实例
fn make_service(db: Arc<DatabaseConnection>) -> CustomerService {
    let search_client: Arc<dyn SearchClient> = Arc::new(ElasticClient::mock());
    CustomerService::new(db, search_client)
}

/// 顺序两次自动建客户，各取不同编码且都真实落库
#[tokio::test]
async fn customer_sequential_create_yields_distinct_code() {
    let db = Arc::new(setup_test_db().await);
    let service = make_service(db.clone());

    let first = service
        .create_customer(make_args("顺序客户甲"))
        .await
        .expect("顺序建客户第一次应成功，不得返回数据库错误");
    let second = service
        .create_customer(make_args("顺序客户乙"))
        .await
        .expect("顺序建客户第二次应成功，不得返回数据库错误");

    let prefix = date_prefix();
    assert!(
        first.customer_code.starts_with(&prefix) && second.customer_code.starts_with(&prefix),
        "自动编码应以当日 CUS 前缀起头，实得 {} 与 {}",
        first.customer_code,
        second.customer_code
    );
    assert_ne!(
        first.customer_code, second.customer_code,
        "顺序两次自动建客户不得取到同一编码，实得两枚同为 {}",
        first.customer_code
    );

    let rows = customer::Entity::find()
        .all(&*db)
        .await
        .expect("回读客户行数失败");
    assert_eq!(
        rows.len(),
        2,
        "两次成功建客户应各落一行，实得 {} 行",
        rows.len()
    );
}

/// 并发两次自动建客户，两笔都必须成功且编码互异，禁止 DATABASE_ERROR
#[tokio::test]
async fn customer_concurrent_create_no_database_error() {
    let db = Arc::new(setup_test_db().await);
    let service = Arc::new(make_service(db.clone()));

    let svc_a = service.clone();
    let svc_b = service.clone();
    let (ra, rb) = tokio::join!(
        svc_a.create_customer(make_args("并发客户甲")),
        svc_b.create_customer(make_args("并发客户乙"))
    );

    let a = ra.expect("并发建客户第一次不得返回 DATABASE_ERROR，应成功落库");
    let b =
        rb.expect("并发建客户第二次不得返回 DATABASE_ERROR，应成功落库（撞号走保存点重试顺延）");

    assert_ne!(
        a.customer_code, b.customer_code,
        "并发建客户不得取到同一编码，实得两枚同为 {}",
        a.customer_code
    );

    let rows = customer::Entity::find()
        .all(&*db)
        .await
        .expect("回读客户行数失败");
    assert_eq!(
        rows.len(),
        2,
        "两次成功建客户应各落一行，实得 {} 行",
        rows.len()
    );
}

/// 撞键重试路径，旁路先占当日首号并保持未提交，服务侧 INSERT 阻塞后收到 23505，
/// 必须在保存点内重新取号顺延成功，写后读回真实编码等于返回编码，全程禁止 DATABASE_ERROR。
#[tokio::test]
async fn customer_conflict_retry_recovers_without_database_error() {
    let db = Arc::new(setup_test_db().await);
    let bypass = bypass_connection().await;

    let taken_no = format!("{}001", date_prefix());
    let btxn = bypass.begin().await.expect("旁路事务开启失败");
    let seed_values: Vec<sea_orm::Value> = vec![
        taken_no.clone().into(),
        "旁路占号客户".into(),
        bingxi_backend::constants::customer_type::OTHER
            .to_string()
            .into(),
    ];
    btxn.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO customers (customer_code, customer_name, customer_type) VALUES ($1, $2, $3)",
        seed_values,
    ))
    .await
    .expect("旁路占号插入失败");

    let service = make_service(db.clone());
    let create_task =
        tokio::spawn(async move { service.create_customer(make_args("撞号重试客户")).await });

    // 轮询服务侧 INSERT 是否已进入等待（被旁路未提交行的唯一索引冲突阻塞），
    // 命中即提交旁路事务，令服务侧同刻收到 23505 走保存点重试顺延
    let mut observed_blocking = false;
    for _ in 0..100 {
        let blocked = bypass
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT COUNT(*) AS cnt FROM pg_stat_activity \
                 WHERE state = 'active' AND query ILIKE '%INSERT INTO \\\"customers\\\"%' \
                   AND pid <> pg_backend_pid()"
                    .to_string(),
            ))
            .await
            .ok()
            .flatten()
            .and_then(|r| r.try_get_by_index::<i64>(0).ok())
            .unwrap_or(0);
        if blocked > 0 {
            observed_blocking = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    btxn.commit().await.expect("旁路占号事务提交失败");

    let result = create_task.await.expect("建客户任务 join 失败");

    match result {
        Err(e) => {
            assert!(
                !is_database_error(&e),
                "撞唯一约束禁止被原样抛成 DATABASE_ERROR（本缺陷的崩溃面），实得 {}",
                e
            );
            // 允许有界重试耗尽后的业务文案，但不允许数据库错误；耗尽路径同样要显式失败可见
            assert!(
                observed_blocking,
                "报错路径应先观测到服务侧 INSERT 阻塞，观测缺失说明交错异常，错误为 {}",
                e
            );
        }
        Ok(model) => {
            // 写后按主键读回真实编码，钉返回体与库中一致
            let reread = customer::Entity::find_by_id(model.id)
                .one(&*db)
                .await
                .expect("按主键读回客户失败")
                .expect("服务建出的客户应能按主键读回");
            assert_eq!(
                reread.customer_code, model.customer_code,
                "返回编码必须等于库中真实编码，实得返回 {} 读回 {}",
                model.customer_code, reread.customer_code
            );
            assert_ne!(
                model.customer_code, taken_no,
                "未观测到阻塞时服务取号必须已看到旁路行并顺延，绝不允许撞出同号双行，\
                 实得旁路 {} 与服务 {}",
                taken_no, model.customer_code
            );
            assert!(
                !observed_blocking || reread.customer_code != taken_no,
                "观测到阻塞后服务侧应经保存点重试顺延到非旁路号，实得 {}",
                reread.customer_code
            );
        }
    }

    let live = customer::Entity::find()
        .all(&*db)
        .await
        .expect("回读客户失败");
    assert!(
        !live
            .iter()
            .any(|r| r.customer_code == taken_no && r.customer_name == "撞号重试客户"),
        "服务侧不得与旁路撞出同一编码的双行"
    );
    for row in live.iter().filter(|r| r.customer_code == taken_no) {
        customer::Entity::delete_by_id(row.id)
            .exec(&*db)
            .await
            .expect("旁路探针行清理失败");
    }
}
