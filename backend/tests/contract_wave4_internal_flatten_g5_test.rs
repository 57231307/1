//! wave4 G5（i500_5 清单 15 文件）：internal 重包拍平 4xx→500 的去除锁 + sku 映射删除引用预检锁
//!
//! 背景契约（修复前的头号缺陷形态）：handler/service 把**已返回 AppError 的调用**用
//! `AppError::internal(format!("{e}"))` 重包，400 业务拒绝 / 403 越权 / 404 不存在一律
//! 拍平成 500 INTERNAL_ERROR/"服务器内部错误" 并丢真实 code 与文案。
//!
//! 锁分层：
//! 1. shrink-only ratchet：每文件 `AppError::internal(` 计数 ≤ 修复前基线，且钉死在当前
//!    水位（cap）——新增 internal 站点必须先把 cap 调低（即先消灭别的站点）才能过。
//!    本组 15 文件全部 `include_str!` 编译期载入，源文件缺失本身即编译失败。
//! 2. AppError-returning 调用零 `map_err(internal)`：对被修复的 service 调用名做窗口扫描，
//!    调用点后 10 行内不得再出现 `map_err` 与 `AppError::internal` 组合（回潮即红）。
//! 3. sku_mapping_service 删除路径：必须存在采购/调拨单据引用预检（purchase_order_items
//!    转采购快照列匹配），被引用走 `business_displayable` 公开规则文案（不含约束名/23503），
//!    delete 函数体内禁止 `AppError::internal`；真正 DbErr 经 `?`（From<DbErr>→DATABASE_ERROR）。
//! 4. 真实断言（路线一：真库 PostgreSQL / 纯校验前置）：
//!    - 辅助核算余额查询非法期间：service 的 `validation_displayable("月份必须在1-12之间")`
//!      经 handler 原样传播 → 400 + 真实原因外显（修复前是 500"服务器内部错误"）。
//!    - 业务追溯不存在五维 ID（已迁移真库的空 business_trace_chain，夹具 TRUNCATE，
//!      不再自建 sqlite 同构表）→ 真实调用 handler → 404 NOT_FOUND（修复前 service 侧
//!      任何 4xx 都被拍平成 500），且出参 message 不得是"服务器内部错误"；
//!      NotFound 出参按 `utils/error.rs` 白名单口径脱敏为固定常量"资源未找到"
//!      （真实原因进 tracing，语义由 status/code 承载）。

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    routing::get,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::{assist_accounting_handler, business_trace_handler};
use serde_json::Value;
use tower::ServiceExt;

mod test_common;
use test_common::setup_test_db;

// =========================================================
// 0) 本组 15 文件编译期载入 + ratchet 数据（基线=修复前处数，cap=修复后水位）
// =========================================================

/// (相对 backend 的文件路径, 修复前基线, 修复后 cap, 源码)
const GROUP: &[(&str, usize, usize, &str)] = &[
    (
        "src/utils/xlsx_export.rs",
        13,
        13,
        include_str!("../src/utils/xlsx_export.rs"),
    ),
    (
        "src/services/totp_service.rs",
        12,
        12,
        include_str!("../src/services/totp_service.rs"),
    ),
    (
        "src/handlers/user_handler.rs",
        9,
        2,
        include_str!("../src/handlers/user_handler.rs"),
    ),
    (
        "src/services/report/ds.rs",
        6,
        0,
        include_str!("../src/services/report/ds.rs"),
    ),
    (
        "src/handlers/assist_accounting_handler.rs",
        6,
        0,
        include_str!("../src/handlers/assist_accounting_handler.rs"),
    ),
    (
        "src/handlers/ai_analysis_handler.rs",
        4,
        0,
        include_str!("../src/handlers/ai_analysis_handler.rs"),
    ),
    (
        "src/handlers/business_trace_handler.rs",
        4,
        0,
        include_str!("../src/handlers/business_trace_handler.rs"),
    ),
    (
        "src/routes/search_api.rs",
        3,
        3,
        include_str!("../src/routes/search_api.rs"),
    ),
    (
        "src/handlers/dashboard_handler.rs",
        2,
        1,
        include_str!("../src/handlers/dashboard_handler.rs"),
    ),
    (
        "src/services/data_permission_service.rs",
        2,
        0,
        include_str!("../src/services/data_permission_service.rs"),
    ),
    (
        "src/handlers/sales_price_handler.rs",
        1,
        1,
        include_str!("../src/handlers/sales_price_handler.rs"),
    ),
    (
        "src/handlers/ap_payment_handler.rs",
        1,
        0,
        include_str!("../src/handlers/ap_payment_handler.rs"),
    ),
    (
        "src/handlers/purchase_inspection_handler.rs",
        1,
        0,
        include_str!("../src/handlers/purchase_inspection_handler.rs"),
    ),
    (
        "src/services/sku_mapping_service.rs",
        1,
        1,
        include_str!("../src/services/sku_mapping_service.rs"),
    ),
    (
        "src/handlers/email_handler.rs",
        1,
        1,
        include_str!("../src/handlers/email_handler.rs"),
    ),
];

