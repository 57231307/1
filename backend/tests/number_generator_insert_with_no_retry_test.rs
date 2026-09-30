//! 任务 #144：`DocumentNumberGenerator::insert_with_no_retry`（调用方事务内
//! SAVEPOINT 包「取号+INSERT」、仅 23505 重试、其余 SQL 错误原样上抛）真实行为契约。
//!
//! 全部打真实 PostgreSQL 真实表（`customers` / `crm_opportunity`），依赖迁移后的
//! schema、`pg_advisory_xact_lock`、SAVEPOINT 与 23505 语义，均为 PG 专属，
//! 故整体 `#[ignore]`，由 CI 专用 job（`--run-ignored only`）执行；需
//! `TEST_DATABASE_URL`。禁止 mock 数据库掩盖契约。
//!
//! 覆盖：
//! (a) 正常插入成功，单号符合 `{前缀}{YYYYMMDD}{NNN}`（默认 3 位流水）；
//! (b) 同一外层事务内连续多次「取号+插入」不撞号、得到连续号；
//! (c) 旁路写入制造真实并发 23505（另一连接插入同一候选号且**先不提交**，
//!     待本事务的 INSERT 被唯一索引等待挂起后再提交）→ `insert_with_no_retry`
//!     在保存点内回滚、重新取号重试成功，而不是把 23505 冒泡成裸 500；
//! (d) 非唯一约束 SQL 错误（故意违反外键：customer_id 指向不存在客户）
//!     **不被吞掉、不重试**：构建闭包只被调用一次，错误以 DatabaseError 原样上抛。

use bingxi_backend::models::{crm_opportunity, customer};
use bingxi_backend::utils::error::AppError;
use bingxi_backend::utils::number_generator::DocumentNumberGenerator;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction,
    EntityTrait, QueryFilter, Set, Statement, TransactionTrait,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

/// 活库连接：缺失 TEST_DATABASE_URL 即显式 panic（不静默回退 sqlite）
async fn live_db() -> DatabaseConnection {
    let url = std::env::var("TEST_DATABASE_URL")
        .expect("活库用例必须设置 TEST_DATABASE_URL（指向已跑完迁移的 PostgreSQL）");
    sea_orm::Database::connect(&url)
        .await
        .expect("测试夹具：活库连接失败")
}

fn today() -> String {
    Utc::now().format("%Y%m%d").to_string()
}

fn code(pfx: &str, seq: u32) -> String {
    format!("{}{}{:03}", pfx, today(), seq)
}

