//! m_crm_lead_claim_record 的 PostgreSQL 活体回放（真库专用通道）
//!
//! 形态复用 `rls_dept_user_sync_live_test.rs`：SQL 常量经 `#[path]` 直引
//! `migration/src/domain/crm_lead_claim_record/sql.rs`（与迁移 up()/down()
//! 逐字符同源，杜绝测试侧另抄一份漂移）；`require_postgres` 在
//! TEST_DATABASE_URL 缺失/非 PG 时**响亮 panic，绝不静默跳过**（sqlite 回退
//! 假绿在本文件是致命错）。用例显式 `#[ignore]`，由 CI 的
//! **ci-test-rust-ignored** job（`nextest --run-ignored only`）选中。
//!
//! 断言清单：
//! 1. up 幂等可重放：`ADD COLUMN IF NOT EXISTS` ×2 + 索引语句连跑两遍零报错；
//! 2. 列语义：两列存在且**可空**（information_schema 判定）——存量行 NULL 即
//!    "无保护期/不计入当日领取"的落地载体（迁移头注释语义）；
//! 3. down 精确可逆：DROP 后列消失，再 up 恢复（新增可空列、不改存量值，
//!    故无需备份表即可逐语句逆向——对照 m0007 备份表思路的适用边界）；
//! 4. 索引就位：idx_crm_lead_last_claimed 存在（每日领取上限计数的支撑）。

#[path = "../migration/src/domain/crm_lead_claim_record/sql.rs"]
mod m_claim_record_sql;

mod test_common;

use chrono::Utc;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, QueryResult, Statement};

async fn require_postgres(db: &DatabaseConnection) {
    let url = std::env::var("TEST_DATABASE_URL");
    assert!(
        matches!(&url, Ok(u) if u.starts_with("postgres")),
        "本用例覆盖 PG 活库列语义（information_schema/ADD COLUMN IF NOT EXISTS），\
         必须跑在 TEST_DATABASE_URL 指向的 PostgreSQL 上（ci-test-rust-ignored 注入；\
         本地需显式 export TEST_DATABASE_URL=postgres://...，禁止条件跳过假绿）。\
         当前 TEST_DATABASE_URL={url:?}"
    );
    assert_eq!(
        db.get_database_backend(),
        DbBackend::Postgres,
        "夹具解析出的后端不是 PostgreSQL，活库断言不可信（setup_test_db 无变量时静默回退 sqlite）"
    );
}

async fn exec(db: &DatabaseConnection, sql: &str) {
    db.execute_raw(Statement::from_string(
        db.get_database_backend(),
        sql.to_string(),
    ))
    .await
    .unwrap_or_else(|e| panic!("PG 语句执行失败: {e}\nSQL: {sql}"));
}

async fn scalar_count(db: &DatabaseConnection, sql: &str) -> i64 {
    let stmt = Statement::from_string(db.get_database_backend(), sql.to_string());
    let row: Option<QueryResult> = db.query_one_raw(stmt).await.ok().flatten();
    row.and_then(|r| r.try_get_by_index::<i64>(0).ok())
        .unwrap_or(0)
}

async fn column_exists(db: &DatabaseConnection, column: &str) -> bool {
    let sql = format!(
        "SELECT COUNT(*) FROM information_schema.columns \
         WHERE table_name = 'crm_lead' AND column_name = '{column}'"
    );
    scalar_count(db, &sql).await == 1
}

async fn index_exists(db: &DatabaseConnection, index: &str) -> bool {
    let sql = format!("SELECT COUNT(*) FROM pg_indexes WHERE indexname = '{index}'");
    scalar_count(db, &sql).await == 1
}

/// 列可空性判定：is_nullable='YES'（NULL 语义是"存量=无保护期"的地基，
/// NOT NULL 会把迁移直接砸在存量行上）
async fn column_is_nullable(db: &DatabaseConnection, column: &str) -> bool {
    let sql = format!(
        "SELECT COUNT(*) FROM information_schema.columns \
         WHERE table_name = 'crm_lead' AND column_name = '{column}' \
           AND is_nullable = 'YES'"
    );
    scalar_count(db, &sql).await == 1
}