fn src_of(rel: &str) -> &'static str {
    GROUP
        .iter()
        .find(|(name, ..)| *name == rel)
        .unwrap_or_else(|| panic!("清单内文件未载入: {rel}"))
        .3
}

fn count_internal(src: &str) -> usize {
    src.matches("AppError::internal(").count()
}

// =========================================================
// 1) shrink-only ratchet
// =========================================================

#[test]
fn g5_internal_ratchet_shrink_only_and_pinned() {
    assert_eq!(GROUP.len(), 15, "本组清单必须整体载入，缺一个文件都不算锁");
    for (file, baseline, cap, src) in GROUP {
        let actual = count_internal(src);
        assert!(
            actual <= *baseline,
            "{file}: AppError::internal 计数 {actual} 超过修复前基线 {baseline}（不得回潮）"
        );
        assert_eq!(
            actual, *cap,
            "{file}: AppError::internal 计数应为当前水位 {cap}，实际 {actual}。新增站点判红；\
             消灭站点后请把 cap 与基线一起下调。保留站点的分类理由见本波修复报告（b/c/e 类）。"
        );
    }
}

// =========================================================
// 2) AppError-returning 调用上 map_err(internal) 零命中（回潮锁）
// =========================================================

/// (文件, 被包裹的 AppError-returning 调用名)——修复口径：一律 `?` 直接传播原码
const APERROR_CALLERS: &[(&str, &str)] = &[
    ("src/handlers/user_handler.rs", "check_permission("),
    ("src/handlers/user_handler.rs", "load_history_from_db("),
    (
        "src/handlers/assist_accounting_handler.rs",
        "query_assist_records(",
    ),
    (
        "src/handlers/assist_accounting_handler.rs",
        "find_by_business(",
    ),
    (
        "src/handlers/assist_accounting_handler.rs",
        "find_by_five_dimension(",
    ),
    (
        "src/handlers/assist_accounting_handler.rs",
        "find_summary_by_period_and_dimension(",
    ),
    (
        "src/handlers/assist_accounting_handler.rs",
        "drill_down_to_assist(",
    ),
    (
        "src/handlers/assist_accounting_handler.rs",
        "calculate_assist_balance(",
    ),
    (
        "src/handlers/business_trace_handler.rs",
        "find_trace_chain_by_five_dimension(",
    ),
    ("src/handlers/business_trace_handler.rs", "forward_trace("),
    ("src/handlers/business_trace_handler.rs", "backward_trace("),
    ("src/handlers/business_trace_handler.rs", "create_snapshot("),
    ("src/handlers/ai_analysis_handler.rs", "forecast_sales("),
    ("src/handlers/ai_analysis_handler.rs", "optimize_inventory("),
    ("src/handlers/ai_analysis_handler.rs", "detect_anomalies("),
    (
        "src/handlers/ai_analysis_handler.rs",
        "generate_recommendations(",
    ),
];