fn make_customer(no: String) -> customer::ActiveModel {
    let now = Utc::now();
    customer::ActiveModel {
        customer_code: Set(no.clone()),
        customer_name: Set(format!("取号重试测试客户 {no}")),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
        customer_type: Set("retail".to_string()),
        owner_id: Set(100),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
}

/// (d) 专用：除 customer_id 指向不存在的客户外，其余必填字段齐全，
/// 让外键约束成为第一个（也是唯一预期的）违例点。
fn make_opportunity_bad_fk(no: String) -> crm_opportunity::ActiveModel {
    crm_opportunity::ActiveModel {
        opportunity_no: Set(no),
        opportunity_name: Set("FK 违例探针商机".to_string()),
        customer_id: Set(2_000_000_123), // 远超真实序列的不存在客户 ID
        owner_id: Set(100),
        owner_name: Set("NGT4 探针".to_string()),
        ..Default::default()
    }
}

async fn cleanup_customer<C: sea_orm::ConnectionTrait>(db: &C, pfx: &str) {
    customer::Entity::delete_many()
        .filter(customer::Column::CustomerCode.starts_with(pfx))
        .exec(db)
        .await
        .unwrap_or_else(|e| panic!("清理客户前缀 {pfx} 失败: {e}"));
}

/// (a) 正常路径：insert_with_no_retry 插入成功，单号严格符合
/// `{前缀}{YYYYMMDD}{NNN}`；提交后真实落库；提交时刻外层事务无残留伤害。
#[tokio::test]
#[ignore = "需已迁移 PG（advisory lock + SAVEPOINT），由专用 job（--run-ignored only）执行"]
async fn insert_with_no_retry_happy_path_persists_standard_format() {
    let pfx = "NGT1";
    let db = live_db().await;
    cleanup_customer(&db, pfx).await;

    let txn = db.begin().await.expect("开启事务失败");
    let model = DocumentNumberGenerator::insert_with_no_retry(
        &txn,
        pfx,
        customer::Entity,
        customer::Column::CustomerCode,
        make_customer,
    )
    .await
    .expect("正常插入应成功返回 Model");
    txn.commit().await.expect("提交失败");

    let expected = code(pfx, 1);
    assert_eq!(
        model.customer_code, expected,
        "号格式契约 {{前缀}}{{YYYYMMDD}}{{3位流水}}，且新前缀干净号段应取到 001"
    );
    // 结构复核：前缀 + 8 位数字日期 + 3 位数字流水
    let rest = model
        .customer_code
        .strip_prefix(pfx)
        .expect("必须以测试前缀开头");
    let (date_part, seq_part) = rest.split_at(8);
    assert!(date_part.len() == 8 && date_part.chars().all(|c| c.is_ascii_digit()));
    assert!(
        seq_part.len() == 3 && seq_part.chars().all(|c| c.is_ascii_digit()),
        "流水段应为 3 位数字，实际 {seq_part}"
    );

    let persisted = customer::Entity::find()
        .filter(customer::Column::CustomerCode.eq(&expected))
        .one(&db)
        .await
        .expect("查库失败")
        .expect("提交后单号应真实落库");
    assert_eq!(persisted.customer_code, expected);

    cleanup_customer(&db, pfx).await;
}

/// (b) 同一外层事务内连续 3 次 insert_with_no_retry：取号→插入→取号→插入
/// 必须得到连续的 001/002/003，绝不撞号（探测可见本事务未提交插入）。
#[tokio::test]
#[ignore = "需已迁移 PG（advisory lock + SAVEPOINT），由专用 job（--run-ignored only）执行"]
async fn insert_with_no_retry_repeated_in_one_txn_yields_consecutive_nos() {
    let pfx = "NGT2";
    let db = live_db().await;
    cleanup_customer(&db, pfx).await;

    let txn = db.begin().await.expect("开启事务失败");
    let mut got: Vec<String> = Vec::new();
    for round in 1..=3 {
        let model = DocumentNumberGenerator::insert_with_no_retry(
            &txn,
            pfx,
            customer::Entity,
            customer::Column::CustomerCode,
            make_customer,
        )
        .await
        .unwrap_or_else(|e| panic!("第 {round} 轮取号+插入应成功，实际: {e:?}"));
        got.push(model.customer_code);
    }
    txn.commit().await.expect("提交失败");

    assert_eq!(
        got,
        vec![code(pfx, 1), code(pfx, 2), code(pfx, 3)],
        "事务内连续取号+插入必须得到连续号（SAVEPOINT 重试机制不得破坏本事务可见性）"
    );
    for c in &got {
        assert!(
            customer::Entity::find()
                .filter(customer::Column::CustomerCode.eq(c.as_str()))
                .one(&db)
                .await
                .expect("查库失败")
                .is_some(),
            "提交后 {c} 应真实存在"
        );
    }

    cleanup_customer(&db, pfx).await;
}

/// (c) 旁路写入制造真实 23505：
/// 连接 B 先插入第一轮候选号 001（**不提交**）→ 连接 A 的 insert_with_no_retry
/// 探测不到未提交行、拿 001 插入并被唯一索引挂起 → 观测到 A 的 INSERT 处于
/// transactionid 等待后 B 提交 → A 的第一次 INSERT 得到 23505 → 保存点回滚、
/// 重新取号得 002 成功；外层事务可正常提交（不是裸 500、不连带外层回滚）。
///
/// 同步点用 pg_stat_activity 观测（确定性），超时则显式 panic 暴露前置失效，
/// 绝不退化成"跳过断言"。
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "需已迁移 PG（advisory lock + SAVEPOINT + 23505 + pg_stat_activity），由专用 job（--run-ignored only）执行"]
async fn insert_with_no_retry_recovers_from_bypass_duplicate_23505() {
    let pfx = "NGT3";
    let db_admin = live_db().await;
    cleanup_customer(&db_admin, pfx).await;

    let dup = code(pfx, 1); // 干净号段下第一轮候选必为 001（allocate_no 基数语义）

    // 连接 B：旁路插入候选号，保持未提交
    let db_b = live_db().await;
    let txn_b: DatabaseTransaction = db_b.begin().await.expect("B 开启事务失败");
    make_customer(dup.clone())
        .insert(&txn_b)
        .await
        .expect("B 旁路插入候选号失败");

    // 连接 A：调用方事务内 insert_with_no_retry（后台运行，等待被唯一索引挂起）
    let conn_a = Arc::new(live_db().await);
    let handle = {
        let conn_a = Arc::clone(&conn_a);
        let pfx_owned = pfx.to_string();
        tokio::spawn(async move {
            let txn_a = conn_a.begin().await.expect("A 开启事务失败");
            let result = DocumentNumberGenerator::insert_with_no_retry(
                &txn_a,
                &pfx_owned,
                customer::Entity,
                customer::Column::CustomerCode,
                make_customer,
            )
            .await;
            match result {
                Ok(model) => {
                    txn_a.commit().await.expect("重试成功后外层事务应可提交");
                    Ok::<String, String>(model.customer_code)
                }
                Err(e) => {
                    txn_a.rollback().await.ok();
                    Err(format!("{e:?}"))
                }
            }
        })
    };

    wait_until_a_insert_blocked(&db_admin, Duration::from_secs(20)).await;

    // B 提交：A 的第一次 INSERT 由此确定性收到 23505（旁路写入此刻才对外可见）
    txn_b.commit().await.expect("B 提交失败");

    let final_code = handle
        .await
        .expect("A 任务 panic")
        .unwrap_or_else(|e| panic!("23505 应由保存点重试消化并成功，不得上抛: {e}"));

    assert_eq!(
        final_code,
        code(pfx, 2),
        "撞 23505 后应重新取号（探测此时可见已提交的 001）得到 002 并成功"
    );

    // 两条旁路+重试的行都真实在库
    for c in [&dup, &final_code] {
        assert!(
            customer::Entity::find()
                .filter(customer::Column::CustomerCode.eq(c.as_str()))
                .one(&db_admin)
                .await
                .expect("查库失败")
                .is_some(),
            "{c} 应真实落库"
        );
    }

    cleanup_customer(&db_admin, pfx).await;
}

