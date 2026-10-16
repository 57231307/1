use std::sync::Mutex;
use tauri::Manager;

/// 记住 gateway（服务器地址设置页）自身的加载地址，供登录失败回弹时精确跳回。
/// 移动端 asset  origins 因平台而异（Android 为 http://tauri.localhost 等），故不硬编码，
/// 而由 gateway 首次加载时把 `window.location.href` 传进来存住。
struct GatewayUrl(Mutex<Option<String>>);

/// Tauri v2 Android 客户端 Rust 内核。
///
/// 形态：WebView 先载入内置 gateway（服务器地址设置页），用户首装手填部署地址 → 持久化 →
/// 同域跳转进该地址上的现有 ERP Web（复用现有 httpOnly+CSRF，Web 全部功能零改继承）。
/// 登录失败需重填时，远端 Web（或 gateway）经 `reopen_gateway` 跳回 gateway。
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(GatewayUrl(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            app_info,
            remember_gateway,
            reopen_gateway
        ])
        .run(tauri::generate_context!())
        .expect("Bingxi ERP 移动端启动失败");
}

/// IPC 探针：确认 WebView↔Rust 通路可用，供前端识别"运行在原生壳内"。
#[tauri::command]
fn app_info() -> serde_json::Value {
    serde_json::json!({ "app": "bingxi-erp-mobile", "shell": "tauri-v2" })
}

/// gateway 首次加载时记录自身地址（用于回弹）。
#[tauri::command]
fn remember_gateway(state: tauri::State<GatewayUrl>, url: String) {
    *state.0.lock().unwrap() = Some(url);
}

/// 跳回 gateway（登录失败自动重填入口）。需先在 gateway 调过 remember_gateway。
#[tauri::command]
fn reopen_gateway(app: tauri::AppHandle, state: tauri::State<GatewayUrl>) -> Result<(), String> {
    let url = state
        .0
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "gateway 地址未记录".to_string())?;
    let parsed: tauri::Url = url.parse().map_err(|e| format!("非法 gateway url: {e}"))?;
    if !is_safe_gateway_url(&parsed) {
        return Err("gateway url 非预期 http/https origin".to_string());
    }
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "主窗口不存在".to_string())?;
    window.navigate(parsed).map_err(|e| e.to_string())
}

/// 仅允许回跳到本地 gateway（tauri 内置 asset origin，http(s) scheme），拒绝被注入的外部目标。
fn is_safe_gateway_url(url: &tauri::Url) -> bool {
    let scheme = url.scheme();
    (scheme == "http" || scheme == "https")
        && matches!(url.host_str(), Some("tauri.localhost") | Some("localhost"))
}

#[cfg(test)]
mod tests {
    use super::is_safe_gateway_url;

    fn u(s: &str) -> tauri::Url {
        s.parse().unwrap()
    }

    #[test]
    fn accepts_local_tauri_gateway() {
        assert!(is_safe_gateway_url(&u("http://tauri.localhost/index.html")));
        assert!(is_safe_gateway_url(&u("http://localhost/")));
    }

    #[test]
    fn rejects_external_and_non_http_targets() {
        // 远端 ERP 站点不应被当作回跳目标（回跳只去本地 gateway）
        assert!(!is_safe_gateway_url(&u("https://erp.example.com/")));
        assert!(!is_safe_gateway_url(&u("file:///etc/passwd")));
        assert!(!is_safe_gateway_url(&u("javascript:alert(1)")));
    }
}
