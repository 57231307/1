// CSRF（Cross-Site Request Forgery）防护中间件
//
// 设计目标：
// - 对所有"有副作用"的 HTTP 方法（POST/PUT/PATCH/DELETE）强制要求请求头 `X-CSRF-Token`。
// - 安全方法（GET/HEAD/OPTIONS）天然无副作用，跳过校验。
// - 公开路径（登录、刷新、初始化、健康检查等）跳过校验，由 [is_public_path] 控制。
// - 校验通过后立即从缓存移除 token，实现"一次性使用"rotation，防止重放。
// - 缺失/无效/绑定不符 token 均以 403 + 业务 code 返回，由前端拦截并跳转登录。
//
// 安全约束：
// - 错误消息走常量 [CSRF_MISSING_MSG] / [CSRF_INVALID_MSG] / [CSRF_BINDING_MSG]，
//   禁止硬编码到响应体中。
// - 命名遵循 ≤9 个英文字符的内部约定（如 `CSRF_HDR`、`CODE_MISS` 等仅在本文件内使用）。
// - 死代码处理遵循项目规范：逐项评估，接入业务或删除，禁止保留无标注死代码。
//
// 双提交（dual-submit）实现口径：
// - 后端把 CSRF Token 以非 httpOnly Cookie 下发，前端从 document.cookie 读出后放入请求头，
//   见 frontend/src/api/auth.ts 与 frontend/src/api/request.ts。
//   跨站页面既读不到本站 Cookie，也无法构造出带该请求头的同源写请求，这是 CSRF 防护的成立条件。
// - 服务端侧的拒绝依据是"这个 token 属于哪个用户、哪个会话"：用户必核，会话已知时必核。
//
// 观测 IP 的定位（重要，勿改回拒绝条件）：
// - 观测 IP 与绑定 IP 不一致不再是拒绝条件，而是"同一会话的客户端换了出口地址"这一异常信号。
//   移动端 Wi-Fi 与蜂窝切换、DHCP 重新租约、CGNAT 与运营商出口池轮换都会让同一个会话的 IP
//   在两次请求之间必然不同；把 IP 全等比较当拒绝条件，等于把正常移动会话硬断成 403。
//   而 IP 本身可被客户端与中间层伪造，不满足"不可伪造的持有者证明"，因此它在 CSRF 威胁模型里
//   从来不是必要条件。
// - 命中漂移时的处置是原子的：旧 token 进入宽限窗口（见 utils::cache 的漂移宽限常量），
//   本请求继续执行，随后由 rotation 重发一枚绑定到新 IP 的新 token，并把这次变化写入综合审计
//   （omni_audit_logs，事件类型 SECURITY_ALERT）。既不静默，也不只记日志不处置。
// - IP 来源优先级：X-Real-IP（边缘 Nginx 以 remote_addr 覆写，客户端伪造不进去）
//   优于 X-Forwarded-For 首段（客户端可控，仅作降级来源）优于 ConnectInfo（直连部署下才是客户端 IP），
//   三者全缺时是占位值，占位值与真实 IP 之间不构成漂移证据。见 middleware::audit_context。

use crate::container::AppState;
use crate::middleware::audit_context::{ClientIpSource, extract_client_ip_with_source};
use crate::middleware::auth_context::{AuthContext, CsrfSession};
use crate::middleware::public_routes::{is_auth_only_path, is_public_path};
use crate::services::omni_audit_service::OmniAuditMessage;
use crate::utils::cache::{CSRF_SESSION_UNBOUND, CsrfConsumeResult};
use axum::{
    body::Body,
    extract::State,
    http::{HeaderName, Method, Request, StatusCode, header},
    middleware::Next,
    response::Response,
};

/// CSRF 请求头名称（小写形式，对应 HTTP/2 规范）
pub const CSRF_HDR_NAME: &str = "x-csrf-token";

/// 轮换后新 token 的下发响应头（前端恢复重放读取用）
const NEW_CSRF_HDR: &str = "x-new-csrf-token";

/// 业务错误码：缺失 CSRF Token
pub const CODE_MISS: &str = "CSRF_TOKEN_MISSING";

/// 业务错误码：CSRF Token 无效或已过期
pub const CODE_INVAL: &str = "CSRF_TOKEN_INVALID";

/// 业务错误码：CSRF Token 与当前用户或会话的绑定核验不通过（疑似盗用，不参与自动恢复重放）
pub const CODE_BIND: &str = "CSRF_TOKEN_BINDING_MISMATCH";

/// 业务错误消息：缺失 CSRF Token
pub const CSRF_MISSING_MSG: &str = "CSRF Token 缺失";

/// 业务错误消息：CSRF Token 无效或已过期
pub const CSRF_INVALID_MSG: &str = "CSRF Token 无效或已过期";

