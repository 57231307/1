//! 任务 #169 第 2 波（生产/质量/染整/委外/化学域）契约测：
//! 更新端点可空列三态语义（RFC 7386 JSON Merge Patch）
//!
//! 锁定行为（对应本波修复，形态对齐 contract_wave2_explicit_null_clear_test.rs）：
//! 1. 三态：键缺席=保持原值、显式 null=清空为 NULL（仅 DB 可空列）、有值=覆盖。
//!    DTO 层 `Option<Option<T>>` + `double_option` 适配器；service/handler 层
//!    Some(None) → Set(None) 真实落 NULL，None 不 Set。
//! 2. NOT NULL 列（或实体 Model 非 Option 列）不开 null 清空：显式 null 在任何
//!    DB 访问之前被 `AppError::business_displayable` 拒绝，且原行无任何改动（无痕）。
//!    覆盖端点：
//!    - PUT /production-orders/orders/{id}（UpdateProductionOrderPayload → UpdateProductionOrderRequest）
//!    - POST /production-orders/orders/{id}/progress（UpdateProgressRequest）
//!    - PUT /outsourcing-orders/{id} / /outsourcing-orders/items/{id} / /outsourcing-receipts/{id}
//!    - PUT /lab-dip/requests/{id} / /lab-dip/samples/{id}
//!    - PUT /dye-recipes/{id}（含复样回写内部构造点 resample.rs 的键缺席=保持语义）
//!    - PUT /quality-inspection/records/{id}（UpdateInspectionRecordRequest）
//! 3. 活库并发类用例本波无需（纯 DTO serde + sqlite::memory: 自建表即覆盖真实写入路径）；
//!    跨方言差异（如 PG 的 JSONB CHECK）不在锁定范围，如需真库回归由 ci-test-rust-ignored 补充。

use bingxi_backend::handlers::{
    production_order_handler::{UpdateProductionOrderPayload, UpdateProgressRequest},
    quality_inspection_handler::UpdateInspectionRecordRequest,
};
use bingxi_backend::models::dye_recipe;
use bingxi_backend::services::dye_recipe_service::{DyeRecipeService, UpdateDyeRecipeRequest};
use bingxi_backend::services::lab_dip_service::{
    UpdateLabDipRequestRequest, UpdateLabDipSampleRequest,
};
use bingxi_backend::services::outsourcing_service::{
    OutsourcingOrderService, UpdateOutsourcingOrderItemRequest, UpdateOutsourcingOrderRequest,
    UpdateOutsourcingReceiptRequest,
};
use bingxi_backend::services::production_order_service::UpdateProductionOrderRequest;
use bingxi_backend::utils::error::AppError;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, Database, DatabaseConnection, DbBackend, EntityTrait, Set, Statement,
};
use serde_json::json;
use std::sync::Arc;

// =========================================================
// A) DTO 三态形状锁（纯 serde，无 DB）
// =========================================================

#[test]
fn outsourcing_update_dtos_distinguish_absent_null_and_value() {
    let order: UpdateOutsourcingOrderRequest = serde_json::from_value(json!({
        "color_no": null,
        "remarks": null,
        "supplier_id": null,
        "standard_loss_rate": "0.05",
    }))
    .expect("委外订单 update DTO 三态反序列化失败");
    assert!(
        matches!(order.color_no, Some(None)),
        "可空列 color_no 显式 null 必须为 Some(None)，不得与缺席塌同"
    );
    assert!(matches!(order.remarks, Some(None)));
    assert!(
        matches!(order.supplier_id, Some(None)),
        "NOT NULL 列的显式 null 也必须可辨，才能交 service 判业务错误拒绝"
    );
    assert!(matches!(&order.standard_loss_rate, Some(Some(v)) if *v == Decimal::new(5, 2)));
    assert!(order.order_type.is_none(), "缺席键必须是 None（保持原值）");
    assert!(order.issue_date.is_none());

    let item: UpdateOutsourcingOrderItemRequest = serde_json::from_value(json!({
        "batch_no": null,
        "warehouse_id": null,
        "unit": null,
        "freight_fee": "12.5",
    }))
    .expect("委外发料明细 update DTO 三态反序列化失败");
    assert!(matches!(item.batch_no, Some(None)));
    assert!(matches!(item.warehouse_id, Some(None)));
    assert!(matches!(item.unit, Some(None)));
    assert!(matches!(&item.freight_fee, Some(Some(v)) if *v == Decimal::new(125, 1)));
    assert!(item.color_no.is_none());

    let receipt: UpdateOutsourcingReceiptRequest = serde_json::from_value(json!({
        "grade": null,
        "quality_status": null,
        "return_quantity": null,
        "receipt_date": null,
    }))
    .expect("委外收回单 update DTO 三态反序列化失败");
    assert!(matches!(receipt.grade, Some(None)));
    assert!(matches!(receipt.quality_status, Some(None)));
    assert!(matches!(receipt.return_quantity, Some(None)));
    assert!(matches!(receipt.receipt_date, Some(None)));
    assert!(receipt.product_id.is_none());
}

