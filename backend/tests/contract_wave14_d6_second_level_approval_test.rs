//! 契约测试：大客户转单二级审批「非 CONVERTED ⇒ 单级」行为锁（正向对照 + 单级负向锁）
//!
//! 锁的判据（源码 `services/crm/customer_transfer_approval_service.rs`）：
//! - 创建门 `fetch_and_validate_lead` 拒绝 `lead_status=CONVERTED` 线索；
//! - `max_level = if check_large_customer {...} else {1}`，`check_large_customer` 的
//!   **唯一输入**是线索 `converted_customer_id` 所指向客户的分层列 `customers.tier`
//!   是否落在 `constants::customer_tier::MAJOR`。
//! 结论：正常生命周期下，能过创建门的线索恒非 CONVERTED、其 `converted_customer_id`
//! 恒为 NULL（该列只在转化时与 CONVERTED 同时写入），故 `max_level` 恒为 1、二级
//! （总监）分支不可达。本文件把「仅一级生效」钉成行为锁，并用正向对照排除「代码把
//! max_level 写死成 1」的假绿：同一条判据喂进 `converted_customer_id`→高档分层客户时
//! `max_level` 必须为 2。
//!
//! 为什么必须真库：判据跨 crm_lead→customers 两真表（经 FK 关联），种子 tier 值必须
//! 真实穿过 CHECK `chk_customers_tier`；审批单号走 `utils/number_generator` 真实取号。
//! 夹具 `setup_test_db()` 缺 `TEST_DATABASE_URL` 或指向 sqlite 直接 panic——本文件行为
//! 断言只有 CI 活库才有结果（本机仅证明编译与格式）。
//!
//! 参照表与 id 带：`departments.id=1` 属密封参照表只引用；本文件独占 996xxx id 带
//! （users 996001/996002、customers 996041、crm_lead 996021/996022），每用例先删后插
//! 保幂等（删序 子(线索)→父(客户)→users）。

mod test_common;

use bingxi_backend::constants::customer_tier::{self, VIP};
use bingxi_backend::models::status::crm_lead as lead_status;
use bingxi_backend::services::crm::customer_transfer_approval_service::{
    CreateTransferApprovalRequest, CustomerTransferApprovalService, TransferApprovalDto,
};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use std::sync::Arc;
use test_common::setup_test_db;

const USER_OWNER: i32 = 996_001;
const USER_TARGET: i32 = 996_002;
/// 高档分层客户（tier=VIP ∈ MAJOR），供正向对照线索挂载
const CUSTOMER_MAJOR: i32 = 996_041;
/// 未挂客户（converted_customer_id NULL）的未转化线索——负向锁，模拟正常生命周期形态
const LEAD_NO_CUSTOMER: i32 = 996_021;
/// 挂着高档分层客户的未转化线索——正向对照，证明判据读数据而非把级数写死
const LEAD_MAJOR: i32 = 996_022;

fn pg_stmt(sql: &str) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, Vec::<sea_orm::Value>::new())
}

async fn exec(db: &DatabaseConnection, sql: &str) {
    db.execute_raw(pg_stmt(sql))
        .await
        .unwrap_or_else(|e| panic!("种子 SQL 执行失败: {e}\nSQL: {sql}"));
}

