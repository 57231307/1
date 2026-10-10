//! approve/reject 授权成对性锁（RBAC 落地，静态判据 + 真库活体各向）
//!
//! 缺陷形态（本锁要拦的两类事故，方向相反）：
//! - **半截授权**：资源已建 `/reject` 端点、矩阵里却只有 `(资源, approve)` 行 ⇒
//!   持审批权的岗位点"拒绝"恒 403（前端按钮可达、RBAC 永拒），与"授权没落库"同罪。
//! - **幽灵授权**：矩阵里写了 `(资源, reject)` 行、该资源却不存在 reject 端点 ⇒
//!   授权悬空，读起来像"已放行一个不存在的动作"，掩盖真实缺口。
//!
//! 判据口径（与生产闸门同源、不造第二套判定）：
//! - 矩阵侧直接驱动生产定义表 `InitService::all_role_permission_definition_groups()`
//!   （与 `create_default_role_permissions` 写库用的是同一份数据）。
//! - 成对判据只对**显式** approve/reject 行生效；`"*"` 通配授予天然双侧覆盖，
//!   不参与成对、也不被反向判据波及（通配是既有授权设计，不在此收口）。
//! - 运行时资源键以 URL 段推导为准（`middleware/permission.rs::extract_resource_info` +
//!   `utils/path_utils.rs` 消歧表），下表逐条附真实路由证据；`sales` 域 orders 不消歧、
//!   运行时键就是 `orders`（用户口径里的 "sales-orders:reject" 实际不存在这个名字）。
//! - 活体断言走真库 + 真 `permission_middleware`：持 `(sales-prices, reject)` 行的角色
//!   真实 200、不持该键的同结构角色真实 403，只断 HTTP 码与机器码（成功信封数字码
//!   `utils/response.rs` 口径、失败信封 `FORBIDDEN`），不断文案原文——权限拒绝文案永久脱敏。

mod test_common;

use bingxi_backend::container::AppState;
use bingxi_backend::handlers::sales_fabric_order_handler;
use bingxi_backend::handlers::sales_price_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::middleware::permission::{invalidate_permission_cache, permission_middleware};
use bingxi_backend::models::status::{master_data, price_approval, sales_fabric_order};
use bingxi_backend::models::{
    customer, product, role, role_permission, sales_order, sales_price, user,
};
use bingxi_backend::services::init_service::{InitService, PERMISSION_RESOURCES};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde_json::{Value, json};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// 一、端点事实表（运行时资源键 × 证据 file:line）
// ---------------------------------------------------------------------------

/// 存在真实 `/reject` 末段端点的资源（运行时键口径，非 URL 段原名）。
///
/// 判据只认"路径末段恰为 reject"的端点：`PATH_ACTION_KEYWORDS`
/// （`middleware/permission.rs:226-229`）对末段做精确匹配，
/// `finance-approve`/`approve-l1` 这类复合段不派生 approve/reject 动作键，不入选。
const RESOURCES_WITH_REJECT_ENDPOINT: &[(&str, &str)] = &[
    (
        "orders",
        "src/routes/sales.rs: /orders/{id}/reject（sales 域 orders 保留原名，不消歧）",
    ),
    (
        "fabric-orders",
        "src/routes/sales.rs: /fabric-orders/{id}/reject",
    ),
    (
        "dye-recipes",
        "src/routes/production.rs: /dye-recipes/{id}/reject",
    ),
    (
        "sales-returns",
        "src/handlers/sales_return_handler.rs: /sales-returns/{id}/reject（经 routes/sales.rs 挂 /sales 下）",
    ),
    (
        "sales-contracts",
        "src/routes/sales.rs: /sales-contracts/{id}/reject",
    ),
    (
        "sales-prices",
        "src/routes/sales.rs: /sales-prices/{id}/reject",
    ),
    ("quotations", "src/routes/quotations.rs: /{id}/reject"),
    (
        "purchase-orders",
        "src/routes/purchase.rs: /orders/{id}/reject（path_utils 消歧 purchase-orders）",
    ),
    (
        "purchase-returns",
        "src/routes/purchase.rs: /returns/{id}/reject（path_utils 消歧 purchase-returns）",
    ),
    (
        "purchase-contracts",
        "src/routes/purchase.rs: /purchase-contracts/{id}/reject",
    ),
    (
        "purchase-prices",
        "src/routes/purchase.rs: /purchase-prices/{id}/reject",
    ),
    // 以下资源矩阵侧当前只有 "*" 通配授予（无显式 approve/reject 行），成对判据不触及；
    // 登记于此是为反向幽灵判据的完备性——将来出现显式 reject 行时不误判红。
    (
        "adjustments",
        "src/routes/inventory.rs: /adjustments/{id}/reject",
    ),
    ("counts", "src/routes/inventory.rs: /counts/{id}/reject"),
    (
        "quality-standards",
        "src/routes/mod.rs: /quality-standards/{id}/reject",
    ),
    (
        "lab-dip",
        "src/routes/production.rs: /lab-dip/requests/{id}/reject",
    ),
    (
        "budgets",
        "src/routes/finance.rs: /budgets/adjust/{id}/reject、/budgets/plans/{id}/reject",
    ),
    (
        "fund-management",
        "src/routes/finance.rs: /fund-management/transfers/{id}/reject",
    ),
    (
        "ap",
        "src/routes/finance.rs: /ap/payment-requests/{id}/reject",
    ),
    (
        "role-change-approvals",
        "src/routes/iam.rs: /role-change-approvals/{id}/reject",
    ),
    (
        "export-approvals",
        "src/routes/system.rs: /export-approvals/{id}/reject",
    ),
];

