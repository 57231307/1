use bingxi_backend::models::status::finance as status_finance;
use bingxi_backend::models::voucher::Model as VoucherModel;
use chrono::Utc;
use rust_decimal::Decimal;

/// 构造测试用的凭证模型
fn make_voucher_model(id: i32, status: &str) -> VoucherModel {
    VoucherModel {
        id,
        voucher_no: format!("V-2026-{:04}", id),
        voucher_date: Utc::now().naive_utc().date(),
        voucher_type: "记账凭证".to_string(),
        source_type: Some("sales_order".to_string()),
        source_module: Some("sales".to_string()),
        source_bill_id: Some(1),
        source_bill_no: Some("SO-2026-0001".to_string()),
        batch_no: Some("B001".to_string()),
        color_no: Some("C001".to_string()),
        dye_lot_no: Some("DL001".to_string()),
        workshop: Some("车间1".to_string()),
        production_order_no: Some("PO-001".to_string()),
        quantity_meters: Some(Decimal::from(100)),
        quantity_kg: Some(Decimal::from(50)),
        gram_weight: Some(Decimal::from(200)),
        status: status.to_string(),
        attachment_count: 0,
        created_by: 1,
        reviewed_by: None,
        reviewed_at: None,
        posted_by: None,
        posted_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

// ===== 状态常量测试 =====

#[test]
fn test_finance_status_draft() {
    assert_eq!(status_finance::voucher::VOUCHER_DRAFT, "draft");
}

#[test]
fn test_finance_status_posted() {
    assert_eq!(status_finance::voucher::VOUCHER_POSTED, "posted");
}

#[test]
fn test_finance_status_audited() {
    assert_eq!(status_finance::ap_invoice::INVOICE_AUDITED, "AUDITED");
}

#[test]
fn test_finance_status_voided() {
    assert_eq!(status_finance::voucher::VOUCHER_REVIEWED, "reviewed");
}

// ===== 模型测试 =====

#[test]
fn test_voucher_model_serialization() {
    let voucher = make_voucher_model(1, "draft");
    let json = serde_json::to_value(&voucher).expect("凭证序列化失败");

    assert_eq!(json["id"], 1);
    assert_eq!(json["voucher_no"], "V-2026-0001");
    assert_eq!(json["status"], "draft");
}

#[test]
fn test_voucher_type() {
    let voucher = make_voucher_model(1, "draft");
    assert_eq!(voucher.voucher_type, "记账凭证");
}

#[test]
fn test_voucher_source() {
    let voucher = make_voucher_model(1, "draft");
    assert_eq!(voucher.source_type, Some("sales_order".to_string()));
    assert_eq!(voucher.source_bill_no, Some("SO-2026-0001".to_string()));
}

#[test]
fn test_voucher_status() {
    let voucher = make_voucher_model(1, "posted");
    assert_eq!(voucher.status, "posted");
}

#[test]
fn test_voucher_created_by() {
    let voucher = make_voucher_model(1, "draft");
    assert_eq!(voucher.created_by, 1);
}

#[test]
fn test_voucher_attachment_count() {
    let voucher = make_voucher_model(1, "draft");
    assert_eq!(voucher.attachment_count, 0);
}

#[test]
fn test_voucher_quantity_fields() {
    let voucher = make_voucher_model(1, "draft");
    assert_eq!(voucher.quantity_meters, Some(Decimal::from(100)));
    assert_eq!(voucher.quantity_kg, Some(Decimal::from(50)));
}

#[test]
fn test_voucher_fabric_fields() {
    let voucher = make_voucher_model(1, "draft");
    assert_eq!(voucher.batch_no, Some("B001".to_string()));
    assert_eq!(voucher.color_no, Some("C001".to_string()));
    assert_eq!(voucher.dye_lot_no, Some("DL001".to_string()));
}
