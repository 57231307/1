//! 调拨出库四维口径锁：染色布缺缸号/缺匹号 → 4xx 业务错（非裸 500/非放行）
//!
//! 锁定的符号契约（行号随重构漂移，以符号为准）：
//! - `backend/src/services/inventory_deduction.rs`（`OutboundDimensions` 四维结构 +
//!   `require_outbound_dimensions`：维度**必填/取值**校验失败属字段族，统一包成
//!   `AppError::validation_displayable("{单据}：{原因}")` —— 400/VALIDATION_ERROR 族且
//!   出参携带真实缺维原因；内部产品 ID 不进 HTTP 出参，只写日志）
//! - `backend/src/services/inv/fabric_class.rs`（唯一判定来源：批次必填；
//!   色号非空=染色布 → 缸号必填 + `normalize_outbound_piece_no` 匹号必填，
//!   仅按空串判定不看名称；trim 归一；白坯免缸号免匹号既有口径不动）
//! - `backend/src/services/inv/stock.rs:24-60,71-114`（check_from_warehouse_inventory：
//!   调拨建单事务内按四维精确匹配调出仓库存；同产品多批次必须传维度定位；
//!   白坯维持宽松不强制缸号——两分支都在此锁死，防止回归改回单维 product_id）
//! - `backend/src/handlers/inventory_transfer_handler.rs:105-150`（create_transfer HTTP 层：
//!   上述业务错经 `?` 传播为 400，扣减校验失败事务整体回滚不残留单据）
//! - PO 回写/退货审批链的四维行为锁在 `contract_wave1_purchase_return_dimensions_test.rs`
//!   （purchase_return_service.rs:444-506/566-734），本文件不重复。
//!
//! 覆盖策略：
//! - require_outbound_dimensions 与调拨 DTO 为纯函数/serde 断言（**无需任何 DB**）：
//!   含错误族别、错误码、Display 原文与 400 信封映射
//! - `#[ignore]` 活库用例：走真实 handler 在已迁移 PG 上建调拨单——
//!   染色布缺缸号 → 400 VALIDATION_ERROR 且调拨单不残留、库存不动（事务回滚）；
//!   白坯四维宽松 → 200（对照防过度收紧）。
//!   注：调拨 create 链路含 advisory_xact_lock 单号生成 + 频率检查用 Postgres 方言
//!   原生 SQL（handler L113-134），**sqlite 无法非活库化此链**（该 PG 方言硬编码本身
//!   已作为挂账记入交付报告）。

mod test_common;

use rust_decimal::Decimal;
use serde_json::json;
use std::str::FromStr;

use bingxi_backend::services::inv::{CreateInventoryTransferRequest, InventoryTransferItemRequest};
use bingxi_backend::services::inventory_deduction::require_outbound_dimensions;
use bingxi_backend::utils::error::AppError;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

// =========================================================
// A) require_outbound_dimensions 纯函数矩阵（无 DB）
// =========================================================

/// 染色布（色号非空）缺缸号 → validation 族错（字段必填族）；Display 携带单据标签+权威原因文案
/// （2026-10-02 口径补第四维匹号：本用例匹号给足，隔离锁定缸号维度）
#[test]
fn dyed_fabric_missing_dye_lot_returns_validation_error() {
    let err =
        require_outbound_dimensions("调拨出库", 7, Some("COL-A"), None, Some("B7"), Some("P-7"))
            .expect_err("染色布缺缸号必须拒绝（本波四维口径，修复前仅 product_id 单维放行）");
    match &err {
        AppError::ValidationErrorDisplayable(_) => {}
        other => panic!("必须是可外显 validation（字段必填族，400 映射），实际: {other:?}"),
    }
    assert_eq!(err.error_code(), "VALIDATION_ERROR");
    let disp = err.to_string();
    assert!(disp.contains("调拨出库"), "实际: {disp}");
    assert!(
        !disp.contains("款号产品"),
        "内部产品 ID 不得进拒绝文案，实际: {disp}"
    );
    assert!(disp.contains("染色布必须提供缸号"), "实际: {disp}");
    assert!(
        disp.contains("COL-A"),
        "原因须回显缺失对象的色号，实际: {disp}"
    );
}