#[test]
fn g5_no_map_err_internal_over_apperror_returning_calls() {
    for (file, callee) in APERROR_CALLERS {
        let src = src_of(file);
        let mut from = 0usize;
        let mut hits = 0usize;
        while let Some(pos) = src[from..].find(callee) {
            let abs = from + pos;
            hits += 1;
            // 从调用点起向后取 10 行窗口（覆盖链式 .await.map_err(...) 形态）
            let window: String = src[abs..].lines().take(10).collect::<Vec<_>>().join("\n");
            assert!(
                !(window.contains("map_err") && window.contains("AppError::internal")),
                "{file}: 调用 {callee} 后被 map_err 重包成 internal——AppError 原码必须用 ? 传播"
            );
            from = abs + callee.len();
        }
        assert!(hits > 0, "{file}: 找不到调用 {callee}，修复可能丢失");
    }
}

// =========================================================
// 源码扫描锁公共工具（禁回潮锁专用）
//
// 三种脆断形态的正解（#4671 判责 B1）：
// ① 禁词/必备项只看执行体 → `code_only` 先剥整行注释（`//`/`///`/`//!`）；
//    被锁源码里的说明注释（"非自己账户需要 user:delete 权限"这类对权限键的**描述**）
//    不是代码，按原文判禁词等于把文档当违例。
// ② 正向 needle 不因 rustfmt 折行/尾逗号假失败 → `canon` 双侧去空白 + 消尾逗号。
// ③ 窗的边界由符号定位，不用"符号 + 固定行数/固定字节长"（重构即漂移，且硬字节窗
//    会在中文字符中间劈开）。函数体截取复用本文件既有 `extract_fn`（char_indices 配平）。
// =========================================================

/// 剥掉整行注释后的文本（字符串字面量原样保留）。按行处理而不做字符级扫描的理由：
/// 被锁源码含跨行 raw string（SQL），单行引号配平会把代码文本误当注释吃掉；
/// 宁少剥（行尾尾注释、块注释不动）不可错剥——错剥会让"必备项"断言假失败。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 规范形（**仅用于正向 contains**）：剔全部空白并消掉闭合定界符前的尾逗号。
fn canon(src: &str) -> String {
    let mut out: String = src.chars().filter(|c| !c.is_whitespace()).collect();
    loop {
        let next = out.replace(",)", ")").replace(",]", "]").replace(",}", "}");
        if next == out {
            break;
        }
        out = next;
    }
    out
}

/// 取 `callee(...)` 的**第一个字符串字面量实参**（= 面向用户的出参文案构造点）。
/// 权限键作为 `check_permission(role_id, "user", "delete", …)` 的入参是正当形态，
/// 真正的缺陷面是"键/内部标识被拼进拒绝文案"，故扫描对象是文案而不是整文件。
fn string_args_of_call(code: &str, callee: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut from = 0usize;
    while let Some(rel) = code[from..].find(callee) {
        let at = from + rel + callee.len();
        from = at;
        let rest = code[at..].trim_start();
        let Some(rest) = rest.strip_prefix('(') else {
            continue;
        };
        let Some(q) = rest.find('"') else {
            continue;
        };
        // 字面量之前先出现 `)` ⇒ 该调用没有文案实参（跳过，不猜）
        if rest[..q].contains(')') {
            continue;
        }
        let body = &rest[q + 1..];
        if let Some(close) = body.find('"') {
            out.push(body[..close].to_string());
        }
    }
    out
}

