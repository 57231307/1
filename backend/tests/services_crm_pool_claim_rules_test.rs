//! 公海规则统一（用户 2026-10-02 拍板 ①）：单条领取纳入三项校验，
//! 且保护期/每日计数判据从 `updated_at` 改为领取事件列
//! `last_claimed_at`/`last_claimed_by`（唯一写点 build_claimed_active，
//! 补列迁移 m_crm_lead_claim_record）。
//!
//! 根因（修复前实证）：
//! 1. 单条路径 `/crm/pool/claim`（claim_lead_ownership）**完全不走**
//!    `validate_claim_rules`/保护期——两条领取路径校验口径分叉；
//! 2. 保护期旧判据取行 `updated_at`，而**回收只改 lead_status 也会刷新
//!    updated_at**（recycle_to_pool → update_lead），"回收 → 立即领取"合法链
//!    被默认 7 天保护期直接判负（e2e 27-06 依赖该顺序）；
//! 3. 每日领取上限旧判据 `owner_id + updated_at ≥ 今日`，把"今天被动过的
//!    存量线索"计入领取量，计数虚高误拒。
//!
//! 本文件锁（真 PostgreSQL 真跑服务层，无 HTTP 伪装；路线一，#4669 判责）：
//! A. 保护期判据 = last_claimed_at：A 领取 → 回收（updated_at 变"刚刚"）→
//!    B 立即领取被拒（单条显式报错 / 批量 claimed=0，两路径同判据）；
//! B. 原领取人本人重领豁免（last_claimed_by 判定，保护期本义防他人抢单）；
//! C. 存量行 last_claimed_at NULL → 无保护期（迁移头注释语义：只影响保护期/
//!    计数，不影响归属 403）；
//! D. 每日上限判据 = 领取事件列：今天被 updated_at 刷新的非领取行**不计入**；
//!    真实领取达 claim_limit（默认 5）后拒领；
//! E. 最大持有数（默认 50）与领取入口无关：达上限后两路径都拒；
//! F. 单条/批量两条路径校验结果一致性（同判据同函数，不存在单条旁路）。
//!
//! 表结构唯一来源 = backend/migration（不再自建 DDL）；customer_pool_rules 真表
//! 无迁移播种、setup_test_db 清空后为空 → get_rule_value 走代码默认值，与
//! "空规则表"前置一致。任何规则断言（保护期/上限/豁免/两路径一致）不软化。

mod test_common;

use bingxi_backend::models::crm_lead;
use bingxi_backend::models::status::crm_lead as lead_status;
use bingxi_backend::services::crm::cust::CrmService;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DbBackend, EntityTrait, Statement,
};
use std::sync::Arc;

const USER_A: i32 = 50;
const USER_B: i32 = 60;

async fn exec(db: &sea_orm::DatabaseConnection, sql: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        Vec::<sea_orm::Value>::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("种子执行失败: {e}\nSQL: {sql}"));
}