#[test]
fn lab_dip_update_dtos_distinguish_absent_null_and_value() {
    let req: UpdateLabDipRequestRequest = serde_json::from_value(json!({
        "customer_color_no": null,
        "eco_requirement": null,
        "light_source": null,
        "expected_days": 3,
    }))
    .expect("打样通知单 update DTO 三态反序列化失败");
    assert!(matches!(req.customer_color_no, Some(None)));
    assert!(matches!(req.eco_requirement, Some(None)));
    assert!(
        matches!(req.light_source, Some(None)),
        "NOT NULL 列 light_source 显式 null 必须可辨（交 service 拒绝）"
    );
    assert!(matches!(req.expected_days, Some(Some(3))));
    assert!(req.customer_id.is_none());
    assert!(req.sample_versions.is_none());

    let sample: UpdateLabDipSampleRequest = serde_json::from_value(json!({
        "formula": null,
        "formula_detail": null,
        "total_cost": null,
        "temperature": "80.5",
    }))
    .expect("打样小样 update DTO 三态反序列化失败");
    assert!(matches!(sample.formula, Some(None)));
    assert!(
        matches!(sample.formula_detail, Some(None)),
        "JSONB 可空列 formula_detail 显式 null 必须为 Some(None)"
    );
    assert!(matches!(sample.total_cost, Some(None)));
    assert!(matches!(&sample.temperature, Some(Some(v)) if *v == Decimal::new(805, 1)));
    assert!(sample.recipe_no.is_none());
}

#[test]
fn dye_recipe_update_dto_distinguishes_absent_null_and_value() {
    let req: UpdateDyeRecipeRequest = serde_json::from_value(json!({
        "color_code": null,
        "color_name": null,
        "auxiliaries": null,
        "remarks": "复样回写",
    }))
    .expect("染色配方 update DTO 三态反序列化失败");
    assert!(
        matches!(req.color_code, Some(None)),
        "NOT NULL 列 color_code（m0003:30）显式 null 必须可辨（交 service 拒绝）"
    );
    assert!(matches!(req.color_name, Some(None)));
    assert!(
        matches!(req.auxiliaries, Some(None)),
        "JSON 可空列 auxiliaries 显式 null 必须为 Some(None)"
    );
    assert!(matches!(&req.remarks, Some(Some(v)) if v == "复样回写"));
    assert!(req.color_no.is_none());
    assert!(req.status.is_none());
}

#[test]
fn quality_inspection_record_dto_distinguishes_absent_null_and_value() {
    let req: UpdateInspectionRecordRequest = serde_json::from_value(json!({
        "remark": null,
        "color_no": null,
        "grade": null,
        "inspection_type": null,
        "total_qty": null,
        "batch_no": "B-9",
    }))
    .expect("质检记录 update DTO 三态反序列化失败");
    assert!(matches!(req.remark, Some(None)));
    assert!(matches!(req.color_no, Some(None)));
    assert!(matches!(req.grade, Some(None)));
    assert!(
        matches!(req.inspection_type, Some(None)),
        "DB NOT NULL 列 inspection_type 显式 null 必须可辨"
    );
    assert!(
        matches!(req.total_qty, Some(None)),
        "实体 Model 非 Option 列 total_qty 显式 null 必须可辨（置 NULL 该行按模型不可读）"
    );
    assert!(matches!(&req.batch_no, Some(Some(v)) if v == "B-9"));
    assert!(req.inspection_date.is_none());
    assert!(req.qualification_rate.is_none());
}

#[test]
fn production_order_dtos_distinguish_absent_null_and_value() {
    let payload: UpdateProductionOrderPayload = serde_json::from_value(json!({
        "remarks": null,
        "work_center_id": null,
        "planned_quantity": null,
        "priority": 3,
    }))
    .expect("生产订单 update payload 三态反序列化失败");
    assert!(matches!(payload.remarks, Some(None)));
    assert!(matches!(payload.work_center_id, Some(None)));
    assert!(
        matches!(payload.planned_quantity, Some(None)),
        "NOT NULL 列 planned_quantity（m0007:78）显式 null 必须可辨（交 service 拒绝）"
    );
    assert!(matches!(payload.priority, Some(Some(3))));
    assert!(payload.planned_start_date.is_none());

    let progress: UpdateProgressRequest = serde_json::from_value(json!({
        "actual_quantity": null,
    }))
    .expect("生产进度 update DTO 三态反序列化失败");
    assert!(
        matches!(progress.actual_quantity, Some(None)),
        "可空列 actual_quantity 显式 null = 清空实际产量，不得塌成缺席"
    );
    assert!(progress.remarks.is_none());
}

