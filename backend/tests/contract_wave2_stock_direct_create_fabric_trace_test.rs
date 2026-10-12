//! 库存直建两端点白坯/染色口径锁（contract wave2）
//!
//! 锁定的 file:line 契约：
//! - `backend/src/services/inv/fabric_class.rs:36-63`（唯一权威判定：批次必填；
//!   色号为空=白坯免缸号、色号非空=染色布缸号必填；trim 归一；禁止按色号文本嗅探）
//! - `backend/src/handlers/inventory_stock_handler_fabric.rs`（`admit_stock_fabric_trace`：
//!   两条直建端点共用收口——拒绝文案 business_displayable 外显 + 白坯主动把缸号归一为 None，
//!   与出库规划 `services/inventory_deduction.rs` 白坯只接受 `dye_lot_no IS NULL` 行同源）
//! - `backend/src/handlers/inventory_stock_handler.rs::create_stock`（POST /inventory/stock：
//!   修复前从未 payload.validate() 且绕开 trace 判定，本文件用源码契约锁防止回归）
//! - `backend/src/handlers/inventory_stock_handler_dto.rs::CreateStockFabricRequest`
//!   （color_no 放开为 Option：白坯允许 None/空串；联动必填不在 DTO 写第二套规则）
//!
//! 覆盖策略：
//! - A/B 组为纯函数与 serde 断言（**无需任何 DB**）
//! - C 组 `#[ignore]` 活库用例：两个真实 handler 在同一入参下结论一致——
//!   白坯空色号可入库且落库 dye_lot_no 为 NULL；染色布缺缸号被 400 BUSINESS_ERROR
//!   拒绝且真实文案外显（非"业务处理失败"脱敏常量）、零脏行落库。

mod test_common;

use rust_decimal::Decimal;
use serde_json::json;
use std::str::FromStr;
use validator::Validate;

use bingxi_backend::handlers::inventory_stock_handler_dto::CreateStockFabricRequest;
use bingxi_backend::handlers::inventory_stock_handler_fabric::admit_stock_fabric_trace;
use bingxi_backend::utils::error::AppError;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn req_json(v: serde_json::Value) -> CreateStockFabricRequest {
    let base = json!({
        "warehouse_id": 1,
        "product_id": 100,
        "batch_no": "B20260920",
        "color_no": "C001",
        "dye_lot_no": "D001",
        "grade": "一等品",
        "quantity_meters": "10.00",
    });
    let mut obj = base.as_object().unwrap().clone();
    if let Some(o) = v.as_object() {
        for (k, val) in o {
            obj.insert(k.clone(), val.clone());
        }
    }
    serde_json::from_value(serde_json::Value::Object(obj)).expect("夹具应可反序列化")
}

// =========================================================
// A) 白坯/染色准入收口纯函数矩阵（无 DB）
// =========================================================

/// 白坯（色号 None / 空串 / 纯空白）放行，且**主动把缸号归一为 None**
/// （防止"白坯带缸号"入库后出库规划 dye_lot_no IS NULL 过滤永久提不出来）
#[test]
fn white_fabric_passes_and_dye_lot_normalized_to_none() {
    for color in [None, Some("".to_string()), Some("   ".to_string())] {
        let trace = admit_stock_fabric_trace(
            color.clone(),
            Some("D9".to_string()),
            Some("B1".to_string()),
        )
        .expect("白坯空色号必须放行（权威口径：色号为空=白坯免缸号）");
        assert_eq!(trace.color_no, "", "白坯归一为空串（落 NOT NULL 空串行）");
        assert_eq!(
            trace.dye_lot_no, None,
            "白坯带缸号入参必须归一为 None，实际: {:?}",
            trace.dye_lot_no
        );
        assert_eq!(trace.batch_no, "B1");
    }
}

/// 染色布（色号非空）缺缸号 → business_displayable 拒绝：code=BUSINESS_ERROR、
/// Display 携带权威原因与用户自己提交的色号（满足外显安全边界）
#[test]
fn dyed_fabric_missing_dye_lot_rejected_displayable() {
    let err = admit_stock_fabric_trace(Some("COL-A".to_string()), None, Some("B7".to_string()))
        .expect_err("染色布缺缸号必须拒绝");
    assert!(
        matches!(err, AppError::BusinessErrorDisplayable(_)),
        "必须是可外显业务错（用户看得见原因），实际: {err:?}"
    );
    assert_eq!(err.error_code(), "BUSINESS_ERROR");
    let disp = err.to_string();
    assert!(disp.contains("染色布必须提供缸号"), "实际: {disp}");
    assert!(
        disp.contains("COL-A"),
        "原因须回显用户提交的色号，实际: {disp}"
    );
}

