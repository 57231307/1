//! 合同模板读写腿的权限键派生与词表形态静态锁（CI 常规可跑，不依赖活库）
//!
//! 功能 钉死模板读写腿的授权面与词表接线形态。一钉 模板族路由的权限键派生——每个端点
//!   经权限中间件的纯函数派生出的 resource 与 action 必须逐字符落在
//!   `contract-templates` 加 read/create/update/delete/approve 五个动作之内，且资源名已在
//!   `PERMISSION_RESOURCES` 与 `path_utils` 直接资源表两处同词登记（任一处缺词即派生死键、
//!   全员 403）。二钉 精确匹配无粗域展开——`matches_permission` 对 `contract-templates`
//!   的授权不得命中同前缀的其它资源，也不得被 `contracts` 这类近似词命中；记录级授权不得
//!   放行整集合读。三钉 服务层状态机词表同源——服务源码对模板生命周期的比较点与写入值
//!   只允许取 `contract_template_status` 常量，不出现裸的 draft active retired 字符串字面量；
//!   全文件把状态写为 ACTIVE 的语句恰好一处（只写目标行，不给自动降位他行留通道）。
//! 调用方 CI 的 Rust 测试 job 与本仓本地门。入参 无，读服务与路由与 handler 与注册表源文本，
//!   并直接调用权限中间件的纯函数做派生断言。
//! 传给谁 无下游，纯断言。存什么 不写库、不写文件。存哪里 不落地。
//!
//! 判据是逐字符相等与出现计数，不是包含
//! - 路由新增端点未同步本锁 派生键清单比对必差集点名；
//! - 有人把 activate 做成第二条写 ACTIVE 的路径（如自动降位旧版） 计数断言判红；
//! - 有人拿 `contracts` 或 `contract*` 近似词授予 精确匹配用例当场判红。

use axum::http::Method;
use bingxi_backend::middleware::permission::{
    extract_action_from_path, extract_resource_info, matches_permission, method_to_action,
};
use bingxi_backend::models::role_permission;
use bingxi_backend::services::init_service::PERMISSION_RESOURCES;
use bingxi_backend::utils::path_utils::is_known_resource_segment;
use std::fs;
use std::path::PathBuf;

fn read(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("读取 {rel} 失败: {e}（路径或改名需同步本锁）"))
}

