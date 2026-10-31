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
//! - 软删的委外单引用必不拦，钉第二条引用腿同样按活单据判
//! - 在架实物为零的两态（NULL 与显式 0）都放行，钉该门不读状态列
//! - 引用数超过采样上限时 message 带总数但不带第 4 条单号，钉点名形态可复核
//! - 门判据只读在架实物两列，不读累计台账两列、不读克重与门幅等规格列（面料行业
//!   一卷布的克重与门幅是规格量不是库存量，把它们当删除判据会把零库存的正常卷拦死）
//!
//! 权限面。另有一节真挂 `permission_middleware` 的活测（不连库的静态派生锁只钉键没被
//! 改坏，放行面必须实测）：只授予 greige-fabrics 的 read 或 create 或删除别的资源的
//! 角色，DELETE 必 403 且档案行零变化；授予 delete 键的角色必 200 且列表不再出现；
//! 有键角色在被引用时仍必 400，钉权限面与门判据相互独立、有键不等于旁路门。
//! 授权行按本文件自建角色与自清理（roles 与 role_permissions 是迁移种子参照表，
//! 夹具不清空，法同 `contract_fabric_order_state_gates_test`）。
//!
//! 另有一节不连库的静态派生锁：DELETE 派生出的键必须是已登记资源，否则除超管外谁都授予
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
use bingxi_backend::middleware::permission::{
    extract_resource_info, invalidate_permission_cache, method_to_action, permission_middleware,
};
use bingxi_backend::models::{
    dye_batch, greige_fabric, outsourcing_order, outsourcing_order_item, role, role_permission,
    user,
};
use bingxi_backend::services::init_service::PERMISSION_RESOURCES;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::ActiveValue::{NotSet, Set};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde_json::Value;
use std::str::FromStr;
use std::sync::Arc;
use tower::ServiceExt;

