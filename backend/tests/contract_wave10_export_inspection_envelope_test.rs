//! 出口商检端点统一响应信封形状锁（本波契约：裸 JSON → 标准 ApiResponse 信封）
//!
//! 锁定的 file:line 契约：
//! - `backend/src/handlers/export_inspection_handler.rs::list_inspections` 返回
//!   `Json<ApiResponse<serde_json::Value>>`，data 为
//!   `PaginatedResponse{items,total,page,page_size}`（与 handlers/purchase_inspection_handler.rs 同形）。
//! - `backend/src/handlers/export_inspection_handler.rs::get_inspection` 返回
//!   `Json<ApiResponse<serde_json::Value>>`，data 为 `export_inspection::Model` 序列化对象。
//! - `backend/src/utils/response.rs:16-26`（ApiResponse 成功出参顶层仅 code/data，message/total 为 None 被 skip）
//!   与 `response.rs:39-45`（PaginatedResponse 字段恰为 items/total/page/page_size）。
//!
//! 回潮判据（对应 CI run #4675 e2e flow 27 片 shard8 红点）：本波前这两个 handler 直接返回裸
//! `{items,total}` / 裸实体对象，顶层没有 `code`，前端 strict 探针 `apiCall`（helpers.ts:1478）
//! 见 `json.code !== 200` 而把 HTTP 200 判成「请求异常」。故本锁同时钉住：
//!   1) 顶层键集合恰为 {code,data}（回退裸 {items,total} → 顶层变 [items,total]，判红）；
//!   2) 列表 data 键集合恰为 {items,total,page,page_size} 且 items 为数组（回退到把 items 摊到
//!      顶层或裸数组，判红）；
//!   3) 详情 data 是实体对象（含 id），且顶层不是裸实体（回退裸 `json!(item)` → 顶层无 code/data，判红）。
//! 只钉形状，不锁任何中文/英文文案，也不锁精确条数（避免夹具残留导致脆断）。
//!
//! 覆盖策略：
//! - 纯 serde 形状断言（无任何 DB）：锁 ApiResponse::success(to_value(PaginatedResponse::new(..))) 的键集合；
//! - 真 PostgreSQL（TEST_DATABASE_URL，夹具清空业务表后由迁移保证 schema）：
//!   走真实 handler 的 HTTP 形状断言（读路径仅 SELECT/按 id 查，无 advisory lock）。

mod test_common;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    response::Response,
    routing::get,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::export_inspection_handler;
use bingxi_backend::models::export_inspection;
use bingxi_backend::utils::response::{ApiResponse, PaginatedResponse};
use chrono::{NaiveDate, Utc};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection};
use serde_json::Value;
use tower::ServiceExt;

/// 响应 JSON 键集合（排序后），用于「恰为」断言
fn sorted_keys(obj: &Value) -> Vec<String> {
    let mut ks: Vec<String> = obj
        .as_object()
        .unwrap_or_else(|| panic!("期望 JSON 对象，实际: {}", obj))
        .keys()
        .cloned()
        .collect();
    ks.sort();
    ks
}

/// 组装 GET /export-inspections 与 /export-inspections/{id} → 真实 handler。
/// 两 handler 均不注入 AuthContext，故无需中间件层；路径形态与 routes 挂载后一致。
fn build_app(state: AppState) -> Router {
    Router::new()
        .route(
            "/export-inspections",
            get(export_inspection_handler::list_inspections),
        )
        .route(
            "/export-inspections/{id}",
            get(export_inspection_handler::get_inspection),
        )
        .with_state(state)
}