/// 批次任何布种必填（白坯也一样）；纯空格批次不得蒙混
#[test]
fn missing_batch_rejected_for_both_kinds() {
    let err =
        admit_stock_fabric_trace(Some("".to_string()), None, None).expect_err("白坯缺批次必须拒绝");
    assert!(err.to_string().contains("批次不得为空"), "实际: {err}");

    let err2 = admit_stock_fabric_trace(
        Some("COL-A".to_string()),
        Some("D9".to_string()),
        Some("  ".to_string()),
    )
    .expect_err("空白批次必须拒绝");
    assert!(err2.to_string().contains("批次不得为空"));
}

/// 四维齐全（染色布）与 trim 归一：与权威判定行为逐字一致（本收口不得改写判定）
#[test]
fn dyed_fabric_full_dims_pass_with_trim_normalization() {
    let trace = admit_stock_fabric_trace(
        Some(" COL-A ".to_string()),
        Some(" D9 ".to_string()),
        Some(" B7 ".to_string()),
    )
    .expect("四维齐全必须放行");
    assert_eq!(trace.color_no, "COL-A");
    assert_eq!(trace.dye_lot_no.as_deref(), Some("D9"));
    assert_eq!(trace.batch_no, "B7");
}

/// 禁止按色号文本嗅探布种：名字带"白"的色号仍是染色布，缺缸号必拒（与权威单测同源）
#[test]
fn white_named_color_still_requires_dye_lot() {
    for name in ["本白", "白色", "白坯", "WHITE"] {
        let err = admit_stock_fabric_trace(Some(name.to_string()), None, Some("B1".to_string()))
            .expect_err(&format!("色号 {name} 是染色布，缺缸号必须报错"));
        assert!(err.to_string().contains("缸号"), "色号 {name} 实际: {err}");
    }
}

/// HTTP 信封：拒绝为 400 + BUSINESS_ERROR 且 message 外显真实文案（非脱敏常量）
#[tokio::test]
async fn rejection_envelope_is_400_business_error_with_visible_message() {
    use axum::response::IntoResponse;

    let err = admit_stock_fabric_trace(Some("COL-A".to_string()), None, Some("B7".to_string()))
        .unwrap_err();
    let resp = err.into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::BAD_REQUEST);
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["code"], "BUSINESS_ERROR");
    assert_ne!(v["code"], "INTERNAL_ERROR", "不得强转 500");
    assert_eq!(
        v["message"], "染色布必须提供缸号（color_no=COL-A 但 dye_lot_no 为空）",
        "business_displayable 出参必须携带真实文案（business 族会被脱敏成'业务处理失败'）"
    );
}

// =========================================================
// B) DTO 可空语义（无 DB）
// =========================================================

/// color_no 为 Option：null / 缺键均按白坯解码通过（修复前 length(min=1) 把合法白坯 400 拒）
#[test]
fn dto_accepts_null_and_missing_color_no() {
    let r = req_json(json!({"color_no": null, "dye_lot_no": null}));
    assert!(r.color_no.is_none() && r.dye_lot_no.is_none());
    r.validate().expect("白坯（无色号无缸号）DTO 校验必须放行");

    // 缺键解码同样合法（serde Option 语义），不再有必填反序列化失败
    let r2: CreateStockFabricRequest = serde_json::from_value(json!({
        "warehouse_id": 1, "product_id": 100, "batch_no": "B20260920",
        "grade": "一等品", "quantity_meters": "10.00",
    }))
    .expect("缺 color_no/dye_lot_no 键必须可解码（白坯）");
    assert!(r2.color_no.is_none() && r2.dye_lot_no.is_none());
    r2.validate().expect("白坯缺键 DTO 校验必须放行");
}

