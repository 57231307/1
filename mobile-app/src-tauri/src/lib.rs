use serde::Serialize;
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::Mutex;
use std::time::Duration;
use tauri::plugin::{Builder, PluginHandle, TauriPlugin};
use tauri::Manager;

/// 记住 gateway（服务器地址设置页）自身的稳定基址，供登录失败回弹时精确跳回。
/// 移动端 asset origins 因平台而异（Android 为 http://tauri.localhost 等），故不硬编码，
/// 而由 gateway 每次加载时把 `window.location.href` 传进来，由 remember_gateway 剥掉
/// query 与 fragment 后存住，重填标记由 reopen_gateway 自行追加。
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
/// 形态，WebView 先载入内置 gateway（服务器地址设置页）。首装手填部署地址，卡页经
/// probe_server_reachability 确认地址可达后持久化并同域跳转进该地址上的现有 ERP Web
/// （复用现有 httpOnly+CSRF，Web 全部功能零改继承）。此后每次启动卡页先探测已存地址，
/// 可达即自动进入，用户无需重复配置；不可达时卡页按连续失败计数推进，未计满上限只给重试
/// 面板不出配置窗，计满上限或远端 Web 自己数到连续失败上限后经 reopen_gateway 跳回时，
/// 本命令在记录的稳定基址上追加 reconfigure=1 标记，卡页据此渲染空表单，不携带旧地址也不
/// 自动跳回。计数的存放与阈值判定见 mobile-app/dist/gateway.js 与 frontend 的 login-failure。
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(bluetooth_plugin())
        .manage(GatewayUrl(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            app_info,
            remember_gateway,
            reopen_gateway,
            probe_server_reachability,
            bluetooth_request_permission,
            app_request_camera_permission,
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

/// gateway（服务器地址设置页）每次加载时记录自身稳定基址（剥掉 query 与 fragment，
/// 回弹标记由 reopen_gateway 自行追加），登录失败回弹后表单为空、不带旧地址。
#[tauri::command]
fn remember_gateway(state: tauri::State<GatewayUrl>, url: String) -> Result<(), String> {
    let mut parsed: tauri::Url = url.parse().map_err(|e| format!("非法 gateway url {e}"))?;
    parsed.set_query(None);
    parsed.set_fragment(None);
    *state.0.lock().unwrap() = Some(parsed.to_string());
    Ok(())
}

/// 跳回 gateway（登录失败自动重填入口）。需先在 gateway 调过 remember_gateway。
/// 重填标记 reconfigure=1 由本命令追加在基址之上，gateway 据此渲染空表单。
#[tauri::command]
fn reopen_gateway(app: tauri::AppHandle, state: tauri::State<GatewayUrl>) -> Result<(), String> {
    let url = state
        .0
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "gateway 地址未记录".to_string())?;
    let mut parsed: tauri::Url = url.parse().map_err(|e| format!("非法 gateway url {e}"))?;
    if !is_safe_gateway_url(&parsed) {
        return Err("gateway url 非预期 http/https origin".to_string());
    }
    parsed.set_query(Some("reconfigure=1"));
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

/// gateway 可达性探测的 TCP 握手等待上限。为什么取 3 秒，首装与启动都要在用户
/// 可感知的等待内给出结论，过长卡页白屏干等，过短会把弱网或云端冷启动下的可达服务器
/// 误判成不可达，3 秒覆盖移动网络 TCP 建连的常见尾延迟，超时即判不可达交回手填。
const GATEWAY_PROBE_TIMEOUT: Duration = Duration::from_secs(3);

/// 探测结论词表，卡页按结论给不同提示，DNS 解析失败与端口拒绝不糊成同一句连接失败。
const PROBE_REACHABLE: &str = "reachable";
const PROBE_DNS_FAILED: &str = "dns";
const PROBE_REFUSED: &str = "refused";
const PROBE_TIMEOUT: &str = "timeout";
const PROBE_UNREACHABLE: &str = "unreachable";

/// 从服务器地址解析探测用主机与端口，纯函数便于单测。
/// 端口缺省按方案标准端口，https 为 443，http 为 80，显式端口照用，非 http/https 拒绝。
fn server_endpoint(target: &str) -> Result<(String, u16), String> {
    let parsed: tauri::Url = target.parse().map_err(|e| format!("非法服务器地址 {e}"))?;
    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(format!("不支持的地址协议 {scheme}"));
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| "地址缺少主机名".to_string())?;
    let port = parsed
        .port_or_known_default()
        .ok_or_else(|| "地址缺少端口且协议无标准端口".to_string())?;
    Ok((host.to_string(), port))
}

/// 阻塞式 TCP 握手探测，返回结论词表中的一个，网络层的失败按可区分的种类返回而不是全部糊成
/// 一种，DNS 解析失败与端口拒绝在卡页是两句不同的提示，仅地址本身不合法返回 Err 供页面提示改填。
fn probe_blocking(target: &str) -> Result<&'static str, String> {
    let (host, port) = server_endpoint(target)?;
    let addrs = match (host.as_str(), port).to_socket_addrs() {
        Ok(iter) => iter.collect::<Vec<_>>(),
        // 域名解析不出与解析后无可用结果都归为解析层失败
        Err(_) => return Ok(PROBE_DNS_FAILED),
    };
    let addr = match addrs.into_iter().next() {
        Some(addr) => addr,
        None => return Ok(PROBE_DNS_FAILED),
    };
    match TcpStream::connect_timeout(&addr, GATEWAY_PROBE_TIMEOUT) {
        Ok(_) => Ok(PROBE_REACHABLE),
        Err(err) => match err.kind() {
            std::io::ErrorKind::ConnectionRefused => Ok(PROBE_REFUSED),
            std::io::ErrorKind::TimedOut => Ok(PROBE_TIMEOUT),
            _ => Ok(PROBE_UNREACHABLE),
        },
    }
}

