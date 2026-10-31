//! 坯布档案删除门的可撤回性契约锁（连已迁移的 PostgreSQL 活测）
//!
//! 契约对象是 `backend/src/handlers/greige_fabric_handler.rs` 的 `delete_greige_fabric`
//! 及其两道门 `find_greige_reference_rejection` 与 `remaining_stock_rejection`，路由腿
//! `DELETE /api/v1/erp/production/greige-fabrics/{id}` 在册于
//! `backend/src/routes/production.rs` 的 `greige_fabrics`。
//!
//! 为什么要有本文件。删除端点、前端删除按钮与 `greige-fabrics:delete` 权限键一直都在，
//! 缺的是"删得掉"。旧门只把 `status` 列与词表常量在库逐字符比对，而建档腿在调用方省略
//! `status` 时把该列恒写成在库，唯一能把状态翻走的出库腿又要求出库量不得超过现有库存与
//! 累计入库量，新档的 weight_kg 与 length_m 与 quantity_kg 与 quantity_meters 四列全为
//! NULL 时任何正出库量都判超量。四条腿互锁的结果是刚建出来的坯布档案永不可删，录错只能
//! 留档。删除判据因此换成可撤回性——先看有没有活单据指着它，再看它自己还有没有在架实物，
//! 两者都不命中才放行软删。
//!
//! 本文件钉住的具体形态：
//! - 归属门最先判，越权必 403 且零副作用，不能被后两道门改写成 400
//! - 引用门命中必 400 且 code 为 BUSINESS_ERROR，message 必须同时点名本条坯布编号与
//!   引用方单据号；引用枚举覆盖 `dye_batch.greige_fabric_id` 与
//!   `outsourcing_order_item.greige_fabric_id` 两条入向列，这两条在库里都是裸加列、
//!   没有 REFERENCES 子句（见 `backend/migration/src/domain/system/mod.rs` 给 dye_batch
//!   加列那句，与 `backend/migration/src/domain/v15/mod.rs` 给 outsourcing_order_item
//!   加列那句），数据库拦不住，只能由应用侧点名
//! - 在架实物门命中必 400，message 点名剩余数量与单位
//! - 两道门都不命中才软删，`is_deleted` 落真，列表按该标记过滤后不再出现该编号
//! - 档案不存在必 404，不得被软删成功路径吞成 200
//!
//! 反向自证（既防"门恒放行"也防"门恒拦"）：
//! - 未流转新档必 2xx，钉引用门与实物门不是一律拒绝
//! - 软删的染色批次引用必不拦，钉引用门按活单据判而不是按行存在判
//! - 在架实物为零的两态（NULL 与显式 0）都放行，钉该门不读状态列
//! - 引用数超过采样上限时 message 带总数但不带第 4 条单号，钉点名形态可复核
//!
//! 权限面另有一节静态派生锁：DELETE 派生出的键必须是已登记资源，否则除超管外谁都授予
//! 不了（同 `contract_wave16_resource_key_derivation_test` 的死键判据），本支不新增端点，
//! 该节只钉既有键没被改坏。
//!
//! 覆盖策略 `test_common::setup_test_db()` 打已迁移 PostgreSQL，每次 TRUNCATE 业务表，
//! 用例之间互不串库。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::greige_fabric_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::middleware::permission::{extract_resource_info, method_to_action};
use bingxi_backend::models::{dye_batch, greige_fabric, outsourcing_order, outsourcing_order_item};
use bingxi_backend::services::init_service::PERMISSION_RESOURCES;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::ActiveValue::{NotSet, Set};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};
use serde_json::Value;
use std::str::FromStr;
use std::sync::Arc;
use tower::ServiceExt;

/// 用例归属人，与 `make_auth` 的 user_id 对齐，数据范围取 self
const OWNER: i32 = 100;
/// 与 OWNER 互不可见的他人，用于越权负例
const OTHER: i32 = 200;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn now_ts() -> chrono::DateTime<Utc> {
    Utc::now()
}