/// DTO 只锁格式（长度/ID>0/批次非空），不写色号-缸号联动规则（判定唯一权威在 fabric_class）：
/// 染色布缺缸号在 DTO 层放行、由准入收口拒绝——两端点对同一入参结论一致的结构性保证
#[test]
fn dto_does_not_encode_second_authority_for_dims() {
    // 有色号无缸号：DTO 校验通过（联动必填由 admit_stock_fabric_trace 拒，见 A 组）
    let r = req_json(json!({"dye_lot_no": null}));
    r.validate().expect("联动规则不在 DTO");
    let err = admit_stock_fabric_trace(r.color_no, r.dye_lot_no, Some(r.batch_no))
        .expect_err("同一入参在收口层必须被拒");
    assert!(matches!(err, AppError::BusinessErrorDisplayable(_)));

    // 空批次：DTO 与收口双层都拒（格式规则属 DTO）
    let bad = req_json(json!({"batch_no": ""}));
    assert!(bad.validate().is_err());
}

/// 超长仍拒（放开 min 不影响 max=50 格式约束）
#[test]
fn dto_keeps_max_length_constraints() {
    let long = "C".repeat(51);
    let r = req_json(json!({"color_no": long, "dye_lot_no": long.clone()}));
    let errs = r.validate().unwrap_err();
    let msg = errs.to_string();
    assert!(msg.contains("色号"), "实际: {msg}");
    assert!(msg.contains("缸号"), "实际: {msg}");
}

// =========================================================
// C) 源码契约锁：两条路径不得再绕开统一口径（无 DB）
// =========================================================

/// POST /inventory/stock 修复前从未 validate() 也未走 trace 判定；锁死收敛后的源码结构，
/// 防止再次出现"预检绿、执行红"的第二条裸写路径
#[test]
fn both_direct_create_paths_delegate_to_single_authority() {
    let general = include_str!("../src/handlers/inventory_stock_handler.rs");
    let fabric = include_str!("../src/handlers/inventory_stock_handler_fabric.rs");

    let general_create = general
        .split("pub async fn create_stock(")
        .nth(1)
        .and_then(|s| s.split("pub async fn ").next())
        .expect("create_stock 函数体应存在");
    assert!(
        general_create.contains("payload.validate()") || general_create.contains(".validate()",),
        "POST /inventory/stock 必须走 DTO 校验"
    );
    assert!(
        general_create.contains("admit_stock_fabric_trace"),
        "POST /inventory/stock 必须经统一准入收口，不得四维裸写落库"
    );

    let fabric_create = fabric
        .split("pub async fn create_stock_fabric(")
        .nth(1)
        .and_then(|s| s.split("pub fn admit_stock_fabric_trace").next())
        .expect("create_stock_fabric 函数体应存在");
    assert!(
        fabric_create.contains("admit_stock_fabric_trace"),
        "POST /inventory/stock/fabric 必须经同一收口"
    );
    assert!(
        !fabric_create.contains("AppError::internal(e.to_string())"),
        "handler 结果已是 AppError，禁止 .map_err(internal) 强转 500，实际: {fabric_create}"
    );
    // 判定唯一来源仍是 fabric_class；收口文件内不得出现第二套色号/缸号联动 if 规则
    assert!(
        fabric.contains("fabric_class::validate_fabric_trace"),
        "收口必须委托权威实现，不得自写判定"
    );
}

// =========================================================
// D) 活库全链（两个真实 handler 同入参对照）：#[ignore]
// =========================================================