// ci-test-rust-ignored 以 --test-threads=1 串行执行本文件用例；即便并行，
// ALTER TABLE ... IF NOT EXISTS / IF EXISTS 均幂等，语句级互不破坏。
#[tokio::test]
#[ignore = "需真实已迁移 PostgreSQL（TEST_DATABASE_URL），由 ci-test-rust-ignored 选中"]
async fn crm_lead_claim_record_migration_live_pg() {
    // 与 rls_dept_user_sync_live_test 同语义：TEST_DATABASE_URL 缺失时
    // setup_test_db 回退 sqlite，而 require_postgres 当场响亮 panic（绝不静默跳过）
    let db = test_common::setup_test_db().await;
    require_postgres(&db).await;

    // —— 1. up 幂等：迁移已在 migrate run 中执行过一遍，这里用同源常量重放第二遍
    exec(&db, m_claim_record_sql::ADD_LAST_CLAIMED_AT_SQL).await;
    exec(&db, m_claim_record_sql::ADD_LAST_CLAIMED_BY_SQL).await;
    exec(&db, m_claim_record_sql::CREATE_CLAIMED_INDEX_SQL).await;
    exec(&db, m_claim_record_sql::ADD_LAST_CLAIMED_AT_SQL).await;
    exec(&db, m_claim_record_sql::ADD_LAST_CLAIMED_BY_SQL).await;
    exec(&db, m_claim_record_sql::CREATE_CLAIMED_INDEX_SQL).await;

    // —— 2. 列语义：存在 + 可空
    assert!(
        column_exists(&db, "last_claimed_at").await,
        "last_claimed_at 列必须存在"
    );
    assert!(
        column_exists(&db, "last_claimed_by").await,
        "last_claimed_by 列必须存在"
    );
    assert!(
        column_is_nullable(&db, "last_claimed_at").await
            && column_is_nullable(&db, "last_claimed_by").await,
        "两列必须可空——NULL=存量行无保护期/不计入当日领取（迁移头注释语义），\
         NOT NULL 等于用迁移凭空制造领取事件或阻断存量行"
    );

    // —— 4. 计数支撑索引就位
    assert!(
        index_exists(&db, "idx_crm_lead_last_claimed").await,
        "每日领取上限计数 (last_claimed_by, last_claimed_at) 索引缺失"
    );

    // —— 3. down 精确可逆：DROP 后列/索引消失，再 up 完整恢复
    exec(&db, m_claim_record_sql::DROP_CLAIMED_INDEX_SQL).await;
    exec(&db, m_claim_record_sql::DROP_LAST_CLAIMED_COLUMNS_SQL).await;
    assert!(
        !column_exists(&db, "last_claimed_at").await
            && !column_exists(&db, "last_claimed_by").await,
        "down 后两列必须消失（新增可空列不改存量值，DROP 即精确逆向）"
    );
    assert!(
        !index_exists(&db, "idx_crm_lead_last_claimed").await,
        "down 后计数索引必须消失"
    );
    exec(&db, m_claim_record_sql::ADD_LAST_CLAIMED_AT_SQL).await;
    exec(&db, m_claim_record_sql::ADD_LAST_CLAIMED_BY_SQL).await;
    exec(&db, m_claim_record_sql::CREATE_CLAIMED_INDEX_SQL).await;
    assert!(
        column_exists(&db, "last_claimed_at").await
            && column_exists(&db, "last_claimed_by").await
            && index_exists(&db, "idx_crm_lead_last_claimed").await,
        "up 重放后必须恢复到与迁移终态一致（同库后续 ignored 用例依赖这两列）"
    );

    // 行为侧最小闭环（服务层已在 sqlite 契约测试覆盖，这里真库补一枪）：
    // 绑定参数按列类型可正常构造（last_claimed_by=INT、last_claimed_at=
    // TIMESTAMPTZ），防方言/类型漂移导致 validate_claim_rules 的过滤在 PG 上失效
    let bind_sql = "SELECT 1 FROM crm_lead WHERE last_claimed_by = $1 \
                    AND last_claimed_at >= $2 LIMIT 1";
    db.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        bind_sql,
        [
            sea_orm::Value::Int(Some(1)),
            sea_orm::Value::DateTimeWithTimeZone(Some(Box::new(chrono::DateTime::<
                chrono::FixedOffset,
            >::from(Utc::now())))),
        ],
    ))
    .await
    .unwrap_or_else(|e| panic!("领取事件列绑定查询失败（列类型漂移？）: {e}"));
}
