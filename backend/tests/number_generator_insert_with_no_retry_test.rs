//! `DocumentNumberGenerator::insert_with_no_retry`（调用方事务内
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
    // 夹具前置 1（连接足迹）：admin 观测 / B 旁路 / A 取号重试三角色共用**同一个池**
    // （三次 begin() 仍是三个独立后端，"未提交冲突行"的并发语义不变）。三角色若各建
    // 一池，同 job 并行跑多个真库 binary 时连接数叠加，易把 PG `max_connections`
    // 顶穿；A 的 begin() 在 spawn 里 panic 后主流程只会撞上观测超时，真实原因被掩盖。
    let db = live_db().await;
    cleanup_customer(&db, pfx).await;

    // 夹具前置 2（确定性次序）：整段演练期间以事务级 ROW EXCLUSIVE 锁住 customers。
    // 缘由：同 job 的兄弟真库用例各自在 setup_test_db() 里对**全部业务表**执行
    // `TRUNCATE ... RESTART IDENTITY CASCADE`（src/services/test_common.rs，取
    // ACCESS EXCLUSIVE）。它与未提交的旁路行构成三方僵死——TRUNCATE 排队等 B 的
    // 行锁，A 的 INSERT 又排在 TRUNCATE 之后（PG 锁队列按到达序），于是 A 永远停在
    // wait_event_type='Lock'/wait_event='relation'，本用例的 transactionid 判据永不
    // 成立、只在观测超时处 panic。ROW EXCLUSIVE 与兄弟用例的正常 INSERT
    // （RowExclusive）互不冲突，只把表级清理挡在演练窗口之外，不改变 A/B 之间的
    // 唯一索引冲突语义 ⇒ 只消时序噪声，不可能掩盖 insert_with_no_retry 的缺陷。
    let guard = db.begin().await.expect("锁守卫开启事务失败");
    let lock_fut = guard.execute_raw(Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        "LOCK TABLE \"customers\" IN ROW EXCLUSIVE MODE".to_string(),
    ));
    tokio::time::timeout(Duration::from_secs(30), lock_fut)
        .await
        .expect("30s 内拿不到 customers 的 ROW EXCLUSIVE 锁：并发夹具仍在本窗口内做表级清理，需人工排查 job 并行度（不允许跳过本用例）")
        .expect("LOCK TABLE 守卫语句执行失败");

    let dup = code(pfx, 1); // 干净号段下第一轮候选必为 001（allocate_no 基数语义）

    // 连接 B：旁路插入候选号，保持未提交
    let txn_b: DatabaseTransaction = db.begin().await.expect("B 开启事务失败");
    make_customer(dup.clone())
        .insert(&txn_b)
        .await
        .expect("B 旁路插入候选号失败");

    // 连接 A：调用方事务内 insert_with_no_retry（后台运行，等待被唯一索引挂起）
    let conn_a = db.clone();
    let (pid_tx, pid_rx) = tokio::sync::oneshot::channel::<i32>();
    let handle = tokio::spawn(async move {
        let txn_a = conn_a.begin().await.expect("A 开启事务失败");
        // 夹具前置 3（观测对象钉死到 A 自己）：把 A 的 backend pid 报回观测方。
        // 观测若按 `query LIKE 'INSERT INTO "customers"%'` 跨会话计数，任何兄弟用例
        // 的 blocked INSERT 都能替 A "满足"判据——观测对象错位，是假绿口子。
        let pid_row = txn_a
            .query_one_raw(Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT pg_backend_pid()".to_string(),
            ))
            .await
            .expect("A 读取自身 backend pid 失败")
            .expect("A 的 pg_backend_pid 必然返回一行");
        let backend_pid: i32 = pid_row
            .try_get_by_index(0)
            .expect("pg_backend_pid 应为 i32");
        let _ = pid_tx.send(backend_pid);
        let result = DocumentNumberGenerator::insert_with_no_retry(
            &txn_a,
            pfx,
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
    });

    let a_pid = match tokio::time::timeout(Duration::from_secs(20), pid_rx).await {
        Ok(Ok(pid)) => pid,
        Ok(Err(_)) => panic!(
            "A 任务在报回 backend pid 之前已结束（取号前的 begin/pg_backend_pid 未通过）：\
             真实原因见同一次 stderr 里的 tokio panic 回溯；前置失效，不允许跳过本用例"
        ),
        Err(_) => panic!(
            "A 任务 20s 内未报回 backend pid（开启事务或读 pid 未完成 ⇒ 连接池/PG 侧被压满）：\
             前置失效，需人工排查并发时序，不允许跳过本用例"
        ),
    };

    wait_until_a_insert_blocked(&db, a_pid, Duration::from_secs(20)).await;

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
                .one(&db)
                .await
                .expect("查库失败")
                .is_some(),
            "{c} 应真实落库"
        );
    }

    cleanup_customer(&db, pfx).await;
    // 释放表级守卫锁：守卫事务内没有任何写入，回滚即等值于纯解锁
    guard.rollback().await.expect("释放锁守卫事务失败");
}