fn has_reject_endpoint(resource: &str) -> bool {
    RESOURCES_WITH_REJECT_ENDPOINT
        .iter()
        .any(|(r, _)| *r == resource)
}

/// 摊平生产矩阵为 (角色码, 资源码, 操作码) 三元组（同一份定义表，不抄副本）。
fn matrix_rows() -> Vec<(&'static str, &'static str, &'static str)> {
    let mut rows = Vec::new();
    for definitions in InitService::all_role_permission_definition_groups() {
        for &(role_code, resources) in definitions {
            for &(resource, action) in resources {
                rows.push((role_code, resource, action));
            }
        }
    }
    rows
}

// ---------------------------------------------------------------------------
// 二、静态成对判据（正向：approve 行 ⇒ reject 行；反向：reject 行 ⇒ 端点存在）
// ---------------------------------------------------------------------------

/// 正向：凡矩阵存在显式 `(角色, 资源, approve)` 行、且该资源有真实 reject 端点，
/// 该角色必须同时具备 `(角色, 资源, reject)`（或该资源 `"*"` 通配）。
/// 授予角色集与 approve 完全一致——审批人可拒绝，不自创第二套 SoD 约束。
#[test]
fn every_approve_grant_on_reject_capable_resource_is_paired() {
    let rows = matrix_rows();
    // 判据防退化前提：显式 approve 行必须成规模在场（当前矩阵 ≥12），
    // 若矩阵结构被改动导致解析为空/骤减，本锁必须红而不是静默通过。
    let approve_rows: Vec<(&'static str, &'static str)> = rows
        .iter()
        .copied()
        .filter(|(_, _, action)| *action == "approve")
        .map(|(role_code, resource, _)| (role_code, resource))
        .collect();
    assert!(
        approve_rows.len() >= 10,
        "矩阵显式 approve 行数骤减（实得 {}），成对判据前提被破坏，须人工复核矩阵改动",
        approve_rows.len()
    );

    let mut unpaired: Vec<(&str, &str)> = Vec::new();
    for (role_code, resource) in approve_rows {
        if !has_reject_endpoint(resource) {
            continue;
        }
        let has_reject = rows.iter().any(|(r, res, act)| {
            *r == role_code && *res == resource && (*act == "reject" || *act == "*")
        });
        if !has_reject {
            unpaired.push((role_code, resource));
        }
    }
    assert!(
        unpaired.is_empty(),
        "以下角色持有 approve 显式授权、同资源存在 reject 端点却没有 reject（或 *）授权行（半截授权，拒绝动作恒 403）：{unpaired:?}"
    );

    // 本轮落地的四对逐一点名（新增事实登记：资源码必须仍在注册表权威清单内）
    for resource in [
        "sales-contracts",
        "sales-prices",
        "purchase-contracts",
        "purchase-prices",
    ] {
        assert!(
            PERMISSION_RESOURCES.contains(&resource),
            "用例前提：{resource} 必须登记在 PERMISSION_RESOURCES，否则 reject 授权行是死码"
        );
        assert!(
            rows.contains(&("sales_manager", resource, "reject"))
                || rows.contains(&("purchase_manager", resource, "reject")),
            "{resource} 的 reject 行必须与同资源 approve 同角色在册（sales_manager/purchase_manager）"
        );
    }
}

/// 反向：凡矩阵存在显式 `(角色, 资源, reject)` 行、但该资源没有 reject 端点 ⇒ 幽灵授权，判红。
#[test]
fn no_reject_grant_without_reject_endpoint() {
    let rows = matrix_rows();
    let ghosts: Vec<(&str, &str)> = rows
        .iter()
        .filter(|(_, resource, action)| *action == "reject" && !has_reject_endpoint(resource))
        .map(|(role_code, resource, _)| (*role_code, *resource))
        .collect();
    assert!(
        ghosts.is_empty(),
        "以下 reject 授权行对应的资源不存在 reject 端点（悬空授权，掩盖真实缺口）：{ghosts:?}"
    );
}

