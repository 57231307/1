//! 波0 契约锁：customers.customer_type 唯一词表模块与六个写入口的等价/旁路封堵
//!
//! 锁定内容（对应派工第 6 项 ①-⑤）：
//! ① 五个写入口对**同一非法值**（`"RETAIL"`、`"normal"`）拒绝形态逐一对齐：
//!    HTTP 400 + 信封 `code=VALIDATION_ERROR`（字段校验=VALIDATION 族；pool 规则入口
//!    是另一张表的规则作用域，允许集独立，只对齐"族别/状态码"这一层，message 不参与
//!    对齐声明）。四个 customers 写入口 message 逐字符对齐
//!    `customer_type: invalid_customer_type`（validator 通道与模块通道同一构造函数）。
//! ② 无校验旁路已封：增强 PUT（crm_customer_handler::update_customer，波0 前裸
//!    Deserialize 直落库）与 ConvertLeadRequest（波0 前连 Validate derive 都没有）各一条
//!    "非法值被拒"正例。校验置于任何触库之前 ⇒ `AppState::default()`（Disconnected
//!    哨兵）即可测；若把入口改回"不校验"，这两笔会直接触库（哨兵 panic/透传落库），
//!    本文件必红。
//! ③ 导入写入值 ∈ ALLOWED：钉死 `import.rs` 客户导入写 `constants::customer_type::RETAIL`
//!    （小写真实 token）；写回大写 `"RETAIL"` 字面量即红（源码扫描 + 集合成员双锁）。
//! ④ 唯一模块与标准入口行为等价：同一输入表逐值比对"validator 通道结果"与
//!    "模块 validate 结果"（现树函数产生，不手抄期望）——接受/拒绝一致、拒绝信封一致、
//!    通过值原文返回（不 trim、不归一大小写）。
//! ⑤ 登记性负例：转化缺省 `"POTENTIAL"` **不属** ALLOWED（本波待裁不改值，只钉成
//!    "已登记的大写脏值来源"；不许为了绿把 POTENTIAL 加进 ALLOWED）。
//!    `potential` 这个 token 已被 CLV 分层占用（services/crm/cust.rs:670-680
//!    segment=champion/loyal/potential/at_risk/lost；models/customer_lifetime_value.rs:40），
//!    属混维撞名，等业务口径裁定后统一处理。
//!
//! 运行前提：①② 仅依赖"validator/校验先于触库"，无 DB；③④⑤ 纯静态/纯函数，无 DB。

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::{IntoResponse, Response},
    routing::{post, put},
};
use bingxi_backend::constants::customer_type;
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::crm_customer_handler;
use bingxi_backend::handlers::crm_handler;
use bingxi_backend::handlers::crm_pool_handler;
use bingxi_backend::handlers::customer_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::utils::error::AppError;
use serde_json::Value;
use tower::ServiceExt;
use validator::Validate;

const CUSTOMERS_TYPE_MSG: &str = "customer_type: invalid_customer_type";

fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("e2e_user_{user_id}"),
        role_id: Some(2),
        department_id: Some(1),
        data_scope: Some("self".to_string()),
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

/// 五个写入口挂在同一个无 DB Router 上（校验全部先于触库，见各 handler 波0 注释）。
/// customers 域四个写入口 + pool 规则入口（不同表/不同词表，仅族别对齐）。
fn build_app() -> Router {
    Router::new()
        .route("/customers", post(customer_handler::create_customer))
        .route("/customers/{id}", put(customer_handler::update_customer))
        .route(
            "/crm/customers/enhanced/{id}",
            put(crm_customer_handler::update_customer),
        )
        .route("/crm/leads/{id}/convert", post(crm_handler::convert_lead))
        .route("/crm/pool/rules", post(crm_pool_handler::create_pool_rule))
        .with_state(AppState::default())
        .layer(from_fn_with_state(make_auth(100), inject_auth))
}

async fn send_json(app: &Router, method: &str, uri: &str, body: &str) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
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
    (status, serde_json::from_slice(&bytes).unwrap())
}