/// 种子：
/// - users：两个销售父行（裁定 R1；AuditLogService::update_with_audit 回读
///   users 取操作人，真表真 FK 语义下必须存在）；
/// - crm_lead：id 1..=6 的公海行（存量形态：last_claimed_at/last_claimed_by
///   NULL、updated_at 早于今日），A/B/C/D/E/F 各用例的领取对象。
/// roles/data_permissions 不再建不再插（本文件用例不依赖）。
async fn seeded_db() -> Arc<sea_orm::DatabaseConnection> {
    let db = test_common::setup_test_db().await;
    exec(
        &db,
        "INSERT INTO users (id,username,password_hash,is_active,is_totp_enabled,department_id,created_at,updated_at) VALUES
         (50,'sales_a','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (60,'sales_b','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;
    // 领取对象：id 1..=6 公海存量行（前置条件显式自种子，不依赖环境已有数据）
    for id in 1..=6 {
        seed_pool_lead(&db, id, USER_A, "2026-01-01T00:00:00Z", None, None).await;
    }
    Arc::new(db)
}

/// 种一条公海行（lead_status='pool'，owner 为回收前原归属人——回收只改状态的
/// 既有语义）。updated_at/last_claimed_at 由调用方决定。
/// department_id 不显式给：真表由 trg_crm_lead_dept 触发器按归属人回填。
async fn seed_pool_lead(
    db: &sea_orm::DatabaseConnection,
    id: i32,
    owner: i32,
    updated_at: &str,
    last_claimed_at: Option<&str>,
    last_claimed_by: Option<i32>,
) {
    let claimed_sql = match last_claimed_at {
        Some(t) => format!("'{}'", t),
        None => "NULL".to_string(),
    };
    let by_sql = match last_claimed_by {
        Some(u) => u.to_string(),
        None => "NULL".to_string(),
    };
    exec(
        db,
        &format!(
            "INSERT INTO crm_lead (id,lead_no,lead_source,lead_status,company_name,
             contact_name,owner_id,owner_name,
             last_claimed_at,last_claimed_by,created_at,updated_at) VALUES
             ({id},'LD{id:04}','website','pool','公司{id}','联系人{id}',
              {owner},'归属人{owner}',
              {claimed_sql},{by_sql},'2026-01-01T00:00:00Z','{updated_at}')"
        ),
    )
    .await;
}

/// 模拟"回收"：只改 lead_status='pool' 并刷新 updated_at（与
/// recycle_to_pool→update_lead 的落库形态一致——**updated_at 必被刷新**，
/// 这正是旧判据误伤"回收→领取"链的原因）
async fn recycle(db: &sea_orm::DatabaseConnection, id: i32) {
    let lead = crm_lead::Entity::find_by_id(id)
        .one(db)
        .await
        .unwrap()
        .unwrap_or_else(|| panic!("线索 {id} 不存在"));
    let mut am: crm_lead::ActiveModel = lead.into();
    am.lead_status = Set(Some(lead_status::POOL.to_string()));
    am.updated_at = Set(Some(Utc::now()));
    am.update(db).await.expect("回收更新失败");
}

async fn row(db: &sea_orm::DatabaseConnection, id: i32) -> crm_lead::Model {
    crm_lead::Entity::find_by_id(id)
        .one(db)
        .await
        .unwrap()
        .unwrap_or_else(|| panic!("线索 {id} 不应消失"))
}

// ---------------------------------------------------------------------------
// A) 保护期判据 = last_claimed_at（不再取 updated_at）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn protection_period_keys_on_last_claimed_at_not_updated_at() {
    let db = seeded_db().await;
    let service = CrmService::new(db.clone());

    // 1. A 单条领取 id=1（存量 last_claimed_at=NULL → 无保护期，可领）
    let lead = row(&db, 1).await;
    let before = Utc::now();
    let claimed = service
        .claim_lead_ownership(lead, USER_A, "sales_a")
        .await
        .expect("A 首次领取应成功");
    let claim_time = claimed
        .last_claimed_at
        .expect("领取必须写入 last_claimed_at（build_claimed_active 唯一写点）");
    assert!(
        claim_time >= before && claim_time <= Utc::now() + chrono::Duration::seconds(5),
        "last_claimed_at 必须是本次领取时刻，实际 {claim_time}"
    );
    assert_eq!(claimed.last_claimed_by, Some(USER_A));

    // 2. 回收：只改状态、updated_at 刷成"刚刚"
    recycle(&db, 1).await;
    let recycled = row(&db, 1).await;
    assert_eq!(recycled.lead_status.as_deref(), Some(lead_status::POOL));

    // 3. B 立即单条领取 → 保护期内他人，显式拒绝（不静默）
    let err = service
        .claim_lead_ownership(row(&db, 1).await, USER_B, "sales_b")
        .await
        .expect_err("保护期内 B 领取必须被拒（判据 last_claimed_at，与旧 updated_at 无关——两者这里同值，区别见用例 C/回收顺序）");
    let text = format!("{err:?}");
    assert!(text.contains("保护期"), "拒绝原因应指向保护期: {text}");
    assert_eq!(row(&db, 1).await.owner_id, USER_A, "被拒后零漂移");

    // 4. 批量路径同判据：claimed=0（批量多行循环，静默跳过+计数是唯一既有形态）
    let n = service
        .claim_pool_customers(vec![1], USER_B, "sales_b")
        .await
        .expect("批量领取调用本身不应报错（跳过以计数体现）");
    assert_eq!(n, 0, "批量路径保护期判据必须与单条同源");

    // 5. 回收 8 天后（last_claimed_at 保持领取时刻不变）→ B 可领
    let lead = row(&db, 1).await;
    let eight_days_ago = Utc::now() - chrono::Duration::days(8);
    let mut am: crm_lead::ActiveModel = lead.into();
    am.last_claimed_at = Set(Some(eight_days_ago));
    am.update(&*db).await.unwrap();
    let n = service
        .claim_pool_customers(vec![1], USER_B, "sales_b")
        .await
        .unwrap();
    assert_eq!(n, 1, "保护期满（按 last_claimed_at 8>7 天）后他人可领");
    assert_eq!(row(&db, 1).await.owner_id, USER_B);
}

