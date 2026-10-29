//! 契约测试：CONVERTED 且所挂客户非大高档 ⇒ 建单必拒（跨放行持久负向锁）
//!
//! 锁的判据（源码 `services/crm/customer_transfer_approval_service.rs::fetch_and_validate_lead`
//! 与 `check_large_customer`）：已转化（lead_status=CONVERTED）线索发起转移审批，仅当
//! `converted_customer_id` 所挂客户的分层列 `customers.tier` 落在
//! `constants::customer_tier::MAJOR` 高档集合时才允许建单；分层为低档（SILVER/NORMAL）、
//! 未定档（NULL）一律拒绝，且拒绝必须零审批行落库。
//!
//! 本锁跨形态有效：无论创建门处于"全部 CONVERTED 拒绝"的收口形态，还是放行
//! "CONVERTED 且 MAJOR"的放开形态，**非大客户的 CONVERTED 恒被拒**——放开二级不得
//! 顺手拆掉一级审批的门前置，本文件把这条负向不变量钉死。
//!
//! 为什么必须真库：判据跨 crm_lead→customers 两真表（FK 关联），种子 tier 必须真实穿过
//! DB CHECK `chk_customers_tier`；夹具 `setup_test_db()` 缺 `TEST_DATABASE_URL` 或指向
//! sqlite 直接 panic——行为断言只有 CI 活库才有结果（本机仅证明编译与格式），交 CI 验证。
//!
//! id 带：本文件独占 985xxx 带（users 985001/985002、customers 985041/985042、
//! crm_lead 985021/985022），每用例先删后插保幂等，删序 审批单→线索→客户→users。

mod test_common;

use bingxi_backend::constants::customer_tier::SILVER;
use bingxi_backend::models::customer_transfer_approval::{self, Entity as TransferApprovalEntity};
use bingxi_backend::models::status::crm_lead as lead_status;
use bingxi_backend::services::crm::customer_transfer_approval_service::{
    CreateTransferApprovalRequest, CustomerTransferApprovalService,
};
use bingxi_backend::utils::error::AppError;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, PaginatorTrait, QueryFilter,
    Statement,
};
use std::sync::Arc;
use test_common::setup_test_db;

const USER_OWNER: i32 = 985_001;
const USER_TARGET: i32 = 985_002;
/// 低档客户（tier=SILVER，集合内但非 MAJOR）
const CUSTOMER_SILVER: i32 = 985_041;
/// 未定档客户（tier=NULL，无依据不判大客户）
const CUSTOMER_UNSET: i32 = 985_042;
/// CONVERTED 且挂低档客户的线索
const LEAD_SILVER: i32 = 985_021;
/// CONVERTED 且挂未定档客户的线索
const LEAD_UNSET: i32 = 985_022;

fn pg_stmt(sql: &str) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, Vec::<sea_orm::Value>::new())
}

async fn exec(db: &DatabaseConnection, sql: &str) {
    db.execute_raw(pg_stmt(sql))
        .await
        .unwrap_or_else(|e| panic!("种子 SQL 执行失败: {e}\nSQL: {sql}"));
}

async fn seed_scene(db: &DatabaseConnection) {
    exec(
        db,
        &format!(
            "DELETE FROM customer_transfer_approvals WHERE lead_id IN ({LEAD_SILVER},{LEAD_UNSET})"
        ),
    )
    .await;
    exec(
        db,
        &format!("DELETE FROM crm_lead WHERE id IN ({LEAD_SILVER},{LEAD_UNSET})"),
    )
    .await;
    exec(
        db,
        &format!("DELETE FROM customers WHERE id IN ({CUSTOMER_SILVER},{CUSTOMER_UNSET})"),
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
             ({USER_OWNER},'nonmajor_owner','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
             ({USER_TARGET},'nonmajor_target','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"
        ),
    )
    .await;
    exec(
        db,
        &format!(
            "INSERT INTO customers (id,customer_code,customer_name,credit_limit,payment_terms,status,owner_id,created_by,customer_type,tier,created_at,updated_at) VALUES
             ({CUSTOMER_SILVER},'CUST-{CUSTOMER_SILVER}','低档客户',0,30,'active',{USER_OWNER},{USER_OWNER},'other','{SILVER}','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
             ({CUSTOMER_UNSET},'CUST-{CUSTOMER_UNSET}','未定档客户',0,30,'active',{USER_OWNER},{USER_OWNER},'other',NULL,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"
        ),
    )
    .await;
    for (lead_id, customer_id, tag, phone) in [
        (LEAD_SILVER, CUSTOMER_SILVER, "silv", "13900000001"),
        (LEAD_UNSET, CUSTOMER_UNSET, "unse", "13900000002"),
    ] {
        exec(
            db,
            &format!(
                "INSERT INTO crm_lead (id,lead_no,lead_source,lead_status,company_name,contact_name,mobile_phone,owner_id,owner_name,converted_customer_id,created_at,updated_at)
                 VALUES ({lead_id},'LD-{tag}','website','{converted}','已转化非大客户','赵六','{phone}',{USER_OWNER},'源归属人',{customer_id},'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
                converted = lead_status::CONVERTED
            ),
        )
        .await;
    }
}

async fn create_attempt(db: &Arc<DatabaseConnection>, lead_id: i32) -> AppError {
    CustomerTransferApprovalService::new(db.clone())
        .create_approval(
            CreateTransferApprovalRequest {
                lead_id,
                to_user_id: USER_TARGET,
                reason: "非大客户 CONVERTED 负向锁：申请转移".to_string(),
            },
            USER_OWNER,
            "nonmajor_owner",
        )
        .await
        .expect_err("CONVERTED 且非 MAJOR 分层的线索建单必须被拒（跨放行形态恒真）")
}

/// 低档（SILVER）分支：建单拒绝（业务族）且零审批行落库。
#[tokio::test]
async fn converted_lead_on_silver_tier_customer_is_rejected_with_zero_rows() {
    let db = Arc::new(setup_test_db().await);
    seed_scene(&db).await;

    let err = create_attempt(&db, LEAD_SILVER).await;
    assert!(
        matches!(
            err,
            AppError::BusinessError(_) | AppError::BusinessErrorDisplayable(_)
        ),
        "创建门拒绝必须归业务族（前置状态未满足），不得漂成 500 或 403，实际: {err:?}"
    );
    let rows = TransferApprovalEntity::find()
        .filter(customer_transfer_approval::Column::LeadId.eq(LEAD_SILVER))
        .count(&*db)
        .await
        .expect("审批行计数失败");
    assert_eq!(rows, 0, "被拒建单不得留下任何审批行，实际 {rows} 条");
}

/// 未定档（NULL）分支：无依据不判大客户，建单拒绝且零审批行落库。
#[tokio::test]
async fn converted_lead_on_unsettled_tier_customer_is_rejected_with_zero_rows() {
    let db = Arc::new(setup_test_db().await);
    seed_scene(&db).await;

    let err = create_attempt(&db, LEAD_UNSET).await;
    assert!(
        matches!(
            err,
            AppError::BusinessError(_) | AppError::BusinessErrorDisplayable(_)
        ),
        "未定档分支拒绝同样必须归业务族，实际: {err:?}"
    );
    let rows = TransferApprovalEntity::find()
        .filter(customer_transfer_approval::Column::LeadId.eq(LEAD_UNSET))
        .count(&*db)
        .await
        .expect("审批行计数失败");
    assert_eq!(rows, 0, "被拒建单不得留下任何审批行，实际 {rows} 条");
}