/// 定向坐实 1：user_handler 删除权限链路
/// - `check_permission` 返回 AppError，不得被 internal 伪装成 500 且丢原因（窗口锁兜）；
/// - 缺角色拒绝保持 permission_denied（403/FORBIDDEN）语义；
/// - 拒绝出参文案不得拼接/泄露内部权限键（"user:delete" 之类的键外泄）。
#[test]
fn g5_user_handler_permission_semantics_locked() {
    let src = src_of("src/handlers/user_handler.rs");
    // 禁词只看执行体：`user_handler.rs:552`/`:577` 两处 "user:delete" **都在注释里**
    // （"非自己账户需要 user:delete 权限"是对权限键的说明），按整文件原文判会把
    // 注释当违例（#4671 判责 B1①）。真实缺陷面是**出参文案构造点**，见下面的正向锁。
    let code = code_only(src);
    assert!(
        code.contains("AppError::permission_denied(\"用户未分配角色，无法执行删除操作\")"),
        "缺角色拒绝必须保持 permission_denied(403) 语义"
    );
    assert!(
        !code.contains("user:delete") && !code.contains("\"user\", \"delete\" 权限"),
        "拒绝出参文案不得拼接/泄露内部权限键"
    );
    // 出参构造点正形锁：两个拒绝分支都必须是固定中文文案，且文案内不得含权限键段名
    // 或内部标识片段（键本身作为 check_permission 入参写在代码里是允许的）。
    let deny_texts = string_args_of_call(&code, "AppError::permission_denied");
    assert!(
        deny_texts.len() >= 2,
        "user_handler 的 permission_denied 文案构造点至少 2 处（缺角色/无权限），实得 {}",
        deny_texts.len()
    );
    assert!(
        deny_texts.iter().any(|t| t.contains("用户未分配角色")),
        "缺角色分支的拒绝文案缺失（出参无因可外显=回潮成脱敏常量或 internal）"
    );
    assert!(
        deny_texts.iter().any(|t| t.contains("没有删除用户的权限")),
        "无权限分支的拒绝文案缺失"
    );
    for text in &deny_texts {
        for banned in ["user:delete", "user", "delete", "role_id"] {
            assert!(
                !text.contains(banned),
                "拒绝文案 {text:?} 含内部权限键/标识片段 {banned:?}（出参外泄内部信息）"
            );
        }
    }

    // 函数体按符号定位（原先是"符号行起算固定 26 行"，重构即漂移）
    let body = extract_fn(&code, "async fn check_delete_user_permission(");
    assert!(
        body.contains(".check_permission(") && body.contains(".await?"),
        "check_permission 的 AppError 必须 ? 原样传播（权限系统故障带真实 code，不伪装 500）"
    );
    assert!(
        !body.contains("map_err"),
        "check_delete_user_permission 内不得再有 map_err 重包"
    );
    // 权限键只能作为**入参**传给 check_permission（正向钉住"确实在按 user/delete 判权限"，
    // 防止有人为了过禁词检查把键判定整个删掉/换成恒真放行）
    assert!(
        canon(&body).contains(&canon(
            "check_permission(role_id, \"user\", \"delete\", Some(target_id))"
        )),
        "跨账户删除必须仍按 user/delete 键判权限（键在代码里合法，禁的是外显到出参）"
    );
}

// =========================================================
// 3) sku 映射删除引用预检锁（普查 A12：裸 FK 23503 → 500）
// =========================================================

fn extract_fn(src: &str, header: &str) -> String {
    let start = src
        .find(header)
        .unwrap_or_else(|| panic!("找不到函数: {header}"));
    let mut depth = 0usize;
    let mut opened = false;
    for (i, ch) in src[start..].char_indices() {
        match ch {
            '{' => {
                depth += 1;
                opened = true;
            }
            '}' => {
                if opened {
                    depth -= 1;
                    if depth == 0 {
                        return src[start..start + i + 1].to_string();
                    }
                }
            }
            _ => {}
        }
    }
    panic!("函数大括号不配对: {header}");
}

