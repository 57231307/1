//! 契约测试：付款申请详情出口的字段权限必须同源约束 items 明细出口
//!
//! 缺陷形态（本文件防的回归）：详情出口先对主单出参做字段过滤、之后才查询并
//! 挂载明细数组；字段过滤是顶层操作、不递归进入数组，于是白名单模式的
//! allowed_fields 触达不到明细行（白名单角色从详情拿到明细全字段），兜底模式
//! 主单申请金额被摘的同时明细行金额键原样发出（明细金额是申请金额在应付单
//! 维度的分摊构成，已补明细的单各行之和即实际申请金额，只隐主单等于没隐）。
//!
//! 锁定口径：
//! 1. 明细的查询与逐行过滤均先于任何合并出参构造（挂载在权限块之后只允许
//!    发生在明细行已被同一份权限结果处理之后）；
//! 2. 无权限配置行且非 admin（含权限查询失败走同一默认处理）：主单隐藏
//!    request_amount / request_amount_foreign / bank_account / bank_name，
//!    明细每行隐藏 apply_amount；invoice_id / notes 属分摊归属信息不隐，
//!    明细行容器保留（不删列口径）；
//! 3. 白名单模式：明细行与主单受同一份 allowed_fields 逐行约束，白名单外
//!    的明细键不得外泄；配置行 hidden_fields 同样落到明细行；
//! 4. admin（roles.code='admin'，判定源 admin_checker::is_admin_role）与
//!    配置行放行的角色保持现语义原文发出；
//! 5. role_id 缺失分支保持现语义不做字段处理（权限中间件在更早处拒无角色
//!    请求，本分支是纵深防御层；收紧属行为变更、另行裁定，不在本锁范围）；
//! 6. handler 内隐藏键字面量唯一出处是两张常量表，不得出现第三份。
//!
//! 断言纪律：隐藏 = 整键移除，断键存在性而非值；必须断到 items 明细行——
//! 只断主单不可见对缺陷形态无区分力（既有兜底本来就摘主单键）。
//!
//! 通道：行为锁经 test_common::setup_test_db 连已迁移 PostgreSQL 真跑
//! （缺 TEST_DATABASE_URL 直接 panic，CI 侧执行）。roles 与 suppliers 为迁移
//! 播种参照表不再插（roles：id=1 code='admin'、id=2/3 为非 admin 参照行；
//! suppliers 取真实首个 id 作外键父行）；ap_invoice 为明细行 invoice_id 的
//! NOT NULL 真外键父行，自种子。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::get,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::ap_payment_request_handler::get_request;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::services::data_permission_service::DataPermissionService;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// 种子与脚手架
// ---------------------------------------------------------------------------

const USER_PLAIN: i32 = 50; // 角色 2（manager）：无 ap_payment_request 权限配置行
const USER_WL: i32 = 51; // 角色 3（operator）：配置行由用例写入
const USER_ADMIN: i32 = 70; // 角色 1（admin）

const ROLE_PLAIN: i32 = 2;
const ROLE_WL: i32 = 3;
const ROLE_ADMIN: i32 = 1;

const REQUEST_ID: i32 = 1;

fn make_auth(user_id: i32, role_id: Option<i32>) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("payreq_fs_user_{user_id}"),
        role_id,
        department_id: Some(1),
        // 行级可见集取 ALL：本文件锁字段级出口，行级 IDOR 已由既有门控独立覆盖
        data_scope: Some("ALL".to_string()),
        dept_ids: Some(Arc::new("1".to_string())),
        dept_member_user_ids: Some(Arc::new("50".to_string())),
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

async fn exec(db: &sea_orm::DatabaseConnection, sql: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        Vec::<sea_orm::Value>::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("种子执行失败: {e}\nSQL: {sql}"));
}

/// 迁移播种的 suppliers 参照表取真实首个 id 作外键父行（不硬编码常量）
async fn seeded_supplier_id(db: &sea_orm::DatabaseConnection) -> i32 {
    let row = db
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT id FROM suppliers ORDER BY id LIMIT 1".to_string(),
        ))
        .await
        .expect("夹具：读取 suppliers 参照表失败")
        .unwrap_or_else(|| panic!("夹具：迁移未播种 suppliers 参照表，FK 父行缺失"));
    row.try_get_by_index::<i32>(0)
        .expect("夹具：suppliers.id 应可解码为 i32")
}

