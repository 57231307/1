//! 契约测试：客户分层（tier）受控写腿行为锁。
//!
//! 锁的判据（源码 `services/customer_ops/update.rs::update_customer_tier`）：
//! 1. 值域门按唯一词表 `constants::customer_tier::ALLOWED` 逐字符校验，词表外值拒绝
//!    且零写入（库内档位不变），错误走可展示业务族（HTTP 400）；
//! 2. 合法档位经服务写口落库真实穿过 DB CHECK `chk_customers_tier`，回读一致，
//!    且写入行落在大客户高档集合 `customer_tier::MAJOR` 时可被判据 `is_major` 命中
//!   （证明二级审批的判据输入经本腿真实可达）；
//! 3. 每次成功写入同事务产出审计行（audit_logs，action=UPDATE、resource_type=customer），
//!    失败写入不产生审计行。
//!
//! 为什么必须真库：档位值必须真实穿过列 CHECK 约束，审计与更新必须验证同事务语义；
//! 夹具 `setup_test_db()` 缺 `TEST_DATABASE_URL` 或指向 sqlite 直接 panic——本文件行为
//! 断言只有 CI 活库才有结果（本机仅证明编译与格式），交 CI 验证。
//!
//! id 带：本文件独占 986xxx 带（users 986001、customers 986041），每用例先删后插保幂等，
//! 删序 审计（按 resource_id 点名）→ 客户 → users。

mod test_common;

use bingxi_backend::constants::customer_tier::{self, VIP};
use bingxi_backend::search::{ElasticClient, SearchClient};
use bingxi_backend::services::customer_service::CustomerService;
use bingxi_backend::utils::error::AppError;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use std::sync::Arc;
use test_common::setup_test_db;

const USER_OWNER: i32 = 986_001;
const CUSTOMER: i32 = 986_041;

fn pg_stmt(sql: &str) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, Vec::<sea_orm::Value>::new())
}

async fn exec(db: &DatabaseConnection, sql: &str) {
    db.execute_raw(pg_stmt(sql))
        .await
        .unwrap_or_else(|e| panic!("种子 SQL 执行失败: {e}\nSQL: {sql}"));
}

fn service(db: &Arc<DatabaseConnection>) -> CustomerService {
    let search_client: Arc<dyn SearchClient> = Arc::new(ElasticClient::mock());
    CustomerService::new(db.clone(), search_client)
}

/// 场景种子：一个未定档（tier NULL）的真实客户行。先清理审计与本带行再插，保幂等。
async fn seed_scene(db: &DatabaseConnection) {
    exec(
        db,
        &format!(
            "DELETE FROM audit_logs WHERE resource_type='customer' AND resource_id='{CUSTOMER}'"
        ),
    )
    .await;
    exec(db, &format!("DELETE FROM customers WHERE id = {CUSTOMER}")).await;
    exec(db, &format!("DELETE FROM users WHERE id = {USER_OWNER}")).await;
    exec(
        db,
        &format!(
            "INSERT INTO users (id,username,password_hash,is_active,is_totp_enabled,department_id,created_at,updated_at) VALUES
             ({USER_OWNER},'tier_leg_owner','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"
        ),
    )
    .await;
    exec(
        db,
        &format!(
            "INSERT INTO customers (id,customer_code,customer_name,credit_limit,payment_terms,status,owner_id,created_by,customer_type,tier,created_at,updated_at)
             VALUES ({CUSTOMER},'CUST-{CUSTOMER}','分层写腿客户',0,30,'active',{USER_OWNER},{USER_OWNER},'other',NULL,'2026-01-01T00:00:00Z','2026-01-01-00:00:00Z')"
        ),
    )
    .await;
}

/// 回读库内档位（真库直读，不经实体默认值兜底，缺列即 panic）。
async fn read_tier(db: &DatabaseConnection) -> Option<String> {
    let row = db
        .query_one_raw(pg_stmt(&format!(
            "SELECT tier FROM customers WHERE id={CUSTOMER}"
        )))
        .await
        .expect("回读档位失败")
        .expect("客户行缺失");
    row.try_get("", "tier").expect("tier 解码失败")
}

/// 锁一（负向）：词表外档位被可展示业务族拒绝，库内档位逐字不变，零审计行。
#[tokio::test]
async fn illegal_tier_value_is_rejected_and_db_unchanged() {
    let db = Arc::new(setup_test_db().await);
    seed_scene(&db).await;

    let err = service(&db)
        .update_customer_tier(CUSTOMER, "DIAMOND", USER_OWNER)
        .await
        .expect_err("词表外档位必须被拒绝，不得静默吞进写口");
    let msg = match err {
        AppError::BusinessErrorDisplayable(m) => m,
        other => panic!(
            "值域门必须走可展示业务族（HTTP 400 且 message 携带行动指引），实际变体: {other:?}"
        ),
    };
    assert!(
        msg.contains("VIP")
            && msg.contains("GOLD")
            && msg.contains("SILVER")
            && msg.contains("NORMAL"),
        "拒绝文案必须点名可选档位让用户知道该怎么做，实际: {msg}"
    );
    assert_eq!(
        read_tier(&db).await,
        None,
        "非法值拒绝后库内档位必须保持 NULL（未定档），不得被兜底写成任何档位"
    );
    let audits = db
        .query_one_raw(pg_stmt(&format!(
            "SELECT COUNT(*)::bigint AS c FROM audit_logs WHERE resource_type='customer' AND resource_id='{CUSTOMER}'"
        )))
        .await
        .expect("审计计数失败")
        .expect("审计计数行缺失");
    let count: i64 = audits.try_get("", "c").expect("审计计数解码失败");
    assert_eq!(count, 0, "失败的档位写入不得留下审计行，实际 {count} 条");
}

/// 锁二（正向）：合法高档档位经服务写口落库穿过 DB CHECK，回读一致、判据命中、审计一行。
#[tokio::test]
async fn legal_major_tier_persists_with_audit_and_hits_major_vocab() {
    let db = Arc::new(setup_test_db().await);
    seed_scene(&db).await;

    assert!(
        customer_tier::is_major(VIP),
        "种子前置：VIP 必须在 MAJOR 高档集合内，否则本锁失去判据可达性意义"
    );
    let updated = service(&db)
        .update_customer_tier(CUSTOMER, VIP, USER_OWNER)
        .await
        .expect("合法档位必须写入成功");
    assert_eq!(
        updated.tier.as_deref(),
        Some(VIP),
        "服务出参档位应为 VIP，实际 {:?}",
        updated.tier
    );
    assert_eq!(
        read_tier(&db).await.as_deref(),
        Some(VIP),
        "库内档位必须真实穿过 chk_customers_tier 并落为 VIP"
    );
    let audits = db
        .query_one_raw(pg_stmt(&format!(
            "SELECT COUNT(*)::bigint AS c FROM audit_logs WHERE action='UPDATE' AND resource_type='customer' AND resource_id='{CUSTOMER}'"
        )))
        .await
        .expect("审计计数失败")
        .expect("审计计数行缺失");
    let count: i64 = audits.try_get("", "c").expect("审计计数解码失败");
    assert_eq!(
        count, 1,
        "成功写入必须同事务留下一条 UPDATE 审计行，实际 {count} 条"
    );
}