fn make_auth(user_id: i32, scope: Option<&str>) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("test_user_{user_id}"),
        role_id: Some(2),
        department_id: Some(1),
        data_scope: scope.map(|s| s.to_string()),
        dept_ids: None,
        dept_member_user_ids: None,
    }
}

async fn inject_auth(
    State(auth): State<AuthContext>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    request.extensions_mut().insert(auth);
    next.run(request).await
}

/// 只挂坯布列表与删除两条腿的最小路由，权限中间件不在本文件作用面内，
/// 归属门由 handler 内的 data_scope 真实执行
fn build_app(db: sea_orm::DatabaseConnection, auth: AuthContext) -> Router {
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    Router::new()
        .route(
            "/greige-fabrics",
            axum::routing::get(greige_fabric_handler::list_greige_fabrics),
        )
        .route(
            "/greige-fabrics/{id}",
            axum::routing::delete(greige_fabric_handler::delete_greige_fabric),
        )
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth))
}

async fn call(app: &Router, method: Method, uri: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(method)
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

// ============================================================
// 播种
// ============================================================

/// 播一条坯布档案。四个数量参数直接给 ActiveValue，调用方用 NotSet 表达"库里该列为
/// NULL"，用 Set(Some(...)) 表达显式值（含显式 0），两态都测得到。
#[allow(clippy::too_many_arguments)]
async fn seed_greige(
    db: &sea_orm::DatabaseConnection,
    id: i32,
    fabric_no: &str,
    created_by: Option<i32>,
    status: Option<&str>,
    weight_kg: sea_orm::ActiveValue<Option<Decimal>>,
    length_m: sea_orm::ActiveValue<Option<Decimal>>,
    quantity_kg: sea_orm::ActiveValue<Option<Decimal>>,
    quantity_meters: sea_orm::ActiveValue<Option<Decimal>>,
) {
    greige_fabric::ActiveModel {
        id: Set(id),
        fabric_no: Set(fabric_no.to_string()),
        fabric_name: Set(format!("契约坯布 {fabric_no}")),
        fabric_type: Set("梭织".to_string()),
        status: Set(status.map(|s| s.to_string())),
        is_deleted: Set(Some(false)),
        created_by: Set(created_by),
        // 四个数量参数本身就是 ActiveValue，直接赋值，不再套一层 Set
        weight_kg,
        length_m,
        quantity_kg,
        quantity_meters,
        created_at: Set(now_ts().fixed_offset()),
        updated_at: Set(now_ts().fixed_offset()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("播种坯布 {fabric_no} 失败: {e}"));
}

/// 播一条指向该坯布的染色批次，返回其缸号。is_deleted 由调用方决定活删两态。
async fn seed_dye_batch(
    db: &sea_orm::DatabaseConnection,
    batch_no: &str,
    greige_fabric_id: i32,
    is_deleted: Option<bool>,
) {
    dye_batch::ActiveModel {
        id: NotSet,
        batch_no: Set(batch_no.to_string()),
        greige_fabric_id: Set(Some(greige_fabric_id)),
        color_code: Set("C-DR".to_string()),
        color_name: Set("契约色".to_string()),
        color_no: Set(Some("C-DR".to_string())),
        dye_lot_no: Set("DL-DR".to_string()),
        planned_quantity: Set(Some(dec("100.00"))),
        actual_output_kg: Set(None),
        actual_output_m: Set(None),
        greige_input_kg: Set(None),
        status: Set(Some("pending_schedule".to_string())),
        started_at: Set(None),
        completed_at: Set(None),
        remarks: Set(None),
        is_deleted: Set(is_deleted),
        created_at: Set(now_ts().fixed_offset()),
        updated_at: Set(now_ts().fixed_offset()),
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("播种染色批次 {batch_no} 失败: {e}"));
}

/// 播一条委外加工单与一行发料明细，明细指向该坯布，返回委外单号。
/// 两张表对 supplier_id 与 product_id 都只有裸 INTEGER 列、库里无外键约束，
/// 故不必先播供应商与产品主档。
async fn seed_outsourcing_reference(
    db: &sea_orm::DatabaseConnection,
    order_no: &str,
    greige_fabric_id: i32,
) {
    let order = outsourcing_order::ActiveModel {
        id: NotSet,
        order_no: Set(order_no.to_string()),
        order_type: Set("委托加工".to_string()),
        supplier_id: Set(1),
        issue_date: Set(NaiveDate::from_ymd_opt(2026, 1, 10).unwrap()),
        issue_quantity: Set(dec("100.0000")),
        issue_unit: Set("kg".to_string()),
        return_quantity: Set(Decimal::ZERO),
        loss_quantity: Set(Decimal::ZERO),
        material_cost: Set(Decimal::ZERO),
        processing_fee: Set(Decimal::ZERO),
        freight_fee: Set(Decimal::ZERO),
        tax_amount: Set(Decimal::ZERO),
        abnormal_loss_amount: Set(Decimal::ZERO),
        total_cost: Set(Decimal::ZERO),
        unit_cost: Set(Decimal::ZERO),
        status: Set("draft".to_string()),
        is_deleted: Set(false),
        created_by: Set(Some(OWNER)),
        created_at: Set(now_ts().fixed_offset()),
        updated_at: Set(now_ts().fixed_offset()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("播种委外订单 {order_no} 失败: {e}"));

    outsourcing_order_item::ActiveModel {
        id: NotSet,
        outsourcing_order_id: Set(order.id),
        product_id: Set(1),
        greige_fabric_id: Set(Some(greige_fabric_id)),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("播种委外发料明细失败: {e}"));
}

/// 回读库里的真值，断言只认 greige_fabric 一行，不认响应体
async fn read_greige(db: &sea_orm::DatabaseConnection, id: i32) -> greige_fabric::Model {
    greige_fabric::Entity::find_by_id(id)
        .one(db)
        .await
        .unwrap()
        .unwrap_or_else(|| panic!("坯布 {id} 行不在库里"))
}

// ============================================================
// A 正向 未流转档案可撤回
// ============================================================

/// 建一条四列数量全 NULL 的新档（建档腿的缺省形态，状态照样是在库），删除必须放行，
/// 并且回读库里软删标记落真、列表不再出现该编号。
#[tokio::test]
async fn fresh_archive_without_stock_is_deletable_and_disappears_from_list() {
    let db = test_common::setup_test_db().await;
    seed_greige(
        &db,
        1,
        "GF-DR-FRESH",
        Some(OWNER),
        Some("在库"),
        NotSet,
        NotSet,
        NotSet,
        NotSet,
    )
    .await;

    let app = build_app(db.clone(), make_auth(OWNER, Some("self")));
    let (status, body) = call(&app, Method::DELETE, "/greige-fabrics/1").await;
    assert_eq!(
        status,
        StatusCode::OK,
        "未流转坯布删除应放行，实际 code={}",
        body["code"]
    );

    let row = read_greige(&db, 1).await;
    assert_eq!(row.is_deleted, Some(true), "软删标记必须落库");

    let (_list_status, list) = call(
        &app,
        Method::GET,
        "/greige-fabrics?fabric_no=GF-DR-FRESH&page=1&page_size=50",
    )
    .await;
    assert_eq!(
        list["data"]["total"], 0,
        "软删后列表计数必须归零，实际出参 {list}"
    );
    assert_eq!(
        list["data"]["items"]
            .as_array()
            .unwrap_or_else(|| panic!("列表 items 不是数组，出参 {list}"))
            .len(),
        0,
        "软删后列表不得再返回该坯布"
    );
}

/// 显式写 0 与 NULL 同判为"无在架实物"，两态都要放行，钉实物门读的是数量而非状态列。
#[tokio::test]
async fn explicit_zero_stock_is_deletable() {
    let db = test_common::setup_test_db().await;
    seed_greige(
        &db,
        2,
        "GF-DR-ZERO",
        Some(OWNER),
        Some("在库"),
        Set(Some(Decimal::ZERO)),
        Set(Some(Decimal::ZERO)),
        Set(Some(Decimal::ZERO)),
        Set(Some(Decimal::ZERO)),
    )
    .await;

    let app = build_app(db.clone(), make_auth(OWNER, Some("self")));
    let (status, body) = call(&app, Method::DELETE, "/greige-fabrics/2").await;
    assert_eq!(
        status,
        StatusCode::OK,
        "数量显式为 0 的新档同样应可撤回，实际出参 {body}"
    );
    assert_eq!(read_greige(&db, 2).await.is_deleted, Some(true));
}

/// 入出相抵的历史卷（累计入库为正、当前库存归零，状态已出库）必须仍可撤回，
/// 钉实物门不读累计入库两列，也不因引用门之外的任何列拒绝。
#[tokio::test]
async fn fully_stocked_out_archive_is_deletable() {
    let db = test_common::setup_test_db().await;
    seed_greige(
        &db,
        3,
        "GF-DR-OUT",
        Some(OWNER),
        Some("已出库"),
        Set(Some(Decimal::ZERO)),
        Set(Some(Decimal::ZERO)),
        Set(Some(dec("50.00"))),
        Set(Some(dec("100.00"))),
    )
    .await;

    let app = build_app(db.clone(), make_auth(OWNER, Some("self")));
    let (status, body) = call(&app, Method::DELETE, "/greige-fabrics/3").await;
    assert_eq!(
        status,
        StatusCode::OK,
        "已出库且当前库存归零的档案应可撤回，实际出参 {body}"
    );
    assert_eq!(read_greige(&db, 3).await.is_deleted, Some(true));
}

// ============================================================
// B 在架实物门 负例
// ============================================================

/// 仍有剩余重量时必拒，且 message 点名本条编号、剩余数量与"出库"这条可行动作。
#[tokio::test]
async fn remaining_weight_is_refused_and_names_the_quantity() {
    let db = test_common::setup_test_db().await;
    seed_greige(
        &db,
        4,
        "GF-DR-WEIGHT",
        Some(OWNER),
        Some("在库"),
        Set(Some(dec("12.50"))),
        NotSet,
        Set(Some(dec("50.00"))),
        NotSet,
    )
    .await;

    let app = build_app(db.clone(), make_auth(OWNER, Some("self")));
    let (status, body) = call(&app, Method::DELETE, "/greige-fabrics/4").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "BUSINESS_ERROR", "出参 {body}");

    let msg = body["message"].as_str().unwrap_or_default();
    assert!(
        msg.contains("GF-DR-WEIGHT"),
        "拒因必须点名是哪条坯布，实际 message = {msg}"
    );
    assert!(
        msg.contains("12.5") && msg.contains("公斤"),
        "拒因必须给出剩余实物量与单位，实际 message = {msg}"
    );
    assert!(
        msg.contains("出库"),
        "拒因必须指向可行动作，实际 message = {msg}"
    );
    assert_ne!(
        body["message"], "业务处理失败",
        "displayable 族不得被包装层降级脱敏"
    );

    let row = read_greige(&db, 4).await;
    assert_eq!(row.is_deleted, Some(false), "被拒路径不得产生删除副作用");
    assert_eq!(row.weight_kg, Some(dec("12.50")), "被拒路径不得改动库存");
}

/// 重量与长度同时为正时两笔量都要点名，缺一项都算文案不完整。
#[tokio::test]
async fn remaining_both_measures_names_both() {
    let db = test_common::setup_test_db().await;
    seed_greige(
        &db,
        5,
        "GF-DR-BOTH",
        Some(OWNER),
        Some("在库"),
        Set(Some(dec("12.50"))),
        Set(Some(dec("100.00"))),
        Set(Some(dec("12.50"))),
        Set(Some(dec("100.00"))),
    )
    .await;

    let app = build_app(db.clone(), make_auth(OWNER, Some("self")));
    let (status, body) = call(&app, Method::DELETE, "/greige-fabrics/5").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let msg = body["message"].as_str().unwrap_or_default();
    assert!(
        msg.contains("12.5") && msg.contains("公斤"),
        "剩余重量必须点名，实际 message = {msg}"
    );
    assert!(
        msg.contains("100") && msg.contains("米"),
        "剩余米数必须点名，实际 message = {msg}"
    );
    assert_eq!(read_greige(&db, 5).await.is_deleted, Some(false));
}

// ============================================================
// C 引用门 负例与判别力
// ============================================================

/// 被活染色批次引用时必拒，文案逐字符钉死，确保点名到缸号而不是"删除失败"。
#[tokio::test]
async fn live_dye_batch_reference_is_refused_and_names_the_batch_no() {
    let db = test_common::setup_test_db().await;
    // 无在架实物，仍必须被引用门挡住，钉两门相互独立
    seed_greige(
        &db,
        6,
        "GF-DR-REF",
        Some(OWNER),
        Some("在库"),
        NotSet,
        NotSet,
        NotSet,
        NotSet,
    )
    .await;
    seed_dye_batch(&db, "DYE-DR-0001", 6, Some(false)).await;

    let app = build_app(db.clone(), make_auth(OWNER, Some("self")));
    let (status, body) = call(&app, Method::DELETE, "/greige-fabrics/6").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "BUSINESS_ERROR", "出参 {body}");
    assert_eq!(
        body["message"],
        "坯布 GF-DR-REF 已被染色批次引用（缸号 DYE-DR-0001），请先删除这些染色批次或把它们改配到其他坯布"
    );

    let row = read_greige(&db, 6).await;
    assert_eq!(row.is_deleted, Some(false), "被引用挡住时不得留软删标记");
}

/// 被委外加工发料明细引用时必拒，并点名所属委外单号（明细本身没有单号，
/// 用户能动手的是那张单，不能只回"存在引用"）。
#[tokio::test]
async fn outsourcing_item_reference_is_refused_and_names_the_order_no() {
    let db = test_common::setup_test_db().await;
    seed_greige(
        &db,
        7,
        "GF-DR-OUTSRC",
        Some(OWNER),
        Some("在库"),
        NotSet,
        NotSet,
        NotSet,
        NotSet,
    )
    .await;
    seed_outsourcing_reference(&db, "WW-DR-0001", 7).await;

    let app = build_app(db.clone(), make_auth(OWNER, Some("self")));
    let (status, body) = call(&app, Method::DELETE, "/greige-fabrics/7").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "BUSINESS_ERROR", "出参 {body}");
    assert_eq!(
        body["message"],
        "坯布 GF-DR-OUTSRC 已被委外加工发料明细引用（委外单号 WW-DR-0001），请先删除这些发料明细或把它们改配到其他坯布"
    );
    assert_eq!(read_greige(&db, 7).await.is_deleted, Some(false));
}