/// 种子链路：supplier（参照行）→ ap_payment_request（含可断言的四键原值）→
/// ap_invoice 两张（明细外键父行）→ ap_payment_request_item 两行（apply_amount
/// 之和等于表头申请金额的已补明细形态）。
/// `allowed_json` / `hidden_json` 非 None 时为角色 3 写入配置行。
async fn seeded_db(
    allowed_json: Option<Value>,
    hidden_json: Option<Value>,
) -> Arc<sea_orm::DatabaseConnection> {
    let db = test_common::setup_test_db().await;
    let sid = seeded_supplier_id(&db).await;

    exec(
        &db,
        &format!(
            "INSERT INTO ap_payment_request \
             (id, request_no, request_date, supplier_id, payment_type, payment_method, \
              request_amount, request_amount_foreign, bank_name, bank_account, created_by) \
             VALUES ({REQUEST_ID}, 'PR-FS-0001', '2026-01-15', {sid}, 'NORMAL', 'TT', \
              '1000.50', '700.35', '测试收款银行', '6222020000000001', {USER_PLAIN})"
        ),
    )
    .await;

    exec(
        &db,
        &format!(
            "INSERT INTO ap_invoice \
             (id, invoice_no, supplier_id, invoice_type, invoice_date, due_date, \
              amount, paid_amount, unpaid_amount, invoice_status, created_by) \
             VALUES \
             (1, 'FS-INV-1', {sid}, 'PURCHASE', '2026-01-01', '2026-12-15', \
              '800.00', '0.00', '800.00', 'AUDITED', {USER_PLAIN}), \
             (2, 'FS-INV-2', {sid}, 'PURCHASE', '2026-01-02', '2026-12-15', \
              '500.00', '0.00', '500.00', 'AUDITED', {USER_PLAIN})"
        ),
    )
    .await;

    exec(
        &db,
        "INSERT INTO ap_payment_request_item (request_id, invoice_id, apply_amount, notes) \
         VALUES (1, 1, '300.20', '首付分摊'), (1, 2, '700.30', '尾款分摊')",
    )
    .await;

    if allowed_json.is_some() || hidden_json.is_some() {
        let svc = DataPermissionService::new(Arc::new(db.clone()));
        svc.set_data_permission(
            ROLE_WL,
            "ap_payment_request".to_string(),
            "ALL".to_string(),
            None,
            allowed_json,
            hidden_json,
        )
        .await
        .expect("夹具：数据权限配置行写入失败");
    }
    Arc::new(db)
}

fn state_from(db: &Arc<sea_orm::DatabaseConnection>) -> AppState {
    AppState {
        db: db.clone(),
        data_permission_service: Arc::new(DataPermissionService::new(db.clone())),
        ..Default::default()
    }
}

fn build_app(db: &Arc<sea_orm::DatabaseConnection>, auth: AuthContext) -> Router {
    Router::new()
        .route("/ap/payment-requests/{id}", get(get_request))
        .with_state(state_from(db))
        .layer(from_fn_with_state(auth, inject_auth))
}

async fn detail_data(app: &Router) -> Value {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/ap/payment-requests/{REQUEST_ID}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "详情 200 契约");
    let bytes = axum::body::to_bytes(resp.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap();
    let v: Value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|e| panic!("响应非 JSON: {e}; body={}", String::from_utf8_lossy(&bytes)));
    assert_eq!(v["code"], json!(200), "成功信封 code 契约: {v}");
    v["data"].clone()
}

/// 取详情出参 items 数组并断容器存在且行数与种子一致（不删列口径的地基断言）
fn items_of(data: &Value) -> Vec<Value> {
    data.get("items")
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("详情出参必须携带 items 明细容器（不删列口径）: {data}"))
        .clone()
}