/// 用例归属人，与 `make_auth` 的 user_id 对齐，数据范围取 self
const OWNER: i32 = 100;
/// 与 OWNER 互不可见的他人，用于越权负例
const OTHER: i32 = 200;
/// 权限面用例的操作人（自建 users 行，与上面的归属人分开，避免与 A 段的播种号互撞）
const PERM_OWNER: i32 = 9601;
/// 权限面自种角色号。roles 与 role_permissions 是迁移种子参照表，夹具不清空，
/// 每轮先按 code 自清理再播种（法同 contract_fabric_order_state_gates_test）
const ROLE_DELETE_GRANTEE: &str = "it_gf_g_delete";
const ROLE_READ_ONLY: &str = "it_gf_g_read";
const ROLE_CREATE_ONLY: &str = "it_gf_g_create";
const ROLE_OTHER_RESOURCE: &str = "it_gf_g_other_resource";
const ALL_GF_ROLES: [&str; 4] = [
    ROLE_DELETE_GRANTEE,
    ROLE_READ_ONLY,
    ROLE_CREATE_ONLY,
    ROLE_OTHER_RESOURCE,
];

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
/// order_is_deleted 给引用门的活删两态：outsourcing_order.is_deleted 库里 NOT NULL
/// DEFAULT false（与可空的 dye_batch.is_deleted 不同形态），所以删除门对它按
/// `= false` 单态判定即完备，不写 IS NULL 分支。
async fn seed_outsourcing_reference(
    db: &DatabaseConnection,
    order_no: &str,
    greige_fabric_id: i32,
    order_is_deleted: bool,
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
        is_deleted: Set(order_is_deleted),
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
    seed_outsourcing_reference(&db, "WW-DR-0001", 7, false).await;

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
    seed_outsourcing_reference(&db, "WW-DR-PROBE", 13, false).await;

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

// ============================================================
// F 判据面补充 状态列与累计台账两列与规格列都不进删除判据
// ============================================================

/// 反向自证 状态列写在库且累计台账两列为正而当前库存两列为零的档案仍必可撤回。
///
/// 这一条同时钉三件事。一 旧门读的状态列已彻底退出判据（旧门下本行必 400，正是本支
/// 要消的建档即死档）。二 累计两列 quantity_kg 与 quantity_meters 不构成独立的在架
/// 实物信号 入库腿把四列一起加、出库腿把四列一起减（见本 handler 的 stock_in 与
/// stock_out 各自的四行赋值），两栏同源。三 更新腿入参结构体 UpdateGreigeFabricRequest
/// 不接收这两列，把它们列入判据等于让建档时误填累计量而当前库存为零的卷重新进入
/// 既不能出库（出库按当前库存判超量）也不能删除的死档，与本轮改造目标相反。
#[tokio::test]
async fn status_token_and_cumulative_ledger_are_not_delete_criteria() {
    let db = test_common::setup_test_db().await;
    seed_greige(
        &db,
        30,
        "GF-DR-LEDGER",
        Some(OWNER),
        Some("在库"),
        Set(Some(Decimal::ZERO)),
        Set(Some(Decimal::ZERO)),
        Set(Some(dec("50.00"))),
        Set(Some(dec("100.00"))),
    )
    .await;

    let app = build_app(db.clone(), make_auth(OWNER, Some("self")));
    let (status, body) = call(&app, Method::DELETE, "/greige-fabrics/30").await;
    assert_eq!(
        status,
        StatusCode::OK,
        "当前库存归零的档案不得因状态列或累计台账两列被拦，实际出参 {body}"
    );
    assert_eq!(read_greige(&db, 30).await.is_deleted, Some(true));
}

/// 反向自证 克重与门幅等规格列不进删除判据，零在架实物即放行。
///
/// 面料行业一卷布的克重(g/m2)与门幅(cm)是规格量不是库存量。建表段
/// backend/migration/src/domain/system/m0002_add_crm_and_greige_tables.rs 的
/// COMMENT ON weight_kg 那句曾把它注释成克重(kg/m2)，而入库出库两腿把它当库存公斤数
/// 用，两套口径并存于同一列。删除门只读库存两列，一旦把规格列拉进判据，一条只填了
/// 克重的正常空卷就会被仍有在架库存误拦。本例把三列规格值都填正、库存两列留 NULL，
/// 必放行。
#[tokio::test]
async fn spec_columns_are_not_delete_criteria() {
    let db = test_common::setup_test_db().await;
    greige_fabric::ActiveModel {
        id: Set(31),
        fabric_no: Set("GF-DR-SPEC".to_string()),
        fabric_name: Set("契约坯布 只填规格".to_string()),
        fabric_type: Set("梭织".to_string()),
        status: Set(Some("在库".to_string())),
        is_deleted: Set(Some(false)),
        created_by: Set(Some(OWNER)),
        gram_weight: Set(Some(dec("0.28"))),
        width_cm: Set(Some(dec("160.00"))),
        width: Set(Some(dec("1.60"))),
        color_no: Set(Some("C-DR-SPEC".to_string())),
        batch_no: Set(Some("B-DR-SPEC".to_string())),
        dye_lot_no: Set(Some("DL-DR-SPEC".to_string())),
        created_at: Set(now_ts().fixed_offset()),
        updated_at: Set(now_ts().fixed_offset()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap_or_else(|e| panic!("播种规格齐备但零库存的坯布失败: {e}"));

    let app = build_app(db.clone(), make_auth(OWNER, Some("self")));
    let (status, body) = call(&app, Method::DELETE, "/greige-fabrics/31").await;
    assert_eq!(
        status,
        StatusCode::OK,
        "克重与门幅等规格列不得把零库存档案拦成不可撤回，实际出参 {body}"
    );
    assert_eq!(read_greige(&db, 31).await.is_deleted, Some(true));
}

/// 判别力反向锁 已作废（软删）的委外单不构成拦删理由，钉第二条引用腿同样按活单据判。
///
/// 缺这一条时，第二条腿写成明细行存在即拒也能骗过 C 段的委外正例。委外发料明细自身
/// 没有软删标记（模型 outsourcing_order_item 无 is_deleted 列，随单级联删），单据主表
/// outsourcing_order 的 is_deleted 库里 NOT NULL（建表段在
/// backend/migration/src/domain/v15/mod.rs 的委外单建表那句），所以该腿按 = false 单态
/// 判定即完备；而 dye_batch 的 is_deleted 是后补的可空裸列（加列段在
/// backend/migration/src/domain/system/mod.rs 的 ALTER TABLE dye_batch 那批），
/// 两态判定的必要性由
/// null_flagged_dye_batch_reference_still_blocks_delete 钉住。
#[tokio::test]
async fn soft_deleted_outsourcing_order_reference_does_not_block_delete() {
    let db = test_common::setup_test_db().await;
    seed_greige(
        &db,
        32,
        "GF-DR-VOIDWW",
        Some(OWNER),
        Some("在库"),
        NotSet,
        NotSet,
        NotSet,
        NotSet,
    )
    .await;
    seed_outsourcing_reference(&db, "WW-DR-VOID01", 32, true).await;

    let app = build_app(db.clone(), make_auth(OWNER, Some("self")));
    let (status, body) = call(&app, Method::DELETE, "/greige-fabrics/32").await;
    assert_eq!(
        status,
        StatusCode::OK,
        "作废委外单的发料明细不应挡住档案撤回，实际出参 {body}"
    );
    assert_eq!(read_greige(&db, 32).await.is_deleted, Some(true));
}

/// 文案面锁 两道门的拒绝文案点名到单据号与数量，且不外泄表名与列名与 SQL 词汇。
///
/// 用户可见的是 message 原文，所以既钉含本条坯布编号与引用方单号，也钉不含库内
/// 标识符，防止把拒因写成数据库报错原文那种不可读形态。
#[tokio::test]
async fn rejection_text_names_the_document_and_hides_schema_identifiers() {
    let db = test_common::setup_test_db().await;
    seed_greige(
        &db,
        33,
        "GF-DR-TEXTBATCH",
        Some(OWNER),
        Some("在库"),
        NotSet,
        NotSet,
        NotSet,
        NotSet,
    )
    .await;
    seed_dye_batch(&db, "DYE-DR-TEXT01", 33, Some(false)).await;
    seed_greige(
        &db,
        34,
        "GF-DR-TEXTWW",
        Some(OWNER),
        Some("在库"),
        Set(Some(dec("9.00"))),
        NotSet,
        NotSet,
        NotSet,
    )
    .await;

    let app = build_app(db.clone(), make_auth(OWNER, Some("self")));

    let (batch_status, batch_body) = call(&app, Method::DELETE, "/greige-fabrics/33").await;
    assert_eq!(batch_status, StatusCode::BAD_REQUEST);
    let batch_msg = batch_body["message"].as_str().unwrap_or_default();
    assert!(
        batch_msg.contains("GF-DR-TEXTBATCH") && batch_msg.contains("DYE-DR-TEXT01"),
        "引用腿必须同时点名本条坯布与引用方缸号，实际 message = {batch_msg}"
    );

    let (stock_status, stock_body) = call(&app, Method::DELETE, "/greige-fabrics/34").await;
    assert_eq!(stock_status, StatusCode::BAD_REQUEST);
    let stock_msg = stock_body["message"].as_str().unwrap_or_default();
    assert!(
        stock_msg.contains("GF-DR-TEXTWW") && stock_msg.contains("9"),
        "实物腿必须同时点名本条坯布与剩余量，实际 message = {stock_msg}"
    );

    for (label, msg) in [("引用腿", batch_msg), ("实物腿", stock_msg)] {
        for token in [
            "dye_batch",
            "greige_fabric",
            "outsourcing_order",
            "is_deleted",
            "greige_fabric_id",
            "SELECT",
            "TABLE",
        ] {
            assert!(
                !msg.contains(token),
                "{label}的可见文案不得外泄库内标识符 {token}，实际 message = {msg}"
            );
        }
    }

    assert_eq!(read_greige(&db, 33).await.is_deleted, Some(false));
    assert_eq!(read_greige(&db, 34).await.is_deleted, Some(false));
}

// ============================================================
// G 权限面 真挂 permission_middleware 的放行与拒绝（连库活测）
// ============================================================

/// 清掉上一轮残留的自种角色与授权行。
///
/// roles 与 role_permissions 是迁移种子参照表，夹具每轮只 TRUNCATE 业务表不清它们，
/// 所以自建授权行必须显式自清理（法同 contract_fabric_order_state_gates_test）。
/// 调用方是本节每条用例的第一行，入参 db 是本用例的活库连接，清完再播种。
async fn cleanup_gf_roles(db: &DatabaseConnection) {
    let role_ids = role::Entity::find()
        .filter(role::Column::Code.is_in(ALL_GF_ROLES))
        .all(db)
        .await
        .unwrap()
        .into_iter()
        .map(|item| item.id)
        .collect::<Vec<i32>>();
    if !role_ids.is_empty() {
        role_permission::Entity::delete_many()
            .filter(role_permission::Column::RoleId.is_in(role_ids.clone()))
            .exec(db)
            .await
            .unwrap();
        for role_id in &role_ids {
            invalidate_permission_cache(*role_id);
        }
    }
    role::Entity::delete_many()
        .filter(role::Column::Code.is_in(ALL_GF_ROLES))
        .exec(db)
        .await
        .unwrap();
}

/// 播种权限面操作人。传给谁 只进 users 表，供会话身份成立。
///
/// greige_fabric.created_by 库里无外键，但归属门按会话 user_id 逐字比对，且 users 表
/// 每轮被夹具清空，所以本用例必须自建这一行，不能假设种子里有。
async fn seed_gf_user(db: &DatabaseConnection) {
    user::ActiveModel {
        id: Set(PERM_OWNER),
        username: Set("it_gf_perm".to_string()),
        password_hash: Set("test-only-not-a-real-hash".to_string()),
        real_name: Set(Some("坯布删除权限面操作人".to_string())),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        created_at: Set(now_ts()),
        updated_at: Set(now_ts()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("播种权限面操作人失败: {e}"));
}

/// 播种一个岗位并回读其 id。存哪里 roles 表，code 用本文件专用前缀便于自清理。
///
/// data_scope 写 all 只为让角色行取值合法；删除门的归属判定读的是 AuthContext 里的
/// self（见 perm_app），两件事互不影响，别把角色列当放行条件。
async fn seed_gf_role(db: &DatabaseConnection, code: &str) -> i32 {
    let model = role::ActiveModel {
        name: Set(format!("坯布删除权限面角色 {code}")),
        code: Set(code.to_string()),
        is_system: Set(false),
        data_scope: Set("all".to_string()),
        created_at: Set(now_ts()),
        updated_at: Set(now_ts()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("播种角色 {code} 失败: {e}"));
    model.id
}

/// 播一行 allowed 为 true 的角色级授权行（resource_id 为 NULL，运行时按精确键命中）。
///
/// 入参 resource 与 action 拼成权限键的资源段与动作段，传给 role_permissions 表；
/// 中间件对 resource_id 为 NULL 的行按「该角色对该资源该动作放行」判定。
async fn seed_gf_grant(db: &DatabaseConnection, role_id: i32, resource: &str, action: &str) {
    role_permission::ActiveModel {
        role_id: Set(role_id),
        resource_type: Set(resource.to_string()),
        resource_id: Set(None),
        action: Set(action.to_string()),
        allowed: Set(true),
        permission_code: Set(None),
        created_at: Set(now_ts()),
        updated_at: Set(now_ts()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("播种授权行 {resource} 的 {action} 失败: {e}"));
}

/// 按真实挂载路径建路由并挂真实权限中间件。
///
/// 调用方是本节三条用例；入参 db 是本用例活库连接、role_id 是被测岗位。路由腿与
/// backend/src/routes/production.rs 的坯布段同路径，于是权限键由 permission_middleware
/// 真实判出，不是测试替身判出。与 A 到 F 段的 build_app 唯一差别就是多挂这一层。
/// 操作人取 PERM_OWNER、数据范围 self，与播种档案的 created_by 对齐，让归属门先过、
/// 权限门成为唯一变量。
fn perm_app(db: DatabaseConnection, role_id: i32) -> Router {
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    let auth = AuthContext {
        user_id: PERM_OWNER,
        username: "it_gf_perm".to_string(),
        role_id: Some(role_id),
        department_id: None,
        data_scope: Some("self".to_string()),
        dept_ids: None,
        dept_member_user_ids: None,
    };
    Router::new()
        .route(
            "/api/v1/erp/production/greige-fabrics",
            axum::routing::get(greige_fabric_handler::list_greige_fabrics),
        )
        .route(
            "/api/v1/erp/production/greige-fabrics/{id}",
            axum::routing::delete(greige_fabric_handler::delete_greige_fabric),
        )
        .layer(from_fn_with_state(state.clone(), permission_middleware))
        .layer(from_fn_with_state(auth, inject_auth))
        .with_state(state)
}

/// 权限面负例 只授予 read 或只授予 create 或只持有别的资源 delete 键的岗位，
/// 删除必 403 且档案行零变化。
///
/// 三条角色一起测，钉两件事。一 同资源的其它动作键不扩权到删除（粗域展开形态）。
/// 二 别的资源的 delete 键不顶用，资源段必须逐字对上 greige-fabrics。
/// 档案按未流转形态播种，所以 403 只可能来自权限门而不是业务门。
#[tokio::test]
async fn roles_without_delete_key_are_forbidden_and_leave_the_archive_intact() {
    let db = test_common::setup_test_db().await;
    cleanup_gf_roles(&db).await;
    seed_gf_user(&db).await;
    let role_read = seed_gf_role(&db, ROLE_READ_ONLY).await;
    seed_gf_grant(&db, role_read, "greige-fabrics", "read").await;
    let role_create = seed_gf_role(&db, ROLE_CREATE_ONLY).await;
    seed_gf_grant(&db, role_create, "greige-fabrics", "create").await;
    let role_other = seed_gf_role(&db, ROLE_OTHER_RESOURCE).await;
    seed_gf_grant(&db, role_other, "dye-batches", "delete").await;
    for role_id in [role_read, role_create, role_other] {
        invalidate_permission_cache(role_id);
    }

    seed_greige(
        &db,
        40,
        "GF-DR-NOPERM",
        Some(PERM_OWNER),
        Some("在库"),
        NotSet,
        NotSet,
        NotSet,
        NotSet,
    )
    .await;

    for (label, role_id) in [
        ("read 键岗位", role_read),
        ("create 键岗位", role_create),
        ("别的资源 delete 键岗位", role_other),
    ] {
        let app = perm_app(db.clone(), role_id);
        let (status, body) = call(
            &app,
            Method::DELETE,
            "/api/v1/erp/production/greige-fabrics/40",
        )
        .await;
        assert_eq!(
            status,
            StatusCode::FORBIDDEN,
            "{label} 没有 greige-fabrics 的 delete 键时删除必 403，实际出参 {body}"
        );
        assert_eq!(body["code"], "FORBIDDEN", "{label} 出参 {body}");
    }

    let row = read_greige(&db, 40).await;
    assert_eq!(
        row.is_deleted,
        Some(false),
        "被权限门拒掉的路径不得留删除标记"
    );
}

/// 权限面正例 授予 delete 键的岗位删除未流转档案必 200，软删标记真落库且列表不再出现。
///
/// 这一条同时钉门真放行（业务门与权限门都不拦）与删的真有效果（库里回读与列表出参
/// 两处都判），不是只看响应绿。
#[tokio::test]
async fn role_with_delete_key_removes_the_archive_and_it_leaves_the_list() {
    let db = test_common::setup_test_db().await;
    cleanup_gf_roles(&db).await;
    seed_gf_user(&db).await;
    let role_delete = seed_gf_role(&db, ROLE_DELETE_GRANTEE).await;
    // 同一岗位双键：delete 是被测的删除腿，read 只给用例末尾那次列表回读用（列表腿
    // 派生的是 greige-fabrics:read，缺它会把列表调用判成 403 而不是把删除判绿）。
    seed_gf_grant(&db, role_delete, "greige-fabrics", "delete").await;
    seed_gf_grant(&db, role_delete, "greige-fabrics", "read").await;
    invalidate_permission_cache(role_delete);

    seed_greige(
        &db,
        41,
        "GF-DR-PERMOK",
        Some(PERM_OWNER),
        Some("在库"),
        NotSet,
        NotSet,
        NotSet,
        NotSet,
    )
    .await;

    let app = perm_app(db.clone(), role_delete);
    let (status, body) = call(
        &app,
        Method::DELETE,
        "/api/v1/erp/production/greige-fabrics/41",
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "持有 greige-fabrics 的 delete 键的岗位删除未流转档案必须放行，实际出参 {body}"
    );
    assert_eq!(body["code"], 200, "成功信封 code 应为 200，出参 {body}");

    assert_eq!(
        read_greige(&db, 41).await.is_deleted,
        Some(true),
        "软删标记必须真落库"
    );

    let (_list_status, list) = call(
        &app,
        Method::GET,
        "/api/v1/erp/production/greige-fabrics?fabric_no=GF-DR-PERMOK&page=1&page_size=50",
    )
    .await;
    assert_eq!(
        list["data"]["total"], 0,
        "有键岗位删除后列表计数必须归零，实际出参 {list}"
    );
}

/// 权限面与业务门相互独立 持有 delete 键的岗位遇到活引用或在架实物仍必 400 且零变化。
///
/// 钉有键不等于旁路删除门，防止把权限放行误写成万能放行。
#[tokio::test]
async fn granted_delete_key_does_not_bypass_the_retractability_gates() {
    let db = test_common::setup_test_db().await;
    cleanup_gf_roles(&db).await;
    seed_gf_user(&db).await;
    let role_delete = seed_gf_role(&db, ROLE_DELETE_GRANTEE).await;
    seed_gf_grant(&db, role_delete, "greige-fabrics", "delete").await;
    invalidate_permission_cache(role_delete);

    seed_greige(
        &db,
        42,
        "GF-DR-PERMREF",
        Some(PERM_OWNER),
        Some("在库"),
        NotSet,
        NotSet,
        NotSet,
        NotSet,
    )
    .await;
    seed_dye_batch(&db, "DYE-DR-PERMREF", 42, Some(false)).await;

    seed_greige(
        &db,
        43,
        "GF-DR-PERMSTOCK",
        Some(PERM_OWNER),
        Some("在库"),
        Set(Some(dec("9.00"))),
        NotSet,
        NotSet,
        NotSet,
    )
    .await;

    let app = perm_app(db.clone(), role_delete);

    let (ref_status, ref_body) = call(
        &app,
        Method::DELETE,
        "/api/v1/erp/production/greige-fabrics/42",
    )
    .await;
    assert_eq!(
        ref_status,
        StatusCode::BAD_REQUEST,
        "有删除键不等于可以删被活缸号指着的档案，实际出参 {ref_body}"
    );
    assert_eq!(ref_body["code"], "BUSINESS_ERROR", "出参 {ref_body}");
    assert!(
        ref_body["message"]
            .as_str()
            .unwrap_or_default()
            .contains("DYE-DR-PERMREF"),
        "被业务门拒掉时仍要点名引用方，实际 message = {}",
        ref_body["message"]
    );

    let (stock_status, stock_body) = call(
        &app,
        Method::DELETE,
        "/api/v1/erp/production/greige-fabrics/43",
    )
    .await;
    assert_eq!(
        stock_status,
        StatusCode::BAD_REQUEST,
        "有删除键不等于可以删仍有在架实物的档案，实际出参 {stock_body}"
    );

    assert_eq!(read_greige(&db, 42).await.is_deleted, Some(false));
    assert_eq!(read_greige(&db, 43).await.is_deleted, Some(false));
}