/// 判别力反向锁 作废（软删）的染色批次不构成拦删理由。少了这条，引用门写成
/// "存在任意行即拒"也能骗过上面两条正例。
#[tokio::test]
async fn soft_deleted_dye_batch_reference_does_not_block_delete() {
    let db = test_common::setup_test_db().await;
    seed_greige(
        &db,
        8,
        "GF-DR-VOIDREF",
        Some(OWNER),
        Some("在库"),
        NotSet,
        NotSet,
        NotSet,
        NotSet,
    )
    .await;
    seed_dye_batch(&db, "DYE-DR-VOID01", 8, Some(true)).await;

    let app = build_app(db.clone(), make_auth(OWNER, Some("self")));
    let (status, body) = call(&app, Method::DELETE, "/greige-fabrics/8").await;
    assert_eq!(
        status,
        StatusCode::OK,
        "作废单据不应挡住档案撤回，实际出参 {body}"
    );
    assert_eq!(read_greige(&db, 8).await.is_deleted, Some(true));
}

/// 判别力反向锁 `dye_batch.is_deleted` 为 NULL 的行按未删除计。判据若写成
/// `<> true`，SQL 三值逻辑会把 NULL 行整批排除，未删除的缸号被漏掉，引用门形同虚设。
#[tokio::test]
async fn null_flagged_dye_batch_reference_still_blocks_delete() {
    let db = test_common::setup_test_db().await;
    seed_greige(
        &db,
        9,
        "GF-DR-NULLREF",
        Some(OWNER),
        Some("在库"),
        NotSet,
        NotSet,
        NotSet,
        NotSet,
    )
    .await;
    seed_dye_batch(&db, "DYE-DR-NULLFLAG", 9, None).await;

    let app = build_app(db.clone(), make_auth(OWNER, Some("self")));
    let (status, body) = call(&app, Method::DELETE, "/greige-fabrics/9").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["message"].as_str().unwrap_or_default(),
        "坯布 GF-DR-NULLREF 已被染色批次引用（缸号 DYE-DR-NULLFLAG），请先删除这些染色批次或把它们改配到其他坯布"
    );
    assert_eq!(read_greige(&db, 9).await.is_deleted, Some(false));
}

