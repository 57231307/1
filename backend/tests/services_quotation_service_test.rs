use bingxi_backend::decs;
use bingxi_backend::models::quotation_create_dto::{CreateQuotationDto, CreateQuotationItemDto};
use bingxi_backend::models::quotation_update_dto::UpdateQuotationDto;
use bingxi_backend::models::status::quotation as quotation_status;
use bingxi_backend::services::quotation_service::{QuotationService, ServiceError};
use bingxi_backend::services::test_common::setup_test_db;
use bingxi_backend::utils::error::AppError;
use bingxi_backend::ymd;
use rust_decimal::Decimal;
use sea_orm::ConnectionTrait;
use std::sync::Arc;

/// 构造合法的 CreateQuotationItemDto（单条明细）
fn sample_item() -> CreateQuotationItemDto {
    CreateQuotationItemDto {
        product_id: 1001,
        color_id: Some(2001),
        specification: Some("规格 A".to_string()),
        unit: "M".to_string(),
        quantity: decs!(100),
        unit_price: decs!(10),
        unit_price_with_tax: decs!(11.3),
        tier_pricing: None,
        discount_rate: None,
        notes: None,
    }
}

/// 构造合法的 CreateQuotationDto（默认 FOB + 不含税 + 13% 税率）
fn sample_dto() -> CreateQuotationDto {
    CreateQuotationDto {
        customer_id: 1,
        sales_user_id: 10,
        quotation_date: ymd!(2026, 7, 19),
        valid_until: ymd!(2026, 8, 19),
        currency: "CNY".to_string(),
        exchange_rate: Decimal::ONE,
        base_currency: "CNY".to_string(),
        price_terms: "FOB".to_string(),
        incoterms_version: Some("2020".to_string()),
        incoterm_location: Some("Shanghai".to_string()),
        tax_inclusive: false,
        tax_rate: decs!(13),
        moq: Some(decs!(50)),
        lead_time_days: Some(30),
        customer_level: Some("A".to_string()),
        notes: Some("测试报价单".to_string()),
        items: vec![sample_item()],
        terms: None,
    }
}

// ============ ServiceError 枚举值正确性测试 ============

/// test_serviceerror_display_gszq
/// 族镜像锁：报价域 `ServiceError::InvalidState` 是「前置状态未满足」状态门，
/// handler 的 `From<ServiceError> for AppError` 装配点必须出 BUSINESS_ERROR 且外显真实文案
/// （修复前出 VALIDATION_ERROR，前端按 code 分支时把业务拒绝当"我填错了"）。
/// 断言跟随源码现状：`AppError::business_displayable("当前状态不允许此操作")`。
#[test]
fn test_serviceerror_invalidstate_maps_to_displayable_business() {
    let err = AppError::from(ServiceError::InvalidState);
    assert!(
        matches!(err, AppError::BusinessErrorDisplayable(_)),
        "状态门必须归 business 族且可外显，实际={err:?}"
    );
    assert_eq!(err.error_code(), "BUSINESS_ERROR");
    assert_eq!(err.to_response().message, "当前状态不允许此操作");

    // 反向对照：提交字段校验仍归校验族，证明未一刀切
    let v = AppError::from(ServiceError::Validation("明细至少 1 条".to_string()));
    assert!(matches!(v, AppError::ValidationErrorDisplayable(_)));
    assert_eq!(v.error_code(), "VALIDATION_ERROR");
}

/// 验证 5 个 ServiceError 变体的 Display 实现返回中文错误信息
#[test]
fn test_serviceerror_display_gszq() {
    assert_eq!(ServiceError::NotFound.to_string(), "报价单不存在");
    assert_eq!(
        ServiceError::InvalidState.to_string(),
        "当前状态不允许此操作"
    );
    assert_eq!(
        ServiceError::Validation("明细至少 1 条".to_string()).to_string(),
        "参数校验失败: 明细至少 1 条"
    );
    let db_err = ServiceError::Database(sea_orm::DbErr::RecordNotFound("test".to_string()));
    assert!(db_err.to_string().starts_with("数据库错误:"));
}

