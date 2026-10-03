//! 契约波 5：交易域两缺陷源码修复的活库集成回归
//!
//! 覆盖缺陷与锁定行为：
//! ① 采购订单附件存了不回显（缺陷①）：
//!    `purchase_orders.attachment_urls`（TEXT[]，models/purchase_order.rs:87，
//!    DDL migration/src/domain/system/mod.rs:326）创建/更新真实落库
//!    （services/po/order_ops/crud.rs:268/:591-592），修复 = `PurchaseOrderDto`
//!    （services/po/order.rs）补同名键，经 Entity::find() 全列 SELECT 按列名直映
//!    （本仓读取富化唯一范式，无二次查询 N+1）。本测以真实 service 链路锁定：
//!    建行带附件 → get_order/list_orders 回读等值 + 出参 JSON 含该键 →
//!    update_order 改附件 → 回读新值。
//! ② 委外三费无处录入（缺陷②）：
//!    `outsourcing_order.processing_fee/freight_fee/tax_amount` NOT NULL
//!    DECIMAL(14,4)（v15/mod.rs:3247-3249）；Create/Update DTO
//!    （outsourcing_ops/types.rs）补齐真实键后锁定：建单带三费回读等值、
//!    draft 期 PUT 三态（覆盖/缺席保持/显式 null 拒清 business_displayable、
//!    负值拒绝）、成本链联动（total_cost=材料+加工费+运费-非正常损耗）、
//!    结算 FEE 凭证金额 = 真实 加工费+运费（非 0）且税额单列回读。
//!
//! 数据库口径：attachment_urls 为 PG TEXT[] 数组列、settle 取号走
//! pg_advisory_xact_lock（utils/number_generator.rs），sqlite 无法同构复现，
//! 故本文件为**活库用例**：TEST_DATABASE_URL 缺失时 `live_pg_db()` 显式炸红
//! （禁止 sqlite 回退假绿/静默 pass）；默认 lane 经 `#[ignore]` reason 显式
//! 声明交由活库 CI lane（ci-test-rust-ignored）执行——形态对齐
//! contract_wave3_explicit_null_clear_test.rs D) 段先例。

use bingxi_backend::models::status::{outsourcing_order_status, outsourcing_voucher_type};
use bingxi_backend::models::{outsourcing_order, outsourcing_voucher, purchase_order, supplier};
use bingxi_backend::services::outsourcing_service::{
    CreateOutsourcingOrderRequest, OutsourcingOrderService, UpdateOutsourcingOrderRequest,
};
use bingxi_backend::services::po::UpdatePurchaseOrderRequest;
use bingxi_backend::services::po::order::PurchaseOrderService;
use bingxi_backend::utils::error::AppError;
use chrono::{TimeZone, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Database, DatabaseConnection, EntityTrait, QueryFilter, Set,
};
use serde_json::json;
use std::sync::Arc;

/// 活库连接：TEST_DATABASE_URL 缺失即显式失败（不许静默回退 sqlite / 静默 skip 假绿）
async fn live_pg_db() -> DatabaseConnection {
    let url = std::env::var("TEST_DATABASE_URL").expect(
        "波5活库用例必须设置 TEST_DATABASE_URL（指向已跑完迁移的 PostgreSQL），禁止 sqlite 回退",
    );
    Database::connect(&url)
        .await
        .expect("波5活库用例：TEST_DATABASE_URL 指向的 PostgreSQL 连接失败")
}

fn uniq_tag() -> String {
    Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_default()
        .to_string()
}