/// 去掉整行注释后的源码。SQL 与 Rust 字面量里不存在行首 //，注释行一律不参与判定。
fn strip_line_comments(text: &str) -> String {
    text.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 模板族端点的派生键权威清单：(路径样例, HTTP 方法, 期望 resource, 期望 action)。
/// 路径样例带一个记录 ID，与运行时真实形态一致。新增或改名端点必须同步本表，
/// 漏同步时下一条用例的路由文件计数比对与派生比对一起判红。
const EXPECTED_ENDPOINTS: &[(&str, Method, &str, &str)] = &[
    (
        "/api/v1/erp/contract-templates",
        Method::GET,
        "contract-templates",
        "read",
    ),
    (
        "/api/v1/erp/contract-templates",
        Method::POST,
        "contract-templates",
        "create",
    ),
    (
        "/api/v1/erp/contract-templates/7",
        Method::GET,
        "contract-templates",
        "read",
    ),
    (
        "/api/v1/erp/contract-templates/7",
        Method::PUT,
        "contract-templates",
        "update",
    ),
    (
        "/api/v1/erp/contract-templates/7",
        Method::DELETE,
        "contract-templates",
        "delete",
    ),
    (
        "/api/v1/erp/contract-templates/7/revise",
        Method::POST,
        "contract-templates",
        "create",
    ),
    (
        "/api/v1/erp/contract-templates/7/approve",
        Method::POST,
        "contract-templates",
        "approve",
    ),
    (
        "/api/v1/erp/contract-templates/7/activate",
        Method::PUT,
        "contract-templates",
        "update",
    ),
    (
        "/api/v1/erp/contract-templates/7/retire",
        Method::PUT,
        "contract-templates",
        "update",
    ),
    (
        "/api/v1/erp/contract-templates/7/clauses",
        Method::GET,
        "contract-templates",
        "read",
    ),
    (
        "/api/v1/erp/contract-templates/7/clauses",
        Method::POST,
        "contract-templates",
        "create",
    ),
    (
        "/api/v1/erp/contract-templates/7/clauses/11",
        Method::PUT,
        "contract-templates",
        "update",
    ),
    (
        "/api/v1/erp/contract-templates/7/clauses/11",
        Method::DELETE,
        "contract-templates",
        "delete",
    ),
    (
        "/api/v1/erp/contract-templates/7/clauses/11/review",
        Method::PUT,
        "contract-templates",
        "update",
    ),
];

/// 派生键逐端点比对 每个模板族端点经权限中间件纯函数派生出的 resource 与 action
/// 必须与权威清单逐字符一致。派生走 `extract_resource_info` 加
/// （路径动作关键字优先，方法派生兜底）的真实组合，不另抄一份中间件逻辑。
#[test]
fn contract_template_routes_derive_exact_permission_keys() {
    for (path, method, want_resource, want_action) in EXPECTED_ENDPOINTS {
        assert!(
            is_known_resource_segment("contract-templates"),
            "路由 {path} 的 seg3 未进 path_utils 白名单，权限中间件会对全员 fail-closed 403"
        );
        let (resource, _resource_id) = extract_resource_info(path);
        assert_eq!(
            &resource, want_resource,
            "路由 {path} 派生的资源名应为 {want_resource}，实得 {resource}（模块前缀表或资源映射被改动会漂名）"
        );
        let action = extract_action_from_path(path).unwrap_or_else(|| method_to_action(method));
        assert_eq!(
            &action, want_action,
            "路由 {path} 的 {method} 派生动作应为 {want_action}，实得 {action}"
        );
    }
}

/// 注册表两处同词 PERMISSION_RESOURCES 与直接资源表都含 contract-templates，
/// 缺任一处即派生死键（授予了也永不命中）或路由级 403。
#[test]
fn contract_templates_registered_in_both_authoritative_lists() {
    assert!(
        PERMISSION_RESOURCES.contains(&"contract-templates"),
        "PERMISSION_RESOURCES 缺 contract-templates 登记，权限目录与鉴权键漂移"
    );
    assert!(
        is_known_resource_segment("contract-templates"),
        "path_utils 已知资源段表缺 contract-templates，白名单校验先于 admin 短路、对 admin 也 403"
    );
}

/// 路由文件端点数与本锁权威清单一致（按方法展开计数）。漏登记端点=授权面失真，
/// 多登记端点=本锁失去检测对象，两头都判红。
#[test]
fn route_file_endpoint_count_matches_lock() {
    let src = strip_line_comments(&read("src/routes/contract_template.rs"));
    let route_defs = src.matches(".route(").count();
    let expanded: usize = EXPECTED_ENDPOINTS.len();
    // 路由文件里一条 .route 可链多个方法；本锁按方法展开计数，链式注册数只作下限守卫。
    assert!(
        route_defs > 0 && route_defs <= expanded,
        "路由文件 .route 定义数 {route_defs} 与本锁展开数 {expanded} 关系失守（端点增删未同步本锁）"
    );
    let registered_methods = src.matches("get(").count()
        + src.matches("post(").count()
        + src.matches("put(").count()
        + src.matches("delete(").count();
    assert_eq!(
        registered_methods, expanded,
        "路由文件按方法登记的端点数 {registered_methods} 与本锁权威清单 {expanded} 不一致，端点增删必须同步本锁"
    );
}

/// matches_permission 精确匹配语义在模板资源上的三个方向钉死：
/// 近似词不命中、通配授权命中、记录级授权不放行整集合读。
#[test]
fn matches_permission_is_exact_for_contract_templates() {
    let grant = |resource_type: &str, action: &str, resource_id: Option<i32>| {
        let now = chrono::Utc::now();
        role_permission::Model {
            id: 1,
            role_id: 1,
            resource_type: resource_type.to_string(),
            resource_id,
            action: action.to_string(),
            allowed: true,
            permission_code: None,
            created_at: now,
            updated_at: now,
        }
    };
    // 正向 同词授权精确命中（角色级覆盖列表与按 id 操作）
    assert!(
        matches_permission(
            &grant("contract-templates", "update", None),
            "contract-templates",
            None,
            "update"
        ),
        "contract-templates:update 的角色级授权必须放行整集合写"
    );
    assert!(
        matches_permission(
            &grant("contract-templates", "approve", None),
            "contract-templates",
            Some(7),
            "approve"
        ),
        "角色级授权必须放行按 id 的 approve 动作"
    );
    // 反向 近似资源词不串域
    for near in [
        "contract",
        "contracts",
        "contract-template",
        "contract_templates",
    ] {
        assert!(
            !matches_permission(
                &grant(near, "update", None),
                "contract-templates",
                None,
                "update"
            ),
            "近似资源词 {near} 的授权不得命中 contract-templates（后端无粗域展开，串域即扩权）"
        );
    }
    // 反向 动作不串
    assert!(
        !matches_permission(
            &grant("contract-templates", "update", None),
            "contract-templates",
            None,
            "delete"
        ),
        "update 授权不得放行 delete 动作"
    );
    // 反向 记录级授权不得放行整集合读
    assert!(
        !matches_permission(
            &grant("contract-templates", "read", Some(7)),
            "contract-templates",
            None,
            "read"
        ),
        "记录级授权（resource_id=Some）不得放行整集合操作"
    );
    // 通配超级授权命中（admin 之外的通配形态口径不变）
    assert!(
        matches_permission(
            &grant("*", "*", None),
            "contract-templates",
            Some(7),
            "activate"
        ),
        "超级通配授权必须命中新资源（通配豁免口径不变）"
    );
}

/// 服务层词表同源钉 模板生命周期的比较点与写入值只允许取权威常量。
/// 判一 服务源码（去整行注释）里不得出现裸的 draft active retired 字符串字面量；
/// 判二 写 ACTIVE 的 Set 语句全文件恰好一处——写 ACTIVE 的第二条通道就是
/// 自动降位或旁路启用的落点，activate 只许写目标行自身。
#[test]
fn service_status_writes_are_vocab_sourced_single_channel() {
    let src = strip_line_comments(&read("src/services/contract_template_service.rs"));
    for token in ["\"draft\"", "\"active\"", "\"retired\""] {
        assert!(
            !src.contains(token),
            "服务源码出现状态裸字面量 {token}，词表取值必须来自 contract_template_status 常量"
        );
    }
    let active_sets = src
        .matches("Set(contract_template_status::ACTIVE.to_string())")
        .count();
    assert_eq!(
        active_sets, 1,
        "服务层写 ACTIVE 状态的语句应恰好一处（activate 的目标行），实得 {active_sets} 处——出现第二处即存在自动降位或旁路启用通道"
    );
    assert!(
        src.contains("contract_template_status::DRAFT")
            && src.contains("contract_template_status::RETIRED"),
        "服务层必须消费权威词表的 DRAFT 与 RETIRED 常量，比较点缺词即状态机漂走"
    );
}

/// handler 层不得自带 router（check-route-mount 约定 R2 的合同模板族投影），
/// 且每个期望端点的 handler 符号真实存在，防止派生键表与实际接线脱钩。
#[test]
fn handler_symbols_exist_and_no_embedded_router() {
    let handler_src = read("src/handlers/contract_template_handler.rs");
    assert!(
        !handler_src.contains("Router::new()"),
        "模板 handler 文件不得自带 router，注册只写在 routes/ 下"
    );
    let routes_src = strip_line_comments(&read("src/routes/contract_template.rs"));
    for symbol in [
        "list_templates",
        "get_template",
        "create_template",
        "update_template",
        "delete_template",
        "revise_template",
        "approve_template",
        "activate_template",
        "retire_template",
        "list_clauses",
        "add_clause",
        "update_clause",
        "delete_clause",
        "review_clause",
    ] {
        assert!(
            routes_src.contains(symbol),
            "路由文件缺少 handler 符号 {symbol} 的登记"
        );
        assert!(
            handler_src.contains(&format!("pub async fn {symbol}")),
            "handler 文件缺少 pub async fn {symbol}（路由登记与实现漂移）"
        );
    }
}