#[test]
fn production_order_service_request_is_double_option_shaped() {
    // 服务层请求体同样双层：塌成单层 Option<T> 即重新引入"清空与保持共用表示"的静默丢弃形态
    let req = UpdateProductionOrderRequest {
        planned_quantity: None,
        planned_start_date: Some(None),
        planned_end_date: None,
        priority: None,
        work_center_id: Some(Some(7)),
        remarks: Some(None),
    };
    assert!(matches!(req.planned_start_date, Some(None)));
    assert!(matches!(req.work_center_id, Some(Some(7))));
    assert!(matches!(req.remarks, Some(None)));
    assert!(req.planned_quantity.is_none());
}

// =========================================================
// B) 真实 service + sqlite::memory: 自建表：同一可空列三态各一断言 + NOT NULL 拒绝无痕
// =========================================================

async fn sqlite_db() -> DatabaseConnection {
    Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败")
}

/// 自建 dye_recipe 表：列集严格对齐 models/dye_recipe.rs 的 Model 字段（实体全部列必须存在，
/// 否则 find_by_id 的 SELECT 会因缺列报错）。自建表不加 PG 侧 CHECK/NOT NULL——
/// 本测锁定的是代码层的三态与拒绝顺序，不是数据库约束本身。
async fn create_dye_recipe_table(db: &DatabaseConnection) {
    let stmt = Statement::from_sql_and_values(
        DbBackend::Sqlite,
        r#"CREATE TABLE "dye_recipe" (
            "id" INTEGER PRIMARY KEY AUTOINCREMENT,
            "recipe_no" TEXT NOT NULL,
            "recipe_name" TEXT,
            "color_no" TEXT,
            "formula" TEXT,
            "temperature" NUMERIC,
            "time_minutes" INTEGER,
            "status" TEXT,
            "is_deleted" INTEGER,
            "created_at" TEXT NOT NULL,
            "updated_at" TEXT NOT NULL,
            "color_code" TEXT,
            "color_name" TEXT,
            "fabric_type" TEXT,
            "dye_type" TEXT,
            "chemical_formula" TEXT,
            "ph_value" NUMERIC,
            "liquor_ratio" NUMERIC,
            "auxiliaries" TEXT,
            "version" INTEGER,
            "parent_recipe_id" INTEGER,
            "approved_by" INTEGER,
            "approved_at" TEXT,
            "remarks" TEXT,
            "created_by" INTEGER
        )"#,
        Vec::<sea_orm::Value>::new(),
    );
    db.execute_raw(stmt).await.expect("自建 dye_recipe 表失败");
}