/// 业务错误消息：CSRF Token 绑定核验不通过（只说用户该做什么，不外泄内部判定细节）
pub const CSRF_BINDING_MSG: &str = "安全令牌与当前会话不匹配，请重新登录后再试";

/// 综合审计事件类型：安全告警（omni_audit_service 对该类型触发告警日志）
const AUDIT_TYPE_SECURITY: &str = "SECURITY_ALERT";

/// 综合审计事件名：CSRF 观测 IP 漂移
const AUDIT_EVENT_IP_DRIFT: &str = "csrf_ip_drift";

/// CSRF 恢复 Cookie 的属性串（非 httpOnly、非加密，前端 document.cookie 可读）
/// 不可写 "HttpOnly=false"——RFC 6265 解析器只认属性名，写了即等效开启，前端永远读不到。
const CSRF_COOKIE_ATTRS: &str = "Path=/; SameSite=Strict; Max-Age=1800";

/// 判断是否为无副作用的 HTTP 安全方法（GET/HEAD/OPTIONS）
fn is_safe_method(method: &Method) -> bool {
    matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
}

/// 公开路径 L-1 防御：要求携带 X-Requested-With 或 X-CSRF-Token 自定义请求头
fn check_public_path_header(
    request: &Request<Body>,
    path: &str,
    method: &Method,
) -> Result<(), Box<Response>> {
    let has_xhr = request
        .headers()
        .get("x-requested-with")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.eq_ignore_ascii_case("XMLHttpRequest"))
        .unwrap_or(false);
    let has_csrf = request.headers().contains_key(CSRF_HDR_NAME);

    if !has_xhr && !has_csrf {
        tracing::warn!(
            path = %path,
            method = %method,
            "CSRF 验证失败：公开端点的非安全方法缺少自定义请求头"
        );
        return Err(Box::new(csrf_error_response(
            CODE_MISS,
            "缺少必要的请求头，请使用 AJAX 方式请求",
        )));
    }
    Ok(())
}

/// 从请求头提取并清理 CSRF Token（去空白、过滤空串）
fn extract_csrf_token(request: &Request<Body>) -> Option<String> {
    request
        .headers()
        .get(CSRF_HDR_NAME)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// 当前认证上下文的用户 ID（未认证为 None，绑定核验据此拒绝）
fn authenticated_user_id(request: &Request<Body>) -> Option<i32> {
    request.extensions().get::<AuthContext>().map(|a| a.user_id)
}

/// 当前 JWT 会话标识（由 auth 中间件注入，缺失时按未绑定会话处理）
fn session_id_of(request: &Request<Body>) -> Option<&str> {
    request
        .extensions()
        .get::<CsrfSession>()
        .map(|s| s.session_id.as_str())
}

/// 构造带 CSRF 恢复能力的 403 错误响应：签发一枚一次性恢复 token 存入缓存
/// （绑定当前请求 IP、当前用户与会话），并在响应头 x-new-csrf-token 与 Set-Cookie 双侧下发。
/// 前端已有的恢复重放会自动读取并携新 token 重试，
/// 从而消除并发或轮换竞态下 CSRF 暂缺即被登出的伪登出成因。
/// 仅用于 NotFound（已消费/过期）与 Missing（头缺失）两种可恢复形态；
/// 绑定核验失败属盗用面，不在此列——给攻击方自动补发新 token 等于替它绕过一次性语义。
/// 恢复 token 走同一一次性消费缓存，不改变失效即登出的整体策略。
fn build_csrf_recovery_response(
    state: &AppState,
    client_ip: &str,
    user_id: i32,
    session_id: &str,
    code: &str,
    message: &str,
) -> Response {
    let recovery_token = uuid::Uuid::new_v4().to_string();
    state.cache.set_csrf_token(
        recovery_token.clone(),
        session_id.to_string(),
        client_ip.to_string(),
        user_id,
        None,
    );
    let mut resp = csrf_error_response(code, message);
    if let Ok(val) = axum::http::HeaderValue::from_str(&recovery_token) {
        resp.headers_mut()
            .insert(HeaderName::from_static(NEW_CSRF_HDR), val);
    }
    // 同步 Set-Cookie，让前端可从 document.cookie 读到恢复 token（非 httpOnly、非加密）。
    let cookie_value = format!("csrf_token={}; {}", recovery_token, CSRF_COOKIE_ATTRS);
    resp.headers_mut().append(
        axum::http::header::SET_COOKIE,
        axum::http::HeaderValue::from_str(&cookie_value)
            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("")),
    );
    resp
}

