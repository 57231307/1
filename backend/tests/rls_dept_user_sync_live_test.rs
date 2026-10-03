//! rls_dept 冗余部门列 · PostgreSQL 触发器语义活体验证(真库专用通道)
//!
//! 背景(假绿盲区):全仓既有"集成测试"跑在 sqlite 手写 DDL 上
//! (`tests/test_common/mod.rs:12、:23-25` 的 sqlite::memory: 静默回退;
//! `tests/contract_wave6_crm_pool_owner_test.rs:50-52` 自述"本文件不依赖 PG 专有特性"),
//! 而 `CREATE TRIGGER ... EXECUTE FUNCTION` / `UPDATE ... FROM` / `IS DISTINCT FROM`
//! 均为 PG 专有形态——rls_dept(m_rls_dept_domain)的触发器同步语义从未被任何测试
//! 真实验证过。本文件三条用例全部显式要求 PostgreSQL:`require_postgres` 在
//! TEST_DATABASE_URL 缺失/非 PG 时**响亮 panic,绝不静默跳过**(形态复用
//! `contract_wave5_inspection_result_authority_test.rs:385-399` 的既有实现语义)。
//!
//! 执行通道:三条用例标 `#[ignore]`,由 CI 的 **ci-test-rust-ignored** job
//! (`.github/workflows/ci-cd.yml:1543`;`nextest ... --run-ignored only`,:1641)
//! 选中——该 job 先对 PostgreSQL 16 service 容器跑 `bingxi migrate run`
//! (含表数硬校验,:1613-1626),本迁移注册后即被应用,触发器在库内真实存在。
//! 10 分片 job(ci-cd.yml:1422 起的迁移 + 分片)不会执行本文件用例(ignored 被分片
//! 排除),因此本文件不构成对分片通道的假绿依赖。
//!
//! 断言清单:
//! 1. 建 user(部门A) + lead(owner=user, department_id=A) → 运行期把 user 改到部门B
//!    (与 user_service.rs:364 同一列的 UPDATE)→ lead.department_id 变 B;
//!    同批种入的 customers/crm_opportunity(按 owner_id)、suppliers/sales_orders
//!    (按 created_by)一并断言,再改回 A 验证双向重算。
//! 2. 公海跨部门领取:u1(A) 的线索回收进公海(只改 lead_status,归属语义保持原 owner,
//!    #204 口径)→ u2(B) 经 `claim_lead_ownership`(pool.rs:153,两条领取路径共用的
//!    唯一归属实现)领取 → owner_id、department_id 同时跟随领取人
//!    (rls_dept 的 BEFORE UPDATE OF owner_id 触发链首次活体真验)。
//! 3. 回填可重入:`sql.rs` 常量(与迁移 up() 逐字符同源,经 `#[path]` 直引)连跑两遍,
//!    对齐不变量偏离数与备份表行数均零变化。
//!
//! 并发说明:nextest 用 --test-threads=1 串行跑 ignored(:1638);即便并行,users 的
//! UPDATE 与 5 表触发器同语句同事务提交,外部语句不可能观察到中间态漂移。

#[path = "../migration/src/domain/rls_dept_user_sync/sql.rs"]
#[allow(dead_code)] // DROP/RESTORE 等常量服务于迁移 down 侧,本文件只回放 up 语义
mod m_sync_sql;

mod test_common;

use bingxi_backend::models::crm_lead;
use bingxi_backend::models::status::crm_lead as lead_status;
use bingxi_backend::models::{crm_opportunity, customer, department, sales_order, supplier, user};
use bingxi_backend::services::crm::cust::CrmService;
use chrono::{DateTime, FixedOffset, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DatabaseConnection, DbBackend,
    EntityTrait, Statement,
};
use std::sync::Arc;

// =========================================================
// 活库夹具(与 contract_wave5_inspection_result_authority_test.rs:385-399 同语义:
// 缺变量必须响亮失败;sqlite 回退假绿在本文件是致命错,不是警告)
// =========================================================

