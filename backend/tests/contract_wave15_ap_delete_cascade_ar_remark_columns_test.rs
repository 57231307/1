//! 财务链活体契约锁（连真库 PostgreSQL）。
//!
//! 锁定两条源码契约面（均走 `test_common::setup_test_db()` 真库 + 真实回读）：
//!
//! 1. **AP 付款申请删除级联清子表明细**：
//!    - 子表外键 `ap_payment_request_item.request_id` → `ap_payment_request.id`
//!      为 NO ACTION、库侧不级联（见 `m0012_add_ap_ar_finance_analysis` 中
//!      `fk_ap_payment_request_item_request`）；
//!    - 删除须在同事务内先按 request_id 清子表再删主表，否则触发外键违例；
//!    - 本锁断 DRAFT 申请（带一条明细）删除后主表/子表均无残留。
//!    调用方：`ApPaymentRequestService::delete`。
//!
//! 2. **AR 收款 remark 独立列、check_no 不被 remark 污染**：
//!    - `models/ar_collection` 同时有 `remark` 与 `check_no` 两列；
//!    - `ar_ops::collection::build_collection_active_model` 只 Set(remark)，不写 check_no；
//!    - `ar_ops::json_helpers::collection_to_json` 两键分列输出；
//!    - 提交 remark 后回读 `remark` 等于提交值、`check_no` 为 null。
//!    调用方：`ArService::create_payment` / `ArService::update_payment`。

mod test_common;