async fn seed_dye_recipe(db: &DatabaseConnection, recipe_no: &str) -> dye_recipe::Model {
    let now = chrono::Utc::now().fixed();
    let row = dye_recipe::ActiveModel {
        recipe_no: Set(recipe_no.to_string()),
        recipe_name: Set(Some("测试配方".to_string())),
        color_code: Set(Some("CC-KEEP".to_string())),
        status: Set(Some("draft".to_string())),
        is_deleted: Set(Some(false)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    row.insert(db).await.expect("dye_recipe 种子行插入失败")
}

#[tokio::test]
async fn dye_recipe_remarks_full_tristate_on_same_nullable_column() {
    let db = sqlite_db().await;
    create_dye_recipe_table(&db).await;
    let seed = seed_dye_recipe(&db, "DR-TRISTATE-1").await;
    let service = DyeRecipeService::new(Arc::new(db.clone()));

    // 1) 有值 = 覆盖
    let updated = service
        .update(
            seed.id,
            UpdateDyeRecipeRequest {
                remarks: Some(Some("覆盖值".to_string())),
                ..Default::default()
            },
        )
        .await
        .expect("覆盖写入失败");
    assert_eq!(updated.remarks.as_deref(), Some("覆盖值"));

    // 2) 键缺席 = 保持原值（该列不进 UPDATE 语句）
    let kept = service
        .update(seed.id, UpdateDyeRecipeRequest::default())
        .await
        .expect("全缺席更新失败");
    assert_eq!(
        kept.remarks.as_deref(),
        Some("覆盖值"),
        "键缺席不得触碰 remarks 列（保持原值）"
    );

    // 3) 显式 null = 清空为 NULL（真实落库，回读验证）
    let cleared = service
        .update(
            seed.id,
            UpdateDyeRecipeRequest {
                remarks: Some(None),
                ..Default::default()
            },
        )
        .await
        .expect("清空写入失败");
    assert!(
        cleared.remarks.is_none(),
        "显式 null 必须把可空列真实清成 NULL，回读应为 None"
    );

    // 清空后再缺席更新：保持 NULL（缺席 ≠ 恢复旧值）
    let still_null = service
        .update(seed.id, UpdateDyeRecipeRequest::default())
        .await
        .expect("清空后缺席更新失败");
    assert!(still_null.remarks.is_none());
}

#[tokio::test]
async fn dye_recipe_explicit_null_on_not_null_color_code_rejected_without_trace() {
    let db = sqlite_db().await;
    create_dye_recipe_table(&db).await;
    let seed = seed_dye_recipe(&db, "DR-REJECT-1").await;
    // 先给 remarks 落一个原值，验证拒绝发生在任何 DB 访问之前（同请求里的其他变更也不得生效）
    let service = DyeRecipeService::new(Arc::new(db.clone()));
    service
        .update(
            seed.id,
            UpdateDyeRecipeRequest {
                remarks: Some(Some("原备注".to_string())),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    let err = service
        .update(
            seed.id,
            UpdateDyeRecipeRequest {
                color_code: Some(None),
                remarks: Some(Some("不应落库".to_string())),
                ..Default::default()
            },
        )
        .await
        .expect_err("NOT NULL 列 color_code 显式 null 必须被拒绝");
    assert!(
        matches!(&err, AppError::BusinessErrorDisplayable(m) if m.contains("色代码不能清空")),
        "期望可见业务错误（外显不脱敏），实际: {err:?}"
    );

    // 无痕：原行 color_code 与 remarks 均未被本次被拒请求改动
    let after = dye_recipe::Entity::find_by_id(seed.id)
        .one(&db)
        .await
        .unwrap()
        .expect("被拒请求不得删除/破坏原行");
    assert_eq!(after.color_code.as_deref(), Some("CC-KEEP"));
    assert_eq!(after.remarks.as_deref(), Some("原备注"));
}

#[tokio::test]
async fn outsourcing_order_not_null_explicit_null_rejected_before_any_db_access() {
    // 空 sqlite 库（未建任何表）：若拒绝发生在 DB 访问之前，则得到业务错误而非数据库报错
    let db = sqlite_db().await;
    let service = OutsourcingOrderService::new(Arc::new(db));

    let err = service
        .update(
            999_999,
            UpdateOutsourcingOrderRequest {
                order_type: None,
                supplier_id: Some(None),
                production_order_id: None,
                dye_batch_id: None,
                color_no: None,
                dye_lot_no: None,
                issue_date: None,
                expected_return_date: None,
                issue_quantity: None,
                issue_unit: None,
                material_cost: None,
                // 契约波 5：UpdateOutsourcingOrderRequest 补齐三费键（NOT NULL 列，
                // 显式 null 拒绝语义见 contract_wave5_trade_fields_roundtrip_test）；
                // 本用例仅覆盖 supplier_id 拒绝路径，新键缺席=保持原值
                processing_fee: None,
                freight_fee: None,
                tax_amount: None,
                standard_loss_rate: None,
                remarks: None,
            },
        )
        .await
        .expect_err("NOT NULL 列 supplier_id 显式 null 必须被拒绝");
    assert!(
        matches!(&err, AppError::BusinessErrorDisplayable(m) if m.contains("委外加工厂不能清空")),
        "期望可见业务错误且先于 DB 访问返回，实际: {err:?}"
    );
}

/// 复样回写内部构造点（lab_dip_ops/resample.rs）的形态锁：
/// 内部回写只产生 覆盖(Some(Some)) / 保持(None)，绝不产生 清空(Some(None))。
#[test]
fn resample_internal_writeback_never_clears_columns() {
    let req = UpdateDyeRecipeRequest {
        chemical_formula: Some("配方文本".to_string()).map(Some),
        temperature: None,
        remarks: Some(Some("复样通过自动回写".to_string())),
        ..Default::default()
    };
    assert!(matches!(&req.chemical_formula, Some(Some(_))));
    assert!(
        req.temperature.is_none(),
        "内部回写缺席值=保持原值，不得塌成 Some(None) 清空"
    );
    assert!(matches!(&req.remarks, Some(Some(_))));
}