async fn require_postgres(db: &DatabaseConnection) {
    let url = std::env::var("TEST_DATABASE_URL");
    assert!(
        matches!(&url, Ok(u) if u.starts_with("postgres")),
        "本用例覆盖 PG 触发器同步语义(CREATE TRIGGER ... EXECUTE FUNCTION / UPDATE ... FROM \
         为 PG 专有,sqlite 建不出等价结构),必须跑在 TEST_DATABASE_URL 指向的**已迁移** \
         PostgreSQL 上(ci-test-rust-ignored 注入,ci-cd.yml:1543;本地需显式 \
         export TEST_DATABASE_URL=postgres://...,禁止条件跳过假绿)。当前 TEST_DATABASE_URL={url:?}"
    );
    assert_eq!(
        db.get_database_backend(),
        DbBackend::Postgres,
        "夹具解析出的后端不是 PostgreSQL,活库触发器不可信(setup_test_db 无变量时静默回退 sqlite)"
    );
}

// =========================================================
// 通用助手
// =========================================================

async fn exec(db: &DatabaseConnection, sql: &str) {
    db.execute_unprepared(sql)
        .await
        .unwrap_or_else(|e| panic!("PG 语句执行失败: {e}\nSQL: {sql}"));
}

/// 单值计数查询(列别名固定 cnt)
async fn scalar_count(db: &DatabaseConnection, sql: &str) -> i64 {
    let row = db
        // SeaORM 2.0.2:原始 Statement 走 query_one_raw(query_one 只收 &impl StatementBuilder)
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            sql.to_string(),
        ))
        .await
        .unwrap_or_else(|e| panic!("count 查询失败: {e}\nSQL: {sql}"));
    let row = row.expect("count 查询应恒返回单行");
    row.try_get::<i64>("", "cnt")
        .unwrap_or_else(|e| panic!("count 列取值失败: {e}"))
}

/// 读取 5 表任意一行的 department_id 现值(表名仅由本文件内部字面量传入)
/// 读取 5 表任意一行的 department_id 现值(表名仅由本文件内部字面量传入)
async fn dept_col(db: &DatabaseConnection, table: &str, id: i32) -> Option<i32> {
    let sql = format!("SELECT department_id FROM {table} WHERE id = {id}");
    let row = db
        .query_one_raw(Statement::from_string(DbBackend::Postgres, sql))
        .await
        .unwrap_or_else(|e| panic!("读取 {table}#{id}.department_id 失败: {e}"));
    row.unwrap_or_else(|| panic!("{table}#{id} 行应存在"))
        .try_get::<Option<i32>>("", "department_id")
        .unwrap_or_else(|e| panic!("{table}#{id} department_id 取值失败: {e}"))
}

fn stamp() -> i64 {
    Utc::now().timestamp_nanos_opt().unwrap_or_default()
}

