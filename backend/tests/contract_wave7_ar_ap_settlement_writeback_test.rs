//! AP/AR 结算「金额回写」真库契约锁（金额列必须显式 Set 才真实落库的防回潮钉）
//!
//! 机制锁定（本锁钉死的 sea-orm 2.0.2 行为语义）：
//! - sea-orm 2.0.2 中 `DeriveActiveModel` 生成的 `From<Model> for ActiveModel` 将**全字段**
//!   标为 `ActiveValue::Unchanged`（sea-orm-macros active_model.rs L115），而 `Update` 只把
//!   `ActiveValue::Set` 列写进 SQL（sea-orm query/update.rs L113-117）。
//!   因此「先在 Model 上直接改金额字段、再 `.into()` 交给 `update_with_audit`」的写法
//!   实际生成的 UPDATE 只带 `updated_at` 一列——金额累加停留在内存副本，**接口 200、库中恒 0**。
//! - 命中的写入路径（全部要求显式 Set 后真实落库）：
//!   1. `ap_payment_service.rs::apply_invoice_payment` —— 付款确认分摊回写
//!      paid_amount/unpaid_amount/invoice_status（04-02、11-01 两钉的钉点）；
//!   2. `ap_verification_service.rs::update_invoice_for_item_txn / process_verify_items /
//!      restore_invoices_on_cancel` —— AP 自动/手工核销回写与 cancel 精确回退；
//!   3. `ar_ops/verification_ops/auto.rs::batch_update_invoice_states` —— AR 自动核销回写
//!      received_amount/unpaid_amount/status（11-02 auto 断言的钉点）；
//!   4. `ar_ops/verification_ops/manual.rs::rollback_invoices` —— AR 取消核销金额回退。
//! - `finance_report_service.rs::get_trial_balance` —— account_subjects.status 写入方词表为
//! 小写 `master_data::ACTIVE`（DDL DEFAULT 'active'，有意与大写 common 区分），
//!   若用大写 `"ACTIVE"` 等值过滤则所有科目被排除、entries 恒空（10-01 钉的锁定面）。
//! - `ap_payment_service.rs::create` —— 未审批付款申请的状态门拒绝用
//!   `AppError::business_displayable`：机器码仍 BUSINESS_ERROR（族不变），出参 message 外显
//!   用户下一步操作所需的公开规则，内部状态 token 只进日志（05-01 钉）。
//!
//! 夹具口径：全部活库用例走 `test_common::setup_test_db()`（真 PostgreSQL，缺
//! `TEST_DATABASE_URL` 直接 panic，禁 sqlite 回退）；业务表每次清空，FK 父行（users/
//! customers 等非 sealed 表）由用例自种子；suppliers 虽为 sealed 参照表但迁移不保证
//! 有可用行，同样自建并用时间戳唯一码避免撞已有行。

mod test_common;

use axum::response::IntoResponse;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use std::str::FromStr;
use std::sync::Arc;

use bingxi_backend::models::status::{
    account_subject as subject_status, ap_invoice as ap_status, ar as ar_status, common,
    master_data, payment,
};
use bingxi_backend::models::{
    account_subject, ap_invoice, ap_payment, ap_payment_request, ap_payment_request_item,
    ar_collection, ar_invoice, customer, supplier, user,
};
use bingxi_backend::services::ap_payment_service::{
    ApPaymentService, CreateApPaymentRequest, UpdateApPaymentRequest,
};
use bingxi_backend::services::ap_verification_service::{
    ApVerificationItemDto, ApVerificationService, ManualVerifyRequest,
};
use bingxi_backend::services::ar_service::ArService;
use bingxi_backend::services::finance_report_service::FinanceReportService;
use bingxi_backend::utils::error::AppError;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn unique_tag() -> i64 {
    Utc::now()
        .timestamp_nanos_opt()
        .expect("Utc::now 纳秒必落在 chrono 可表示区间（约 1678-2262 年），None 不可达")
}

fn today() -> NaiveDate {
    Utc::now().date_naive()
}

// =========================================================
// 种子助手（FK 父行自建，不依赖外部库状态）
// =========================================================