/// 批次（batch_no）任何布种必填：缺失报"批次不得为空"族错
#[test]
fn missing_batch_returns_validation_error() {
    let err = require_outbound_dimensions("调拨出库", 9, Some(""), Some("DYE-1"), None, None)
        .expect_err("批次必填");
    let disp = err.to_string();
    assert!(disp.contains("批次不得为空"), "实际: {disp}");
    assert!(matches!(err, AppError::ValidationErrorDisplayable(_)));

    // trim 后为空串等价缺失（不允许空格蒙混）
    let err2 = require_outbound_dimensions("调拨出库", 9, None, None, Some("   "), None)
        .expect_err("空白批次必须拒绝");
    assert!(err2.to_string().contains("批次不得为空"));
}

/// 白坯布（色号为空/缺失）：不强制缸号也不强制匹号，维持宽松放行；维度归一值精确断言
#[test]
fn white_fabric_passes_without_dye_lot_and_normalizes() {
    let dims = require_outbound_dimensions("调拨出库", 3, None, None, Some(" B7 "), None)
        .expect("白坯免缸号免匹号（回归锁：不得把白坯也强制四维）");
    assert_eq!(dims.color_no, "", "白坯归一为空串");
    assert_eq!(dims.dye_lot_no, None);
    assert_eq!(dims.piece_no, None, "白坯匹号免填归一 None");
    assert_eq!(dims.batch_no, "B7", "trim 归一后落库/匹配");

    // 纯空白色号按白坯处理（fabric_class.rs 注释：仅按空判定，不看名称）
    let dims2 = require_outbound_dimensions("调拨出库", 3, Some("  "), None, Some("B7"), None)
        .expect("空白色号=白坯");
    assert_eq!(dims2.color_no, "");

    // 白坯给了缸号/匹号也不报错（维度多给不排除匹配宽松性；匹号 trim 保留不丢弃）
    let dims3 = require_outbound_dimensions(
        "调拨出库",
        3,
        None,
        Some("DYE-9"),
        Some("B7"),
        Some(" P-3 "),
    )
    .expect("白坯带缸号/匹号仍放行");
    assert_eq!(dims3.dye_lot_no.as_deref(), Some("DYE-9"));
    assert_eq!(dims3.piece_no.as_deref(), Some("P-3"));
}

/// 染色布四维齐全（缸/色/批/匹，用户 2026-10-02 纠正口径）→ 放行且四维原值归一
#[test]
fn dyed_fabric_with_all_four_dims_passes() {
    let dims = require_outbound_dimensions(
        "调拨出库",
        4,
        Some("COL-A"),
        Some("DYE-9"),
        Some("B7"),
        Some(" P-4 "),
    )
    .expect("四维齐全必须放行");
    assert_eq!(
        (
            dims.color_no.as_str(),
            dims.dye_lot_no.as_deref(),
            dims.batch_no.as_str(),
            dims.piece_no.as_deref(),
        ),
        ("COL-A", Some("DYE-9"), "B7", Some("P-4"))
    );
}

/// 染色布缺第四维匹号 → 同样 400 VALIDATION_ERROR 族（字段必填=VALIDATION 边界，
/// 2026-10-02 口径补强锁）
#[test]
fn dyed_fabric_missing_piece_no_returns_validation_error() {
    let err = require_outbound_dimensions(
        "调拨出库",
        7,
        Some("COL-A"),
        Some("DYE-9"),
        Some("B7"),
        None,
    )
    .expect_err("染色布缺匹号必须拒绝");
    assert!(matches!(err, AppError::ValidationErrorDisplayable(_)));
    assert_eq!(err.error_code(), "VALIDATION_ERROR");
    assert!(err.to_string().contains("匹号"), "实际: {err}");
}