fn assert_items_shape(items: &[Value]) {
    assert_eq!(items.len(), 2, "明细行数与种子不符: {items:?}");
}

// ---------------------------------------------------------------------------
// 行为锁（真库，CI 侧执行）
// ---------------------------------------------------------------------------

/// 无权限配置行且非 admin：主单四键与明细行金额键必须**同时**被摘——
/// 缺陷形态下主单四键本来就被摘、泄漏全在明细行，故断言重心在 items。
#[tokio::test]
async fn detail_without_permission_row_hides_amount_on_main_and_items() {
    let db = seeded_db(None, None).await;
    let app = build_app(&db, make_auth(USER_PLAIN, Some(ROLE_PLAIN)));
    let data = detail_data(&app).await;

    for key in [
        "request_amount",
        "request_amount_foreign",
        "bank_account",
        "bank_name",
    ] {
        assert!(
            data.get(key).is_none(),
            "主单金额/账户键 {key} 应被默认处理整键移除: {data}"
        );
    }

    let items = items_of(&data);
    assert_items_shape(&items);
    for (idx, item) in items.iter().enumerate() {
        assert!(
            item.get("apply_amount").is_none(),
            "明细第 {idx} 行金额键 apply_amount 应随主单金额同源隐藏（只断主单会假绿）: {item}"
        );
        assert!(
            item.get("invoice_id").is_some(),
            "分摊归属信息 invoice_id 非金额类不得隐藏: {item}"
        );
        assert!(
            item.get("notes").is_some(),
            "notes 非金额类不得隐藏: {item}"
        );
    }
}

/// 白名单模式：明细行与主单受同一份 allowed_fields 约束，
/// 白名单外的明细键（含金额键）一律不外泄；容器行保留。
#[tokio::test]
async fn detail_whitelist_without_item_keys_does_not_leak_item_fields() {
    let allowed = json!(["id", "request_no", "supplier_id", "approval_status"]);
    let db = seeded_db(Some(allowed), None).await;
    let app = build_app(&db, make_auth(USER_WL, Some(ROLE_WL)));
    let data = detail_data(&app).await;

    // 主单按白名单保留
    assert_eq!(data["request_no"], json!("PR-FS-0001"));
    assert!(
        data.get("request_amount").is_none(),
        "主单白名单外键必须移除"
    );

    let items = items_of(&data);
    assert_items_shape(&items);
    for (idx, item) in items.iter().enumerate() {
        assert!(
            item.get("apply_amount").is_none(),
            "白名单模式明细第 {idx} 行金额键泄漏（缺陷面之二）: {item}"
        );
        assert!(
            item.get("invoice_id").is_none(),
            "白名单模式明细第 {idx} 行白名单外键泄漏: {item}"
        );
        assert!(
            item.get("request_id").is_none(),
            "白名单外 request_id 不得外泄: {item}"
        );
        assert!(
            item.get("notes").is_none(),
            "白名单外 notes 不得外泄: {item}"
        );
        // 同名列 id 命中白名单：证明明细确实过了同一份白名单，而非整体消失或整体放行
        assert!(
            item.get("id").is_some(),
            "同名列 id 应命中白名单保留: {item}"
        );
    }
}

/// 配置行 hidden_fields：与主单同一份清单同样落到明细行
#[tokio::test]
async fn detail_hidden_fields_config_applies_to_item_rows() {
    let hidden = json!(["request_amount", "apply_amount"]);
    let db = seeded_db(None, Some(hidden)).await;
    let app = build_app(&db, make_auth(USER_WL, Some(ROLE_WL)));
    let data = detail_data(&app).await;

    assert!(data.get("request_amount").is_none(), "配置行隐藏主单金额键");
    assert!(
        data.get("bank_name").is_some(),
        "未配置隐藏的主单键保持可见"
    );

    let items = items_of(&data);
    assert_items_shape(&items);
    for item in &items {
        assert!(
            item.get("apply_amount").is_none(),
            "配置行隐藏应触达明细行: {item}"
        );
        assert!(
            item.get("invoice_id").is_some(),
            "未配置隐藏的明细键保持可见: {item}"
        );
    }
}