// ---------------------------------------------------------------------------
// B) 原领取人本人重领豁免（保护期本义防他人抢单）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn original_claimer_reclaim_is_exempt_from_protection() {
    let db = seeded_db().await;
    let service = CrmService::new(db.clone());

    // A 领取 → 立即回收（updated_at/last_claimed_at 都是"刚刚"）
    service
        .claim_lead_ownership(row(&db, 1).await, USER_A, "sales_a")
        .await
        .expect("A 领取成功");
    recycle(&db, 1).await;

    // A 本人重领：豁免保护期，成功（e2e 27-06 "回收→原领取人再领取"链的依据）
    let claimed = service
        .claim_lead_ownership(row(&db, 1).await, USER_A, "sales_a")
        .await
        .expect("原领取人本人重领应豁免保护期");
    assert_eq!(claimed.lead_status.as_deref(), Some(lead_status::NEW));
    assert_eq!(claimed.last_claimed_by, Some(USER_A));
    // 重领刷新领取事件（保护期从本次重算）
    assert!(claimed.last_claimed_at.is_some());
}

// ---------------------------------------------------------------------------
// C) 存量行 last_claimed_at NULL → 无保护期；updated_at"刚刚"不再判负
// ---------------------------------------------------------------------------

#[tokio::test]
async fn legacy_null_last_claimed_at_has_no_protection() {
    let db = seeded_db().await;
    let service = CrmService::new(db.clone());

    // 迁移前存量公海行：last_claimed_at NULL，updated_at = 刚刚（刚被回收）
    seed_pool_lead(&db, 9, USER_A, &Utc::now().to_rfc3339(), None, None).await;
    let claimed = service
        .claim_lead_ownership(row(&db, 9).await, USER_B, "sales_b")
        .await
        .expect("存量 NULL 行按'无保护期'语义放行（迁移头注释）");
    assert_eq!(claimed.owner_id, USER_B);
    assert!(claimed.last_claimed_at.is_some(), "领取后补齐领取事件列");
}