/// 缺缸号错误的 HTTP 信封：400 + VALIDATION_ERROR + **真实缺维原因**（用户 2026-10-02 口径：
/// 提交被拒必须能看到缺哪一维；内部产品 ID 不外显，只进日志）
#[tokio::test]
async fn dim_error_http_envelope_is_400_not_500() {
    use axum::response::IntoResponse;

    let err =
        require_outbound_dimensions("调拨出库", 7, Some("COL-A"), None, Some("B7"), Some("P-7"))
            .unwrap_err();
    let resp = err.into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::BAD_REQUEST);
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["code"], "VALIDATION_ERROR");
    let message = v["message"].as_str().unwrap_or_default();
    assert_ne!(
        message, "请求参数验证失败",
        "缺维拒绝不得再被脱敏成固定常量（用户看不到缺哪一维）"
    );
    assert!(
        message.contains("缸号"),
        "出参 message 须携带真实缺维原因，实际: {message}"
    );
    assert!(!message.contains("款号产品"), "实际: {message}");
    assert_ne!(v["code"], "INTERNAL_ERROR");
    assert_ne!(v["code"], "DATABASE_ERROR");
}

// =========================================================
// B) 调拨请求 DTO serde（无 DB）
// =========================================================

/// 调拨明细三维为 Option：缺键解码通过（建单是否放行由服务四维判定，非 serde 层）
#[test]
fn transfer_item_request_dims_optional() {
    let item: InventoryTransferItemRequest = serde_json::from_value(json!({
        "product_id": 5, "quantity": "10.00", "batch_no": "B7"
    }))
    .unwrap();
    assert_eq!(item.product_id, Some(5));
    assert_eq!(item.quantity, Some(dec("10.00")));
    assert!(item.color_no.is_none() && item.dye_lot_no.is_none());
    assert_eq!(item.batch_no.as_deref(), Some("B7"));

    let req: CreateInventoryTransferRequest = serde_json::from_value(json!({
        "from_warehouse_id": 1, "to_warehouse_id": 2,
        "items": [
            { "product_id": 5, "quantity": "10.00", "color_no": "COL-A", "batch_no": "B7" }
        ]
    }))
    .unwrap();
    let items = req.items.unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].color_no.as_deref(), Some("COL-A"));
    assert!(
        items[0].dye_lot_no.is_none(),
        "染色布缺缸号在解码层不拒（服务层拒，锁 A 组用例）"
    );
}

// =========================================================
// C) 活库全链（HTTP create_transfer）：#[ignore]
// =========================================================