/// 已迁移 PG 上走真实 handler：
/// ① 白坯（color_no 空串）带缸号入参 → 两端点均 200，落库 color_no=""、dye_lot_no IS NULL；
/// ② 染色布缺缸号 → 两端点均 400 BUSINESS_ERROR、真实文案外显、零脏行落库；
/// ③ 同一入参两路径结论一致（HTTP 状态逐对照）。
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL（service 层校验仓库/产品存在性 + 真实 insert）"]
async fn live_both_endpoints_consistent_on_fabric_trace() {
    use axum::{
        Router,
        body::Body,
        extract::State,
        http::{Method, Request, StatusCode},
        routing::post,
    };
    use bingxi_backend::container::AppState;
    use bingxi_backend::handlers::{inventory_stock_handler, inventory_stock_handler_fabric};
    use bingxi_backend::middleware::auth_context::AuthContext;
    use bingxi_backend::models::{inventory_stock, product, warehouse};
    use chrono::Utc;
    use sea_orm::{
        ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
    };
    use std::sync::Arc;
    use tower::ServiceExt;

    async fn inject_auth(
        State(auth): State<AuthContext>,
        mut request: Request<Body>,
        next: axum::middleware::Next,
    ) -> axum::response::Response {
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
            "/inventory/stock",
            post(inventory_stock_handler::create_stock),
        )
        .route(
            "/inventory/stock/fabric",
            post(inventory_stock_handler_fabric::create_stock_fabric),
        )
        .with_state(state)
        .layer(axum::middleware::from_fn_with_state(
            AuthContext {
                user_id: 9601,
                username: "e2e_stock_9601".to_string(),
                role_id: Some(2),
                department_id: Some(1),
                data_scope: Some("self".to_string()),
                dept_ids: None,
                dept_member_user_ids: None,
            },
            inject_auth,
        ));

    // —— 播种产品/仓库 ——
    let ts = Utc::now().timestamp_nanos_opt().expect("测试造数取当前时刻纳秒：Utc::now 必落在 chrono 纳秒可表示区间（约1678-2262 年），None 不可达；旧 timestamp_nanos 超界同样 panic，行为等价");
    let p = product::ActiveModel {
        name: Set("wave2直建套件坯布".to_string()),
        code: Set(format!("PRD-W2-{ts}")),
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
    let wh = warehouse::ActiveModel {
        warehouse_code: Set(format!("WH-W2-{ts}")),
        name: Set("直建仓".to_string()),
        is_default: Set(false),
        is_active: Set(true),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(&*db)
    .await
    .unwrap();

    async fn post_json(
        app: &Router,
        uri: &str,
        body: &serde_json::Value,
    ) -> (StatusCode, serde_json::Value) {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(uri)
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let v = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
        (status, v)
    }

    let white_body = json!({
        "warehouse_id": wh.id, "product_id": p.id,
        "batch_no": "B-W2-WHITE", "color_no": "", "dye_lot_no": "D-误传",
        "grade": "一等品", "quantity_meters": "10.00", "quantity_kg": "5.00",
    });
    let dyed_missing_body = json!({
        "warehouse_id": wh.id, "product_id": p.id,
        "batch_no": "B-W2-DYED", "color_no": "COL-A", "grade": "一等品",
        "quantity_meters": "10.00", "quantity_kg": "5.00",
    });

    for (path, label) in [
        ("/inventory/stock", "POST /inventory/stock"),
        ("/inventory/stock/fabric", "POST /inventory/stock/fabric"),
    ] {
        // —— ① 白坯：放行且缸号归 None ——
        let (status, v) = post_json(&app, path, &white_body).await;
        assert!(
            status.is_success(),
            "{label} 白坯空色号必须放行，实际 {status}: {v}"
        );
        assert_eq!(v["data"]["color_no"], "", "{label} 白坯落库色号应为空串");
        assert_eq!(
            v["data"]["dye_lot_no"],
            serde_json::Value::Null,
            "{label} 白坯缸号必须归一为 None（防永久提不出库的死行），实际: {v}"
        );

        // —— ② 染色布缺缸号：400 BUSINESS_ERROR + 真实文案外显、零脏行 ——
        let (status, v) = post_json(&app, path, &dyed_missing_body).await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "{label} 染色布缺缸号必须 400"
        );
        assert_eq!(v["code"], "BUSINESS_ERROR");
        assert_ne!(v["code"], "INTERNAL_ERROR", "{label} 不得强转 500");
        assert!(
            v["message"]
                .as_str()
                .unwrap_or("")
                .contains("染色布必须提供缸号"),
            "{label} 拒绝原因必须对用户可见（非脱敏常量），实际: {v}"
        );
        let dirty = inventory_stock::Entity::find()
            .filter(inventory_stock::Column::ProductId.eq(p.id))
            .filter(inventory_stock::Column::BatchNo.eq("B-W2-DYED"))
            .count(&*db)
            .await
            .unwrap();
        assert_eq!(dirty, 0, "{label} 拒绝路径不得落脏行");
    }

    // —— ③ 两路径同入参结论一致已由上面逐路径断言覆盖；
    //    白坯行实际落库且可被出库规划的四维全等匹配命中 ——
    let white_rows = inventory_stock::Entity::find()
        .filter(inventory_stock::Column::ProductId.eq(p.id))
        .filter(inventory_stock::Column::BatchNo.eq("B-W2-WHITE"))
        .all(&*db)
        .await
        .unwrap();
    assert_eq!(white_rows.len(), 2, "两端点各成一行白坯库存");
    for row in white_rows {
        assert_eq!(row.color_no, "");
        assert!(row.dye_lot_no.is_none(), "落库行 dye_lot_no 必须为 NULL");
        assert_eq!(
            row.quantity_meters,
            dec("10.00"),
            "数量原样落库不被校验路径改写"
        );
    }
}