/// 一次性消费 CSRF Token，并把它映射为"放行"或"403 响应"。
/// 放行形态有两种：绑定与 IP 都一致（Ok）；绑定一致但观测 IP 漂移（OkIpDrift，调用方已先留痕）。
/// 拒绝形态：NotFound（不存在或已过期，可恢复）、BindingMismatch（盗用面，不可恢复）。
fn reject_or_accept(
    state: &AppState,
    outcome: CsrfConsumeResult,
    client_ip: &str,
    user_id: i32,
    session_id: &str,
    path: &str,
    method: &Method,
) -> Result<(), Box<Response>> {
    match outcome {
        CsrfConsumeResult::Ok | CsrfConsumeResult::OkIpDrift { .. } => Ok(()),
        CsrfConsumeResult::BindingMismatch(kind) => {
            tracing::warn!(
                path = %path,
                method = %method,
                mismatch = ?kind,
                "CSRF 验证失败：Token 的用户或会话绑定与当前认证上下文不符"
            );
            Err(Box::new(csrf_error_response(CODE_BIND, CSRF_BINDING_MSG)))
        }
        CsrfConsumeResult::NotFound => {
            tracing::warn!(
                path = %path,
                method = %method,
                "CSRF 验证失败：Token 不存在或已被消费/过期"
            );
            // 并发竞争缓解：一次性消费 + 轮换机制下，同一会话并发 POST 必有一个携带已消费的旧 token。
            // NotFound（已消费/过期）属可恢复形态，签发恢复 token 并下发 x-new-csrf-token + Set-Cookie，
            // 让竞败方经前端恢复重放立即恢复，避免被误踢出登录态（多标签页场景同样受益）。
            Err(Box::new(build_csrf_recovery_response(
                state,
                client_ip,
                user_id,
                session_id,
                CODE_INVAL,
                CSRF_INVALID_MSG,
            )))
        }
    }
}

/// 消费成功后的轮换：生成新 CSRF Token 绑定到当前 IP/用户/会话写入 cache，
/// 返回待下发的新 token（未认证上下文时不轮换，返回 None）。
/// 会话标识与签发时同源，保证"当前会话的下一个请求"必然能通过绑定核验。
fn rotate_csrf_after_consume(
    state: &AppState,
    request: &Request<Body>,
    client_ip: &str,
) -> Option<String> {
    let user_id = authenticated_user_id(request)?;
    let new_csrf_token = uuid::Uuid::new_v4().to_string();
    state.cache.set_csrf_token(
        new_csrf_token.clone(),
        session_id_of(request)
            .unwrap_or(CSRF_SESSION_UNBOUND)
            .to_string(),
        client_ip.to_string(),
        user_id,
        None,
    );
    Some(new_csrf_token)
}

/// 把轮换出的新 token 追加为 Set-Cookie 头（非加密非 httpOnly，前端 JS 可读取明文）
fn append_csrf_cookie(response: &mut Response, token: &str) {
    let cookie_value = format!("csrf_token={}; {}", token, CSRF_COOKIE_ATTRS);
    response.headers_mut().append(
        header::SET_COOKIE,
        axum::http::HeaderValue::from_str(&cookie_value)
            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("")),
    );
}

/// 观测 IP 漂移留痕：写入综合审计通道 omni_audit_logs（HMAC 签名 + SECURITY_ALERT 告警规则），
/// 不新造一套存储。事件只记会话、用户、前后 IP 与 IP 来源档位，绝不落 Token 明文。
/// 来源档位用于事后区分真换网与有人伪造 X-Forwarded-For。
fn record_ip_drift_audit(
    state: &AppState,
    request: &Request<Body>,
    path: &str,
    method: &Method,
    previous_ip: &str,
    current_ip: &str,
    ip_source: ClientIpSource,
) {
    let auth = request.extensions().get::<AuthContext>();
    let session_id = session_id_of(request).unwrap_or(CSRF_SESSION_UNBOUND);
    let payload = serde_json::json!({
        "event": AUDIT_EVENT_IP_DRIFT,
        "previous_ip": previous_ip,
        "current_ip": current_ip,
        "ip_source": ip_source.as_str(),
        "session_id": session_id,
        "request_path": path,
        "request_method": method.as_str(),
    });
    let message = OmniAuditMessage {
        trace_id: crate::utils::error::current_trace_id().0,
        user_id: auth.map(|a| a.user_id),
        username: auth.map(|a| a.username.clone()),
        event_type: AUDIT_TYPE_SECURITY.to_string(),
        event_name: AUDIT_EVENT_IP_DRIFT.to_string(),
        resource: path.to_string(),
        action: method.as_str().to_string(),
        resource_type: Some("csrf".to_string()),
        resource_id: Some(session_id.to_string()),
        resource_name: None,
        description: Some(format!(
            "CSRF 观测 IP 由 {} 漂移至 {}（来源档位 {}），会话保持放行并重发令牌",
            previous_ip,
            current_ip,
            ip_source.as_str()
        )),
        payload: Some(payload),
        ip_address: Some(current_ip.to_string()),
        user_agent: request
            .headers()
            .get(header::USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string()),
        request_method: Some(method.as_str().to_string()),
        request_path: Some(path.to_string()),
        request_body: None,
        duration_ms: 0,
        // 该请求被放行并继续执行，状态记 SUCCESS；告警由 event_type=SECURITY_ALERT 触发，
        // 不靠把状态伪装成 DENIED 来制造噪音（把正常换网记成拒绝会污染拒绝率指标）
        status: "SUCCESS".to_string(),
        error_msg: None,
        old_value: Some(serde_json::json!({ "csrf_ip": previous_ip })),
        new_value: Some(serde_json::json!({ "csrf_ip": current_ip })),
        condition: None,
    };
    state.omni_audit.log(message);
}