// ============ validate_create 业务校验测试 ============

/// test_validate_create_kmxjj
#[tokio::test]
async fn test_validate_create_kmxjj() {
    let db = setup_test_db().await;
    let svc = QuotationService::new(Arc::new(db));
    let mut dto = sample_dto();
    dto.items.clear();
    let result = svc.validate_create(&dto);
    assert!(matches!(result, Err(ServiceError::Validation(_))));
    if let Err(ServiceError::Validation(msg)) = result {
        assert!(msg.contains("明细至少 1 条"));
    }
}

/// test_validate_create_yxqzybjrqjj
#[tokio::test]
async fn test_validate_create_yxqzybjrqjj() {
    let db = setup_test_db().await;
    let svc = QuotationService::new(Arc::new(db));
    let mut dto = sample_dto();
    dto.valid_until = ymd!(2026, 6, 19);
    dto.quotation_date = ymd!(2026, 7, 19);
    let result = svc.validate_create(&dto);
    assert!(matches!(result, Err(ServiceError::Validation(_))));
    if let Err(ServiceError::Validation(msg)) = result {
        assert!(msg.contains("有效期截止必须不早于报价日期"));
    }
}

/// test_validate_create_ffmysyjj
#[tokio::test]
async fn test_validate_create_ffmysyjj() {
    let db = setup_test_db().await;
    let svc = QuotationService::new(Arc::new(db));
    let mut dto = sample_dto();
    dto.price_terms = "XYZ".to_string();
    let result = svc.validate_create(&dto);
    assert!(matches!(result, Err(ServiceError::Validation(_))));
    if let Err(ServiceError::Validation(msg)) = result {
        assert!(msg.contains("FOB") || msg.contains("合法取值"));
    }
}

/// test_validate_create_hfcstg
#[tokio::test]
async fn test_validate_create_hfcstg() {
    let db = setup_test_db().await;
    let svc = QuotationService::new(Arc::new(db));
    let dto = sample_dto();
    let result = svc.validate_create(&dto);
    assert!(result.is_ok());
}

// ============ calculate_totals 金额计算测试 ============

/// test_calculate_totals_bhsjejszq
#[tokio::test]
async fn test_calculate_totals_bhsjejszq() {
    let db = setup_test_db().await;
    let svc = QuotationService::new(Arc::new(db));
    let dto = sample_dto();
    let (subtotal, tax_amount, total_amount) = svc.calculate_totals(&dto).unwrap();
    assert_eq!(subtotal, decs!(1000));
    assert_eq!(tax_amount, decs!(130));
    assert_eq!(total_amount, decs!(1130));
}

/// test_calculate_totals_hsjesewl
#[tokio::test]
async fn test_calculate_totals_hsjesewl() {
    let db = setup_test_db().await;
    let svc = QuotationService::new(Arc::new(db));
    let mut dto = sample_dto();
    dto.tax_inclusive = true;
    let (subtotal, tax_amount, total_amount) = svc.calculate_totals(&dto).unwrap();
    assert_eq!(subtotal, decs!(1000));
    assert_eq!(tax_amount, Decimal::ZERO);
    assert_eq!(total_amount, decs!(1000));
}

/// test_calculate_totals_dmxhzzq
#[tokio::test]
async fn test_calculate_totals_dmxhzzq() {
    let db = setup_test_db().await;
    let svc = QuotationService::new(Arc::new(db));
    let mut dto = sample_dto();
    dto.items.push(CreateQuotationItemDto {
        product_id: 1002,
        color_id: None,
        specification: None,
        unit: "M".to_string(),
        quantity: decs!(200),
        unit_price: decs!(20),
        unit_price_with_tax: decs!(22.6),
        tier_pricing: None,
        discount_rate: None,
        notes: None,
    });
    let (subtotal, _, _) = svc.calculate_totals(&dto).unwrap();
    assert_eq!(subtotal, decs!(5000));
}