/// 点名形态锁 五条引用只列前三条并带总数，第四第五条不得塞进文案（否则提示变成噪声），
/// 总数必须报五条（否则用户以为处理三张就够）。
#[tokio::test]
async fn many_references_name_three_and_report_the_total() {
    let db = test_common::setup_test_db().await;
    seed_greige(
        &db,
        10,
        "GF-DR-MANY",
        Some(OWNER),
        Some("在库"),
        NotSet,
        NotSet,
        NotSet,
        NotSet,
    )
    .await;
    // 故意按逆序播种，钉文案里的单号来自排序取数而非插入顺序
    for seq in (1..=5).rev() {
        seed_dye_batch(&db, &format!("DYE-DR-M{seq:02}"), 10, Some(false)).await;
    }

    let app = build_app(db.clone(), make_auth(OWNER, Some("self")));
    let (status, body) = call(&app, Method::DELETE, "/greige-fabrics/10").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let msg = body["message"].as_str().unwrap_or_default();
    assert!(
        msg.contains("DYE-DR-M01、DYE-DR-M02、DYE-DR-M03"),
        "文案应按缸号升序列出前三条，实际 message = {msg}"
    );
    assert!(
        msg.contains("等，共 5 个"),
        "超出采样上限时应带总数，实际 message = {msg}"
    );
    assert!(
        !msg.contains("DYE-DR-M04"),
        "第四条起不应展开，实际 message = {msg}"
    );
}