/// 种一个最小真实供应商（PO 表对 suppliers 有 FK：m0001_initial_schema.rs:636；
/// 委外 service 创建校验 supplier::Entity::find_by_id 真实存在）。
/// 列齐性依据：`models/supplier.rs` 的 supplier_short_name/supplier_type/credit_code/
/// registered_address/legal_representative/registered_capital/establishment_date/
/// taxpayer_type/bank_name/bank_account/contact_phone/is_processor 均为非 Option 字段，
/// SeaORM insert 必须逐列显式 Set——CI #4672 实证只给 code/name 即
/// `Type("Missing value for column 'supplier_short_name'")`；取值口径照抄本仓活库
/// 范式 `contract_wave5_inspection_result_authority_test.rs:107-128`（同为供应商 FK 父行种子）。
async fn seed_supplier(db: &DatabaseConnection, tag: &str) -> i32 {
    let sup_now: chrono::DateTime<chrono::FixedOffset> = Utc::now().into();
    let s = supplier::ActiveModel {
        supplier_code: Set(format!("W5S{tag}")),
        supplier_name: Set(format!("波5契约供应商{tag}")),
        supplier_short_name: Set(format!("契供{tag}")),
        supplier_type: Set("面料供应商".to_string()),
        credit_code: Set(format!("91330000W5{tag}X")),
        registered_address: Set("波5契约注册地址".to_string()),
        legal_representative: Set("契约法人".to_string()),
        registered_capital: Set(Decimal::ZERO),
        establishment_date: Set(chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
        taxpayer_type: Set("一般纳税人".to_string()),
        bank_name: Set("波5契约银行".to_string()),
        bank_account: Set(format!("6222000000{tag}")),
        contact_phone: Set("13800000001".to_string()),
        is_processor: Set(false),
        created_at: Set(sup_now),
        updated_at: Set(sup_now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("波5夹具：供应商种子写入失败");
    s.id
}

fn d(v: i64) -> Decimal {
    Decimal::from(v)
}

// =========================================================
// ① 采购订单 attachment_urls：写入→DTO 回读→更新→回读
// =========================================================

#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL：attachment_urls 为 TEXT[] 数组列，sqlite 方言不支持，无法非活库化"]
async fn po_attachment_urls_roundtrip_through_dto() {
    let db = live_pg_db().await;
    let tag = uniq_tag();
    let supplier_id = seed_supplier(&db, &tag).await;
    let service = PurchaseOrderService::new(Arc::new(db.clone()));

    // 创建落库：与 crud.rs:268 build_order_header_active_model 同一列写入路径
    let order_no = format!("W5P{tag}");
    let attachments = vec![
        "https://example.com/contract.pdf".to_string(),
        "https://example.com/spec.txt".to_string(),
    ];
    let order = purchase_order::ActiveModel {
        order_no: Set(order_no.clone()),
        supplier_id: Set(supplier_id),
        order_date: Set(Utc
            .with_ymd_and_hms(2026, 9, 20, 0, 0, 0)
            .unwrap()
            .date_naive()),
        warehouse_id: Set(1),
        department_id: Set(1),
        purchaser_id: Set(9101),
        currency: Set("CNY".to_string()),
        exchange_rate: Set(Decimal::ONE),
        total_amount: Set(Decimal::ZERO),
        total_amount_foreign: Set(Decimal::ZERO),
        total_quantity: Set(Decimal::ZERO),
        total_quantity_alt: Set(Decimal::ZERO),
        order_status: Set(bingxi_backend::models::status::purchase_order::DRAFT.to_string()),
        attachment_urls: Set(Some(attachments.clone())),
        created_by: Set(9101),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("波5夹具：采购订单建行失败");

    // 详情出参（缺陷①靶心：修复前 PurchaseOrderDto 无该键，回读恒 None）
    let detail = service
        .get_order(order.id, None)
        .await
        .expect("get_order 查询失败");
    assert_eq!(
        detail.attachment_urls.as_deref(),
        Some(attachments.as_slice()),
        "PO 详情 DTO.attachment_urls 必须与落库数组逐元素等值"
    );
    let detail_json = serde_json::to_value(&detail).expect("PurchaseOrderDto 序列化失败");
    assert_eq!(
        detail_json.get("attachment_urls"),
        Some(&json!(attachments)),
        "出参 JSON 必须含键 attachment_urls（与模型列同源 snake_case）且值为真实附件数组"
    );

    // 列表出参同源
    let (rows, _total) = service
        .list_orders(1, 100, None, Some(supplier_id), Some(order_no), None)
        .await
        .expect("list_orders 查询失败");
    let row = rows
        .iter()
        .find(|r| r.id == order.id)
        .expect("列表应含本用例订单");
    assert_eq!(
        row.attachment_urls.as_deref(),
        Some(attachments.as_slice()),
        "PO 列表 DTO.attachment_urls 必须与详情同源回显"
    );

    // 更新链路（crud.rs:591-592）→ 回读新值
    let updated_att = vec!["https://example.com/amended.pdf".to_string()];
    service
        .update_order(
            order.id,
            UpdatePurchaseOrderRequest {
                attachment_urls: Some(updated_att.clone()),
                ..Default::default()
            },
            9101,
        )
        .await
        .expect("update_order 失败");
    let after = service
        .get_order(order.id, None)
        .await
        .expect("改后 get_order 失败");
    assert_eq!(
        after.attachment_urls.as_deref(),
        Some(updated_att.as_slice()),
        "PUT 后详情回读必须等于新附件列表（再编辑显示得出来）"
    );

    // 清理（尽力而为）：直删本用例行
    let _ = purchase_order::Entity::delete_by_id(order.id)
        .exec(&db)
        .await;
    let _ = supplier::Entity::delete_by_id(supplier_id).exec(&db).await;
}

// =========================================================
// ② 委外三费：Create/Update DTO 键 → 落库回读 → 成本链 → 结算 FEE 凭证
// =========================================================

fn make_create_req(supplier_id: i32, order_no: &str) -> CreateOutsourcingOrderRequest {
    CreateOutsourcingOrderRequest {
        order_no: order_no.to_string(),
        order_type: "dyeing".to_string(),
        supplier_id,
        production_order_id: None,
        dye_batch_id: None,
        color_no: Some("W5C".to_string()),
        dye_lot_no: Some("W5L".to_string()),
        issue_date: Utc
            .with_ymd_and_hms(2026, 9, 20, 0, 0, 0)
            .unwrap()
            .date_naive(),
        expected_return_date: None,
        issue_quantity: d(100),
        issue_unit: Some("kg".to_string()),
        material_cost: d(1000),
        processing_fee: d(100),
        freight_fee: d(20),
        tax_amount: d(8),
        standard_loss_rate: Some(Decimal::new(10, 2)),
        remarks: Some("波5契约测".to_string()),
        created_by: Some(9101),
    }
}

#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL：settle 取号走 pg_advisory_xact_lock，sqlite 方言不支持"]
async fn outsourcing_fees_create_update_and_fee_voucher() {
    let db = live_pg_db().await;
    let tag = uniq_tag();
    let supplier_id = seed_supplier(&db, &tag).await;
    let service = OutsourcingOrderService::new(Arc::new(db.clone()));
    let order_no = format!("W5O{tag}");

    // —— 建单带三费：真实落库 + 回读等值；total_cost 成本链含三费 ——
    let created = service
        .create(make_create_req(supplier_id, &order_no))
        .await
        .expect("带三费的委外建单失败");
    assert_eq!(created.processing_fee, d(100), "建单加工费必须原样落库回读");
    assert_eq!(created.freight_fee, d(20), "建单运费必须原样落库回读");
    assert_eq!(created.tax_amount, d(8), "建单税额必须原样落库回读");
    assert_eq!(
        created.total_cost,
        d(1120),
        "建单 total_cost=材料1000+加工100+运费20-非正常损耗0（compute_total_cost 权威公式）"
    );

    // —— PUT 三态·覆盖：processing_fee=150（键缺席的 freight/tax 必须保持） ——
    let req: UpdateOutsourcingOrderRequest =
        serde_json::from_value(json!({ "processing_fee": "150" }))
            .expect("update DTO 应接受 processing_fee 键（缺陷②修复根因键）");
    let updated = service.update(created.id, req).await.expect("三费更新失败");
    assert_eq!(updated.processing_fee, d(150), "覆盖后加工费回读等值");
    assert_eq!(updated.freight_fee, d(20), "键缺席运费必须保持");
    assert_eq!(updated.tax_amount, d(8), "键缺席税额必须保持");
    assert_eq!(
        updated.total_cost,
        d(1170),
        "total_cost 随加工费覆盖联动重算=1000+150+20"
    );

    // —— NOT NULL 列拒清：显式 null 在任何落库前 business_displayable 拒绝且无痕 ——
    for (payload, phrase) in [
        (json!({ "processing_fee": null }), "加工费不能清空"),
        (json!({ "freight_fee": null }), "运费不能清空"),
        (json!({ "tax_amount": null }), "税额不能清空"),
    ] {
        let clear: UpdateOutsourcingOrderRequest =
            serde_json::from_value(payload).expect("显式 null 应可反序列化为 Some(None)");
        let err = service
            .update(created.id, clear)
            .await
            .expect_err("NOT NULL 列显式 null 必须被拒绝");
        assert!(
            matches!(&err, AppError::BusinessErrorDisplayable(m) if m.contains(phrase)),
            "期望可外显业务错误「{phrase}…」，实际: {err:?}"
        );
    }
    let untouched = service
        .get_by_id(created.id)
        .await
        .expect("被拒请求后回读失败");
    assert_eq!(
        (
            untouched.processing_fee,
            untouched.freight_fee,
            untouched.tax_amount
        ),
        (d(150), d(20), d(8)),
        "拒清必须无痕：三费保持拒绝前原值"
    );

    // —— 负值拒绝 ——
    let neg: UpdateOutsourcingOrderRequest =
        serde_json::from_value(json!({ "freight_fee": "-1" })).unwrap();
    let err = service
        .update(created.id, neg)
        .await
        .expect_err("负运费必须被拒绝");
    assert!(
        matches!(&err, AppError::BusinessError(m) if m.contains("运费不能为负")),
        "实际: {err:?}"
    );
    let neg_create = {
        let mut r = make_create_req(supplier_id, &format!("W5N{tag}"));
        r.processing_fee = Decimal::from(-5);
        service.create(r).await
    };
    assert!(
        matches!(&neg_create, Err(AppError::BusinessError(m)) if m.contains("加工费不能为负")),
        "建单负加工费必须被拒绝，实际: {:?}",
        neg_create.err()
    );

    // —— 推进 received（测试侧状态铺垫，真实门控在 receipt.confirm）后结算 ——
    let mut active: outsourcing_order::ActiveModel =
        service.get_by_id(created.id).await.unwrap().into();
    active.status = Set(outsourcing_order_status::RECEIVED.to_string());
    active.update(&db).await.expect("状态铺垫失败");

    let settled = service.settle(created.id).await.expect("结算失败");
    assert_eq!(settled.status, outsourcing_order_status::SETTLED);
    assert_eq!(
        settled.total_cost,
        d(1170),
        "结算 total_cost 必须按真实三费重算（非 0）"
    );

    // FEE 凭证金额 = 真实 加工费+运费 = 170（缺陷②靶心：修复前经 API 无处录入 ⇒ 恒 0）
    let fee_voucher = outsourcing_voucher::Entity::find()
        .filter(outsourcing_voucher::Column::OutsourcingOrderId.eq(created.id))
        .filter(outsourcing_voucher::Column::VoucherType.eq(outsourcing_voucher_type::FEE))
        .one(&db)
        .await
        .expect("FEE 凭证查询失败")
        .expect("结算应生成 FEE 凭证");
    assert_eq!(
        fee_voucher.amount,
        d(170),
        "FEE 凭证金额必须=processing_fee+freight_fee 真实值（不再恒 0）"
    );
    assert_eq!(
        fee_voucher.tax_amount,
        d(8),
        "FEE 凭证 tax_amount 必须回读建单录入的进项税额"
    );
    assert!(
        settled.voucher_no_fee.is_some(),
        "结算必须回写 FEE 凭证号（OVFE 前缀，settle 链路同源）"
    );

    // 清理（尽力而为）
    let _ = outsourcing_voucher::Entity::delete_many()
        .filter(outsourcing_voucher::Column::OutsourcingOrderId.eq(created.id))
        .exec(&db)
        .await;
    let _ = outsourcing_order::Entity::delete_by_id(created.id)
        .exec(&db)
        .await;
    let _ = supplier::Entity::delete_by_id(supplier_id).exec(&db).await;
}

/// 建单缺省三费键 = 0 起步（serde(default)，与原 Set(ZERO) 初始化同值；
/// 显式 null 在 serde 层即被类型校验拒绝，绝不落 NULL）
#[test]
fn outsourcing_create_dto_fee_semantics_are_not_null_safe() {
    // 键缺席 ⇒ Decimal::ZERO（非 Option，NOT NULL 列类型层不标可选）
    let absent: CreateOutsourcingOrderRequest = serde_json::from_value(json!({
        "order_no": "X1", "order_type": "dyeing", "supplier_id": 1,
        "issue_date": "2026-09-20", "issue_quantity": "100", "material_cost": "1000",
    }))
    .expect("缺省三费键建单请求应可反序列化");
    assert_eq!(absent.processing_fee, Decimal::ZERO);
    assert_eq!(absent.freight_fee, Decimal::ZERO);
    assert_eq!(absent.tax_amount, Decimal::ZERO);
    // 显式 null ⇒ 反序列化失败（类型错误），不可能把 NULL 送进 NOT NULL 列
    let null_res: Result<CreateOutsourcingOrderRequest, _> = serde_json::from_value(json!({
        "order_no": "X2", "order_type": "dyeing", "supplier_id": 1,
        "issue_date": "2026-09-20", "issue_quantity": "100", "material_cost": "1000",
        "processing_fee": null,
    }));
    assert!(
        null_res.is_err(),
        "NOT NULL 列显式 null 必须在反序列化层拒绝，不得塌成缺省"
    );
    // 有值 ⇒ 真实值（number/string 双形态按 rust_decimal 既有口径）
    let given: CreateOutsourcingOrderRequest = serde_json::from_value(json!({
        "order_no": "X3", "order_type": "dyeing", "supplier_id": 1,
        "issue_date": "2026-09-20", "issue_quantity": "100", "material_cost": "1000",
        "processing_fee": "100.5", "freight_fee": 20, "tax_amount": 8,
    }))
    .expect("带值三费建单请求应可反序列化");
    assert_eq!(given.processing_fee, Decimal::new(1005, 1));
    assert_eq!(given.freight_fee, Decimal::from(20));
    assert_eq!(given.tax_amount, Decimal::from(8));
}