use chrono::{NaiveDate, TimeZone, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::str::FromStr;
use std::sync::Arc;

use bingxi_backend::models::status::{
    accounting_period as period_status, ap_invoice as ap_status, common,
};
use bingxi_backend::models::{
    accounting_period, ap_invoice, ap_payment_request, ap_payment_request_item, customer, supplier,
    user,
};
use bingxi_backend::services::ap_payment_request_service::ApPaymentRequestService;
use bingxi_backend::services::ar_service::{ArService, CreateArPaymentParams};

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn unique_tag() -> i64 {
    Utc::now()
        .timestamp_nanos_opt()
        .expect("Utc::now 纳秒必落在 chrono 可表示区间，None 不可达")
}

fn fixture_date() -> NaiveDate {
    // 落在下方 seed 的 2026-04 开放期间内
    NaiveDate::from_ymd_opt(2026, 4, 10).unwrap()
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
        supplier_code: Set(format!("SUP-W15-{}", unique_tag())),
        supplier_name: Set("W15 契约测试供应商".to_string()),
        supplier_short_name: Set("W15供".to_string()),
        supplier_type: Set("面料供应商".to_string()),
        credit_code: Set("91330000W15T00000X1".to_string()),
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
        customer_code: Set(format!("CUS-W15-{}", unique_tag())),
        customer_name: Set("W15 契约测试客户".to_string()),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
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

async fn seed_open_period_2026_04(db: &DatabaseConnection) {
    let start = NaiveDate::from_ymd_opt(2026, 4, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let end = NaiveDate::from_ymd_opt(2026, 4, 30)
        .unwrap()
        .and_hms_opt(23, 59, 59)
        .unwrap();
    accounting_period::ActiveModel {
        year: Set(2026),
        period: Set(4),
        period_name: Set("2026-04".to_string()),
        start_date: Set(Utc.from_utc_datetime(&start)),
        end_date: Set(Utc.from_utc_datetime(&end)),
        status: Set(period_status::OPEN.to_string()),
        created_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

async fn seed_ap_invoice(db: &DatabaseConnection, supplier_id: i32, user_id: i32) -> i32 {
    let now = Utc::now();
    let amt = dec("1000.00");
    ap_invoice::ActiveModel {
        invoice_no: Set(format!("APW15-{}", unique_tag())),
        supplier_id: Set(supplier_id),
        invoice_type: Set("PURCHASE".to_string()),
        invoice_date: Set(fixture_date()),
        due_date: Set(fixture_date()),
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

/// 直插 DRAFT 付款申请 + 一条明细（真实外键父行齐备；状态取写入方词表 common::STATUS_DRAFT，
/// 与 delete 门控同源，命中「仅草稿/被拒可删」分支）。
async fn seed_draft_request_with_item(
    db: &DatabaseConnection,
    supplier_id: i32,
    user_id: i32,
    invoice_id: i32,
) -> i32 {
    let now = Utc::now();
    let request = ap_payment_request::ActiveModel {
        request_no: Set(format!("PRW15-{}", unique_tag())),
        request_date: Set(fixture_date()),
        supplier_id: Set(supplier_id),
        payment_type: Set("PROGRESS".to_string()),
        payment_method: Set("TT".to_string()),
        request_amount: Set(dec("1000.00")),
        approval_status: Set(common::STATUS_DRAFT.to_string()),
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
        apply_amount: Set(dec("1000.00")),
        notes: Set(Some("W15 删除级联明细".to_string())),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    request.id
}

// =========================================================
// 活体锁 1：AP 删除级联清子表
// =========================================================

#[tokio::test]
async fn ap_request_delete_cascades_items_no_database_error() {
    let db = test_common::setup_test_db().await;
    let uid = seed_user(&db, "w15_ap_del").await;
    let supplier_id = seed_supplier(&db).await;
    let invoice_id = seed_ap_invoice(&db, supplier_id, uid).await;
    let request_id = seed_draft_request_with_item(&db, supplier_id, uid, invoice_id).await;

    // 前置确认：明细确实存在（若不存在则删除不会命中外键违例，锁失去意义）
    let items_before = ap_payment_request_item::Entity::find()
        .filter(ap_payment_request_item::Column::RequestId.eq(request_id))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(
        items_before.len(),
        1,
        "删除前应恰有一条子表明细，否则该锁无法证明级联"
    );

    let svc = ApPaymentRequestService::new(Arc::new(db.clone()));
    // 若仅删主表而残留明细，Postgres 外键违例 → 裸 DbErr → DATABASE_ERROR(500)，
    // expect 判红；级联清子表后删除成功。
    svc.delete(request_id, uid)
        .await
        .expect("带明细的草稿付款申请删除必须成功（修复前报 DATABASE_ERROR）");

    // 删除后主表与子表均无残留：真实回读，非弱断言
    assert!(
        ap_payment_request::Entity::find_by_id(request_id)
            .one(&db)
            .await
            .unwrap()
            .is_none(),
        "主表付款申请必须已删除"
    );
    let items_after = ap_payment_request_item::Entity::find()
        .filter(ap_payment_request_item::Column::RequestId.eq(request_id))
        .all(&db)
        .await
        .unwrap();
    assert!(
        items_after.is_empty(),
        "子表明细必须被级联删除，不得残留（残留即外键违例根因）"
    );
}

// =========================================================
// 活体锁 2：AR 收款 remark 独立列、check_no 不被污染
// =========================================================

#[tokio::test]
async fn ar_collection_remark_persists_independently_not_polluting_check_no() {
    let db = test_common::setup_test_db().await;
    seed_open_period_2026_04(&db).await;
    let uid = seed_user(&db, "w15_ar_remark").await;
    let cid = seed_customer(&db, uid).await;

    let service = ArService::new(Arc::new(db.clone()));
    let created = service
        .create_payment(
            CreateArPaymentParams {
                customer_id: cid,
                amount: dec("2500.75"),
                payment_method: "银行汇款".to_string(),
                payment_date: fixture_date(),
                bank_account: None,
                remark: Some("E2E-收款-建单".to_string()),
                invoice_ids: None,
            },
            uid,
        )
        .await
        .expect("创建收款应成功");
    let payment_id = created["id"].as_i64().expect("响应应含收款单 id") as i32;

    // 真实回读（GET 详情走 collection_to_json 同一序列化路径）
    let detail = service
        .get_payment(payment_id, None)
        .await
        .expect("回读收款详情应成功");

    // 正确契约：提交值落 remark 列、原样回读
    assert!(
        detail.get("remark").is_some(),
        "出参必须含真实键 remark，实际键={:?}",
        detail.as_object().map(|m| m.keys().collect::<Vec<_>>())
    );
    assert_eq!(
        detail["remark"].as_str(),
        Some("E2E-收款-建单"),
        "remark 必须回读为提交值（S-2 独立列契约）"
    );
    // 契约面：check_no 绝不被 remark 污染——创建路径不写 check_no，回读必为 null。
    // 若源码回潮把 remark 塞进 check_no，本断言判红。
    assert!(
        detail.get("check_no").is_some(),
        "出参应含真实键 check_no（即便为 null），实际键={:?}",
        detail.as_object().map(|m| m.keys().collect::<Vec<_>>())
    );
    assert!(
        detail["check_no"].is_null(),
        "check_no 不得被 remark 污染：创建路径未写 check_no，回读必为 null，实际={:?}",
        detail["check_no"]
    );

    // 三态更新：仅覆盖 remark，check_no 仍独立不被牵连
    service
        .update_payment(
            payment_id,
            serde_json::json!({ "remark": "E2E-收款-改后" }),
            uid,
        )
        .await
        .expect("pending 收款改 remark 应成功");
    let after = service
        .get_payment(payment_id, None)
        .await
        .expect("回读收款详情应成功");
    assert_eq!(
        after["remark"].as_str(),
        Some("E2E-收款-改后"),
        "remark 更新后应回读新值"
    );
    assert!(
        after["check_no"].is_null(),
        "改 remark 不得写入 check_no（两列互不覆盖），实际={:?}",
        after["check_no"]
    );
}
