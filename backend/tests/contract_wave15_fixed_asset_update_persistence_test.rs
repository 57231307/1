//! 固定资产更新持久化契约锁（连真库 PostgreSQL）。
//!
//! 锁定两条源码契约面（均走 `test_common::setup_test_db()` 真库 + 真实回读）：
//!
//! 1. **PUT 更新资产后逐字段回读值等于提交值**：
//!    - `fixed_asset_handler::update_asset` 接收 `UpdateAssetDto`（恰有 4 个业务列：
//!      asset_name / asset_category / specification / use_location），先 `.into()` 转
//!      ActiveModel，再对 DTO 中为 `Some` 的列逐字段显式 `Set()`；
//!    - SeaORM 仅写被 `Set` 的列，其余保持 `Unchanged`（不落库、不改值）；
//!    - 本锁以 HTTP PUT 真实驱动生产 handler（路由 `/fixed-assets/{id}` →
//!      `fixed_asset_handler::update_asset`），断言写入后逐字段回读等于提交值；
//!      若 handler 退回缺失逐列 `Set`（仅 `.into()` 后 update 或漏某列），该列回读
//!      等于种子值而非提交值，断言当场失败。
//!    调用方：测试经 axum `Router::oneshot` 命中 `fixed_asset_handler::update_asset`
//!      → `FixedAssetService::get_by_id` + `ActiveModel::update` 落库。
//!
//! 2. **DTO 中 None 字段保持原值（单层 Option null=不变更）**：
//!    - `fixed_asset_handler::update_asset` 仅对 `Some` 列 `Set`，`None` 列保持 Unchanged；
//!    - 本锁真实 PUT 仅携带 asset_name（其余三列缺省），断言 asset_name 更新、
//!      其余三列回读等于种子原值；若 handler 误把 None 也写入（Set(None)）则原值丢失，断言失败。
//!
//! 3. **折旧状态门负例**：
//!    - `FixedAssetService::validate_asset_for_depreciation` 门仅允许 `active`；
//!    - 处置（`DISPOSED`）资产计提折旧返回 400 BUSINESS_ERROR（文案可外显）；
//!    - 本锁真实调服务层 `depreciate`，断 `error_code()=="BUSINESS_ERROR"`
//!      且 HTTP 状态 `StatusCode::BAD_REQUEST`。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::put,
};
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

use bingxi_backend::container::AppState;
use bingxi_backend::handlers::fixed_asset_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::fixed_asset;
use bingxi_backend::models::status::fixed_asset as fa_status;
use bingxi_backend::services::fixed_asset_service::{CreateAssetRequest, FixedAssetService};
use bingxi_backend::utils::error::AppError;

use test_common::setup_test_db;

fn unique_tag() -> String {
    Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_default()
        .to_string()
}

/// 会话注入用 AuthContext：update_asset 仅读 auth.username 做日志，不读身份归属列，
/// 故只需保证结构体字段齐全（与生产 AuthContext 定义逐字段一致）。
fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("w15_fa_update_{user_id}"),
        role_id: Some(1),
        department_id: Some(1),
        data_scope: Some("all".to_string()),
        dept_ids: None,
        dept_member_user_ids: None,
    }
}

/// 中间件把会话身份塞进 request extensions，供 handler 的 `AuthContext` 抽取器读取。
async fn inject_auth(
    State(auth): State<AuthContext>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    request.extensions_mut().insert(auth);
    next.run(request).await
}

/// 组装带会话注入层的 Router，按生产挂载形状注册 PUT `/fixed-assets/{id}`
/// （`src/routes/finance.rs` 中 `put(fixed_asset_handler::update_asset)`），
/// 使测试请求命中生产 handler 本体而非在测试里重拼 ActiveModel Set。
fn layered_app(state: AppState) -> Router {
    Router::new()
        .route("/fixed-assets/{id}", put(fixed_asset_handler::update_asset))
        .with_state(state)
        .layer(from_fn_with_state(make_auth(1), inject_auth))
}