// ---------------------------------------------------------------------------
// D) 每日领取上限判据 = 领取事件列（updated_at 刷新不再计入）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn daily_claim_limit_counts_claim_events_not_updated_rows() {
    let db = seeded_db().await;
    let service = CrmService::new(db.clone());
    let now = Utc::now().to_rfc3339();

    // B 名下 10 条线索今天都被 updated_at 刷过（跟进/回收等），但**无领取事件**
    // ——旧判据会把它们全计入当日领取量直接撞 claim_limit=5 误拒（回归锁）
    for id in 20..30 {
        seed_pool_lead(&db, id, USER_B, &now, None, None).await;
        recycle_keep_owner(&db, id).await;
    }

    // B 真实领取 5 条（默认 claim_limit=5）
    for id in 1..=5 {
        service
            .claim_lead_ownership(row(&db, id).await, USER_B, "sales_b")
            .await
            .unwrap_or_else(|e| panic!("B 第 {id} 次领取应成功: {e:?}"));
    }
    // 第 6 条：达每日上限，拒（单条显式报错）
    let err = service
        .claim_lead_ownership(row(&db, 6).await, USER_B, "sales_b")
        .await
        .expect_err("达每日上限后必须拒领");
    assert!(
        format!("{err:?}").contains("每日上限"),
        "拒绝原因应指向领取上限: {err:?}"
    );
    // 上限按人隔离：A 领取第 6 条不受 B 的计数影响
    service
        .claim_lead_ownership(row(&db, 6).await, USER_A, "sales_a")
        .await
        .expect("上限判定按用户隔离");
}

/// 把私海行"回收"但保持 last_claimed_at NULL（模拟存量行今天被 updated_at
/// 刷新的场景）
async fn recycle_keep_owner(db: &sea_orm::DatabaseConnection, id: i32) {
    let lead = row(db, id).await;
    let mut am: crm_lead::ActiveModel = lead.into();
    am.lead_status = Set(Some(lead_status::NEW.to_string()));
    am.updated_at = Set(Some(Utc::now()));
    am.update(db).await.unwrap();
}

// ---------------------------------------------------------------------------
// E) 最大持有数上限（默认 50）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn max_holdings_blocks_claim_once_at_limit() {
    let db = seeded_db().await;
    let service = CrmService::new(db.clone());
    let now = Utc::now().to_rfc3339();

    // B 先攒 50 条活跃持有（new 状态、owner=B；不含 converted/lost/pool）。
    // department_id 由 trg_crm_lead_dept 按归属人回填，不显式给。
    for id in 30..80 {
        exec(
            &db,
            &format!(
                "INSERT INTO crm_lead (id,lead_no,lead_source,lead_status,company_name,
                 contact_name,owner_id,owner_name,created_at,updated_at) VALUES
                 ({id},'LD{id:04}','ad','new','持有{id}','联系人{id}',
                  {USER_B},'销售乙','2026-01-01T00:00:00Z','{now}')"
            ),
        )
        .await;
    }
    let err = service
        .claim_lead_ownership(row(&db, 1).await, USER_B, "sales_b")
        .await
        .expect_err("达最大持有数 50 必须拒领");
    assert!(
        format!("{err:?}").contains("最大持有数"),
        "拒绝原因应指向最大持有数: {err:?}"
    );
    // A 无持有压力：同一条线索由 A 领取成功（上限按人隔离）
    service
        .claim_lead_ownership(row(&db, 1).await, USER_A, "sales_a")
        .await
        .expect("A 领取应成功");
}

// ---------------------------------------------------------------------------
// F) 领取后归属落库（统一实现回归）：两条路径逐字段一致 + 领取事件列同源
// ---------------------------------------------------------------------------

#[tokio::test]
async fn both_paths_write_ownership_and_claim_event_columns() {
    let db = seeded_db().await;
    let service = CrmService::new(db.clone());

    service
        .claim_pool_customers(vec![1], USER_B, "sales_b")
        .await
        .unwrap();
    let batch = row(&db, 1).await;
    recycle(&db, 1).await;
    service
        .claim_lead_ownership(row(&db, 1).await, USER_B, "sales_b")
        .await
        .unwrap();
    let single = row(&db, 1).await;

    // 归属与领取事件逐字段一致（同一 build_claimed_active）
    assert_eq!(batch.owner_id, single.owner_id);
    assert_eq!(batch.owner_name, single.owner_name);
    assert_eq!(single.lead_status.as_deref(), Some(lead_status::NEW));
    assert_eq!(single.last_claimed_by, Some(USER_B));
    assert!(single.last_claimed_at.is_some());
}