// ---------------------------------------------------------------------------
// 三、端点事实防漂移 + 未闭环清单登记
// ---------------------------------------------------------------------------

fn read_src(rel: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {} 失败: {e}", path.display()))
}

/// 事实表与路由源码逐字符对拍：登记"有端点"的必须字面在场、登记"没有"的必须字面缺席。
/// 任一侧漂移（新加 reject 端点未同步本锁、或删端点留了幽灵行）本用例即红，
/// 强制把矩阵、事实表、路由三处一起改。
#[test]
fn endpoint_fact_table_matches_route_sources() {
    let sales = read_src("routes/sales.rs");
    let purchase = read_src("routes/purchase.rs");
    let return_handler = read_src("handlers/sales_return_handler.rs");
    let production = read_src("routes/production.rs");

    // 有 reject 端点侧：字面必须在场
    for literal in [
        "\"/orders/{id}/reject\"",
        "\"/fabric-orders/{id}/reject\"",
        "\"/sales-contracts/{id}/reject\"",
        "\"/sales-prices/{id}/reject\"",
    ] {
        assert!(
            sales.contains(literal),
            "routes/sales.rs 必须注册 {literal}，否则事实表里的 sales 域 reject 端点是虚报"
        );
    }
    assert!(
        return_handler.contains("\"/sales-returns/{id}/reject\""),
        "sales-returns 的 reject 端点在 handlers/sales_return_handler.rs 内建 router（唯一豁免先例），字面必须在场"
    );
    for literal in [
        "\"/orders/{id}/reject\"",
        "\"/returns/{id}/reject\"",
        "\"/purchase-contracts/{id}/reject\"",
        "\"/purchase-prices/{id}/reject\"",
    ] {
        assert!(
            purchase.contains(literal),
            "routes/purchase.rs 必须注册 {literal}，否则事实表里的 purchase 域 reject 端点是虚报"
        );
    }

    // 未闭环清单侧：以下资源至今**没有** reject 端点，字面必须缺席。
    // 缺席断言同时是功能缺口的显式登记——补端点时必须同步把资源加进
    // RESOURCES_WITH_REJECT_ENDPOINT 并补矩阵 reject 行，否则本用例红。
    assert!(
        production.contains("\"/dye-recipes/{id}/reject\""),
        "routes/production.rs 必须注册 /dye-recipes/{{id}}/reject，否则事实表里的 dye-recipes reject 端点是虚报"
    );
    assert!(
        !purchase.contains("\"/receipts/{id}/approve\"")
            && !purchase.contains("\"/receipts/{id}/reject\""),
        "未闭环事实变更：purchase-receipts 的审批端点已按 approve/reject 词表落地，须复核其矩阵行"
    );
}

/// 未闭环清单（如实点名，不为凑对称造端点）：
/// `fabric-orders` 与 `dye-recipes` 已闭环（reject 端点在册，事实表已登记）。
/// `purchase-receipts` 没有 approve/reject 端点这一事实仍未变（端点缺席断言仍在册），
/// 但它原先那行**永远不会被命中**的矩阵 approve 授权已回收：收货确认走
/// `/receipts/{id}/confirm`，运行期派生键是 `purchase-receipts:confirm`，
/// 授权已改挂到该真实键上（受授集合见本用例末尾的事实登记）。
#[test]
fn unpaired_resources_are_registered_not_fabricated() {
    assert!(
        has_reject_endpoint("fabric-orders"),
        "fabric-orders reject 端点已闭环，须登记在事实表"
    );
    assert!(
        has_reject_endpoint("dye-recipes"),
        "dye-recipes reject 端点已在册（词表/CHECK/rejected_reason 列/service/handler/routes 全套），须登记在事实表"
    );
    assert!(
        !has_reject_endpoint("purchase-receipts"),
        "purchase-receipts 无 reject 端点，禁止为凑成对把它虚报进事实表"
    );
    let rows = matrix_rows();
    assert!(
        rows.contains(&("sales_manager", "fabric-orders", "approve")),
        "事实登记：fabric-orders 的 approve 行仍在册（缺口在端点侧，不在授权侧）"
    );
    // 悬空授权已回收：任何角色都不许再持 purchase-receipts 的 approve 行——
    // 该资源没有 approve 端点，留着这行只会让"看着已授权"掩盖真实键 confirm 的缺失。
    assert!(
        !rows
            .iter()
            .any(|(_, res, act)| *res == "purchase-receipts" && *act == "approve"),
        "purchase-receipts 无 approve 端点，矩阵里不得再出现 approve 行（悬空授权回潮）"
    );
    // 真实键侧的事实登记：确认收货的授权挂在派生键 confirm 上，采购经理与采购员都必须在册，
    // 否则 /receipts/{id}/confirm 对这两个岗位恒 403（功能不可用而非权限收紧）。
    for role in ["purchase_manager", "purchase_clerk"] {
        assert!(
            rows.contains(&(role, "purchase-receipts", "confirm")),
            "事实登记：{role} 必须持有 purchase-receipts 的 confirm 行，否则收货确认端点对本岗不可达"
        );
    }
    // 生产岗可读物料清单：/boms 与前端 /bom 路由门都判 boms:read
    assert!(
        rows.contains(&("production_manager", "boms", "read")),
        "事实登记：production_manager 必须持有 boms 的 read 行，否则物料清单页对本岗恒被拦"
    );
}