/// 以 JSON 请求体发一次 PUT，返回 (HTTP 状态码, 解析后的响应体)。
/// 状态码用于区分「路由/抽取器未命中 handler」与「持久化值不符」两类失败。
async fn call_put(app: &Router, path: &str, body: &Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(Method::PUT)
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

// =========================================================
// 测试 1：真调生产 handler 更新资产后逐字段回读等于提交值
// =========================================================

#[tokio::test]
async fn fixed_asset_update_persists_all_set_fields() {
    let db = setup_test_db().await;
    let arc_db = Arc::new(db);
    let state = AppState {
        db: Arc::clone(&arc_db),
        ..Default::default()
    };
    let app = layered_app(state);

    // 种子：经生产服务层创建一个活跃资产（fixture 数据准备，非被测目标）
    let svc = FixedAssetService::new(Arc::clone(&arc_db));
    let tag = unique_tag();
    let created = svc
        .create(
            CreateAssetRequest {
                asset_no: Some(format!("FA-UPD-{}", tag)),
                asset_name: Some("原始名称".to_string()),
                asset_category: Some("原始类别".to_string()),
                specification: Some("原始规格".to_string()),
                location: Some("原始地点".to_string()),
                original_value: Some(Decimal::new(100000, 0)),
                useful_life: Some(10),
                depreciation_method: Some("straight_line".to_string()),
                purchase_date: Some(chrono::NaiveDate::from_ymd_opt(2026, 1, 10).unwrap()),
                put_in_date: Some(chrono::NaiveDate::from_ymd_opt(2026, 1, 15).unwrap()),
                supplier_id: None,
            },
            1,
        )
        .await
        .expect("种子资产创建失败（需 TEST_DATABASE_URL 指向已 migrate 的库）");

    let asset_id = created.id;

    // 提交 DTO 全部 4 个业务列，命中生产 handler 本体（不再在测试里重拼 ActiveModel Set）
    let new_name = "E2E14资产改名".to_string();
    let new_category = "电子设备".to_string();
    let new_spec = "E2E-SPEC-200".to_string();
    let new_location = "二车间".to_string();
    let body = json!({
        "asset_name": new_name,
        "asset_category": new_category,
        "specification": new_spec,
        "use_location": new_location,
    });
    let (status, _resp) = call_put(&app, &format!("/fixed-assets/{asset_id}"), &body).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "PUT 必须命中生产 update_asset 并成功，实际 {status} {_resp}"
    );

    // 逐字段回读断言——等于提交值（从真库读，不信任 handler 返回值）
    let readback = svc.get_by_id(asset_id).await.expect("更新后回读失败");

    assert_eq!(
        readback.asset_name, new_name,
        "asset_name 回读值必须逐字等于提交值（handler 漏 Set 此列将保持种子值而失败）"
    );
    assert_eq!(
        readback.asset_category,
        Some(new_category),
        "asset_category 回读值必须逐字等于提交值（handler 漏 Set 此列将保持种子值而失败）"
    );
    assert_eq!(
        readback.specification,
        Some(new_spec),
        "specification 回读值必须逐字等于提交值（handler 漏 Set 此列将保持种子值而失败）"
    );
    assert_eq!(
        readback.use_location,
        Some(new_location),
        "use_location 回读值必须逐字等于提交值（handler 漏 Set 此列将保持种子值而失败）"
    );

    // 清理
    fixed_asset::Entity::delete_by_id(asset_id)
        .exec(&*arc_db)
        .await
        .ok();
}

// =========================================================
// 测试 2：真调生产 handler 仅提交 asset_name，其余 None 字段保持原值
// =========================================================