#[test]
fn g5_sku_mapping_delete_requires_reference_precheck() {
    let src = src_of("src/services/sku_mapping_service.rs");
    let delete_fn = extract_fn(src, "pub async fn delete(&self, id: i32)");

    // 预检必须存在：先取映射行（不存在→404），再核查采购/调拨单据快照引用
    assert!(
        delete_fn.contains("product_supplier_mapping::Entity::find_by_id(id)"),
        "delete 必须先加载映射行做存在性检查（不存在→404 原码）"
    );
    assert!(
        delete_fn.contains("purchase_order_item::Entity::find()"),
        "delete 必须对采购/调拨单据（purchase_order_items 转采购快照列）做引用预检"
    );
    assert!(
        delete_fn.contains("business_displayable")
            && delete_fn.contains("该映射已被采购/调拨单据引用，无法删除"),
        "被引用时必须用 business_displayable 公开规则文案拒绝（business 会被脱敏成固定常量）"
    );
    // 公开规则文案不含约束名/错误码/内部标识（保密分层）
    assert!(
        !delete_fn.contains("23503") && !delete_fn.contains("foreign key"),
        "拒绝文案与错误构造不得外泄 FK 约束名/SQL 细节"
    );
    // delete 路径禁止 internal：真正 DbErr 经 `?` 走 From<DbErr>→DATABASE_ERROR + ERROR 日志
    assert!(
        !delete_fn.contains("AppError::internal"),
        "delete 路径禁止 internal 拍平"
    );
    assert!(
        delete_fn.contains(".exec(&*self.db)\n            .await?")
            || delete_fn.contains(".await?;"),
        "硬删语句必须用 ? 让 DbErr 归 database 族（不得吞掉）"
    );
    // 全文件 internal 只剩"供应商商品记录数据异常：关联不存在"完整性不变式 1 处（水位由 ratchet 锁兜）
    assert_eq!(
        count_internal(src),
        1,
        "sku_mapping_service internal 水位应为 1"
    );
}

// =========================================================
// 4) 真实断言：错误经真实 handler 出参，不再被拍平成 500
// =========================================================

async fn request_json(app: &Router, req: Request<Body>) -> (StatusCode, Value) {
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

/// 辅助核算余额：非法期间（月份 13）由 service `parse_period` 产出
/// `validation_displayable`，修复后经 handler 原样传播——400 + 真实原因外显；
/// 修复前被 `.map_err(|e| AppError::internal(e.to_string()))` 拍平成 500"服务器内部错误"。
#[tokio::test]
async fn assist_balance_invalid_period_surfaces_real_reason_400() {
    let app = Router::new()
        .route(
            "/assist-accounting/balance",
            get(assist_accounting_handler::get_assist_balance),
        )
        .with_state(AppState::default());

    let (status, v) = request_json(
        &app,
        Request::get("/assist-accounting/balance?accounting_period=2026-13&dimension_code=dept")
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "实际出参: {v}");
    assert_eq!(v["code"], "VALIDATION_ERROR");
    assert_eq!(
        v["message"], "月份必须在1-12之间",
        "真实拒绝原因必须外显（validation_displayable 白名单口径）"
    );
    assert_ne!(
        v["message"], "服务器内部错误",
        "不得再被 internal 拍平成 500 文案"
    );
}

/// 业务追溯：不存在的五维 ID 经真实 handler → 404 NOT_FOUND（不再是 500 INTERNAL_ERROR）。
/// 夹具为已迁移真库（路线一）：business_trace_chain 由迁移 m0013 建表、夹具 TRUNCATE
/// 后为空表——"五维 ID 不存在"即生产语义本身，不再自建 sqlite 同构表。
/// 出参 message 按 `utils/error.rs` 白名单脱敏口径为固定常量"资源未找到"，
/// 语义由 status/code 承载；修复前 service 侧 4xx 全被拍平成 500。
#[tokio::test]
async fn trace_missing_id_returns_404_not_500() {
    let db = setup_test_db().await;
    let state = AppState {
        db: std::sync::Arc::new(db),
        ..Default::default()
    };

    let app = Router::new()
        .route(
            "/business-trace/by-five-dimension/{fid}",
            get(business_trace_handler::get_trace_by_five_dimension),
        )
        .with_state(state);

    let (status, v) = request_json(
        &app,
        Request::get("/business-trace/by-five-dimension/NOT_EXIST_5D_ID")
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND, "实际出参: {v}");
    assert_eq!(v["code"], "NOT_FOUND");
    assert_ne!(v["code"], "INTERNAL_ERROR", "不得再被拍平成 500");
    assert_ne!(v["message"], "服务器内部错误");
    assert_eq!(
        v["message"], "资源未找到",
        "NotFound 族出参按白名单脱敏口径固定"
    );
}