async fn get_json(app: &Router, uri: &str) -> (StatusCode, Value) {
    let resp: Response = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

/// 播种一行出口商检记录，返回自增主键 id（夹具已清空业务表，首行为 1，但显式取回更稳）。
async fn seed_one_inspection(db: &DatabaseConnection) -> i32 {
    let created = export_inspection::ActiveModel {
        inspection_no: Set("EI-ENVELOPE-LOCK-0001".to_string()),
        sales_order_id: Set(1),
        product_name: Set("envelope-lock-fabric".to_string()),
        hs_code: Set("5407.10".to_string()),
        inspection_type: Set("TYPE-A".to_string()),
        inspection_agency: Set("CIQ".to_string()),
        inspection_date: Set(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
        result: Set("pending".to_string()),
        created_by: Set(1),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    created.id
}

fn state_from(db: DatabaseConnection) -> AppState {
    AppState {
        db: std::sync::Arc::new(db),
        ..Default::default()
    }
}

fn json_row(id: u64) -> serde_json::Value {
    serde_json::json!({ "id": id })
}

// =========================================================
// 纯 serde 形状断言（无需任何 DB）
// =========================================================

/// 与 handler 完全同构的成功出参：顶层恰 {code,data}，data 恰 {items,total,page,page_size}。
/// 回归锁：本波前 handler 直接 `Json(json!({items,total}))`，顶层没有 code/data。
#[test]
fn export_list_envelope_shape_via_success_and_paginated_to_value() {
    let inner = serde_json::to_value(PaginatedResponse::new(
        vec![json_row(1), json_row(2)],
        25,
        2,
        10,
    ))
    .unwrap();
    let resp: ApiResponse<Value> = ApiResponse::success(inner);
    let v = serde_json::to_value(&resp).unwrap();

    assert_eq!(
        sorted_keys(&v),
        vec!["code", "data"],
        "顶层键恰为 code/data（回退裸 {{items,total}} 判红）"
    );
    assert_eq!(v["code"], serde_json::json!(200));
    assert!(
        v["data"].is_object(),
        "data 必须是分页对象（旧裸数组/摊平形状判红）"
    );
    assert_eq!(
        sorted_keys(&v["data"]),
        vec!["items", "page", "page_size", "total"],
        "分页 data 键集合必须恰好为 {{items,total,page,page_size}}，多一少一皆判红"
    );
    assert!(v["data"]["items"].is_array());
    assert_eq!(v["data"]["total"], 25u64);
}

/// 详情同构出参：顶层 {code,data}，data 是实体对象（含 id，非列表键集合）。
#[test]
fn export_detail_envelope_shape_via_success() {
    let entity = serde_json::json!({ "id": 1, "inspection_no": "EI-1", "result": "pending" });
    let resp: ApiResponse<Value> = ApiResponse::success(entity);
    let v = serde_json::to_value(&resp).unwrap();

    assert_eq!(
        sorted_keys(&v),
        vec!["code", "data"],
        "顶层键恰为 code/data（回退裸 json!(item) 判红）"
    );
    assert_eq!(v["code"], serde_json::json!(200));
    assert!(v["data"].is_object(), "详情 data 必须是对象");
    assert_eq!(v["data"]["id"], 1, "详情 data 承载实体本身（含 id）");
}

// =========================================================
// HTTP 层真实 handler 形状（真 PostgreSQL，迁移建表，无需自建 DDL）
// =========================================================

/// 列表端点真实出参：2xx + 顶层 {code,data} + data 四键 + items 为数组。
#[tokio::test]
async fn list_inspections_returns_unified_envelope_shape() {
    let db = test_common::setup_test_db().await;
    seed_one_inspection(&db).await;
    let app = build_app(state_from(db));

    let (status, v) = get_json(&app, "/export-inspections?page=1&page_size=5").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        sorted_keys(&v),
        vec!["code", "data"],
        "列表 handler 出参顶层必须恰为 code/data（回潮裸形状判红）"
    );
    assert_eq!(
        v["code"],
        serde_json::json!(200),
        "成功信封 code 必须为数字 200（前端 strict 探针据此判健康）"
    );
    assert_eq!(
        sorted_keys(&v["data"]),
        vec!["items", "page", "page_size", "total"],
        "列表 data 必须是分页四键对象"
    );
    let items = v["data"]["items"]
        .as_array()
        .unwrap_or_else(|| panic!("data.items 必须是数组，实际: {}", v["data"]["items"]));
    assert!(
        !items.is_empty(),
        "播种一行后 items 不应为空（否则取数链路断了）"
    );
    assert!(
        v["data"]["total"].is_number(),
        "total 必须是数字（非字符串/缺失）"
    );
}

/// 详情端点真实出参：2xx + 顶层 {code,data} + data 是实体对象（含 id，等于播种主键）。
#[tokio::test]
async fn get_inspection_returns_unified_envelope_shape() {
    let db = test_common::setup_test_db().await;
    let seeded_id = seed_one_inspection(&db).await;
    let app = build_app(state_from(db));

    let (status, v) = get_json(&app, &format!("/export-inspections/{seeded_id}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        sorted_keys(&v),
        vec!["code", "data"],
        "详情 handler 出参顶层必须恰为 code/data（回潮裸 json!(item) 判红）"
    );
    assert_eq!(v["code"], serde_json::json!(200));
    assert!(
        v["data"].is_object(),
        "详情 data 必须是实体对象，实际: {}",
        v["data"]
    );
    assert_eq!(
        v["data"]["id"].as_i64(),
        Some(seeded_id as i64),
        "详情 data.id 必须回显被查实体"
    );
}