async fn seed_department(db: &DatabaseConnection, tag: &str) -> department::Model {
    let nanos = stamp();
    department::ActiveModel {
        name: Set(format!("部门同步活测-{tag}-{nanos}")),
        code: Set(format!("WD_SYNC_{tag}_{nanos}")),
        sort_order: Set(0),
        is_active: Set(true),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子部门({tag})失败: {e}"))
}

async fn seed_user(db: &DatabaseConnection, tag: &str, dept_id: i32) -> user::Model {
    let nanos = stamp();
    user::ActiveModel {
        username: Set(format!("wd_sync_{tag}_{nanos}")),
        // 测试种子行不经登录链路,hash 占位不参与任何断言
        password_hash: Set("wd-sync-live-test-not-a-real-hash".to_string()),
        department_id: Set(Some(dept_id)),
        is_active: Set(true),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子用户({tag})失败: {e}"))
}

/// 以 owner(归属人)身份各插一行到 5 张 RLS 表,返回行 id。
/// 插入即触发 rls_dept 的 BEFORE INSERT 触发器,department_id 应自动等于
/// owner 当时的 users.department_id——用例先断言这一点,再改部门断言重算。
struct OwnedIds {
    customer: i32,
    lead: i32,
    opportunity: i32,
    supplier: i32,
    sales_order: i32,
}

async fn seed_five_owned_rows(db: &DatabaseConnection, owner: &user::Model) -> OwnedIds {
    let nanos = stamp();
    let cust = customer::ActiveModel {
        customer_code: Set(format!("CUS-WS-{nanos}")),
        customer_name: Set("部门同步活测客户".to_string()),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
        customer_type: Set("retail".to_string()),
        owner_id: Set(owner.id),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子客户失败: {e}"));

    let so_now = Utc::now();
    let so = sales_order::ActiveModel {
        order_no: Set(format!("SO-WS-{nanos}")),
        customer_id: Set(cust.id),
        order_date: Set(so_now),
        required_date: Set(Some(so_now)),
        status: Set("PENDING".to_string()),
        subtotal: Set(Decimal::ZERO),
        tax_amount: Set(Decimal::ZERO),
        discount_amount: Set(Decimal::ZERO),
        shipping_cost: Set(Decimal::ZERO),
        total_amount: Set(Decimal::ZERO),
        paid_amount: Set(Decimal::ZERO),
        balance_amount: Set(Decimal::ZERO),
        created_by: Set(Some(owner.id)),
        created_at: Set(so_now),
        updated_at: Set(so_now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子销售订单失败: {e}"));

    let sup_now: DateTime<FixedOffset> = Utc::now().into();
    let sup = supplier::ActiveModel {
        supplier_code: Set(format!("SUP-WS-{nanos}")),
        supplier_name: Set("部门同步活测供应商".to_string()),
        supplier_short_name: Set("同供".to_string()),
        supplier_type: Set("面料供应商".to_string()),
        credit_code: Set("91330000WDTEST000X".to_string()),
        registered_address: Set("活测注册地址".to_string()),
        legal_representative: Set("活测法人".to_string()),
        registered_capital: Set(Decimal::ZERO),
        establishment_date: Set(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
        taxpayer_type: Set("一般纳税人".to_string()),
        bank_name: Set("活测银行".to_string()),
        bank_account: Set("6222000000000000".to_string()),
        contact_phone: Set("13800000000".to_string()),
        created_at: Set(sup_now),
        updated_at: Set(sup_now),
        created_by: Set(Some(owner.id)),
        is_processor: Set(false),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子供应商失败: {e}"));

    let lead = crm_lead::ActiveModel {
        lead_no: Set(format!("LEAD-WS-{nanos}")),
        lead_source: Set("展会".to_string()),
        lead_status: Set(Some(lead_status::NEW.to_string())),
        contact_name: Set("活测联系人".to_string()),
        owner_id: Set(owner.id),
        // owner_name 取真实种子用户名,不用 format!("用户{id}") 造名(本仓硬规则)
        owner_name: Set(owner.username.clone()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子线索失败: {e}"));

    let opp = crm_opportunity::ActiveModel {
        opportunity_no: Set(format!("OPP-WS-{nanos}")),
        opportunity_name: Set("部门同步活测商机".to_string()),
        customer_id: Set(cust.id),
        owner_id: Set(owner.id),
        owner_name: Set(owner.username.clone()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子商机失败: {e}"));

    OwnedIds {
        customer: cust.id,
        lead: lead.id,
        opportunity: opp.id,
        supplier: sup.id,
        sales_order: so.id,
    }
}

/// 按 FK 依赖序清理业务行;users/departments 保留(时间戳唯一键防重撞,
/// 且 audit_logs 已锚定操作用户 id——CI 库是 job 内容器,一次性用完即弃)
async fn cleanup_owned_rows(db: &DatabaseConnection, ids: &OwnedIds) {
    sales_order::Entity::delete_by_id(ids.sales_order)
        .exec(db)
        .await
        .unwrap_or_else(|e| panic!("清理 sales_order#{} 失败: {e}", ids.sales_order));
    crm_opportunity::Entity::delete_by_id(ids.opportunity)
        .exec(db)
        .await
        .unwrap_or_else(|e| panic!("清理 crm_opportunity#{} 失败: {e}", ids.opportunity));
    crm_lead::Entity::delete_by_id(ids.lead)
        .exec(db)
        .await
        .unwrap_or_else(|e| panic!("清理 crm_lead#{} 失败: {e}", ids.lead));
    supplier::Entity::delete_by_id(ids.supplier)
        .exec(db)
        .await
        .unwrap_or_else(|e| panic!("清理 supplier#{} 失败: {e}", ids.supplier));
    customer::Entity::delete_by_id(ids.customer)
        .exec(db)
        .await
        .unwrap_or_else(|e| panic!("清理 customer#{} 失败: {e}", ids.customer));
}

// =========================================================
// 用例 1:users.department_id 运行期变更 → 5 表冗余列同事务重算
// =========================================================

#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL(trg_users_dept_sync 为 PG 专有触发器,由 ci-test-rust-ignored 执行)"]
async fn live_user_dept_change_realigns_five_tables() {
    let db = test_common::setup_test_db().await;
    require_postgres(&db).await;

    let dept_a = seed_department(&db, "A").await;
    let dept_b = seed_department(&db, "B").await;
    let owner = seed_user(&db, "owner", dept_a.id).await;
    let ids = seed_five_owned_rows(&db, &owner).await;

    // 插入瞬间:rls_dept 的 BEFORE INSERT 触发器已把 5 行染成部门 A
    assert_eq!(
        dept_col(&db, "customers", ids.customer).await,
        Some(dept_a.id),
        "customers 插入即应由 trg_customers_dept 反查 owner 部门"
    );
    assert_eq!(dept_col(&db, "crm_lead", ids.lead).await, Some(dept_a.id));
    assert_eq!(
        dept_col(&db, "crm_opportunity", ids.opportunity).await,
        Some(dept_a.id)
    );
    assert_eq!(
        dept_col(&db, "suppliers", ids.supplier).await,
        Some(dept_a.id)
    );
    assert_eq!(
        dept_col(&db, "sales_orders", ids.sales_order).await,
        Some(dept_a.id)
    );

    // 运行期换部门(与 user_service.rs:364 同一列的 UPDATE;AFTER 触发器应同事务重算 5 表)
    exec(
        &db,
        &format!(
            "UPDATE users SET department_id = {} WHERE id = {}",
            dept_b.id, owner.id
        ),
    )
    .await;

    assert_eq!(
        dept_col(&db, "customers", ids.customer).await,
        Some(dept_b.id),
        "换部门后 customers.department_id 必须跟随归属人新部门(旧部门越界可见=泄漏)"
    );
    assert_eq!(dept_col(&db, "crm_lead", ids.lead).await, Some(dept_b.id));
    assert_eq!(
        dept_col(&db, "crm_opportunity", ids.opportunity).await,
        Some(dept_b.id)
    );
    assert_eq!(
        dept_col(&db, "suppliers", ids.supplier).await,
        Some(dept_b.id),
        "suppliers 按 created_by 归属,同样必须重算"
    );
    assert_eq!(
        dept_col(&db, "sales_orders", ids.sales_order).await,
        Some(dept_b.id)
    );

    // 改回部门 A:双向都重算(触发器 WHEN 谓词只挡"值未变",不挡方向)
    exec(
        &db,
        &format!(
            "UPDATE users SET department_id = {} WHERE id = {}",
            dept_a.id, owner.id
        ),
    )
    .await;
    assert_eq!(
        dept_col(&db, "customers", ids.customer).await,
        Some(dept_a.id)
    );
    assert_eq!(dept_col(&db, "crm_lead", ids.lead).await, Some(dept_a.id));
    assert_eq!(
        dept_col(&db, "crm_opportunity", ids.opportunity).await,
        Some(dept_a.id)
    );
    assert_eq!(
        dept_col(&db, "suppliers", ids.supplier).await,
        Some(dept_a.id)
    );
    assert_eq!(
        dept_col(&db, "sales_orders", ids.sales_order).await,
        Some(dept_a.id)
    );

    cleanup_owned_rows(&db, &ids).await;
}

// =========================================================
// 用例 2:公海跨部门领取 → owner_id 与 department_id 同时跟随领取人
// =========================================================

#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL(领取链依赖 crm_lead 的 BEFORE UPDATE OF owner_id 触发器,由 ci-test-rust-ignored 执行)"]
async fn live_pool_claim_cross_dept_follows_owner_and_dept() {
    let db = test_common::setup_test_db().await;
    require_postgres(&db).await;

    let dept_a = seed_department(&db, "A").await;
    let dept_b = seed_department(&db, "B").await;
    let u1 = seed_user(&db, "orig", dept_a.id).await;
    let u2 = seed_user(&db, "claimer", dept_b.id).await;

    let nanos = stamp();
    let seeded = crm_lead::ActiveModel {
        lead_no: Set(format!("LEAD-WS2-{nanos}")),
        lead_source: Set("官网".to_string()),
        lead_status: Set(Some(lead_status::NEW.to_string())),
        contact_name: Set("活测联系人2".to_string()),
        owner_id: Set(u1.id),
        owner_name: Set(u1.username.clone()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap_or_else(|e| panic!("种子线索失败: {e}"));
    let lead_id = seeded.id;
    assert!(lead_id > 0, "种子线索应返回有效主键");

    // 回收进公海:#204 口径——recycle 只写 lead_status=pool,owner/department 保持原值
    let pooled_model = crm_lead::Entity::find_by_id(lead_id)
        .one(&db)
        .await
        .unwrap()
        .expect("种子线索应存在");
    let mut pooled: crm_lead::ActiveModel = pooled_model.clone().into();
    pooled.lead_status = Set(Some(lead_status::POOL.to_string()));
    let pooled = pooled
        .update(&db)
        .await
        .unwrap_or_else(|e| panic!("回收进公海失败: {e}"));
    assert_eq!(
        pooled.department_id,
        Some(dept_a.id),
        "公海行的 department_id 仍跟随原 owner(回收不改归属,rls_dept/mod.rs:76-77 口径)"
    );

    // 跨部门领取:走两条领取路径共用的唯一归属实现 claim_lead_ownership(pool.rs:153)
    let svc = CrmService::new(Arc::new(db.clone()));
    let claimed = svc
        .claim_lead_ownership(pooled, u2.id, &u2.username)
        .await
        .unwrap_or_else(|e| panic!("公海领取必须成功,实际: {e:?}"));
    assert_eq!(claimed.owner_id, u2.id, "领取后归属人必须是领取人");
    assert_eq!(claimed.lead_status.as_deref(), Some(lead_status::NEW));

    // 核心断言(触发链活体验证):UPDATE ... SET owner_id=u2 命中
    // BEFORE UPDATE OF owner_id 触发器,department_id 必须同步为领取人部门 B
    let fresh = crm_lead::Entity::find_by_id(lead_id)
        .one(&db)
        .await
        .unwrap()
        .expect("领取后线索行应存在");
    assert_eq!(fresh.owner_id, u2.id);
    assert_eq!(
        fresh.department_id,
        Some(dept_b.id),
        "领取人属部门 B,lead.department_id 必须同步为 B——否则 B 部门列表看不到刚领取的行、\
         A 部门经理仍可见(越界)"
    );

    crm_lead::Entity::delete_by_id(lead_id)
        .exec(&db)
        .await
        .unwrap_or_else(|e| panic!("清理 crm_lead#{lead_id} 失败: {e}"));
}

// =========================================================
// 用例 3:回填/备份可重入(与迁移 up() 同源 SQL 连跑两遍零差异)
// =========================================================

#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL(重放的是 m_rls_dept_user_sync 的 PG 回填语句,由 ci-test-rust-ignored 执行)"]
async fn live_backfill_and_backup_are_reentrant() {
    let db = test_common::setup_test_db().await;
    require_postgres(&db).await;

    // 第一遍重放(迁移 up 已由 CI migrate 步骤执行过,这里等价重放 up 的数据语句)
    exec(&db, m_sync_sql::CREATE_BACKUP_TABLE_SQL).await;
    exec(&db, m_sync_sql::BACKUP_BEFORE_ALIGN_SQL).await;
    exec(&db, m_sync_sql::BACKFILL_ALIGN_SQL).await;
    let drift1 = scalar_count(
        &db,
        &format!(
            "SELECT count(*) AS cnt FROM ({}) drift",
            m_sync_sql::ALIGNMENT_CHECK_SQL
        ),
    )
    .await;
    let backup1 = scalar_count(&db, "SELECT count(*) AS cnt FROM mig_user_dept_sync_backup").await;

    // 第二遍重放:幂等语句不应产生任何新差异
    exec(&db, m_sync_sql::BACKUP_BEFORE_ALIGN_SQL).await;
    exec(&db, m_sync_sql::BACKFILL_ALIGN_SQL).await;
    let drift2 = scalar_count(
        &db,
        &format!(
            "SELECT count(*) AS cnt FROM ({}) drift",
            m_sync_sql::ALIGNMENT_CHECK_SQL
        ),
    )
    .await;
    let backup2 = scalar_count(&db, "SELECT count(*) AS cnt FROM mig_user_dept_sync_backup").await;

    assert_eq!(
        drift1, 0,
        "迁移 up() 执行后不变量应成立:5 表 department_id 恒等于归属人当前部门,\
         偏离 {drift1} 行意味着回填/触发器存在漏网路径"
    );
    assert_eq!(drift1, drift2, "回填重放第二遍不得引入新的偏离行");
    assert_eq!(
        backup1, backup2,
        "备份 INSERT 带 NOT EXISTS 守卫,重放不得重复插入备份行"
    );
}
