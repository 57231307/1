//! 坏账核销主链路集成测试（需真实 PostgreSQL，`#[ignore]`，不进常规 CI）。
//!
//! 运行：`TEST_DATABASE_URL=postgres://user:pass@host:5432/bingxi_test \
//!         cargo test --test bad_debt_writeoff_main_link_test -- --ignored`
//! 前置：库已跑完迁移（含 v15 建表与 ar_invoices 存在）。
//!
//! 语义断言（锁死修复）：审核通过（ar_invoice.approval_status 大写 APPROVED）的应收单
//! 可以发起并走完坏账核销二级审批至终态。修复前比较点用小写 "approved"，
//! create_writeoff 会恒判「未审核通过」→ 本测试在修复前应失败、修复后应通过。

mod test_common;

use std::sync::Arc;

use chrono::{TimeZone, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};

use bingxi_backend::models::ar_invoice::{self, ActiveModel as ArInvoiceActive};
use bingxi_backend::models::customer::ActiveModel as CustomerActive;
use bingxi_backend::models::status::common;
use bingxi_backend::models::user::ActiveModel as UserActive;

use bingxi_backend::models::bad_debt_dto::{ApproveWriteoffRequest, CreateWriteoffRequest};
use bingxi_backend::models::status::bad_debt_writeoff_status as writeoff_status;
use bingxi_backend::services::bad_debt_service::BadDebtService;
use test_common::setup_test_db;

async fn seed_user(db: &sea_orm::DatabaseConnection, name: &str) -> i32 {
    let now = Utc::now();
    let u = UserActive {
        username: Set(name.to_string()),
        password_hash: Set("x".repeat(60)),
        real_name: Set(Some(name.to_string())),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    u.id
}

async fn seed_customer(db: &sea_orm::DatabaseConnection, owner_id: i32) -> i32 {
    let now = Utc::now();
    let c = CustomerActive {
        customer_code: Set(format!("CUST-{owner_id}-{now:}")),
        customer_name: Set("坏账核销测试客户".to_string()),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
        customer_type: Set("company".to_string()),
        owner_id: Set(owner_id),
        created_by: Set(Some(owner_id)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    c.id
}

/// 写入一张「已审批通过」的应收单（approval_status 走写入点词表：大写 APPROVED）。
async fn seed_approved_invoice(
    db: &sea_orm::DatabaseConnection,
    customer_id: i32,
    created_by: i32,
) -> (i32, Decimal) {
    let now = Utc::now();
    let inv = ArInvoiceActive {
        invoice_no: Set(format!("AR-{now:}-{customer_id}")),
        invoice_date: Set(now.date_naive()),
        due_date: Set(Utc
            .with_ymd_and_hms(2022, 1, 1, 0, 0, 0)
            .unwrap()
            .date_naive()),
        customer_id: Set(customer_id),
        invoice_amount: Set(Decimal::new(100000, 2)),
        received_amount: Set(Decimal::ZERO),
        unpaid_amount: Set(Decimal::new(100000, 2)),
        status: Set(common::STATUS_APPROVED.to_string()),
        approval_status: Set(common::STATUS_APPROVED.to_string()),
        created_by: Set(created_by),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    (inv.id, inv.unpaid_amount)
}

#[tokio::test]
#[ignore] // 需真实已迁移 PostgreSQL，仅在有 TEST_DATABASE_URL 的活库环境显式运行
async fn approved_ar_invoice_can_be_written_off_end_to_end() {
    let db = setup_test_db().await;
    let svc = BadDebtService::new(Arc::new(db.clone()));

    let applicant = seed_user(&db, "bd_applicant").await;
    let finance = seed_user(&db, "bd_finance").await;
    let gm = seed_user(&db, "bd_gm").await;
    let customer = seed_customer(&db, applicant).await;
    let (invoice_id, unpaid) = seed_approved_invoice(&db, customer, applicant).await;

    // 1. 审核通过的应收单可发起核销（修复前此处恒失败）
    let wo = svc
        .create_writeoff(
            CreateWriteoffRequest {
                customer_id: customer as i64,
                ar_invoice_id: invoice_id,
                writeoff_amount: unpaid,
                reason: "确认无法收回".to_string(),
                remark: None,
            },
            applicant,
            "bd_applicant".to_string(),
        )
        .await
        .expect("审核通过的应收单应能核销，create_writeoff 不应报错");
    assert_eq!(wo.approval_status, writeoff_status::PENDING);

    // 2. 财务经理一级审批
    let wo = svc
        .finance_approve(wo.id, finance, ApproveWriteoffRequest { comment: None })
        .await
        .expect("一级审批应通过");
    assert_eq!(wo.approval_status, writeoff_status::FINANCE_APPROVED);

    // 3. 总经理二级审批 → 终态
    let wo = svc
        .general_manager_approve(wo.id, gm, ApproveWriteoffRequest { comment: None })
        .await
        .expect("二级审批应通过");
    assert_eq!(wo.approval_status, writeoff_status::APPROVED);
    assert!(wo.completed_at.is_some());

    // 4. 回读确认落库终态
    let reloaded = ar_invoice::Entity::find_by_id(invoice_id)
        .one(&db)
        .await
        .unwrap()
        .expect("应收单应仍存在");
    assert_eq!(reloaded.approval_status, common::STATUS_APPROVED);
}