#[tokio::test]
async fn fixed_asset_update_none_fields_preserved() {
    let db = setup_test_db().await;
    let arc_db = Arc::new(db);
    let state = AppState {
        db: Arc::clone(&arc_db),
        ..Default::default()
    };
    let app = layered_app(state);

    let svc = FixedAssetService::new(Arc::clone(&arc_db));
    let tag = unique_tag();
    let created = svc
        .create(
            CreateAssetRequest {
                asset_no: Some(format!("FA-KEP-{}", tag)),
                asset_name: Some("保持名称".to_string()),
                asset_category: Some("保持类别".to_string()),
                specification: Some("保持规格".to_string()),
                location: Some("保持地点".to_string()),
                original_value: Some(Decimal::new(50000, 0)),
                useful_life: Some(5),
                depreciation_method: Some("straight_line".to_string()),
                purchase_date: Some(chrono::NaiveDate::from_ymd_opt(2025, 6, 1).unwrap()),
                put_in_date: Some(chrono::NaiveDate::from_ymd_opt(2025, 6, 1).unwrap()),
                supplier_id: None,
            },
            1,
        )
        .await
        .expect("种子资产创建失败");

    let asset_id = created.id;

    // 仅提交 asset_name，其余三列缺省（serde Option => None）——命中生产 handler，
    // handler 仅对 Some 列 Set，None 列保持 Unchanged 不写库
    let body = json!({ "asset_name": "仅改名" });
    let (status, _resp) = call_put(&app, &format!("/fixed-assets/{asset_id}"), &body).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "PUT 必须命中生产 update_asset 并成功，实际 {status} {_resp}"
    );

    let readback = svc.get_by_id(asset_id).await.expect("更新后回读失败");

    assert_eq!(readback.asset_name, "仅改名", "被 Set 的 asset_name 应更新");
    assert_eq!(
        readback.asset_category,
        Some("保持类别".to_string()),
        "未提交（handler 不 Set）的 asset_category 应保持原值"
    );
    assert_eq!(
        readback.specification,
        Some("保持规格".to_string()),
        "未提交（handler 不 Set）的 specification 应保持原值"
    );
    assert_eq!(
        readback.use_location,
        Some("保持地点".to_string()),
        "未提交（handler 不 Set）的 use_location 应保持原值"
    );

    fixed_asset::Entity::delete_by_id(asset_id)
        .exec(&*arc_db)
        .await
        .ok();
}

// =========================================================
// 测试 3：非 active 资产计提折旧被状态门拒绝（400 BUSINESS_ERROR）
// =========================================================

#[tokio::test]
async fn depreciate_disposed_asset_returns_business_error() {
    let db = setup_test_db().await;
    let arc_db = Arc::new(db);

    let svc = FixedAssetService::new(Arc::clone(&arc_db));

    let tag = unique_tag();
    let created = svc
        .create(
            CreateAssetRequest {
                asset_no: Some(format!("FA-GATE-{}", tag)),
                asset_name: Some("门测试资产".to_string()),
                asset_category: Some("机器设备".to_string()),
                specification: None,
                location: Some("仓库".to_string()),
                original_value: Some(Decimal::new(120000, 0)),
                useful_life: Some(10),
                depreciation_method: Some("straight_line".to_string()),
                purchase_date: Some(chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
                put_in_date: Some(chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
                supplier_id: None,
            },
            1,
        )
        .await
        .expect("种子资产创建失败");

    let asset_id = created.id;

    // 手动将状态置为 disposed（模拟处置完成）
    let asset = svc.get_by_id(asset_id).await.expect("回读资产失败");
    let mut active_model: fixed_asset::ActiveModel = asset.into();
    active_model.status = Set(fa_status::DISPOSED.to_string());
    active_model.updated_at = Set(Utc::now());
    active_model
        .update(&*arc_db)
        .await
        .expect("置为 disposed 失败");

    // 调用 depreciate —— 必须被状态门拒绝
    let err = svc
        .depreciate(asset_id, "2099-06", 1)
        .await
        .expect_err("disposed 资产计提折旧必须被状态门拒绝，不应 Ok");

    // 断言：error_code == BUSINESS_ERROR（非 VALIDATION/INTERNAL）
    assert_eq!(
        err.error_code(),
        "BUSINESS_ERROR",
        "状态门必须归业务族 BUSINESS_ERROR，实际={:?}",
        err,
    );

    // 断言：变体为 BusinessErrorDisplayable（文案可外显，非脱敏族）
    assert!(
        matches!(err, AppError::BusinessErrorDisplayable(_)),
        "状态门文案必须可外显（business_displayable），实际={:?}",
        err,
    );

    // 断言：HTTP 状态码 400（非 500）
    assert_eq!(
        <AppError as axum::response::IntoResponse>::into_response(err.clone()).status(),
        axum::http::StatusCode::BAD_REQUEST,
        "状态门拒绝必须返回 HTTP 400"
    );

    // 断言：外显文案等于服务源码
    assert_eq!(
        err.to_response().message,
        "只有活跃状态的资产才能计提折旧",
        "出参必须外显真实拒绝原因"
    );

    // 断言：文案不含记录 ID（脱敏红线）
    assert!(
        !err.to_response().message.contains(&asset_id.to_string()),
        "用户可见文案禁带记录 ID（脱敏红线），实际={}",
        err.to_response().message,
    );

    // 清理：删除测试资产（disposed 态可删）
    svc.delete(asset_id, 1).await.ok();
}