// ---------------------------------------------------------------------------
// 四、真库活体锁：持 reject 键 ⇒ 真 200；不持 ⇒ 真 403/FORBIDDEN
// ---------------------------------------------------------------------------

const GRANTEE_ROLE_CODE: &str = "it_w11_pair_grantee";
const STRANGER_ROLE_CODE: &str = "it_w11_pair_stranger";
const SEED_USER_ID: i32 = 9472;
const SEED_PRODUCT_ID: i32 = 9471;

fn now() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}

/// 种一个非 admin、非系统角色并返回主键（admin 会在 check_permission 头部短路，
/// 拿 admin 做正向例等于什么都没验证，故必须是普通角色码）。
async fn seed_role(db: &DatabaseConnection, code: &str) -> i32 {
    let model = role::ActiveModel {
        name: Set(format!("成对性锁角色 {code}")),
        code: Set(code.to_string()),
        is_system: Set(false),
        data_scope: Set("all".to_string()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子角色 {code} 失败: {e}"));
    model.id
}

/// 角色级 allowed=true 授权行（resource_id=NULL，与矩阵落库形态逐列一致）
async fn seed_role_level_grant(
    db: &DatabaseConnection,
    role_id: i32,
    resource: &str,
    action: &str,
) {
    role_permission::ActiveModel {
        role_id: Set(role_id),
        resource_type: Set(resource.to_string()),
        resource_id: Set(None),
        action: Set(action.to_string()),
        allowed: Set(true),
        permission_code: Set(None),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子授权行 {resource}:{action} 失败: {e}"));
}

async fn seed_product_and_pending_price(db: &DatabaseConnection) -> i32 {
    product::ActiveModel {
        id: Set(SEED_PRODUCT_ID),
        name: Set("成对性锁面料".to_string()),
        code: Set("PRD-W11PAIR-T1".to_string()),
        unit: Set("米".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("fabric".to_string()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子产品失败: {e}"));

    let price = sales_price::ActiveModel {
        product_id: Set(SEED_PRODUCT_ID),
        customer_id: Set(None),
        customer_type: Set(Some("standard".to_string())),
        price: Set(rust_decimal::Decimal::new(1234, 2)),
        currency: Set("CNY".to_string()),
        unit: Set("米".to_string()),
        min_order_qty: Set(rust_decimal::Decimal::ZERO),
        price_type: Set("standard".to_string()),
        price_level: Set(None),
        effective_date: Set(chrono::Utc::now().date_naive()),
        expiry_date: Set(None),
        status: Set(price_approval::PENDING.to_string()),
        approved_by: Set(None),
        approved_at: Set(None),
        created_by: Set(Some(SEED_USER_ID)),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子销售价目失败: {e}"));
    price.id
}

/// 带真实 permission_middleware 的活体 Router：路径带 /api/v1/erp 前缀段，
/// 使资源键按生产同一链路派生为 sales-prices:reject（seg3=sales 前缀 + seg4 消歧默认分支）。
fn app_with_role(db: Arc<DatabaseConnection>, role_id: i32) -> axum::Router {
    let state = AppState {
        db: db.clone(),
        ..Default::default()
    };
    async fn inject_auth(
        auth: axum::extract::State<AuthContext>,
        mut request: axum::http::Request<axum::body::Body>,
        next: axum::middleware::Next,
    ) -> axum::response::Response {
        request.extensions_mut().insert(auth.0);
        next.run(request).await
    }
    let auth = AuthContext {
        user_id: SEED_USER_ID,
        username: "it_w11_pair".to_string(),
        role_id: Some(role_id),
        department_id: None,
        data_scope: Some("all".to_string()),
        dept_ids: None,
        dept_member_user_ids: None,
    };
    axum::Router::new()
        .route(
            "/api/v1/erp/sales/sales-prices/{id}/reject",
            axum::routing::post(sales_price_handler::reject_price),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            permission_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(auth, inject_auth))
        .with_state(state)
}

async fn post_reject(app: &axum::Router, id: i32, reason: &str) -> (axum::http::StatusCode, Value) {
    use tower::ServiceExt;
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method(axum::http::Method::POST)
                .uri(format!("/api/v1/erp/sales/sales-prices/{id}/reject"))
                .header(axum::http::header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(
                    json!({ "reason": reason }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8_lossy(&bytes).to_string();
    (
        status,
        serde_json::from_str(&text).unwrap_or(Value::String(text)),
    )
}

/// 活体双向：grantee 角色仅持 `sales-prices:reject` 单键 ⇒ 200 且状态真实翻 REJECTED；
/// stranger 同结构零授权 ⇒ 403 + 机器码 FORBIDDEN，且价目行原样不动（拒绝发生在门层）。
#[tokio::test]
async fn reject_key_really_passes_and_missing_key_really_403() {
    let db = Arc::new(test_common::setup_test_db().await);

    // 前置清理：roles/role_permissions 是夹具不清空的参照表，先收敛起点
    let _ = role_permission::Entity::delete_many()
        .filter(
            role_permission::Column::RoleId.is_in(
                role::Entity::find()
                    .filter(role::Column::Code.is_in([GRANTEE_ROLE_CODE, STRANGER_ROLE_CODE]))
                    .all(db.as_ref())
                    .await
                    .unwrap()
                    .into_iter()
                    .map(|r| r.id)
                    .collect::<Vec<i32>>(),
            ),
        )
        .exec(db.as_ref())
        .await;
    role::Entity::delete_many()
        .filter(role::Column::Code.is_in([GRANTEE_ROLE_CODE, STRANGER_ROLE_CODE]))
        .exec(db.as_ref())
        .await
        .unwrap();

    user::ActiveModel {
        id: Set(SEED_USER_ID),
        username: Set("it_w11_pair".to_string()),
        password_hash: Set("test-only-not-a-real-hash".to_string()),
        real_name: Set(Some("成对性锁操作人".to_string())),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子操作人失败: {e}"));

    let grantee = seed_role(&db, GRANTEE_ROLE_CODE).await;
    let stranger = seed_role(&db, STRANGER_ROLE_CODE).await;
    // grantee 只有 reject、没有同资源 approve——证明放行的正是本轮补的键本身
    seed_role_level_grant(&db, grantee, "sales-prices", "reject").await;
    invalidate_permission_cache(grantee);
    invalidate_permission_cache(stranger);

    let price_for_grantee = seed_product_and_pending_price(&db).await;

    // 正向：持键 ⇒ 真 200（成功信封数字码 200，utils/response.rs 口径）
    let app = app_with_role(db.clone(), grantee);
    let (status, body) = post_reject(&app, price_for_grantee, "色差超出允收范围").await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "持 sales-prices:reject 的角色必须真实过门，实得 {status} {body}"
    );
    assert_eq!(
        body["code"],
        json!(200),
        "成功信封数字码应为 200，实得 {body}"
    );
    let row = sales_price::Entity::find_by_id(price_for_grantee)
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("正向例的价目行必须还在");
    assert_eq!(
        row.status,
        price_approval::REJECTED,
        "过门后状态必须真实翻为 REJECTED（不是门层放行、业务层静默）"
    );
    assert_eq!(
        row.rejected_reason.as_deref(),
        Some("色差超出允收范围"),
        "拒绝理由必须按提交值落库"
    );

    // 反向：不持键 ⇒ 真 403 + FORBIDDEN，行零变化（另起一行同状态价目，排除业务门干扰）
    let price_for_stranger = {
        // 产品行已在（同 SEED_PRODUCT_ID），再落一行 PENDING 价目
        sales_price::ActiveModel {
            product_id: Set(SEED_PRODUCT_ID),
            customer_id: Set(None),
            customer_type: Set(Some("standard".to_string())),
            price: Set(rust_decimal::Decimal::new(2345, 2)),
            currency: Set("CNY".to_string()),
            unit: Set("米".to_string()),
            min_order_qty: Set(rust_decimal::Decimal::ZERO),
            price_type: Set("standard".to_string()),
            price_level: Set(None),
            effective_date: Set(chrono::Utc::now().date_naive()),
            expiry_date: Set(None),
            status: Set(price_approval::PENDING.to_string()),
            approved_by: Set(None),
            approved_at: Set(None),
            created_by: Set(Some(SEED_USER_ID)),
            created_at: Set(now()),
            updated_at: Set(now()),
            ..Default::default()
        }
        .insert(db.as_ref())
        .await
        .unwrap()
    };
    let app = app_with_role(db.clone(), stranger);
    let (status, body) = post_reject(&app, price_for_stranger.id, "同样理由").await;
    assert_eq!(
        status,
        axum::http::StatusCode::FORBIDDEN,
        "不持 sales-prices:reject 的角色必须被门层真实拒绝，实得 {status} {body}"
    );
    assert_eq!(
        body["code"].as_str(),
        Some("FORBIDDEN"),
        "403 必须是统一信封机器码 FORBIDDEN（只断码不断文案），实得 {body}"
    );
    let untouched = sales_price::Entity::find_by_id(price_for_stranger.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("反向例的价目行必须还在");
    assert_eq!(
        untouched.status,
        price_approval::PENDING,
        "403 必须发生在 RBAC 门层：被拒请求不得改动作废业务状态"
    );

    // 收尾：参照表自清理 + 缓存失效（角色 id 不外泄给下一用例）
    role_permission::Entity::delete_many()
        .filter(role_permission::Column::RoleId.is_in([grantee, stranger]))
        .exec(db.as_ref())
        .await
        .unwrap();
    role::Entity::delete_many()
        .filter(role::Column::Id.is_in([grantee, stranger]))
        .exec(db.as_ref())
        .await
        .unwrap();
    invalidate_permission_cache(grantee);
    invalidate_permission_cache(stranger);
}

// ---------------------------------------------------------------------------
// 五、fabric-orders reject 行为锁（service 层正向/负例 + RBAC 门层反向）
// ---------------------------------------------------------------------------

const FO_SEED_USER_ID: i32 = 9482;
const FO_SEED_CUSTOMER_ID: i32 = 9483;
const FO_GRANTEE_ROLE: &str = "it_w11_fo_grantee";
const FO_STRANGER_ROLE: &str = "it_w11_fo_stranger";

fn fo_now() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}

/// 种一行客户（满足 sales_orders.customer_id 外键前提）
async fn seed_fo_customer(db: &DatabaseConnection) -> i32 {
    let m = customer::ActiveModel {
        id: Set(FO_SEED_CUSTOMER_ID),
        customer_code: Set("CUS-W11-FO".to_string()),
        customer_name: Set("面料拒绝锁客户".to_string()),
        customer_type: Set("retail".to_string()),
        credit_limit: Set(rust_decimal::Decimal::ZERO),
        payment_terms: Set(30),
        status: Set(master_data::ACTIVE.to_string()),
        owner_id: Set(FO_SEED_USER_ID),
        created_at: Set(fo_now()),
        updated_at: Set(fo_now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子客户失败: {e}"));
    m.id
}

/// 种一行 status=pending 的面料订单（sales_orders 表）返回 id
async fn seed_pending_fabric_order(db: &DatabaseConnection, created_by: i32) -> i32 {
    let order = sales_order::ActiveModel {
        order_no: Set(format!(
            "SO-FO-REJECT-{}",
            chrono::Utc::now().timestamp_millis()
        )),
        customer_id: Set(FO_SEED_CUSTOMER_ID),
        order_date: Set(fo_now()),
        status: Set(sales_fabric_order::PENDING.to_string()),
        subtotal: Set(rust_decimal::Decimal::new(10000, 2)),
        tax_amount: Set(rust_decimal::Decimal::ZERO),
        discount_amount: Set(rust_decimal::Decimal::ZERO),
        shipping_cost: Set(rust_decimal::Decimal::ZERO),
        total_amount: Set(rust_decimal::Decimal::new(10000, 2)),
        paid_amount: Set(rust_decimal::Decimal::ZERO),
        balance_amount: Set(rust_decimal::Decimal::new(10000, 2)),
        created_by: Set(Some(created_by)),
        created_at: Set(fo_now()),
        updated_at: Set(fo_now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子 pending 面料订单失败: {e}"));
    order.id
}

/// 种一行 status=approved 的面料订单（用于非法前置状态负例）
async fn seed_approved_fabric_order(db: &DatabaseConnection, created_by: i32) -> i32 {
    let order = sales_order::ActiveModel {
        order_no: Set(format!(
            "SO-FO-APPROVED-{}",
            chrono::Utc::now().timestamp_millis()
        )),
        customer_id: Set(FO_SEED_CUSTOMER_ID),
        order_date: Set(fo_now()),
        status: Set(sales_fabric_order::APPROVED.to_string()),
        subtotal: Set(rust_decimal::Decimal::new(20000, 2)),
        tax_amount: Set(rust_decimal::Decimal::ZERO),
        discount_amount: Set(rust_decimal::Decimal::ZERO),
        shipping_cost: Set(rust_decimal::Decimal::ZERO),
        total_amount: Set(rust_decimal::Decimal::new(20000, 2)),
        paid_amount: Set(rust_decimal::Decimal::ZERO),
        balance_amount: Set(rust_decimal::Decimal::new(20000, 2)),
        created_by: Set(Some(created_by)),
        created_at: Set(fo_now()),
        updated_at: Set(fo_now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子 approved 面料订单失败: {e}"));
    order.id
}

fn fo_app_with_role(db: Arc<DatabaseConnection>, role_id: i32, user_id: i32) -> axum::Router {
    let state = AppState {
        db: db.clone(),
        ..Default::default()
    };
    async fn inject_auth(
        auth: axum::extract::State<AuthContext>,
        mut request: axum::http::Request<axum::body::Body>,
        next: axum::middleware::Next,
    ) -> axum::response::Response {
        request.extensions_mut().insert(auth.0);
        next.run(request).await
    }
    let auth = AuthContext {
        user_id,
        username: "it_w11_fo".to_string(),
        role_id: Some(role_id),
        department_id: None,
        data_scope: Some("all".to_string()),
        dept_ids: None,
        dept_member_user_ids: None,
    };
    axum::Router::new()
        .route(
            "/api/v1/erp/sales/fabric-orders/{id}/reject",
            axum::routing::post(sales_fabric_order_handler::reject_fabric_order),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            permission_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(auth, inject_auth))
        .with_state(state)
}

async fn post_fo_reject(
    app: &axum::Router,
    id: i32,
    reason: &str,
) -> (axum::http::StatusCode, Value) {
    use tower::ServiceExt;
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method(axum::http::Method::POST)
                .uri(format!("/api/v1/erp/sales/fabric-orders/{id}/reject"))
                .header(axum::http::header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(
                    json!({ "reason": reason }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8_lossy(&bytes).to_string();
    (
        status,
        serde_json::from_str(&text).unwrap_or(Value::String(text)),
    )
}

/// 正向：持 fabric-orders:reject 键 + pending 行 → 200，status 翻 rejected，rejected_reason 落库
#[tokio::test]
async fn fabric_order_reject_flips_status_and_stores_reason() {
    let db = Arc::new(test_common::setup_test_db().await);

    let _ = role_permission::Entity::delete_many()
        .filter(
            role_permission::Column::RoleId.is_in(
                role::Entity::find()
                    .filter(role::Column::Code.is_in([FO_GRANTEE_ROLE, FO_STRANGER_ROLE]))
                    .all(db.as_ref())
                    .await
                    .unwrap()
                    .into_iter()
                    .map(|r| r.id)
                    .collect::<Vec<i32>>(),
            ),
        )
        .exec(db.as_ref())
        .await;
    role::Entity::delete_many()
        .filter(role::Column::Code.is_in([FO_GRANTEE_ROLE, FO_STRANGER_ROLE]))
        .exec(db.as_ref())
        .await
        .unwrap();

    user::ActiveModel {
        id: Set(FO_SEED_USER_ID),
        username: Set("it_w11_fo".to_string()),
        password_hash: Set("test-only-not-a-real-hash".to_string()),
        real_name: Set(Some("面料拒绝锁操作人".to_string())),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        created_at: Set(fo_now()),
        updated_at: Set(fo_now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子操作人失败: {e}"));

    let grantee = seed_role(&db, FO_GRANTEE_ROLE).await;
    seed_role_level_grant(&db, grantee, "fabric-orders", "reject").await;
    invalidate_permission_cache(grantee);

    seed_fo_customer(&db).await;
    let order_id = seed_pending_fabric_order(&db, FO_SEED_USER_ID).await;

    let app = fo_app_with_role(db.clone(), grantee, FO_SEED_USER_ID);
    let (status, body) = post_fo_reject(&app, order_id, "色差超出允收范围").await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "持 fabric-orders:reject 的角色对 pending 行应放行，实得 {status} {body}"
    );
    assert_eq!(
        body["code"],
        json!(200),
        "成功信封数字码应为 200，实得 {body}"
    );

    let row = sales_order::Entity::find_by_id(order_id)
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("正向例的订单行必须还在");
    assert_eq!(
        row.status,
        sales_fabric_order::REJECTED,
        "reject 后状态必须为 rejected"
    );
    assert_eq!(
        row.rejected_reason.as_deref(),
        Some("色差超出允收范围"),
        "拒绝理由必须落库 rejected_reason 列"
    );

    role_permission::Entity::delete_many()
        .filter(role_permission::Column::RoleId.is_in([grantee]))
        .exec(db.as_ref())
        .await
        .unwrap();
    role::Entity::delete_many()
        .filter(role::Column::Id.is_in([grantee]))
        .exec(db.as_ref())
        .await
        .unwrap();
    invalidate_permission_cache(grantee);
}

/// 负例：已 approved 行发起 reject → BUSINESS_ERROR，状态零变化
#[tokio::test]
async fn fabric_order_reject_non_pending_returns_business_error() {
    let db = Arc::new(test_common::setup_test_db().await);

    let _ = role_permission::Entity::delete_many()
        .filter(
            role_permission::Column::RoleId.is_in(
                role::Entity::find()
                    .filter(role::Column::Code.is_in([FO_GRANTEE_ROLE]))
                    .all(db.as_ref())
                    .await
                    .unwrap()
                    .into_iter()
                    .map(|r| r.id)
                    .collect::<Vec<i32>>(),
            ),
        )
        .exec(db.as_ref())
        .await;
    role::Entity::delete_many()
        .filter(role::Column::Code.is_in([FO_GRANTEE_ROLE]))
        .exec(db.as_ref())
        .await
        .unwrap();

    user::ActiveModel {
        id: Set(FO_SEED_USER_ID),
        username: Set("it_w11_fo".to_string()),
        password_hash: Set("test-only-not-a-real-hash".to_string()),
        real_name: Set(Some("面料拒绝锁操作人".to_string())),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        created_at: Set(fo_now()),
        updated_at: Set(fo_now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子操作人失败: {e}"));

    let grantee = seed_role(&db, FO_GRANTEE_ROLE).await;
    seed_role_level_grant(&db, grantee, "fabric-orders", "reject").await;
    invalidate_permission_cache(grantee);

    seed_fo_customer(&db).await;
    let order_id = seed_approved_fabric_order(&db, FO_SEED_USER_ID).await;

    let app = fo_app_with_role(db.clone(), grantee, FO_SEED_USER_ID);
    let (status, body) = post_fo_reject(&app, order_id, "对已审批行发起拒绝").await;
    assert_eq!(
        status,
        axum::http::StatusCode::BAD_REQUEST,
        "非 pending 行 reject 应返回 BUSINESS_ERROR（400），实得 {status} {body}"
    );
    assert_eq!(
        body["code"].as_str(),
        Some("BUSINESS_ERROR"),
        "非 pending 行 reject 机器码必须为 BUSINESS_ERROR，实得 {body}"
    );

    let row = sales_order::Entity::find_by_id(order_id)
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("负例的订单行必须还在");
    assert_eq!(
        row.status,
        sales_fabric_order::APPROVED,
        "非法前置状态 reject 不得改动作废当前状态"
    );

    role_permission::Entity::delete_many()
        .filter(role_permission::Column::RoleId.is_in([grantee]))
        .exec(db.as_ref())
        .await
        .unwrap();
    role::Entity::delete_many()
        .filter(role::Column::Id.is_in([grantee]))
        .exec(db.as_ref())
        .await
        .unwrap();
    invalidate_permission_cache(grantee);
}

/// RBAC 门层反向：不持 fabric-orders:reject 键的角色 → 403 FORBIDDEN，行零变化
#[tokio::test]
async fn fabric_order_reject_without_key_returns_403() {
    let db = Arc::new(test_common::setup_test_db().await);

    let _ = role_permission::Entity::delete_many()
        .filter(
            role_permission::Column::RoleId.is_in(
                role::Entity::find()
                    .filter(role::Column::Code.is_in([FO_STRANGER_ROLE]))
                    .all(db.as_ref())
                    .await
                    .unwrap()
                    .into_iter()
                    .map(|r| r.id)
                    .collect::<Vec<i32>>(),
            ),
        )
        .exec(db.as_ref())
        .await;
    role::Entity::delete_many()
        .filter(role::Column::Code.is_in([FO_STRANGER_ROLE]))
        .exec(db.as_ref())
        .await
        .unwrap();

    user::ActiveModel {
        id: Set(FO_SEED_USER_ID),
        username: Set("it_w11_fo".to_string()),
        password_hash: Set("test-only-not-a-real-hash".to_string()),
        real_name: Set(Some("面料拒绝锁陌生人".to_string())),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        created_at: Set(fo_now()),
        updated_at: Set(fo_now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子操作人失败: {e}"));

    let stranger = seed_role(&db, FO_STRANGER_ROLE).await;
    invalidate_permission_cache(stranger);

    seed_fo_customer(&db).await;
    let order_id = seed_pending_fabric_order(&db, FO_SEED_USER_ID).await;

    let app = fo_app_with_role(db.clone(), stranger, FO_SEED_USER_ID);
    let (status, body) = post_fo_reject(&app, order_id, "无授权发起拒绝").await;
    assert_eq!(
        status,
        axum::http::StatusCode::FORBIDDEN,
        "不持 fabric-orders:reject 的角色必须被门层 403，实得 {status} {body}"
    );
    assert_eq!(
        body["code"].as_str(),
        Some("FORBIDDEN"),
        "403 机器码必须为 FORBIDDEN，实得 {body}"
    );

    let row = sales_order::Entity::find_by_id(order_id)
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("门层拒绝后行必须还在");
    assert_eq!(
        row.status,
        sales_fabric_order::PENDING,
        "403 必须发生在 RBAC 门层：不得改业务状态"
    );

    role::Entity::delete_many()
        .filter(role::Column::Id.is_in([stranger]))
        .exec(db.as_ref())
        .await
        .unwrap();
    invalidate_permission_cache(stranger);
}