/// 轮询观测：连接 A 的 INSERT 正被唯一索引的未提交冲突行挂起
/// （wait_event_type='Transaction' AND wait_event='transactionid'）。
/// 这是"B 提交前 A 的第一次取号+插入已就位"的确定性证据；超时即 panic，
/// 暴露前置条件失效（防止测试在没真正演练 23505 路径的情况下假绿）。
async fn wait_until_a_insert_blocked(db: &DatabaseConnection, timeout: Duration) {
    let started = std::time::Instant::now();
    loop {
        let stmt = Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT count(*) FROM pg_stat_activity \
             WHERE wait_event_type = 'Transaction' AND wait_event = 'transactionid' \
             AND query LIKE $1",
            vec![sea_orm::Value::String(Some(
                "INSERT INTO \"customers\"%".to_string(),
            ))],
        );
        // SeaORM 2.0.2：原始 Statement 走 ConnectionTrait::query_one_raw
        // （query_one 只收 &impl StatementBuilder，Statement 本身不实现该 trait）
        let row: sea_orm::QueryResult = db
            .query_one_raw(stmt)
            .await
            .expect("pg_stat_activity 观测查询失败")
            .expect("count 查询必然返回一行");
        // count(*) 在 PG 返回 BIGINT（系统视图列，非 m0044 收窄后的业务表列），取 i64
        let blocked: i64 = row.try_get_by_index(0).expect("count 应为 i64");
        if blocked >= 1 {
            return;
        }
        if started.elapsed() > timeout {
            panic!(
                "超时未观测到被旁路重复号挂起的 INSERT：insert_with_no_retry 未按预期\
                 在探测后插入候选号（前置失效，需人工排查并发时序，不允许跳过本用例）"
            );
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// (d) 非唯一约束错误不吞、不重试：外键违例（customer_id 指向不存在客户）。
/// 断言：构建闭包**只被调用一次**（没有重新取号重试循环）；错误原样上抛为
/// AppError::DatabaseError（500 级，真实分类文案在内部保留），而不是被伪装成
/// 成功或被降级成"单号连续冲突"的业务提示。
#[tokio::test]
#[ignore = "需已迁移 PG（真实 FK 约束 + SAVEPOINT），由专用 job（--run-ignored only）执行"]
async fn insert_with_no_retry_propagates_non_unique_sql_error_without_retry() {
    let pfx = "NGT4";
    let db = live_db().await;
    // 前置：确认探针客户 ID 不存在（FK 必然违例）
    assert!(
        customer::Entity::find_by_id(2_000_000_123)
            .one(&db)
            .await
            .expect("查库失败")
            .is_none(),
        "测试前置：2_000_000_123 号客户必须不存在，否则 FK 场景不成立"
    );

    let attempts = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&attempts);
    let build = move |no: String| {
        counter.fetch_add(1, Ordering::SeqCst);
        make_opportunity_bad_fk(no)
    };

    let txn = db.begin().await.expect("开启事务失败");
    let err = DocumentNumberGenerator::insert_with_no_retry(
        &txn,
        pfx,
        crm_opportunity::Entity,
        crm_opportunity::Column::OpportunityNo,
        build,
    )
    .await
    .expect_err("FK 违例必须原样上抛，绝不能被重试循环吞掉或返回 Ok");

    assert_eq!(
        attempts.load(Ordering::SeqCst),
        1,
        "非唯一约束错误重试无意义：构建闭包应只被调用一次（发生重试即为机制缺陷）"
    );
    match &err {
        AppError::DatabaseError(_) => {}
        other => panic!("期望 AppError::DatabaseError（FK 归类上抛），实际: {other:?}"),
    }
    assert_eq!(err.error_code(), "DATABASE_ERROR");

    txn.rollback().await.expect("错误后外层事务应可正常回滚");
    // 回滚后无残留：任何以本测试前缀开头的商机都不应存在
    let leftover = crm_opportunity::Entity::find()
        .filter(crm_opportunity::Column::OpportunityNo.starts_with(pfx))
        .one(&db)
        .await
        .expect("查库失败");
    assert!(leftover.is_none(), "FK 违例路径不得留下任何半成品商机行");
}