async fn seed_user(db: &DatabaseConnection, name: &str) -> i32 {
    let now = Utc::now();
    user::ActiveModel {
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
    .unwrap()
    .id
}

async fn seed_supplier(db: &DatabaseConnection) -> i32 {
    let now = Utc::now();
    supplier::ActiveModel {
        supplier_code: Set(format!("SUP-W7-{}", unique_tag())),
        supplier_name: Set("W7回写测试供应商".to_string()),
        supplier_short_name: Set("W7供".to_string()),
        supplier_type: Set("面料供应商".to_string()),
        credit_code: Set("91330000W7T00000X1".to_string()),
        registered_address: Set("测试注册地址".to_string()),
        legal_representative: Set("测试法人".to_string()),
        registered_capital: Set(Decimal::ZERO),
        establishment_date: Set(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
        taxpayer_type: Set("一般纳税人".to_string()),
        bank_name: Set("测试银行".to_string()),
        bank_account: Set("6222000000000001".to_string()),
        contact_phone: Set("13800000001".to_string()),
        created_at: Set(now.into()),
        updated_at: Set(now.into()),
        is_processor: Set(false),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
    .id
}

async fn seed_customer(db: &DatabaseConnection, owner_id: i32) -> i32 {
    let now = Utc::now();
    customer::ActiveModel {
        customer_code: Set(format!("CUS-W7-{}", unique_tag())),
        customer_name: Set("W7回写测试客户".to_string()),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set(master_data::ACTIVE.to_string()),
        customer_type: Set("retail".to_string()),
        owner_id: Set(owner_id),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
    .id
}

/// 建应付单：AUDITED、paid=0、unpaid=amount（approve 之后的稳态形状）
async fn seed_ap_invoice(
    db: &DatabaseConnection,
    supplier_id: i32,
    user_id: i32,
    amount: &str,
) -> i32 {
    let now = Utc::now();
    let amt = dec(amount);
    ap_invoice::ActiveModel {
        invoice_no: Set(format!("APW7-{}", unique_tag())),
        supplier_id: Set(supplier_id),
        invoice_type: Set("PURCHASE".to_string()),
        invoice_date: Set(today()),
        due_date: Set(today()),
        amount: Set(amt),
        paid_amount: Set(Decimal::ZERO),
        unpaid_amount: Set(amt),
        invoice_status: Set(ap_status::INVOICE_AUDITED.to_string()),
        created_by: Set(user_id),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
    .id
}

/// 直插已审批付款申请 + 单条明细（绕开分级审批权限检查——本测锁的是付款确认回写，
/// 申请审批链自身已有 wave1 契约测覆盖）
async fn seed_approved_request_with_item(
    db: &DatabaseConnection,
    supplier_id: i32,
    user_id: i32,
    invoice_id: i32,
    apply_amount: &str,
) -> i32 {
    let now = Utc::now();
    let request = ap_payment_request::ActiveModel {
        request_no: Set(format!("PRW7-{}", unique_tag())),
        request_date: Set(today()),
        supplier_id: Set(supplier_id),
        payment_type: Set("PROGRESS".to_string()),
        payment_method: Set("TT".to_string()),
        request_amount: Set(dec(apply_amount)),
        approval_status: Set(common::STATUS_APPROVED.to_string()),
        currency: Set("CNY".to_string()),
        exchange_rate: Set(Decimal::ONE),
        created_by: Set(user_id),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    ap_payment_request_item::ActiveModel {
        request_id: Set(request.id),
        invoice_id: Set(invoice_id),
        apply_amount: Set(dec(apply_amount)),
        notes: Set(Some("W7 契约测明细".to_string())),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    request.id
}

/// 直插已确认付款单（手工核销的资金来源行）
async fn seed_confirmed_payment(
    db: &DatabaseConnection,
    supplier_id: i32,
    user_id: i32,
    amount: &str,
) -> i32 {
    let now = Utc::now();
    ap_payment::ActiveModel {
        payment_no: Set(format!("PAYW7-{}", unique_tag())),
        payment_date: Set(today()),
        supplier_id: Set(supplier_id),
        request_id: Set(None),
        payment_method: Set("TT".to_string()),
        payment_amount: Set(dec(amount)),
        payment_status: Set(payment::PAYMENT_CONFIRMED.to_string()),
        currency: Set("CNY".to_string()),
        exchange_rate: Set(Decimal::ONE),
        transaction_no: Set(Some(format!("TXNW7-{}", unique_tag()))),
        created_by: Set(user_id),
        created_at: Set(now),
        updated_at: Set(now),
        confirmed_by: Set(Some(user_id)),
        confirmed_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
    .id
}

/// 建应收单：status=APPROVED、received=0、unpaid=amount
async fn seed_ar_invoice(
    db: &DatabaseConnection,
    customer_id: i32,
    user_id: i32,
    amount: &str,
) -> i32 {
    let now = Utc::now();
    let amt = dec(amount);
    ar_invoice::ActiveModel {
        invoice_no: Set(format!("ARW7-{}", unique_tag())),
        invoice_date: Set(today()),
        due_date: Set(today()),
        customer_id: Set(customer_id),
        invoice_amount: Set(amt),
        received_amount: Set(Decimal::ZERO),
        unpaid_amount: Set(amt),
        status: Set(common::STATUS_APPROVED.to_string()),
        approval_status: Set(common::STATUS_APPROVED.to_string()),
        created_by: Set(user_id),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
    .id
}

/// 建已确认收款（不带关联——模拟 11-02「收款先确认再核销」的真实链路形状）
async fn seed_confirmed_collection(
    db: &DatabaseConnection,
    customer_id: i32,
    user_id: i32,
    amount: &str,
) -> i32 {
    let now = Utc::now();
    ar_collection::ActiveModel {
        collection_no: Set(format!("COLW7-{}", unique_tag())),
        collection_date: Set(today()),
        customer_id: Set(customer_id),
        collection_amount: Set(dec(amount)),
        collection_method: Set(Some("银行转账".to_string())),
        status: Set(ar_status::COLLECTION_CONFIRMED.to_string()),
        confirmed_by: Set(Some(user_id)),
        confirmed_at: Set(Some(now)),
        created_by: Set(user_id),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
    .id
}

async fn reload_ap_invoice(db: &DatabaseConnection, id: i32) -> ap_invoice::Model {
    ap_invoice::Entity::find_by_id(id)
        .one(db)
        .await
        .unwrap()
        .unwrap_or_else(|| panic!("应付单 {id} 应存在"))
}

async fn reload_ar_invoice(db: &DatabaseConnection, id: i32) -> ar_invoice::Model {
    ar_invoice::Entity::find_by_id(id)
        .one(db)
        .await
        .unwrap()
        .unwrap_or_else(|| panic!("应收单 {id} 应存在"))
}

// =========================================================
// 活库（真 PostgreSQL）回写用例
// =========================================================

/// 【04-02 / 11-01 基线钉】付款确认后，应付单 paid/unpaid/状态必须**真实落库**：
/// paid 400、unpaid 600、PARTIAL_PAID、恒等 amount-paid=unpaid。
/// 防回潮面：若退回"Model 改字段 + `.into()`"（全 Unchanged）写法，UPDATE 不含
/// 金额列、paid 恒 0——本用例断 paid=400 即锁该契约面。
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 指向已跑完迁移的 PostgreSQL：confirm 链用 lock_exclusive + advisory_xact_lock"]
async fn ap_payment_confirm_writes_back_invoice_amounts() {
    let db = test_common::setup_test_db().await;
    let user_id = seed_user(&db, "w7_ap_confirm").await;
    let supplier_id = seed_supplier(&db).await;
    let invoice_id = seed_ap_invoice(&db, supplier_id, user_id, "1000.00").await;
    let request_id =
        seed_approved_request_with_item(&db, supplier_id, user_id, invoice_id, "400.00").await;

    let service = ApPaymentService::new(Arc::new(db.clone()));
    let payment = service
        .create(
            CreateApPaymentRequest {
                request_id,
                payment_date: today(),
                notes: None,
                attachment_urls: None,
            },
            user_id,
        )
        .await
        .expect("已审批申请应能创建付款单");

    service
        .update(
            payment.id,
            UpdateApPaymentRequest {
                payment_date: None,
                payment_method: None,
                bank_name: None,
                bank_account: None,
                transaction_no: Some(format!("TXN-W7-{}", unique_tag())),
                notes: None,
                attachment_urls: None,
            },
            user_id,
        )
        .await
        .expect("登记态付款单应能补交易流水号");

    service
        .confirm(payment.id, user_id)
        .await
        .expect("付款确认应成功");

    let inv = reload_ap_invoice(&db, invoice_id).await;
    assert_eq!(
        inv.paid_amount,
        dec("400.00"),
        "confirm 后 paid_amount 必须真实落库"
    );
    assert_eq!(
        inv.unpaid_amount,
        dec("600.00"),
        "confirm 后 unpaid_amount 必须真实回退"
    );
    assert_eq!(
        inv.invoice_status,
        payment::PAYMENT_PARTIAL_PAID,
        "部分付款状态必须落库"
    );
    assert_eq!(
        inv.amount - inv.paid_amount,
        inv.unpaid_amount,
        "会计恒等：amount-paid=unpaid"
    );
}

/// 【05-01 钉】未审批付款申请被拒：机器码仍是 BUSINESS_ERROR（族不变），
/// 出参 message 外显「审批」原因且不含内部状态 token（若用 `AppError::business`
/// 则被脱敏成固定常量、用户看不到拒绝原因）。
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 指向已跑完迁移的 PostgreSQL：create 查 ap_payment_request 真表"]
async fn ap_payment_create_unapproved_rejection_is_displayable_business_error() {
    let db = test_common::setup_test_db().await;
    let user_id = seed_user(&db, "w7_ap_unapproved").await;
    let supplier_id = seed_supplier(&db).await;
    // DRAFT 申请（无明细也要被拒——审批门在任何明细校验之前）
    let now = Utc::now();
    let request = ap_payment_request::ActiveModel {
        request_no: Set(format!("PRW7D-{}", unique_tag())),
        request_date: Set(today()),
        supplier_id: Set(supplier_id),
        payment_type: Set("PROGRESS".to_string()),
        payment_method: Set("TT".to_string()),
        request_amount: Set(dec("500.00")),
        // DRAFT 直接取写入方词表常量（common::STATUS_DRAFT 与本表写入点同源，
        // 见 ap_payment_request_service.rs build_payment_request_active_model）
        approval_status: Set(common::STATUS_DRAFT.to_string()),
        currency: Set("CNY".to_string()),
        exchange_rate: Set(Decimal::ONE),
        created_by: Set(user_id),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let service = ApPaymentService::new(Arc::new(db.clone()));
    let err: AppError = service
        .create(
            CreateApPaymentRequest {
                request_id: request.id,
                payment_date: today(),
                notes: None,
                attachment_urls: None,
            },
            user_id,
        )
        .await
        .expect_err("DRAFT 申请创建付款必须被拒");

    assert_eq!(
        err.error_code(),
        "BUSINESS_ERROR",
        "状态门控拒绝必须保持 BUSINESS_ERROR 族"
    );
    let resp = err.into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::BAD_REQUEST);
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let message = json["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("审批"),
        "出参文案必须外显拒绝原因（未审批不可付款），实际={message}"
    );
    assert!(
        !message.contains("DRAFT") && !message.contains("业务处理失败"),
        "文案不得回显内部状态 token，也不得仍是脱敏常量，实际={message}"
    );
}

/// 【11-01 手工核销 + cancel 钉】manual 后 B paid 增 unpaid 减至结清 PAID；
/// cancel 后金额**精确回退**（paid=0/unpaid=600/AUDITED）。
/// `process_verify_items`/`restore_invoices_on_cancel` 同属全 Unchanged 陷阱面：
/// 若回潮则核销只插明细、主表金额恒 0，cancel 也回退不出痕迹。
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 指向已跑完迁移的 PostgreSQL：manual/cancel 用 lock_exclusive + advisory_xact_lock"]
async fn ap_manual_verification_adds_and_cancel_rolls_back_exact_amounts() {
    let db = test_common::setup_test_db().await;
    let user_id = seed_user(&db, "w7_ap_manual").await;
    let supplier_id = seed_supplier(&db).await;
    let invoice_id = seed_ap_invoice(&db, supplier_id, user_id, "600.00").await;
    let payment_id = seed_confirmed_payment(&db, supplier_id, user_id, "600.00").await;

    let service = ApVerificationService::new(Arc::new(db.clone()));
    let verification = service
        .manual_verify(
            ManualVerifyRequest {
                supplier_id,
                items: vec![ApVerificationItemDto {
                    invoice_id,
                    payment_id,
                    verify_amount: dec("600.00"),
                    notes: Some("W7 契约测手工核销".to_string()),
                }],
                notes: None,
            },
            user_id,
        )
        .await
        .expect("手工核销应成功");

    let after = reload_ap_invoice(&db, invoice_id).await;
    assert_eq!(
        after.paid_amount,
        dec("600.00"),
        "manual 后 paid 必须累加落库"
    );
    assert_eq!(
        after.unpaid_amount,
        Decimal::ZERO,
        "manual 全额核销后 unpaid 必须归零"
    );
    assert_eq!(
        after.invoice_status,
        payment::PAYMENT_PAID,
        "结清状态必须落库"
    );

    service
        .cancel(verification.id, "W7 契约测回退".to_string(), user_id)
        .await
        .expect("取消核销应成功");

    let rolled = reload_ap_invoice(&db, invoice_id).await;
    assert_eq!(
        rolled.paid_amount,
        Decimal::ZERO,
        "cancel 后 paid 必须精确回退"
    );
    assert_eq!(
        rolled.unpaid_amount,
        dec("600.00"),
        "cancel 后 unpaid 必须精确恢复"
    );
    assert_eq!(
        rolled.invoice_status,
        ap_status::INVOICE_AUDITED,
        "paid=0 分支门控回 AUDITED"
    );
}

/// 【11-02 auto 钉】AR 自动核销后发票 received/unpaid/状态必须真实回写
/// （received=300、unpaid=700、PARTIAL_PAID）。若 `batch_update_invoice_states`
/// 退回全字段 Unchanged，UPDATE 只刷 updated_at，received 恒 0。
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 指向已跑完迁移的 PostgreSQL：auto_verify 全局贪心 + lock_exclusive"]
async fn ar_auto_verification_writes_back_invoice_amounts() {
    let db = test_common::setup_test_db().await;
    let user_id = seed_user(&db, "w7_ar_auto").await;
    let customer_id = seed_customer(&db, user_id).await;
    let invoice_id = seed_ar_invoice(&db, customer_id, user_id, "1000.00").await;
    let _collection_id = seed_confirmed_collection(&db, customer_id, user_id, "300.00").await;

    let service = ArService::new(Arc::new(db.clone()));
    service
        .auto_verify(user_id)
        .await
        .expect("存在已确认收款与未核销发票时自动核销应成功");

    let inv = reload_ar_invoice(&db, invoice_id).await;
    assert_eq!(
        inv.received_amount,
        dec("300.00"),
        "auto 核销后 received 必须累加落库"
    );
    assert_eq!(
        inv.unpaid_amount,
        dec("700.00"),
        "auto 核销后 unpaid 必须扣减"
    );
    assert_eq!(
        inv.status,
        payment::PAYMENT_PARTIAL_PAID,
        "部分核销状态必须落库"
    );
    assert_eq!(
        inv.invoice_amount - inv.received_amount,
        inv.unpaid_amount,
        "会计恒等：invoice-received=unpaid"
    );
}

/// 【11-02 manual + cancel 全链钉】两笔收款分别手工核销 300 + 700 → 结清 PAID；
/// 取消第二笔核销只回退那笔 700（received 回 300、状态回 PARTIAL_PAID），
/// 第一笔核销不受牵连——锁 `rollback_invoices` 显式 Set 与「按本单明细回退」的语义。
/// （刻意不用 auto 打底：auto_verify 全局贪心对同日期收款的消费顺序无 DB 保证，
/// 会引入不确定基线；auto 回写本身由上一用例独立钉死。）
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 指向已跑完迁移的 PostgreSQL：manual/cancel 用 lock_exclusive + advisory_xact_lock"]
async fn ar_manual_verify_settles_and_cancel_rolls_back_only_that_amount() {
    let db = test_common::setup_test_db().await;
    let user_id = seed_user(&db, "w7_ar_manual").await;
    let customer_id = seed_customer(&db, user_id).await;
    let invoice_id = seed_ar_invoice(&db, customer_id, user_id, "1000.00").await;
    let collection1 = seed_confirmed_collection(&db, customer_id, user_id, "300.00").await;
    let collection2 = seed_confirmed_collection(&db, customer_id, user_id, "700.00").await;

    let service = ArService::new(Arc::new(db.clone()));
    service
        .manual_verify(invoice_id, collection1, dec("300.00"), None, user_id)
        .await
        .expect("第一笔手工核销 300 应成功");
    let manual2 = service
        .manual_verify(invoice_id, collection2, dec("700.00"), None, user_id)
        .await
        .expect("第二笔手工核销 700 应成功");
    let manual2_id = manual2["id"]
        .as_i64()
        .expect("manual_verify 返回体应含核销单 id") as i32;

    let settled = reload_ar_invoice(&db, invoice_id).await;
    assert_eq!(
        settled.received_amount,
        dec("1000.00"),
        "两笔核销后 received 应达全额"
    );
    assert_eq!(settled.unpaid_amount, Decimal::ZERO, "结清后 unpaid 应归零");
    assert_eq!(settled.status, payment::PAYMENT_PAID, "结清状态应落库");

    service
        .cancel_verification(manual2_id, user_id)
        .await
        .expect("取消第二笔核销应成功");

    let rolled = reload_ar_invoice(&db, invoice_id).await;
    assert_eq!(
        rolled.received_amount,
        dec("300.00"),
        "cancel 只回退第二笔的 700"
    );
    assert_eq!(
        rolled.unpaid_amount,
        dec("700.00"),
        "cancel 后 unpaid 回 700"
    );
    assert_eq!(
        rolled.status,
        payment::PAYMENT_PARTIAL_PAID,
        "第一笔 300 仍在 → 状态回 PARTIAL_PAID"
    );
}

/// 【10-01 钉】试算平衡必须包含小写 'active' 的科目（写入方词表 master_data），
/// 且大写 'ACTIVE'（非写入值）不得混入——锁比较点与写入值逐字符同源。
/// 若过滤改用大写常量 → entries 恒空 → 「entries 应含借方科目」断言必失败。
/// 注意：account_subjects 是 sealed 参照表（夹具不清空），用例自建行、断言后自删。
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 指向已跑完迁移的 PostgreSQL：读取 account_subjects 真表"]
async fn trial_balance_filters_subjects_by_write_side_status_vocabulary() {
    let db = test_common::setup_test_db().await;
    let now = Utc::now();
    let tag = unique_tag();
    let active_code = format!("W7A{tag}");
    let legacy_code = format!("W7B{tag}");

    let mk = |code: &str, status: &str| account_subject::ActiveModel {
        code: Set(code.to_string()),
        name: Set(format!("W7 回写契约科目 {code}")),
        level: Set(1),
        // 余额方向取写入方词表（models/status/finance.rs account_subject::DIRECTION_*，小写）
        balance_direction: Set(Some(subject_status::DIRECTION_DEBIT.to_string())),
        initial_balance_debit: Set(Decimal::ZERO),
        initial_balance_credit: Set(Decimal::ZERO),
        current_period_debit: Set(dec("1000.00")),
        current_period_credit: Set(Decimal::ZERO),
        ending_balance_debit: Set(dec("1000.00")),
        ending_balance_credit: Set(Decimal::ZERO),
        assist_customer: Set(false),
        assist_supplier: Set(false),
        assist_department: Set(false),
        assist_employee: Set(false),
        assist_project: Set(false),
        assist_batch: Set(false),
        assist_color_no: Set(false),
        assist_dye_lot: Set(false),
        assist_grade: Set(false),
        assist_workshop: Set(false),
        enable_dual_unit: Set(false),
        is_cash_account: Set(false),
        is_bank_account: Set(false),
        allow_manual_entry: Set(true),
        require_summary: Set(false),
        status: Set(status.to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    let active_row = mk(&active_code, master_data::ACTIVE)
        .insert(&db)
        .await
        .unwrap();
    // 大写 'ACTIVE' 不属于写入方词表（common::STATUS_ACTIVE 与大写主数据状态是两族），
    // 插入它是为了锁「过滤条件不会把非写入值也吞进来」的反向口径。
    let legacy_row = mk(&legacy_code, "ACTIVE").insert(&db).await.unwrap();

    let result = FinanceReportService::new(Arc::new(db.clone()))
        .get_trial_balance(None)
        .await
        .expect("试算平衡查询应成功");

    let codes: Vec<&str> = result
        .entries
        .iter()
        .map(|e| e.subject_code.as_str())
        .collect();
    let contain_active = codes.contains(&active_code.as_str());
    let contain_legacy = codes.contains(&legacy_code.as_str());

    // 无论断言结果如何都先清理自建行（sealed 表不参与夹具 truncate，绝不留残数据）
    account_subject::Entity::delete_by_id(active_row.id)
        .exec(&db)
        .await
        .unwrap();
    account_subject::Entity::delete_by_id(legacy_row.id)
        .exec(&db)
        .await
        .unwrap();

    assert!(
        contain_active,
        "写入方词表 master_data::ACTIVE（小写）的科目必须进试算平衡 entries（修复前恒空）"
    );
    assert!(
        !contain_legacy,
        "大写 ACTIVE 不是 account_subjects.status 的写入值，不得被本比较点放行"
    );
}