/// 已迁移 PG 上走真实 handler：
/// ① 染色布缺缸号建单 → 400 VALIDATION_ERROR，且调拨单事务整体回滚（不残留单据、库存不动）；
/// ② 同料白坯（免缸号宽松）→ 200 且单据落库 status=pending。
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL：单号生成 advisory_xact_lock + 频率检查 Postgres 方言原生 SQL"]
async fn live_create_transfer_dyed_missing_dye_lot_400_and_rollback() {
    use axum::{
        Router,
        body::Body,
        extract::State,
        http::{Method, Request, StatusCode},
        middleware::{Next, from_fn_with_state},
        response::Response,
        routing::post,
    };
    use bingxi_backend::container::AppState;
    use bingxi_backend::handlers::inventory_transfer_handler;
    use bingxi_backend::middleware::auth_context::AuthContext;
    use bingxi_backend::models::{inventory_stock, inventory_transfer, product, warehouse};
    use chrono::Utc;
    use sea_orm::{
        ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
    };
    use std::sync::Arc;
    use tower::ServiceExt;

    async fn inject_auth(
        State(auth): State<AuthContext>,
        mut request: Request<Body>,
        next: Next,
    ) -> Response {
        request.extensions_mut().insert(auth);
        next.run(request).await
    }

    let db = Arc::new(test_common::setup_test_db().await);
    let state = AppState {
        db: db.clone(),
        ..Default::default()
    };
    let app = Router::new()
        .route(
            "/inventory/transfers",
            post(inventory_transfer_handler::create_transfer),
        )
        .with_state(state)
        .layer(from_fn_with_state(
            AuthContext {
                user_id: 9501,
                username: "e2e_purchaser_9501".to_string(),
                role_id: Some(2),
                department_id: Some(1),
                data_scope: Some("self".to_string()),
                dept_ids: None,
                dept_member_user_ids: None,
            },
            inject_auth,
        ));

    // —— 播种：产品/两仓/白坯库存行（数量 50）——
    let p = product::ActiveModel {
        name: Set("四维套件坯布".to_string()),
        code: Set(format!("PRD-TRF-{}", Utc::now().timestamp_nanos_opt().expect("测试造数取当前时刻纳秒：Utc::now 必落在 chrono 纳秒可表示区间（约1678-2262 年），None 不可达；旧 timestamp_nanos 超界同样 panic，行为等价"))),
        unit: Set("米".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("fabric".to_string()),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(&*db)
    .await
    .unwrap();
    let mut wh_ids = Vec::new();
    for (tag, name) in [("F", "调出仓"), ("T", "调入仓")] {
        let wh = warehouse::ActiveModel {
            warehouse_code: Set(format!("WH-TRF-{tag}-{}", Utc::now().timestamp_nanos_opt().expect("测试造数取当前时刻纳秒：Utc::now 必落在 chrono 纳秒可表示区间（约1678-2262 年），None 不可达；旧 timestamp_nanos 超界同样 panic，行为等价"))),
            name: Set(name.to_string()),
            is_default: Set(false),
            is_active: Set(true),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(&*db)
        .await
        .unwrap();
        wh_ids.push(wh.id);
    }
    let stock = inventory_stock::ActiveModel {
        warehouse_id: Set(wh_ids[0]),
        product_id: Set(p.id),
        quantity_on_hand: Set(dec("50.00")),
        quantity_available: Set(dec("50.00")),
        quantity_reserved: Set(Decimal::ZERO),
        quantity_shipped: Set(Decimal::ZERO),
        quantity_incoming: Set(Decimal::ZERO),
        reorder_point: Set(Decimal::ZERO),
        max_stock_point: Set(Decimal::ZERO),
        reorder_quantity: Set(Decimal::ZERO),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        batch_no: Set("B7".to_string()),
        color_no: Set("COL-A".to_string()),
        dye_lot_no: Set(Some("DYE-9".to_string())),
        grade: Set("一等品".to_string()),
        quantity_meters: Set(dec("50.00")),
        quantity_kg: Set(Decimal::ZERO),
        stock_status: Set("正常".to_string()),
        quality_status: Set("合格".to_string()),
        version: Set(0),
        replenishment_strategy: Set("reorder_point".to_string()),
        ..Default::default()
    }
    .insert(&*db)
    .await
    .unwrap();

    let count_before = inventory_transfer::Entity::find()
        .filter(inventory_transfer::Column::FromWarehouseId.eq(wh_ids[0]))
        .count(&*db)
        .await
        .unwrap();

    // —— ① 染色布缺缸号：期望 400 VALIDATION_ERROR ——
    let dyed_missing = json!({
        "from_warehouse_id": wh_ids[0],
        "to_warehouse_id": wh_ids[1],
        "items": [{ "product_id": p.id, "quantity": "10.00", "color_no": "COL-A", "batch_no": "B7" }]
    });
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/inventory/transfers")
                .header("content-type", "application/json")
                .body(Body::from(dyed_missing.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "染色布缺缸号必须 400，实际 {status}: {v}"
    );
    assert_eq!(v["code"], "VALIDATION_ERROR");
    assert_ne!(v["code"], "INTERNAL_ERROR");

    // 事务回滚：单据零残留
    let count_after = inventory_transfer::Entity::find()
        .filter(inventory_transfer::Column::FromWarehouseId.eq(wh_ids[0]))
        .count(&*db)
        .await
        .unwrap();
    assert_eq!(
        count_before, count_after,
        "校验失败不得残留调拨单（txn 回滚）"
    );
    let stock_after = inventory_stock::Entity::find_by_id(stock.id)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        stock_after.quantity_meters,
        dec("50.00"),
        "失败路径不得动库存"
    );

    // —— ② 对照：白坯（无色号）免缸号宽松放行 ——
    let white_body = json!({
        "from_warehouse_id": wh_ids[0],
        "to_warehouse_id": wh_ids[1],
        "items": [{ "product_id": p.id, "quantity": "5.00", "batch_no": "B7" }]
    });
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/inventory/transfers")
                .header("content-type", "application/json")
                .body(Body::from(white_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(
        status.is_success(),
        "白坯免缸号必须放行（防过度收紧回归），实际 {status}: {v}"
    );
    assert_eq!(
        v["data"]["status"], "pending",
        "调拨单初始态与写入方词表同源小写 pending"
    );
}