/// test_calculate_totals_jdgyd2wxs
/// 小数量×单价的尾差口径：33.333 × 3 = 99.999，小计按 round_dp(2) 舍入 ⇒ 100.00。
#[tokio::test]
async fn test_calculate_totals_jdgyd2wxs() {
    let db = setup_test_db().await;
    let svc = QuotationService::new(Arc::new(db));
    let mut dto = sample_dto();
    dto.items = vec![CreateQuotationItemDto {
        product_id: 1,
        color_id: None,
        specification: None,
        unit: "M".to_string(),
        quantity: decs!(3),
        unit_price: decs!(33.333),
        unit_price_with_tax: decs!(33.333),
        tier_pricing: None,
        discount_rate: None,
        notes: None,
    }];
    let (subtotal, _, _) = svc.calculate_totals(&dto).unwrap();
    assert_eq!(subtotal, decs!(100));
}

// ============ validate_price_terms 贸易术语校验测试 ============

/// test_validate_price_terms_hfdmfhmj
#[test]
fn test_validate_price_terms_hfdmfhmj() {
    let valid_codes = [
        "EXW", "FCA", "CPT", "CIP", "DAP", "DPU", "DDP", "FAS", "FOB", "CFR", "CIF",
    ];
    for code in valid_codes {
        let result = QuotationService::validate_price_terms(code);
        assert!(result.is_ok(), "合法代码 {} 应通过校验", code);
    }
}

/// test_validate_price_terms_dxxbmg
#[test]
fn test_validate_price_terms_dxxbmg() {
    let lower = QuotationService::validate_price_terms("fob");
    let upper = QuotationService::validate_price_terms("FOB");
    assert!(lower.is_ok());
    assert!(upper.is_ok());
}

/// test_validate_price_terms_ffdmfhcw
#[test]
fn test_validate_price_terms_ffdmfhcw() {
    let result = QuotationService::validate_price_terms("XYZ");
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("FOB"));
}

// ============ 状态常量值正确性测试 ============

/// test_bjdztcl_zzqx
#[test]
fn test_bjdztcl_zzqx() {
    assert_eq!(quotation_status::DRAFT, "draft");
    assert_eq!(quotation_status::APPROVED, "approved");
    assert_eq!(quotation_status::REJECTED, "rejected");
    assert_eq!(quotation_status::CANCELLED, "cancelled");
}

/// test_bjdztcl_hbxt
#[test]
fn test_bjdztcl_hbxt() {
    let states = [
        quotation_status::DRAFT,
        quotation_status::APPROVED,
        quotation_status::REJECTED,
        quotation_status::CANCELLED,
    ];
    let unique: std::collections::HashSet<&str> = states.iter().copied().collect();
    assert_eq!(unique.len(), 4);
}

// ============ QuotationService 构造与 DB 连接测试 ============

/// test_quotationservice_new_zqcysjklj
#[tokio::test]
async fn test_quotationservice_new_zqcysjklj() {
    let db = Arc::new(setup_test_db().await);
    let svc = QuotationService::new(db.clone());
    let _ = svc
        .db
        .execute_raw(sea_orm::Statement::from_sql_and_values(
            svc.db.get_database_backend(),
            "SELECT 1",
            Vec::new(),
        ))
        .await
        .expect("数据库连接应可用");
}

/// test_quotationservice_get_by_id_ksjkfherr —— 钉"已建库空表上 get_by_id 不存在记录
/// ⇒ Err(ServiceError::NotFound)，而非 panic"。
///
/// 契约依据（读函数体）：`src/services/quotation_ops/crud.rs:318-322`
/// find_by_id().one() 空表 ⇒ None ⇒ `.ok_or(ServiceError::NotFound)`。
/// 钉 NotFound 变体而非裸 `is_err()`，防把夹具退化的 Query Err 也当作本条命中；
/// 并同锁装配点出参（`handlers/quotation_handler.rs:598-601`
/// `ServiceError::NotFound` ⇒ `AppError::not_found`，code=NOT_FOUND）。
#[tokio::test]
async fn test_quotationservice_get_by_id_ksjkfherr() {
    let db = setup_test_db().await;
    let svc = QuotationService::new(Arc::new(db));
    let err = svc
        .get_by_id(9999)
        .await
        .expect_err("已建库空表上 get_by_id 不存在记录必须返回 Err 而非 panic");
    assert!(
        matches!(err, ServiceError::NotFound),
        "空表 get_by_id 必须命中 NotFound 变体（crud.rs:321），实得 {err:?}"
    );
    assert_eq!(
        AppError::from(err).error_code(),
        "NOT_FOUND",
        "NotFound 装配点必须出 NOT_FOUND 机器码（quotation_handler.rs:564）"
    );
}