/// 场景种子：users → customers(tier=VIP) → 两条未转化(new)线索，一条不挂客户（负向）、
/// 一条挂上述高档客户（正向对照）。先删后插保幂等。
async fn seed_scene(db: &DatabaseConnection) {
    exec(
        db,
        &format!("DELETE FROM crm_lead WHERE id IN ({LEAD_NO_CUSTOMER},{LEAD_MAJOR})"),
    )
    .await;
    exec(
        db,
        &format!("DELETE FROM customers WHERE id = {CUSTOMER_MAJOR}"),
    )
    .await;
    exec(
        db,
        &format!("DELETE FROM users WHERE id IN ({USER_OWNER},{USER_TARGET})"),
    )
    .await;
    exec(
        db,
        &format!(
            "INSERT INTO users (id,username,password_hash,is_active,is_totp_enabled,department_id,created_at,updated_at) VALUES
             ({USER_OWNER},'tier_lock_owner','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
             ({USER_TARGET},'tier_lock_target','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"
        ),
    )
    .await;
    // 高档分层客户：tier=VIP 必须真实穿过 chk_customers_tier 且 ∈ MAJOR
    assert!(
        customer_tier::is_major(VIP),
        "种子前置：VIP 必须在 MAJOR 高档集合内，否则正向对照失去意义"
    );
    exec(
        db,
        &format!(
            "INSERT INTO customers (id,customer_code,customer_name,credit_limit,payment_terms,status,owner_id,created_by,customer_type,tier,created_at,updated_at)
             VALUES ({CUSTOMER_MAJOR},'CUST-{CUSTOMER_MAJOR}','高档分层客户',0,30,'active',{USER_OWNER},{USER_OWNER},'other','VIP','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"
        ),
    )
    .await;
    // 负向线索：未转化(new) 且未挂客户(converted_customer_id NULL)——正常生命周期恒有的形态
    exec(
        db,
        &format!(
            "INSERT INTO crm_lead (id,lead_no,lead_source,lead_status,company_name,contact_name,mobile_phone,owner_id,owner_name,converted_customer_id,created_at,updated_at)
             VALUES ({LEAD_NO_CUSTOMER},'LD-{LEAD_NO_CUSTOMER}','website','{new}','无客户线索','李四','13900000000',{USER_OWNER},'源归属人',NULL,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
            new = lead_status::NEW
        ),
    )
    .await;
    // 正向对照线索：状态同为未转化(new, 可过创建门)，但 converted_customer_id 挂高档分层客户
    exec(
        db,
        &format!(
            "INSERT INTO crm_lead (id,lead_no,lead_source,lead_status,company_name,contact_name,mobile_phone,owner_id,owner_name,converted_customer_id,created_at,updated_at)
             VALUES ({LEAD_MAJOR},'LD-{LEAD_MAJOR}','website','{new}','高档关联线索','王五','13900000001',{USER_OWNER},'源归属人',{CUSTOMER_MAJOR},'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
            new = lead_status::NEW
        ),
    )
    .await;
}

async fn create(db: &Arc<DatabaseConnection>, lead_id: i32) -> TransferApprovalDto {
    CustomerTransferApprovalService::new(db.clone())
        .create_approval(
            CreateTransferApprovalRequest {
                lead_id,
                to_user_id: USER_TARGET,
                reason: "二级审批可达性锁：申请转移".to_string(),
            },
            USER_OWNER,
            "tier_lock_owner",
        )
        .await
        .unwrap_or_else(|e| panic!("创建转移审批失败(lead={lead_id}): {e}"))
}

/// 负向锁：未转化且未挂客户的线索（正常生命周期恒有的形态）创建审批恒得单级 max_level=1。
#[tokio::test]
async fn non_converted_lead_without_converted_customer_is_single_level() {
    let db = Arc::new(setup_test_db().await);
    seed_scene(&db).await;

    // 前置自证：该线索确实非 CONVERTED、converted_customer_id 确为 NULL（本锁的判据前提）
    let row = db
        .query_one_raw(pg_stmt(&format!(
            "SELECT lead_status, converted_customer_id FROM crm_lead WHERE id={LEAD_NO_CUSTOMER}"
        )))
        .await
        .expect("回读线索失败")
        .expect("线索行缺失");
    let status: Option<String> = row
        .try_get("", "lead_status")
        .expect("lead_status 解码失败");
    let conv: Option<i32> = row
        .try_get("", "converted_customer_id")
        .expect("converted_customer_id 解码失败");
    assert_ne!(
        status.as_deref(),
        Some(lead_status::CONVERTED),
        "种子必须是非 CONVERTED 线索才可过创建门，实测 lead_status={status:?}"
    );
    assert_eq!(
        conv, None,
        "种子线索 converted_customer_id 应为 NULL（正常生命周期形态），实测 {conv:?}"
    );

    let approval = create(&db, LEAD_NO_CUSTOMER).await;
    assert!(
        !approval.is_large_customer,
        "未挂客户的未转化线索必须判非大客户，实际 is_large_customer={}",
        approval.is_large_customer
    );
    assert_eq!(
        approval.max_level, 1,
        "非大客户审批级数必须为 1（源码 `max_level = if is_large_customer {{2}} else {{1}}`），实际 {}",
        approval.max_level
    );
    assert_eq!(
        approval.current_level, 1,
        "新建审批单从经理层（current_level=1）起步"
    );
}

/// 正向对照：同一条判据喂进 converted_customer_id→高档分层客户时 max_level 必须为 2，
/// 证明负向的「恒为 1」来自数据前提（converted_customer_id NULL）而非把级数写死，排除假绿。
#[tokio::test]
async fn lead_carrying_major_tier_customer_reaches_second_level() {
    let db = Arc::new(setup_test_db().await);
    seed_scene(&db).await;

    let approval = create(&db, LEAD_MAJOR).await;
    assert!(
        approval.is_large_customer,
        "挂高档分层客户（tier=VIP∈MAJOR）的线索必须判大客户，否则判据未生效（疑被写死单级）"
    );
    assert_eq!(
        approval.max_level, 2,
        "大客户审批级数必须为 2，实际 max_level={}",
        approval.max_level
    );
    assert_eq!(
        approval.current_level, 1,
        "新建审批单仍从经理层（1）起步，二级指 max_level=2 的后续总监层"
    );
    assert!(
        approval.approval_no.starts_with("TA"),
        "审批单号经统一取号器产出 TA 前缀（证明走生产 service 路径），实际 {:?}",
        approval.approval_no
    );
}