/// admin 按现语义放行：主单金额与明细金额原文外显（十进制字符串形状）
#[tokio::test]
async fn detail_admin_keeps_amount_on_main_and_items() {
    let db = seeded_db(None, None).await;
    let app = build_app(&db, make_auth(USER_ADMIN, Some(ROLE_ADMIN)));
    let data = detail_data(&app).await;

    assert_eq!(
        data["request_amount"],
        json!("1000.50"),
        "admin 主单金额原文"
    );

    let items = items_of(&data);
    assert_items_shape(&items);
    let mut amounts: Vec<String> = items
        .iter()
        .map(|item| {
            item.get("apply_amount")
                .and_then(Value::as_str)
                .unwrap_or_else(|| panic!("admin 明细金额键应保留且为十进制字符串: {item}"))
                .to_string()
        })
        .collect();
    amounts.sort();
    assert_eq!(amounts, vec!["300.20".to_string(), "700.30".to_string()]);
}

// ---------------------------------------------------------------------------
// 静态锁（源码扫描，无需 DB）
// ---------------------------------------------------------------------------

const HANDLER_RAW: &str = include_str!("../src/handlers/ap_payment_request_handler.rs");

/// 剔整行注释（与仓内既有 code_only 先例同款：注释不参与语句形态判定），
/// 并按 LF 口径归一行尾后再返回可扫描的代码文本
fn handler_code() -> String {
    let lf = HANDLER_RAW.replace('\r', "");
    code_only(&lf)
}

/// 剔整行注释（与仓内既有 code_only 先例同款：注释不参与语句形态判定）
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 取 get_request 函数体（自签名起、到下一个 pub async fn 为界）
fn get_request_body(code: &str) -> &str {
    let sig = "pub async fn get_request(";
    let start = code.find(sig).expect("handler 必须存在 get_request 出口");
    let rest = &code[start..];
    let end = rest[sig.len()..]
        .find("pub async fn")
        .map(|p| p + sig.len())
        .unwrap_or(rest.len());
    &rest[..end]
}

/// 明细查询/逐行过滤与合并出参构造的顺序判据：违规返回描述清单，全过返回空。
/// 钉死的语句形态：
/// - 明细查询 ap_payment_request_item::Entity::find() 必须先于权限块起点
///   （get_role_data_permission），即明细出参处于权限块可达范围；
/// - 明细白名单过滤 filter_fields_batch( 与明细兜底过滤
///   DEFAULT_HIDDEN_ITEM_FIELDS 必须先于挂载语句 request_json["items"]；
/// - 三个锚点任一缺失即结构漂移，同样判违规（缺陷形态恰好缺后两者）。
fn ordering_violations(body: &str) -> Vec<&'static str> {
    let mut violations = Vec::new();
    let item_find = body.find("ap_payment_request_item::Entity::find()");
    let perm_lookup = body.find("get_role_data_permission(");
    let items_whitelist = body.find("filter_fields_batch(");
    let items_fallback = body.find("DEFAULT_HIDDEN_ITEM_FIELDS");
    let attach = body.find("request_json[\"items\"]");

    let (Some(find), Some(lookup)) = (item_find, perm_lookup) else {
        violations.push("明细查询或权限块锚点缺失");
        return violations;
    };
    if find > lookup {
        violations.push("明细查询晚于权限块起点（先过滤后查挂明细的缺陷形态复现）");
    }
    let Some(at) = attach else {
        violations.push("明细挂载语句缺失（items 出口形态漂移）");
        return violations;
    };
    match items_whitelist {
        Some(p) => {
            if p > at {
                violations.push("明细白名单过滤晚于合并出参构造");
            }
        }
        None => violations.push("明细白名单逐行过滤缺失（filter_fields_batch 未作用于明细行）"),
    }
    match items_fallback {
        Some(p) => {
            if p > at {
                violations.push("明细兜底隐藏晚于合并出参构造");
            }
        }
        None => violations.push("明细兜底隐藏缺失（DEFAULT_HIDDEN_ITEM_FIELDS 未作用于明细行）"),
    }
    violations
}