async fn envelope_of(err: AppError) -> (StatusCode, Value) {
    let resp = err.into_response();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

// =========================================================
// ① 五个写入口 × 同一非法值：拒绝族别逐一对齐（400 + VALIDATION_ERROR）
// =========================================================

/// 每条返回 (入口名, status, code, message)
async fn all_entries_reject(
    app: &Router,
    invalid: &str,
) -> Vec<(String, StatusCode, String, String)> {
    let body_type = format!(r#"{{"customer_type":"{invalid}"}}"#);
    let mut out = Vec::new();

    // 1) 标准创建：payload.validate() 先于一切（customer_handler.rs）
    let body = format!(r#"{{"customer_name":"契约锁客户","customer_type":"{invalid}"}}"#);
    let (status, v) = send_json(app, "POST", "/customers", &body).await;
    out.push((
        "POST /customers".to_string(),
        status,
        v["code"].as_str().unwrap_or_default().to_string(),
        v["message"].as_str().unwrap_or_default().to_string(),
    ));

    // 2) 标准更新：payload.validate() 先于一切
    let (status, v) = send_json(app, "PUT", "/customers/1", &body_type).await;
    out.push((
        "PUT /customers/{{id}}".to_string(),
        status,
        v["code"].as_str().unwrap_or_default().to_string(),
        v["message"].as_str().unwrap_or_default().to_string(),
    ));

    // 3) 增强 PUT（波0 前无校验旁路）
    let (status, v) = send_json(app, "PUT", "/crm/customers/enhanced/1", &body_type).await;
    out.push((
        "PUT /crm/customers/enhanced/{{id}}".to_string(),
        status,
        v["code"].as_str().unwrap_or_default().to_string(),
        v["message"].as_str().unwrap_or_default().to_string(),
    ));

    // 4) 线索转化（波0 前无校验旁路；非法值必须拒，缺省 POTENTIAL 分支见⑤登记）
    let (status, v) = send_json(app, "POST", "/crm/leads/1/convert", &body_type).await;
    out.push((
        "POST /crm/leads/{{id}}/convert".to_string(),
        status,
        v["code"].as_str().unwrap_or_default().to_string(),
        v["message"].as_str().unwrap_or_default().to_string(),
    ));

    // 5) pool 规则入口：另一张表的"规则作用域"，允许集 all/wholesale/retail/vip 独立，
    //    两个非法值同样不在其集内 ⇒ 只对齐族别/状态码，message 不做跨表一致声明。
    let pool_body = format!(
        r#"{{"name":"契约锁规则","rule_type":"claim_limit","rule_value":3,"customer_type":"{invalid}","notes":null}}"#
    );
    let (status, v) = send_json(app, "POST", "/crm/pool/rules", &pool_body).await;
    out.push((
        "POST /crm/pool/rules".to_string(),
        status,
        v["code"].as_str().unwrap_or_default().to_string(),
        v["message"].as_str().unwrap_or_default().to_string(),
    ));

    out
}

#[tokio::test]
async fn five_write_entries_reject_uppercase_retail_with_validation_envelope() {
    let app = build_app();
    let results = all_entries_reject(&app, "RETAIL").await;
    for (entry, status, code, message) in &results {
        assert_eq!(
            *status,
            StatusCode::BAD_REQUEST,
            "入口 {entry} 对 RETAIL 必须 400（不得 500/透传）"
        );
        assert_eq!(
            code.as_str(),
            "VALIDATION_ERROR",
            "入口 {entry} 拒绝族别必须为 VALIDATION_ERROR（字段校验=VALIDATION）"
        );
        // pool 入口 message 是其自身词表文案，customers 族四入口逐字符对齐
        if !entry.contains("pool") {
            assert_eq!(
                message.as_str(),
                CUSTOMERS_TYPE_MSG,
                "入口 {entry} 的 message 必须与 validator 通道逐字符一致"
            );
        }
    }
    assert_eq!(results.len(), 5);
}

#[tokio::test]
async fn five_write_entries_reject_normal_with_validation_envelope() {
    let app = build_app();
    let results = all_entries_reject(&app, "normal").await;
    for (entry, status, code, message) in &results {
        assert_eq!(
            *status,
            StatusCode::BAD_REQUEST,
            "入口 {entry} 对 normal 必须 400"
        );
        assert_eq!(
            code.as_str(),
            "VALIDATION_ERROR",
            "入口 {entry} 拒绝族别必须为 VALIDATION_ERROR"
        );
        if !entry.contains("pool") {
            assert_eq!(
                message.as_str(),
                CUSTOMERS_TYPE_MSG,
                "入口 {entry} message 漂移"
            );
        }
    }
}

// =========================================================
// ② 无校验旁路已封（增强 PUT / ConvertLead 各一条非法值被拒正例）
// =========================================================

/// 增强 PUT：波0 前该字段裸 Deserialize 直落 CustomerService::update_customer。
/// 若在 crm_customer_handler::update_customer 去掉模块校验，本请求会在
/// AppState::default() 的 Disconnected 哨兵上触库（panic）而非 400 —— 本测必红。
#[tokio::test]
async fn enhanced_put_illegal_customer_type_is_rejected_not_passthrough() {
    let app = build_app();
    let (status, v) = send_json(
        &app,
        "PUT",
        "/crm/customers/enhanced/1",
        r#"{"customer_name":"改名","customer_type":"RETAIL"}"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(v["code"], "VALIDATION_ERROR");
    assert_eq!(v["message"], CUSTOMERS_TYPE_MSG);
}

/// ConvertLeadRequest：波0 前连 Validate derive 都没有、handler 直传 service。
/// 若去掉 crm_handler::convert_lead 顶部的模块校验，本请求会带着非法值触库 ⇒ 必红。
#[tokio::test]
async fn convert_lead_illegal_customer_type_is_rejected_not_passthrough() {
    let app = build_app();
    let (status, v) = send_json(
        &app,
        "POST",
        "/crm/leads/1/convert",
        r#"{"customer_type":"enterprise","notes":"契约锁"}"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(v["code"], "VALIDATION_ERROR");
    assert_eq!(v["message"], CUSTOMERS_TYPE_MSG);
}

// =========================================================
// ③ 导入写入值 ∈ ALLOWED（钉第 2 项缺陷修：写回 "RETAIL" 即红）
// =========================================================

#[test]
fn import_write_value_is_allowed_lowercase_token() {
    // 集合成员锁：导入缺省写值就是唯一词表的真实 token
    assert!(
        customer_type::ALLOWED.contains(&customer_type::RETAIL),
        "导入写入缺省值必须是 ALLOWED 成员（当前：{}）",
        customer_type::RETAIL
    );
    assert_eq!(
        customer_type::RETAIL,
        "retail",
        "导入缺省 token 必须小写（读侧小写精确匹配）"
    );
    assert_ne!(
        customer_type::RETAIL.to_uppercase().as_str(),
        customer_type::RETAIL,
        "防御：RETAIL≠retail 的大小写差异是本缺陷的根因，两端不可混"
    );

    // 源码扫描锁：写入点必须引用模块常量，不得回潮大写裸字面量
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/services/import_export_ops/import.rs");
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {path:?} 失败: {e}"));
    assert!(
        src.contains(r#"customer_type: Set(crate::constants::customer_type::RETAIL.to_string())"#),
        "import.rs 客户导入必须写 constants::customer_type::RETAIL（值只允许在唯一模块出现一次）"
    );
    assert!(
        !src.contains(r#"customer_type: Set("RETAIL".to_string())"#),
        "import.rs 回潮大写裸字面量 Set(\"RETAIL\")：读侧小写精确匹配 ⇒ 导入客户类型筛选永不命中（波0 前缺陷）"
    );
}

// =========================================================
// ④ 唯一模块与标准入口行为等价（同一输入表逐值比对，现树函数，不手抄期望）
// =========================================================

/// 标准入口通道（CreateCustomerRequest::validate → From<ValidationErrors>）
/// 与模块 `customer_type::validate` 对同一输入必须给出同构结论。
#[tokio::test]
async fn module_validate_is_equivalent_to_standard_validator_channel() {
    let inputs: Vec<&str> = vec![
        // 现行白名单五值（逐字符来自模块自身，不在测试里抄第二套集合）
        customer_type::ALLOWED[0],
        customer_type::ALLOWED[1],
        customer_type::ALLOWED[2],
        customer_type::ALLOWED[3],
        customer_type::ALLOWED[4],
        // 拒绝族样本
        "RETAIL",
        "normal",
        "vip",
        "enterprise",
        "POTENTIAL",
        "",
        " retail",
        "Retail",
    ];

    for input in inputs {
        // 旧通道：标准入口 validator（customer_handler 的 custom function）
        let req: customer_handler::CreateCustomerRequest =
            serde_json::from_value(serde_json::json!({
                "customer_name": "等价表锁客户",
                "customer_type": input,
            }))
            .unwrap_or_else(|e| panic!("输入 {input:?} 反序列化失败: {e}"));
        let old_result = req.validate();

        // 新通道：唯一模块
        let new_result = customer_type::validate(Some(input));

        match (old_result, &new_result) {
            (Ok(()), Ok(norm)) => {
                assert_eq!(
                    norm.as_str(),
                    input,
                    "输入 {input:?} 被接受时必须原文返回（不 trim、不归一大小写）"
                );
            }
            (Err(old_errs), Err(new_err)) => {
                // 两条通道产生的 AppError 信封逐字段一致（族别+文案同一构造函数）
                let (old_status, old_env) = envelope_of(AppError::from(old_errs.clone())).await;
                let (new_status, new_env) = envelope_of(new_err.clone()).await;
                assert_eq!(old_status, new_status, "输入 {input:?} 拒绝状态码不一致");
                assert_eq!(old_status, StatusCode::BAD_REQUEST);
                assert_eq!(
                    old_env["code"], new_env["code"],
                    "输入 {input:?} 拒绝 code 不一致"
                );
                assert_eq!(
                    new_env["code"], "VALIDATION_ERROR",
                    "输入 {input:?} 族别漂移"
                );
                assert_eq!(
                    old_env["message"], new_env["message"],
                    "输入 {input:?} 拒绝文案不一致"
                );
            }
            (Ok(()), Err(_)) => {
                panic!("输入 {input:?}：模块拒绝而 validator 通道接受（旁路语义漂移）")
            }
            (Err(_), Ok(_)) => panic!("输入 {input:?}：模块接受而 validator 通道拒绝（词表漂移）"),
        }
    }
}

#[tokio::test]
async fn module_none_default_equals_create_entry_default_and_is_allowed() {
    // 缺省语义锁：validate(None) == 标准 create 入口现行缺省（"retail"），且该缺省 ∈ ALLOWED。
    let default_value = customer_type::validate(None)
        .expect("None 缺省分支不得报错（现行行为：创建入口缺省 retail）");
    assert!(
        customer_type::ALLOWED.contains(&default_value.as_str()),
        "创建入口缺省值必须是 ALLOWED 成员，当前：{default_value}"
    );
    // validator 通道对 None 不触发 custom 函数（Option 跳过）：
    let req: customer_handler::CreateCustomerRequest =
        serde_json::from_value(serde_json::json!({"customer_name": "缺省锁客户"})).unwrap();
    req.validate()
        .expect("customer_type 缺省（None）必须通过 validator 通道");
}

#[test]
fn vocabulary_not_rehardcoded_in_standard_handler() {
    // 值集合只允许在唯一模块出现一次：标准 handler 不得再抄内联白名单
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/handlers/customer_handler.rs");
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {path:?} 失败: {e}"));
    assert!(
        !src.contains("let valid_types = ["),
        "customer_handler.rs 回潮内联白名单 valid_types：值集合必须只在 constants/customer_type.rs 出现一次"
    );
    assert!(
        src.contains("crate::constants::customer_type::check(customer_type)"),
        "customer_handler.rs 校验器必须委托唯一模块 check"
    );
}

// =========================================================
// ⑤ 登记性负例：转化缺省 "POTENTIAL" 是已登记的大写脏值来源（本波不改，待裁）
// =========================================================

#[test]
fn lead_conversion_potential_default_is_registered_dirty_value_not_in_allowed() {
    // 朴素断言：经 services/crm/lead.rs:691-694（缺省分支）落库的值 "POTENTIAL"
    // 不等于 ALLOWED 集合内任何值——精确比较 + 忽略大小写比较都不等（大小写/词表双重不等）。
    let entry_default = "POTENTIAL";
    assert!(
        customer_type::ALLOWED.iter().all(|t| *t != entry_default),
        "POTENTIAL 不得出现在 ALLOWED（本波未裁不许并入；为绿加值即违红线）"
    );
    assert!(
        customer_type::ALLOWED
            .iter()
            .all(|t| t.to_lowercase() != entry_default.to_lowercase()),
        "POTENTIAL 连大小写变体都不属于 ALLOWED ⇒ 经该入口落库的行必然落在词表之外"
    );
    // 且模块对该值判拒（混维撞名细节见文件头注释：potential 已被 CLV 分层占用）
    assert!(
        customer_type::check(entry_default).is_err(),
        "模块必须拒 POTENTIAL（缺省分支只在 service 层承接，写口显式传值一律走校验）"
    );

    // 登记锁：转化入口缺省**当前仍是** "POTENTIAL"（本波不改值）。
    // 若后续波改了该缺省而未同步本登记，本扫描即红，强制两处一起收口。
    let path =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/services/crm/lead.rs");
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {path:?} 失败: {e}"));
    assert!(
        src.contains(r#".unwrap_or_else(|| "POTENTIAL".to_string())"#),
        "lead.rs 转化缺省已非 \"POTENTIAL\"：裁定波须同步撤换本登记锁与 lead.rs 注释（勿只改一边）"
    );
}