// ============================================================
// D 门次序 与 404
// ============================================================

/// 归属门必须最先判 他人删"被引用且有货"的档案，拿到的应是 403 而不是 400，
/// 否则等于用拒因泄露他人的数据形态。
#[tokio::test]
async fn non_owner_gets_forbidden_even_when_referenced_and_stocked() {
    let db = test_common::setup_test_db().await;
    seed_greige(
        &db,
        11,
        "GF-DR-OWNER",
        Some(OWNER),
        Some("在库"),
        Set(Some(dec("5.00"))),
        NotSet,
        NotSet,
        NotSet,
    )
    .await;
    seed_dye_batch(&db, "DYE-DR-OWNER", 11, Some(false)).await;

    let app = build_app(db.clone(), make_auth(OTHER, Some("self")));
    let (status, body) = call(&app, Method::DELETE, "/greige-fabrics/11").await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "FORBIDDEN", "出参 {body}");

    let row = read_greige(&db, 11).await;
    assert_eq!(row.is_deleted, Some(false), "越权路径不得改动数据");
    assert_eq!(row.weight_kg, Some(dec("5.00")), "越权路径不得扣减库存");
}

/// 档案不存在必 404，不得被软删成功路径吞成 200，也不得由引用预检先撞出 400。
#[tokio::test]
async fn missing_archive_returns_not_found() {
    let db = test_common::setup_test_db().await;

    let app = build_app(db.clone(), make_auth(OWNER, Some("self")));
    let (status, body) = call(&app, Method::DELETE, "/greige-fabrics/999999").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "NOT_FOUND", "出参 {body}");
}