/// 真实 handler：顺序判据必须零违规
#[test]
fn static_get_request_item_filter_precedes_attach() {
    let code = handler_code();
    let body = get_request_body(&code);
    let violations = ordering_violations(body);
    assert!(
        violations.is_empty(),
        "get_request 明细出口顺序违规: {violations:?}"
    );
}

/// 夹具自证：缺陷形态样本必须被同一判据抓出，合规形态样本必须零违规——
/// 证明判据对该顺序缺陷有区分力（否则真源恒绿也是假绿）
#[test]
fn static_ordering_guard_distinguishes_buggy_shape() {
    let buggy = r#"
pub async fn get_request() {
    let mut request_json = serde_json::to_value(request)?;
    if let Some(role_id) = auth.role_id {
        let permission = state
            .data_permission_service
            .get_role_data_permission(role_id, "ap_payment_request")
            .await;
        if let Some(permission) = permission {
            state.data_permission_service.filter_fields(&mut request_json, &a, &h);
        } else if !admin_checker::is_admin_role(&state.db, role_id).await {
            obj.remove("request_amount");
        }
    }
    let items = crate::models::ap_payment_request_item::Entity::find()
        .all(&*state.db)
        .await?;
    request_json["items"] = serde_json::to_value(items)?;
}
"#;
    let violations = ordering_violations(buggy);
    assert!(
        violations
            .iter()
            .any(|v| v.contains("明细查询晚于权限块起点")),
        "判据必须抓出先过滤后挂明细的缺陷形态: {violations:?}"
    );
    assert!(
        violations.iter().any(|v| v.contains("白名单逐行过滤缺失")),
        "判据必须抓出明细未过白名单的缺陷形态: {violations:?}"
    );

    let good = r#"
pub async fn get_request() {
    let mut request_json = serde_json::to_value(request)?;
    let items = crate::models::ap_payment_request_item::Entity::find()
        .all(&*state.db)
        .await?;
    let mut items_json: Vec<JsonValue> = collect(items);
    if let Some(role_id) = auth.role_id {
        let permission = state
            .data_permission_service
            .get_role_data_permission(role_id, "ap_payment_request")
            .await;
        if let Some(permission) = permission {
            state.data_permission_service.filter_fields(&mut request_json, &a, &h);
            state.data_permission_service.filter_fields_batch(&mut items_json, &a, &h);
        } else if !admin_checker::is_admin_role(&state.db, role_id).await {
            hide_fields(&mut request_json, &DEFAULT_HIDDEN_REQUEST_FIELDS);
            for item in &mut items_json {
                hide_fields(item, &DEFAULT_HIDDEN_ITEM_FIELDS);
            }
        }
    }
    request_json["items"] = JsonValue::Array(items_json);
}
"#;
    assert!(ordering_violations(good).is_empty(), "合规形态必须零违规");
}

/// 隐藏键字面量单源钉：五个隐藏键字符串字面量在整个 handler 内各只允许出现
/// 一次（即两张常量表的定义处），列表兜底、详情兜底主单、详情明细行三个
/// 引用点一律经常量表取列集——handler 内不得存在第三份字面量清单。
#[test]
fn static_hidden_key_literals_defined_once_only() {
    let code = handler_code();
    // 判据只统计与之完全相等的字符串字面量 token：注释已被剔除，
    // 字段访问形态（request.request_amount 之类无引号）天然不计入；
    // request_amount_foreign 带独立闭合引号，不会与前缀键的判定重叠。
    let count_token = |needle: &str| -> usize {
        let mut n = 0usize;
        let mut rest = code.as_str();
        while let Some(p) = rest.find(needle) {
            n += 1;
            rest = &rest[p + needle.len()..];
        }
        n
    };
    for key in [
        "\"request_amount\"",
        "\"request_amount_foreign\"",
        "\"bank_account\"",
        "\"bank_name\"",
        "\"apply_amount\"",
    ] {
        assert_eq!(
            count_token(key),
            1,
            "隐藏键 {key} 在 handler 内的字面量出现次数应为常量表唯一定义"
        );
    }
}
