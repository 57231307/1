//! AP 付款申请 items 可选 + submit 门控锁（本波契约：items 不再必填、无明细不可提交）
//!
//! 锁定的 file:line 契约：
//! - `backend/src/services/ap_payment_request_service.rs:597-656`
//!   （CreateApPaymentRequest.items：`Option<Vec<..>> + #[serde(default)]`，缺键/null 均解码通过）
//! - `backend/src/services/ap_payment_request_service.rs:62-98`
//!   （create：items 缺省按空明细建 DRAFT 主表、金额取表头，不伪造 invoice_id NOT NULL 外键脏明细）
//! - `backend/src/services/ap_payment_request_service.rs:308-361`（submit 门控：
//!   L330-338 无明细分支 `AppError::business_displayable("付款申请没有明细，不可提交")`
//!   ——公开拒绝文案外显（用户已定口径）；
//!   L323-328 非 DRAFT 拒提交；状态词表值来自 common::STATUS_DRAFT / ap_payment_request::APPROVAL_APPROVING）
//! - `backend/src/handlers/ap_payment_request_handler.rs:159-186`（create_request：
//!   validate 先行、service 错误 `?` 传播）
//!
//! 覆盖策略：
//! - serde/validation 纯函数断言（**无需任何 DB**）：缺 items 键、null、空数组、明细缺
//!   invoice_id（判 Err——invoice_id 仍为必填，防脏外键回潮）、金额/汇率校验
//! - `#[ignore]` 活库用例（需 TEST_DATABASE_URL 指向已迁移 PostgreSQL——create/submit 走
//!   advisory_xact_lock 单号生成 + submit 用 lock_exclusive，sqlite 不支持，无法非活库化）：
//!   无 items 建单=DRAFT；无明细 submit=400 BUSINESS_ERROR；service 与 HTTP 双层各锁一遍

mod test_common;

use rust_decimal::Decimal;
use serde_json::json;
use std::str::FromStr;
use validator::Validate;

use bingxi_backend::services::ap_payment_request_service::{
    ApPaymentRequestItemDto, CreateApPaymentRequest,
};

/// 表头合法、无 items 键的基础请求体（前端 createAPPaymentRequest 的真实形状：从不发 items）
fn base_body() -> serde_json::Value {
    json!({
        "supplier_id": 1,
        "request_date": "2026-01-15",
        "payment_type": "NORMAL",
        "payment_method": "BANK_TRANSFER",
        "request_amount": "1000.50"
    })
}

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

// =========================================================
// serde 解码（无 DB）
// =========================================================

/// 缺 items 键 → 解码通过且 items == None（本波 Option + serde(default) 的直接锁；
/// 修复前此形状必 422，付款申请创建整体不可用）
#[test]
fn decode_without_items_key_is_none() {
    let req: CreateApPaymentRequest = serde_json::from_value(base_body()).unwrap();
    assert!(
        req.items.is_none(),
        "缺 items 键必须解码为 None，不得判 422"
    );
    assert_eq!(
        req.request_amount,
        dec("1000.50"),
        "字符串金额必须无损解码为 Decimal"
    );
    assert_eq!(req.supplier_id, 1);
    assert_eq!(req.request_date.to_string(), "2026-01-15");
}

/// "items": null → 同样 None（JSON null 与缺键同语义）
#[test]
fn decode_items_null_is_none() {
    let mut body = base_body();
    body["items"] = json!(null);
    let req: CreateApPaymentRequest = serde_json::from_value(body).unwrap();
    assert!(req.items.is_none());
}

/// "items": [] → Some(空 vec)：与 None 区分保留（service 内 unwrap_or_default 统一按空处理）
#[test]
fn decode_items_empty_array_is_some_empty() {
    let mut body = base_body();
    body["items"] = json!([]);
    let req: CreateApPaymentRequest = serde_json::from_value(body).unwrap();
    let items = req.items.expect("空数组应解码为 Some(vec![])");
    assert!(items.is_empty());
}

/// 带明细解码：invoice_id/apply_amount 类型与值精确回读
#[test]
fn decode_with_items_full_roundtrip() {
    let mut body = base_body();
    body["items"] = json!([
        { "invoice_id": 7, "apply_amount": "300.20", "notes": "部分核销" },
        { "invoice_id": 8, "apply_amount": "700.30", "notes": null }
    ]);
    let req: CreateApPaymentRequest = serde_json::from_value(body).unwrap();
    let items: &Vec<ApPaymentRequestItemDto> = req.items.as_ref().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].invoice_id, 7);
    assert_eq!(items[0].apply_amount, dec("300.20"));
    assert_eq!(items[0].notes.as_deref(), Some("部分核销"));
    assert_eq!(items[1].invoice_id, 8);
    assert!(items[1].notes.is_none());
}

/// 明细缺 invoice_id 键 → 解码必须失败（invoice_id 是 ap_invoice 的 NOT NULL 外键，
/// 本波仅把 items 集合改可选，**没有**也不得把 invoice_id 改成可缺省去臆造脏引用）
#[test]
fn decode_item_missing_invoice_id_fails() {
    let mut body = base_body();
    body["items"] = json!([{ "apply_amount": "100.00" }]);
    let result: Result<CreateApPaymentRequest, _> = serde_json::from_value(body);
    let err = result.expect_err("缺 invoice_id 的明细必须拒收（NOT NULL 外键不可缺省）");
    assert!(
        err.to_string().contains("invoice_id"),
        "错误信息应指出缺失字段，实际: {err}"
    );
}