/// gateway 启动免配置与保存前校验用的 TCP 可达性探测。
/// 调用方 内置 gateway 卡页（本地 origin），入参 target 为规范化后的服务器地址，
/// 结论字符串回卡页决定自动进入还是留在空表单以及该给哪句提示，命令名注册在 capabilities。
/// 探测成功不等于页面可用，随后登录仍失败时按远端登录页自己的连续失败计数处理，
/// 卡页与远端页面 origin 不同，本地存储不互通，卡页不读也不清那份计数。
/// 存什么 不存储。
#[tauri::command]
async fn probe_server_reachability(target: String) -> Result<&'static str, String> {
    tauri::async_runtime::spawn_blocking(move || probe_blocking(&target))
        .await
        .map_err(|e| e.to_string())?
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

/// 申请相机运行时权限，供扫码对话框启动 getUserMedia 前显式授权。
#[tauri::command]
async fn app_request_camera_permission(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    bluetooth_backend(&app)?
        .handle
        .run_mobile_plugin_async("requestCameraPermission", ())
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
    use super::{
        is_safe_gateway_url, probe_blocking, server_endpoint, BluetoothPrintTextPayload,
        PROBE_DNS_FAILED, PROBE_REFUSED,
    };

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

    #[test]
    fn probe_endpoint_uses_scheme_default_ports() {
        // 免配置启动的成败取决于端口口径，https 缺省 443，http 缺省 80，显式端口照用
        assert_eq!(
            server_endpoint("https://erp.example.com").unwrap(),
            ("erp.example.com".to_string(), 443)
        );
        assert_eq!(
            server_endpoint("http://localhost").unwrap(),
            ("localhost".to_string(), 80)
        );
        assert_eq!(
            server_endpoint("http://127.0.0.1:3000/erp").unwrap(),
            ("127.0.0.1".to_string(), 3000)
        );
    }

    #[test]
    fn probe_rejects_non_http_and_unparsable() {
        assert!(server_endpoint("ftp://erp.example.com").is_err());
        assert!(server_endpoint("不是地址").is_err());
        assert!(server_endpoint("http://").is_err());
    }

    #[test]
    fn probe_reports_distinguishable_outcomes() {
        // .invalid 是 RFC 保留顶级域，解析必败，结论应是解析失败而不是笼统连接失败
        assert_eq!(
            probe_blocking("http://invalid.invalid.").unwrap(),
            PROBE_DNS_FAILED
        );
        // 回环 1 端口无人监听时系统立刻拒绝，与解析失败是两种可区分的结论
        assert_eq!(probe_blocking("http://127.0.0.1:1").unwrap(), PROBE_REFUSED);
    }
}