/// 轮询观测：**A 自己的后端**（按 backend pid 精确锁定）确实走到了候选号 INSERT，并被
/// B 未提交的重复唯一键行挡在唯一索引冲突的等待上。PG 把"INSERT 等待持冲突行的未提交
/// 事务出结果"这一等待归类为 `wait_event_type='Lock'`（等待事件枚举里根本不存在
/// `'Transaction'` 这一类型），其 wait_event 为行/事务级冲突形态：`transactionid`＝等待
/// 对方事务结束、`tuple`＝等待冲突元组锁、`speculativeinsertion`＝btree 投机插入令牌
/// 等待——三者都是 INSERT 撞**未提交**重复键时的真实挂起形态，一并作为放行判据以如实
/// 覆盖真实等待形态全集；再叠加"正在执行的语句是 `INSERT INTO "customers"`"把观测对象
/// 钉死到 A 的候选号插入本身，排除停在 `pg_advisory_xact_lock`（wait_event='advisory'）
/// 或停在兄弟用例表级清理（wait_event='relation'）等无关形态。这是"B 提交前 A 的第一次
/// 取号+插入已就位、且确实被唯一索引挡下而非静默成功"的确定性证据；超时即 panic，并把
/// 该后端当时的 state / 实际等待事件 / 正在执行的语句原文一起打出，暴露前置失效的真实
/// 方向（连接断了？被表级锁挡住？根本没走到 INSERT？），防止退化成"跳过断言"的假绿。
/// 判据只认上列真实冲突形态、未观测到挂起就绝不放行，也绝不放宽成"只要 A 在跑就算通过"。
async fn wait_until_a_insert_blocked(db: &DatabaseConnection, backend_pid: i32, timeout: Duration) {
    let started = std::time::Instant::now();
    loop {
        let stmt = Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT wait_event_type::text, wait_event::text, state::text, query \
             FROM pg_stat_activity WHERE pid = $1",
            vec![sea_orm::Value::Int(Some(backend_pid))],
        );
        // SeaORM 2.0.2：原始 Statement 走 ConnectionTrait::query_one_raw
        // （query_one 只收 &impl StatementBuilder，Statement 本身不实现该 trait）
        let row = db
            .query_one_raw(stmt)
            .await
            .expect("pg_stat_activity 观测查询失败");
        if let Some(row) = row {
            let wait_type: Option<String> = row
                .try_get_by_index(0)
                .expect("wait_event_type 应为可空文本");
            let wait_event: Option<String> =
                row.try_get_by_index(1).expect("wait_event 应为可空文本");
            let state: Option<String> = row.try_get_by_index(2).expect("state 应为可空文本");
            let query: String = row
                .try_get_by_index(3)
                .unwrap_or_else(|_| "<不可读>".to_string());
            // PG 中 INSERT 撞未提交重复键的挂起一律归 wait_event_type='Lock'，
            // wait_event 为行/事务级冲突三形态之一；再要求正在执行的语句确为
            // customers 的候选号 INSERT，把观测对象钉死到 A 的被挡插入本身。
            let blocked_by_unique_conflict = wait_type.as_deref() == Some("Lock")
                && matches!(
                    wait_event.as_deref(),
                    Some("transactionid") | Some("tuple") | Some("speculativeinsertion")
                )
                && query.contains("INSERT INTO \"customers\"");
            if blocked_by_unique_conflict {
                return;
            }
            if started.elapsed() > timeout {
                panic!(
                    "超时未观测到 A 后端(pid={backend_pid}) 因旁路重复号被唯一索引挂起：\
                     state={state:?} wait_event_type={wait_type:?} wait_event={wait_event:?} \
                     当前语句={query}（insert_with_no_retry 未按预期在探测后插入候选号，\
                     前置失效，需人工排查并发时序，不允许跳过本用例）"
                );
            }
        } else if started.elapsed() > timeout {
            panic!(
                "超时：A 后端(pid={backend_pid}) 已不在 pg_stat_activity（连接被断开或事务已结束），\
                 前置失效，需人工排查并发时序，不允许跳过本用例"
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