/// test_quotationservice_list_ksjkfherr —— 钉"已建库、业务表已清空 ⇒ `list` 不 panic
/// 且返回**空集**（total=0）"。
///
/// 契约依据（读函数体）：`src/services/quotation_ops/crud.rs:228-264` 对空表
/// 走 `paginate_with_total`（`utils/pagination.rs:17-23` fetch_page=[] /
/// num_items=0）⇒ `Ok(([], 0))`；`attach_names` 对空入参早退 Ok
/// （crud.rs:275-276）。缺 schema 的报错形态不属本条职责——负前提交集由
/// 各域用例经空 schema 库夹具（`connect_empty_schema_db`）另行钉死。
#[tokio::test]
async fn test_quotationservice_list_ksjkfherr() {
    let db = setup_test_db().await;
    let svc = QuotationService::new(Arc::new(db));
    let (items, total) = svc
        .list(1, 20, None, None, None, None, None)
        .await
        .expect("已建库空表上 list 应返回 Ok 空集，而非 Err/panic");
    assert!(
        items.is_empty(),
        "夹具已 TRUNCATE 业务表，列表必须是空集，实得 {} 行",
        items.len()
    );
    assert_eq!(total, 0, "空表的 total 计数应为 0，实得 {total}");
}

/// test_quotationservice_cancel_bczfhapperror —— 钉"已建库空表上 cancel 不存在的单
/// ⇒ NOT_FOUND 机器码，而非 panic"。
///
/// 契约依据：`src/services/quotation_ops/lifecycle.rs:21-26` begin 后
/// find_by_id + lock_exclusive，空表 ⇒ None ⇒ `AppError::not_found`。
/// 钉机器码而非裸 `is_err()`：把状态门（converted ⇒ BUSINESS，lifecycle.rs:27-32）
/// 与夹具退化（DATABASE 族）都排除在外，本条锁的仅是"记录不存在"这一件事。
#[tokio::test]
async fn test_quotationservice_cancel_bczfhapperror() {
    let db = setup_test_db().await;
    let svc = QuotationService::new(Arc::new(db));
    let err = svc
        .cancel(9999, 1)
        .await
        .expect_err("已建库空表上 cancel 不存在的报价单必须返回 Err 而非 panic");
    assert_eq!(
        err.error_code(),
        "NOT_FOUND",
        "空表 cancel 必须命中 not_found（lifecycle.rs:26），实得 {}",
        err.error_code()
    );
}

// ============ update 状态机校验测试 ============

/// test_quotationservice_update_bczfhapperror —— 钉"已建库空表上
/// update 不存在的单 ⇒ NOT_FOUND 机器码，而非 panic"。
///
/// 契约依据：`src/services/quotation_ops/update.rs:33-34` begin 后先
/// `load_for_update`（update.rs:56-74 find_by_id + lock_exclusive，None ⇒
/// `AppError::not_found` :64）；状态门（仅 draft/rejected 可改 ⇒ BUSINESS）在
/// 记录存在时才可达，本条夹具下必先在 not_found 处返回。
#[tokio::test]
async fn test_quotationservice_update_bczfhapperror() {
    let db = setup_test_db().await;
    let svc = QuotationService::new(Arc::new(db));
    let dto = UpdateQuotationDto::default();
    let err = svc
        .update(9999, dto, 1)
        .await
        .expect_err("已建库空表上 update 不存在的报价单必须返回 Err 而非 panic");
    assert_eq!(
        err.error_code(),
        "NOT_FOUND",
        "空表 update 必须命中 not_found（update.rs:64），实得 {}",
        err.error_code()
    );
}