/// 软删过的档案再删一次应重新走存在性判定并成功（幂等），不得报 500。
#[tokio::test]
async fn deleting_an_already_soft_deleted_archive_is_idempotent() {
    let db = test_common::setup_test_db().await;
    greige_fabric::ActiveModel {
        id: Set(12),
        fabric_no: Set("GF-DR-AGAIN".to_string()),
        fabric_name: Set("契约坯布 重复删".to_string()),
        fabric_type: Set("梭织".to_string()),
        status: Set(Some("在库".to_string())),
        is_deleted: Set(Some(true)),
        created_by: Set(Some(OWNER)),
        created_at: Set(now_ts().fixed_offset()),
        updated_at: Set(now_ts().fixed_offset()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let app = build_app(db.clone(), make_auth(OWNER, Some("self")));
    let (status, body) = call(&app, Method::DELETE, "/greige-fabrics/12").await;
    assert_eq!(status, StatusCode::OK, "出参 {body}");
    assert_eq!(read_greige(&db, 12).await.is_deleted, Some(true));
}

// ============================================================
// E 权限面 静态派生锁（不连库）
// ============================================================

/// DELETE 派生出的资源段必须是 PERMISSION_RESOURCES 里登记的资源，否则该键除超管外
/// 谁都授予不了（死键）。本支不新增端点，这一节钉既有派生链没被改坏，
/// 判据与 `contract_wave16_resource_key_derivation_test` 同源。
#[test]
fn delete_route_derives_a_registered_grantable_key() {
    let detail_path = "/api/v1/erp/production/greige-fabrics/1";
    let (resource, id) = extract_resource_info(detail_path);
    assert_eq!(
        resource, "greige-fabrics",
        "生产是模块前缀，资源段应取坯布段名且不被消歧表改写"
    );
    assert_eq!(id, Some(1), "末段数字应解析成记录 id 而非动作段");

    assert_eq!(method_to_action(&Method::DELETE), "delete");

    assert!(
        PERMISSION_RESOURCES.contains(&"greige-fabrics"),
        "资源未登记时门层在超管旁路之前就判未知的资源路径，端点对任何角色都永不可达"
    );

    // 角色矩阵里至少有一个岗位被授予 delete 或通配，否则前端按钮可见而调用必 403
    let matrix = include_str!("../src/services/init_service_ops/permission.rs");
    let matrix_code = matrix
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<&str>>()
        .join("\n");
    let granted_exact = matrix_code.contains("(\"greige-fabrics\", \"delete\")");
    let granted_wild = matrix_code.contains("(\"greige-fabrics\", \"*\")");
    assert!(
        granted_exact || granted_wild,
        "greige-fabrics 的删除键必须至少授予一个岗位，否则该端点除超管外全员不可达"
    );
}

/// 引用枚举的检测力自证 本文件播种的两张引用表确实能在库里被查到，
/// 否则 C 段的正例可能只是"路径没走通"而非"门真挡住了"。
#[tokio::test]
async fn reference_seeding_really_reaches_the_two_tables() {
    let db = test_common::setup_test_db().await;
    seed_greige(
        &db,
        13,
        "GF-DR-PROBE",
        Some(OWNER),
        Some("在库"),
        NotSet,
        NotSet,
        NotSet,
        NotSet,
    )
    .await;
    seed_dye_batch(&db, "DYE-DR-PROBE", 13, Some(false)).await;
    seed_outsourcing_reference(&db, "WW-DR-PROBE", 13).await;

    let batches = dye_batch::Entity::find()
        .filter(dye_batch::Column::GreigeFabricId.eq(13))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(batches.len(), 1, "dye_batch 引用行必须真落库");
    assert_eq!(batches[0].batch_no, "DYE-DR-PROBE");

    let items = outsourcing_order_item::Entity::find()
        .filter(outsourcing_order_item::Column::GreigeFabricId.eq(13))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(items.len(), 1, "outsourcing_order_item 引用行必须真落库");

    let order = outsourcing_order::Entity::find_by_id(items[0].outsourcing_order_id)
        .one(&db)
        .await
        .unwrap();
    assert_eq!(
        order.map(|o| o.order_no),
        Some("WW-DR-PROBE".to_string()),
        "委外单号应能按明细回查到，供拒因点名"
    );
}