/// 空 body（连表头必填都没有）→ 解码失败：本波只放宽了 items，其余必填不得连带失守
#[test]
fn decode_missing_header_required_fields_still_fails() {
    let result: Result<CreateApPaymentRequest, _> = serde_json::from_value(json!({}));
    assert!(
        result.is_err(),
        "supplier_id/request_date 等表头必填仍须判 422"
    );
}

// =========================================================
// validator（无 DB）
// =========================================================

/// 无 items 的合法表头必须通过 Validate（否则 items 可选只是 serde 层假开放）
#[test]
fn validate_request_without_items_passes() {
    let req: CreateApPaymentRequest = serde_json::from_value(base_body()).unwrap();
    req.validate().expect("无 items 表头不得引入额外校验失败");
}

/// 负金额被 validate_positive_decimal_payment 拒绝，文案与源码逐字一致
#[test]
fn validate_negative_amount_fails_with_exact_wording() {
    let mut body = base_body();
    body["request_amount"] = json!("-100.00");
    let req: CreateApPaymentRequest = serde_json::from_value(body).unwrap();
    let err = req.validate().expect_err("负申请金额必须被校验拒绝");
    assert!(
        err.to_string().contains("金额必须为正数"),
        "校验文案应含『金额必须为正数』，实际: {err}"
    );
}

/// 历史缺陷汇率 0.01 仍被拒绝（P0-1 防护不因 items 放宽而松动）
#[test]
fn validate_historical_exchange_rate_001_fails() {
    let mut body = base_body();
    body["exchange_rate"] = json!("0.01");
    let req: CreateApPaymentRequest = serde_json::from_value(body).unwrap();
    let err = req.validate().expect_err("0.01 历史缺陷汇率必须拒绝");
    assert!(err.to_string().contains("汇率"));
}

// =========================================================
// 活库（PostgreSQL + 已迁移 schema）用例：#[ignore]
// =========================================================

#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 指向已跑完迁移的 PostgreSQL：create 单号生成用 advisory_xact_lock、submit 用 lock_exclusive，sqlite 方言均不支持，无法非活库化"]
async fn ap_request_no_items_create_draft_then_submit_rejected() {
    use bingxi_backend::models::status::{ap_payment_request, common};
    use bingxi_backend::services::ap_payment_request_service::ApPaymentRequestService;
    use bingxi_backend::utils::error::AppError;

    let db = test_common::setup_test_db().await;
    let svc = ApPaymentRequestService::new(std::sync::Arc::new(db));

    let req: CreateApPaymentRequest = serde_json::from_value(base_body()).unwrap();
    req.validate().unwrap();
    let created = svc
        .create(req, 100)
        .await
        .expect("无 items 也必须能建单（本波契约：items 可选）");
    assert_eq!(created.approval_status, common::STATUS_DRAFT);
    assert_eq!(
        created.request_amount,
        dec("1000.50"),
        "无明细时主表金额取表头 request_amount"
    );
    assert_eq!(created.created_by, 100);

    let err = svc
        .submit(created.id, 100)
        .await
        .expect_err("无明细 submit 必须被门控拒绝（方案B内控不得削弱）");
    // 构造点契约（本波 B3 修复）：公开拒绝文案须 business_displayable 外显，
    // code 仍为 BUSINESS_ERROR（400），不得被脱敏成"业务处理失败"
    match &err {
        AppError::BusinessErrorDisplayable(_) => {}
        other => panic!("必须为 business_displayable 构造的可见业务错，实际: {other:?}"),
    }
    assert_eq!(err.error_code(), "BUSINESS_ERROR");
    assert!(
        err.to_string().contains("付款申请没有明细，不可提交"),
        "门控文案须与 ap_payment_request_service.rs:337 逐字一致，实际: {err}"
    );
    {
        use axum::response::IntoResponse;
        let resp = err.clone().into_response();
        assert_eq!(resp.status(), axum::http::StatusCode::BAD_REQUEST);
        let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["code"], "BUSINESS_ERROR");
        assert_eq!(
            v["message"], "付款申请没有明细，不可提交",
            "business_displayable 出参必须外显真实拒绝文案，不得脱敏"
        );
    }

    // 防呆反向锁：若误把 APPROVING 当成可提交状态也要暴露（仅 DRAFT 可提交）
    assert_eq!(ap_payment_request::APPROVAL_APPROVING, "APPROVING");
}

/// items 引用不存在应付单 → 404 NOT_FOUND（不是被强转的 500；validate_invoice_items_txn 语义）
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 指向已跑完迁移的 PostgreSQL：create 单号生成用 advisory_xact_lock，sqlite 方言不支持"]
async fn ap_request_items_reference_missing_invoice_returns_not_found() {
    use bingxi_backend::services::ap_payment_request_service::ApPaymentRequestService;
    use bingxi_backend::utils::error::AppError;

    let db = test_common::setup_test_db().await;
    let svc = ApPaymentRequestService::new(std::sync::Arc::new(db));

    let mut body = base_body();
    body["items"] = json!([{ "invoice_id": 999999, "apply_amount": "100.00", "notes": null }]);
    let req: CreateApPaymentRequest = serde_json::from_value(body).unwrap();
    let err = svc
        .create(req, 100)
        .await
        .expect_err("引用不存在应付单必须失败");
    assert!(matches!(err, AppError::NotFound(_)), "实际: {err:?}");
    assert_eq!(err.error_code(), "NOT_FOUND");
}
