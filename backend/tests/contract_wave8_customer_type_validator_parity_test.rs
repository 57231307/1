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
//! ⑤ 缺省口径锁：本列语义=渠道，**缺省一律 `other`**（未知渠道不猜零售）——
//!    标准创建入口（`validate(None)`）与线索转化 service 缺省分支同一口径；
//!    `"POTENTIAL"` / `vip` / `normal` 一律**不属** ALLOWED（分层词不入渠道列，
//!    `potential` 已被 CLV 分层 segment 占用：services/crm/cust.rs:670-680
//!    segment=champion/loyal/potential/at_risk/lost；models/customer_lifetime_value.rs:40）。
//! ⑥ 判据锁：大客户转移审批的**唯一判据**是分层列 `customers.tier` 逐字符落在高档集合
//!    `constants::customer_tier::MAJOR`（经 `customer_tier::is_major` 判定，NULL=未定档
//!    判非大客户）。渠道列 `customer_type`（含旧 `== "vip"` 分支）与信用额度/预估金额
//!    等一切金额维度都不得参与判定；`MAJOR` 集合定义、`is_major` 谓词与分层词表的成员
//!    枚举在全 `src` 下只允许出现在 `constants/customer_tier.rs` 一处，第二处定义或
//!    内联白名单即红。
//!
//! 运行前提：①② 仅依赖"validator/校验先于触库"，无 DB；③④⑤⑥ 纯静态/纯函数，无 DB。

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::{IntoResponse, Response},
    routing::{post, put},
};
use bingxi_backend::constants::customer_tier;
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
async fn module_none_default_equals_create_entry_default_and_is_other() {
    // 缺省口径锁：validate(None) 必须 = `OTHER`（"other"）且 ∈ ALLOWED。
    // 缺省表达"渠道未知"，不是"猜零售"——写回 RETAIL 即业务口径回潮，本测必红。
    let default_value = customer_type::validate(None)
        .expect("None 缺省分支不得报错（现行行为：创建入口缺省 other）");
    assert_eq!(
        default_value,
        customer_type::OTHER,
        "创建入口缺省必须等于 constants::customer_type::OTHER"
    );
    assert_eq!(
        default_value, "other",
        "缺省 token 字面值必须是小写 other（与 DB 列 DEFAULT 'other' 同源）"
    );
    assert!(
        customer_type::ALLOWED.contains(&default_value.as_str()),
        "创建入口缺省值必须是 ALLOWED 成员，当前：{default_value}"
    );
    assert_ne!(
        default_value,
        customer_type::RETAIL,
        "缺省不得等于 retail：未知渠道不猜零售"
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
// ⑤ 缺省口径锁：线索转化缺省 = other；分层词 POTENTIAL/vip/normal 永不入渠道列
// =========================================================

#[test]
fn lead_conversion_default_is_other_and_tier_tokens_never_enter_channel_column() {
    // 分层词不是渠道值：POTENTIAL / potential / vip / normal 大小写变体都不许出现在 ALLOWED
    // （为绿把它们并进 ALLOWED 即混维回潮，违红线）。
    for tier_token in ["POTENTIAL", "potential", "vip", "VIP", "normal"] {
        assert!(
            customer_type::ALLOWED
                .iter()
                .all(|t| *t != tier_token && t.to_lowercase() != tier_token.to_lowercase()),
            "{tier_token} 属分层词，不得出现在渠道词表 ALLOWED 内"
        );
        assert!(
            customer_type::check(tier_token).is_err(),
            "模块必须拒 {tier_token}"
        );
    }

    // 转化入口缺省已收口到唯一词表的 OTHER：
    // 若改回裸字面量 "POTENTIAL"（或缺省猜 retail），下面两条扫描锁即红。
    let path =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/services/crm/lead.rs");
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {path:?} 失败: {e}"));
    assert!(
        src.contains(r#".unwrap_or_else(|| crate::constants::customer_type::OTHER.to_string())"#),
        "lead.rs 转化缺省必须是 constants::customer_type::OTHER（值只允许在唯一模块出现一次）"
    );
    assert!(
        !src.contains("POTENTIAL"),
        "lead.rs 回潮大写脏值 \"POTENTIAL\"：渠道列写分层词 ⇒ 读侧小写精确匹配永不命中（混维撞名）"
    );
    assert!(
        !src.contains(r#"customer_type::RETAIL.to_string())"#),
        "lead.rs 转化缺省不得改成 RETAIL：未知渠道不猜零售"
    );

    // "来源=线索" 由 customers.source 承载，不许新造 customer_type token 表达来源。
    assert!(
        src.contains(r#"source: Set(Some("lead".to_string()))"#),
        "lead.rs 必须继续写 source='lead'（转化来源语义的既有载体）"
    );
}

// =========================================================
// ⑥ 判据锁：大客户转移审批唯一判据 = customers.tier ∈ MAJOR（分层列，非金额、非渠道）
// =========================================================

/// 递归收集目录下的全部 `.rs` 文件（供唯一性形状扫描，只读不改）。
fn collect_rs_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("读取目录 {dir:?} 失败: {e}"))
    {
        let path = entry
            .unwrap_or_else(|e| panic!("目录项读取失败（{dir:?}）: {e}"))
            .path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// 返回 `src` 下源码包含 `marker` 的全部文件路径（统一 `/` 分隔并按路径排序，
/// 使断言失败消息跨平台可读、命中集可复现）。
fn src_files_containing(marker: &str) -> Vec<String> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    let mut rs = Vec::new();
    collect_rs_files(&root, &mut rs);
    for path in rs {
        let text =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {path:?} 失败: {e}"));
        if text.contains(marker) {
            files.push(path.to_string_lossy().replace('\\', "/"));
        }
    }
    files.sort();
    files
}

#[test]
fn transfer_approval_large_customer_trigger_is_tier_major_only() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/services/crm/customer_transfer_approval_service.rs");
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {path:?} 失败: {e}"));

    // 正判据形状：唯一入口 = 分层列读取 + 高档集合谓词 + 线索客户外键载体，三者缺一即判据被换掉
    assert!(
        src.contains("customer_tier::is_major"),
        "二级审批判据必须唯一经 constants::customer_tier::is_major（高档集合唯一真源）"
    );
    assert!(
        src.contains("c.tier.as_deref()"),
        "check_large_customer 必须读 customers.tier 分层列（NULL=未定档由 Option 通道归为非大客户）"
    );
    assert!(
        src.contains("converted_customer_id"),
        "判据载体必须是线索关联客户 converted_customer_id（未挂客户的线索判非大客户）"
    );

    // 负形状锁：金额判据的任何形态不得残留在审批服务——旧信用额度阈值口径已被终裁废除
    for banned in [
        "500_000",
        "credit_limit",
        "DEFAULT_LARGE_CUSTOMER_CREDIT_THRESHOLD",
        "estimated_amount",
        "threshold",
        "Decimal",
    ] {
        assert!(
            !src.contains(banned),
            "审批服务源码残留金额判据痕迹 {banned:?}——判据只许读 customers.tier ∈ MAJOR，\
             以信用额度/预估金额代理判大客户属第二套口径，已被终裁废除"
        );
    }

    // 被清除的伪判据：customer_type == "vip" 受渠道列校验永不可达（列上不可能有 vip），
    // 留着它等于把业务规则建在自己的漏洞上；不许以别名或死代码形式回潮。
    assert!(
        !src.contains(".customer_type"),
        "审批服务不得再读取 customers.customer_type 作判据（vip 分支须删干净，不留别名/死代码）"
    );
    assert!(
        !src.contains("customer_type =="),
        "审批服务不许回潮 customer_type 比较表达式（分层词不属渠道列，渠道列也不属分层判据）"
    );
    assert!(
        !src.contains("\"vip\""),
        "审批服务内不许再出现 \"vip\" 字符串字面量（判据只认分层列高档集合成员）"
    );
    // 高档集合不许在判据文件内联抄成员——只许引用唯一模块
    assert!(
        !src.contains("\"VIP\"") && !src.contains("\"GOLD\""),
        "审批服务不许内联高档 token 字面量（\"VIP\"/\"GOLD\"）——成员清单只允许定义在 \
         constants/customer_tier.rs 一处，判据处内联即第二套口径回潮"
    );

    // 唯一模块锁：MAJOR 定义、is_major 谓词、高档/分层成员枚举在全 src 下各只命中唯一模块文件
    let tier_module = format!(
        "{}/src/constants/customer_tier.rs",
        env!("CARGO_MANIFEST_DIR")
    )
    .replace('\\', "/");
    for marker in [
        "const MAJOR:",
        "fn is_major(",
        "&[VIP, GOLD]",
        "&[VIP, GOLD, SILVER, NORMAL]",
    ] {
        let hits = src_files_containing(marker);
        assert_eq!(
            hits,
            vec![tier_module.clone()],
            "标记 {marker:?} 只允许出现在 constants/customer_tier.rs（分层词表与高档集合的\
             唯一真源）；实际命中：{hits:?}——第二处定义/内联白名单即判据口径分裂"
        );
    }
    // 带引号成员对（内联白名单的典型回潮形态）：唯一模块之外全 src 零命中
    let quoted_hits = src_files_containing("\"VIP\", \"GOLD\"");
    assert!(
        quoted_hits.iter().all(|h| *h == tier_module),
        "全 src 中出现带引号高档成员对 \"VIP\", \"GOLD\" 的文件只许是 constants/customer_tier.rs；\
         实际命中：{quoted_hits:?}"
    );

    // 语义分离锁：分层词表与渠道词表互斥——判据只读 tier，两维混装即口径污染
    for tier_token in customer_tier::ALLOWED {
        assert!(
            !customer_type::ALLOWED.contains(tier_token),
            "分层 token '{tier_token}' 混进了渠道词表 customer_type::ALLOWED（两列语义必须严格分离）"
        );
    }
    for channel_token in customer_type::ALLOWED {
        assert!(
            !customer_tier::ALLOWED.contains(channel_token),
            "渠道 token '{channel_token}' 混进了分层词表 customer_tier::ALLOWED（混维即回潮）"
        );
        assert!(
            !customer_tier::MAJOR.contains(channel_token),
            "渠道 token '{channel_token}' 混进了高档集合 MAJOR（判据集合被渠道词污染）"
        );
    }
}
