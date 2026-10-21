use serde::Serialize;
use std::sync::Mutex;
use tauri::plugin::{Builder, PluginHandle, TauriPlugin};
use tauri::Manager;

/// 记住 gateway（服务器地址设置页）自身的加载地址，供登录失败回弹时精确跳回。
/// 移动端 asset  origins 因平台而异（Android 为 http://tauri.localhost 等），故不硬编码，
/// 而由 gateway 首次加载时把 `window.location.href` 传进来存住。
struct GatewayUrl(Mutex<Option<String>>);

/// 安卓蓝牙打印插件句柄。仅安卓注册成功后才 manage，桌面端命令据此返回不可用。
struct BluetoothBackend {
    handle: PluginHandle<tauri::Wry>,
}

/// 连接命令载荷，address 为系统蓝牙设置中已配对打印机的 MAC 地址。
#[derive(Serialize)]
struct BluetoothConnectPayload<'a> {
    address: &'a str,
}

/// 打印命令载荷，字段名与 Kotlin 侧 BluetoothPrintTextArgs 一一对应。
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BluetoothPrintTextPayload<'a> {
    text: &'a str,
    cut: bool,
    feed_lines: u8,
    encoding: &'a str,
}

/// Tauri v2 Android 客户端 Rust 内核。
///
/// 形态：WebView 先载入内置 gateway（服务器地址设置页），用户首装手填部署地址 → 持久化 →
/// 同域跳转进该地址上的现有 ERP Web（复用现有 httpOnly+CSRF，Web 全部功能零改继承）。
/// 登录失败需重填时，远端 Web（或 gateway）经 `reopen_gateway` 跳回 gateway。
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(bluetooth_plugin())
        .manage(GatewayUrl(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            app_info,
            remember_gateway,
            reopen_gateway,
            bluetooth_request_permission,
            bluetooth_open_settings,
            bluetooth_list_printers,
            bluetooth_connect,
            bluetooth_print_text,
            bluetooth_disconnect
        ])
        .run(tauri::generate_context!())
        .expect("Bingxi ERP 移动端启动失败");
}

/// 注册安卓原生蓝牙打印插件；桌面端不注册，对应命令统一返回不可用。
fn bluetooth_plugin() -> TauriPlugin<tauri::Wry> {
    Builder::new("bluetooth-print")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            {
                let handle =
                    api.register_android_plugin("com.bingxi.erp", "BluetoothPrintPlugin")?;
                app.manage(BluetoothBackend { handle });
            }
            #[cfg(not(target_os = "android"))]
            let _ = (app, api);
            Ok(())
        })
        .build()
}

fn bluetooth_backend(app: &tauri::AppHandle) -> Result<tauri::State<'_, BluetoothBackend>, String> {
    app.try_state::<BluetoothBackend>()
        .ok_or_else(|| "蓝牙打印仅在安卓客户端可用".to_string())
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

/// 申请安卓 12 及以上所需的附近设备运行时权限，返回是否已授予。
#[tauri::command]
async fn bluetooth_request_permission(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    bluetooth_backend(&app)?
        .handle
        .run_mobile_plugin_async("requestPermission", ())
        .await
        .map_err(|e| e.to_string())
}

/// 打开系统蓝牙设置页，供用户在系统里配对打印机。
#[tauri::command]
async fn bluetooth_open_settings(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    bluetooth_backend(&app)?
        .handle
        .run_mobile_plugin_async("openSettings", ())
        .await
        .map_err(|e| e.to_string())
}

/// 列出系统里已配对的蓝牙设备及其连接状态。
#[tauri::command]
async fn bluetooth_list_printers(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    bluetooth_backend(&app)?
        .handle
        .run_mobile_plugin_async("listPrinters", ())
        .await
        .map_err(|e| e.to_string())
}

/// 与指定 MAC 的已配对打印机建立 SPP 连接。
#[tauri::command]
async fn bluetooth_connect(
    app: tauri::AppHandle,
    address: String,
) -> Result<serde_json::Value, String> {
    bluetooth_backend(&app)?
        .handle
        .run_mobile_plugin_async("connect", BluetoothConnectPayload { address: &address })
        .await
        .map_err(|e| e.to_string())
}

/// 向已连接的打印机发送纯文本小票。
#[tauri::command]
async fn bluetooth_print_text(
    app: tauri::AppHandle,
    text: String,
    cut: Option<bool>,
    feed_lines: Option<u8>,
    encoding: Option<String>,
) -> Result<serde_json::Value, String> {
    let encoding = encoding.unwrap_or_else(|| "GBK".to_string());
    let payload = BluetoothPrintTextPayload {
        text: &text,
        cut: cut.unwrap_or(true),
        feed_lines: feed_lines.unwrap_or(3),
        encoding: &encoding,
    };
    bluetooth_backend(&app)?
        .handle
        .run_mobile_plugin_async("printText", payload)
        .await
        .map_err(|e| e.to_string())
}

/// 断开当前打印机连接。
#[tauri::command]
async fn bluetooth_disconnect(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    bluetooth_backend(&app)?
        .handle
        .run_mobile_plugin_async("disconnect", ())
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::{is_safe_gateway_url, BluetoothPrintTextPayload};

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

    #[test]
    fn print_payload_matches_kotlin_field_names() {
        // Kotlin BluetoothPrintTextArgs 按 camelCase 反序列化，字段名漂移会静默打印空小票
        let payload = BluetoothPrintTextPayload {
            text: "小票",
            cut: true,
            feed_lines: 3,
            encoding: "GBK",
        };
        assert_eq!(
            serde_json::to_value(payload).unwrap(),
            serde_json::json!({ "text": "小票", "cut": true, "feedLines": 3, "encoding": "GBK" })
        );
    }
}