/// CSRF 验证中间件：跳过安全方法，公开路径要求自定义请求头，
/// 非公开路径校验 Token 的用户与会话绑定，观测 IP 漂移只做留痕与重发、不做拒绝
pub async fn csrf_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    let method = request.method().clone();
    let path = request.uri().path().to_string();

    // 1. 跳过无副作用方法
    if is_safe_method(&method) {
        return Ok(next.run(request).await);
    }

    // 2. 公开路径：L-1 修复 - 要求自定义请求头（防御简单表单提交 CSRF）
    if is_public_path(&path) {
        check_public_path_header(&request, &path, &method).map_err(|e| *e)?;
        return Ok(next.run(request).await);
    }

    // 2b. 认证豁免路径（record-print / ws/ticket）：仅需 JWT 认证，不消耗一次性 token。
    // 必须在 token 提取/消费之前短路——csrf_middleware 在 permission_middleware 之外层执行，
    // 若不在此豁免，is_auth_only_path 永远轮不到，ws/ticket 的重连风暴会与页面其它 POST
    // 争抢一次性 token，把并发请求全部打成 CSRF_TOKEN_INVALID（见 permission.rs 的声明）。
    if is_auth_only_path(&path) {
        return Ok(next.run(request).await);
    }

    // 3. 提取客户端 IP 与来源档位（MISSING 恢复与消费校验共用）
    let (client_ip, ip_source) = extract_client_ip_with_source(&request);

    // 3b. 提取并校验 CSRF Token 头
    let token = match extract_csrf_token(&request) {
        Some(t) => t,
        None => {
            tracing::warn!(
                path = %path,
                method = %method,
                "CSRF 验证失败：请求头 X-CSRF-Token 缺失"
            );
            // MISSING 恢复：与 NotFound 同模式签发恢复 token，附带 x-new-csrf-token 响应头 + Set-Cookie，
            // 让前端恢复重放自动携新 token 重试，消除点更新就登出的伪登出成因。
            // 仍返回 403（不放宽失效即登出的整体策略），只是给可恢复的缺失态一条恢复通道。
            return Err(build_csrf_recovery_response(
                &state,
                &client_ip,
                authenticated_user_id(&request).unwrap_or(0),
                session_id_of(&request).unwrap_or(CSRF_SESSION_UNBOUND),
                CODE_MISS,
                CSRF_MISSING_MSG,
            ));
        }
    };

    // 4. 一次性消费 token：用户与会话是拒绝依据，观测 IP 只判漂移
    let expected_user_id = authenticated_user_id(&request);
    let expected_session_id = session_id_of(&request);
    let outcome =
        state
            .cache
            .consume_csrf_token(&token, &client_ip, expected_user_id, expected_session_id);
    if let CsrfConsumeResult::OkIpDrift { ref previous_ip } = outcome {
        record_ip_drift_audit(
            &state,
            &request,
            &path,
            &method,
            previous_ip,
            &client_ip,
            ip_source,
        );
    }
    reject_or_accept(
        &state,
        outcome,
        &client_ip,
        expected_user_id.unwrap_or(0),
        expected_session_id.unwrap_or(CSRF_SESSION_UNBOUND),
        &path,
        &method,
    )
    .map_err(|e| *e)?;

    // 5. 消费成功后轮换：生成新 CSRF Token（绑定当前 IP、当前用户与会话）存入 cache，随响应 Set-Cookie 下发
    let rotated = rotate_csrf_after_consume(&state, &request, &client_ip);

    let mut response = next.run(request).await;
    if let Some(new_csrf_token) = rotated {
        append_csrf_cookie(&mut response, &new_csrf_token);
    }
    Ok(response)
}

/// 构造 403 CSRF 错误响应（统一 JSON 格式）
pub fn csrf_error_response(code: &str, message: &str) -> Response {
    crate::utils::response::unified_error_response(StatusCode::FORBIDDEN, code, message)
}
